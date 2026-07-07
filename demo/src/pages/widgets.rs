use dioxus::prelude::*;
use dioxus_code::{code, Code};

use crate::examples::embedded_signin::EmbeddedSignInExample;
use crate::examples::embedded_signup::EmbeddedSignUpExample;
use crate::examples::profile::ProfileExample;
use crate::ui::{snippet_theme, DocLink, ExampleSection, PageHeader};

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
            intro: "Routing::Path keeps Clerk's sub-steps under /sign-in, so the router also accepts /sign-in/:..segments for SSO and email callbacks.",
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
            intro: "Like <SignIn>, path routing needs a /sign-up/:..segments catch-all so Clerk's verification sub-steps resolve inside the app.",
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
            intro: "Hosted account UI for a signed-in user: a lightweight avatar image, the account menu, and the full inline profile manager.",
        }
        ExampleSection {
            title: "UserAvatar, UserButton, and UserProfile",
            intro: "UserAvatar is a plain <img> of the user's image. UserButton is the account menu (user_profile_mode picks modal vs. navigation). UserProfile mounts the full account UI inline.",
            demo: rsx! { ProfileExample {} },
            code: rsx! { Code { src: code!("src/examples/profile.rs"), theme: snippet_theme() } },
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
