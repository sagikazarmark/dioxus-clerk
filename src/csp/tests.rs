use super::*;

// `pk_test_` + base64("foo.clerk.accounts.dev$").
const DEV_KEY: &str = "pk_test_Zm9vLmNsZXJrLmFjY291bnRzLmRldiQ";
// `pk_live_` + base64("clerk.example.com$").
const LIVE_KEY: &str = "pk_live_Y2xlcmsuZXhhbXBsZS5jb20k";

fn csp(key: &str) -> ClerkCsp {
    ClerkCsp::from_publishable_key(key).expect("test keys decode")
}

fn strings(sources: &[&str]) -> Vec<String> {
    sources.iter().map(|source| (*source).to_owned()).collect()
}

/// Whether `sources` lets the page load `url` as a script, judged the way a
/// browser would for the URL shapes the loader can inject. Written apart from
/// `url_source` so the agreement test does not just compare the parser with
/// itself.
fn allows(sources: &[String], url: &str) -> bool {
    let has = |source: &str| sources.iter().any(|candidate| candidate == source);
    if let Some(rest) = url.strip_prefix("//") {
        return has(rest.split('/').next().unwrap());
    }
    match url.split_once("://") {
        Some((scheme, rest)) => has(&format!("{scheme}://{}", rest.split('/').next().unwrap())),
        None => has("'self'"),
    }
}

#[test]
fn development_key_yields_clerks_documented_sources() {
    let clerk = csp(DEV_KEY);

    assert_eq!(clerk.frontend_api_host(), "foo.clerk.accounts.dev");
    assert_eq!(
        clerk.script_src(),
        strings(&[
            "https://foo.clerk.accounts.dev",
            "https://challenges.cloudflare.com",
            "https://*.protect.clerk.com",
        ])
    );
    assert_eq!(
        clerk.connect_src(),
        strings(&[
            "https://foo.clerk.accounts.dev",
            "https://*.protect.clerk.com:*",
            "https://img.clerk.com",
            "https://clerk-telemetry.com",
            "https://*.clerk-telemetry.com",
        ])
    );
    assert_eq!(clerk.img_src(), strings(&["https://img.clerk.com"]));
    assert_eq!(clerk.style_src(), strings(&["'unsafe-inline'"]));
    assert_eq!(
        clerk.frame_src(),
        strings(&[
            "https://challenges.cloudflare.com",
            "https://*.protect.clerk.com"
        ])
    );
    assert_eq!(clerk.worker_src(), strings(&["'self'", "blob:"]));
}

#[test]
fn telemetry_follows_the_instance_type_by_default() {
    let telemetry = "https://clerk-telemetry.com".to_owned();

    // clerk-js only sends telemetry from development instances.
    assert!(csp(DEV_KEY).connect_src().contains(&telemetry));
    assert!(!csp(LIVE_KEY).connect_src().contains(&telemetry));
}

#[test]
fn telemetry_can_be_overridden_either_way() {
    let telemetry = "https://clerk-telemetry.com".to_owned();

    assert!(
        !csp(DEV_KEY)
            .with_telemetry(false)
            .connect_src()
            .contains(&telemetry)
    );
    assert!(
        csp(LIVE_KEY)
            .with_telemetry(true)
            .connect_src()
            .contains(&telemetry)
    );
}

#[test]
fn rejects_keys_clerk_cannot_load_with() {
    assert!(ClerkCsp::from_publishable_key("sk_test_secret").is_none());
    assert!(ClerkCsp::from_publishable_key("not-a-key").is_none());
    assert!(ClerkCsp::from_publishable_key("pk_test_not*base64").is_none());
    // base64("$"): decodes to an empty host.
    assert!(ClerkCsp::from_publishable_key("pk_test_JA").is_none());
}

#[test]
fn rejects_a_key_whose_host_would_break_out_of_its_directive() {
    // base64("evil.com; script-src *$").
    let key = "pk_test_ZXZpbC5jb207IHNjcmlwdC1zcmMgKiQ";
    assert!(ClerkCsp::from_publishable_key(key).is_none());
}

#[test]
fn script_src_allows_every_bundle_the_loader_would_inject() {
    let overrides = [
        None,
        Some("https://cdn.example.com/assets/clerk.js"),
        Some("http://localhost:8080/clerk.js"),
        Some("//static.example.com/clerk.js"),
        Some("/assets/clerk.js"),
    ];

    for key in [DEV_KEY, LIVE_KEY] {
        for js in overrides {
            for ui in overrides {
                let mut clerk = csp(key);
                if let Some(url) = js {
                    clerk = clerk.with_clerk_js_url(url);
                }
                if let Some(url) = ui {
                    clerk = clerk.with_clerk_ui_url(url);
                }

                for injected in [
                    script_urls::clerk_js_url(key, js),
                    script_urls::clerk_ui_url(key, ui),
                ] {
                    assert!(
                        allows(clerk.script_src(), &injected),
                        "{injected} (key {key}) is not allowed by {:?}",
                        clerk.script_src()
                    );
                }
            }
        }
    }
}

#[test]
fn self_hosted_bundles_keep_the_frontend_api_host() {
    let clerk = csp(LIVE_KEY)
        .with_clerk_js_url("https://cdn.example.com/clerk.browser.js")
        .with_clerk_ui_url("https://cdn.example.com/ui.browser.js");

    assert_eq!(
        clerk.script_src(),
        strings(&[
            "https://clerk.example.com",
            "https://cdn.example.com",
            "https://challenges.cloudflare.com",
            "https://*.protect.clerk.com",
        ])
    );
}

#[test]
fn unusable_override_urls_contribute_nothing() {
    let baseline = csp(LIVE_KEY);
    for url in [
        "data:text/javascript,alert(1)",
        "https://cdn.example.com; script-src *",
        "https://user@cdn.example.com/clerk.js",
        "https://[::1]/clerk.js",
        "https:cdn.example.com/clerk.js",
        "",
    ] {
        let clerk = csp(LIVE_KEY).with_clerk_js_url(url).with_clerk_ui_url(url);
        assert_eq!(
            clerk.script_src(),
            baseline.script_src(),
            "override {url:?}"
        );
    }
}

#[test]
fn a_proxy_replaces_the_frontend_api_host_in_connect_src() {
    let clerk = csp(LIVE_KEY).with_proxy_url("https://example.com/__clerk");

    assert_eq!(clerk.connect_src()[0], "https://example.com");
    assert!(
        !clerk
            .connect_src()
            .contains(&"https://clerk.example.com".to_owned())
    );
    // The loader still fetches the bundles from the key's host.
    assert!(
        clerk
            .script_src()
            .contains(&"https://clerk.example.com".to_owned())
    );
    assert!(
        clerk
            .script_src()
            .contains(&"https://example.com".to_owned())
    );
}

#[test]
fn a_same_origin_proxy_contributes_self() {
    let clerk = csp(LIVE_KEY).with_proxy_url("/__clerk");

    assert_eq!(clerk.connect_src()[0], "'self'");
}

#[test]
fn a_production_satellite_talks_to_its_own_frontend_api() {
    for domain in [
        "satellite.dev",
        "https://satellite.dev",
        "https://satellite.dev/",
    ] {
        let clerk = csp(LIVE_KEY).with_satellite_domain(domain);
        assert_eq!(
            clerk.connect_src()[0],
            "https://clerk.satellite.dev",
            "{domain}"
        );
    }
}

#[test]
fn a_development_satellite_keeps_the_keys_host() {
    let clerk = csp(DEV_KEY).with_satellite_domain("satellite.dev");

    assert_eq!(clerk.connect_src()[0], "https://foo.clerk.accounts.dev");
}

#[test]
fn a_proxy_takes_precedence_over_a_satellite_domain() {
    let clerk = csp(LIVE_KEY)
        .with_satellite_domain("satellite.dev")
        .with_proxy_url("https://proxy.example.com/__clerk");

    assert_eq!(clerk.connect_src()[0], "https://proxy.example.com");
}

#[test]
fn directives_match_the_accessors() {
    let clerk = csp(DEV_KEY).with_proxy_url("https://proxy.example.com");
    let directives: Vec<_> = clerk.directives().collect();

    assert_eq!(
        directives,
        vec![
            ("script-src", clerk.script_src()),
            ("connect-src", clerk.connect_src()),
            ("img-src", clerk.img_src()),
            ("style-src", clerk.style_src()),
            ("frame-src", clerk.frame_src()),
            ("worker-src", clerk.worker_src()),
        ]
    );
}

#[test]
fn source_lists_have_no_duplicates() {
    let clerk = csp(DEV_KEY)
        .with_clerk_js_url("https://foo.clerk.accounts.dev/custom/clerk.js")
        .with_clerk_ui_url("/ui.js")
        .with_proxy_url("/__clerk");

    for (directive, sources) in clerk.directives() {
        let mut unique = sources.to_vec();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), sources.len(), "{directive}: {sources:?}");
    }
}

#[test]
fn url_source_reduces_urls_to_origins() {
    let cases = [
        (
            "https://CDN.Example.com/a/b.js?v=1#x",
            Some("https://cdn.example.com"),
        ),
        ("HTTPS://cdn.example.com", Some("https://cdn.example.com")),
        (
            "https://cdn.example.com:8443/b.js",
            Some("https://cdn.example.com:8443"),
        ),
        ("https://cdn.example.com?x", Some("https://cdn.example.com")),
        ("//cdn.example.com/b.js", Some("cdn.example.com")),
        ("/assets/clerk.js", Some("'self'")),
        ("assets/clerk.js", Some("'self'")),
        ("/a:b/clerk.js", Some("'self'")),
        // A bare `host:port` parses as a URL with scheme `localhost`.
        ("localhost:8080/clerk.js", None),
        ("ftp://cdn.example.com/clerk.js", None),
        ("https://cdn.example.com:/clerk.js", None),
        ("https://cdn.example.com:80a/clerk.js", None),
        ("https://-cdn.example.com/clerk.js", None),
        ("https:///clerk.js", None),
    ];

    for (url, expected) in cases {
        assert_eq!(url_source(url).as_deref(), expected, "{url}");
    }
}
