//! Where the clerk-js and `@clerk/ui` bundles are loaded from.
//!
//! Target-neutral because two halves of the crate must agree on it: the
//! browser loader injects these URLs, and [`crate::csp`] has to allow them,
//! typically from a server that never runs the loader. Deriving both from one
//! function means a version bump or a new default host cannot leave the CSP
//! sources behind.

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

/// The clerk-js script URL: `override_url` when the app self-hosts the bundle,
/// otherwise served from the instance's Frontend API host (matching Clerk's own
/// loaders), falling back to a pinned public CDN when the key cannot be
/// decoded.
pub(crate) fn clerk_js_url(publishable_key: &str, override_url: Option<&str>) -> String {
    resolve(
        publishable_key,
        override_url,
        &format!("npm/@clerk/clerk-js@{CLERK_JS_MAJOR}/dist/clerk.browser.js"),
    )
}

/// The `@clerk/ui` script URL, resolved like [`clerk_js_url`] so the two
/// bundles come from the same Frontend API host and stay version-matched.
pub(crate) fn clerk_ui_url(publishable_key: &str, override_url: Option<&str>) -> String {
    resolve(
        publishable_key,
        override_url,
        &format!("npm/@clerk/ui@{CLERK_UI_MAJOR}/dist/ui.browser.js"),
    )
}

fn resolve(publishable_key: &str, override_url: Option<&str>, package_path: &str) -> String {
    if let Some(url) = override_url {
        return url.to_owned();
    }
    match crate::publishable_key::frontend_api_host(publishable_key) {
        Some(host) => format!("https://{host}/{package_path}"),
        None => format!("{FALLBACK_CDN}/{package_path}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `pk_test_` + base64("foo.clerk.accounts.dev$").
    const KEY: &str = "pk_test_Zm9vLmNsZXJrLmFjY291bnRzLmRldiQ";

    #[test]
    fn serves_bundles_from_the_frontend_api_host() {
        assert_eq!(
            clerk_js_url(KEY, None),
            "https://foo.clerk.accounts.dev/npm/@clerk/clerk-js@6/dist/clerk.browser.js"
        );
        assert_eq!(
            clerk_ui_url(KEY, None),
            "https://foo.clerk.accounts.dev/npm/@clerk/ui@1/dist/ui.browser.js"
        );
    }

    #[test]
    fn falls_back_to_the_public_cdn_when_the_key_does_not_decode() {
        assert_eq!(
            clerk_js_url("pk_test_not*base64", None),
            "https://cdn.jsdelivr.net/npm/@clerk/clerk-js@6/dist/clerk.browser.js"
        );
        assert_eq!(
            clerk_ui_url("pk_test_not*base64", None),
            "https://cdn.jsdelivr.net/npm/@clerk/ui@1/dist/ui.browser.js"
        );
    }

    #[test]
    fn an_override_wins() {
        let url = "https://cdn.example.com/clerk.browser.js";
        assert_eq!(clerk_js_url(KEY, Some(url)), url);
        assert_eq!(clerk_ui_url(KEY, Some(url)), url);
    }
}
