#![cfg(target_arch = "wasm32")]
#![allow(dead_code)]

use dioxus::dioxus_core::NoOpMutations;
use dioxus::prelude::VirtualDom;
use futures_util::FutureExt;
use gloo_timers::future::TimeoutFuture;
use js_sys::{Array, Function, Object, Reflect};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

/// Render any work the VirtualDom has ready, without blocking on future work.
fn drain(dom: &mut VirtualDom) {
    while dom.wait_for_work().now_or_never().is_some() {
        dom.render_immediate(&mut NoOpMutations);
    }
}

/// Drive the VirtualDom and the browser event loop until `cond` holds, up to a
/// generous cap. Renders pending work each tick and returns as soon as `cond`
/// is true, so healthy runs finish in a few ticks; returns false if the cap
/// elapses first. The cap (~6s) tolerates a slow/loaded CI container, where the
/// old fixed `0..20` (500 ms) budget was too tight for the load → loadedness →
/// action chain to complete.
pub async fn settle_until(dom: &mut VirtualDom, mut cond: impl FnMut() -> bool) -> bool {
    for _ in 0..240 {
        drain(dom);
        if cond() {
            return true;
        }
        TimeoutFuture::new(25).await;
    }
    drain(dom);
    cond()
}

/// Drive the VirtualDom for a fixed number of ticks, rendering pending work
/// each tick. For negative assertions ("after settling, X still hasn't
/// happened"), where a condition-driven wait would defeat the purpose.
pub async fn settle_ticks(dom: &mut VirtualDom, ticks: usize) {
    for _ in 0..ticks {
        TimeoutFuture::new(25).await;
        drain(dom);
    }
}

#[derive(Clone, Copy)]
struct MountedWidgetMock {
    mount_method: &'static str,
    unmount_method: &'static str,
}

const MOUNTED_WIDGET_MOCKS: [MountedWidgetMock; 4] = [
    MountedWidgetMock {
        mount_method: "mountSignIn",
        unmount_method: "unmountSignIn",
    },
    MountedWidgetMock {
        mount_method: "mountSignUp",
        unmount_method: "unmountSignUp",
    },
    MountedWidgetMock {
        mount_method: "mountUserButton",
        unmount_method: "unmountUserButton",
    },
    MountedWidgetMock {
        mount_method: "mountUserProfile",
        unmount_method: "unmountUserProfile",
    },
];

pub fn window() -> web_sys::Window {
    web_sys::window().expect("wasm tests run in a browser window")
}

pub fn clear_clerk() {
    let key = JsValue::from_str("Clerk");
    let window = window();
    if let Some(document) = window.document() {
        for id in ["__dioxus_clerk_js", "__clerk_initial_state"] {
            if let Some(script) = document.get_element_by_id(id) {
                script.remove();
            }
        }
    }
    let _ = Reflect::delete_property(window.as_ref(), &key);
    Reflect::set(window.as_ref(), &key, &JsValue::UNDEFINED).unwrap();
    // Reset page-scoped Clerk.load() state so a prior test's never-settling
    // mock load (LOAD_IN_FLIGHT left set) can't make this test's provider block
    // in the load-in-flight wait loop. Makes the suite order-independent.
    dioxus_clerk::__reset_load_state();
}

pub fn set_prop(target: &JsValue, key: &str, value: &JsValue) {
    Reflect::set(target, &JsValue::from_str(key), value).unwrap();
}

pub fn get_prop(target: &JsValue, key: &str) -> JsValue {
    Reflect::get(target, &JsValue::from_str(key)).unwrap()
}

pub fn has_function(target: &JsValue, key: &str) -> bool {
    get_prop(target, key).dyn_ref::<Function>().is_some()
}

pub fn bool_prop(target: &JsValue, key: &str) -> bool {
    get_prop(target, key).as_bool().unwrap_or(false)
}

pub fn number_prop(target: &JsValue, key: &str) -> f64 {
    get_prop(target, key).as_f64().unwrap_or(0.0)
}

fn install_mounted_widget_mocks(clerk: &Object, include_unmounts: bool) {
    for widget in MOUNTED_WIDGET_MOCKS {
        set_prop(
            clerk.as_ref(),
            widget.mount_method,
            mount_recorder(widget.mount_method).as_ref(),
        );

        if include_unmounts {
            set_prop(
                clerk.as_ref(),
                widget.unmount_method,
                unmount_recorder(widget.unmount_method).as_ref(),
            );
        }
    }
}

fn mount_recorder(method: &str) -> Function {
    let method = serde_json::to_string(method).unwrap();
    Function::new_no_args(&format!(
        r#"
        var method = {method};
        var countKey = method + "CallCount";
        this.mountCallCount = (this.mountCallCount || 0) + 1;
        this[countKey] = (this[countKey] || 0) + 1;
        this.lastMountedElement = arguments[0];
        this.lastMountedElementId = arguments[0] && arguments[0].id;
        this.lastMountedOptions = arguments[1];
        this.mountedElementIdsByMethod = this.mountedElementIdsByMethod || {{}};
        this.mountedElementIdsByMethod[method] = this.lastMountedElementId;
        this.mountedOptionsByMethod = this.mountedOptionsByMethod || {{}};
        this.mountedOptionsByMethod[method] = arguments[1];
        this.mountedElementIds = this.mountedElementIds || [];
        this.mountedElementIds.push(this.lastMountedElementId);
        this.mountedOptions = this.mountedOptions || [];
        this.mountedOptions.push(arguments[1]);
        "#
    ))
}

fn unmount_recorder(method: &str) -> Function {
    let method = serde_json::to_string(method).unwrap();
    Function::new_no_args(&format!(
        r#"
        var method = {method};
        var countKey = method + "CallCount";
        this.unmountCallCount = (this.unmountCallCount || 0) + 1;
        this[countKey] = (this[countKey] || 0) + 1;
        this.lastUnmountedElement = arguments[0];
        this.lastUnmountedElementId = arguments[0] && arguments[0].id;
        this.unmountedElementIdsByMethod = this.unmountedElementIdsByMethod || {{}};
        this.unmountedElementIdsByMethod[method] = this.lastUnmountedElementId;
        this.unmountedElementIds = this.unmountedElementIds || [];
        this.unmountedElementIds.push(this.lastUnmountedElementId);
        "#
    ))
}

pub fn install_clerk_mock(is_signed_in: bool) -> JsValue {
    let clerk = Object::new();
    let load = Function::new_no_args(
        r#"
        this.loadCallCount = (this.loadCallCount || 0) + 1;
        this.lastLoadOptions = arguments[0];
        this.loaded = true;
        return Promise.resolve();
        "#,
    );
    let add_listener = Function::new_no_args(
        r#"
        var clerk = this;
        this.lastListener = arguments[0];
        return function unsubscribe() {
            clerk.unsubscribeCallCount = (clerk.unsubscribeCallCount || 0) + 1;
            clerk.lastListener = undefined;
        };
        "#,
    );
    let user = Object::new();
    let session = Object::new();

    set_prop(user.as_ref(), "id", &JsValue::from_str("user_2abc"));
    set_prop(session.as_ref(), "id", &JsValue::from_str("sess_2def"));
    set_prop(session.as_ref(), "status", &JsValue::from_str("active"));
    set_prop(
        session.as_ref(),
        "getToken",
        Function::new_no_args(
            r#"
            this.getTokenCallCount = (this.getTokenCallCount || 0) + 1;
            this.lastGetTokenOptions = arguments[0];
            return Promise.resolve('session_token_2def');
            "#,
        )
        .as_ref(),
    );

    set_prop(clerk.as_ref(), "load", load.as_ref());
    set_prop(
        clerk.as_ref(),
        "signOut",
        Function::new_no_args(
            r#"
            this.signOutCallCount = (this.signOutCallCount || 0) + 1;
            this.lastSignOutOptions = arguments[0];
            return Promise.resolve();
            "#,
        )
        .as_ref(),
    );
    set_prop(clerk.as_ref(), "addListener", add_listener.as_ref());
    set_prop(clerk.as_ref(), "loaded", &JsValue::FALSE);
    set_prop(
        clerk.as_ref(),
        "isSignedIn",
        &JsValue::from_bool(is_signed_in),
    );
    set_prop(clerk.as_ref(), "user", user.as_ref());
    set_prop(clerk.as_ref(), "session", session.as_ref());
    install_mounted_widget_mocks(&clerk, true);
    set_prop(
        clerk.as_ref(),
        "redirectToSignIn",
        Function::new_no_args(
            r#"
            this.redirectCallCount = (this.redirectCallCount || 0) + 1;
            this.lastRedirectOptions = arguments[0];
            return Promise.resolve();
            "#,
        )
        .as_ref(),
    );
    set_prop(
        clerk.as_ref(),
        "redirectToSignUp",
        Function::new_no_args(
            r#"
            this.redirectSignUpCallCount = (this.redirectSignUpCallCount || 0) + 1;
            this.lastRedirectSignUpOptions = arguments[0];
            return Promise.resolve();
            "#,
        )
        .as_ref(),
    );
    set_prop(
        clerk.as_ref(),
        "openSignIn",
        Function::new_no_args(
            r#"
            this.openSignInCallCount = (this.openSignInCallCount || 0) + 1;
            this.lastOpenSignInOptions = arguments[0];
            "#,
        )
        .as_ref(),
    );
    set_prop(
        clerk.as_ref(),
        "closeSignIn",
        Function::new_no_args(
            r#"
            this.closeSignInCallCount = (this.closeSignInCallCount || 0) + 1;
            "#,
        )
        .as_ref(),
    );
    set_prop(
        clerk.as_ref(),
        "openSignUp",
        Function::new_no_args(
            r#"
            this.openSignUpCallCount = (this.openSignUpCallCount || 0) + 1;
            this.lastOpenSignUpOptions = arguments[0];
            "#,
        )
        .as_ref(),
    );
    set_prop(
        clerk.as_ref(),
        "closeSignUp",
        Function::new_no_args(
            r#"
            this.closeSignUpCallCount = (this.closeSignUpCallCount || 0) + 1;
            "#,
        )
        .as_ref(),
    );
    set_prop(
        clerk.as_ref(),
        "openUserProfile",
        Function::new_no_args(
            r#"
            this.openUserProfileCallCount = (this.openUserProfileCallCount || 0) + 1;
            this.lastOpenUserProfileOptions = arguments[0];
            "#,
        )
        .as_ref(),
    );
    set_prop(
        clerk.as_ref(),
        "closeUserProfile",
        Function::new_no_args(
            r#"
            this.closeUserProfileCallCount = (this.closeUserProfileCallCount || 0) + 1;
            "#,
        )
        .as_ref(),
    );

    let clerk: JsValue = clerk.into();
    set_prop(window().as_ref(), "Clerk", &clerk);
    clerk
}

pub fn install_rejecting_clerk() -> JsValue {
    let clerk = Object::new();
    let load = Function::new_no_args(
        r#"
        this.loadCallCount = (this.loadCallCount || 0) + 1;
        return Promise.reject('bad key');
        "#,
    );
    set_prop(clerk.as_ref(), "load", load.as_ref());
    set_prop(clerk.as_ref(), "loaded", &JsValue::FALSE);

    let clerk: JsValue = clerk.into();
    set_prop(window().as_ref(), "Clerk", &clerk);
    clerk
}

pub fn install_pending_load_clerk() -> JsValue {
    let clerk = Object::new();
    let load = Function::new_no_args(
        r#"
        this.loadCallCount = (this.loadCallCount || 0) + 1;
        return new Promise(function () {});
        "#,
    );
    set_prop(clerk.as_ref(), "load", load.as_ref());
    set_prop(clerk.as_ref(), "loaded", &JsValue::FALSE);
    set_prop(clerk.as_ref(), "isSignedIn", &JsValue::FALSE);
    install_mounted_widget_mocks(&clerk, false);

    let clerk: JsValue = clerk.into();
    set_prop(window().as_ref(), "Clerk", &clerk);
    clerk
}

pub fn make_redirect_throw(clerk: &JsValue, message: &str) {
    make_method_throw(clerk, "redirectToSignIn", message);
}

pub fn make_sign_out_reject(clerk: &JsValue, message: &str) {
    let source = format!(
        r#"
        this.signOutCallCount = (this.signOutCallCount || 0) + 1;
        this.lastSignOutOptions = arguments[0];
        return Promise.reject({});
        "#,
        serde_json::to_string(message).unwrap()
    );
    set_prop(clerk, "signOut", Function::new_no_args(&source).as_ref());
}

pub fn make_sign_in_mount_throw(clerk: &JsValue, message: &str) {
    make_method_throw(clerk, "mountSignIn", message);
}

fn make_method_throw(clerk: &JsValue, method: &str, message: &str) {
    let source = format!(
        "throw new Error({});",
        serde_json::to_string(message).unwrap()
    );
    set_prop(clerk, method, Function::new_no_args(&source).as_ref());
}

pub fn last_load_options(clerk: &JsValue) -> JsValue {
    get_prop(clerk, "lastLoadOptions")
}

pub fn last_redirect_options(clerk: &JsValue) -> JsValue {
    get_prop(clerk, "lastRedirectOptions")
}

pub fn last_redirect_sign_up_options(clerk: &JsValue) -> JsValue {
    get_prop(clerk, "lastRedirectSignUpOptions")
}

pub fn last_sign_out_options(clerk: &JsValue) -> JsValue {
    get_prop(clerk, "lastSignOutOptions")
}

pub fn last_open_sign_in_options(clerk: &JsValue) -> JsValue {
    get_prop(clerk, "lastOpenSignInOptions")
}

pub fn mounted_options_for_method(clerk: &JsValue, method: &str) -> JsValue {
    let options_by_method = get_prop(clerk, "mountedOptionsByMethod");
    get_prop(&options_by_method, method)
}

pub fn mounted_element_id_for_method(clerk: &JsValue, method: &str) -> Option<String> {
    let ids_by_method = get_prop(clerk, "mountedElementIdsByMethod");
    get_prop(&ids_by_method, method).as_string()
}

pub fn mounted_element_ids(clerk: &JsValue) -> Vec<String> {
    string_array_prop(clerk, "mountedElementIds")
}

pub fn unmounted_element_ids(clerk: &JsValue) -> Vec<String> {
    string_array_prop(clerk, "unmountedElementIds")
}

pub fn mounted_options(clerk: &JsValue) -> Vec<JsValue> {
    js_array_prop(clerk, "mountedOptions")
}

fn string_array_prop(target: &JsValue, key: &str) -> Vec<String> {
    js_array_prop(target, key)
        .into_iter()
        .filter_map(|value| value.as_string())
        .collect()
}

fn js_array_prop(target: &JsValue, key: &str) -> Vec<JsValue> {
    let value = get_prop(target, key);
    if !Array::is_array(&value) {
        return vec![];
    }

    Array::from(&value).iter().collect()
}

pub fn trigger_listener(clerk: &JsValue) {
    let callback = get_prop(clerk, "lastListener");
    let callback: Function = callback.unchecked_into();
    callback.call1(&JsValue::NULL, &JsValue::UNDEFINED).unwrap();
}

pub fn create_mount_element() -> web_sys::Element {
    window()
        .document()
        .expect("document exists")
        .create_element("div")
        .unwrap()
}

pub fn append_mount_element_with_id(id: &str) -> web_sys::Element {
    let document = window().document().expect("document exists");
    let element = document.create_element("div").unwrap();
    element.set_id(id);
    document
        .body()
        .expect("body exists")
        .append_child(&element)
        .unwrap();
    element
}

pub fn remove_element_by_id(id: &str) {
    if let Some(document) = window().document() {
        if let Some(element) = document.get_element_by_id(id) {
            element.remove();
        }
    }
}
