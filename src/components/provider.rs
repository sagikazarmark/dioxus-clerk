//! Root provider component.
//!
//! Cross-target. On the server: emits a `<script id="__clerk_initial_state">`
//! element and provides context. On wasm: consumes SSR initial state if present,
//! then delegates clerk-js loading and auth updates to the Clerk lifecycle.

use crate::context::ClerkContext;
use crate::core::{AuthRuntimeState, ClerkError};
use crate::options::ClerkOptions;
use crate::ssr::INITIAL_STATE_SCRIPT_ID;
use dioxus::prelude::*;

/// Root provider. Place at the top of your app or route layout.
///
/// Use exactly one `ClerkProvider` per document: a server render emits a
/// `<script id="__clerk_initial_state">` element, and two providers would
/// produce duplicate element ids.
///
/// Configuration props (`publishable_key`, `options`, `clerk_js_url`, ...)
/// are read once when the provider first mounts, matching clerk-js's
/// load-once model; later prop changes are ignored for the lifetime of the
/// provider instance.
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn App() -> Element {
///     // Bake the publishable key in at compile time, e.g.
///     // `env!("CLERK_PUBLISHABLE_KEY")`. `option_env!` keeps builds without
///     // the variable working (the provider then reports a config error).
///     let publishable_key = option_env!("CLERK_PUBLISHABLE_KEY").map(String::from);
///
///     rsx! {
///         ClerkProvider { publishable_key,
///             SignedOut { SignInButton {} }
///             SignedIn { UserButton {} }
///         }
///     }
/// }
/// ```
#[component]
pub fn ClerkProvider(
    /// Required on web-only apps. In fullstack apps, pass this during the
    /// server render so SSR initial state can carry it to the wasm client; the
    /// wasm/client render may omit it only when that SSR state is present.
    #[props(into)]
    publishable_key: Option<String>,
    /// Application sign-in URL forwarded to `Clerk.load(...)`.
    #[props(into)]
    sign_in_url: Option<String>,
    /// Application sign-up URL forwarded to `Clerk.load(...)`.
    #[props(into)]
    sign_up_url: Option<String>,
    /// URL to redirect to after sign-in when no `redirect_url` is in play.
    #[props(into)]
    sign_in_fallback_redirect_url: Option<String>,
    /// URL to always redirect to after sign-in, overriding any `redirect_url`.
    #[props(into)]
    sign_in_force_redirect_url: Option<String>,
    /// URL to redirect to after sign-up when no `redirect_url` is in play.
    #[props(into)]
    sign_up_fallback_redirect_url: Option<String>,
    /// URL to always redirect to after sign-up, overriding any `redirect_url`.
    #[props(into)]
    sign_up_force_redirect_url: Option<String>,
    /// URL Clerk should use after sign-out.
    #[props(into)]
    after_sign_out_url: Option<String>,
    /// URL Clerk should use after signing out one account in multi-session apps.
    #[props(into)]
    after_multi_session_single_sign_out_url: Option<String>,
    /// URL Clerk should use after switching sessions in multi-session apps.
    #[props(into)]
    after_switch_session_url: Option<String>,
    /// Application waitlist URL forwarded to `Clerk.load(...)`.
    #[props(into)]
    waitlist_url: Option<String>,
    /// Application user profile URL forwarded to `Clerk.load(...)`.
    #[props(into)]
    user_profile_url: Option<String>,
    /// Application organization profile URL forwarded to `Clerk.load(...)`.
    #[props(into)]
    organization_profile_url: Option<String>,
    /// Application create-organization URL forwarded to `Clerk.load(...)`.
    #[props(into)]
    create_organization_url: Option<String>,
    /// Clerk proxy URL for reverse-proxy deployments.
    #[props(into)]
    proxy_url: Option<String>,
    /// Clerk satellite-domain setting when the app is a satellite application.
    #[props(into)]
    domain: Option<String>,
    /// Whether Clerk should treat this app as a satellite application.
    is_satellite: Option<bool>,
    /// Whether satellite apps should automatically sync on initial page load.
    satellite_auto_sync: Option<bool>,
    /// Whether Clerk should prefetch its UI package when supported.
    prefetch_ui: Option<bool>,
    /// Clerk `allowedRedirectOrigins` list.
    allowed_redirect_origins: Option<Vec<String>>,
    /// Clerk `allowedRedirectProtocols` list.
    allowed_redirect_protocols: Option<Vec<String>>,
    /// Raw Clerk appearance object.
    appearance: Option<serde_json::Value>,
    /// Raw Clerk localization object.
    localization: Option<serde_json::Value>,
    /// Advanced options forwarded to `Clerk.load(...)`, as a
    /// [`ClerkOptions`](crate::ClerkOptions) builder or a raw
    /// `serde_json::Value`. Explicit props win when both set the same Clerk
    /// option key.
    #[props(default = ClerkOptions::from_value(serde_json::Value::Null), into)]
    options: ClerkOptions,
    /// Optional CSP nonce added to the injected clerk-js script tag.
    #[props(into)]
    script_nonce: Option<String>,
    /// Override the clerk-js script URL. Defaults to Clerk's CDN package URL.
    #[props(into)]
    clerk_js_url: Option<String>,
    /// Override the `@clerk/ui` script URL. clerk-js 6 loads UI components from
    /// this separate bundle; defaults to Clerk's CDN package URL matched to the
    /// same Frontend API host as clerk-js. Set alongside `clerk_js_url` when
    /// self-hosting both bundles.
    #[props(into)]
    clerk_ui_url: Option<String>,
    /// Whether `ClerkProvider` should inject clerk-js. Disable when the app
    /// loads clerk-js externally.
    #[props(default = true)]
    load_clerk_js: bool,
    /// Optional router push callback forwarded to `Clerk.load(...)` as
    /// `routerPush` for SPA-native navigation.
    ///
    /// Read at the page's single `Clerk.load()`: clerk-js loads once per
    /// page, so a remounted provider cannot add or remove router callbacks —
    /// pass them on the provider that first loads clerk-js.
    #[props(into)]
    router_push: Option<Callback<String>>,
    /// Optional router replace callback forwarded to `Clerk.load(...)` as
    /// `routerReplace` for SPA-native navigation. Read at the page's single
    /// `Clerk.load()`, like `router_push`.
    #[props(into)]
    router_replace: Option<Callback<String>>,
    children: Element,
) -> Element {
    let options = options
        .maybe_sign_in_url(sign_in_url)
        .maybe_sign_up_url(sign_up_url)
        .maybe_sign_in_fallback_redirect_url(sign_in_fallback_redirect_url)
        .maybe_sign_in_force_redirect_url(sign_in_force_redirect_url)
        .maybe_sign_up_fallback_redirect_url(sign_up_fallback_redirect_url)
        .maybe_sign_up_force_redirect_url(sign_up_force_redirect_url)
        .maybe_after_sign_out_url(after_sign_out_url)
        .maybe_after_multi_session_single_sign_out_url(after_multi_session_single_sign_out_url)
        .maybe_after_switch_session_url(after_switch_session_url)
        .maybe_waitlist_url(waitlist_url)
        .maybe_user_profile_url(user_profile_url)
        .maybe_organization_profile_url(organization_profile_url)
        .maybe_create_organization_url(create_organization_url)
        .maybe_proxy_url(proxy_url)
        .maybe_domain(domain)
        .maybe_is_satellite(is_satellite)
        .maybe_satellite_auto_sync(satellite_auto_sync)
        .maybe_prefetch_ui(prefetch_ui)
        .maybe_allowed_redirect_origins(allowed_redirect_origins)
        .maybe_allowed_redirect_protocols(allowed_redirect_protocols)
        .maybe_appearance(appearance)
        .maybe_localization(localization)
        .into_value();

    // Startup discovery reads the document / fullstack context; run it once
    // per provider instance rather than on every re-render.
    let startup = use_hook({
        let publishable_key = publishable_key.clone();
        move || std::rc::Rc::new(crate::startup::provider_startup(publishable_key))
    });
    let initial_state_json = startup.initial_state_json.clone();

    let auth = use_signal({
        let initial_auth = startup.auth.clone();
        move || initial_auth.clone()
    });
    let load_error = use_signal(|| None);
    // Startup config warnings (seed mismatch, malformed seed) are non-fatal:
    // loading proceeds, so they go to the recoverable channel where they
    // surface through `use_clerk_error` without gating the action pipeline
    // or rendering `ClerkFailed`.
    let action_error = use_signal({
        let warning = startup.warning.clone();
        move || warning.clone()
    });
    let pending = use_signal(Vec::new);

    #[cfg(clerk_client)]
    crate::lifecycle::use_drive_lifecycle(
        crate::lifecycle::ClerkLifecycleSignals { auth, load_error },
        startup.publishable_key.clone(),
        crate::lifecycle::ClerkLoadOptions {
            value: options.clone(),
            router_push,
            router_replace,
        },
        crate::lifecycle::ClerkScriptOptions {
            load_clerk_js,
            script: crate::loader::ScriptOptions {
                url: clerk_js_url,
                ui_url: clerk_ui_url,
                nonce: script_nonce,
            },
        },
    );

    #[cfg(not(clerk_client))]
    let _ = &options;
    #[cfg(not(clerk_client))]
    let _ = (
        &script_nonce,
        &clerk_js_url,
        &clerk_ui_url,
        load_clerk_js,
        &router_push,
        &router_replace,
    );

    rsx! {
        if let Some(json) = initial_state_json {
            script {
                id: INITIAL_STATE_SCRIPT_ID,
                r#type: "application/json",
                dangerous_inner_html: "{json}",
            }
        }
        ClerkContextProvider { auth, load_error, action_error, pending, children }
    }
}

#[component]
fn ClerkContextProvider(
    auth: Signal<AuthRuntimeState>,
    load_error: Signal<Option<ClerkError>>,
    action_error: Signal<Option<ClerkError>>,
    pending: Signal<Vec<crate::actions::ClerkOperation>>,
    children: Element,
) -> Element {
    let ctx = use_context_provider(|| ClerkContext {
        auth,
        load_error,
        action_error,
        pending,
    });

    // One Clerk action dispatch scheduler per provider: every hook's
    // fire-and-forget operations drain through this queue in request order.
    #[cfg(clerk_client)]
    crate::actions::use_action_scheduler(ctx);
    #[cfg(not(clerk_client))]
    let _ = ctx;

    rsx! { {children} }
}
