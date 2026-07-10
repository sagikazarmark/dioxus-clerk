# demo

A single deployable Dioxus app that showcases the library **feature by feature** and doubles as a docs-by-example gallery. Every page mounts a real component next to the exact source that produced it, so the snippet you read is the code that runs. All pages share one `ClerkProvider`, one router layout, and one Tailwind/DaisyUI setup.

The demo has two supported server modes:

- Native Dioxus fullstack with Axum, SSR, and Dioxus server functions.
- Cloudflare Workers as a static SPA plus explicit `/api/*` Worker routes.

## Structure

| Directory | Role |
| --- | --- |
| `src/examples/` | Small, pure components — one per feature. Mounted live *and* rendered as the on-page snippet. |
| `src/pages/` | Route components: prose, docs links, setup callouts, and the example's source via `code!`. |
| `src/app.rs` | Router (`Route`), the single `ClerkProvider`, and the header + grouped sidebar shell. |
| `src/ui.rs` | Presentation-only helpers (`ExampleSection`, `SetupCallout`, …); no Clerk usage. |
| `src/server_api.rs` | The `/server` page's backend: a cookie-verified server fn and a bearer `/api/whoami` route. |
| `src/worker.rs` | Cloudflare Worker entry (`lib.rs`); mirrors the `/api/*` routes. |

The `web`/`server` builds use [`dioxus-code`](https://crates.io/crates/dioxus-code)'s compile-time `code!` macro for the snippets; the Worker build renders no pages, so the highlighter is gated out of the `worker` feature.

## Routes

| Path | Section | What it shows |
| --- | --- | --- |
| `/` | Basics | Auth-aware landing page linking to each feature |
| `/minimal` | Basics | Provider + auth buttons + signed-in/out gates |
| `/buttons` | Basics | `SignInButton`/`SignUpButton`/`SignOutButton` and `AuthButtonMode` (modal vs. redirect) |
| `/sign-in` | Components | Embedded `<SignIn />` widget (path routing) |
| `/sign-up` | Components | Embedded `<SignUp />` widget (path routing) |
| `/profile` | Components | `UserAvatar`, `UserButton`, and inline `<UserProfile />` |
| `/organizations` | Components | Org switcher/list/create/profile + role-gated `Protect` ⚙️ |
| `/waitlist` | Components | `<Waitlist />` widget ⚙️ |
| `/gating` | Advanced | Gates, `ClerkLoading`/`Loaded`/`Failed`, `Protect`, redirects |
| `/hooks` | Advanced | Protected page reading `use_auth`/`use_user`/`use_session` |
| `/imperative` | Advanced | `use_clerk()` actions and awaited `try_*` variants |
| `/errors` | Advanced | `use_clerk_error` + `use_clear_clerk_error` |
| `/server` | Server | Cookie-verified server fn **and** `get_token()` → bearer `/api/whoami` |

⚙️ = needs Clerk Dashboard configuration; each such page carries a callout linking the relevant setting and docs, and degrades to the widget's empty state when unconfigured.

The demo also accepts Clerk-owned child paths under `/sign-in` and `/sign-up`, such as `/sign-in/sso-callback`, so path-routed OAuth and SSO callbacks can mount the same embedded widget instead of failing route parsing.

Styled with [Tailwind CSS](https://tailwindcss.com) + [DaisyUI](https://daisyui.com) using a custom light-default theme.

## Prerequisites

See the [top-level Getting started](../README.md#getting-started) for the Rust toolchain, the `wasm32` target, the `dx` CLI, and how to obtain Clerk keys. Running the tools locally also needs Node (or Bun) for the Tailwind build; the Dagger path below needs none of these. This demo reads both Clerk keys:

| Var | When this example reads it | Notes |
| --- | --- | --- |
| `CLERK_PUBLISHABLE_KEY` | build time, via `env!` | Baked into both the wasm and server binaries — keep it consistent across the two halves. |
| `CLERK_SECRET_KEY` | runtime, server only | Read by the native `serve()` block / Worker when constructing `ClerkAuthLayer`. Never reaches the wasm bundle. |

Provide them by exporting into your shell, or by creating a `.env`. Copy [`.env.dist`](.env.dist) as a starting point:

```bash
cp .env.dist .env   # then fill in pk_test_... / sk_test_...
```

## Run locally

```bash
cd demo

# 1. Keys — skip if you use a repo-root .env under devenv
export CLERK_PUBLISHABLE_KEY=pk_test_xxx
export CLERK_SECRET_KEY=sk_test_xxx

# 2. Install the Tailwind/DaisyUI toolchain
npm install          # or: bun install

# 3. Build the stylesheet (or run `npm run watch` in a second terminal)
npm run build        # or: bun run build

# 4. Serve the native fullstack app (SSR + server functions)
dx serve --fullstack --features fullstack-web
```

`--features fullstack-web` is required: this crate's plain `web` feature is the **Cloudflare-SPA** client (it reaches the backend by fetching the Worker's `/api/*` routes), while `fullstack-web` is the **native fullstack** client (it calls Dioxus server functions directly). Serving with just `dx serve` builds the SPA client, so the server-call page can't reach the local server function and returns `405 Method Not Allowed`.

`dx serve` prints a URL once it's up. Rebuild the CSS whenever you change Tailwind classes, or keep `npm run watch` running alongside. (Or skip all of this and use `dagger call serve up` below.)

## Run with Dagger

Dagger builds and runs everything in containers — no local Node, `dx`, or Wrangler required. It reads the Clerk (and, for the Worker, Cloudflare) keys from a `.env`, so **that file is required** for these commands:

```bash
cd demo

dagger up        # native fullstack, tunnelled to a local port
dagger call worker dev up   # Cloudflare Worker via `wrangler dev`
```

## Build-only check

```bash
cd demo
export CLERK_PUBLISHABLE_KEY=pk_test_dummy

# Static web client used by Cloudflare Workers.
cargo check --no-default-features --features web --target wasm32-unknown-unknown

# Native server build.
cargo check --features server

# Cloudflare Worker build.
cargo check --no-default-features --features worker --target wasm32-unknown-unknown
```

## Notes

- Native fullstack uses Dioxus fullstack + Axum for SSR and server functions.
- Cloudflare Workers use static assets plus explicit Worker API routes. Worker SSR/fullstack is intentionally not part of this demo because standard Dioxus fullstack currently pulls in Tokio networking paths that do not compile for Workers.
- Keep `CLERK_PUBLISHABLE_KEY` consistent between wasm and server builds.
- `dioxus-code` (the snippet highlighter) is compiled only for the `web`/`server` builds; the Worker build renders no pages, so it stays lean.
