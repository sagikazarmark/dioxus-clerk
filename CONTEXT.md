# Project context

## Domain terms

### Clerk lifecycle

The browser-side lifecycle for loading clerk-js, discovering `window.Clerk`,
calling `Clerk.load`, registering Clerk listeners, surfacing readiness/errors,
and coordinating Clerk-mounted UI.

### Clerk action dispatch

The coordination of application-requested clerk-js browser actions (opening
and closing Clerk UI, signing out, redirecting) with the Clerk lifecycle. An
action never touches clerk-js before the lifecycle reports loaded.
Fire-and-forget actions run in request order and surface failures through the
provider's error reporting; awaited actions report their outcome directly to
the caller. When the lifecycle fails to load, pending fire-and-forget actions
are discarded because the load failure is already surfaced.

### Auth state

The application-visible authentication knowledge at a point in time. Auth state
can be unknown/loading, signed out, signed in from a verified server snapshot,
or signed in with full clerk-js `User` and `Session` details. It includes
whether clerk-js has finished loading; callers should not need to combine a
separate loadedness fact with auth knowledge. Server-rendered auth facts do not
make browser loadedness true by themselves.

### SSR seed

The JSON payload emitted into the server-rendered document and consumed during
wasm startup. It carries server-verified auth facts and, when provided, a
publishable key for clerk-js initialization. It does not carry browser
loadedness; Auth state derives browser loadedness during clerk-js startup.
Each platform startup path reads the seed into one seed-read value
(missing, present, or malformed); a single consumer interprets that value
into provider startup facts.

### Mounted Clerk UI

Clerk prebuilt UI rendered by mounting clerk-js widgets into stable Dioxus DOM
nodes, such as `SignIn`, `SignUp`, `UserButton`, and `UserProfile`.

### Clerk widget

An individual clerk-js prebuilt UI widget, such as sign-in, sign-up, user
button, or user profile. Internally, shared widget hosting code coordinates the
Dioxus host node, Clerk loadedness, mount-once behavior, and cleanup for these
widgets.

### JS bridge layer

The boundary that reads browser-global clerk-js values and converts JavaScript
values, promises, callbacks, users, and sessions into library-level handles and
auth data. It executes Clerk action dispatch operations against clerk-js; it
does not include provider orchestration, dispatch ordering, mounted UI timing,
or route/component control flow.

### Clerk options mapping

The single surface translating Rust-side option names into clerk-js JSON
option keys. Typed option builders are the one implementation of the mapping;
flat Clerk widget props delegate to the builders, and explicit props win over
raw options passed alongside them. Each clerk-js key is stated exactly once.

### Server verification outcome

The server-side result of checking request credentials before handlers or
server functions run: missing credentials, valid credentials, invalid
credentials, or temporarily unavailable verification infrastructure.
