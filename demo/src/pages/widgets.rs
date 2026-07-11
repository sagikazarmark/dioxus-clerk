use dioxus::prelude::*;
use dioxus_code::{Code, code};

use crate::examples::embedded_signin::EmbeddedSignInExample;
use crate::examples::embedded_signup::EmbeddedSignUpExample;
use crate::examples::profile_avatar::ProfileAvatarExample;
use crate::examples::profile_button::ProfileButtonExample;
use crate::examples::profile_embedded::ProfileEmbeddedExample;
use crate::ui::{DocLink, ExampleSection, InlineCode, PageHeader, snippet_theme};

#[component]
pub fn SignInPage() -> Element {
    rsx! {
        PageHeader {
            eyebrow: "Components",
            title: "Embedded sign-in",
            intro: "Clerk owns the form; Dioxus owns the route. The provider's router callbacks keep Clerk's navigation inside the SPA.",
        }
        ExampleSection {
            title: "<SignIn> with path routing",
            stacked: true,
            intro: rsx! {
                InlineCode { "Routing::Path" }
                " keeps Clerk's sub-steps under "
                InlineCode { "/sign-in" }
                ", so the router also accepts "
                InlineCode { "/sign-in/:..segments" }
                " for SSO and email callbacks."
            },
            demo: rsx! { EmbeddedSignInExample {} },
            code: rsx! { Code { src: code!("src/examples/embedded_signin.rs"), theme: snippet_theme() } },
        }
        p { class: "mt-6 text-sm text-base-content/60",
            "Clerk docs: "
            DocLink { href: "https://clerk.com/docs/components/authentication/sign-in", "<SignIn /> component" }
            "."
        }
    }
}

#[component]
pub fn SignInCallbackPage(segments: Vec<String>) -> Element {
    let _ = segments;
    rsx! { SignInPage {} }
}

#[component]
pub fn SignUpPage() -> Element {
    rsx! {
        PageHeader {
            eyebrow: "Components",
            title: "Embedded sign-up",
            intro: "The sign-up form shares the same route-aware provider setup as sign-in, including the fallback redirect to a protected page.",
        }
        ExampleSection {
            title: "<SignUp> with path routing",
            stacked: true,
            intro: rsx! {
                "Like "
                InlineCode { "<SignIn>" }
                ", path routing needs a "
                InlineCode { "/sign-up/:..segments" }
                " catch-all so Clerk's verification sub-steps resolve inside the app."
            },
            demo: rsx! { EmbeddedSignUpExample {} },
            code: rsx! { Code { src: code!("src/examples/embedded_signup.rs"), theme: snippet_theme() } },
        }
        p { class: "mt-6 text-sm text-base-content/60",
            "Clerk docs: "
            DocLink { href: "https://clerk.com/docs/components/authentication/sign-up", "<SignUp /> component" }
            "."
        }
    }
}

#[component]
pub fn SignUpCallbackPage(segments: Vec<String>) -> Element {
    let _ = segments;
    rsx! { SignUpPage {} }
}

#[component]
pub fn ProfilePage() -> Element {
    rsx! {
        PageHeader {
            eyebrow: "Components",
            title: "Profile & avatar",
            intro: "Hosted account UI for a signed-in user, from smallest to largest: a plain avatar image, the account menu, and the full inline profile manager.",
        }
        ExampleSection {
            title: "Avatar only",
            intro: rsx! {
                InlineCode { "UserAvatar" }
                " renders just the current user's image as a plain "
                InlineCode { "<img>" }
                ", with a "
                InlineCode { "fallback" }
                " while it loads, ideal for a custom header or nav bar."
            },
            demo: rsx! { ProfileAvatarExample {} },
            code: rsx! { Code { src: code!("src/examples/profile_avatar.rs"), theme: snippet_theme() } },
        }
        ExampleSection {
            title: "Account menu",
            intro: rsx! {
                InlineCode { "UserButton" }
                " is the hosted account dropdown. "
                InlineCode { "user_profile_mode" }
                " picks whether \"Manage account\" opens the profile in a modal or navigates to a page."
            },
            demo: rsx! { ProfileButtonExample {} },
            code: rsx! { Code { src: code!("src/examples/profile_button.rs"), theme: snippet_theme() } },
        }
        ExampleSection {
            title: "Embedded profile",
            stacked: true,
            intro: rsx! {
                InlineCode { "UserProfile" }
                " mounts the full account-management UI inline, the same surface the "
                InlineCode { "UserButton" }
                " modal shows, for a dedicated account route rather than a popup."
            },
            demo: rsx! { ProfileEmbeddedExample {} },
            code: rsx! { Code { src: code!("src/examples/profile_embedded.rs"), theme: snippet_theme() } },
        }
        p { class: "mt-6 text-sm text-base-content/60",
            "Clerk docs: "
            DocLink { href: "https://clerk.com/docs/components/user/user-button", "<UserButton />" }
            " and "
            DocLink { href: "https://clerk.com/docs/components/user/user-profile", "<UserProfile />" }
            "."
        }
    }
}
