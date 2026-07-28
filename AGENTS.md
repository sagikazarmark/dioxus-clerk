# Working on dioxus-clerk

Clerk authentication for Dioxus 0.7. One crate, two build targets: a browser
(wasm) client that drives clerk-js, and a native server half with Axum
middleware that verifies Clerk session tokens.

Read [CONTEXT.md](CONTEXT.md) first — it defines the domain vocabulary (Clerk
lifecycle, action dispatch, auth state, SSR seed, JS bridge layer, verification
outcome). Use those terms; the code and comments already do.

## The target split is the main constraint

Most mistakes here come from forgetting that code compiles for two very
different targets.

- `src/components`, `src/hooks`, `src/core` — both targets.
- `src/bindings.rs`, `src/bridge.rs`, `src/handle.rs`, `src/lifecycle.rs`,
  `src/loader.rs` — browser only, gated on `cfg(clerk_client)` (emitted by
  `build.rs`: wasm32 *without* the `worker` feature).
- `src/server/` — native only, behind the `server` feature.
- `src/testing.rs` — behind the `testing` feature, independent of `server` so a
  harness that only mints tokens does not pull in axum and reqwest.

A change that compiles for one target routinely breaks the other. Check both
before claiming done.

## Checks

```bash
cargo fmt
cargo clippy --all-features --all-targets   # must be warning-free
cargo test --all-features                   # native + integration tests
cargo build --target wasm32-unknown-unknown # the browser half
cargo doc --no-deps --features server,testing
wasm-pack test --headless --chrome          # browser tests (slow)
```

`dagger check` runs all of it in pinned containers and is exactly what CI runs.
Prefer it before opening a PR; prefer the individual commands while iterating.

Some tests only exist under a feature or target: `tests/*.rs` files start with a
`#![cfg(...)]` gate, so a bare `cargo test` silently runs almost none of the
server or testing coverage. Use `--all-features`.

## Conventions the code already follows

Match these; they are visible in any file you open.

- **Comments explain *why*, not *what*.** The non-obvious reasoning — why a
  redirect is refused, why the cache is locked through poison recovery, why a
  claim is rejected — belongs next to the code. Do not narrate mechanics.
- **Security reasoning lives at the decision point.** See
  `src/server/config.rs` (claim acceptance) and `src/server/verification.rs`
  (redirects, origin checks, response size limits) for the register.
- **No key material in the repository.** The `testing` feature generates keys at
  runtime; nothing is committed, and nothing should be.
- **Public API is curated.** `src/lib.rs` re-exports an explicit list rather than
  glob-re-exporting modules, so a new `pub` item does not silently land at the
  crate root. Add to the list deliberately.
- **`#![warn(missing_docs)]` and `#![forbid(unsafe_code)]` are on.** Every public
  item needs a doc comment.
- **`#[non_exhaustive]` on public enums and structs** that mirror Clerk concepts,
  so new Clerk fields are not breaking changes.

## Tests

Integration tests drive the real code path rather than asserting on
intermediate representations — `tests/testing_issuer.rs` runs minted tokens
through an actual `ClerkAuthLayer` instead of checking claim JSON, so the
helpers stay honest when the verifier moves.

Browser tests (`tests/wasm_*.rs`) install a fake `window.Clerk` and must be
order-independent; page-scoped state is reset through
`dioxus_clerk::__reset_load_state()`.

## Docs

Markdown in `docs/` is the source of truth for long-form guides. Each is also
included into rustdoc via `src/lib.rs`'s `guides` module, so it ships versioned
on docs.rs. Two consequences when editing them:

- Use absolute `https://` links, not relative paths to other Markdown files —
  relative links break in the rustdoc rendering.
- Non-Rust fences need a language tag (`js`, `toml`, `bash`); untagged fences
  are treated as Rust and compiled as doctests.

`README.md` is for people arriving from GitHub or crates.io; keep it a summary
that points at the guides rather than a second copy of them.

## Publishing

`Cargo.toml` has no `include`/`exclude`, so **everything git-tracked ships** —
CI workflows, devenv and dagger config, and these agent instructions included.
Check what a release would carry with:

```bash
cargo package --list          # what would ship
cargo package                 # also runs a verification build
```

Narrowing this to an allowlist is planned (see Known gaps). The working recipe,
for whenever it lands:

```toml
exclude = ["*", "!src", "!src/**", "!docs", "!docs/**", "!build.rs",
           "!CONTEXT.md", "!LICENSE-APACHE", "!LICENSE-MIT", "!README.md"]
```

Two traps, both verified the hard way:

- A directory needs *two* entries (`!docs` and `!docs/**`). Gitignore rules
  cannot re-include a file whose parent is still excluded, so `!docs` alone
  re-admits the directory entry and none of its contents — `cargo package`
  succeeds and publishes a crate with no source.
- Do not spell it as `include = [...]`. It produces an identical package, but
  makes cargo enumerate the filesystem instead of asking git for the file list,
  which trips over symlink loops in devenv's Nix profile and prints a dozen
  warnings on every `cargo check`.

`docs/` must stay in whatever list is used: `src/lib.rs`'s `guides` module
`include_str!`s it, so docs.rs fails to build the crate without it.

## Known gaps

- `README.md` links to `docs/react-migration.md`, which has never existed.
- The published package carries contributor tooling and these instructions;
  see Publishing for the allowlist that would fix it.
