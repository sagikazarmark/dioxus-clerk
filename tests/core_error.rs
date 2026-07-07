use dioxus_clerk::core::{ClerkError, InvalidTokenReason};

#[test]
fn error_is_send_sync_static() {
    fn assert_send_sync_static<T: Send + Sync + 'static>() {}
    assert_send_sync_static::<ClerkError>();
}

#[test]
fn error_display_messages_are_stable_per_variant() {
    let cases = [
        (ClerkError::NotLoaded, "clerk has not finished loading"),
        (
            ClerkError::UnsupportedTarget,
            "clerk-js is not available on this build target",
        ),
        (
            ClerkError::ScriptLoad("network".into()),
            "clerk-js failed to load: network",
        ),
        (ClerkError::Unauthenticated, "unauthenticated"),
        (ClerkError::TokenExpired, "session token expired"),
        (
            ClerkError::JwksUnavailable("fetch failed".into()),
            "clerk jwks unavailable: fetch failed",
        ),
        (
            ClerkError::InvalidConfig("missing key".into()),
            "invalid clerk configuration: missing key",
        ),
        (ClerkError::Js("boom".into()), "clerk js error: boom"),
    ];

    for (error, expected) in cases {
        assert_eq!(error.to_string(), expected);
    }

    assert!(
        ClerkError::NoServerContext
            .to_string()
            .contains("no server context available")
    );
}

#[test]
fn invalid_token_reasons_map_to_actionable_errors() {
    assert_eq!(
        ClerkError::from(InvalidTokenReason::Expired),
        ClerkError::TokenExpired
    );
    assert_eq!(
        ClerkError::from(InvalidTokenReason::NotYetValid),
        ClerkError::Unauthenticated
    );
    assert_eq!(
        ClerkError::from(InvalidTokenReason::Other),
        ClerkError::Unauthenticated
    );
}

#[test]
fn errors_have_no_source_by_design() {
    // Causes are flattened into the message at the boundary where they occur;
    // `source()` staying `None` is part of the documented design.
    let error = ClerkError::ScriptLoad("network".into());
    assert!(std::error::Error::source(&error).is_none());
}
