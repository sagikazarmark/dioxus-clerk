#![cfg(target_arch = "wasm32")]

mod wasm_support;

use dioxus::dioxus_core::{AttributeValue, Mutation, Mutations, NoOpMutations};
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use std::cell::RefCell;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

thread_local! {
    static TEST_OPTIONS: RefCell<serde_json::Value> = const { RefCell::new(serde_json::Value::Null) };
}

thread_local! {
    static PROVIDER_ERRORS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

#[wasm_bindgen_test(async)]
async fn mounted_ui_retries_until_host_exists_and_mounts_once() {
    set_test_options(serde_json::Value::Null);
    reset_provider_errors();
    wasm_support::clear_clerk();
    let clerk = wasm_support::install_clerk_mock(false);

    let mut dom = VirtualDom::new(SignInTestApp);
    let host_id = rebuild_and_capture_host_id(&mut dom);

    wasm_support::settle_until(&mut dom, || {
        wasm_support::number_prop(&clerk, "loadCallCount") == 1.0
    })
    .await;

    assert_eq!(wasm_support::number_prop(&clerk, "loadCallCount"), 1.0);
    assert_eq!(wasm_support::number_prop(&clerk, "mountCallCount"), 0.0);
    assert!(!provider_saw_error());

    wasm_support::append_mount_element_with_id(&host_id);

    drive_mounted_ui(&mut dom, &clerk, 1.0).await;

    assert_eq!(wasm_support::number_prop(&clerk, "mountCallCount"), 1.0);
    assert_eq!(
        wasm_support::get_prop(&clerk, "lastMountedElementId")
            .as_string()
            .as_deref(),
        Some(host_id.as_str())
    );

    wasm_support::settle_ticks(&mut dom, 3).await;

    assert_eq!(wasm_support::number_prop(&clerk, "mountCallCount"), 1.0);

    wasm_support::remove_element_by_id(&host_id);
}

#[wasm_bindgen_test(async)]
async fn mounted_ui_unmounts_after_successful_mount() {
    set_test_options(serde_json::Value::Null);
    wasm_support::clear_clerk();
    let clerk = wasm_support::install_clerk_mock(false);

    let mut dom = VirtualDom::new(SignInTestApp);
    let host_id = rebuild_and_capture_host_id(&mut dom);
    wasm_support::append_mount_element_with_id(&host_id);

    drive_mounted_ui(&mut dom, &clerk, 1.0).await;

    assert_eq!(wasm_support::number_prop(&clerk, "mountCallCount"), 1.0);

    drop(dom);

    for _ in 0..5 {
        if wasm_support::number_prop(&clerk, "unmountCallCount") == 1.0 {
            break;
        }
        TimeoutFuture::new(25).await;
    }

    let unmount_count = wasm_support::number_prop(&clerk, "unmountCallCount");
    wasm_support::remove_element_by_id(&host_id);

    assert_eq!(unmount_count, 1.0);
}

#[wasm_bindgen_test(async)]
async fn mounted_ui_does_not_unmount_without_successful_mount() {
    set_test_options(serde_json::Value::Null);
    wasm_support::clear_clerk();
    let clerk = wasm_support::install_clerk_mock(false);

    let mut dom = VirtualDom::new(SignInTestApp);
    let _host_id = rebuild_and_capture_host_id(&mut dom);

    wasm_support::settle_until(&mut dom, || {
        wasm_support::number_prop(&clerk, "loadCallCount") == 1.0
    })
    .await;

    assert_eq!(wasm_support::number_prop(&clerk, "loadCallCount"), 1.0);

    drop(dom);

    for _ in 0..3 {
        TimeoutFuture::new(25).await;
    }

    assert_eq!(wasm_support::number_prop(&clerk, "mountCallCount"), 0.0);
    assert_eq!(wasm_support::number_prop(&clerk, "unmountCallCount"), 0.0);
}

#[wasm_bindgen_test(async)]
async fn mounted_ui_mount_failure_surfaces_provider_error() {
    set_test_options(serde_json::Value::Null);
    reset_provider_errors();
    wasm_support::clear_clerk();
    let clerk = wasm_support::install_clerk_mock(false);
    wasm_support::make_sign_in_mount_throw(&clerk, "mount failed");

    let mut dom = VirtualDom::new(SignInTestApp);
    let host_id = rebuild_and_capture_host_id(&mut dom);
    wasm_support::append_mount_element_with_id(&host_id);

    wasm_support::settle_until(&mut dom, || provider_error_contains("mount failed")).await;

    wasm_support::remove_element_by_id(&host_id);
    assert!(provider_error_contains("mount failed"));
}

#[wasm_bindgen_test(async)]
async fn mounted_ui_remounts_when_options_change() {
    reset_provider_errors();
    wasm_support::clear_clerk();
    let clerk = wasm_support::install_clerk_mock(false);

    let mut dom = VirtualDom::new(UpdatingSignInTestApp);
    let host_id = rebuild_and_capture_host_id(&mut dom);
    wasm_support::append_mount_element_with_id(&host_id);

    drive_mounted_ui(&mut dom, &clerk, 1.0).await;
    wait_for_unmount_count(&clerk, 1.0).await;
    drive_mounted_ui(&mut dom, &clerk, 2.0).await;

    assert_eq!(wasm_support::number_prop(&clerk, "mountCallCount"), 2.0);
    assert_eq!(wasm_support::number_prop(&clerk, "unmountCallCount"), 1.0);
    assert_js_string_prop(
        &wasm_support::mounted_options_for_method(&clerk, "mountSignIn"),
        "version",
        "second",
    );

    drop(dom);
    wait_for_unmount_count(&clerk, 2.0).await;
    wasm_support::remove_element_by_id(&host_id);
    assert!(!provider_saw_error());
}

#[wasm_bindgen_test(async)]
async fn mounted_ui_public_widgets_mount_once_forward_options_and_unmount() {
    wasm_support::clear_clerk();
    let clerk = wasm_support::install_clerk_mock(false);

    let mut dom = VirtualDom::new(AllPublicWidgetsTestApp);
    let host_ids = rebuild_and_capture_host_ids(&mut dom);
    assert_eq!(host_ids.len(), 4);
    for host_id in &host_ids {
        wasm_support::append_mount_element_with_id(host_id);
    }

    drive_mounted_ui(&mut dom, &clerk, 4.0).await;

    assert_eq!(wasm_support::number_prop(&clerk, "mountCallCount"), 4.0);
    assert_mounted_host_ids(&clerk, &host_ids);
    assert_mounted_component_options(&clerk, ["SignIn", "SignUp", "UserButton", "UserProfile"]);

    wasm_support::settle_ticks(&mut dom, 3).await;

    assert_eq!(wasm_support::number_prop(&clerk, "mountCallCount"), 4.0);

    drop(dom);
    wait_for_unmount_count(&clerk, 4.0).await;

    let unmount_count = wasm_support::number_prop(&clerk, "unmountCallCount");
    for host_id in &host_ids {
        wasm_support::remove_element_by_id(host_id);
    }
    assert_eq!(unmount_count, 4.0);
    assert_unmounted_host_ids(&clerk, &host_ids);
}

#[wasm_bindgen_test(async)]
async fn mounted_ui_public_widgets_do_not_mount_before_auth_loadedness() {
    wasm_support::clear_clerk();
    let clerk = wasm_support::install_pending_load_clerk();

    let mut dom = VirtualDom::new(AllPublicWidgetsTestApp);
    let host_ids = rebuild_and_capture_host_ids(&mut dom);
    assert_eq!(host_ids.len(), 4);
    for host_id in &host_ids {
        wasm_support::append_mount_element_with_id(host_id);
    }

    wasm_support::settle_ticks(&mut dom, 5).await;

    assert_eq!(wasm_support::number_prop(&clerk, "loadCallCount"), 1.0);
    assert_eq!(wasm_support::number_prop(&clerk, "mountCallCount"), 0.0);

    drop(dom);
    for host_id in &host_ids {
        wasm_support::remove_element_by_id(host_id);
    }
}

async fn drive_mounted_ui(
    dom: &mut VirtualDom,
    clerk: &wasm_bindgen::JsValue,
    expected_mount_count: f64,
) {
    wasm_support::settle_until(dom, || {
        wasm_support::number_prop(clerk, "mountCallCount") == expected_mount_count
    })
    .await;
}

async fn wait_for_unmount_count(clerk: &wasm_bindgen::JsValue, expected_unmount_count: f64) {
    for _ in 0..5 {
        if wasm_support::number_prop(clerk, "unmountCallCount") == expected_unmount_count {
            break;
        }
        TimeoutFuture::new(25).await;
    }
}

fn rebuild_and_capture_host_id(dom: &mut VirtualDom) -> String {
    let mut host_ids = rebuild_and_capture_host_ids(dom);
    assert_eq!(host_ids.len(), 1);
    host_ids.remove(0)
}

fn rebuild_and_capture_host_ids(dom: &mut VirtualDom) -> Vec<String> {
    let mut mutations = Mutations::default();
    dom.rebuild(&mut mutations);
    let host_ids = host_ids_from_mutations(&mutations);
    assert!(
        !host_ids.is_empty(),
        "public widget should create a host id"
    );
    dom.render_immediate(&mut NoOpMutations);
    host_ids
}

fn host_ids_from_mutations(mutations: &Mutations) -> Vec<String> {
    mutations
        .edits
        .iter()
        .filter_map(|edit| {
            let Mutation::SetAttribute {
                name,
                value: AttributeValue::Text(value),
                ..
            } = edit
            else {
                return None;
            };
            (*name == "id" && !value.is_empty()).then(|| value.clone())
        })
        .collect()
}

fn assert_js_string_prop(options: &wasm_bindgen::JsValue, key: &str, expected: &str) {
    assert_eq!(
        js_sys::Reflect::get(options, &wasm_bindgen::JsValue::from_str(key))
            .unwrap()
            .as_string()
            .as_deref(),
        Some(expected)
    );
}

fn assert_mounted_host_ids(clerk: &wasm_bindgen::JsValue, host_ids: &[String]) {
    let mounted_ids = wasm_support::mounted_element_ids(clerk);
    assert_eq!(mounted_ids.len(), host_ids.len());
    for host_id in host_ids {
        assert!(
            mounted_ids.contains(host_id),
            "mounted UI used unexpected host ids: {mounted_ids:?}"
        );
    }
}

fn assert_unmounted_host_ids(clerk: &wasm_bindgen::JsValue, host_ids: &[String]) {
    let unmounted_ids = wasm_support::unmounted_element_ids(clerk);
    assert_eq!(unmounted_ids.len(), host_ids.len());
    for host_id in host_ids {
        assert!(
            unmounted_ids.contains(host_id),
            "mounted UI cleaned up unexpected host ids: {unmounted_ids:?}"
        );
    }
}

fn assert_mounted_component_options(
    clerk: &wasm_bindgen::JsValue,
    expected_components: impl IntoIterator<Item = &'static str>,
) {
    let options = wasm_support::mounted_options(clerk);
    let mut components = options
        .iter()
        .filter_map(|options| {
            js_sys::Reflect::get(options, &wasm_bindgen::JsValue::from_str("component"))
                .unwrap()
                .as_string()
        })
        .collect::<Vec<_>>();
    components.sort();

    let mut expected = expected_components
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();
    expected.sort();

    assert_eq!(components, expected);
    for options in options {
        assert_js_string_prop(&options, "routing", "hash");
    }
}

fn set_test_options(options: serde_json::Value) {
    TEST_OPTIONS.with(|current| *current.borrow_mut() = options);
}

fn reset_provider_errors() {
    PROVIDER_ERRORS.with(|errors| errors.borrow_mut().clear());
}

fn provider_saw_error() -> bool {
    PROVIDER_ERRORS.with(|errors| !errors.borrow().is_empty())
}

fn provider_error_contains(needle: &str) -> bool {
    PROVIDER_ERRORS.with(|errors| errors.borrow().iter().any(|error| error.contains(needle)))
}

fn mounted_options(component: &str) -> serde_json::Value {
    serde_json::json!({
        "routing": "hash",
        "component": component,
    })
}

#[component]
fn UpdatingSignInTestApp() -> Element {
    let mut version = use_signal(|| "first".to_string());
    let did_schedule = use_signal(|| false);

    use_effect(move || {
        let mut did_schedule = did_schedule;
        if *did_schedule.read() {
            return;
        }
        did_schedule.set(true);
        spawn(async move {
            TimeoutFuture::new(75).await;
            version.set("second".to_string());
        });
    });

    let current_version = version.read().clone();
    let options = serde_json::json!({ "version": current_version });

    rsx! {
        dioxus_clerk::ClerkProvider { publishable_key: Some("pk_test_mounted".to_string()),
            dioxus_clerk::SignIn { options }
            ProviderErrorProbe {}
        }
    }
}

#[component]
fn SignInTestApp() -> Element {
    let options = TEST_OPTIONS.with(|current| current.borrow().clone());

    rsx! {
        dioxus_clerk::ClerkProvider { publishable_key: Some("pk_test_mounted".to_string()),
            dioxus_clerk::SignIn { options }
            ProviderErrorProbe {}
        }
    }
}

#[component]
fn AllPublicWidgetsTestApp() -> Element {
    let sign_in_options = mounted_options("SignIn");
    let sign_up_options = mounted_options("SignUp");
    let user_button_options = mounted_options("UserButton");
    let user_profile_options = mounted_options("UserProfile");

    rsx! {
        dioxus_clerk::ClerkProvider { publishable_key: Some("pk_test_mounted".to_string()),
            dioxus_clerk::SignIn { options: sign_in_options }
            dioxus_clerk::SignUp { options: sign_up_options }
            dioxus_clerk::UserButton { options: user_button_options }
            dioxus_clerk::UserProfile { options: user_profile_options }
        }
    }
}

#[component]
fn ProviderErrorProbe() -> Element {
    let error = dioxus_clerk::use_clerk_error();
    if let Some(error) = error.read().as_ref() {
        PROVIDER_ERRORS.with(|errors| errors.borrow_mut().push(format!("{error}")));
    }
    rsx! {}
}
