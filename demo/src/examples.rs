//! Small, focused example components: one per feature area.
//!
//! Each component keeps the `dioxus_clerk` API front and center; purely
//! presentational bits (spinners, status lines, state readouts) are delegated
//! to shared helpers in [`crate::components`] so the source reads as a tight snippet of
//! the library being demonstrated rather than layout markup. The `pages` module
//! both mounts these live *and* renders their source with the compile-time
//! `code!` macro, guaranteeing the code shown is the code that runs.

pub mod buttons;
pub mod embedded_signin;
pub mod embedded_signup;
pub mod errors;
pub mod gating;
pub mod hooks;
pub mod imperative;
pub mod minimal;
pub mod org_create;
pub mod org_list;
pub mod org_profile;
pub mod org_protect;
pub mod org_switcher;
pub mod profile_avatar;
pub mod profile_button;
pub mod profile_embedded;
pub mod reverification;
pub mod server_call;
pub mod session_tasks;
pub mod waitlist;
