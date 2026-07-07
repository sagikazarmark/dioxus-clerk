use dioxus_clerk::core::{
    AuthRequirement, AuthState, ClerkAuth, ClerkError, Session, User, VerificationOutcome,
};
use dioxus_clerk::ssr::InitialState;

#[test]
fn core_module_exposes_shared_auth_types() {
    let mut auth = ClerkAuth::new("user_2abc", 9_999_999_999);
    auth.session_id = Some("sess_2def".into());
    auth.org_id = Some("org_2ghi".into());
    auth.org_slug = Some("acme".into());
    auth.org_role = Some("admin".into());
    auth.org_permissions = vec!["org:read".into()];

    let outcome = VerificationOutcome::Valid(auth.clone());
    let state = InitialState::from_outcome(Some(&outcome), Some("pk_test_state"));

    assert!(state.auth.is_signed_in());
    assert_eq!(state.auth.user_id.as_deref(), Some("user_2abc"));
    assert!(state.script_html().contains("__clerk_initial_state"));

    let auth_state = AuthState::from(&auth);
    assert!(auth_state.is_signed_in());
    assert!(auth_state.has(&AuthRequirement::permission("org:read")));

    let user = User::new("user_2abc");
    let session = Session::new("sess_2def", "active");
    assert_eq!(user.id, "user_2abc");
    assert_eq!(session.id, "sess_2def");
    assert!(session.is_active());

    let error = ClerkError::Unauthenticated;
    assert_eq!(error.to_string(), "unauthenticated");
}
