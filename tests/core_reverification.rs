use dioxus_clerk::core::{ClerkError, ReverificationLevel};
use serde_json::json;

fn needs_reverification_level(value: &serde_json::Value) -> Option<ReverificationLevel> {
    match ClerkError::from_reverification_hint(value) {
        Some(ClerkError::NeedsReverification { level }) => level,
        other => panic!("expected NeedsReverification, got {other:?}"),
    }
}

// clerk-js / Clerk backend signals "step-up reverification required" with a 403
// hint of this exact shape (see @clerk/shared `isReverificationHint`): a
// `clerk_error` with `type: "forbidden"`, `reason: "reverification-error"`, and
// the required factor level under `metadata.reverification.level`.
#[test]
fn reverification_hint_maps_to_needs_reverification_with_level() {
    let hint = json!({
        "clerk_error": {
            "type": "forbidden",
            "reason": "reverification-error",
            "metadata": {
                "reverification": { "level": "second_factor" }
            }
        }
    });

    assert_eq!(
        ClerkError::from_reverification_hint(&hint),
        Some(ClerkError::NeedsReverification {
            level: Some(ReverificationLevel::SecondFactor)
        })
    );
}

// The parser must not swallow unrelated failures: only a genuine
// reverification hint maps, everything else passes through as `None` so the
// caller can keep its own error.
#[test]
fn non_reverification_values_do_not_map() {
    // A different `forbidden` reason (e.g. plain authorization failure).
    let other_forbidden = json!({
        "clerk_error": { "type": "forbidden", "reason": "authorization-error" }
    });
    assert_eq!(ClerkError::from_reverification_hint(&other_forbidden), None);

    // Arbitrary non-hint payloads.
    assert_eq!(
        ClerkError::from_reverification_hint(
            &json!({ "errors": [{ "code": "form_password_incorrect" }] })
        ),
        None
    );
    assert_eq!(ClerkError::from_reverification_hint(&json!("nope")), None);
    assert_eq!(ClerkError::from_reverification_hint(&json!(null)), None);
}

// A hint is still a valid reverification signal when clerk omits the level, and
// a level string this crate has not named round-trips through `Other` rather
// than being dropped.
#[test]
fn hint_level_is_optional_and_unknown_levels_round_trip() {
    let no_level = json!({
        "clerk_error": { "type": "forbidden", "reason": "reverification-error" }
    });
    assert_eq!(needs_reverification_level(&no_level), None);

    let unknown = json!({
        "clerk_error": {
            "type": "forbidden",
            "reason": "reverification-error",
            "metadata": { "reverification": { "level": "passkey" } }
        }
    });
    let level = needs_reverification_level(&unknown).expect("level present");
    assert_eq!(level.as_str(), "passkey");
    assert!(matches!(level, ReverificationLevel::Other(_)));
}
