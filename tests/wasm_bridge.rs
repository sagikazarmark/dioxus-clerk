#![cfg(target_arch = "wasm32")]

mod wasm_support;

use dioxus::dioxus_core::NoOpMutations;
use dioxus::prelude::*;
use js_sys::Reflect;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

thread_local! {
    static ACTION_ERRORS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

#[wasm_bindgen_test(async)]
async fn use_clerk_open_sign_in_waits_for_loadedness_and_forwards_options() {
    wasm_support::clear_clerk();
    let clerk = wasm_support::install_clerk_mock(false);

    let mut dom = VirtualDom::new(OpenSignInActionApp);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    wasm_support::settle_until(&mut dom, || {
        wasm_support::number_prop(&clerk, "openSignInCallCount") == 1.0
    })
    .await;

    assert_eq!(
        wasm_support::number_prop(&clerk, "openSignInCallCount"),
        1.0
    );
    let options = wasm_support::last_open_sign_in_options(&clerk);
    assert_eq!(
        Reflect::get(&options, &JsValue::from_str("routing"))
            .unwrap()
            .as_string()
            .as_deref(),
        Some("hash")
    );
}

#[wasm_bindgen_test(async)]
async fn use_clerk_open_sign_in_does_not_touch_bridge_before_loadedness() {
    wasm_support::clear_clerk();
    let clerk = wasm_support::install_pending_load_clerk();

    let mut dom = VirtualDom::new(OpenSignInActionApp);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    wasm_support::settle_ticks(&mut dom, 5).await;

    assert_eq!(wasm_support::number_prop(&clerk, "loadCallCount"), 1.0);
    assert_eq!(
        wasm_support::number_prop(&clerk, "openSignInCallCount"),
        0.0
    );
}

#[wasm_bindgen_test(async)]
async fn use_clerk_open_sign_in_converts_null_options_to_undefined() {
    wasm_support::clear_clerk();
    let clerk = wasm_support::install_clerk_mock(false);

    let mut dom = VirtualDom::new(OpenSignInNullActionApp);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    wasm_support::settle_until(&mut dom, || {
        wasm_support::number_prop(&clerk, "openSignInCallCount") == 1.0
    })
    .await;

    assert_eq!(
        wasm_support::number_prop(&clerk, "openSignInCallCount"),
        1.0
    );
    assert!(wasm_support::last_open_sign_in_options(&clerk).is_undefined());
}

#[wasm_bindgen_test(async)]
async fn use_clerk_sign_out_runs_after_loadedness() {
    wasm_support::clear_clerk();
    let clerk = wasm_support::install_clerk_mock(false);

    let mut dom = VirtualDom::new(SignOutActionApp);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    wasm_support::settle_until(&mut dom, || {
        wasm_support::number_prop(&clerk, "signOutCallCount") == 1.0
    })
    .await;

    assert_eq!(wasm_support::number_prop(&clerk, "signOutCallCount"), 1.0);
}

#[wasm_bindgen_test(async)]
async fn use_clerk_sign_out_errors_are_read_through_error_hook() {
    wasm_support::clear_clerk();
    reset_action_errors();
    let clerk = wasm_support::install_clerk_mock(false);
    wasm_support::make_sign_out_reject(&clerk, "sign out failed");

    let mut dom = VirtualDom::new(SignOutErrorActionApp);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    assert!(
        wasm_support::settle_until(&mut dom, || action_error_contains("sign out failed")).await,
        "use_clerk sign_out error was not surfaced through use_clerk_error"
    );
}

#[component]
fn OpenSignInActionApp() -> Element {
    rsx! {
        dioxus_clerk::ClerkProvider { publishable_key: Some("pk_test_actions".to_string()),
            OpenSignInActionProbe {}
        }
    }
}

#[component]
fn SignOutActionApp() -> Element {
    rsx! {
        dioxus_clerk::ClerkProvider { publishable_key: Some("pk_test_actions".to_string()),
            SignOutActionProbe {}
        }
    }
}

#[component]
fn OpenSignInNullActionApp() -> Element {
    rsx! {
        dioxus_clerk::ClerkProvider { publishable_key: Some("pk_test_actions".to_string()),
            OpenSignInNullActionProbe {}
        }
    }
}

#[component]
fn SignOutErrorActionApp() -> Element {
    rsx! {
        dioxus_clerk::ClerkProvider { publishable_key: Some("pk_test_actions".to_string()),
            SignOutActionProbe {}
            ErrorProbe {}
        }
    }
}

#[component]
fn OpenSignInActionProbe() -> Element {
    let clerk = dioxus_clerk::use_clerk();
    let did_schedule = use_signal(|| false);

    use_effect(move || {
        let mut did_schedule = did_schedule;
        if *did_schedule.read() {
            return;
        }
        did_schedule.set(true);
        clerk.open_sign_in_with_options(serde_json::json!({ "routing": "hash" }));
    });

    rsx! {}
}

#[component]
fn OpenSignInNullActionProbe() -> Element {
    let clerk = dioxus_clerk::use_clerk();
    let did_schedule = use_signal(|| false);

    use_effect(move || {
        let mut did_schedule = did_schedule;
        if *did_schedule.read() {
            return;
        }
        did_schedule.set(true);
        clerk.open_sign_in();
    });

    rsx! {}
}

#[component]
fn SignOutActionProbe() -> Element {
    let clerk = dioxus_clerk::use_clerk();
    let did_schedule = use_signal(|| false);

    use_effect(move || {
        let mut did_schedule = did_schedule;
        if *did_schedule.read() {
            return;
        }
        did_schedule.set(true);
        clerk.sign_out();
    });

    rsx! {}
}

#[component]
fn ErrorProbe() -> Element {
    let error = dioxus_clerk::use_clerk_error();
    if let Some(error) = error.read().as_ref() {
        ACTION_ERRORS.with(|errors| errors.borrow_mut().push(format!("{error}")));
    }
    rsx! {}
}

fn reset_action_errors() {
    ACTION_ERRORS.with(|errors| errors.borrow_mut().clear());
}

fn action_error_contains(needle: &str) -> bool {
    ACTION_ERRORS.with(|errors| errors.borrow().iter().any(|error| error.contains(needle)))
}
