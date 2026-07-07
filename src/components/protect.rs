use crate::context::use_clerk_context;
use dioxus::prelude::*;

/// Renders `children` when Auth state is signed in. `fallback` is rendered
/// otherwise (or nothing if absent). A plain `Protect` with no role or
/// permission prop renders for signed-in Auth state.
///
/// The `role` and `permission` props are rendering gates backed by
/// server-verified org claims when those claims are present. Role and
/// permission gates fail closed when claims are absent or do not match.
/// Server-side authorization is still required for security-sensitive work.
///
/// Matching Clerk React, `permission` takes precedence when both props are
/// set; the `role` prop is ignored in that case.
///
/// # Behavior while auth is loading
///
/// `Protect` fails closed: it renders `fallback` for every non-signed-in Auth
/// state, including the *loading* state before clerk-js resolves. In a
/// fullstack app that did not seed a signed-in SSR snapshot, an authorized user
/// therefore sees `fallback` briefly, then `children` once auth resolves. This
/// is the safe default (never flash gated content at a not-yet-authorized
/// user), but it can flash the fallback. If you need to distinguish "still
/// loading" from "denied" — e.g. to show a spinner instead of the fallback —
/// read [`use_auth`](crate::use_auth) directly and branch on
/// [`is_loading`](crate::UseAuth::is_loading).
///
/// # Example
///
/// ```no_run
/// use dioxus::prelude::*;
/// use dioxus_clerk::*;
///
/// #[component]
/// fn AdminLink() -> Element {
///     rsx! {
///         Protect {
///             role: "admin",
///             fallback: rsx! {},
///             a { href: "/admin", "Admin" }
///         }
///     }
/// }
/// ```
#[component]
pub fn Protect(
    /// Org role (e.g. `"admin"`) the signed-in user must hold, verified
    /// against server-verified org claims. Ignored when `permission` is set.
    #[props(into)]
    role: Option<String>,
    /// Org permission (e.g. `"org:read"`) the signed-in user must hold,
    /// verified against server-verified org claims. Takes precedence over
    /// `role`, matching Clerk React.
    #[props(into)]
    permission: Option<String>,
    /// Rendered when the requirement is not met.
    #[props(default = rsx! {})]
    fallback: Element,
    /// Whether a session with pending after-auth tasks is treated as signed
    /// out. clerk-js's default is `true`; set `false` to evaluate the gate for
    /// a pending session. Mirrors Clerk React's `treatPendingAsSignedOut`.
    /// Role/permission gates still fail closed for a pending session, which
    /// carries no server-verified org claims.
    #[props(default = true)]
    treat_pending_as_signed_out: bool,
    children: Element,
) -> Element {
    let ctx = use_clerk_context();
    let state = ctx.auth.read();
    let allowed = state
        .resolve_pending(treat_pending_as_signed_out)
        .allows_signed_in_gate(role.as_deref(), permission.as_deref());
    if allowed {
        rsx! { {children} }
    } else {
        fallback
    }
}
