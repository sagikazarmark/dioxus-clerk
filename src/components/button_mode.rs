/// Behavior used by sign-in and sign-up button components.
///
/// Unlike [`Routing`](crate::Routing) and [`UserProfileMode`](crate::UserProfileMode),
/// which mirror clerk-js's fixed option sets, this is a crate-owned concept
/// that may gain variants (e.g. a future non-modal, non-redirect flow). It is
/// therefore `#[non_exhaustive]`: match it with a wildcard arm.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum AuthButtonMode {
    /// Open Clerk's modal flow on the current page.
    Modal,
    /// Redirect through Clerk's configured sign-in/sign-up URL.
    #[default]
    Redirect,
}
