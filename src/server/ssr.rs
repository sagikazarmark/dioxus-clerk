//! SSR initial state helpers.
//!
//! The helpers return either the canonical initial state or a
//! `<script id="__clerk_initial_state">` HTML fragment carrying the verified
//! auth snapshot plus the publishable key. The schema and interpretation rules
//! live in `crate::ssr` so producer and consumer behavior cannot
//! drift.
//!
//! These helpers exist for integrations that render HTML **without**
//! [`crate::ClerkProvider`] (custom shells, non-Dioxus templating). A
//! server-rendered `ClerkProvider` already emits its own
//! `<script id="__clerk_initial_state">` element; combining it with these
//! helpers puts two elements with the same id in the document, and the client
//! reads whichever comes first.

use crate::core::VerificationOutcome;
use crate::ssr::{InitialState, InitialStateRead, ProviderStartup};

/// Returns the canonical SSR initial state for explicit Server verification outcome
/// input.
pub fn initial_state(
    outcome: Option<&VerificationOutcome>,
    publishable_key: Option<&str>,
) -> InitialState {
    InitialState::from_outcome(outcome, publishable_key)
}

/// Returns a self-contained `<script>` HTML fragment carrying SSR initial state
/// for client hydration.
///
/// Only `VerificationOutcome::Valid` contributes signed-in auth; verified
/// non-sessions (`Missing`, `Invalid`) produce signed-out initial state, and
/// absent or `Unavailable` outcomes produce an unverified seed the client
/// resolves itself. See [`InitialState::from_outcome`].
///
/// Do not combine with a server-rendered [`crate::ClerkProvider`], which
/// already emits this element — see the module docs.
pub fn initial_state_script(
    outcome: Option<&VerificationOutcome>,
    publishable_key: Option<&str>,
) -> String {
    InitialState::from_outcome(outcome, publishable_key).script_html()
}

/// Returns the canonical SSR initial state for the current Dioxus fullstack context.
pub fn initial_state_from_current_context(publishable_key: Option<&str>) -> InitialState {
    let outcome = super::context::current_outcome();
    initial_state(outcome.as_ref(), publishable_key)
}

/// Returns an SSR initial state script for the current Dioxus fullstack context.
///
/// Do not combine with a server-rendered [`crate::ClerkProvider`], which
/// already emits this element — see the module docs.
pub fn initial_state_script_from_current_context(publishable_key: Option<&str>) -> String {
    let outcome = super::context::current_outcome();
    initial_state_script(outcome.as_ref(), publishable_key)
}

/// Returns provider-ready startup facts for the current Dioxus fullstack context.
///
/// The server always produces a `Present` seed read: even without a
/// verification outcome, [`InitialState::from_outcome`] seeds an unverified
/// snapshot the client resolves itself.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) fn provider_startup_from_current_context(
    prop_publishable_key: Option<String>,
) -> ProviderStartup {
    let outcome = super::context::current_outcome();
    let seed = InitialState::from_outcome(outcome.as_ref(), prop_publishable_key.as_deref());
    crate::ssr::provider_startup_from_read(InitialStateRead::Present(seed), prop_publishable_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ClerkAuth, InvalidTokenReason};
    use axum::http::Request;
    use dioxus_fullstack_core::FullstackContext;

    #[tokio::test]
    async fn provider_startup_from_current_context_uses_valid_outcome() {
        let cx = fullstack_context_with_outcome(VerificationOutcome::Valid(sample_auth()));

        let startup = cx
            .scope(async { provider_startup_from_current_context(Some("pk_test_xxx".into())) })
            .await;

        let state = startup.auth.to_state();
        assert!(!state.is_loaded);
        assert!(state.is_signed_in());
        assert_eq!(state.user_id.as_deref(), Some("user_2abc"));
        assert_eq!(startup.publishable_key.as_deref(), Some("pk_test_xxx"));
        assert!(startup.warning.is_none());
        assert!(
            startup
                .initial_state_json
                .as_deref()
                .is_some_and(|json| json.contains("user_2abc"))
        );
    }

    #[tokio::test]
    async fn provider_startup_from_current_context_ignores_stale_raw_auth() {
        let cx = fullstack_context_with_invalid_outcome_and_stale_auth();

        let startup = cx
            .scope(async { provider_startup_from_current_context(Some("pk_test_xxx".into())) })
            .await;

        let state = startup.auth.to_state();
        assert!(!state.is_loaded);
        assert!(!state.is_signed_in());
        assert!(state.user_id.is_none());
        assert_eq!(startup.publishable_key.as_deref(), Some("pk_test_xxx"));
        assert!(startup.warning.is_none());
        assert!(startup.initial_state_json.is_some());
    }

    fn fullstack_context_with_outcome(outcome: VerificationOutcome) -> FullstackContext {
        let mut req = Request::builder().uri("/").body(()).unwrap();
        req.extensions_mut().insert(outcome);
        let (parts, _body) = req.into_parts();

        FullstackContext::new(parts)
    }

    fn fullstack_context_with_invalid_outcome_and_stale_auth() -> FullstackContext {
        let mut req = Request::builder().uri("/").body(()).unwrap();
        req.extensions_mut().insert(sample_auth());
        req.extensions_mut()
            .insert(VerificationOutcome::Invalid(InvalidTokenReason::Other));
        let (parts, _body) = req.into_parts();

        FullstackContext::new(parts)
    }

    fn sample_auth() -> ClerkAuth {
        ClerkAuth {
            user_id: "user_2abc".into(),
            session_id: Some("sess_2def".into()),
            org_id: Some("org_2ghi".into()),
            org_slug: Some("acme".into()),
            org_role: Some("admin".into()),
            org_permissions: vec!["org:read".into(), "org:write".into()],
            exp: 9_999_999_999,
            nbf: 0,
            iat: 0,
        }
    }
}
