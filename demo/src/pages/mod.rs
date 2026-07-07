//! Route components. Each page frames one or more `examples` components with
//! prose, a Clerk docs link, and the example's own source (via `code!`).
//!
//! Pages are grouped into files by nav section; `app.rs`'s `Route` enum wires
//! them up by name (`use crate::pages::*`).

mod advanced;
mod basics;
mod orgs;
mod server;
mod widgets;

pub use advanced::{Errors, Gating, Hooks, Imperative, Reverification, SessionTasks};
pub use basics::{Buttons, Home, Minimal};
pub use orgs::{Organizations, WaitlistPage};
pub use server::ServerDemo;
pub use widgets::{ProfilePage, SignInCallbackPage, SignInPage, SignUpCallbackPage, SignUpPage};
