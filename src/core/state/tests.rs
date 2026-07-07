use super::{
    AuthObservation, AuthRuntimeState, Session, SessionStatus, SessionTask, SessionTaskKey, User,
};
use crate::core::AuthStatus;
use crate::ssr::InitialAuthSnapshot;

fn signed_in_initial_auth_snapshot() -> InitialAuthSnapshot {
    InitialAuthSnapshot {
        status: crate::ssr::InitialAuthStatus::SignedIn,
        user_id: Some("user_2abc".into()),
        session_id: Some("sess_2def".into()),
        org_id: Some("org_2ghi".into()),
        org_slug: Some("acme".into()),
        org_role: None,
        org_permissions: vec![],
    }
}

fn signed_out_initial_auth_snapshot() -> InitialAuthSnapshot {
    InitialAuthSnapshot::signed_out()
}

fn signed_in_snapshot_state() -> AuthRuntimeState {
    AuthRuntimeState::from_initial_auth_snapshot(&signed_in_initial_auth_snapshot())
}

fn signed_in_state() -> AuthRuntimeState {
    let previous = signed_in_snapshot_state();
    AuthRuntimeState::from_js_session(
        sample_user("user_2abc"),
        sample_session_in_org("sess_2def", Some("org_2ghi")),
        &previous,
    )
}

fn sample_user(id: &str) -> User {
    User {
        id: id.into(),
        first_name: None,
        last_name: None,
        primary_email_address: None,
        image_url: None,
    }
}

fn sample_session(id: &str) -> Session {
    sample_session_in_org(id, None)
}

fn pending_session_with_task(id: &str, key: SessionTaskKey) -> Session {
    Session {
        status: SessionStatus::Pending,
        current_task: Some(SessionTask { key: key.clone() }),
        tasks: vec![SessionTask { key }],
        ..sample_session(id)
    }
}

fn sample_session_in_org(id: &str, org_id: Option<&str>) -> Session {
    Session {
        id: id.into(),
        status: "active".into(),
        last_active_organization_id: org_id.map(Into::into),
        last_active_at: None,
        expire_at: None,
        current_task: None,
        tasks: Vec::new(),
    }
}

#[test]
fn auth_state_default_is_loading() {
    let state = AuthRuntimeState::default();
    assert!(!state.is_loaded());
    assert_eq!(state.to_state().status, AuthStatus::Loading);
    assert!(!state.to_state().is_signed_in());
}

#[test]
fn signed_out_initial_auth_snapshot_is_resolved_even_before_browser_loadedness() {
    let state = AuthRuntimeState::from_initial_auth_snapshot(&signed_out_initial_auth_snapshot());
    let snapshot = state.to_state();

    assert!(!snapshot.is_loaded);
    assert_eq!(snapshot.status, AuthStatus::SignedOut);
    assert!(snapshot.is_signed_out());
}

#[test]
fn user_round_trips_through_serde_with_unknown_fields_ignored() {
    let json = r#"{
        "id": "user_2abc",
        "first_name": "Ada",
        "last_name": "Lovelace",
        "primary_email_address": "ada@example.com",
        "image_url": "https://img/x.png",
        "extra_unknown_field": 42
    }"#;
    let user: User = serde_json::from_str(json).unwrap();
    assert_eq!(user.id, "user_2abc");
    assert_eq!(user.first_name.as_deref(), Some("Ada"));
    assert_eq!(
        user.primary_email_address.as_deref(),
        Some("ada@example.com")
    );
}

#[test]
fn user_deserializes_clerk_js_camel_case_fields() {
    let json = r#"{
        "id": "user_2abc",
        "firstName": "Ada",
        "lastName": "Lovelace",
        "primaryEmailAddress": "ada@example.com",
        "imageUrl": "https://img/x.png"
    }"#;
    let user: User = serde_json::from_str(json).unwrap();

    assert_eq!(user.first_name.as_deref(), Some("Ada"));
    assert_eq!(user.last_name.as_deref(), Some("Lovelace"));
    assert_eq!(
        user.primary_email_address.as_deref(),
        Some("ada@example.com")
    );
    assert_eq!(user.image_url.as_deref(), Some("https://img/x.png"));
}

#[test]
fn session_round_trips() {
    let json =
        r#"{ "id": "sess_2", "status": "active", "last_active_at": 12345, "expire_at": 99999 }"#;
    let s: Session = serde_json::from_str(json).unwrap();
    assert_eq!(s.id, "sess_2");
    assert_eq!(s.status, SessionStatus::Active);
    assert!(s.is_active());
}

#[test]
fn session_deserializes_clerk_js_camel_case_fields() {
    let json = r#"{
        "id": "sess_2",
        "status": "active",
        "lastActiveOrganizationId": "org_2ghi",
        "lastActiveAt": 12345,
        "expireAt": 99999
    }"#;
    let s: Session = serde_json::from_str(json).unwrap();

    assert_eq!(s.last_active_organization_id.as_deref(), Some("org_2ghi"));
    assert_eq!(s.last_active_at, Some(12345));
    assert_eq!(s.expire_at, Some(99999));
}

#[test]
fn session_deserializes_clerk_js_current_task_and_tasks() {
    // A clerk-js v6 pending session carries `currentTask` (camelCase) and a
    // `tasks` array; both must surface as typed values for app-side routing.
    let json = r#"{
        "id": "sess_2",
        "status": "pending",
        "currentTask": { "key": "setup-mfa" },
        "tasks": [{ "key": "setup-mfa" }, { "key": "choose-organization" }]
    }"#;
    let s: Session = serde_json::from_str(json).unwrap();

    assert_eq!(s.status, SessionStatus::Pending);
    assert_eq!(
        s.current_task,
        Some(SessionTask {
            key: SessionTaskKey::SetupMfa
        })
    );
    assert_eq!(
        s.tasks,
        vec![
            SessionTask {
                key: SessionTaskKey::SetupMfa
            },
            SessionTask {
                key: SessionTaskKey::ChooseOrganization
            },
        ]
    );
}

#[test]
fn session_without_tasks_defaults_current_task_none_and_tasks_empty() {
    // A `tasks: null` (clerk-js `Array<SessionTask> | null`) or an omitted
    // `currentTask` must read as no pending task, not a deserialize error.
    let json = r#"{ "id": "sess_2", "status": "active", "tasks": null }"#;
    let s: Session = serde_json::from_str(json).unwrap();

    assert_eq!(s.current_task, None);
    assert!(s.tasks.is_empty());
}

#[test]
fn session_task_key_round_trips_named_and_unknown_keys() {
    assert_eq!(SessionTaskKey::from("setup-mfa"), SessionTaskKey::SetupMfa);
    assert_eq!(
        SessionTaskKey::from("choose-organization").as_str(),
        "choose-organization"
    );
    let unknown = SessionTaskKey::from("brand-new-task");
    assert!(matches!(unknown, SessionTaskKey::Other(_)));
    assert_eq!(unknown.as_str(), "brand-new-task");
    // A known key string is always canonicalized to its named variant, so an
    // `Other` can never alias one.
    assert_eq!(
        SessionTaskKey::from("reset-password".to_string()),
        SessionTaskKey::ResetPassword
    );

    // Unknown keys survive a serde round-trip through the raw JSON string.
    let task = SessionTask::new("brand-new-task");
    let json = serde_json::to_string(&task).unwrap();
    assert_eq!(json, r#"{"key":"brand-new-task"}"#);
    let back: SessionTask = serde_json::from_str(&json).unwrap();
    assert_eq!(back, task);
}

#[test]
fn session_status_round_trips_named_and_unknown_statuses() {
    assert_eq!(SessionStatus::from("active"), SessionStatus::Active);
    assert_eq!(SessionStatus::from("revoked").as_str(), "revoked");
    let unknown = SessionStatus::from("brand_new_status");
    assert!(matches!(unknown, SessionStatus::Other(_)));
    assert_eq!(unknown.as_str(), "brand_new_status");
    // A known status string is always canonicalized to its named variant, so
    // an `Other` can never alias one.
    assert_eq!(
        SessionStatus::from("active".to_string()),
        SessionStatus::Active
    );
    assert_eq!(SessionStatus::from("pending").to_string(), "pending");

    let session = Session::new("sess_2", "expired");
    let json = serde_json::to_string(&session).unwrap();
    let back: Session = serde_json::from_str(&json).unwrap();
    assert_eq!(back.status, SessionStatus::Expired);
    assert!(!back.is_active());
}

#[test]
fn user_deserializes_clerk_js_object_form_primary_email_address() {
    let json = r#"{
        "id": "user_2abc",
        "primaryEmailAddress": { "id": "idn_1", "emailAddress": "ada@example.com" }
    }"#;
    let user: User = serde_json::from_str(json).unwrap();

    assert_eq!(
        user.primary_email_address.as_deref(),
        Some("ada@example.com")
    );
}

#[test]
fn signed_in_snapshot_converts_back_to_auth_snapshot_before_clerk_js_loads() {
    let state = signed_in_snapshot_state();

    let snapshot = state.to_state();

    assert!(!snapshot.is_loaded);
    assert!(snapshot.is_signed_in());
    assert_eq!(snapshot.user_id.as_deref(), Some("user_2abc"));
    assert_eq!(snapshot.session_id.as_deref(), Some("sess_2def"));
    assert_eq!(snapshot.org_id.as_deref(), Some("org_2ghi"));
}

#[test]
fn auth_state_carries_loadedness_into_snapshot_without_external_signal() {
    let state = AuthRuntimeState::from_initial_auth_snapshot(&InitialAuthSnapshot {
        status: crate::ssr::InitialAuthStatus::SignedIn,
        user_id: Some("user_2abc".into()),
        session_id: Some("sess_2def".into()),
        org_id: Some("org_2ghi".into()),
        org_slug: Some("acme".into()),
        org_role: None,
        org_permissions: vec![],
    });

    assert!(!state.is_loaded());
    assert!(!state.to_state().is_loaded);
    assert!(state.to_state().is_signed_in());

    let loaded = state.apply_loaded_observation(AuthObservation::SignedIn {
        user: User {
            id: "user_2abc".into(),
            first_name: None,
            last_name: None,
            primary_email_address: None,
            image_url: None,
        },
        session: Session {
            id: "sess_2def".into(),
            status: "active".into(),
            last_active_organization_id: Some("org_2ghi".into()),
            last_active_at: None,
            expire_at: None,
            current_task: None,
            tasks: Vec::new(),
        },
    });

    assert!(loaded.is_loaded());
    assert!(loaded.to_state().is_loaded);
    assert!(loaded.to_state().is_signed_in());
    assert_eq!(loaded.to_state().org_id.as_deref(), Some("org_2ghi"));
}

#[test]
fn loaded_signed_out_observation_marks_auth_state_loaded() {
    let loaded = AuthRuntimeState::loading().apply_loaded_observation(AuthObservation::SignedOut);

    assert!(loaded.is_loaded());
    assert!(!loaded.to_state().is_signed_in());
    assert!(loaded.should_render_signed_out());
}

#[test]
fn signed_in_full_state_converts_to_snapshot_with_org_id() {
    let state = signed_in_state();

    let snapshot = state.to_state();

    assert!(snapshot.is_loaded);
    assert!(snapshot.is_signed_in());
    assert_eq!(snapshot.user_id.as_deref(), Some("user_2abc"));
    assert_eq!(snapshot.session_id.as_deref(), Some("sess_2def"));
    assert_eq!(snapshot.org_id.as_deref(), Some("org_2ghi"));
}

#[test]
fn malformed_signed_in_snapshot_without_user_id_stays_loading() {
    let snapshot = InitialAuthSnapshot {
        status: crate::ssr::InitialAuthStatus::SignedIn,
        user_id: None,
        session_id: Some("sess_2def".into()),
        org_id: None,
        org_slug: None,
        org_role: None,
        org_permissions: vec![],
    };

    let state = AuthRuntimeState::from_initial_auth_snapshot(&snapshot);

    assert!(!state.is_loaded());
    assert!(!state.to_state().is_signed_in());
}

#[test]
fn malformed_signed_in_snapshot_with_empty_user_id_stays_loading() {
    let snapshot = InitialAuthSnapshot {
        status: crate::ssr::InitialAuthStatus::SignedIn,
        user_id: Some(String::new()),
        session_id: Some("sess_2def".into()),
        org_id: None,
        org_slug: None,
        org_role: None,
        org_permissions: vec![],
    };

    let state = AuthRuntimeState::from_initial_auth_snapshot(&snapshot);

    assert!(!state.is_loaded());
    assert_eq!(state.to_state().status, AuthStatus::Loading);
}

#[test]
fn unverified_snapshot_stays_loading_so_signed_in_users_never_flash_signed_out() {
    let snapshot = InitialAuthSnapshot {
        status: crate::ssr::InitialAuthStatus::Unverified,
        user_id: None,
        session_id: None,
        org_id: None,
        org_slug: None,
        org_role: None,
        org_permissions: vec![],
    };

    let state = AuthRuntimeState::from_initial_auth_snapshot(&snapshot);

    assert!(!state.is_loaded());
    assert_eq!(state.to_state().status, AuthStatus::Loading);
    assert!(!state.to_state().is_signed_out());
}

#[test]
fn full_js_state_preserves_org_id_from_signed_in_snapshot() {
    let previous = signed_in_snapshot_state();

    let state = AuthRuntimeState::from_js_session(
        sample_user("user_2abc"),
        sample_session_in_org("sess_2def", Some("org_2ghi")),
        &previous,
    );

    assert_eq!(state.to_state().org_id.as_deref(), Some("org_2ghi"));
}

#[test]
fn full_js_state_preserves_verified_claims_from_previous_matching_full_signed_in_state() {
    let previous = signed_in_state();

    let state = AuthRuntimeState::from_js_session(
        sample_user("user_2abc"),
        sample_session_in_org("sess_2def", Some("org_2ghi")),
        &previous,
    );

    assert_eq!(state.to_state().org_id.as_deref(), Some("org_2ghi"));
}

#[test]
fn full_js_state_drops_org_claims_when_active_organization_changes() {
    let previous = AuthRuntimeState::from_initial_auth_snapshot(&InitialAuthSnapshot {
        status: crate::ssr::InitialAuthStatus::SignedIn,
        user_id: Some("user_2abc".into()),
        session_id: Some("sess_2def".into()),
        org_id: Some("org_2ghi".into()),
        org_slug: Some("acme".into()),
        org_role: Some("admin".into()),
        org_permissions: vec!["org:read".into()],
    });

    // clerk-js setActive({ organization }) switches org without a new session.
    let state = AuthRuntimeState::from_js_session(
        sample_user("user_2abc"),
        sample_session_in_org("sess_2def", Some("org_other")),
        &previous,
    );

    let snapshot = state.to_state();
    assert_eq!(snapshot.org_id, None);
    assert_eq!(snapshot.org_role, None);
    assert!(snapshot.org_permissions.is_empty());
    assert!(!state.allows_signed_in_gate(Some("admin"), None));
}

#[test]
fn full_js_state_drops_org_claims_when_session_leaves_organization() {
    let previous = AuthRuntimeState::from_initial_auth_snapshot(&InitialAuthSnapshot {
        status: crate::ssr::InitialAuthStatus::SignedIn,
        user_id: Some("user_2abc".into()),
        session_id: Some("sess_2def".into()),
        org_id: Some("org_2ghi".into()),
        org_slug: Some("acme".into()),
        org_role: Some("admin".into()),
        org_permissions: vec!["org:read".into()],
    });

    let state = AuthRuntimeState::from_js_session(
        sample_user("user_2abc"),
        sample_session("sess_2def"),
        &previous,
    );

    let snapshot = state.to_state();
    assert_eq!(snapshot.org_id, None);
    assert!(!state.allows_signed_in_gate(None, Some("org:read")));
}

#[test]
fn full_js_state_drops_snapshot_org_id_when_user_changes() {
    let previous = AuthRuntimeState::from_initial_auth_snapshot(&InitialAuthSnapshot {
        status: crate::ssr::InitialAuthStatus::SignedIn,
        user_id: Some("user_old".into()),
        session_id: Some("sess_2def".into()),
        org_id: Some("org_stale".into()),
        org_slug: Some("stale".into()),
        org_role: None,
        org_permissions: vec![],
    });

    let state = AuthRuntimeState::from_js_session(
        sample_user("user_new"),
        sample_session("sess_2def"),
        &previous,
    );

    assert_eq!(state.to_state().org_id, None);
}

#[test]
fn full_js_state_drops_snapshot_org_id_when_session_changes() {
    let previous = AuthRuntimeState::from_initial_auth_snapshot(&InitialAuthSnapshot {
        status: crate::ssr::InitialAuthStatus::SignedIn,
        user_id: Some("user_2abc".into()),
        session_id: Some("sess_old".into()),
        org_id: Some("org_stale".into()),
        org_slug: Some("stale".into()),
        org_role: None,
        org_permissions: vec![],
    });

    let state = AuthRuntimeState::from_js_session(
        sample_user("user_2abc"),
        sample_session("sess_new"),
        &previous,
    );

    assert_eq!(state.to_state().org_id, None);
}

#[test]
fn signed_in_observation_preserves_matching_snapshot_org_id() {
    let previous = signed_in_snapshot_state();

    let next = previous.apply_observation(AuthObservation::SignedIn {
        user: sample_user("user_2abc"),
        session: sample_session_in_org("sess_2def", Some("org_2ghi")),
    });

    assert_eq!(next.to_state().org_id.as_deref(), Some("org_2ghi"));
}

#[test]
fn signed_out_observation_replaces_previous_signed_in_state() {
    let previous = signed_in_snapshot_state();

    let next = previous.apply_observation(AuthObservation::SignedOut);

    assert!(!next.is_loaded());
    assert!(!next.to_state().is_signed_in());
    assert!(next.should_render_signed_out());
}

#[test]
fn loaded_loading_observation_preserves_signed_in_snapshot() {
    let previous = signed_in_snapshot_state();

    let next = previous.apply_loaded_observation(AuthObservation::Loading);
    let snapshot = next.to_state();

    assert!(next.is_loaded());
    assert!(snapshot.is_signed_in());
    assert_eq!(snapshot.user_id.as_deref(), Some("user_2abc"));
    assert!(next.should_render_signed_in());
    assert!(!next.should_render_signed_out());
}

#[test]
fn loaded_loading_observation_preserves_full_signed_in_state() {
    let previous = signed_in_state();

    let next = previous.apply_loaded_observation(AuthObservation::Loading);
    let snapshot = next.to_state();

    assert!(next.is_loaded());
    assert!(snapshot.is_signed_in());
    assert_eq!(snapshot.user_id.as_deref(), Some("user_2abc"));
    assert!(next.user().is_some());
    assert!(next.session().is_some());
    assert!(next.should_render_signed_in());
    assert!(!next.should_render_signed_out());
}

#[test]
fn loading_observation_replaces_previous_state_with_loading() {
    let previous =
        AuthRuntimeState::from_initial_auth_snapshot(&signed_out_initial_auth_snapshot());

    let next = previous.apply_observation(AuthObservation::Loading);

    assert!(!next.is_loaded());
    assert!(!next.to_state().is_signed_in());
    assert!(!next.should_render_signed_in());
    assert!(!next.should_render_signed_out());
}

#[test]
fn signed_in_gate_allows_plain_snapshot_and_full_state() {
    let snapshot = AuthRuntimeState::from_initial_auth_snapshot(&InitialAuthSnapshot {
        status: crate::ssr::InitialAuthStatus::SignedIn,
        user_id: Some("user_2abc".into()),
        session_id: Some("sess_2def".into()),
        org_id: None,
        org_slug: None,
        org_role: None,
        org_permissions: vec![],
    });
    let full = AuthRuntimeState::from_js_session(
        sample_user("user_2abc"),
        sample_session("sess_2def"),
        &snapshot,
    );

    assert!(snapshot.allows_signed_in_gate(None, None));
    assert!(full.allows_signed_in_gate(None, None));
    assert!(
        !AuthRuntimeState::from_initial_auth_snapshot(&signed_out_initial_auth_snapshot())
            .allows_signed_in_gate(None, None)
    );
    assert!(!AuthRuntimeState::loading().allows_signed_in_gate(None, None));
}

#[test]
fn signed_in_gate_denies_role_or_permission_when_snapshot_has_no_claims() {
    let state = AuthRuntimeState::from_initial_auth_snapshot(&InitialAuthSnapshot {
        status: crate::ssr::InitialAuthStatus::SignedIn,
        user_id: Some("user_2abc".into()),
        session_id: Some("sess_2def".into()),
        org_id: None,
        org_slug: None,
        org_role: None,
        org_permissions: vec![],
    });

    assert!(!state.allows_signed_in_gate(Some("admin"), None));
    assert!(!state.allows_signed_in_gate(None, Some("org:read")));
    assert!(!state.allows_signed_in_gate(Some("admin"), Some("org:read")));
}

#[test]
fn signed_in_gate_allows_matching_server_verified_role_and_permission() {
    let state = AuthRuntimeState::from_initial_auth_snapshot(&InitialAuthSnapshot {
        status: crate::ssr::InitialAuthStatus::SignedIn,
        user_id: Some("user_2abc".into()),
        session_id: Some("sess_2def".into()),
        org_id: Some("org_2ghi".into()),
        org_slug: Some("acme".into()),
        org_role: Some("admin".into()),
        org_permissions: vec!["org:read".into(), "org:write".into()],
    });

    assert!(state.allows_signed_in_gate(Some("admin"), None));
    assert!(state.allows_signed_in_gate(None, Some("org:read")));
    assert!(state.allows_signed_in_gate(Some("admin"), Some("org:write")));
}

#[test]
fn signed_in_gate_denies_mismatched_server_verified_role_or_permission() {
    let state = AuthRuntimeState::from_initial_auth_snapshot(&InitialAuthSnapshot {
        status: crate::ssr::InitialAuthStatus::SignedIn,
        user_id: Some("user_2abc".into()),
        session_id: Some("sess_2def".into()),
        org_id: Some("org_2ghi".into()),
        org_slug: Some("acme".into()),
        org_role: Some("admin".into()),
        org_permissions: vec!["org:read".into()],
    });

    assert!(!state.allows_signed_in_gate(Some("basic_member"), None));
    assert!(!state.allows_signed_in_gate(None, Some("org:delete")));
    assert!(!state.allows_signed_in_gate(Some("admin"), Some("org:delete")));
}

#[test]
fn signed_in_gate_prefers_permission_over_role_when_both_are_requested() {
    let state = AuthRuntimeState::from_initial_auth_snapshot(&InitialAuthSnapshot {
        status: crate::ssr::InitialAuthStatus::SignedIn,
        user_id: Some("user_2abc".into()),
        session_id: Some("sess_2def".into()),
        org_id: Some("org_2ghi".into()),
        org_slug: Some("acme".into()),
        org_role: Some("admin".into()),
        org_permissions: vec!["org:read".into()],
    });

    // Matching Clerk React: a mismatched role is ignored when a permission
    // gate is requested alongside it.
    assert!(state.allows_signed_in_gate(Some("basic_member"), Some("org:read")));
    assert!(!state.allows_signed_in_gate(Some("admin"), Some("org:delete")));
}

#[test]
fn signed_in_gate_preserves_server_verified_claims_after_matching_js_session() {
    let snapshot = AuthRuntimeState::from_initial_auth_snapshot(&InitialAuthSnapshot {
        status: crate::ssr::InitialAuthStatus::SignedIn,
        user_id: Some("user_2abc".into()),
        session_id: Some("sess_2def".into()),
        org_id: Some("org_2ghi".into()),
        org_slug: Some("acme".into()),
        org_role: Some("admin".into()),
        org_permissions: vec!["org:read".into()],
    });
    let full = AuthRuntimeState::from_js_session(
        sample_user("user_2abc"),
        sample_session_in_org("sess_2def", Some("org_2ghi")),
        &snapshot,
    );

    assert!(full.allows_signed_in_gate(Some("admin"), Some("org:read")));
}

#[test]
fn signed_in_gate_drops_server_verified_claims_when_snapshot_session_is_missing() {
    let snapshot = AuthRuntimeState::from_initial_auth_snapshot(&InitialAuthSnapshot {
        status: crate::ssr::InitialAuthStatus::SignedIn,
        user_id: Some("user_2abc".into()),
        session_id: None,
        org_id: Some("org_2ghi".into()),
        org_slug: Some("acme".into()),
        org_role: Some("admin".into()),
        org_permissions: vec!["org:read".into()],
    });

    let full = AuthRuntimeState::from_js_session(
        sample_user("user_2abc"),
        sample_session("sess_from_js"),
        &snapshot,
    );

    let full_snapshot = full.to_state();
    assert_eq!(full_snapshot.org_id, None);
    assert_eq!(full_snapshot.org_role, None);
    assert!(full_snapshot.org_permissions.is_empty());
    assert!(!full.allows_signed_in_gate(Some("admin"), Some("org:read")));
}

#[test]
fn signed_in_gate_preserves_server_verified_claims_after_repeated_matching_js_sessions() {
    let snapshot = AuthRuntimeState::from_initial_auth_snapshot(&InitialAuthSnapshot {
        status: crate::ssr::InitialAuthStatus::SignedIn,
        user_id: Some("user_2abc".into()),
        session_id: Some("sess_2def".into()),
        org_id: Some("org_2ghi".into()),
        org_slug: Some("acme".into()),
        org_role: Some("admin".into()),
        org_permissions: vec!["org:read".into()],
    });
    let hydrated = snapshot.apply_observation(AuthObservation::SignedIn {
        user: sample_user("user_2abc"),
        session: sample_session_in_org("sess_2def", Some("org_2ghi")),
    });

    let updated = hydrated.apply_observation(AuthObservation::SignedIn {
        user: sample_user("user_2abc"),
        session: sample_session_in_org("sess_2def", Some("org_2ghi")),
    });

    let updated_snapshot = updated.to_state();
    assert!(updated.allows_signed_in_gate(Some("admin"), Some("org:read")));
    assert_eq!(updated_snapshot.org_id.as_deref(), Some("org_2ghi"));
    assert_eq!(updated_snapshot.org_role.as_deref(), Some("admin"));
    assert_eq!(updated_snapshot.org_permissions, ["org:read"]);
}

#[test]
fn signed_in_gate_denies_role_or_permission_without_server_verified_claims() {
    let state = AuthRuntimeState::from_js_session(
        sample_user("user_2abc"),
        sample_session("sess_2def"),
        &AuthRuntimeState::loading(),
    );

    assert!(state.allows_signed_in_gate(None, None));
    assert!(!state.allows_signed_in_gate(Some("admin"), None));
    assert!(!state.allows_signed_in_gate(None, Some("org:read")));
}

#[test]
fn pending_session_gates_as_signed_out_but_exposes_current_task() {
    let previous = signed_in_snapshot_state();
    let session = pending_session_with_task("sess_2def", SessionTaskKey::SetupMfa);

    let next = previous.apply_loaded_observation(AuthObservation::Pending {
        user: sample_user("user_2abc"),
        session: session.clone(),
    });

    // Treated as signed-out for gating, matching clerk-js's
    // `treatPendingAsSignedOut: true` default.
    assert!(!next.should_render_signed_in());
    assert!(next.should_render_signed_out());
    assert!(!next.allows_signed_in_gate(None, None));
    assert_eq!(next.to_state().status, AuthStatus::SignedOut);
    assert!(!next.to_state().is_signed_in());

    // But the pending session and its current task are available for routing.
    assert!(next.is_loaded());
    assert_eq!(next.session(), Some(&session));
    assert_eq!(
        next.session()
            .and_then(|s| s.current_task.as_ref())
            .map(|task| &task.key),
        Some(&SessionTaskKey::SetupMfa)
    );
}

#[test]
fn resolve_pending_opt_out_reads_pending_session_as_signed_in() {
    let pending = signed_in_snapshot_state().apply_loaded_observation(AuthObservation::Pending {
        user: sample_user("user_2abc"),
        session: pending_session_with_task("sess_2def", SessionTaskKey::SetupMfa),
    });

    // Default (treat_pending_as_signed_out = true): gates as signed-out.
    let default = pending.resolve_pending(true);
    assert!(!default.should_render_signed_in());
    assert!(default.should_render_signed_out());
    assert!(!default.to_state().is_signed_in());

    // Opt out: the pending session is read as signed-in, mirroring clerk-js
    // `resolveAuthState` with `treatPendingAsSignedOut: false`.
    let opted_out = pending.resolve_pending(false);
    assert!(opted_out.should_render_signed_in());
    assert!(!opted_out.should_render_signed_out());
    assert!(opted_out.allows_signed_in_gate(None, None));
    assert!(opted_out.to_state().is_signed_in());
    assert_eq!(opted_out.to_state().user_id.as_deref(), Some("user_2abc"));
    // Role/permission gates still fail closed: a pending session carries no
    // server-verified org claims.
    assert!(!opted_out.allows_signed_in_gate(Some("admin"), None));
    // The session (and its current_task) stays available under either flag.
    assert!(opted_out.session().is_some());
    assert!(default.session().is_some());
}

#[test]
fn resolve_pending_is_a_no_op_for_non_pending_states() {
    let signed_in = signed_in_state();
    assert!(signed_in.resolve_pending(false).should_render_signed_in());
    assert!(signed_in.resolve_pending(true).should_render_signed_in());

    let signed_out =
        AuthRuntimeState::from_initial_auth_snapshot(&signed_out_initial_auth_snapshot());
    assert!(signed_out.resolve_pending(false).should_render_signed_out());
    assert!(!signed_out.resolve_pending(false).should_render_signed_in());
}

#[test]
fn loaded_loading_observation_preserves_pending_session() {
    // A transient interim clerk-js event must not drop a resolved pending
    // session back to loading and flash the wrong UI.
    let pending = signed_in_snapshot_state().apply_loaded_observation(AuthObservation::Pending {
        user: sample_user("user_2abc"),
        session: pending_session_with_task("sess_2def", SessionTaskKey::SetupMfa),
    });

    let next = pending.apply_loaded_observation(AuthObservation::Loading);

    assert!(next.is_pending());
    assert!(next.should_render_signed_out());
    assert!(!next.should_render_signed_in());
    assert!(next.session().is_some());
}

#[test]
fn loading_renders_neither_signed_in_nor_signed_out() {
    let state = AuthRuntimeState::loading();

    assert!(!state.should_render_signed_in());
    assert!(!state.should_render_signed_out());
}

#[test]
fn signed_out_renders_only_signed_out() {
    let state = AuthRuntimeState::from_initial_auth_snapshot(&signed_out_initial_auth_snapshot());

    assert!(!state.should_render_signed_in());
    assert!(state.should_render_signed_out());
}

#[test]
fn signed_in_snapshot_renders_signed_in_without_full_user_or_session() {
    let state = signed_in_snapshot_state();

    assert!(state.should_render_signed_in());
    assert!(!state.should_render_signed_out());
    assert!(state.user().is_none());
    assert!(state.session().is_none());
}
