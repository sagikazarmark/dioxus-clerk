use crate::core::VerificationOutcome;
use axum::body::Body;
use axum::http::{Request, header};
use axum_extra::extract::cookie::Cookie;

use super::{Verifier, verify_token};

const SESSION_COOKIE_NAME: &str = "__session";

/// Upper bound on session-cookie candidates verified per request.
///
/// A request legitimately carries the unsuffixed `__session` plus at most a
/// handful of suffixed multi-session variants. Capping the fan-out stops a
/// caller from packing its own `Cookie` header with many bad-signature tokens
/// to force one RSA verification each (CPU amplification): the one unbounded
/// work multiplier the rest of the verification path deliberately avoids. The
/// most-specific, likely-valid cookies are ordered first (see
/// [`session_cookies`]), so the cap costs a legitimate request nothing.
const MAX_SESSION_COOKIE_CANDIDATES: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RequestCredentials {
    Missing,
    Bearer(String),
    /// The `__session` cookie plus any suffixed `__session_*` variants Clerk
    /// sets when multiple Clerk apps share a registrable domain.
    SessionCookies(Vec<String>),
}

impl RequestCredentials {
    pub(super) fn from_request(req: &Request<Body>) -> Self {
        if let Some(token) = bearer_token(req) {
            return Self::Bearer(token.to_string());
        }

        let cookies = session_cookies(req);
        if cookies.is_empty() {
            Self::Missing
        } else {
            Self::SessionCookies(cookies)
        }
    }

    pub(super) async fn verify(self, verifier: Verifier) -> VerificationOutcome {
        match self {
            Self::Missing => VerificationOutcome::Missing,
            Self::Bearer(token) => verify_token(&token, verifier).await,
            Self::SessionCookies(tokens) => {
                // Any valid cookie wins. Otherwise `Unavailable` outranks
                // `Invalid` regardless of cookie order: a candidate that could
                // not be checked must fail closed, not demote the
                // request to anonymous.
                let mut failure = None;
                for token in &tokens {
                    let outcome = verify_token(token, verifier.clone()).await;
                    match outcome {
                        VerificationOutcome::Valid(_) => return outcome,
                        VerificationOutcome::Unavailable => {
                            failure = Some(VerificationOutcome::Unavailable);
                        }
                        _ => {
                            failure.get_or_insert(outcome);
                        }
                    }
                }
                // `SessionCookies` is only constructed with a non-empty list
                // (empty values are filtered in `session_cookies`), so the loop
                // above always ran at least once and set `failure`. The
                // `Missing` fallback is therefore unreachable in practice; it
                // keeps the expression total without a panic.
                failure.unwrap_or(VerificationOutcome::Missing)
            }
        }
    }
}

fn bearer_token(req: &Request<Body>) -> Option<&str> {
    let header = req
        .headers()
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .trim_start();
    let (scheme, token) = header.split_once(' ')?;
    // RFC 7235 auth scheme names are case-insensitive.
    if !scheme.eq_ignore_ascii_case("Bearer") {
        return None;
    }
    let token = token.trim();
    (!token.is_empty()).then_some(token)
}

/// Session cookie values to try, unsuffixed `__session` cookies first.
///
/// Parsed straight from the `Cookie` headers, not through a `CookieJar`: a
/// jar deduplicates by name keeping the *last* occurrence, while a request
/// can legitimately carry several `__session` cookies (e.g. a host-only and
/// a domain-wide cookie) and RFC 6265 §5.4 orders the more specific (usually
/// correct) one *first*. Every occurrence must stay a candidate so a stale
/// duplicate cannot shadow the valid session (any-valid-wins in `verify`).
fn session_cookies(req: &Request<Body>) -> Vec<String> {
    let cookies: Vec<Cookie<'_>> = req
        .headers()
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(Cookie::split_parse)
        .filter_map(Result::ok)
        .collect();

    let unsuffixed = cookies.iter().filter(|c| c.name() == SESSION_COOKIE_NAME);
    let mut suffixed: Vec<&Cookie<'_>> = cookies
        .iter()
        .filter(|c| {
            c.name()
                .strip_prefix(SESSION_COOKIE_NAME)
                .is_some_and(|rest| rest.starts_with('_'))
        })
        .collect();
    suffixed.sort_by(|a, b| a.name().cmp(b.name()));

    unsuffixed
        .chain(suffixed)
        .map(|c| c.value().to_string())
        // An empty `__session=` cookie carries no token; keeping it would make
        // an anonymous request look like one bearing invalid credentials (401)
        // instead of `Missing`.
        .filter(|value| !value.is_empty())
        // Bound the per-request verification fan-out. Filtering before the cap
        // means empty cookies never consume the budget.
        .take(MAX_SESSION_COOKIE_CANDIDATES)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::config::ClerkAuthLayerConfig;

    #[test]
    fn bearer_token_is_extracted_from_authorization_header() {
        let req = Request::builder()
            .uri("/")
            .header(header::AUTHORIZATION, "Bearer bearer-token")
            .body(Body::empty())
            .unwrap();

        assert_eq!(
            RequestCredentials::from_request(&req),
            RequestCredentials::Bearer("bearer-token".into())
        );
    }

    #[test]
    fn session_cookie_is_extracted_when_bearer_token_is_absent() {
        let req = Request::builder()
            .uri("/")
            .header(header::COOKIE, "__session=session-token")
            .body(Body::empty())
            .unwrap();

        assert_eq!(
            RequestCredentials::from_request(&req),
            RequestCredentials::SessionCookies(vec!["session-token".into()])
        );
    }

    #[test]
    fn bearer_scheme_is_case_insensitive() {
        let req = Request::builder()
            .uri("/")
            .header(header::AUTHORIZATION, "bearer bearer-token")
            .body(Body::empty())
            .unwrap();

        assert_eq!(
            RequestCredentials::from_request(&req),
            RequestCredentials::Bearer("bearer-token".into())
        );
    }

    #[test]
    fn suffixed_session_cookies_are_extracted_after_the_unsuffixed_cookie() {
        let req = Request::builder()
            .uri("/")
            .header(
                header::COOKIE,
                "__session_def=token-b; other=x; __session=token-main; __session_abc=token-a",
            )
            .body(Body::empty())
            .unwrap();

        assert_eq!(
            RequestCredentials::from_request(&req),
            RequestCredentials::SessionCookies(vec![
                "token-main".into(),
                "token-a".into(),
                "token-b".into(),
            ])
        );
    }

    #[test]
    fn duplicate_session_cookies_all_stay_candidates_in_header_order() {
        // RFC 6265 §5.4: when a host-only and a domain-wide `__session`
        // coexist, the more specific one is sent first; a stale duplicate
        // must not shadow it.
        let req = Request::builder()
            .uri("/")
            .header(
                header::COOKIE,
                "__session=host-token; __session=domain-token",
            )
            .body(Body::empty())
            .unwrap();

        assert_eq!(
            RequestCredentials::from_request(&req),
            RequestCredentials::SessionCookies(vec!["host-token".into(), "domain-token".into()])
        );
    }

    #[test]
    fn session_cookies_split_across_multiple_cookie_headers_all_stay_candidates() {
        let req = Request::builder()
            .uri("/")
            .header(header::COOKIE, "__session=token-one")
            .header(header::COOKIE, "other=x; __session=token-two")
            .body(Body::empty())
            .unwrap();

        assert_eq!(
            RequestCredentials::from_request(&req),
            RequestCredentials::SessionCookies(vec!["token-one".into(), "token-two".into()])
        );
    }

    #[test]
    fn session_cookie_candidates_are_capped_to_bound_verification_fan_out() {
        // A caller controls its own `Cookie` header, so an attacker could pack
        // many bad-signature `__session_*` tokens to force one RSA verify each.
        // Only the first `MAX_SESSION_COOKIE_CANDIDATES` (most-specific first)
        // are kept as candidates.
        let mut header = String::from("__session=main");
        for i in 0..50 {
            // Zero-padded so the suffix ordering is deterministic.
            header.push_str(&format!("; __session_{i:02}=token-{i:02}"));
        }
        let req = Request::builder()
            .uri("/")
            .header(header::COOKIE, header)
            .body(Body::empty())
            .unwrap();

        let RequestCredentials::SessionCookies(candidates) = RequestCredentials::from_request(&req)
        else {
            panic!("session cookies should be extracted");
        };

        assert_eq!(candidates.len(), MAX_SESSION_COOKIE_CANDIDATES);
        // The unsuffixed cookie wins the first slot, then the lowest-sorted
        // suffixed variants fill the rest.
        assert_eq!(candidates[0], "main");
        assert_eq!(candidates[1], "token-00");
        assert_eq!(
            candidates[MAX_SESSION_COOKIE_CANDIDATES - 1],
            format!("token-{:02}", MAX_SESSION_COOKIE_CANDIDATES - 2)
        );
    }

    #[test]
    fn empty_session_cookie_value_is_treated_as_missing_not_invalid() {
        let req = Request::builder()
            .uri("/")
            .header(header::COOKIE, "__session=")
            .body(Body::empty())
            .unwrap();

        assert_eq!(
            RequestCredentials::from_request(&req),
            RequestCredentials::Missing
        );
    }

    #[test]
    fn empty_session_cookie_does_not_shadow_a_valid_duplicate() {
        let req = Request::builder()
            .uri("/")
            .header(header::COOKIE, "__session=; __session=real-token")
            .body(Body::empty())
            .unwrap();

        assert_eq!(
            RequestCredentials::from_request(&req),
            RequestCredentials::SessionCookies(vec!["real-token".into()])
        );
    }

    #[test]
    fn bearer_token_takes_precedence_over_session_cookie() {
        let req = Request::builder()
            .uri("/")
            .header(header::AUTHORIZATION, "Bearer bearer-token")
            .header(header::COOKIE, "__session=session-token")
            .body(Body::empty())
            .unwrap();

        assert_eq!(
            RequestCredentials::from_request(&req),
            RequestCredentials::Bearer("bearer-token".into())
        );
    }

    #[test]
    fn malformed_authorization_header_falls_back_to_session_cookie() {
        let req = Request::builder()
            .uri("/")
            .header(header::AUTHORIZATION, "Basic not-a-bearer-token")
            .header(header::COOKIE, "__session=session-token")
            .body(Body::empty())
            .unwrap();

        assert_eq!(
            RequestCredentials::from_request(&req),
            RequestCredentials::SessionCookies(vec!["session-token".into()])
        );
    }

    #[tokio::test]
    async fn missing_credentials_map_to_missing_outcome() {
        let verifier = Verifier::new(ClerkAuthLayerConfig::new("sk_test_unused")).unwrap();

        let outcome = RequestCredentials::Missing.verify(verifier).await;

        assert!(matches!(outcome, VerificationOutcome::Missing));
    }

    #[tokio::test]
    async fn malformed_credentials_map_to_invalid_outcome() {
        let verifier = Verifier::new(ClerkAuthLayerConfig::new("sk_test_unused")).unwrap();

        let outcome = RequestCredentials::Bearer("invalid-token".into())
            .verify(verifier)
            .await;

        assert!(matches!(outcome, VerificationOutcome::Invalid(_)));
    }
}
