#![cfg(target_arch = "wasm32")]

mod wasm_support;

use dioxus::dioxus_core::NoOpMutations;
use dioxus::prelude::*;
use dioxus_clerk::core::AuthState;
use dioxus_clerk::ssr::{INITIAL_STATE_SCRIPT_ID, InitialAuthSnapshot, InitialState};
use js_sys::Reflect;
use std::cell::RefCell;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

thread_local! {
    static PROVIDER_SNAPSHOTS: RefCell<Vec<AuthState>> = const { RefCell::new(Vec::new()) };
    static PROVIDER_ERRORS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static PUSHED_ROUTES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static RECEIVED_TOKENS: RefCell<Vec<Option<String>>> = const { RefCell::new(Vec::new()) };
    static RENDERED_TAGS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
    static HYDRATED_SNAPSHOTS: RefCell<Vec<(dioxus_clerk::core::User, dioxus_clerk::core::Session)>> =
        const { RefCell::new(Vec::new()) };
}

static SHOW_PROVIDER: GlobalSignal<bool> = Signal::global(|| true);
static ROUTER_VARIANT: GlobalSignal<u8> = Signal::global(|| 0);

/// Writing a `GlobalSignal` requires a current dioxus runtime; the test body
/// has none, so route the write through the VirtualDom's runtime. Call after
/// `VirtualDom::new` (which creates the runtime) — for the initial value,
/// before `rebuild_in_place`.
fn set_show_provider(dom: &VirtualDom, value: bool) {
    dom.in_runtime(|| *SHOW_PROVIDER.write() = value);
}

fn set_router_variant(dom: &VirtualDom, value: u8) {
    dom.in_runtime(|| *ROUTER_VARIANT.write() = value);
}

#[wasm_bindgen_test(async)]
async fn clerk_provider_polling_observes_delayed_window_clerk() {
    wasm_support::clear_clerk();
    reset_provider_snapshots();

    let mut dom = VirtualDom::new(ProviderPollingApp);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    install_delayed_mock_clerk(25);

    assert!(
        wasm_support::settle_until(&mut dom, provider_saw_loaded_signed_in).await,
        "ClerkProvider did not progress to a loaded signed-in auth snapshot"
    );
}

#[wasm_bindgen_test(async)]
async fn clerk_provider_surfaces_load_rejection_through_context() {
    wasm_support::clear_clerk();
    reset_provider_errors();
    wasm_support::install_rejecting_clerk();

    let mut dom = VirtualDom::new(ProviderErrorApp);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    assert!(
        wasm_support::settle_until(&mut dom, provider_saw_error).await,
        "ClerkProvider did not surface load rejection through context error"
    );
}

#[wasm_bindgen_test(async)]
async fn clerk_provider_listener_updates_auth_state_after_load() {
    wasm_support::clear_clerk();
    reset_provider_snapshots();
    let clerk = wasm_support::install_clerk_mock(false);

    let mut dom = VirtualDom::new(ProviderPollingApp);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    // Wait for the load to fully settle: `addListener` (which sets
    // `lastListener`) runs in the load task's Ok branch after the promise
    // resolves, so keying on `loadCallCount` alone would race the listener.
    wasm_support::settle_until(&mut dom, || {
        wasm_support::number_prop(&clerk, "loadCallCount") == 1.0
            && wasm_support::has_function(&clerk, "lastListener")
    })
    .await;

    assert!(!provider_saw_loaded_signed_in());

    wasm_support::set_prop(&clerk, "isSignedIn", &JsValue::TRUE);
    wasm_support::trigger_listener(&clerk);

    assert!(
        wasm_support::settle_until(&mut dom, provider_saw_loaded_signed_in).await,
        "ClerkProvider listener did not update Auth state to signed in"
    );
}

#[wasm_bindgen_test(async)]
async fn clerk_provider_forwards_load_options_to_clerk_js() {
    wasm_support::clear_clerk();
    let clerk = wasm_support::install_clerk_mock(false);

    let mut dom = VirtualDom::new(ProviderLoadOptionsApp);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    wasm_support::settle_until(&mut dom, || {
        wasm_support::number_prop(&clerk, "loadCallCount") == 1.0
    })
    .await;

    let options = wasm_support::last_load_options(&clerk);
    assert_js_string_prop(&options, "signInFallbackRedirectUrl", "/dashboard");
    assert_js_string_prop(&options, "afterSignInUrl", "/raw-dashboard");
    assert_js_string_prop(&options, "signInUrl", "/sign-in");
}

#[wasm_bindgen_test(async)]
async fn clerk_provider_converts_default_load_options_to_undefined() {
    wasm_support::clear_clerk();
    let clerk = wasm_support::install_clerk_mock(false);

    let mut dom = VirtualDom::new(ProviderPollingApp);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    wasm_support::settle_until(&mut dom, || {
        wasm_support::number_prop(&clerk, "loadCallCount") == 1.0
    })
    .await;

    assert!(wasm_support::last_load_options(&clerk).is_undefined());
}

#[wasm_bindgen_test(async)]
async fn clerk_provider_preserves_initial_signed_in_status_during_transient_loading_observation() {
    wasm_support::clear_clerk();
    reset_provider_snapshots();
    install_signed_in_initial_state();
    let clerk = wasm_support::install_clerk_mock(true);
    wasm_support::set_prop(clerk.as_ref(), "user", &JsValue::UNDEFINED);
    wasm_support::set_prop(clerk.as_ref(), "session", &JsValue::UNDEFINED);

    let mut dom = VirtualDom::new(ProviderPollingApp);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    wasm_support::settle_until(&mut dom, provider_saw_loaded_signed_in).await;

    assert!(provider_saw_initial_signed_in());
    assert!(provider_saw_loaded_signed_in());
    assert!(!provider_saw_loaded_signed_out());
}

#[wasm_bindgen_test(async)]
async fn clerk_provider_reports_missing_publishable_key_without_external_clerk() {
    wasm_support::clear_clerk();
    reset_provider_errors();

    let mut dom = VirtualDom::new(ProviderMissingKeyApp);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    assert!(
        wasm_support::settle_until(&mut dom, provider_saw_publishable_key_error).await,
        "ClerkProvider did not surface missing publishable key error"
    );
}

#[wasm_bindgen_test(async)]
async fn redirect_to_sign_in_does_not_watch_auth_after_loaded() {
    wasm_support::clear_clerk();
    let clerk = wasm_support::install_clerk_mock(true);

    let mut dom = VirtualDom::new(RedirectApp);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    // Wait for the listener to be registered (`addListener` runs after the load
    // promise resolves), not just for `loadCallCount`, before triggering it.
    wasm_support::settle_until(&mut dom, || {
        wasm_support::number_prop(&clerk, "loadCallCount") == 1.0
            && wasm_support::has_function(&clerk, "lastListener")
    })
    .await;

    assert_eq!(wasm_support::number_prop(&clerk, "redirectCallCount"), 0.0);

    wasm_support::set_prop(clerk.as_ref(), "isSignedIn", &JsValue::FALSE);
    wasm_support::trigger_listener(&clerk);

    wasm_support::settle_ticks(&mut dom, 5).await;

    assert_eq!(wasm_support::number_prop(&clerk, "redirectCallCount"), 0.0);
}

#[wasm_bindgen_test(async)]
async fn redirect_to_sign_in_runs_once_after_loadedness() {
    wasm_support::clear_clerk();
    let clerk = wasm_support::install_clerk_mock(false);

    let mut dom = VirtualDom::new(RedirectApp);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    wasm_support::settle_until(&mut dom, || {
        wasm_support::number_prop(&clerk, "redirectCallCount") == 1.0
    })
    .await;

    assert_eq!(wasm_support::number_prop(&clerk, "redirectCallCount"), 1.0);

    wasm_support::trigger_listener(&clerk);

    wasm_support::settle_ticks(&mut dom, 5).await;

    assert_eq!(wasm_support::number_prop(&clerk, "redirectCallCount"), 1.0);
}

#[wasm_bindgen_test(async)]
async fn redirect_to_sign_in_forwards_options_after_loadedness() {
    wasm_support::clear_clerk();
    let clerk = wasm_support::install_clerk_mock(false);

    let mut dom = VirtualDom::new(RedirectOptionsApp);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    wasm_support::settle_until(&mut dom, || {
        wasm_support::number_prop(&clerk, "redirectCallCount") == 1.0
    })
    .await;

    assert_eq!(wasm_support::number_prop(&clerk, "redirectCallCount"), 1.0);

    let options = wasm_support::last_redirect_options(&clerk);
    assert_js_string_prop(&options, "redirectUrl", "/dashboard");
    assert_js_string_prop(&options, "signUpUrl", "/signup");
}

#[wasm_bindgen_test(async)]
async fn redirect_to_sign_in_surfaces_bridge_errors_through_provider_context() {
    wasm_support::clear_clerk();
    reset_provider_errors();
    let clerk = wasm_support::install_clerk_mock(false);
    wasm_support::make_redirect_throw(&clerk, "redirect failed");

    let mut dom = VirtualDom::new(RedirectErrorApp);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    assert!(
        wasm_support::settle_until(&mut dom, provider_saw_js_error).await,
        "RedirectToSignIn did not surface bridge error through provider context"
    );
}

#[wasm_bindgen_test(async)]
async fn redirect_to_sign_in_waits_for_auth_loadedness_before_calling_bridge() {
    wasm_support::clear_clerk();
    let clerk = wasm_support::install_pending_load_clerk();

    let mut dom = VirtualDom::new(RedirectApp);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    wasm_support::settle_ticks(&mut dom, 5).await;

    assert_eq!(wasm_support::number_prop(&clerk, "loadCallCount"), 1.0);
    assert_eq!(wasm_support::number_prop(&clerk, "redirectCallCount"), 0.0);
}

#[wasm_bindgen_test(async)]
async fn clerk_provider_remount_skips_second_clerk_load() {
    wasm_support::clear_clerk();
    reset_provider_snapshots();
    let clerk = wasm_support::install_clerk_mock(true);

    let mut dom = VirtualDom::new(RemountApp);
    set_show_provider(&dom, true);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    assert!(
        wasm_support::settle_until(&mut dom, || {
            wasm_support::number_prop(&clerk, "loadCallCount") == 1.0
                && provider_saw_loaded_signed_in()
        })
        .await,
        "condition not reached while pumping the VirtualDom"
    );

    // Unmount the provider, then mount a fresh instance.
    set_show_provider(&dom, false);
    wasm_support::settle_ticks(&mut dom, 3).await;
    reset_provider_snapshots();
    set_show_provider(&dom, true);
    assert!(
        wasm_support::settle_until(&mut dom, provider_saw_loaded_signed_in).await,
        "condition not reached while pumping the VirtualDom"
    );

    // Clerk.loaded was already true, so the remounted provider must not call
    // Clerk.load() a second time.
    assert_eq!(wasm_support::number_prop(&clerk, "loadCallCount"), 1.0);
}

#[wasm_bindgen_test(async)]
async fn clerk_provider_remount_after_failed_load_surfaces_error_without_reloading() {
    wasm_support::clear_clerk();
    reset_provider_errors();
    // Live singleton whose Clerk.load() rejects; load_clerk_js is false so the
    // provider drives this instance instead of injecting a fresh script.
    let clerk = wasm_support::install_rejecting_clerk();

    let mut dom = VirtualDom::new(RemountErrorApp);
    set_show_provider(&dom, true);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    // First mount attempts the load, which rejects and surfaces an error.
    assert!(
        wasm_support::settle_until(&mut dom, || {
            wasm_support::number_prop(&clerk, "loadCallCount") == 1.0 && provider_saw_error()
        })
        .await,
        "condition not reached while pumping the VirtualDom"
    );

    // Unmount, then remount a fresh provider over the same failed singleton.
    set_show_provider(&dom, false);
    wasm_support::settle_ticks(&mut dom, 3).await;
    reset_provider_errors();
    set_show_provider(&dom, true);

    // The remounted provider surfaces the recorded failure...
    assert!(
        wasm_support::settle_until(&mut dom, provider_saw_error).await,
        "condition not reached while pumping the VirtualDom"
    );

    // ...without issuing a second Clerk.load() on the already-failed singleton,
    // which clerk-js does not support.
    assert_eq!(wasm_support::number_prop(&clerk, "loadCallCount"), 1.0);
}

#[wasm_bindgen_test(async)]
async fn router_push_is_forwarded_and_survives_provider_unmount() {
    wasm_support::clear_clerk();
    PUSHED_ROUTES.with(|routes| routes.borrow_mut().clear());
    let clerk = wasm_support::install_clerk_mock(false);

    let mut dom = VirtualDom::new(RouterApp);
    set_show_provider(&dom, true);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    assert!(
        wasm_support::settle_until(&mut dom, || {
            wasm_support::number_prop(&clerk, "loadCallCount") == 1.0
        })
        .await,
        "condition not reached while pumping the VirtualDom"
    );

    let options = wasm_support::last_load_options(&clerk);
    let router_push: js_sys::Function = Reflect::get(&options, &JsValue::from_str("routerPush"))
        .unwrap()
        .dyn_into()
        .expect("routerPush is forwarded to Clerk.load as a function");

    router_push
        .call1(&JsValue::NULL, &JsValue::from_str("/first"))
        .unwrap();
    assert_eq!(
        PUSHED_ROUTES.with(|routes| routes.borrow().clone()),
        vec!["/first".to_string()]
    );

    // Unmount the provider; clerk-js still holds the JS closure. Calling it
    // now must be a no-op, not a call into a dropped scope-owned Callback.
    set_show_provider(&dom, false);
    wasm_support::settle_ticks(&mut dom, 3).await;

    router_push
        .call1(&JsValue::NULL, &JsValue::from_str("/after-unmount"))
        .unwrap();
    assert_eq!(
        PUSHED_ROUTES.with(|routes| routes.borrow().clone()),
        vec!["/first".to_string()]
    );

    // Remount: Clerk.load() is skipped (already loaded), but the closure from
    // the first load must route to the fresh provider's callback again.
    set_show_provider(&dom, true);
    wasm_support::settle_ticks(&mut dom, 3).await;
    assert_eq!(wasm_support::number_prop(&clerk, "loadCallCount"), 1.0);

    router_push
        .call1(&JsValue::NULL, &JsValue::from_str("/after-remount"))
        .unwrap();
    assert_eq!(
        PUSHED_ROUTES.with(|routes| routes.borrow().clone()),
        vec!["/first".to_string(), "/after-remount".to_string()]
    );
}

#[wasm_bindgen_test(async)]
async fn router_push_routes_to_replacement_provider_after_swap() {
    wasm_support::clear_clerk();
    PUSHED_ROUTES.with(|routes| routes.borrow_mut().clear());
    let clerk = wasm_support::install_clerk_mock(false);

    let mut dom = VirtualDom::new(RouterSwapApp);
    set_router_variant(&dom, 0);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    assert!(
        wasm_support::settle_until(&mut dom, || {
            wasm_support::number_prop(&clerk, "loadCallCount") == 1.0
        })
        .await,
        "condition not reached while pumping the VirtualDom"
    );

    let options = wasm_support::last_load_options(&clerk);
    let router_push: js_sys::Function = Reflect::get(&options, &JsValue::from_str("routerPush"))
        .unwrap()
        .dyn_into()
        .expect("routerPush is forwarded to Clerk.load as a function");

    router_push
        .call1(&JsValue::NULL, &JsValue::from_str("/one"))
        .unwrap();

    // Replace provider A with provider B in one update. Whichever order Dioxus
    // uses for mount-vs-drop, B ends up owning the router slots, so the JS
    // closure from the first Clerk.load() must route to B's callback — not
    // silently no-op because A's late use_drop wiped B's fresh callbacks.
    set_router_variant(&dom, 1);
    wasm_support::settle_ticks(&mut dom, 3).await;
    assert_eq!(wasm_support::number_prop(&clerk, "loadCallCount"), 1.0);

    router_push
        .call1(&JsValue::NULL, &JsValue::from_str("/two"))
        .unwrap();
    assert_eq!(
        PUSHED_ROUTES.with(|routes| routes.borrow().clone()),
        vec!["a:/one".to_string(), "b:/two".to_string()]
    );
}

#[wasm_bindgen_test(async)]
async fn redirect_to_sign_up_runs_once_and_forwards_options() {
    wasm_support::clear_clerk();
    let clerk = wasm_support::install_clerk_mock(false);

    let mut dom = VirtualDom::new(RedirectSignUpApp);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    assert!(
        wasm_support::settle_until(&mut dom, || {
            wasm_support::number_prop(&clerk, "redirectSignUpCallCount") == 1.0
        })
        .await,
        "condition not reached while pumping the VirtualDom"
    );

    let options = wasm_support::last_redirect_sign_up_options(&clerk);
    assert_js_string_prop(&options, "forceRedirectUrl", "/welcome");
    assert_js_string_prop(&options, "signInFallbackRedirectUrl", "/here");

    wasm_support::trigger_listener(&clerk);
    wasm_support::settle_ticks(&mut dom, 5).await;

    assert_eq!(
        wasm_support::number_prop(&clerk, "redirectSignUpCallCount"),
        1.0
    );
}

#[wasm_bindgen_test(async)]
async fn use_auth_get_token_returns_session_token_after_load() {
    wasm_support::clear_clerk();
    RECEIVED_TOKENS.with(|tokens| tokens.borrow_mut().clear());
    wasm_support::install_clerk_mock(true);

    let mut dom = VirtualDom::new(GetTokenApp);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    assert!(
        wasm_support::settle_until(&mut dom, || {
            RECEIVED_TOKENS.with(|tokens| !tokens.borrow().is_empty())
        })
        .await,
        "condition not reached while pumping the VirtualDom"
    );

    assert_eq!(
        RECEIVED_TOKENS.with(|tokens| tokens.borrow().clone()),
        vec![Some("session_token_2def".to_string())]
    );
}

#[wasm_bindgen_test(async)]
async fn use_auth_get_token_with_options_forwards_clerk_option_keys() {
    wasm_support::clear_clerk();
    RECEIVED_TOKENS.with(|tokens| tokens.borrow_mut().clear());
    let clerk = wasm_support::install_clerk_mock(true);

    let mut dom = VirtualDom::new(GetTokenOptionsApp);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    assert!(
        wasm_support::settle_until(&mut dom, || {
            RECEIVED_TOKENS.with(|tokens| !tokens.borrow().is_empty())
        })
        .await,
        "condition not reached while pumping the VirtualDom"
    );

    assert_eq!(
        RECEIVED_TOKENS.with(|tokens| tokens.borrow().clone()),
        vec![Some("session_token_2def".to_string())]
    );

    let session = wasm_support::get_prop(&clerk, "session");
    assert_eq!(
        wasm_support::number_prop(&session, "getTokenCallCount"),
        1.0
    );
    let options = wasm_support::get_prop(&session, "lastGetTokenOptions");
    assert_js_string_prop(&options, "template", "supabase");
    assert_js_string_prop(&options, "organizationId", "org_2ghi");
}

#[wasm_bindgen_test(async)]
async fn use_user_and_use_session_hydrate_clerk_js_fields_after_load() {
    wasm_support::clear_clerk();
    HYDRATED_SNAPSHOTS.with(|snapshots| snapshots.borrow_mut().clear());
    let clerk = wasm_support::install_clerk_mock(true);
    let user = wasm_support::get_prop(&clerk, "user");
    wasm_support::set_prop(&user, "firstName", &JsValue::from_str("Ada"));
    wasm_support::set_prop(
        &user,
        "imageUrl",
        &JsValue::from_str("https://img.clerk.test/ada.png"),
    );
    let session = wasm_support::get_prop(&clerk, "session");
    wasm_support::set_prop(
        &session,
        "lastActiveOrganizationId",
        &JsValue::from_str("org_2ghi"),
    );

    let mut dom = VirtualDom::new(HydrationApp);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    assert!(
        wasm_support::settle_until(&mut dom, || {
            HYDRATED_SNAPSHOTS.with(|snapshots| !snapshots.borrow().is_empty())
        })
        .await,
        "condition not reached while pumping the VirtualDom"
    );

    HYDRATED_SNAPSHOTS.with(|snapshots| {
        let (user, session) = snapshots.borrow().first().cloned().expect("hydrated pair");
        assert_eq!(user.id, "user_2abc");
        assert_eq!(user.first_name.as_deref(), Some("Ada"));
        assert_eq!(
            user.image_url.as_deref(),
            Some("https://img.clerk.test/ada.png")
        );
        assert_eq!(session.id, "sess_2def");
        assert!(session.is_active());
        assert_eq!(
            session.last_active_organization_id.as_deref(),
            Some("org_2ghi")
        );
    });
}

#[wasm_bindgen_test(async)]
async fn control_components_switch_from_loading_to_loaded_signed_out() {
    wasm_support::clear_clerk();
    RENDERED_TAGS.with(|tags| tags.borrow_mut().clear());
    wasm_support::install_clerk_mock(false);

    let mut dom = VirtualDom::new(ControlApp);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    assert!(saw_tag("loading"));
    assert!(saw_tag("checking-auth"));
    assert!(!saw_tag("loaded"));
    assert!(!saw_tag("signed-out"));

    assert!(
        wasm_support::settle_until(&mut dom, || saw_tag("loaded") && saw_tag("signed-out")).await,
        "condition not reached while pumping the VirtualDom"
    );

    assert!(!saw_tag("failed"));
}

#[wasm_bindgen_test(async)]
async fn control_components_render_failed_on_load_rejection() {
    wasm_support::clear_clerk();
    RENDERED_TAGS.with(|tags| tags.borrow_mut().clear());
    wasm_support::install_rejecting_clerk();

    let mut dom = VirtualDom::new(ControlApp);
    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    assert!(
        wasm_support::settle_until(&mut dom, || saw_tag("failed")).await,
        "condition not reached while pumping the VirtualDom"
    );

    // A load failure renders ClerkFailed; SignedOutWhenLoaded renders neither
    // children nor (after the error) its checking fallback.
    assert!(!saw_tag("loaded"));
    assert!(!saw_tag("signed-out"));
}

fn saw_tag(tag: &'static str) -> bool {
    RENDERED_TAGS.with(|tags| tags.borrow().contains(&tag))
}

#[component]
fn RemountApp() -> Element {
    rsx! {
        if SHOW_PROVIDER() {
            dioxus_clerk::ClerkProvider { publishable_key: "pk_test_remount", load_clerk_js: false,
                ProviderAuthProbe {}
            }
        }
    }
}

#[component]
fn RouterApp() -> Element {
    rsx! {
        if SHOW_PROVIDER() {
            dioxus_clerk::ClerkProvider {
                publishable_key: "pk_test_router",
                load_clerk_js: false,
                router_push: move |to: String| PUSHED_ROUTES.with(|routes| routes.borrow_mut().push(to)),
                ProviderAuthProbe {}
            }
        }
    }
}

#[component]
fn RouterSwapApp() -> Element {
    rsx! {
        if ROUTER_VARIANT() == 0 {
            dioxus_clerk::ClerkProvider {
                publishable_key: "pk_test_router_swap",
                load_clerk_js: false,
                router_push: move |to: String| {
                    PUSHED_ROUTES.with(|routes| routes.borrow_mut().push(format!("a:{to}")))
                },
                ProviderAuthProbe {}
            }
        } else {
            dioxus_clerk::ClerkProvider {
                publishable_key: "pk_test_router_swap",
                load_clerk_js: false,
                router_push: move |to: String| {
                    PUSHED_ROUTES.with(|routes| routes.borrow_mut().push(format!("b:{to}")))
                },
                ProviderAuthProbe {}
            }
        }
    }
}

#[component]
fn RedirectSignUpApp() -> Element {
    rsx! {
        dioxus_clerk::ClerkProvider { publishable_key: "pk_test_redirect_sign_up", load_clerk_js: false,
            dioxus_clerk::RedirectToSignUp {
                force_redirect_url: "/welcome",
                sign_in_fallback_redirect_url: "/here",
            }
        }
    }
}

#[component]
fn GetTokenApp() -> Element {
    rsx! {
        dioxus_clerk::ClerkProvider { publishable_key: "pk_test_get_token", load_clerk_js: false,
            GetTokenProbe {}
        }
    }
}

#[component]
fn GetTokenProbe() -> Element {
    let auth = dioxus_clerk::use_auth();
    use_hook(move || {
        spawn(async move {
            if let Ok(token) = auth.get_token().await {
                RECEIVED_TOKENS.with(|tokens| tokens.borrow_mut().push(token));
            }
        });
    });
    rsx! {}
}

#[component]
fn GetTokenOptionsApp() -> Element {
    rsx! {
        dioxus_clerk::ClerkProvider { publishable_key: "pk_test_get_token_options", load_clerk_js: false,
            GetTokenOptionsProbe {}
        }
    }
}

#[component]
fn GetTokenOptionsProbe() -> Element {
    let auth = dioxus_clerk::use_auth();
    use_hook(move || {
        spawn(async move {
            let options = dioxus_clerk::GetTokenOptions::new()
                .template("supabase")
                .organization_id("org_2ghi");
            if let Ok(token) = auth.get_token_with_options(options).await {
                RECEIVED_TOKENS.with(|tokens| tokens.borrow_mut().push(token));
            }
        });
    });
    rsx! {}
}

#[component]
fn HydrationApp() -> Element {
    rsx! {
        dioxus_clerk::ClerkProvider { publishable_key: "pk_test_hydration", load_clerk_js: false,
            HydrationProbe {}
        }
    }
}

#[component]
fn HydrationProbe() -> Element {
    let user = dioxus_clerk::use_user();
    let session = dioxus_clerk::use_session();
    if let (Some(user), Some(session)) = (user.user(), session.session()) {
        HYDRATED_SNAPSHOTS.with(|snapshots| snapshots.borrow_mut().push((user, session)));
    }
    rsx! {}
}

#[component]
fn ControlApp() -> Element {
    rsx! {
        dioxus_clerk::ClerkProvider { publishable_key: "pk_test_control", load_clerk_js: false,
            dioxus_clerk::ClerkLoading { RenderTag { tag: "loading" } }
            dioxus_clerk::ClerkLoaded { RenderTag { tag: "loaded" } }
            dioxus_clerk::ClerkFailed { RenderTag { tag: "failed" } }
            dioxus_clerk::SignedOutWhenLoaded {
                fallback: rsx! { RenderTag { tag: "checking-auth" } },
                RenderTag { tag: "signed-out" }
            }
        }
    }
}

#[component]
fn RenderTag(tag: &'static str) -> Element {
    RENDERED_TAGS.with(|tags| tags.borrow_mut().push(tag));
    rsx! {}
}

#[component]
fn ProviderPollingApp() -> Element {
    rsx! {
        dioxus_clerk::ClerkProvider { publishable_key: None, load_clerk_js: false,
            ProviderAuthProbe {}
        }
    }
}

#[component]
fn ProviderErrorApp() -> Element {
    rsx! {
        dioxus_clerk::ClerkProvider { publishable_key: Some("pk_test_error".to_string()),
            ProviderErrorProbe {}
        }
    }
}

#[component]
fn RemountErrorApp() -> Element {
    rsx! {
        if SHOW_PROVIDER() {
            dioxus_clerk::ClerkProvider { publishable_key: "pk_test_remount_error", load_clerk_js: false,
                ProviderErrorProbe {}
            }
        }
    }
}

#[component]
fn ProviderMissingKeyApp() -> Element {
    rsx! {
        dioxus_clerk::ClerkProvider { publishable_key: None,
            ProviderErrorProbe {}
        }
    }
}

#[component]
fn ProviderLoadOptionsApp() -> Element {
    rsx! {
        dioxus_clerk::ClerkProvider {
            publishable_key: Some("pk_test_options".to_string()),
            sign_in_url: "/sign-in",
            sign_in_fallback_redirect_url: "/dashboard",
            options: serde_json::json!({
                "afterSignInUrl": "/raw-dashboard",
            }),
            ProviderAuthProbe {}
        }
    }
}

#[component]
fn RedirectApp() -> Element {
    rsx! {
        dioxus_clerk::ClerkProvider { publishable_key: Some("pk_test_redirect".to_string()),
            dioxus_clerk::RedirectToSignIn {}
        }
    }
}

#[component]
fn RedirectOptionsApp() -> Element {
    rsx! {
        dioxus_clerk::ClerkProvider { publishable_key: Some("pk_test_redirect_options".to_string()),
            dioxus_clerk::RedirectToSignIn {
                options: serde_json::json!({
                    "redirectUrl": "/dashboard",
                    "signUpUrl": "/signup",
                })
            }
        }
    }
}

#[component]
fn RedirectErrorApp() -> Element {
    rsx! {
        dioxus_clerk::ClerkProvider { publishable_key: Some("pk_test_redirect_error".to_string()),
            dioxus_clerk::RedirectToSignIn {}
            ProviderErrorProbe {}
        }
    }
}

#[component]
fn ProviderAuthProbe() -> Element {
    let auth = dioxus_clerk::use_auth();
    PROVIDER_SNAPSHOTS.with(|snapshots| snapshots.borrow_mut().push(auth.state()));
    rsx! {}
}

#[component]
fn ProviderErrorProbe() -> Element {
    let error = dioxus_clerk::use_clerk_error();
    if let Some(error) = error.read().as_ref() {
        PROVIDER_ERRORS.with(|errors| errors.borrow_mut().push(format!("{error}")));
    }
    rsx! {}
}

fn reset_provider_snapshots() {
    PROVIDER_SNAPSHOTS.with(|snapshots| snapshots.borrow_mut().clear());
}

fn provider_saw_loaded_signed_in() -> bool {
    PROVIDER_SNAPSHOTS.with(|snapshots| {
        snapshots
            .borrow()
            .iter()
            .any(|snapshot| snapshot.is_loaded() && snapshot.is_signed_in())
    })
}

fn provider_saw_initial_signed_in() -> bool {
    PROVIDER_SNAPSHOTS.with(|snapshots| {
        snapshots
            .borrow()
            .iter()
            .any(|snapshot| !snapshot.is_loaded() && snapshot.is_signed_in())
    })
}

fn provider_saw_loaded_signed_out() -> bool {
    PROVIDER_SNAPSHOTS.with(|snapshots| {
        snapshots
            .borrow()
            .iter()
            .any(|snapshot| snapshot.is_loaded() && !snapshot.is_signed_in())
    })
}

fn reset_provider_errors() {
    PROVIDER_ERRORS.with(|errors| errors.borrow_mut().clear());
}

fn provider_saw_error() -> bool {
    PROVIDER_ERRORS.with(|errors| !errors.borrow().is_empty())
}

fn provider_saw_publishable_key_error() -> bool {
    PROVIDER_ERRORS.with(|errors| {
        errors
            .borrow()
            .iter()
            .any(|error| error.contains("publishable key"))
    })
}

fn provider_saw_js_error() -> bool {
    PROVIDER_ERRORS.with(|errors| {
        errors
            .borrow()
            .iter()
            .any(|error| error.contains("clerk js error"))
    })
}

fn assert_js_string_prop(options: &JsValue, key: &str, expected: &str) {
    assert_eq!(
        Reflect::get(options, &JsValue::from_str(key))
            .unwrap()
            .as_string()
            .as_deref(),
        Some(expected)
    );
}

fn install_delayed_mock_clerk(delay_ms: i32) {
    let callback = Closure::once_into_js(move || {
        wasm_support::install_clerk_mock(true);
    });
    wasm_support::window()
        .set_timeout_with_callback_and_timeout_and_arguments_0(callback.unchecked_ref(), delay_ms)
        .unwrap();
}

fn install_signed_in_initial_state() {
    let mut snapshot = InitialAuthSnapshot::signed_in("user_2abc");
    snapshot.session_id = Some("sess_2def".into());
    let initial_state = InitialState::new(snapshot, Some("pk_test_state"));
    let document = wasm_support::window().document().expect("document exists");
    let script = document.create_element("script").unwrap();
    script.set_id(INITIAL_STATE_SCRIPT_ID);
    script.set_attribute("type", "application/json").unwrap();
    script.set_text_content(Some(&serde_json::to_string(&initial_state).unwrap()));
    document
        .body()
        .expect("body exists")
        .append_child(&script)
        .unwrap();
}
