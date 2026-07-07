//! Client-side reader for the `<script id="__clerk_initial_state">` blob
//! emitted server-side as SSR initial state.

use crate::ssr::{
    INITIAL_STATE_SCRIPT_ID, InitialState, InitialStateRead, ProviderStartup,
    provider_startup_from_read,
};
use web_sys::window;

/// Read the browser document and interpret SSR initial state into startup facts.
pub(crate) fn startup(prop_publishable_key: Option<String>) -> ProviderStartup {
    provider_startup_from_read(read_initial_state(), prop_publishable_key)
}

/// Read and parse SSR initial state from the browser document.
pub(crate) fn read_initial_state() -> InitialStateRead {
    let Some(doc) = window().and_then(|window| window.document()) else {
        return InitialStateRead::Missing;
    };
    let Some(el) = doc.get_element_by_id(INITIAL_STATE_SCRIPT_ID) else {
        return InitialStateRead::Missing;
    };
    let Some(txt) = el.text_content() else {
        return InitialStateRead::Malformed("initial state script has no text content".into());
    };

    match serde_json::from_str::<InitialState>(&txt) {
        Ok(initial_state) => InitialStateRead::Present(initial_state),
        Err(error) => InitialStateRead::Malformed(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssr::{INITIAL_STATE_SCRIPT_ID, InitialAuthSnapshot};
    use dioxus::prelude::*;
    use std::cell::RefCell;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    thread_local! {
        static PROVIDER_KEY: RefCell<Option<String>> = const { RefCell::new(None) };
        static PROVIDER_ERRORS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    #[wasm_bindgen_test]
    fn read_initial_state_returns_missing_when_script_is_absent() {
        remove_initial_state_script();

        assert!(matches!(read_initial_state(), InitialStateRead::Missing));
    }

    #[wasm_bindgen_test]
    fn read_initial_state_returns_malformed_when_script_is_empty() {
        remove_initial_state_script();
        append_initial_state_script(None);

        assert!(matches!(
            read_initial_state(),
            InitialStateRead::Malformed(_)
        ));

        remove_initial_state_script();
    }

    #[wasm_bindgen_test]
    fn read_initial_state_returns_malformed_when_script_contains_invalid_json() {
        remove_initial_state_script();
        append_initial_state_script(Some("not json"));

        assert!(matches!(
            read_initial_state(),
            InitialStateRead::Malformed(_)
        ));

        remove_initial_state_script();
    }

    #[wasm_bindgen_test]
    fn clerk_provider_surfaces_malformed_document_initial_state_error() {
        reset_provider_state(Some("pk_test_prop".into()));
        remove_initial_state_script();
        append_initial_state_script(Some("not json"));

        let mut dom = VirtualDom::new(ProviderErrorApp);
        dom.rebuild_in_place();

        assert!(provider_error_contains("malformed SSR initial state"));
        remove_initial_state_script();
    }

    #[wasm_bindgen_test]
    fn clerk_provider_surfaces_document_initial_state_publishable_key_mismatch() {
        reset_provider_state(Some("pk_test_prop".into()));
        remove_initial_state_script();
        let initial_state = InitialState {
            auth: InitialAuthSnapshot::signed_out(),
            publishable_key: Some("pk_test_state".into()),
        };
        let json = serde_json::to_string(&initial_state).unwrap();
        append_initial_state_script(Some(&json));

        let mut dom = VirtualDom::new(ProviderErrorApp);
        dom.rebuild_in_place();

        assert!(provider_error_contains("publishable key mismatch"));
        remove_initial_state_script();
    }

    fn reset_provider_state(publishable_key: Option<String>) {
        PROVIDER_KEY.with(|key| *key.borrow_mut() = publishable_key);
        PROVIDER_ERRORS.with(|errors| errors.borrow_mut().clear());
    }

    fn provider_error_contains(needle: &str) -> bool {
        PROVIDER_ERRORS.with(|errors| errors.borrow().iter().any(|error| error.contains(needle)))
    }

    fn append_initial_state_script(text: Option<&str>) {
        let document = window().and_then(|window| window.document()).unwrap();
        let script = document.create_element("script").unwrap();
        script.set_id(INITIAL_STATE_SCRIPT_ID);
        if let Some(text) = text {
            script.set_text_content(Some(text));
        }
        document.body().unwrap().append_child(&script).unwrap();
    }

    fn remove_initial_state_script() {
        if let Some(document) = window().and_then(|window| window.document()) {
            if let Some(script) = document.get_element_by_id(INITIAL_STATE_SCRIPT_ID) {
                script.remove();
            }
        }
    }

    #[component]
    fn ProviderErrorApp() -> Element {
        let publishable_key = PROVIDER_KEY.with(|key| key.borrow().clone());
        rsx! {
            crate::components::ClerkProvider { publishable_key,
                ProviderErrorProbe {}
            }
        }
    }

    #[component]
    fn ProviderErrorProbe() -> Element {
        let ctx = use_context::<crate::context::ClerkContext>();
        // Startup config warnings surface through the recoverable channel
        // (`current_error`), not `load_error`, so loading can still proceed.
        if let Some(error) = ctx.current_error() {
            PROVIDER_ERRORS.with(|errors| errors.borrow_mut().push(format!("{error}")));
        }
        assert!(
            ctx.load_error.read().is_none(),
            "startup config warnings must not poison load_error"
        );
        rsx! {}
    }
}
