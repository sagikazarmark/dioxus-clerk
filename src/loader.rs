//! Inject the clerk-js script tag once per page.

use std::cell::RefCell;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{HtmlScriptElement, window};

/// Major clerk-js version this crate targets. The default script URL pins the
/// major so patch/minor updates flow in without a breaking jump.
const CLERK_JS_MAJOR: &str = "6";
/// Major `@clerk/ui` version this crate targets. clerk-js 6 no longer bundles
/// UI components; they ship in the separately-versioned `@clerk/ui` package
/// (`Clerk.uiVersion`), which must be loaded alongside clerk-js and handed to
/// `Clerk.load({ ui: { ClerkUI } })`.
const CLERK_UI_MAJOR: &str = "1";
const SCRIPT_ID: &str = "__dioxus_clerk_js";

/// Fallback clerk-js CDN, used only when the publishable key cannot be decoded
/// into a Frontend API host. Derived from [`CLERK_JS_MAJOR`] so it can't drift
/// from the Frontend-API-hosted URL. The preferred source is the instance's own
/// Frontend API host (see [`default_script_url`]).
fn fallback_cdn_url() -> String {
    format!("https://cdn.jsdelivr.net/npm/@clerk/clerk-js@{CLERK_JS_MAJOR}/dist/clerk.browser.js")
}

/// Fallback `@clerk/ui` CDN (mirrors [`fallback_cdn_url`]), derived from
/// [`CLERK_UI_MAJOR`].
fn fallback_ui_url() -> String {
    format!("https://cdn.jsdelivr.net/npm/@clerk/ui@{CLERK_UI_MAJOR}/dist/ui.browser.js")
}
const UI_SCRIPT_ID: &str = "__dioxus_clerk_ui";

/// The clerk-js script URL to inject for a publishable key.
///
/// Derives the instance's Frontend API host from the key and serves clerk-js
/// from it — matching Clerk's own loaders, so the script stays version-matched
/// with the backend it calls. Falls back to a pinned public CDN only when the
/// key cannot be decoded.
fn default_script_url(publishable_key: &str) -> String {
    match crate::publishable_key::frontend_api_host(publishable_key) {
        Some(host) => {
            format!("https://{host}/npm/@clerk/clerk-js@{CLERK_JS_MAJOR}/dist/clerk.browser.js")
        }
        None => fallback_cdn_url(),
    }
}

/// The `@clerk/ui` script URL to inject for a publishable key.
///
/// Served from the same Frontend API host as clerk-js (see
/// [`default_script_url`]) so the two bundles stay version-matched, with the
/// same pinned-public-CDN fallback when the key cannot be decoded.
fn default_ui_url(publishable_key: &str) -> String {
    match crate::publishable_key::frontend_api_host(publishable_key) {
        Some(host) => format!("https://{host}/npm/@clerk/ui@{CLERK_UI_MAJOR}/dist/ui.browser.js"),
        None => fallback_ui_url(),
    }
}

thread_local! {
    /// Set by the injected script's `error` event so the load loop can fail
    /// fast instead of waiting out the full poll timeout.
    static SCRIPT_LOAD_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// The failure message recorded by the injected script's `error` event, if any.
pub(crate) fn script_load_error() -> Option<String> {
    SCRIPT_LOAD_ERROR.with(|slot| slot.borrow().clone())
}

/// Whether the `@clerk/ui` script tag from [`inject_script`] is in the document
/// — i.e. this crate is loading the UI bundle and the load flow must wait for
/// its constructor before calling `Clerk.load()`. False when a live
/// `window.Clerk` was already present and injection was skipped.
pub(crate) fn ui_script_injected() -> bool {
    window()
        .and_then(|w| w.document())
        .and_then(|doc| doc.get_element_by_id(UI_SCRIPT_ID))
        .is_some()
}

/// Browser script injection settings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ScriptOptions {
    pub(crate) url: Option<String>,
    pub(crate) ui_url: Option<String>,
    pub(crate) nonce: Option<String>,
}

/// Inject the clerk-js and `@clerk/ui` script tags unless they were already
/// injected or `window.Clerk` already exists (clerk-js loaded externally).
/// Returns true if it added fresh tags.
///
/// clerk-js 6 no longer bundles UI components, so the `@clerk/ui` bundle is
/// injected alongside clerk-js; the load flow then hands its constructor to
/// `Clerk.load({ ui: { ClerkUI } })` (see [`crate::bindings::clerk_ui_ctor`]).
/// The UI tag is appended first so its global is more likely present by the
/// time clerk-js finishes, though the load flow also waits for the constructor.
///
/// Tags left behind by a failed earlier load do not block injection: the dead
/// tags are removed and the recorded error cleared, so a provider remount can
/// retry after a transient CDN/network failure instead of failing instantly
/// until a full page reload.
pub(crate) fn inject_script(publishable_key: &str, options: &ScriptOptions) -> bool {
    let Some(window) = window() else { return false };
    let Some(doc) = window.document() else {
        return false;
    };
    if doc.get_element_by_id(SCRIPT_ID).is_some() || doc.get_element_by_id(UI_SCRIPT_ID).is_some() {
        if script_load_error().is_none() {
            return false;
        }
        if let Some(existing) = doc.get_element_by_id(SCRIPT_ID) {
            existing.remove();
        }
        if let Some(existing) = doc.get_element_by_id(UI_SCRIPT_ID) {
            existing.remove();
        }
        SCRIPT_LOAD_ERROR.with(|slot| *slot.borrow_mut() = None);
    }
    // A second copy of clerk-js would re-execute and replace `window.Clerk`,
    // racing anything already holding the first instance.
    if crate::bindings::clerk_singleton().is_some() {
        return false;
    }

    let ui_url = options
        .ui_url
        .clone()
        .unwrap_or_else(|| default_ui_url(publishable_key));
    let clerk_url = options
        .url
        .clone()
        .unwrap_or_else(|| default_script_url(publishable_key));

    let ui_ok = inject_one(
        &doc,
        UI_SCRIPT_ID,
        &ui_url,
        None,
        options.nonce.as_deref(),
        "@clerk/ui",
    );
    let clerk_ok = inject_one(
        &doc,
        SCRIPT_ID,
        &clerk_url,
        Some(publishable_key),
        options.nonce.as_deref(),
        "clerk-js",
    );
    ui_ok && clerk_ok
}

/// Create one script tag, wire its fail-fast error handler, and append it to
/// `<head>`. `publishable_key` is set as `data-clerk-publishable-key` only on
/// the clerk-js tag. Returns true on success.
fn inject_one(
    doc: &web_sys::Document,
    id: &str,
    url: &str,
    publishable_key: Option<&str>,
    nonce: Option<&str>,
    label: &str,
) -> bool {
    let Some(script) = doc
        .create_element("script")
        .ok()
        .and_then(|element| element.dyn_into::<HtmlScriptElement>().ok())
    else {
        return false;
    };
    script.set_id(id);
    script.set_src(url);
    if let Some(key) = publishable_key {
        script.set_attribute("data-clerk-publishable-key", key).ok();
    }
    if let Some(nonce) = nonce {
        script.set_attribute("nonce", nonce).ok();
    }
    script.set_attribute("crossorigin", "anonymous").ok();

    let error_url = url.to_owned();
    let label = label.to_owned();
    let onerror = Closure::<dyn FnMut(JsValue)>::new(move |_event: JsValue| {
        SCRIPT_LOAD_ERROR.with(|slot| {
            *slot.borrow_mut() = Some(format!(
                "the {label} script at {error_url} failed to load; check the network tab for DNS/CSP/offline failures"
            ));
        });
    });
    script.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    // The handler must outlive this function; it fires at most once per
    // injected tag (a retry after a failed load leaks one handler per
    // attempt, which is negligible next to the page reload it replaces).
    onerror.forget();

    match doc.head() {
        Some(head) => head.append_child(&script).is_ok(),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use js_sys::{Function, Object, Reflect};
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    // A URL that can never load, so tests exercise injection without pulling
    // real clerk-js into the harness browser.
    const UNREACHABLE_URL: &str = "https://clerk.invalid/clerk.browser.js";

    const UNREACHABLE_UI_URL: &str = "https://clerk.invalid/ui.browser.js";

    fn cleanup() {
        let window = window().expect("wasm tests run in a browser window");
        if let Some(document) = window.document() {
            for id in [SCRIPT_ID, UI_SCRIPT_ID] {
                if let Some(script) = document.get_element_by_id(id) {
                    script.remove();
                }
            }
        }
        Reflect::set(
            window.as_ref(),
            &JsValue::from_str("Clerk"),
            &JsValue::UNDEFINED,
        )
        .unwrap();
        // Reset the recorded script error too: it is a thread-local shared
        // across tests, and a stale error left by the failed-load tests would
        // otherwise push a later injection down the retry path (re-injecting
        // instead of returning false), making the suite order-dependent.
        SCRIPT_LOAD_ERROR.with(|slot| *slot.borrow_mut() = None);
    }

    fn unreachable_options() -> ScriptOptions {
        ScriptOptions {
            url: Some(UNREACHABLE_URL.into()),
            ui_url: Some(UNREACHABLE_UI_URL.into()),
            nonce: None,
        }
    }

    #[wasm_bindgen_test]
    fn inject_script_adds_one_tag_per_page() {
        cleanup();

        assert!(inject_script("pk_test_loader", &unreachable_options()));
        assert!(
            !inject_script("pk_test_loader", &unreachable_options()),
            "a second injection must not add another clerk-js copy"
        );

        cleanup();
    }

    #[wasm_bindgen_test]
    fn inject_script_sets_source_and_publishable_key() {
        cleanup();

        assert!(inject_script("pk_test_loader", &unreachable_options()));

        let document = window().unwrap().document().unwrap();
        let script = document
            .get_element_by_id(SCRIPT_ID)
            .expect("script tag was injected");
        assert_eq!(
            script
                .get_attribute("data-clerk-publishable-key")
                .as_deref(),
            Some("pk_test_loader")
        );
        assert_eq!(
            script.get_attribute("src").as_deref(),
            Some(UNREACHABLE_URL)
        );
        assert_eq!(
            script.get_attribute("crossorigin").as_deref(),
            Some("anonymous")
        );

        cleanup();
    }

    #[wasm_bindgen_test]
    fn inject_script_adds_clerk_ui_bundle() {
        cleanup();

        assert!(inject_script("pk_test_loader", &unreachable_options()));

        let document = window().unwrap().document().unwrap();
        let ui = document
            .get_element_by_id(UI_SCRIPT_ID)
            .expect("the @clerk/ui script tag was injected");
        assert_eq!(ui.get_attribute("src").as_deref(), Some(UNREACHABLE_UI_URL));
        assert_eq!(
            ui.get_attribute("crossorigin").as_deref(),
            Some("anonymous")
        );
        // The publishable key belongs on the clerk-js tag only.
        assert!(ui.get_attribute("data-clerk-publishable-key").is_none());

        cleanup();
    }

    #[wasm_bindgen_test]
    fn inject_script_skips_when_clerk_global_already_exists() {
        cleanup();

        let clerk = Object::new();
        Reflect::set(
            clerk.as_ref(),
            &JsValue::from_str("load"),
            Function::new_no_args("return Promise.resolve();").as_ref(),
        )
        .unwrap();
        Reflect::set(
            window().unwrap().as_ref(),
            &JsValue::from_str("Clerk"),
            clerk.as_ref(),
        )
        .unwrap();

        assert!(
            !inject_script("pk_test_loader", &unreachable_options()),
            "a second clerk-js copy would replace the live window.Clerk"
        );

        cleanup();
    }

    #[wasm_bindgen_test(async)]
    async fn failed_script_load_records_fail_fast_error() {
        cleanup();

        assert!(inject_script("pk_test_loader", &unreachable_options()));

        for _ in 0..200 {
            if script_load_error().is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(25).await;
        }

        let message = script_load_error().expect("script error handler records a failure");
        assert!(message.contains("failed to load"));

        cleanup();
    }

    #[wasm_bindgen_test(async)]
    async fn failed_script_load_allows_reinjection_retry() {
        cleanup();

        assert!(inject_script("pk_test_loader", &unreachable_options()));

        for _ in 0..200 {
            if script_load_error().is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(25).await;
        }
        assert!(script_load_error().is_some());

        // The dead tag and recorded error must not poison the page: a
        // provider remount injects a fresh tag and clears the error.
        assert!(
            inject_script("pk_test_loader", &unreachable_options()),
            "retry after a failed load must inject a fresh tag"
        );
        assert!(
            script_load_error().is_none(),
            "retry must clear the recorded load error"
        );

        cleanup();
    }
}
