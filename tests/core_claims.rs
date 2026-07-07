use dioxus_clerk::core::{AuthRequirement, AuthState, AuthStatus, ClerkAuth};

fn auth_with_org_claims() -> ClerkAuth {
    let mut auth = ClerkAuth::new("user_2abc", 9_999_999_999);
    auth.session_id = Some("sess_2def".into());
    auth.org_id = Some("org_2ghi".into());
    auth.org_slug = Some("acme".into());
    auth.org_role = Some("admin".into());
    auth.org_permissions = vec!["org:read".into(), "org:write".into()];
    auth
}

#[test]
fn auth_state_signed_in() {
    let auth = auth_with_org_claims();

    let snap: AuthState = (&auth).into();
    assert_eq!(snap.status(), AuthStatus::SignedIn);
    assert_eq!(snap.user_id(), Some("user_2abc"));
    assert_eq!(snap.session_id(), Some("sess_2def"));
    assert!(snap.is_signed_in());
    // Server-verified claims say nothing about browser loadedness.
    assert!(!snap.is_loaded());
}

#[test]
fn auth_state_from_clerk_auth_preserves_verified_org_claims() {
    let snapshot = AuthState::from(&auth_with_org_claims());

    assert_eq!(snapshot.org_role(), Some("admin"));
    assert_eq!(snapshot.org_permissions(), &["org:read", "org:write"]);
}

#[test]
fn auth_state_authorization_helpers_read_verified_org_claims() {
    let snapshot = AuthState::from(&auth_with_org_claims());

    assert!(snapshot.has_role("admin"));
    assert!(snapshot.has_permission("org:read"));
    assert!(snapshot.has(&AuthRequirement::signed_in()));
    assert!(snapshot.has(&AuthRequirement::role("admin")));
    assert!(snapshot.has(&AuthRequirement::permission("org:write")));
    assert!(!snapshot.has_role("basic_member"));
    assert!(!snapshot.has_permission("org:delete"));
}

#[test]
fn clerk_auth_has_mirrors_auth_state_authorization_vocabulary() {
    let auth = auth_with_org_claims();

    // Verified claims are always signed in, so `SignedIn` holds unconditionally.
    assert!(auth.has(&AuthRequirement::signed_in()));
    assert!(auth.has(&AuthRequirement::role("admin")));
    assert!(auth.has(&AuthRequirement::permission("org:write")));
    assert!(!auth.has(&AuthRequirement::role("basic_member")));
    assert!(!auth.has(&AuthRequirement::permission("org:delete")));
}

#[test]
fn auth_state_signed_out() {
    let snap = AuthState::signed_out();
    assert_eq!(snap.status(), AuthStatus::SignedOut);
    assert_eq!(snap.user_id(), None);
    assert!(!snap.is_signed_in());
    // Constructors never claim browser loadedness; set `is_loaded` explicitly.
    assert!(!snap.is_loaded());
}

#[test]
fn auth_state_constructors_are_coherent() {
    let loading = AuthState::loading();
    assert_eq!(loading.status(), AuthStatus::Loading);
    assert!(loading.is_loading());
    assert!(!loading.is_signed_in());
    assert!(!loading.is_signed_out());

    let signed_in = AuthState::signed_in("user_2abc").with_org_role("admin");
    assert_eq!(signed_in.status(), AuthStatus::SignedIn);
    assert!(signed_in.is_signed_in());
    assert_eq!(signed_in.user_id(), Some("user_2abc"));
    assert_eq!(signed_in.require_signed_in().unwrap(), "user_2abc");
}

#[test]
fn role_and_permission_requirements_imply_signed_in() {
    let loading = AuthState::loading()
        .with_org_role("admin")
        .with_org_permissions(["org:read"]);

    assert!(!loading.has(&AuthRequirement::signed_in()));
    assert!(!loading.has(&AuthRequirement::role("admin")));
    assert!(!loading.has(&AuthRequirement::permission("org:read")));

    let signed_out = AuthState::signed_out().with_org_role("admin");
    assert!(!signed_out.has(&AuthRequirement::role("admin")));
}

#[test]
fn clerk_auth_round_trips_through_serde_with_missing_optionals() {
    let auth = auth_with_org_claims();
    let json = serde_json::to_string(&auth).unwrap();
    let back: ClerkAuth = serde_json::from_str(&json).unwrap();
    assert_eq!(back, auth);

    // Every optional claim tolerates absence on the wire.
    let minimal: ClerkAuth =
        serde_json::from_str(r#"{ "user_id": "user_2abc", "exp": 1, "nbf": 0, "iat": 0 }"#)
            .unwrap();
    assert_eq!(minimal.user_id, "user_2abc");
    assert_eq!(minimal.session_id, None);
    assert_eq!(minimal.org_id, None);
    assert_eq!(minimal.org_role, None);
    assert!(minimal.org_permissions.is_empty());
}

#[test]
fn require_signed_in_errors_for_loading_and_signed_out_states() {
    use dioxus_clerk::core::ClerkError;

    assert!(matches!(
        AuthState::loading().require_signed_in(),
        Err(ClerkError::NotLoaded)
    ));
    assert!(matches!(
        AuthState::signed_out().require_signed_in(),
        Err(ClerkError::Unauthenticated)
    ));
}

#[test]
fn signed_in_constructor_treats_empty_user_id_as_signed_out() {
    let state = AuthState::signed_in("");

    assert_eq!(state.status(), AuthStatus::SignedOut);
    assert_eq!(state.user_id(), None);
    assert!(matches!(
        state.require_signed_in(),
        Err(dioxus_clerk::core::ClerkError::Unauthenticated)
    ));
}

#[test]
fn auth_requirement_can_be_reused_across_checks_by_reference() {
    let requirement = AuthRequirement::role("admin");
    let snapshot = AuthState::from(&auth_with_org_claims());

    assert!(snapshot.has(&requirement));
    assert!(snapshot.has(&requirement));
}

#[test]
fn auth_state_from_empty_subject_is_signed_out() {
    let auth = ClerkAuth::new("", 0);

    let snap = AuthState::from(&auth);

    assert_eq!(snap.status(), AuthStatus::SignedOut);
    assert!(!snap.is_signed_in());
    assert_eq!(snap.user_id(), None);
    assert_eq!(snap.session_id(), None);
}
