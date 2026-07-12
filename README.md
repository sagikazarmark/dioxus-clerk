# dioxus-clerk

[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/sagikazarmark/dioxus-clerk/dagger.yaml?style=flat-square)](https://github.com/sagikazarmark/dioxus-clerk/actions/workflows/dagger.yaml)
[![OpenSSF Scorecard](https://api.securityscorecards.dev/projects/github.com/sagikazarmark/dioxus-clerk/badge?style=flat-square)](https://securityscorecards.dev/viewer/?uri=github.com/sagikazarmark/dioxus-clerk)
[![crates.io](https://img.shields.io/crates/v/dioxus-clerk?style=flat-square)](https://crates.io/crates/dioxus-clerk)
[![docs.rs](https://img.shields.io/docsrs/dioxus-clerk?style=flat-square)](https://docs.rs/dioxus-clerk)

**Clerk integration for [Dioxus](https://dioxuslabs.com): components, hooks, and SSR initial state for web and fullstack apps.**

## Features

- **Web (WASM):** drop-in `<SignIn />`, `<UserButton />`, `<UserProfile />`, organization, and waitlist widgets via Clerk's prebuilt JS UI; unstyled `<SignInButton />` / `<SignUpButton />` / `<SignOutButton />`; reactive `use_auth` / `use_user` / `use_session` hooks; cross-target `<SignedIn>` / `<SignedOut>` / `<Protect>` / `<RedirectToSignIn />` control-flow.
- **Fullstack (Axum):** `ClerkAuthLayer` non-rejecting tower middleware + `current_auth()` context reader for `#[server]` functions, with SSR initial auth state so the client hydrates without a flash of unauthenticated content.
- **Step-up & session tasks:** `use_reverification()` guards sensitive actions behind Clerk reverification; `TaskSetupMFA` mounts clerk-js's MFA-setup task.

## API philosophy

`dioxus-clerk` is a Dioxus-native Clerk integration with React-inspired names
where they map cleanly. APIs such as `ClerkProvider`, `SignIn`, `UserButton`,
`SignedIn`, `use_auth`, and `use_user` should feel familiar to Clerk React
users, but this crate is not a React compatibility layer.

The primary API follows Dioxus and Rust conventions: typed options, signals,
server functions, Axum middleware, and SSR initial auth state. When React
semantics do not fit Dioxus well, this crate chooses the Dioxus-native design
and documents the difference. See [Migrating from Clerk
React](docs/react-migration.md) for the full mapping.

## Getting started

Most apps depend on `dioxus-clerk` directly:

```toml
[dependencies]
dioxus-clerk = "0.2"
```

Fullstack apps enable the `server` feature on the native server build:

```toml
[dependencies]
dioxus-clerk = "0.2"

[features]
default = []
web = []
server = ["dioxus-clerk/server"]
```

### Feature flags

| Feature | Default | Enables |
| --- | --- | --- |
| *(none)* | ✅ | Client components, hooks, guards, Clerk widgets, and SSR initial-state consumption. |
| `server` | | Axum middleware, extractors, `#[server]` context readers (`current_auth`), and SSR initial-state helpers. Enable on the native server build only. |
| `worker` | | `server` plus `Send`-wrapped middleware futures for single-threaded Cloudflare Workers. |

### 60-second setup

For the smallest SPA integration, mount `ClerkProvider` at the app root or a
route-layout root, render a signed-out sign-in button, and render signed-in
user controls:

```rust
use dioxus::prelude::*;
use dioxus_clerk::*;

fn App() -> Element {
    rsx! {
        ClerkProvider { publishable_key: env!("CLERK_PUBLISHABLE_KEY"),
            SignedOut { SignInButton { class: "btn" } }
            SignedIn { UserButton {} }
        }
    }
}
```

`ClerkProvider` is intended to wrap a whole app or route layout, not arbitrary
inline markup. In fullstack/SSR renders it may emit Clerk initial-state markup,
so avoid mounting it inside semantic containers such as `table`, `ul`, `ol`,
or `p`.

The rest of this section covers prerequisites, key sourcing, the bundled demo,
and fullstack setup.

### Get a Clerk publishable key

1. Sign up at [clerk.com](https://clerk.com) and create an application.
2. From the application dashboard, copy the **Publishable key** (`pk_test_...` for development, `pk_live_...` for production).
3. For fullstack apps, also copy the **Secret key** (`sk_test_...` / `sk_live_...`).

### Where the keys live

The library doesn't dictate how you source the publishable key. `ClerkProvider` accepts an optional `publishable_key` prop, web-only apps normally pass it directly, and fullstack apps normally pass it during the server render so the server-rendered SSR initial state can carry the key to the wasm client. A wasm/client render can omit the prop only when an SSR initial-state script already includes the key. Source it however suits your app:

| Strategy | Works on | Notes |
| --- | --- | --- |
| `env!("CLERK_PUBLISHABLE_KEY")` | wasm + native | Build fails at compile time if missing. **What the bundled demo does**: opinionated about catching misconfigured env loud and fast. In fullstack builds this bakes the same value into both halves, so keep the build env consistent. |
| `option_env!("CLERK_PUBLISHABLE_KEY")` | wasm + native | Same as `env!` but returns `Option<&str>` instead of erroring; lets you fall back to a runtime lookup. |
| `std::env::var("CLERK_PUBLISHABLE_KEY")` | server / native only | Read at runtime. Use on the server render or axum entrypoint, not in wasm code. |
| Hardcoded `&str` constant | anywhere | Fine for hello-world / dev. |
| Fetched from a `/config` endpoint before provider mount, or hydrated via SSR initial state | wasm at first provider render | Pattern for multi-tenant apps where the key varies per deploy. Render `ClerkProvider` only after the key is available; later `publishable_key` prop changes are intentionally ignored. For fullstack SSR hydration, the server-rendered provider must receive the key so it can emit it into the SSR initial state; the bundled demo uses the simpler `env!` path. |

For server-side `ClerkAuthLayer` setup, the **secret key** has to be runtime env (or some other secret store); it must never reach the wasm bundle. Use `dioxus_clerk::server::ClerkAuthLayer::from_env()` for the conventional `CLERK_SECRET_KEY`, pass an explicit secret to `ClerkAuthLayer::new(secret)`, or use `ClerkAuthLayerConfig` plus `ClerkAuthLayer::from_config(...)` when you need a non-default Backend API base URL or optional claim validation such as authorized parties/audience. The publishable key stays separate: pass it to `ClerkProvider` on first server render, pass it directly in web-only apps, or arrange for SSR initial state / client config to provide it before clerk-js starts.

### Run the demo

The demo uses `env!`, so the build fails at compile time with a clear rustc
error if `CLERK_PUBLISHABLE_KEY` isn't set:

```bash
# Inline for one-off runs:
CLERK_PUBLISHABLE_KEY=pk_test_xxx dx serve

# Or export in your shell (the server also needs the secret key):
export CLERK_PUBLISHABLE_KEY=pk_test_xxx
export CLERK_SECRET_KEY=sk_test_xxx
```

Because `env!` resolves at build time, the same value must be present when the wasm and server halves of a fullstack app are compiled. `dx serve` handles this automatically. If clerk-js init fails at runtime (bad key, dashboard origin not whitelisted, network), `use_clerk_error()` exposes the failure so apps can render an error UI instead of staying stuck on `Loading`.

To run the whole demo in containers without a local Node/`dx` toolchain, use Dagger instead (it reads the keys from a `.env` file):

```bash
cd demo
dagger up
```

See [`demo/README.md`](demo/README.md) for full run instructions, the Cloudflare Worker mode, and the matching Dagger commands. The demo combines the minimal SPA, router, and fullstack server-function flows into one app.

## Quickstart (web-only SPA)

```rust
use dioxus::prelude::*;
use dioxus_clerk::*;

fn main() { launch(App); }

fn App() -> Element {
    let pk = option_env!("CLERK_PUBLISHABLE_KEY")
        .map(String::from)
        .unwrap_or_else(|| "pk_test_REPLACE_ME".into());
    rsx! {
        ClerkProvider { publishable_key: pk,
            ClerkFailed { p { "Auth failed to initialize." } }
            SignedOut { SignInButton {} }
            SignedIn {
                UserButton {}
                SignOutButton {}
            }
        }
    }
}
```

See `demo/` for a working app; its `/minimal` route contains the same web-only shape.

## Quickstart (fullstack)

The `demo/` app uses this fullstack shape:

```rust
#[cfg(feature = "server")]
{
    use dioxus::server::{axum, serve, DioxusRouterExt, ServeConfig};
    use dioxus_clerk::server::ClerkAuthLayer;
    serve(|| async move {
        let auth_layer = ClerkAuthLayer::from_env().expect("CLERK_SECRET_KEY");
        Ok(axum::Router::new()
            .serve_dioxus_application(ServeConfig::new(), App)
            .layer(auth_layer))
    });
}

#[server]
async fn whoami() -> Result<String, ServerFnError> {
    use dioxus_clerk::server::current_auth;
    let auth = current_auth()?;
    Ok(auth.user_id)
}
```

With the `server` feature enabled, `ClerkError` converts into Dioxus'
`ServerFnError`, so server functions returning `Result<_, ServerFnError>` can
use `current_auth()?` directly.

## Auth lifecycle

`window.Clerk` and auth loadedness are separate lifecycle facts:

- `window.Clerk` appears after clerk-js has executed in the browser.
- `AuthState::is_loaded` becomes true only after `Clerk.load()` completes.
- `AuthState::status()` is the safest rendering input: `Loading`, `SignedOut`, or `SignedIn`.
- Fullstack SSR initial state can report `is_signed_in: true` while `is_loaded: false`.
- `use_auth()` exposes the signed-in initial auth snapshot immediately, including `user_id`, `session_id`, `org_id`, and `org_slug` when available.
- `use_user()` and `use_session()` mirror Clerk React's stateful hook shape with `status`, `is_loaded`, `is_signed_in`, and optional hydrated browser `User` / `Session` details.

This means a signed-in fullstack page can render plain signed-in guards during
hydration without waiting for clerk-js details. Transient clerk-js loading
observations preserve existing signed-in knowledge, so signed-in SSR initial state
should not flash signed-out or loading UI while browser details catch up.

## Migrating from Clerk React

The public API intentionally follows Clerk React names where Dioxus can support
the same concept directly. Prop names become Rust snake_case, finite options
become Rust enums, and the unstyled buttons render a native `<button>` (don't
nest your own).

| Clerk React | dioxus-clerk |
| --- | --- |
| `<ClerkProvider publishableKey=...>` | `ClerkProvider { publishable_key: ... }` |
| `<SignIn />`, `<SignUp />`, `<Waitlist />` | `SignIn {}`, `SignUp {}`, `Waitlist {}`; mounted widgets support `fallback`, host `class`, and host `id` |
| `<UserButton />`, `<UserProfile />`, `<UserAvatar />` | Same names; `UserAvatar` is a lightweight `<img>` from `use_user()` |
| `<SignInButton />`, `<SignUpButton />`, `<SignOutButton />` | Same names; render a native `<button>`, `type="button"` by default |
| `<SignedIn />`, `<SignedOut />`, `<Protect />` | Same names; `Protect` supports `role` / `permission` |
| `<RedirectToSignIn />`, `<RedirectToSignUp />` | Same names |
| `<ClerkLoading />`, `<ClerkLoaded />`, `<ClerkFailed />` | Same names |
| `useAuth()`, `useUser()`, `useSession()`, `useClerk()` | `use_auth()`, `use_user()`, `use_session()`, `use_clerk()` |
| `useReverification()` | `use_reverification()` |

**See [docs/react-migration.md](docs/react-migration.md)** for the full parity
matrix, the list of missing React APIs and their workarounds, intentional
behavioural differences, and a side-by-side migration cookbook.

## Recipes

### Configure Clerk routing with props

```rust
ClerkProvider {
    publishable_key: pk,
    sign_in_url: "/sign-in",
    sign_up_url: "/sign-up",
    sign_in_fallback_redirect_url: "/dashboard",
    sign_up_fallback_redirect_url: "/dashboard",
    Router::<Route> {}
}
```

Common Clerk options are component props. Use the `options` prop as an advanced raw escape hatch for Clerk options this crate has not named yet; explicit props win when both set the same Clerk option key.

When using `Routing::Path` for embedded auth widgets, make sure your app router also matches Clerk-owned child paths under the widget URL. OAuth and SSO flows can return to paths such as `/sign-in/sso-callback`; in Dioxus Router, add a catch-all route such as `#[route("/sign-in/:..segments")]` that renders the same `<SignIn />` page. Use `Routing::Hash` if you want Clerk's internal routes kept out of the app path.

```rust
SignIn {
    routing: Routing::Hash,
    path: "/sign-in",
    fallback: rsx! { div { class: "skeleton h-96 w-full" } },
    options: serde_json::json!({
        "appearance": { "variables": { "colorPrimary": "blue" } }
    }),
}
```

### Programmatic Clerk actions

```rust
let clerk = use_clerk();
clerk.open_sign_in();
clerk.open_sign_up_with_options(SignUpOptions::new().routing(Routing::Hash));
clerk.sign_out_with_options(SignOutOptions::new().redirect_url("/"));
```

`use_clerk()` is cross-target, so fullstack components no longer need `cfg(target_arch = "wasm32")` just to render auth buttons. Browser actions are still only executed after hydration in the browser.

Use the `try_*` variants when the caller needs to await completion or handle
errors locally:

```rust
let clerk = use_clerk();
rsx! {
    button {
        onclick: move |_| async move {
            if let Err(err) = clerk.try_sign_out().await {
                // Show local error UI, or fall back to use_clerk_error().
                let _ = err;
            }
        },
        "Sign out"
    }
}
```

`use_auth().get_token().await` mirrors Clerk React's `useAuth().getToken()`,
and `use_auth().sign_out()` mirrors the common `useAuth().signOut()` path:

```rust
let auth = use_auth();
let token = auth
    .get_token_with_options(
        GetTokenOptions::new()
            .template("api")
            .organization_id("org_2ghi")
            .leeway_in_seconds(30),
    )
    .await?;
auth.sign_out_with_options(SignOutOptions::new().redirect_url("/"));
```

Use `use_clerk()` directly for custom design-system buttons:

```rust
let clerk = use_clerk();
rsx! {
    button {
        class: "btn btn-primary",
        onclick: move |_| clerk.open_sign_in(),
        "Sign in"
    }
}
```

The generated `<SignInButton />`, `<SignUpButton />`, and `<SignOutButton />`
components render their own native `<button>` and treat children as the button
contents. Unlike Clerk React, they are not wrappers that can safely receive a
nested custom `<button>`.

### Step-up reverification

`use_reverification()` guards a sensitive action: run it through `guard`, and if
the action reports `ClerkError::NeedsReverification` (typically mapped from a
server 403 via `ClerkError::from_reverification_hint`), the guard opens clerk-js's
reverification prompt and retries the action once. A dismissed prompt surfaces as
`ClerkError::ReverificationCancelled`.

```rust
let reverify = use_reverification();
rsx! {
    button {
        onclick: move |_| async move {
            let outcome = reverify.guard(|| async move {
                delete_account().await // #[server] fn that may demand step-up
            }).await;

            match outcome {
                Ok(_) => { /* action ran */ }
                Err(ClerkError::ReverificationCancelled) => { /* user dismissed */ }
                Err(_error) => { /* surface the failure */ }
            }
        },
        "Delete account"
    }
}
```

### Session tasks (MFA setup)

When a session is in a pending task state, read the task from
`use_session().session().current_task` and mount the matching task widget, or
pass `task_urls` to `ClerkProvider` to let clerk-js route pending users
automatically. Opt the gate in to pending sessions with
`treat_pending_as_signed_out: false`, otherwise `SignedIn` hides a pending user:

```rust
SignedIn { treat_pending_as_signed_out: false,
    if let Some(task) = use_session().session().and_then(|s| s.current_task) {
        match task.key {
            SessionTaskKey::SetupMfa => rsx! { TaskSetupMFA {} },
            key => rsx! { p { "Pending task: {key}. Route the user here." } },
        }
    }
}
```

### Router and script loading

`router_push` and `router_replace` are forwarded to Clerk as SPA navigation
callbacks. Pass `Some(Callback::new(...))` when your router can navigate from a
string path.

```rust
ClerkProvider {
    publishable_key: pk,
    sign_in_url: "/sign-in",
    sign_up_url: "/sign-up",
    router_push: Some(Callback::new(move |path: String| {
        // Navigate with your app router here.
    })),
    Router::<Route> {}
}
```

For CSP or custom script hosting, configure the injected clerk-js script:

```rust
ClerkProvider {
    publishable_key: pk,
    script_nonce: nonce,
    clerk_js_url: "https://cdn.example.com/clerk.browser.js",
    // Set false if you load clerk-js yourself before the provider starts.
    load_clerk_js: true,
    AppRoutes {}
}
```

### Loading and failure UI

```rust
ClerkLoading { p { "Loading auth..." } }
ClerkLoaded { p { "Auth is ready." } }
ClerkFailed { p { "Auth failed to initialize." } }
```

Use `use_clerk_error()` when you want to display the specific error. Use
`use_clear_clerk_error()` from a dismiss button when the displayed error is
recoverable:

```rust
let error = use_clerk_error();
let clear_error = use_clear_clerk_error();

rsx! {
    if let Some(error) = error.read().as_ref() {
        div { class: "alert alert-error",
            span { "{error}" }
            button { onclick: move |_| clear_error.call(()), "Dismiss" }
        }
    }
}
```

In fullstack/SSR apps, prefer `SignedOutWhenLoaded` for signed-out CTAs that
should not flash before clerk-js has finished checking the browser session:

```rust
SignedOutWhenLoaded {
    fallback: rsx! { p { "Checking auth..." } },
    SignInButton { class: "btn", "Sign in" }
}
SignedIn { UserButton {} }
```

### Rendering gates

```rust
Protect { permission: "org:read", span { "Protected" } }

Protect {
    permission: "org:invoices:create",
    fallback: rsx! { span { "No access" } },
    span { "Create invoice" }
}
```

`use_auth().has_role(...)`, `has_permission(...)`, and `has(AuthRequirement::permission(...))` use the same server-verified org claims.

### Axum handlers

For Dioxus server functions, use `current_auth()` / `current_auth_opt()`. For
regular Axum handlers under `ClerkAuthLayer`, use extractors:

```rust
use dioxus_clerk::server::ClerkAuth;

async fn private_handler(auth: ClerkAuth) -> String {
    auth.user_id
}

async fn public_handler(auth: Option<ClerkAuth>) -> String {
    auth.map(|auth| auth.user_id).unwrap_or_else(|| "anonymous".into())
}
```

## Development

The crate targets both native (`server`) and wasm (`web`) builds, so the checks
span two targets. The pure Rust/Dioxus workflow needs the toolchain pinned in
[`rust-toolchain.toml`](rust-toolchain.toml) with the `wasm32-unknown-unknown`
target and `wasm-pack`:

```bash
# Format and lint
cargo fmt
cargo clippy --all-targets

# Build both halves: native/server and web (wasm)
cargo build --features server
cargo build --target wasm32-unknown-unknown

# Build the docs
cargo doc --no-deps

# Run the native + integration tests
cargo test

# Run the wasm-bindgen browser tests
wasm-pack test --headless --chrome
```

Dagger runs the same build, clippy, doc, test, and wasm-pack checks in pinned
containers with a single command; this is exactly what CI runs:

```bash
dagger check
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
