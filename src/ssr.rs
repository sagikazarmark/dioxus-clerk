//! Shared SSR initial state schema and lifecycle helpers.
//!
//! Used by both the server (`dioxus_clerk::server::ssr`) and the wasm client
//! so producer, consumer, and interpretation semantics stay in one place.

use crate::core::{AuthRuntimeState, ClerkAuth, ClerkError, VerificationOutcome};
use serde::{Deserialize, Serialize};

/// Stable id of the `<script>` element carrying the SSR initial state.
pub const INITIAL_STATE_SCRIPT_ID: &str = "__clerk_initial_state";

/// Server-side auth resolution carried by the SSR initial state.
///
/// Distinguishes "the server verified this request" (signed-in or signed-out)
/// from "verification never happened" (`Unverified`), so a render that never
/// checked the session cannot seed a resolved signed-out state: a returning
/// signed-in user must not see a signed-out flash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InitialAuthStatus {
    /// The server verified a signed-in session.
    SignedIn,
    /// The server verified the request and found no valid session.
    SignedOut,
    /// The server did not (or could not) verify the request: the auth layer
    /// was absent or verification infrastructure was unavailable. The client
    /// seeds a loading state and resolves auth itself.
    ///
    /// Also the deserialization fallback for status strings this crate has
    /// not named yet (`#[serde(other)]`), so a newer server emitting a future
    /// status degrades an older client to "resolve auth yourself" instead of
    /// failing the whole seed parse.
    #[serde(other)]
    Unverified,
}

/// Server-verified auth snapshot carried by the SSR initial state.
///
/// This is not app-visible [`crate::core::AuthState`]. It intentionally has no browser
/// loadedness field; `AuthRuntimeState` derives browser loadedness during Clerk
/// lifecycle startup.
///
/// The struct is `#[non_exhaustive]`; construct values with
/// [`InitialAuthSnapshot::signed_out`], [`InitialAuthSnapshot::signed_in`],
/// [`InitialAuthSnapshot::unverified`], or `From<&ClerkAuth>` and set optional
/// fields directly. Optional fields tolerate absence on the wire; `status` is
/// required, so a snapshot without it is rejected as malformed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct InitialAuthSnapshot {
    /// Server-side auth resolution for this render.
    pub status: InitialAuthStatus,
    /// Clerk user id, if signed in.
    #[serde(default)]
    pub user_id: Option<String>,
    /// Active session id, if signed in and available.
    #[serde(default)]
    pub session_id: Option<String>,
    /// Active organization id, if any.
    #[serde(default)]
    pub org_id: Option<String>,
    /// Active organization slug, if any.
    #[serde(default)]
    pub org_slug: Option<String>,
    /// Organization role from verified server auth, if any.
    #[serde(default)]
    pub org_role: Option<String>,
    /// Organization permissions from verified server auth.
    #[serde(default)]
    pub org_permissions: Vec<String>,
}

impl InitialAuthSnapshot {
    /// Initial auth snapshot for a server-verified anonymous render.
    pub fn signed_out() -> Self {
        Self {
            status: InitialAuthStatus::SignedOut,
            user_id: None,
            session_id: None,
            org_id: None,
            org_slug: None,
            org_role: None,
            org_permissions: vec![],
        }
    }

    /// Initial auth snapshot for a render where the server did not verify the
    /// request. Seeds a loading state on the client, not signed-out.
    pub fn unverified() -> Self {
        Self {
            status: InitialAuthStatus::Unverified,
            ..Self::signed_out()
        }
    }

    /// Initial auth snapshot for a server-verified signed-in session.
    /// Set optional session/org fields directly.
    ///
    /// An empty user id cannot represent a signed-in session and produces
    /// [`InitialAuthSnapshot::signed_out`], matching
    /// [`crate::core::AuthState::signed_in`].
    pub fn signed_in(user_id: impl Into<String>) -> Self {
        let user_id = user_id.into();
        if user_id.is_empty() {
            return Self::signed_out();
        }

        Self {
            status: InitialAuthStatus::SignedIn,
            user_id: Some(user_id),
            ..Self::signed_out()
        }
    }

    /// Whether the server verified a signed-in session.
    pub fn is_signed_in(&self) -> bool {
        matches!(self.status, InitialAuthStatus::SignedIn)
    }
}

impl From<&ClerkAuth> for InitialAuthSnapshot {
    fn from(auth: &ClerkAuth) -> Self {
        if auth.user_id.is_empty() {
            return Self::signed_out();
        }

        Self {
            status: InitialAuthStatus::SignedIn,
            user_id: Some(auth.user_id.clone()),
            session_id: auth.session_id.clone(),
            org_id: auth.org_id.clone(),
            org_slug: auth.org_slug.clone(),
            org_role: auth.org_role.clone(),
            org_permissions: auth.org_permissions.clone(),
        }
    }
}

/// Canonical JSON shape for SSR initial state. Both producer and consumer
/// reference this struct so the schema lives in one place.
///
/// The struct is `#[non_exhaustive]`; construct values with
/// [`InitialState::new`], [`InitialState::from_verified_auth`], or
/// [`InitialState::from_outcome`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct InitialState {
    /// Verified auth snapshot at the moment of server render.
    pub auth: InitialAuthSnapshot,
    /// Publishable key forwarded from the server-side `ClerkProvider`
    /// prop so the client can initialize clerk-js without a separate
    /// configuration dance. `None` if the server didn't have one to share.
    #[serde(default)]
    pub publishable_key: Option<String>,
}

impl InitialState {
    /// Build SSR initial state from an auth snapshot and an optional publishable key.
    pub fn new(auth: InitialAuthSnapshot, publishable_key: Option<&str>) -> Self {
        Self {
            auth,
            publishable_key: publishable_key.map(String::from),
        }
    }

    /// Build SSR initial state from server-verified auth and an optional publishable key.
    pub fn from_verified_auth(auth: Option<&ClerkAuth>, publishable_key: Option<&str>) -> Self {
        Self::new(
            auth.map(InitialAuthSnapshot::from)
                .unwrap_or_else(InitialAuthSnapshot::signed_out),
            publishable_key,
        )
    }

    /// Build SSR initial state from a Server verification outcome.
    ///
    /// Only `VerificationOutcome::Valid` contributes signed-in auth. Verified
    /// non-sessions (`Missing`, `Invalid`) seed a resolved signed-out state.
    /// `None` (no `ClerkAuthLayer` ran for this request) and
    /// `Unavailable` (verification infrastructure was down) seed
    /// [`InitialAuthStatus::Unverified`] so the client resolves auth itself
    /// instead of flashing signed-out UI at a possibly signed-in user.
    pub fn from_outcome(
        outcome: Option<&VerificationOutcome>,
        publishable_key: Option<&str>,
    ) -> Self {
        let auth = match outcome {
            Some(VerificationOutcome::Valid(auth)) => InitialAuthSnapshot::from(auth),
            Some(VerificationOutcome::Missing | VerificationOutcome::Invalid(_)) => {
                InitialAuthSnapshot::signed_out()
            }
            // Unavailable, future outcomes, or no auth layer on this request.
            _ => InitialAuthSnapshot::unverified(),
        };
        Self::new(auth, publishable_key)
    }

    /// Build the safe `<script id="__clerk_initial_state">` HTML fragment for
    /// this SSR initial state.
    ///
    /// Every `<` inside the JSON body is emitted as the `\u003c` JSON escape,
    /// so an untrusted string can neither close the script element
    /// (`</script>`) nor open a `<!--<script>` script-data double-escaped
    /// section. JSON parsers recover the original bytes.
    pub fn script_html(&self) -> String {
        format!(
            r#"<script id="{INITIAL_STATE_SCRIPT_ID}" type="application/json">{}</script>"#,
            self.script_json()
        )
    }

    /// The JSON body used by [`InitialState::script_html`], with `<` escaped
    /// as `\u003c` so it is safe to embed inside a `<script>` element.
    pub fn script_json(&self) -> String {
        serde_json::to_string(self)
            .expect("serialization is infallible")
            .replace('<', "\\u003c")
    }
}

/// Per-platform result of reading the SSR seed: what a startup path found
/// before interpretation. The browser derives it from the initial state
/// script, the server from the Server verification outcome, and native
/// renders report `Missing` because no seed exists.
///
/// Not every variant is constructed on every target (e.g. `Malformed` only
/// arises from the browser document read).
#[cfg_attr(not(clerk_client), allow(dead_code))]
#[derive(Debug, Clone)]
pub(crate) enum InitialStateRead {
    /// No SSR initial state script exists. This is normal for web-only apps.
    Missing,
    /// An initial state script existed and parsed successfully.
    Present(InitialState),
    /// An initial state script existed but could not be parsed.
    Malformed(String),
}

/// Provider-ready facts derived from SSR initial state discovery and explicit provider config.
#[derive(Debug, Clone)]
pub(crate) struct ProviderStartup {
    /// Initial provider runtime auth state.
    pub(crate) auth: AuthRuntimeState,
    /// Effective publishable key for clerk-js initialization.
    #[cfg_attr(not(clerk_client), allow(dead_code))]
    pub(crate) publishable_key: Option<String>,
    /// Non-fatal configuration warning, if any. Surfaced through the
    /// recoverable error channel (`use_clerk_error`), never through
    /// `load_error`: startup proceeds despite these, so they must not gate
    /// the action pipeline or render `ClerkFailed`.
    pub(crate) warning: Option<ClerkError>,
    /// Escaped initial-state JSON for the provider to render into its
    /// `<script id="__clerk_initial_state">` element, if any.
    pub(crate) initial_state_json: Option<String>,
}

/// Interpret an SSR seed read into provider-ready startup facts.
///
/// This is the single consumer for every platform startup path; the paths
/// differ only in how they produce the [`InitialStateRead`]. A `Missing`
/// seed stays `loading`, not signed-out, because nothing was verified: a
/// returning signed-in user must not see a signed-out flash from a render
/// that never checked their session. For a `Present` seed the explicit
/// provider prop wins over the seed's publishable key, and the seed JSON is
/// re-emitted for the provider's own initial-state script.
pub(crate) fn provider_startup_from_read(
    read: InitialStateRead,
    prop_publishable_key: Option<String>,
) -> ProviderStartup {
    match read {
        InitialStateRead::Missing => ProviderStartup {
            auth: AuthRuntimeState::loading(),
            publishable_key: prop_publishable_key,
            warning: None,
            initial_state_json: None,
        },
        InitialStateRead::Malformed(message) => ProviderStartup {
            auth: AuthRuntimeState::loading(),
            publishable_key: prop_publishable_key,
            warning: Some(ClerkError::InvalidConfig(format!(
                "malformed SSR initial state: {message}"
            ))),
            initial_state_json: None,
        },
        InitialStateRead::Present(initial_state) => {
            let auth = AuthRuntimeState::from_initial_auth_snapshot(&initial_state.auth);
            let publishable_key = prop_publishable_key
                .clone()
                .or_else(|| initial_state.publishable_key.clone());
            let warning = match (
                prop_publishable_key.as_deref(),
                initial_state.publishable_key.as_deref(),
            ) {
                (Some(prop), Some(state)) if prop != state => Some(ClerkError::InvalidConfig(
                    "SSR initial state publishable key mismatch; using ClerkProvider publishable_key prop"
                        .into(),
                )),
                _ => None,
            };

            ProviderStartup {
                auth,
                publishable_key,
                warning,
                initial_state_json: Some(initial_state.script_json()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::InvalidTokenReason;

    #[test]
    fn provider_startup_from_missing_initial_state_keeps_loading_auth_and_explicit_key_without_html()
     {
        let startup =
            provider_startup_from_read(InitialStateRead::Missing, Some("pk_test_prop".into()));

        assert!(startup.auth.to_state().is_loading());
        assert!(!startup.auth.to_state().is_signed_in());
        assert!(!startup.auth.should_render_signed_out());
        assert_eq!(startup.publishable_key.as_deref(), Some("pk_test_prop"));
        assert!(startup.warning.is_none());
        assert!(startup.initial_state_json.is_none());
    }

    #[test]
    fn provider_startup_from_malformed_initial_state_surfaces_error_without_html() {
        let startup = provider_startup_from_read(
            InitialStateRead::Malformed("expected value at line 1 column 1".into()),
            Some("pk_test_prop".into()),
        );

        assert!(!startup.auth.to_state().is_signed_in());
        assert!(!startup.auth.should_render_signed_out());
        assert_eq!(startup.publishable_key.as_deref(), Some("pk_test_prop"));
        assert!(
            matches!(startup.warning, Some(ClerkError::InvalidConfig(message)) if message.contains("malformed SSR initial state"))
        );
        assert!(startup.initial_state_json.is_none());
    }

    #[test]
    fn provider_startup_from_present_initial_state_uses_snapshot_auth_and_keeps_browser_unloaded() {
        let startup = provider_startup_from_read(
            InitialStateRead::Present(signed_in_initial_state(Some("pk_test_state"))),
            None,
        );

        let state = startup.auth.to_state();
        assert!(!state.is_loaded);
        assert!(state.is_signed_in());
        assert_eq!(state.user_id.as_deref(), Some("user_2abc"));
        assert_eq!(state.session_id.as_deref(), Some("sess_2def"));
        assert_eq!(state.org_id.as_deref(), Some("org_2ghi"));
        assert_eq!(startup.publishable_key.as_deref(), Some("pk_test_state"));
        assert!(startup.warning.is_none());
        assert!(
            startup
                .initial_state_json
                .as_deref()
                .is_some_and(|json| json.contains("user_2abc"))
        );
    }

    #[test]
    fn provider_startup_prefers_explicit_key_and_surfaces_state_key_mismatch() {
        let startup = provider_startup_from_read(
            InitialStateRead::Present(signed_in_initial_state(Some("pk_test_state"))),
            Some("pk_test_prop".into()),
        );

        assert_eq!(startup.publishable_key.as_deref(), Some("pk_test_prop"));
        assert!(
            matches!(startup.warning, Some(ClerkError::InvalidConfig(message)) if message.contains("publishable key mismatch"))
        );
        assert!(startup.initial_state_json.is_some());
    }

    #[test]
    fn initial_state_from_outcome_maps_verification_to_three_state_seed() {
        let cases = vec![
            // No auth layer ran: the client must resolve auth itself.
            (None, InitialAuthStatus::Unverified, None),
            // Verified non-sessions resolve to signed-out.
            (
                Some(VerificationOutcome::Missing),
                InitialAuthStatus::SignedOut,
                None,
            ),
            (
                Some(VerificationOutcome::Invalid(InvalidTokenReason::Other)),
                InitialAuthStatus::SignedOut,
                None,
            ),
            // Infrastructure outage: unknown, not signed-out.
            (
                Some(VerificationOutcome::Unavailable),
                InitialAuthStatus::Unverified,
                None,
            ),
            (
                Some(VerificationOutcome::Valid(sample_auth())),
                InitialAuthStatus::SignedIn,
                Some("user_2abc"),
            ),
        ];

        for (outcome, expected_status, expected_user_id) in cases {
            let initial_state = InitialState::from_outcome(outcome.as_ref(), Some("pk_test_state"));

            assert_eq!(initial_state.auth.status, expected_status);
            assert_eq!(initial_state.auth.user_id.as_deref(), expected_user_id);
            assert_eq!(
                initial_state.publishable_key.as_deref(),
                Some("pk_test_state")
            );
        }
    }

    #[test]
    fn unverified_initial_state_seeds_loading_not_signed_out() {
        let initial_state = InitialState::from_outcome(None, Some("pk_test_state"));

        let startup = provider_startup_from_read(
            InitialStateRead::Present(initial_state),
            Some("pk_test_state".into()),
        );

        let state = startup.auth.to_state();
        assert!(state.is_loading());
        assert!(!state.is_signed_in());
        assert!(!startup.auth.should_render_signed_out());
        assert!(startup.warning.is_none());
    }

    #[test]
    fn initial_state_from_empty_subject_is_signed_out_without_user_id() {
        let initial_state = InitialState::from_verified_auth(Some(&ClerkAuth::new("", 0)), None);

        assert!(!initial_state.auth.is_signed_in());
        assert_eq!(initial_state.auth.status, InitialAuthStatus::SignedOut);
        assert!(initial_state.auth.user_id.is_none());
        assert!(initial_state.auth.session_id.is_none());
    }

    #[test]
    fn outcome_seed_preserves_auth_loadedness_and_key_conflict_error() {
        let initial_state = InitialState::from_outcome(
            Some(&VerificationOutcome::Valid(sample_auth())),
            Some("pk_test_state"),
        );

        let startup = provider_startup_from_read(
            InitialStateRead::Present(initial_state),
            Some("pk_test_prop".into()),
        );

        let state = startup.auth.to_state();
        assert!(!state.is_loaded);
        assert!(state.is_signed_in());
        assert_eq!(state.user_id.as_deref(), Some("user_2abc"));
        assert_eq!(startup.publishable_key.as_deref(), Some("pk_test_prop"));
        assert!(
            matches!(startup.warning, Some(ClerkError::InvalidConfig(message)) if message.contains("publishable key mismatch"))
        );
        assert!(
            startup
                .initial_state_json
                .as_deref()
                .is_some_and(|json| json.contains("user_2abc"))
        );
    }

    #[test]
    fn provider_startup_from_valid_outcome_emits_signed_in_initial_auth_snapshot() {
        let outcome = VerificationOutcome::Valid(sample_auth());

        let startup = provider_startup_from_read(
            InitialStateRead::Present(InitialState::from_outcome(
                Some(&outcome),
                Some("pk_test_prop"),
            )),
            Some("pk_test_prop".into()),
        );

        let state = startup.auth.to_state();
        assert!(!state.is_loaded);
        assert!(state.is_signed_in());
        assert_eq!(state.user_id.as_deref(), Some("user_2abc"));
        assert_eq!(startup.publishable_key.as_deref(), Some("pk_test_prop"));
        assert!(startup.warning.is_none());
        assert!(
            startup
                .initial_state_json
                .as_deref()
                .is_some_and(|json| json.contains("user_2abc"))
        );
    }

    #[test]
    fn ssr_seed_round_trip_keeps_server_facts_without_browser_loadedness() {
        let outcome = VerificationOutcome::Valid(sample_auth());
        let server_startup = provider_startup_from_read(
            InitialStateRead::Present(InitialState::from_outcome(Some(&outcome), Some("pk_test"))),
            Some("pk_test".into()),
        );
        let json = server_startup
            .initial_state_json
            .as_deref()
            .expect("server startup emits SSR seed JSON");
        let decoded =
            serde_json::from_str::<InitialState>(json).expect("SSR seed script body is JSON");

        let browser_startup = provider_startup_from_read(InitialStateRead::Present(decoded), None);

        let state = browser_startup.auth.to_state();
        assert!(!state.is_loaded);
        assert!(state.is_signed_in());
        assert_eq!(state.user_id.as_deref(), Some("user_2abc"));
        assert_eq!(state.org_role.as_deref(), Some("admin"));
        assert_eq!(browser_startup.publishable_key.as_deref(), Some("pk_test"));
    }

    #[test]
    fn provider_startup_from_invalid_outcome_is_anonymous() {
        let startup = provider_startup_from_read(
            InitialStateRead::Present(InitialState::from_outcome(
                Some(&VerificationOutcome::Invalid(InvalidTokenReason::Other)),
                Some("pk_test_prop"),
            )),
            Some("pk_test_prop".into()),
        );

        let state = startup.auth.to_state();
        assert!(!state.is_loaded);
        assert!(!state.is_signed_in());
        assert!(state.user_id.is_none());
        assert_eq!(startup.publishable_key.as_deref(), Some("pk_test_prop"));
        assert!(startup.warning.is_none());
        assert!(startup.initial_state_json.is_some());
    }

    #[test]
    fn unknown_initial_auth_status_degrades_to_unverified_instead_of_failing_the_parse() {
        let json = r#"{"auth":{"status":"passkey_pending","user_id":"user_2abc"},"publishable_key":"pk_test"}"#;

        let decoded = serde_json::from_str::<InitialState>(json)
            .expect("a future status string must not fail the whole seed parse");

        assert_eq!(decoded.auth.status, InitialAuthStatus::Unverified);
        let startup = provider_startup_from_read(InitialStateRead::Present(decoded), None);
        assert!(startup.auth.to_state().is_loading());
        assert!(startup.warning.is_none());
    }

    #[test]
    fn script_html_escapes_every_angle_bracket_in_string_fields() {
        let mut snapshot = InitialAuthSnapshot::signed_in("user_2abc");
        snapshot.org_slug = Some("</script><script>alert(1)</script>".into());
        snapshot.org_role = Some("<!--<script>".into());
        let initial_state = InitialState::new(snapshot, Some("pk_test"));

        let html = initial_state.script_html();
        let open_end = html.find('>').expect("opening script tag");
        let close_start = html.rfind("</script>").expect("closing script tag");
        let body = &html[open_end + 1..close_start];

        assert!(!body.contains('<'));
        assert!(!body.contains("<!--"));

        let decoded = serde_json::from_str::<InitialState>(body).expect("body is valid JSON");
        assert_eq!(
            decoded.auth.org_slug.as_deref(),
            Some("</script><script>alert(1)</script>")
        );
        assert_eq!(decoded.auth.org_role.as_deref(), Some("<!--<script>"));
    }

    fn signed_in_initial_state(publishable_key: Option<&str>) -> InitialState {
        let mut snapshot = InitialAuthSnapshot::signed_in("user_2abc");
        snapshot.session_id = Some("sess_2def".into());
        snapshot.org_id = Some("org_2ghi".into());
        snapshot.org_slug = Some("acme".into());
        snapshot.org_role = Some("admin".into());
        snapshot.org_permissions = vec!["org:read".into()];
        InitialState::new(snapshot, publishable_key)
    }

    fn sample_auth() -> ClerkAuth {
        let mut auth = ClerkAuth::new("user_2abc", 9_999_999_999);
        auth.session_id = Some("sess_2def".into());
        auth.org_id = Some("org_2ghi".into());
        auth.org_slug = Some("acme".into());
        auth.org_role = Some("admin".into());
        auth.org_permissions = vec!["org:read".into()];
        auth
    }
}
