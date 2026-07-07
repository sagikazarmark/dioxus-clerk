//! Small, self-contained example components — one per feature area.
//!
//! Each component is deliberately minimal and free of demo chrome: it uses
//! only `dioxus` and `dioxus_clerk`, so it reads well as a copy-pasteable
//! snippet. The `pages` module both mounts these live *and* renders their
//! source with the compile-time `code!` macro, guaranteeing the code shown is
//! the code that runs.

pub mod buttons;
pub mod embedded_signin;
pub mod embedded_signup;
pub mod errors;
pub mod gating;
pub mod hooks;
pub mod imperative;
pub mod minimal;
pub mod organizations;
pub mod profile;
pub mod reverification;
pub mod server_call;
pub mod session_tasks;
pub mod waitlist;
