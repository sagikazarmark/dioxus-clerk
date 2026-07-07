//! Drop-in components.

mod button_host;
mod button_mode;
mod control;
mod create_organization;
mod organization_list;
mod organization_profile;
mod organization_switcher;
mod protect;
mod provider;
mod redirect;
mod sign_in;
mod sign_in_button;
mod sign_out_button;
mod sign_up;
mod sign_up_button;
mod signed_in;
mod signed_out;
mod task_setup_mfa;
mod user_avatar;
mod user_button;
mod user_profile;
mod waitlist;
// pub(crate): the JS bridge layer's widget mount/unmount surface takes this
// module's `Widget` enum, so the widget vocabulary is defined exactly once.
pub(crate) mod widget;

pub use button_mode::AuthButtonMode;
pub use control::{ClerkFailed, ClerkLoaded, ClerkLoading};
pub use create_organization::CreateOrganization;
pub use organization_list::OrganizationList;
pub use organization_profile::OrganizationProfile;
pub use organization_switcher::OrganizationSwitcher;
pub use protect::Protect;
pub use provider::ClerkProvider;
pub use redirect::{RedirectToSignIn, RedirectToSignUp};
pub use sign_in::SignIn;
pub use sign_in_button::SignInButton;
pub use sign_out_button::SignOutButton;
pub use sign_up::SignUp;
pub use sign_up_button::SignUpButton;
pub use signed_in::{SignedIn, SignedInWhenLoaded};
pub use signed_out::{SignedOut, SignedOutWhenLoaded};
pub use task_setup_mfa::TaskSetupMFA;
pub use user_avatar::UserAvatar;
pub use user_button::UserButton;
pub use user_profile::UserProfile;
pub use waitlist::Waitlist;
