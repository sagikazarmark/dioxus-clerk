use std::collections::HashSet;
use std::time::Duration;

use ct_codecs::{Base64UrlSafeNoPadding, Decoder};
use jwk_simple::{Algorithm, KeyMatcher, KeyOperation, KeySet};
use jwt_simple::JWTError;
use jwt_simple::prelude::RSAPublicKeyLike;
use jwt_simple::prelude::{Duration as JwtDuration, RS256PublicKey, VerificationOptions};
use serde::Deserialize;

use crate::core::InvalidTokenReason;
use crate::server::claims::{ClerkJwtClaims, VerifiedClerkClaims};

use super::VerificationFailure;

const JWT_HEADER_TYPE: &str = "JWT";
const JWT_ALGORITHM: &str = "RS256";

#[derive(Debug)]
pub(super) struct JwtVerificationProfile {
    authorized_parties: HashSet<String>,
    // Collected into sets once at construction: `verification_options` clones
    // them per token (jwt-simple takes ownership), and cloning a `HashSet`
    // copies the table without re-hashing, unlike re-collecting a `Vec`.
    audiences: HashSet<String>,
    issuers: HashSet<String>,
    clock_skew: Duration,
    require_session_id: bool,
}

impl JwtVerificationProfile {
    pub(super) fn new(
        authorized_parties: Vec<String>,
        audiences: Vec<String>,
        issuers: Vec<String>,
        clock_skew: Duration,
        require_session_id: bool,
    ) -> Self {
        Self {
            authorized_parties: authorized_parties.into_iter().collect(),
            audiences: audiences.into_iter().collect(),
            issuers: issuers.into_iter().collect(),
            clock_skew,
            require_session_id,
        }
    }

    pub(super) fn key_id_from_token_header(
        &self,
        token: &str,
    ) -> Result<String, VerificationFailure> {
        let header = token_header(token).ok_or(VerificationFailure::invalid())?;
        if header.alg.as_deref() != Some(JWT_ALGORITHM) {
            return Err(VerificationFailure::invalid());
        }
        if !header
            .typ
            .as_deref()
            .is_some_and(|typ| typ.eq_ignore_ascii_case(JWT_HEADER_TYPE))
        {
            return Err(VerificationFailure::invalid());
        }

        header.kid.ok_or(VerificationFailure::invalid())
    }

    pub(super) fn rs256_key_for_kid(
        &self,
        keyset: &KeySet,
        kid: &str,
    ) -> Result<RS256PublicKey, VerificationFailure> {
        let jwk = keyset
            .selector(&[Algorithm::Rs256])
            .select(KeyMatcher::new(KeyOperation::Verify, Algorithm::Rs256).with_kid(kid))
            .map_err(|error| {
                tracing::debug!(error = ?error, "failed to select Clerk JWT verification key");
                VerificationFailure::invalid()
            })?
            .clone();

        jwk.try_into().map_err(|error| {
            tracing::debug!(error = ?error, "failed to convert Clerk JWK into RS256 verifier");
            VerificationFailure::invalid()
        })
    }

    pub(super) fn verify_claims(
        &self,
        key: &RS256PublicKey,
        token: &str,
        kid: &str,
    ) -> Result<VerifiedClerkClaims, VerificationFailure> {
        let claims = key
            .verify_token::<ClerkJwtClaims>(token, Some(self.verification_options(kid)))
            .map_err(|error| {
                tracing::debug!(error = ?error, "Clerk JWT validation failed");
                VerificationFailure::Invalid(invalid_reason(&error))
            })?;

        self.validate_authorized_party(claims.custom.authorized_party())?;
        self.validate_session_token(claims.custom.session_id())?;
        Ok(VerifiedClerkClaims::new(claims))
    }

    /// Reject instance-signed JWTs that are not session tokens when the profile
    /// requires a session id. Clerk session tokens carry `sid`; JWT-template
    /// tokens do not, so this keeps a leaked template token from being replayed
    /// as a session.
    fn validate_session_token(&self, sid: Option<&str>) -> Result<(), VerificationFailure> {
        if self.require_session_id && sid.is_none() {
            return Err(VerificationFailure::invalid());
        }
        Ok(())
    }

    fn verification_options(&self, kid: &str) -> VerificationOptions {
        // Millisecond precision so sub-second skews are not truncated to zero.
        let clock_skew_ms = u64::try_from(self.clock_skew.as_millis()).unwrap_or(u64::MAX);
        let mut options = VerificationOptions {
            time_tolerance: Some(JwtDuration::from_millis(clock_skew_ms)),
            required_signature_type: Some(JWT_HEADER_TYPE.into()),
            required_key_id: Some(kid.into()),
            ..Default::default()
        };
        if !self.audiences.is_empty() {
            options.allowed_audiences = Some(self.audiences.clone());
        }
        if !self.issuers.is_empty() {
            options.allowed_issuers = Some(self.issuers.clone());
        }
        options
    }

    fn validate_authorized_party(&self, azp: Option<&str>) -> Result<(), VerificationFailure> {
        if self.authorized_parties.is_empty() || azp.is_none() {
            return Ok(());
        }

        if azp.is_some_and(|azp| self.authorized_parties.contains(azp)) {
            Ok(())
        } else {
            Err(VerificationFailure::invalid())
        }
    }
}

fn invalid_reason(error: &jwt_simple::Error) -> InvalidTokenReason {
    match error.downcast_ref::<JWTError>() {
        Some(JWTError::TokenHasExpired) => InvalidTokenReason::Expired,
        // ClockDrift is jwt-simple's "iat is in the future beyond tolerance".
        Some(JWTError::TokenNotValidYet | JWTError::ClockDrift) => InvalidTokenReason::NotYetValid,
        _ => InvalidTokenReason::Other,
    }
}

#[derive(Debug, Deserialize)]
struct JwtHeader {
    kid: Option<String>,
    alg: Option<String>,
    typ: Option<String>,
}

fn token_header(token: &str) -> Option<JwtHeader> {
    let header = token.split('.').next()?;
    let bytes = base64_url_decode(header)?;
    serde_json::from_slice(&bytes).ok()
}

fn base64_url_decode(input: &str) -> Option<Vec<u8>> {
    let mut buffer = vec![0_u8; input.len() / 4 * 3 + 3];
    let decoded_len = Base64UrlSafeNoPadding::decode(&mut buffer, input, None)
        .ok()?
        .len();
    buffer.truncate(decoded_len);
    Some(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jwt_profile_accepts_rs256_jwt_header_with_key_id() {
        let token = token_with_header(serde_json::json!({
            "alg": "RS256",
            "typ": "JWT",
            "kid": "test-kid"
        }));

        assert_eq!(
            test_profile().key_id_from_token_header(&token).unwrap(),
            "test-kid"
        );
    }

    #[test]
    fn jwt_profile_rejects_non_rs256_header() {
        let token = token_with_header(serde_json::json!({
            "alg": "HS256",
            "typ": "JWT",
            "kid": "test-kid"
        }));

        assert!(matches!(
            test_profile().key_id_from_token_header(&token),
            Err(VerificationFailure::Invalid(_))
        ));
    }

    #[test]
    fn jwt_profile_rejects_missing_header_type() {
        let token = token_with_header(serde_json::json!({
            "alg": "RS256",
            "kid": "test-kid"
        }));

        assert!(matches!(
            test_profile().key_id_from_token_header(&token),
            Err(VerificationFailure::Invalid(_))
        ));
    }

    #[test]
    fn jwt_profile_rejects_missing_key_id() {
        let token = token_with_header(serde_json::json!({
            "alg": "RS256",
            "typ": "JWT"
        }));

        assert!(matches!(
            test_profile().key_id_from_token_header(&token),
            Err(VerificationFailure::Invalid(_))
        ));
    }

    #[test]
    fn authorized_party_validation_skips_missing_azp() {
        let profile = JwtVerificationProfile::new(
            vec!["https://example.com".into()],
            vec![],
            vec![],
            Duration::from_secs(5),
            true,
        );

        assert!(profile.validate_authorized_party(None).is_ok());
    }

    #[test]
    fn authorized_party_validation_rejects_mismatch() {
        let profile = JwtVerificationProfile::new(
            vec!["https://example.com".into()],
            vec![],
            vec![],
            Duration::from_secs(5),
            true,
        );

        assert!(matches!(
            profile.validate_authorized_party(Some("https://evil.example")),
            Err(VerificationFailure::Invalid(_))
        ));
    }

    #[test]
    fn base64_url_decode_rejects_padded_and_malformed_input() {
        assert_eq!(base64_url_decode("aGk").as_deref(), Some(b"hi".as_slice()));
        assert!(base64_url_decode("aGk=").is_none());
        assert!(base64_url_decode("aGk=garbage").is_none());
        assert!(base64_url_decode("a+b/").is_none());
    }

    #[test]
    fn session_token_validation_rejects_a_token_without_sid_when_required() {
        let profile =
            JwtVerificationProfile::new(vec![], vec![], vec![], Duration::from_secs(5), true);

        assert!(profile.validate_session_token(Some("sess_1")).is_ok());
        assert!(matches!(
            profile.validate_session_token(None),
            Err(VerificationFailure::Invalid(_))
        ));
    }

    #[test]
    fn session_token_validation_allows_a_missing_sid_when_not_required() {
        let profile =
            JwtVerificationProfile::new(vec![], vec![], vec![], Duration::from_secs(5), false);

        assert!(profile.validate_session_token(None).is_ok());
    }

    fn test_profile() -> JwtVerificationProfile {
        JwtVerificationProfile::new(vec![], vec![], vec![], Duration::from_secs(5), true)
    }

    fn token_with_header(header: serde_json::Value) -> String {
        format!(
            "{}.payload.signature",
            base64_url_encode(header.to_string().as_bytes())
        )
    }

    fn base64_url_encode(input: &[u8]) -> String {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

        let mut output = String::with_capacity(input.len().div_ceil(3) * 4);
        let mut chunks = input.chunks_exact(3);
        for chunk in &mut chunks {
            output.push(TABLE[(chunk[0] >> 2) as usize] as char);
            output
                .push(TABLE[(((chunk[0] & 0b0000_0011) << 4) | (chunk[1] >> 4)) as usize] as char);
            output
                .push(TABLE[(((chunk[1] & 0b0000_1111) << 2) | (chunk[2] >> 6)) as usize] as char);
            output.push(TABLE[(chunk[2] & 0b0011_1111) as usize] as char);
        }

        match chunks.remainder() {
            [first] => {
                output.push(TABLE[(first >> 2) as usize] as char);
                output.push(TABLE[((first & 0b0000_0011) << 4) as usize] as char);
            }
            [first, second] => {
                output.push(TABLE[(first >> 2) as usize] as char);
                output.push(TABLE[(((first & 0b0000_0011) << 4) | (second >> 4)) as usize] as char);
                output.push(TABLE[((second & 0b0000_1111) << 2) as usize] as char);
            }
            [] => {}
            _ => unreachable!(),
        }

        output
    }
}
