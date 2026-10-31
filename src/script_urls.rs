//! Where the clerk-js and `@clerk/ui` bundles are loaded from.
//!
//! Target-neutral because two halves of the crate must agree on it: the
//! browser loader injects these URLs, and [`crate::csp`] has to allow them,
//! typically from a server that never runs the loader. Deriving both from one
//! function means a version bump or a new default host cannot leave the CSP
//! sources behind.
//!
//! The rules mirror Clerk's own loader (`clerkJSScriptUrl` and
//! `buildScriptHost` in `@clerk/shared`), so a Dioxus app behind a proxy or on
//! a satellite domain fetches the bundles from the same place a React app
//! would.

/// Major clerk-js version this crate targets. The default script URL pins the
/// major so patch/minor updates flow in without a breaking jump.
const CLERK_JS_MAJOR: &str = "6";
/// Major `@clerk/ui` version this crate targets. clerk-js 6 no longer bundles
/// UI components; they ship in the separately-versioned `@clerk/ui` package
/// (`Clerk.uiVersion`), which must be loaded alongside clerk-js and handed to
/// `Clerk.load({ ui: { ClerkUI } })`.
const CLERK_UI_MAJOR: &str = "1";

/// Public CDN used only when the publishable key cannot be decoded into a
/// Frontend API host. The preferred source is the instance's own Frontend API
/// host, which keeps the script version-matched with the backend it calls.
const FALLBACK_CDN: &str = "https://cdn.jsdelivr.net";

/// Host suffixes of Clerk's development and staging Frontend APIs
/// (`DEV_OR_STAGING_SUFFIXES` in `@clerk/shared`). A `domain` never moves the
/// script host off one of these: development instances always serve from the
/// key's own host.
const DEV_OR_STAGING_SUFFIXES: [&str; 9] = [
    ".lcl.dev",
    ".stg.dev",
    ".lclstage.dev",
    ".stgstage.dev",
    ".dev.lclclerk.com",
    ".stg.lclclerk.com",
    ".accounts.lclclerk.com",
    "accountsstage.dev",
    "accounts.dev",
];

/// The `ClerkProvider` settings that move clerk-js off the key's own Frontend
/// API host: a reverse proxy (`proxy_url`) or a satellite `domain`.
///
/// clerk-js reads both only once, from its script tag, when it constructs
/// `window.Clerk`; the loader puts them there and the script URLs follow them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct FrontendApiRouting<'a> {
    pub(crate) proxy_url: Option<&'a str>,
    pub(crate) domain: Option<&'a str>,
}

impl<'a> FrontendApiRouting<'a> {
    /// An empty string means "unset" to clerk-js (it tests truthiness), so it
    /// does here too.
    pub(crate) fn new(proxy_url: Option<&'a str>, domain: Option<&'a str>) -> Self {
        Self {
            proxy_url: proxy_url.filter(|value| !value.is_empty()),
            domain: domain.filter(|value| !value.is_empty()),
        }
    }
}

/// The clerk-js script URL: `override_url` when the app self-hosts the bundle,
/// otherwise served through the proxy, the satellite domain's Frontend API, or
/// the instance's own Frontend API host (matching Clerk's own loaders),
/// falling back to a pinned public CDN when the key cannot be decoded.
pub(crate) fn clerk_js_url(
    publishable_key: &str,
    override_url: Option<&str>,
    routing: FrontendApiRouting<'_>,
) -> String {
    resolve(
        publishable_key,
        override_url,
        routing,
        &format!("npm/@clerk/clerk-js@{CLERK_JS_MAJOR}/dist/clerk.browser.js"),
    )
}

/// The `@clerk/ui` script URL, resolved like [`clerk_js_url`] so the two
/// bundles come from the same host and stay version-matched.
pub(crate) fn clerk_ui_url(
    publishable_key: &str,
    override_url: Option<&str>,
    routing: FrontendApiRouting<'_>,
) -> String {
    resolve(
        publishable_key,
        override_url,
        routing,
        &format!("npm/@clerk/ui@{CLERK_UI_MAJOR}/dist/ui.browser.js"),
    )
}

/// The Frontend API host a satellite `domain` maps to in production:
/// `clerk.{domain}`, the `addClerkPrefix(stripScheme(domain))` clerk-js
/// applies. Accepts a bare host or a URL; a leading `clerk.` is not doubled.
pub(crate) fn domain_frontend_api_host(domain: &str) -> String {
    let domain = domain.trim();
    let host = domain.split_once("://").map_or(domain, |(_, rest)| rest);
    let host = host.trim_end_matches('/');
    // A Clerk-owned development host is already a Frontend API host.
    if host.contains(".clerk.accounts") {
        return host.to_owned();
    }
    let mut bare = host;
    while bare.len() >= 6 && bare[..6].eq_ignore_ascii_case("clerk.") {
        bare = &bare[6..];
    }
    format!("clerk.{bare}")
}

fn resolve(
    publishable_key: &str,
    override_url: Option<&str>,
    routing: FrontendApiRouting<'_>,
    package_path: &str,
) -> String {
    if let Some(url) = override_url {
        return url.to_owned();
    }
    // A relative proxy stays relative, so the bundles load same-origin
    // through it.
    if let Some(proxy) = routing.proxy_url.filter(|proxy| proxy.starts_with('/')) {
        return format!("{}/{package_path}", proxy.trim_end_matches('/'));
    }
    match script_host(publishable_key, routing) {
        Some(host) => format!("https://{host}/{package_path}"),
        None => format!("{FALLBACK_CDN}/{package_path}"),
    }
}

/// `buildScriptHost`: an absolute proxy (its host and path, always over
/// https), then `clerk.{domain}` unless the key is for a development or
/// staging Frontend API, then the key's own host.
///
/// A proxy that is neither absolute `http(s)` nor relative is ignored here, as
/// in Clerk's loader; clerk-js itself rejects it with a clear error once it
/// starts.
fn script_host(publishable_key: &str, routing: FrontendApiRouting<'_>) -> Option<String> {
    if let Some(rest) = routing.proxy_url.and_then(|proxy| {
        proxy
            .strip_prefix("https://")
            .or_else(|| proxy.strip_prefix("http://"))
    }) {
        return Some(rest.trim_end_matches('/').to_owned());
    }
    let key_host = crate::publishable_key::frontend_api_host(publishable_key);
    if let Some(domain) = routing.domain {
        let development = key_host.as_deref().is_some_and(|host| {
            DEV_OR_STAGING_SUFFIXES
                .iter()
                .any(|suffix| host.ends_with(suffix))
        });
        if !development {
            return Some(domain_frontend_api_host(domain));
        }
    }
    key_host
}

#[cfg(test)]
mod tests {
    use super::*;

    // `pk_test_` + base64("foo-bar-13.clerk.accounts.dev$"), as in Clerk's
    // own loader tests.
    const DEV_KEY: &str = "pk_test_Zm9vLWJhci0xMy5jbGVyay5hY2NvdW50cy5kZXYk";
    // `pk_live_` + base64("clerk.example.com$").
    const LIVE_KEY: &str = "pk_live_Y2xlcmsuZXhhbXBsZS5jb20k";

    const JS_PATH: &str = "npm/@clerk/clerk-js@6/dist/clerk.browser.js";
    const UI_PATH: &str = "npm/@clerk/ui@1/dist/ui.browser.js";

    fn proxy(url: &str) -> FrontendApiRouting<'_> {
        FrontendApiRouting::new(Some(url), None)
    }

    fn domain(domain: &str) -> FrontendApiRouting<'_> {
        FrontendApiRouting::new(None, Some(domain))
    }

    fn js(key: &str, routing: FrontendApiRouting<'_>) -> String {
        clerk_js_url(key, None, routing)
    }

    #[test]
    fn serves_bundles_from_the_frontend_api_host() {
        let routing = FrontendApiRouting::default();
        assert_eq!(
            clerk_js_url(DEV_KEY, None, routing),
            format!("https://foo-bar-13.clerk.accounts.dev/{JS_PATH}")
        );
        assert_eq!(
            clerk_ui_url(DEV_KEY, None, routing),
            format!("https://foo-bar-13.clerk.accounts.dev/{UI_PATH}")
        );
    }

    #[test]
    fn falls_back_to_the_public_cdn_when_the_key_does_not_decode() {
        let routing = FrontendApiRouting::default();
        assert_eq!(
            clerk_js_url("pk_test_not*base64", None, routing),
            format!("https://cdn.jsdelivr.net/{JS_PATH}")
        );
        assert_eq!(
            clerk_ui_url("pk_test_not*base64", None, routing),
            format!("https://cdn.jsdelivr.net/{UI_PATH}")
        );
    }

    #[test]
    fn an_override_wins_over_everything() {
        let url = "https://cdn.example.com/clerk.browser.js";
        let routing = FrontendApiRouting::new(Some("/__clerk"), Some("satellite.dev"));
        assert_eq!(clerk_js_url(LIVE_KEY, Some(url), routing), url);
        assert_eq!(clerk_ui_url(LIVE_KEY, Some(url), routing), url);
    }

    #[test]
    fn a_relative_proxy_serves_bundles_same_origin() {
        assert_eq!(
            js(DEV_KEY, proxy("/__clerk")),
            format!("/__clerk/{JS_PATH}")
        );
        assert_eq!(
            js(DEV_KEY, proxy("/__clerk/")),
            format!("/__clerk/{JS_PATH}")
        );
        assert_eq!(
            clerk_ui_url(DEV_KEY, None, proxy("/__clerk")),
            format!("/__clerk/{UI_PATH}")
        );
    }

    #[test]
    fn an_absolute_proxy_serves_bundles_over_https_keeping_its_path() {
        assert_eq!(
            js(DEV_KEY, proxy("https://proxy.clerk.com")),
            format!("https://proxy.clerk.com/{JS_PATH}")
        );
        assert_eq!(
            js(LIVE_KEY, proxy("https://example.com/__clerk/")),
            format!("https://example.com/__clerk/{JS_PATH}")
        );
        assert_eq!(
            js(LIVE_KEY, proxy("http://example.com/__clerk")),
            format!("https://example.com/__clerk/{JS_PATH}")
        );
    }

    #[test]
    fn an_invalid_proxy_is_ignored() {
        assert_eq!(
            js(LIVE_KEY, proxy("proxy.example.com")),
            format!("https://clerk.example.com/{JS_PATH}")
        );
    }

    #[test]
    fn a_production_domain_serves_bundles_from_its_frontend_api() {
        for value in [
            "custom.com",
            "https://custom.com",
            "clerk.custom.com",
            "custom.com/",
        ] {
            assert_eq!(
                js(LIVE_KEY, domain(value)),
                format!("https://clerk.custom.com/{JS_PATH}"),
                "{value}"
            );
        }
    }

    #[test]
    fn a_development_key_ignores_the_domain() {
        assert_eq!(
            js(DEV_KEY, domain("custom.com")),
            format!("https://foo-bar-13.clerk.accounts.dev/{JS_PATH}")
        );
    }

    #[test]
    fn a_proxy_wins_over_a_domain() {
        let routing = FrontendApiRouting::new(Some("https://proxy.clerk.com"), Some("custom.com"));
        assert_eq!(
            js(LIVE_KEY, routing),
            format!("https://proxy.clerk.com/{JS_PATH}")
        );
    }

    #[test]
    fn empty_settings_count_as_unset() {
        let routing = FrontendApiRouting::new(Some(""), Some(""));
        assert_eq!(
            js(LIVE_KEY, routing),
            format!("https://clerk.example.com/{JS_PATH}")
        );
    }
}
