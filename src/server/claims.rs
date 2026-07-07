//! Internal verified Clerk JWT claim conversion.

use crate::core::{ClerkAuth, InvalidTokenReason, VerificationOutcome};
use jwt_simple::prelude::{JWTClaims, UnixTimeStamp};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct ClerkJwtClaims {
    #[serde(default)]
    azp: Option<String>,
    #[serde(default)]
    sid: Option<String>,
    #[serde(default)]
    org_id: Option<String>,
    #[serde(default)]
    org_slug: Option<String>,
    #[serde(default)]
    org_role: Option<String>,
    #[serde(default)]
    org_permissions: Vec<String>,
}

impl ClerkJwtClaims {
    pub(super) fn authorized_party(&self) -> Option<&str> {
        self.azp.as_deref()
    }

    /// The session id (`sid`) claim, treating an empty string as absent. Clerk
    /// session tokens carry it; JWT-template tokens do not.
    pub(super) fn session_id(&self) -> Option<&str> {
        self.sid.as_deref().filter(|sid| !sid.is_empty())
    }
}

#[derive(Debug)]
pub(super) struct VerifiedClerkClaims {
    jwt: JWTClaims<ClerkJwtClaims>,
}

impl VerifiedClerkClaims {
    pub(super) fn new(jwt: JWTClaims<ClerkJwtClaims>) -> Self {
        Self { jwt }
    }

    pub(super) fn into_outcome(self) -> VerificationOutcome {
        self.into_auth()
            .map(VerificationOutcome::Valid)
            .unwrap_or(VerificationOutcome::Invalid(InvalidTokenReason::Other))
    }

    fn into_auth(self) -> Option<ClerkAuth> {
        let jwt = self.jwt;
        let user_id = jwt.subject.filter(|sub| !sub.is_empty())?;
        Some(ClerkAuth {
            user_id,
            session_id: jwt.custom.sid,
            org_id: jwt.custom.org_id,
            org_slug: jwt.custom.org_slug,
            org_role: jwt.custom.org_role,
            org_permissions: jwt.custom.org_permissions,
            // `exp` is required — a session token must expire. `nbf`/`iat`
            // are optional per RFC 7519, so a signature-valid token that
            // omits them must not be rejected here; default them to 0.
            exp: timestamp_seconds(jwt.expires_at)?,
            nbf: timestamp_seconds(jwt.invalid_before).unwrap_or(0),
            iat: timestamp_seconds(jwt.issued_at).unwrap_or(0),
        })
    }
}

fn timestamp_seconds(timestamp: Option<UnixTimeStamp>) -> Option<i64> {
    let seconds = timestamp?.as_secs();
    i64::try_from(seconds).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verified_claims_produce_auth_with_all_clerk_auth_fields() {
        let claims = verified_claims(serde_json::json!({
            "sub": "user_2abc",
            "sid": "sess_2def",
            "org_id": "org_2ghi",
            "org_slug": "acme",
            "org_role": "admin",
            "org_permissions": ["org:read", "org:write"],
            "exp": 1_700_000_600,
            "nbf": 1_700_000_000,
            "iat": 1_699_999_900,
        }));

        let outcome = claims.into_outcome();

        match outcome {
            VerificationOutcome::Valid(auth) => {
                assert_eq!(auth.user_id, "user_2abc");
                assert_eq!(auth.session_id.as_deref(), Some("sess_2def"));
                assert_eq!(auth.org_id.as_deref(), Some("org_2ghi"));
                assert_eq!(auth.org_slug.as_deref(), Some("acme"));
                assert_eq!(auth.org_role.as_deref(), Some("admin"));
                assert_eq!(auth.org_permissions, vec!["org:read", "org:write"]);
                assert_eq!(auth.exp, 1_700_000_600);
                assert_eq!(auth.nbf, 1_700_000_000);
                assert_eq!(auth.iat, 1_699_999_900);
            }
            VerificationOutcome::Missing
            | VerificationOutcome::Invalid(_)
            | VerificationOutcome::Unavailable => panic!("complete claims should be valid"),
        }
    }

    #[test]
    fn verified_claims_missing_subject_produce_invalid_outcome() {
        let claims = verified_claims(serde_json::json!({
            "sid": "sess_2def",
            "exp": 1_700_000_600,
            "nbf": 1_700_000_000,
            "iat": 1_699_999_900,
        }));

        let outcome = claims.into_outcome();

        assert!(matches!(outcome, VerificationOutcome::Invalid(_)));
    }

    #[test]
    fn verified_claims_empty_subject_produce_invalid_outcome() {
        let claims = verified_claims(serde_json::json!({
            "sub": "",
            "sid": "sess_2def",
            "exp": 1_700_000_600,
            "nbf": 1_700_000_000,
            "iat": 1_699_999_900,
        }));

        let outcome = claims.into_outcome();

        assert!(matches!(outcome, VerificationOutcome::Invalid(_)));
    }

    #[test]
    fn verified_claims_missing_exp_produce_invalid_outcome() {
        let claims = verified_claims(serde_json::json!({
            "sub": "user_2abc",
            "sid": "sess_2def",
            "nbf": 1_700_000_000,
            "iat": 1_699_999_900,
        }));

        let outcome = claims.into_outcome();

        assert!(matches!(outcome, VerificationOutcome::Invalid(_)));
    }

    #[test]
    fn verified_claims_missing_nbf_iat_default_to_zero() {
        let claims = verified_claims(serde_json::json!({
            "sub": "user_2abc",
            "exp": 1_700_000_600,
        }));

        match claims.into_outcome() {
            VerificationOutcome::Valid(auth) => {
                assert_eq!(auth.exp, 1_700_000_600);
                assert_eq!(auth.nbf, 0);
                assert_eq!(auth.iat, 0);
            }
            other => panic!("optional nbf/iat should not reject a valid token: {other:?}"),
        }
    }

    fn verified_claims(claims: serde_json::Value) -> VerifiedClerkClaims {
        VerifiedClerkClaims::new(
            serde_json::from_value::<JWTClaims<ClerkJwtClaims>>(claims).unwrap(),
        )
    }
}
