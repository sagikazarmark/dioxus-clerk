//! Internal verified Clerk JWT claim conversion.

use crate::core::{ClerkAuth, InvalidTokenReason, VerificationOutcome};
use jwt_simple::prelude::{JWTClaims, UnixTimeStamp};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct ClerkJwtClaims {
    #[serde(default, rename = "azp")]
    authorized_party: Option<String>,
    #[serde(default, rename = "sid")]
    session_id: Option<String>,
    #[serde(default, rename = "fea")]
    features: Option<String>,
    #[serde(default, rename = "o")]
    organization: Option<ClerkV2OrganizationClaims>,
    #[serde(default)]
    org_id: Option<String>,
    #[serde(default)]
    org_slug: Option<String>,
    #[serde(default)]
    org_role: Option<String>,
    #[serde(default)]
    org_permissions: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ClerkV2OrganizationClaims {
    id: String,
    #[serde(default, rename = "slg")]
    slug: Option<String>,
    #[serde(default, rename = "rol")]
    role: Option<String>,
    #[serde(default, rename = "per")]
    permissions: Option<String>,
    #[serde(default, rename = "fpm")]
    feature_permission_map: Option<String>,
}

#[derive(Default)]
struct OrganizationAuthClaims {
    id: Option<String>,
    slug: Option<String>,
    role: Option<String>,
    permissions: Vec<String>,
}

impl ClerkJwtClaims {
    pub(super) fn authorized_party(&self) -> Option<&str> {
        self.authorized_party.as_deref()
    }

    /// The session id (`sid`) claim, treating an empty string as absent. Clerk
    /// session tokens carry it; JWT-template tokens do not.
    pub(super) fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref().filter(|sid| !sid.is_empty())
    }
}

impl ClerkV2OrganizationClaims {
    fn into_auth_claims(self, features: Option<&str>) -> OrganizationAuthClaims {
        if self.id.is_empty() {
            return OrganizationAuthClaims::default();
        }
        let permissions = self.decode_permissions(features);

        OrganizationAuthClaims {
            id: Some(self.id),
            slug: self.slug.filter(|slug| !slug.is_empty()),
            role: self.role.filter(|role| !role.is_empty()).map(|role| {
                if role.starts_with("org:") {
                    role
                } else {
                    format!("org:{role}")
                }
            }),
            permissions,
        }
    }

    fn decode_permissions(&self, features: Option<&str>) -> Vec<String> {
        let (Some(features), Some(permissions), Some(feature_permission_map)) = (
            features,
            self.permissions.as_deref(),
            self.feature_permission_map.as_deref(),
        ) else {
            return vec![];
        };
        let features: Vec<_> = features.split(',').collect();
        let permissions: Vec<_> = permissions.split(',').collect();
        if features.iter().any(|feature| {
            feature.is_empty()
                || feature.strip_prefix("o:") == Some("")
                || feature.strip_prefix("u:") == Some("")
        }) || permissions.is_empty()
            || permissions.len() > u128::BITS as usize
            || permissions.iter().any(|permission| permission.is_empty())
        {
            return vec![];
        }
        let Some(feature_permission_map) = feature_permission_map
            .split(',')
            .map(|mask| mask.parse::<u128>())
            .collect::<Result<Vec<_>, _>>()
            .ok()
        else {
            return vec![];
        };
        // `fpm` stores one decimal mask per feature; bit N (least-significant
        // first) corresponds to permission N in `per`.
        let valid_permission_bits = if permissions.len() == u128::BITS as usize {
            u128::MAX
        } else {
            (1 << permissions.len()) - 1
        };
        if features.len() != feature_permission_map.len()
            || feature_permission_map
                .iter()
                .any(|mask| mask & !valid_permission_bits != 0)
        {
            return vec![];
        }

        features
            .into_iter()
            .zip(feature_permission_map)
            .filter_map(|(feature, mask)| feature.strip_prefix("o:").map(|feature| (feature, mask)))
            .flat_map(|(feature, mask)| {
                permissions
                    .iter()
                    .enumerate()
                    .filter_map(move |(index, permission)| {
                        if mask & (1 << index) != 0 {
                            Some(format!("org:{feature}:{permission}"))
                        } else {
                            None
                        }
                    })
            })
            .collect()
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
        let ClerkJwtClaims {
            authorized_party: _,
            session_id,
            features,
            organization,
            org_id,
            org_slug,
            org_role,
            org_permissions,
        } = jwt.custom;
        let organization = match organization {
            Some(organization) => organization.into_auth_claims(features.as_deref()),
            None => OrganizationAuthClaims {
                id: org_id,
                slug: org_slug,
                role: org_role,
                permissions: org_permissions,
            },
        };
        Some(ClerkAuth {
            user_id,
            session_id,
            org_id: organization.id,
            org_slug: organization.slug,
            org_role: organization.role,
            org_permissions: organization.permissions,
            // `exp` is required: a session token must expire. `nbf`/`iat`
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
            "org_role": "org:admin",
            "org_permissions": ["org:dashboard:read", "org:teams:manage"],
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
                assert_eq!(auth.org_role.as_deref(), Some("org:admin"));
                assert_eq!(
                    auth.org_permissions,
                    vec!["org:dashboard:read", "org:teams:manage"]
                );
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
