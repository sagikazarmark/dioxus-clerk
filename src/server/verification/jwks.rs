use std::sync::Arc;
use std::time::Duration;

use jwk_simple::KeySet;
use web_time::SystemTime;

use crate::server::config::{
    DEFAULT_JWKS_CACHE_TTL, DEFAULT_JWKS_FAILURE_BACKOFF, DEFAULT_JWKS_MAX_STALE,
    DEFAULT_UNKNOWN_KID_REFRESH_INTERVAL,
};

/// Tuning knobs for cache lookup decisions, bundled so a future knob extends
/// this struct instead of every `lookup` call site.
#[derive(Clone, Copy)]
pub(super) struct JwksCachePolicy {
    pub(super) ttl: Duration,
    pub(super) unknown_kid_refresh_interval: Duration,
    pub(super) failure_backoff: Duration,
    /// Stale-while-error window: how long past `ttl` a stored keyset may
    /// still be served when refreshing fails.
    pub(super) max_stale: Duration,
}

impl Default for JwksCachePolicy {
    fn default() -> Self {
        Self {
            ttl: DEFAULT_JWKS_CACHE_TTL,
            unknown_kid_refresh_interval: DEFAULT_UNKNOWN_KID_REFRESH_INTERVAL,
            failure_backoff: DEFAULT_JWKS_FAILURE_BACKOFF,
            max_stale: DEFAULT_JWKS_MAX_STALE,
        }
    }
}

#[derive(Default)]
pub(super) struct JwksCache {
    // Stored behind an `Arc` so a cache hit on the per-request auth path clones
    // a pointer rather than the whole keyset (RSA key material).
    keyset: Option<Arc<KeySet>>,
    last_updated: Option<SystemTime>,
    /// When the last upstream fetch failed, if it failed more recently than it
    /// succeeded. Drives the failure backoff so an outage does not turn every
    /// queued request into its own slow upstream fetch.
    last_failure: Option<SystemTime>,
}

pub(super) enum JwksCacheLookup {
    Hit(Arc<KeySet>),
    Refresh,
    RejectUnknownKid,
    /// A refresh is needed but a recent fetch failure is inside the backoff
    /// window; fail unavailable without hitting the upstream again.
    Unavailable,
}

impl JwksCache {
    pub(super) fn lookup(
        &self,
        kid: &str,
        policy: JwksCachePolicy,
        now: SystemTime,
    ) -> JwksCacheLookup {
        if self.is_uninitialized() || self.is_expired(policy.ttl, now) {
            if self.in_failure_backoff(policy.failure_backoff, now) {
                // Stale-while-error: upstream is failing, so a known kid is
                // served from the stored keyset instead of failing closed.
                if let Some(keyset) = self.stale_keyset_for_kid(kid, policy, now) {
                    return JwksCacheLookup::Hit(keyset);
                }
                return JwksCacheLookup::Unavailable;
            }
            return JwksCacheLookup::Refresh;
        }

        if let Some(keyset) = self.keyset_for_kid(kid) {
            return JwksCacheLookup::Hit(keyset);
        }

        if self.can_refresh_for_unknown_kid(policy.unknown_kid_refresh_interval, now)
            && !self.in_failure_backoff(policy.failure_backoff, now)
        {
            return JwksCacheLookup::Refresh;
        }

        JwksCacheLookup::RejectUnknownKid
    }

    fn is_uninitialized(&self) -> bool {
        self.last_updated.is_none()
    }

    fn is_expired(&self, ttl: Duration, now: SystemTime) -> bool {
        self.age_at(now).is_none_or(|elapsed| elapsed >= ttl)
    }

    fn can_refresh_for_unknown_kid(&self, min_age: Duration, now: SystemTime) -> bool {
        self.age_at(now).is_none_or(|elapsed| elapsed >= min_age)
    }

    fn in_failure_backoff(&self, backoff: Duration, now: SystemTime) -> bool {
        // A clock that went backwards fails toward refresh, matching age_at.
        self.last_failure
            .and_then(|last_failure| now.duration_since(last_failure).ok())
            .is_some_and(|elapsed| elapsed < backoff)
    }

    fn age_at(&self, now: SystemTime) -> Option<Duration> {
        self.last_updated
            .and_then(|last_updated| now.duration_since(last_updated).ok())
    }

    fn keyset_for_kid(&self, kid: &str) -> Option<Arc<KeySet>> {
        self.keyset
            .as_ref()
            .filter(|keyset| keyset.get_by_kid(kid).is_some())
            .map(Arc::clone)
    }

    /// The stored keyset for a known kid while inside the stale-while-error
    /// window (`ttl + max_stale` since the last successful fetch). A clock
    /// that went backwards yields `None`, failing toward unavailable.
    pub(super) fn stale_keyset_for_kid(
        &self,
        kid: &str,
        policy: JwksCachePolicy,
        now: SystemTime,
    ) -> Option<Arc<KeySet>> {
        let within_stale_window = self
            .age_at(now)
            .is_some_and(|elapsed| elapsed < policy.ttl + policy.max_stale);
        if !within_stale_window {
            return None;
        }
        self.keyset_for_kid(kid)
    }

    pub(super) fn store(&mut self, keyset: Arc<KeySet>) {
        self.store_at(keyset, SystemTime::now());
    }

    fn store_at(&mut self, keyset: Arc<KeySet>, now: SystemTime) {
        self.keyset = Some(keyset);
        self.last_updated = Some(now);
        self.last_failure = None;
    }

    pub(super) fn record_failure(&mut self) {
        self.record_failure_at(SystemTime::now());
    }

    fn record_failure_at(&mut self, now: SystemTime) {
        self.last_failure = Some(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jwks_cache_refreshes_when_uninitialized() {
        let cache = JwksCache::default();

        assert!(matches!(
            cache.lookup("test-kid", test_policy(), sample_time()),
            JwksCacheLookup::Refresh
        ));
    }

    #[test]
    fn jwks_cache_uses_known_kid_before_ttl() {
        let now = sample_time();
        let mut cache = JwksCache::default();
        cache.store_at(test_keyset(), now - Duration::from_secs(10));

        match cache.lookup("test-kid", test_policy(), now) {
            JwksCacheLookup::Hit(keyset) => assert!(keyset.get_by_kid("test-kid").is_some()),
            _ => panic!("known kid should use cached keyset before TTL"),
        }
    }

    #[test]
    fn jwks_cache_refreshes_known_kid_after_ttl() {
        let now = sample_time();
        let mut cache = JwksCache::default();
        cache.store_at(test_keyset(), now - Duration::from_secs(3_601));

        assert!(matches!(
            cache.lookup("test-kid", test_policy(), now),
            JwksCacheLookup::Refresh
        ));
    }

    #[test]
    fn jwks_cache_rejects_unknown_kid_before_refresh_interval() {
        let now = sample_time();
        let mut cache = JwksCache::default();
        cache.store_at(test_keyset(), now - Duration::from_secs(30));

        assert!(matches!(
            cache.lookup("unknown-kid", test_policy(), now),
            JwksCacheLookup::RejectUnknownKid
        ));
    }

    #[test]
    fn jwks_cache_refreshes_unknown_kid_after_refresh_interval() {
        let now = sample_time();
        let mut cache = JwksCache::default();
        cache.store_at(test_keyset(), now - Duration::from_secs(301));

        assert!(matches!(
            cache.lookup("unknown-kid", test_policy(), now),
            JwksCacheLookup::Refresh
        ));
    }

    #[test]
    fn jwks_cache_is_unavailable_during_failure_backoff_when_refresh_is_needed() {
        let now = sample_time();
        let mut cache = JwksCache::default();
        cache.record_failure_at(now - Duration::from_secs(3));

        assert!(matches!(
            cache.lookup("test-kid", test_policy(), now),
            JwksCacheLookup::Unavailable
        ));
    }

    #[test]
    fn jwks_cache_refreshes_again_after_failure_backoff_elapses() {
        let now = sample_time();
        let mut cache = JwksCache::default();
        cache.record_failure_at(now - Duration::from_secs(11));

        assert!(matches!(
            cache.lookup("test-kid", test_policy(), now),
            JwksCacheLookup::Refresh
        ));
    }

    #[test]
    fn jwks_cache_rejects_unknown_kid_instead_of_refetching_during_failure_backoff() {
        let now = sample_time();
        let mut cache = JwksCache::default();
        cache.store_at(test_keyset(), now - Duration::from_secs(301));
        cache.record_failure_at(now - Duration::from_secs(3));

        assert!(matches!(
            cache.lookup("unknown-kid", test_policy(), now),
            JwksCacheLookup::RejectUnknownKid
        ));
    }

    #[test]
    fn jwks_cache_store_clears_failure_backoff() {
        let now = sample_time();
        let mut cache = JwksCache::default();
        cache.record_failure_at(now - Duration::from_secs(3));
        cache.store_at(test_keyset(), now);

        match cache.lookup("test-kid", test_policy(), now) {
            JwksCacheLookup::Hit(_) => {}
            _ => panic!("successful store should clear failure backoff"),
        }
    }

    #[test]
    fn jwks_cache_serves_cached_known_kid_during_failure_backoff() {
        let now = sample_time();
        let mut cache = JwksCache::default();
        cache.store_at(test_keyset(), now - Duration::from_secs(10));
        cache.record_failure_at(now - Duration::from_secs(3));

        assert!(matches!(
            cache.lookup("test-kid", test_policy(), now),
            JwksCacheLookup::Hit(_)
        ));
    }

    #[test]
    fn jwks_cache_serves_stale_known_kid_during_failure_backoff_after_ttl() {
        let now = sample_time();
        let mut cache = JwksCache::default();
        cache.store_at(test_keyset(), now - Duration::from_secs(3_601));
        cache.record_failure_at(now - Duration::from_secs(3));

        assert!(matches!(
            cache.lookup("test-kid", test_policy(), now),
            JwksCacheLookup::Hit(_)
        ));
    }

    #[test]
    fn jwks_cache_is_unavailable_for_unknown_kid_during_failure_backoff_after_ttl() {
        let now = sample_time();
        let mut cache = JwksCache::default();
        cache.store_at(test_keyset(), now - Duration::from_secs(3_601));
        cache.record_failure_at(now - Duration::from_secs(3));

        assert!(matches!(
            cache.lookup("unknown-kid", test_policy(), now),
            JwksCacheLookup::Unavailable
        ));
    }

    #[test]
    fn jwks_cache_stops_serving_stale_keyset_past_max_stale_window() {
        let now = sample_time() + Duration::from_secs(60 * 60 * 48);
        let mut cache = JwksCache::default();
        cache.store_at(test_keyset(), now - Duration::from_secs(60 * 60 * 25 + 1));
        cache.record_failure_at(now - Duration::from_secs(3));

        assert!(matches!(
            cache.lookup("test-kid", test_policy(), now),
            JwksCacheLookup::Unavailable
        ));
        assert!(
            cache
                .stale_keyset_for_kid("test-kid", test_policy(), now)
                .is_none()
        );
    }

    #[test]
    fn stale_keyset_for_kid_serves_known_kid_within_stale_window() {
        let now = sample_time();
        let mut cache = JwksCache::default();
        cache.store_at(test_keyset(), now - Duration::from_secs(3_601));

        assert!(
            cache
                .stale_keyset_for_kid("test-kid", test_policy(), now)
                .is_some()
        );
        assert!(
            cache
                .stale_keyset_for_kid("unknown-kid", test_policy(), now)
                .is_none()
        );
    }

    fn test_keyset() -> Arc<KeySet> {
        Arc::new(
            serde_json::from_str(include_str!("../../../tests/fixtures/test_jwks.json")).unwrap(),
        )
    }

    fn sample_time() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(10_000)
    }

    fn test_policy() -> JwksCachePolicy {
        JwksCachePolicy {
            ttl: Duration::from_secs(60 * 60),
            unknown_kid_refresh_interval: Duration::from_secs(60 * 5),
            failure_backoff: Duration::from_secs(10),
            max_stale: Duration::from_secs(60 * 60 * 24),
        }
    }
}
