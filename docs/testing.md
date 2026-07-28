# Testing

Most tests that involve Clerk are not *about* Clerk. You want a signed-in user
so you can test what your application does. This guide starts there, then
covers testing the auth behaviour itself, and finally testing through real
Clerk when you need it.

## Choosing an approach

| | What it needs | Clerk UI | User IDs | Good for |
| --- | --- | --- | --- | --- |
| [`TestClerk`](#testing-your-application) | Nothing | Not rendered | You choose them | Your app's logic, guards, org-scoped views. The default. |
| [`TestIssuer` / `TestSession`](#testing-authentication-itself) | Nothing | Not rendered | You choose them | Auth behaviour: permissions, expiry, what gets rejected. |
| [`@clerk/testing`](#through-real-clerk) | Live Clerk instance, secrets, network | Real and drivable | Assigned by Clerk | Sign-in/sign-up flows, MFA — Clerk's own UI. |

The first two are the same offline machinery at different altitudes: no Clerk
instance, no network, no secrets in CI. Reach for the third only for the
handful of tests that go *through* Clerk's interface.

Tokens from all three are equally valid to your backend — the real-Clerk mode
issues genuine Clerk-signed session tokens, and the offline modes issue tokens
your layer is configured to trust. The catch is that a given server process
trusts [one family or the other](#the-two-modes-cannot-share-a-server), not both.

---

## Setup

Add the crate under dev-dependencies with the `testing` feature. Keep it out of
`[dependencies]` — it mints tokens a correctly configured verifier accepts.

```toml
[dependencies]
dioxus-clerk = { version = "0.4", features = ["server"] }

[dev-dependencies]
dioxus-clerk = { version = "0.4", features = ["server", "testing"] }
```

### Environment variables

**None are required.** No `CLERK_SECRET_KEY`, no `CLERK_PUBLISHABLE_KEY`,
nothing in CI secrets.

Two caveats:

- If your application calls `ClerkAuthLayerConfig::from_env()` in its own
  wiring, tests must not go through that path. See
  [Wiring your app for tests](#wiring-your-app-for-tests).
- A `.env` file with real Clerk keys can leak into a test run and make failures
  confusing. Prefer an explicit test config over ambient environment.

---

## Testing your application

`TestClerk` is the whole API when auth is a precondition rather than the
subject. It generates a key, wires a layer that verifies against it, and hands
you credentials:

```rust,ignore
use dioxus_clerk::testing::TestClerk;

#[tokio::test]
async fn the_dashboard_lists_the_users_projects() {
    let clerk = TestClerk::new().unwrap();
    let app = my_app::router(clerk.layer().unwrap());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/dashboard")
                .header(header::COOKIE, clerk.cookie("user_2abc").unwrap())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    // ...assert on what your app actually rendered.
}
```

That's the entire surface for this case:

| Call | Gives you |
| --- | --- |
| `clerk.layer()` | A `ClerkAuthLayer` that verifies this setup's tokens and never hits the network. |
| `clerk.cookie(user_id)` | A `__session=<token>` header value — what Clerk's browser SDK sends. |
| `clerk.bearer(user_id)` | An `Authorization: Bearer <token>` value. Verification accepts either. |
| `clerk.config()` | The layer config, if you need to harden it before building the layer. |

An unauthenticated request is just a request with no cookie — nothing special
needed.

### Share one setup

RSA-2048 generation costs ~100ms and is variable. Fine once per suite, wasteful
per test:

```rust,ignore
use std::sync::LazyLock;
use dioxus_clerk::testing::TestClerk;

static CLERK: LazyLock<TestClerk> = LazyLock::new(|| TestClerk::new().expect("test Clerk"));
```

Each integration test binary is its own process and gets its own key, which is
fine — each also builds its layer from that same setup.

### Wiring your app for tests

The one structural change worth making: let the router take a layer instead of
building one from the environment internally.

```rust,ignore
// Production wiring reads the environment; tests hand in a test layer.
pub fn router(auth: ClerkAuthLayer) -> Router {
    Router::new()
        .route("/dashboard", get(dashboard))
        .layer(auth)
}

pub fn router_from_env() -> Result<Router, ClerkError> {
    Ok(router(ClerkAuthLayer::from_config(
        ClerkAuthLayerConfig::from_env()?,
    )?))
}
```

### Signing in as someone specific

`clerk.session(user_id)` returns a `TestSession` to customize, and
`cookie_for` / `bearer_for` / `token_for` sign it:

```rust,ignore
let admin = clerk
    .session("user_admin")
    .with_organization("org_acme")
    .with_organization_role("org:admin")
    .with_organization_permissions(["org:dashboard:manage"]);

let cookie = clerk.cookie_for(&admin).unwrap();
```

### Choosing user IDs

You pick the user ID, and that is a large part of why this mode suits
application tests. If your app keys data on the user — rows, ownership checks,
seeded fixtures — a chosen ID lets the fixtures hardcode it. Nothing has to be
discovered at runtime, and nothing is coupled to a Clerk instance's internal
state.

Verification only requires `sub` to be non-empty; there is no format check and
no `user_` prefix requirement. Use obviously-synthetic, `user_`-prefixed IDs
anyway, so nothing in your app trips on a format assumption and a test ID is
never mistaken for a real one in a log:

```rust,ignore
pub const TEST_ADMIN: &str = "user_test_admin";
pub const TEST_MEMBER: &str = "user_test_member";
```

The contrast is with [`@clerk/testing`](#through-real-clerk), where the ID is
assigned by Clerk, opaque, and changes if the test user or instance is ever
recreated. If your test needs a *specific* user, mint the token here rather
than signing in through Clerk.

### SSR tests

Server-rendered output for a signed-in user needs no token, no layer, and no
key — construct the initial state directly:

```rust,ignore
use dioxus_clerk::core::ClerkAuth;
use dioxus_clerk::ssr::{InitialAuthSnapshot, InitialState};

let mut auth = ClerkAuth::new("user_2abc", 9_999_999_999);
auth.session_id = Some("sess_2def".into());
auth.org_role = Some("org:admin".into());

let signed_in = InitialState::new(InitialAuthSnapshot::from(&auth), Some("pk_test_..."));
let signed_out = InitialState::new(InitialAuthSnapshot::signed_out(), Some("pk_test_..."));
```

This is the cheapest way to assert that `<SignedIn>` / `<SignedOut>` /
`<Protect>` render the right branch, and that hydration starts from the right
state instead of flashing unauthenticated content.

---

## Testing authentication itself

When the auth behaviour *is* the subject, drop to `TestIssuer` and
`TestSession`. `TestClerk::issuer()` exposes the issuer if you started with the
wrapper.

```rust,ignore
use dioxus_clerk::server::{ClerkAuthLayer, ClerkAuthLayerConfig};
use dioxus_clerk::testing::{TestIssuer, TestSession};

let issuer = TestIssuer::generate()?;
let layer = ClerkAuthLayer::from_config(
    ClerkAuthLayerConfig::new("").with_static_jwks(issuer.jwks_json()?),
)?;
```

`with_static_jwks` is what makes this offline: the layer verifies against a
fixed keyset, so no HTTP client is built and nothing is fetched. The secret key
goes unused and can be empty.

> `with_static_jwks` is not test-only — it also pins signing keys in
> production. But a fixed keyset does not rotate, so tokens signed by a newly
> rotated Clerk key are rejected until you update it. Prefer the fetched default.

### Organizations and permissions

Permissions go in as plain `org:<feature>:<permission>` strings and are encoded
into Clerk's packed v2 `fea`/`per`/`fpm` claims for you — the shape the
verifier decodes back into `ClerkAuth::org_permissions`:

```rust,ignore
TestSession::new("user_2abc")
    .with_organization("org_2ghi")
    .with_organization_slug("acme")
    .with_organization_role("org:admin")
    .with_organization_permissions(["org:dashboard:read", "org:teams:manage"])
```

The role accepts `admin` or `org:admin`; both normalize to `org:admin`. A
permission not in the three-part form errors at `sign()` rather than silently
decoding to an empty list.

For the older flat `org_id` / `org_slug` / `org_role` / `org_permissions`
claims, add `.with_v1_organization_claims()`. Clerk issues v2 now; use v1 only
to cover the legacy shape deliberately.

### Testing what should be rejected

Guards are only proven by the requests they turn away:

```rust,ignore
// Past its `exp`, well beyond the configured clock skew.
TestSession::new("user_2abc").expired()

// No `sid` — the shape of a Clerk JWT-template token, rejected by default so a
// leaked template token cannot be replayed as a session.
TestSession::new("user_2abc").without_session_id()

// Fails a configured add_authorized_party / add_issuer / add_audience.
TestSession::new("user_2abc").with_authorized_party("https://evil.example.com")

// Anything else, including deliberately malformed claims.
TestSession::new("user_2abc").with_claim("sub", serde_json::Value::Null)
```

A token signed by a second `TestIssuer::generate()` covers the
foreign-signature case.

Note that `with_static_jwks` can never produce
`VerificationOutcome::Unavailable` — there is nothing to fetch, so nothing can
be unavailable, and an unknown `kid` is reported as invalid. For the
unavailable path you need the fetching path below.

### Testing the JWKS fetching path

Skip this unless you specifically care about cache, refresh, or outage
behaviour — that logic lives in this crate and is covered by its own tests. If
you do want it, serve the issuer's JWKS from a mock server:

```rust,ignore
let issuer = TestIssuer::generate().unwrap();
let server = wiremock::MockServer::start().await;
wiremock::Mock::given(wiremock::matchers::method("GET"))
    .and(wiremock::matchers::path("/v1/jwks"))
    .respond_with(
        wiremock::ResponseTemplate::new(200).set_body_string(issuer.jwks_json().unwrap()),
    )
    .mount(&server)
    .await;

let config = ClerkAuthLayerConfig::new("sk_test_unused")
    .with_insecure_backend_api_base_url(format!("{}/v1", server.uri()));
```

The JWKS endpoint is the base URL plus `/jwks`.
`with_insecure_backend_api_base_url` is what permits a plain `http://` URL; it
logs a warning, by design.

---

## Browser tests

Two modes, and they cannot share a browser context — pick per Playwright
project.

### Offline (testing your app)

This is the browser counterpart to `TestClerk`: a fake `window.Clerk` for the
client, a real minted cookie for the server.

#### Why a fake is required

`ClerkProvider` injects clerk-js (and `@clerk/ui`) from your instance's
Frontend API host. In a test environment that is a real network fetch, and
without one the provider never becomes loaded.

The escape hatch is built into the loader: **script injection is skipped
entirely when `window.Clerk` already exists.** Install a fake before the app
boots and nothing is ever fetched.

The check is specific — `window.Clerk` must exist *and* `window.Clerk.load`
must be a function. A global without a callable `load` reads as "clerk-js has
not executed yet", and the loader injects the real script anyway.

#### The surface your fake must implement

| Member | Required | Notes |
| --- | --- | --- |
| `load(opts)` | **Yes** | Must be a function returning a Promise. Its presence is how the crate detects clerk-js. |
| `loaded` | **Yes** | Boolean. The provider is not loaded until this is `true`. |
| `addListener(cb)` | **Yes** | Must return an unsubscribe **function**; returning `undefined` breaks the provider. |
| `isSignedIn` | Recommended | Boolean. Falls back to "is `user` present" when absent. |
| `user` | If signed in | Needs `id`. Optional: `firstName`, `lastName`, `imageUrl`, `primaryEmailAddress.emailAddress`. |
| `session` | If signed in | Needs **both** `id` and `status` (`"active"` or `"pending"`). Optional: `getToken()`, `lastActiveOrganizationId`, `tasks`. |
| `signOut(opts)` | If used | Returns a Promise. |
| `mount*` / `unmount*` | If used | `mountSignIn`, `mountUserButton`, … one per Clerk widget you render. |
| `open*` / `close*` | If used | `openSignIn`, `closeUserProfile`, … |
| `redirectToSignIn` / `redirectToSignUp` | If used | Returns a Promise. |

A `Proxy` covers the long tail of widget methods without enumerating them:

```js
// tests/browser/clerk-fake.js
export function installClerkFake({ signedIn = true, userId = "user_2abc" } = {}) {
  const clerk = {
    loaded: true,
    isSignedIn: signedIn,
    user: signedIn
      ? {
          id: userId,
          firstName: "Test",
          lastName: "User",
          primaryEmailAddress: { emailAddress: "test@example.com" },
        }
      : null,
    session: signedIn
      ? { id: `sess_${userId}`, status: "active", tasks: [], getToken: async () => null }
      : null,
    load: async () => {},
    // Must return a function: the provider stores it as the unsubscribe.
    addListener: () => () => {},
    signOut: async () => {},
  };

  // Auto-stub every widget method so rendering a Clerk component is a no-op
  // instead of a TypeError.
  window.Clerk = new Proxy(clerk, {
    get(target, property) {
      if (property in target) return target[property];
      if (typeof property === "string" && /^(mount|unmount|open|close|redirectTo)/.test(property)) {
        return () => Promise.resolve();
      }
      return undefined;
    },
  });
}
```

#### Sharing a key between the server and Playwright

The Rust server and the Node process need the same key. Put it on disk once and
have both sides read it:

```rust,ignore
let issuer = TestIssuer::from_pem_file_or_generate("target/clerk-test-key.pem")?;
let clerk = TestClerk::from_issuer(issuer);
let layer = clerk.layer()?;
```

`from_pem_file_or_generate` creates the key on first use and reuses it after, so
a fresh checkout needs no setup step. Concurrent first runs converge on one key
rather than each signing with its own. Gitignore the path:

```gitignore
/target/clerk-test-key.pem
```

Node cannot sign Clerk tokens without reimplementing the claim shapes, so mint
them in Rust and hand them over:

```rust,ignore
// examples/mint-test-tokens.rs
use dioxus_clerk::testing::{TestClerk, TestIssuer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let clerk = TestClerk::from_issuer(TestIssuer::from_pem_file_or_generate(
        "target/clerk-test-key.pem",
    )?);

    let long = std::time::Duration::from_secs(3600);
    let tokens = serde_json::json!({
        "admin": clerk.token_for(
            &clerk.session("user_admin")
                .with_organization("org_acme")
                .with_organization_role("org:admin")
                .with_lifetime(long),
        )?,
        "member": clerk.token_for(
            &clerk.session("user_member")
                .with_organization("org_acme")
                .with_organization_role("org:member")
                .with_lifetime(long),
        )?,
    });

    std::fs::write("target/test-tokens.json", serde_json::to_vec_pretty(&tokens)?)?;
    Ok(())
}
```

Give tokens a lifetime that outlives the suite. The default is one minute,
matching Clerk's real session tokens — long enough for a server test, not for a
browser run.

#### Playwright configuration

```js
// playwright.config.js
export default defineConfig({
  globalSetup: "./tests/browser/global-setup.js",
  use: { baseURL: "http://127.0.0.1:8080" },
  webServer: {
    command: "cargo run --features server",
    url: "http://127.0.0.1:8080",
    reuseExistingServer: !process.env.CI,
  },
});
```

```js
// tests/browser/global-setup.js
import { execFileSync } from "node:child_process";

export default function globalSetup() {
  execFileSync("cargo", ["run", "--quiet", "--example", "mint-test-tokens"], {
    stdio: "inherit",
  });
}
```

```js
import { readFileSync } from "node:fs";
import { installClerkFake } from "./clerk-fake.js";

const tokens = JSON.parse(readFileSync("target/test-tokens.json", "utf8"));

// One constant drives both halves — see the note below.
const USER = "user_test_admin";

test.beforeEach(async ({ context, baseURL }) => {
  // addInitScript runs before any page script, on every navigation — which is
  // what stops the provider from injecting real clerk-js.
  await context.addInitScript(installClerkFake, { userId: USER });

  await context.addCookies([
    { name: "__session", value: tokens[USER], url: baseURL, httpOnly: true, sameSite: "Lax" },
  ]);
});
```

Three things to get right:

- **`addInitScript` on the `context`, not the `page`.** It must run before the
  app's own scripts, on every navigation.
- **The cookie is what the server sees.** The fake only satisfies the client;
  the `__session` cookie is what the middleware verifies. They can disagree —
  which is itself worth a test.
- **Keep the two user IDs in sync.** The identity lives in two places: the `sub`
  of the minted cookie, and `user.id` in the fake. If they drift, the server
  thinks you are one user while the client renders another — a confusing failure
  that looks like a caching bug. Drive both from one constant, as above.

#### The publishable key

Your app still needs one for `ClerkProvider`, but with the fake installed the
key's Frontend API host is never contacted. Use a well-formed placeholder —
`pk_test_` followed by base64 of `<host>$` — so nothing falls down a
malformed-key path:

```text
# base64("test.clerk.accounts.dev$")
pk_test_dGVzdC5jbGVyay5hY2NvdW50cy5kZXYk
```

#### What this does not cover

The fake reports state; it does not render Clerk's UI. `mountSignIn` is a
no-op, so `<SignIn />` produces an empty container. You can test everything
*around* authentication — routing, guards, org-scoped views, signed-in and
signed-out branches — but not a user typing into Clerk's sign-in form.

### Through real Clerk

For that last case, Clerk ships
[`@clerk/testing`](https://clerk.com/docs/guides/development/testing/playwright/overview),
which drives a real development instance. There is nothing for this crate to
integrate: it is an npm package that operates on `window.Clerk` in the page, so
it composes with `dioxus-clerk` rather than plugging into it.

It composes for a specific reason worth knowing: `ClerkProvider` registers
`Clerk.addListener` and, on every event, **re-reads the full state from
`window.Clerk`** rather than trusting the event payload. A session established
by external code — which is exactly what `clerk.signIn()` does via `setActive`
— therefore propagates into the provider automatically.

```bash
npm install --save-dev @clerk/testing
```

```js
// tests/browser/clerk.setup.js
import { clerkSetup } from "@clerk/testing/playwright";

export default async function globalSetup() {
  // Fetches a Testing Token so Clerk's bot detection does not block automation.
  await clerkSetup();
}
```

```js
import { clerk, setupClerkTestingToken } from "@clerk/testing/playwright";

test("a user can sign in through Clerk's form", async ({ page }) => {
  await setupClerkTestingToken({ page });

  // Required: land on an unprotected page that mounts ClerkProvider first.
  await page.goto("/");
  await clerk.loaded({ page });

  await clerk.signIn({
    page,
    signInParams: { strategy: "password", identifier: "...", password: "..." },
  });

  await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
});
```

#### Backend calls work unchanged

`clerk.signIn()` does not fake a session — it establishes a **real** one on a
real instance, so clerk-js sets a real `__session` cookie carrying a genuine
Clerk-signed JWT. Your middleware verifies it exactly as in production: normal
`from_env()` config, real JWKS fetch, no `with_static_jwks`, nothing special to
configure. Requests from these tests are indistinguishable from real traffic.

The distinction that trips people up: **the Testing Token is not the session
token.** It is a short-lived, instance-scoped credential passed as a
`__clerk_testing_token` query parameter on *Frontend API* requests, and its only
job is [bypassing bot detection](https://clerk.com/docs/guides/development/testing/overview)
so automation is not blocked. It never reaches your backend.

#### The two modes cannot share a server

The server's configuration decides which token family verifies, and it is one or
the other per process:

| Server config | Minted tokens | Real Clerk tokens |
| --- | --- | --- |
| `with_static_jwks` | Verify | Rejected |
| `from_env()` (real keys) | Rejected | Verify |

So this is not a per-test choice — it is per server process, and therefore per
Playwright project, each pointed at a differently-configured backend. (The
browser context differs too: the offline fake exists specifically to stop real
clerk-js from loading.)

#### What this mode costs

- A live Clerk **development instance**, with `CLERK_PUBLISHABLE_KEY` and
  `CLERK_SECRET_KEY` available to the test run — including in CI. On a public
  repository, fork pull requests cannot read repository secrets, so these tests
  fail for outside contributions.
- Network round-trips on every sign-in.
- User IDs are assigned by Clerk, not chosen. If your app keys data on the user,
  resolve the ID in `globalSetup` via the Backend API and seed from it — or use
  the offline mode, where you [pick the ID](#choosing-user-ids).
- Real users in a real instance means shared, mutable state: seeded accounts,
  isolation between parallel tests, and cleanup.
- `signInParams` [handles first-factor verification only](https://clerk.com/docs/guides/development/testing/playwright/test-helpers);
  MFA needs the `emailAddress` strategy, which uses the secret key server-side.
- Concurrency has been a rough edge —
  [clerk/javascript#7891](https://github.com/clerk/javascript/issues/7891)
  reports `signIn()` timing out under `--workers=2+`, with `--workers=1` as the
  workaround. That issue is closed with a related fix, so verify against your
  own suite rather than assuming either way.
- Against a *production* instance, code-based auth (OTP) does not work with
  Testing Tokens; only email/password or direct email sign-in.

Keep this mode to the few tests that genuinely exercise Clerk's UI, and use the
offline mode for everything else.

---

## Troubleshooting

| Symptom | Cause |
| --- | --- |
| Test hangs; network request to `*.clerk.accounts.dev` | The fake was installed too late, or `Clerk.load` is not a function. Both make the loader inject the real script. Use `context.addInitScript`. |
| Provider never becomes loaded | `loaded` is not `true`, or `addListener` returned something other than a function. |
| Client shows signed out, server sees a valid user | The `__session` cookie is set but the `window.Clerk` fake is signed out or missing. They are independent. |
| Client and server disagree about *who* the user is | The fake's `user.id` and the token's `sub` have drifted. Drive both from one constant. |
| Real Clerk tokens rejected, or minted tokens rejected | A server process trusts one family, not both. Check whether it was built with `with_static_jwks` or real credentials. |
| Every request is 401 after a while | Token expired. The default lifetime is 60 seconds; set `with_lifetime` for browser runs. |
| `VerificationOutcome::Unavailable` / HTTP 503 | The layer is still fetching a JWKS. `with_static_jwks` was not applied, or the config was rebuilt from the environment somewhere. |
| Token rejected with no obvious reason | A `sid` is required by default. `without_session_id()` and real JWT-template tokens are rejected unless `allow_non_session_tokens()` is set. |
| `org_permissions` comes back empty | Permissions must be `org:<feature>:<permission>`. Malformed ones error at `sign()`; if you hand-built claims, check the `fea`/`per`/`fpm` encoding. |
| Key regenerated on every run | The path is inside a cleaned directory, or CI does not persist it. Fine in itself — just make sure the server and the minting step use the same one. |

## See also

- [`TestClerk` API docs](https://docs.rs/dioxus-clerk/latest/dioxus_clerk/testing/struct.TestClerk.html)
- [`TestIssuer` API docs](https://docs.rs/dioxus-clerk/latest/dioxus_clerk/testing/struct.TestIssuer.html)
- [`with_static_jwks`](https://docs.rs/dioxus-clerk/latest/dioxus_clerk/server/struct.ClerkAuthLayerConfig.html#method.with_static_jwks)
- [Testing with Playwright — Clerk docs](https://clerk.com/docs/guides/development/testing/playwright/overview)
