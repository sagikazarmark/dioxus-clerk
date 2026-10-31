//! Content-Security-Policy sources Clerk needs, derived from a publishable key.
//!
//! An app that sends a real CSP has to allow what this crate's loader injects
//! and what clerk-js then does: the instance's Frontend API host (encoded in
//! the publishable key), the bundle URLs this crate picks, and the bot- and
//! fraud-protection hosts from [Clerk's CSP guide]. [`ClerkCsp`] computes those
//! as per-directive source lists for the app to merge into its own policy, so a
//! change to Clerk's list arrives with `cargo update` rather than through a
//! broken sign-in.
//!
//! It is deliberately not a header builder: the policy is the app's. It never
//! contributes `default-src`, `frame-ancestors`, or `form-action`, and adds
//! `'self'` only where Clerk itself requires it (`worker-src`) or where the
//! crate loads something same-origin (a relative override URL). The app adds
//! its own needs, such as `'self'`, `'wasm-unsafe-eval'` to instantiate the
//! wasm module, and any hashes or nonces.
//!
//! The module is pure and has no feature gate, so a native server, a
//! Cloudflare Worker, and the browser build can all use it.
//!
//! # Example
//!
//! Merge Clerk's sources into the app's own policy and render the header:
//!
//! ```
//! use std::collections::{BTreeMap, BTreeSet};
//! use dioxus_clerk::csp::ClerkCsp;
//!
//! # let publishable_key = "pk_live_Y2xlcmsuZXhhbXBsZS5jb20k";
//! // `None` when the key is not a publishable key; Clerk cannot load then
//! // either, so treat it as a configuration error.
//! let clerk = ClerkCsp::from_publishable_key(publishable_key)
//!     .expect("CLERK_PUBLISHABLE_KEY is a Clerk publishable key");
//!
//! // The app's own policy.
//! let mut policy: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
//! for directive in ["default-src", "script-src", "connect-src", "img-src", "style-src"] {
//!     policy.entry(directive).or_default().insert("'self'".into());
//! }
//! policy.entry("script-src").or_default().insert("'wasm-unsafe-eval'".into());
//!
//! // Clerk's share. A directive may appear only once in a header (browsers
//! // ignore repeats), so merge per directive rather than appending.
//! for (directive, sources) in clerk.directives() {
//!     policy.entry(directive).or_default().extend(sources.iter().cloned());
//! }
//!
//! let header = policy
//!     .iter()
//!     .map(|(directive, sources)| {
//!         let sources: Vec<&str> = sources.iter().map(String::as_str).collect();
//!         format!("{directive} {}", sources.join(" "))
//!     })
//!     .collect::<Vec<_>>()
//!     .join("; ");
//!
//! assert!(header.contains("https://clerk.example.com"));
//! ```
//!
//! When `ClerkProvider` is configured with `clerk_js_url`, `clerk_ui_url`,
//! `proxy_url`, or a satellite `domain`, mirror those settings on [`ClerkCsp`]
//! so its sources follow:
//!
//! ```
//! # use dioxus_clerk::csp::ClerkCsp;
//! # let publishable_key = "pk_live_Y2xlcmsuZXhhbXBsZS5jb20k";
//! let clerk = ClerkCsp::from_publishable_key(publishable_key)
//!     .expect("a Clerk publishable key")
//!     .with_clerk_js_url("https://cdn.example.com/clerk.browser.js")
//!     .with_clerk_ui_url("https://cdn.example.com/ui.browser.js")
//!     .with_proxy_url("/__clerk");
//!
//! assert!(clerk.script_src().contains(&"https://cdn.example.com".to_string()));
//! ```
//!
//! `style-src 'unsafe-inline'` is unavoidable today: Clerk's components style
//! themselves with runtime CSS-in-JS.
//!
//! [Clerk's CSP guide]: https://clerk.com/docs/guides/secure/best-practices/csp-headers

use crate::script_urls;

const SELF: &str = "'self'";
/// Cloudflare Turnstile, Clerk's bot protection; it loads a script and renders
/// in an iframe.
const CLOUDFLARE_CHALLENGES: &str = "https://challenges.cloudflare.com";
/// Clerk's abuse and fraud protection hosts.
const CLERK_PROTECT: &str = "https://*.protect.clerk.com";
/// The fraud-protection hosts as a `connect-src` source. The `:*` is required:
/// they are served on ports other than 443, and a source without a port only
/// matches the scheme's default port. Clerk's own Next.js middleware shipped
/// without it in `>=7.5.21 <7.7.8` and blocked those requests.
const CLERK_PROTECT_ANY_PORT: &str = "https://*.protect.clerk.com:*";
/// User and organization avatars.
const CLERK_IMAGES: &str = "https://img.clerk.com";
/// Development-instance telemetry collector (see [`ClerkCsp::with_telemetry`]).
const CLERK_TELEMETRY: [&str; 2] = [
    "https://clerk-telemetry.com",
    "https://*.clerk-telemetry.com",
];

/// The Content-Security-Policy sources Clerk needs, per directive.
///
/// Built from a publishable key, plus the same overrides the app passes to
/// `ClerkProvider`. Each accessor returns the sources for one directive, in a
/// stable order without duplicates; [`directives`](Self::directives) yields
/// all of them for a policy builder to merge. See the [module
/// documentation](self) for an example.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "builders are consuming; assign or chain the returned value"]
pub struct ClerkCsp {
    config: Config,
    sources: Sources,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Config {
    publishable_key: String,
    frontend_api_host: String,
    development: bool,
    clerk_js_url: Option<String>,
    clerk_ui_url: Option<String>,
    proxy_url: Option<String>,
    satellite_domain: Option<String>,
    telemetry: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Sources {
    script_src: Vec<String>,
    connect_src: Vec<String>,
    img_src: Vec<String>,
    style_src: Vec<String>,
    frame_src: Vec<String>,
    worker_src: Vec<String>,
}

impl ClerkCsp {
    /// Derive Clerk's CSP sources from a publishable key.
    ///
    /// Returns `None` when the key is not a `pk_test_`/`pk_live_` publishable
    /// key, or does not decode to a plain host name. Clerk cannot load with
    /// such a key either, so there is nothing meaningful to allow.
    pub fn from_publishable_key(publishable_key: &str) -> Option<Self> {
        let decoded = crate::publishable_key::frontend_api_host(publishable_key)?;
        // The host is spliced into a header string, so anything that is not a
        // plain host (a `;`, whitespace, a quote) would end the directive early
        // and inject one of its own. Keys are configuration rather than request
        // input, but a malformed one must not corrupt the app's whole policy.
        let frontend_api_host =
            authority(&decoded).filter(|host| *host == decoded.to_ascii_lowercase())?;
        let development = publishable_key.starts_with("pk_test_");

        let config = Config {
            publishable_key: publishable_key.to_owned(),
            frontend_api_host,
            development,
            clerk_js_url: None,
            clerk_ui_url: None,
            proxy_url: None,
            satellite_domain: None,
            telemetry: development,
        };
        Some(Self::from_config(config))
    }

    /// Mirror `ClerkProvider`'s `clerk_js_url` when the app self-hosts the
    /// clerk-js bundle.
    ///
    /// The bundle's origin is added to `script-src`; a relative URL adds
    /// `'self'`. The Frontend API host stays in `script-src` regardless, as
    /// Clerk requires it there.
    pub fn with_clerk_js_url(mut self, url: impl Into<String>) -> Self {
        self.config.clerk_js_url = Some(url.into());
        self.rebuilt()
    }

    /// Mirror `ClerkProvider`'s `clerk_ui_url` when the app self-hosts the
    /// `@clerk/ui` bundle. Handled like [`with_clerk_js_url`](Self::with_clerk_js_url).
    pub fn with_clerk_ui_url(mut self, url: impl Into<String>) -> Self {
        self.config.clerk_ui_url = Some(url.into());
        self.rebuilt()
    }

    /// Mirror `ClerkProvider`'s `proxy_url` when Frontend API traffic goes
    /// through a reverse proxy.
    ///
    /// clerk-js then talks to the proxy instead of the Frontend API host, so
    /// the proxy's origin replaces the host in `connect-src` (and joins it in
    /// `script-src`). A same-origin proxy such as `/__clerk` contributes
    /// `'self'`. Takes precedence over
    /// [`with_satellite_domain`](Self::with_satellite_domain), as in clerk-js.
    pub fn with_proxy_url(mut self, url: impl Into<String>) -> Self {
        self.config.proxy_url = Some(url.into());
        self.rebuilt()
    }

    /// Mirror `ClerkProvider`'s `domain` on a satellite application
    /// (`is_satellite: true`).
    ///
    /// A production satellite talks to the Frontend API at `clerk.{domain}`
    /// instead of the host in the key; a development satellite keeps using the
    /// key's host, so this has no effect with a `pk_test_` key. Accepts a bare
    /// host (`satellite.example.com`) or a URL.
    pub fn with_satellite_domain(mut self, domain: impl Into<String>) -> Self {
        self.config.satellite_domain = Some(domain.into());
        self.rebuilt()
    }

    /// Whether to allow Clerk's telemetry collector in `connect-src`.
    ///
    /// Defaults to on for development (`pk_test_`) keys and off for production
    /// (`pk_live_`) keys, because clerk-js only sends telemetry from
    /// development instances. Pass `false` when the app also disables
    /// telemetry in `Clerk.load`, e.g. with
    /// `ClerkOptions::new().option("telemetry", false)`, to keep the
    /// development policy as narrow as production. Leaving it out while
    /// telemetry is on is harmless but logs a CSP violation per batch.
    pub fn with_telemetry(mut self, enabled: bool) -> Self {
        self.config.telemetry = enabled;
        self.rebuilt()
    }

    /// The Frontend API host decoded from the publishable key, e.g.
    /// `clerk.example.com`.
    ///
    /// This is the key's host even when a proxy or satellite domain is set:
    /// it is where this crate loads the Clerk bundles from by default.
    pub fn frontend_api_host(&self) -> &str {
        &self.config.frontend_api_host
    }

    /// Sources for `script-src`: the Frontend API host, the origins the Clerk
    /// bundles are loaded from, and Clerk's bot- and fraud-protection hosts.
    pub fn script_src(&self) -> &[String] {
        &self.sources.script_src
    }

    /// Sources for `connect-src`: the Frontend API (or its proxy), the
    /// fraud-protection hosts on any port, Clerk's image host, and the
    /// telemetry collector when enabled.
    pub fn connect_src(&self) -> &[String] {
        &self.sources.connect_src
    }

    /// Sources for `img-src`: Clerk's image host.
    pub fn img_src(&self) -> &[String] {
        &self.sources.img_src
    }

    /// Sources for `style-src`: `'unsafe-inline'`, for Clerk's runtime
    /// CSS-in-JS.
    pub fn style_src(&self) -> &[String] {
        &self.sources.style_src
    }

    /// Sources for `frame-src`: Clerk's bot- and fraud-protection hosts.
    pub fn frame_src(&self) -> &[String] {
        &self.sources.frame_src
    }

    /// Sources for `worker-src`: `'self'` and `blob:`, both required by Clerk.
    pub fn worker_src(&self) -> &[String] {
        &self.sources.worker_src
    }

    /// Every directive Clerk contributes to, with its sources, for merging
    /// into the app's policy.
    ///
    /// Directive names are the lowercase header spellings (`script-src`, ...).
    /// Later releases may yield additional directives as Clerk's requirements
    /// change.
    pub fn directives(&self) -> impl Iterator<Item = (&'static str, &[String])> + '_ {
        let sources = &self.sources;
        [
            ("script-src", sources.script_src.as_slice()),
            ("connect-src", sources.connect_src.as_slice()),
            ("img-src", sources.img_src.as_slice()),
            ("style-src", sources.style_src.as_slice()),
            ("frame-src", sources.frame_src.as_slice()),
            ("worker-src", sources.worker_src.as_slice()),
        ]
        .into_iter()
    }

    fn from_config(config: Config) -> Self {
        let sources = Sources::for_config(&config);
        Self { config, sources }
    }

    fn rebuilt(self) -> Self {
        Self::from_config(self.config)
    }
}

impl Sources {
    fn for_config(config: &Config) -> Self {
        let key_host = format!("https://{}", config.frontend_api_host);
        let frontend_api = config.frontend_api_source();

        let mut script_src = Vec::new();
        // Clerk's guide requires the Frontend API host in `script-src` even
        // when both bundles are self-hosted, and it is where they load from by
        // default.
        push(&mut script_src, key_host);
        push_opt(&mut script_src, frontend_api.clone());
        // Resolved through the same functions the loader injects with, so a
        // change to the default URLs cannot leave these sources behind.
        let bundles = [
            script_urls::clerk_js_url(&config.publishable_key, config.clerk_js_url.as_deref()),
            script_urls::clerk_ui_url(&config.publishable_key, config.clerk_ui_url.as_deref()),
        ];
        for bundle in &bundles {
            push_opt(&mut script_src, url_source(bundle));
        }
        push(&mut script_src, CLOUDFLARE_CHALLENGES);
        push(&mut script_src, CLERK_PROTECT);

        let mut connect_src = Vec::new();
        push_opt(&mut connect_src, frontend_api);
        push(&mut connect_src, CLERK_PROTECT_ANY_PORT);
        // Clerk's automatic config lists the image host here as well as in
        // `img-src`; the manual example lags it.
        push(&mut connect_src, CLERK_IMAGES);
        if config.telemetry {
            for source in CLERK_TELEMETRY {
                push(&mut connect_src, source);
            }
        }

        Self {
            script_src,
            connect_src,
            img_src: vec![CLERK_IMAGES.to_owned()],
            style_src: vec!["'unsafe-inline'".to_owned()],
            frame_src: vec![CLOUDFLARE_CHALLENGES.to_owned(), CLERK_PROTECT.to_owned()],
            worker_src: vec![SELF.to_owned(), "blob:".to_owned()],
        }
    }
}

impl Config {
    /// The source for wherever clerk-js sends Frontend API requests, following
    /// clerk-js's own precedence (`parsePublishableKey` in `@clerk/shared`): a
    /// proxy wins, then a production satellite's `clerk.{domain}`, then the
    /// key's host. `None` only when an override is unusable, in which case
    /// clerk-js cannot reach the Frontend API either.
    fn frontend_api_source(&self) -> Option<String> {
        if let Some(proxy) = &self.proxy_url {
            return url_source(proxy);
        }
        if let Some(domain) = self
            .satellite_domain
            .as_deref()
            .filter(|_| !self.development)
        {
            let domain = domain.trim();
            let host = domain.split_once("://").map_or(domain, |(_, rest)| rest);
            return authority(host).map(|host| format!("https://clerk.{host}"));
        }
        Some(format!("https://{}", self.frontend_api_host))
    }
}

fn push(list: &mut Vec<String>, source: impl Into<String>) {
    let source = source.into();
    if !list.contains(&source) {
        list.push(source);
    }
}

fn push_opt(list: &mut Vec<String>, source: Option<String>) {
    if let Some(source) = source {
        push(list, source);
    }
}

/// The CSP source that allows loading `url`.
///
/// Origin only, never the path: CSP ignores paths after a redirect, and an
/// origin-level source is what Clerk's guide uses. An absolute `http(s)` URL
/// yields `scheme://host[:port]`, a protocol-relative URL the scheme-less host
/// (which, like the URL, follows the page's scheme), and a relative URL
/// `'self'`. Any other scheme, or an authority that is not a plain host, yields
/// `None` rather than a source that could break out of its directive.
fn url_source(url: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }
    if let Some(rest) = url.strip_prefix("//") {
        return authority(rest);
    }
    match url.split_once(':') {
        Some((scheme, rest)) if is_scheme(scheme) => {
            let scheme = scheme.to_ascii_lowercase();
            if scheme != "https" && scheme != "http" {
                return None;
            }
            let authority = authority(rest.strip_prefix("//")?)?;
            Some(format!("{scheme}://{authority}"))
        }
        _ => Some(SELF.to_owned()),
    }
}

/// Whether `candidate` is a URL scheme (RFC 3986: a letter, then letters,
/// digits, `+`, `-`, or `.`), which tells an absolute URL from a relative one
/// whose path happens to contain a colon.
fn is_scheme(candidate: &str) -> bool {
    let mut bytes = candidate.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
}

/// The lowercased `host[:port]` at the start of `rest` (everything up to the
/// path, query, or fragment), if it is a plain DNS name or IPv4 host with an
/// optional numeric port.
///
/// Deliberately stricter than URL parsing: user info, IPv6 literals, and any
/// character outside `[a-z0-9.-]` are refused, because the result is spliced
/// verbatim into a header and CSP host sources cannot express most of them
/// anyway.
fn authority(rest: &str) -> Option<String> {
    let end = rest.find(['/', '?', '#', '\\']).unwrap_or(rest.len());
    let authority = rest[..end].to_ascii_lowercase();
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority.as_str(), None),
    };
    let host_ok = !host.is_empty()
        && !host.starts_with(['.', '-'])
        && !host.ends_with('-')
        && host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'));
    let port_ok =
        port.is_none_or(|port| !port.is_empty() && port.bytes().all(|byte| byte.is_ascii_digit()));
    (host_ok && port_ok).then_some(authority)
}

#[cfg(test)]
mod tests;
