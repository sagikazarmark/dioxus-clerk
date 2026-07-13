//! Privacy policy and terms of service. These are intentionally minimal:
//! enough to satisfy the "legal links" requirement of some OAuth / provider
//! verifications for what is only a demo application.

use dioxus::prelude::*;

use crate::components::PageHeader;

/// Shared prose wrapper: a readable measure with sensible spacing between the
/// headings and paragraphs the two legal pages emit.
#[component]
fn LegalBody(children: Element) -> Element {
    rsx! {
        section { class: "mt-8 max-w-[70ch] space-y-4 text-base leading-7 text-base-content/75 [&_h2]:mt-8 [&_h2]:text-xl [&_h2]:font-semibold [&_h2]:text-base-content [&_a]:link [&_a]:link-primary",
            {children}
        }
    }
}

#[component]
pub fn PrivacyPolicy() -> Element {
    rsx! {
        PageHeader {
            eyebrow: "Legal",
            title: "Privacy policy",
            intro: "This is a demo application for the dioxus-clerk library. It exists to showcase authentication components and is not a commercial service.",
        }
        LegalBody {
            p { "Last updated: July 9, 2026." }

            h2 { "What we collect" }
            p {
                "Signing in is optional and handled entirely by "
                a { href: "https://clerk.com", target: "_blank", rel: "noopener noreferrer", "Clerk" }
                ", our authentication provider. If you choose to sign in, Clerk stores the account details you provide (such as your email address and any profile information) on its infrastructure. This demo does not run its own database and does not collect analytics."
            }

            h2 { "How it's used" }
            p { "Account data is used only to demonstrate authenticated sessions within this demo. We do not sell it, share it for advertising, or use it for any purpose beyond running the demo." }

            h2 { "Third parties" }
            p {
                "Authentication and the associated data are governed by Clerk's own "
                a { href: "https://clerk.com/legal/privacy", target: "_blank", rel: "noopener noreferrer", "privacy policy" }
                ". Please review it to understand how Clerk processes your information."
            }

            h2 { "Deleting your data" }
            p { "You can delete your account at any time from the profile page. Removing your account removes the associated data held by Clerk for this demo." }

            h2 { "Contact" }
            p {
                "Questions? Reach out at "
                a { href: "mailto:dioxus-clerk-demo@sagikazarmark.com", "dioxus-clerk-demo@sagikazarmark.com" }
                "."
            }
        }
    }
}

#[component]
pub fn TermsOfService() -> Element {
    rsx! {
        PageHeader {
            eyebrow: "Legal",
            title: "Terms of service",
            intro: "By using this demo application you agree to the terms below. This is a demonstration of the dioxus-clerk library, provided for illustrative purposes only.",
        }
        LegalBody {
            p { "Last updated: July 9, 2026." }

            h2 { "Use of the demo" }
            p { "This application is provided free of charge to demonstrate authentication features. You may explore it freely, but you agree not to misuse it, disrupt its operation, or attempt to access data that is not yours." }

            h2 { "No warranty" }
            p { "The demo is provided \"as is\", without warranties of any kind. It may change, break, or go offline at any time without notice. Do not rely on it for anything important, and do not store sensitive information in it." }

            h2 { "Accounts" }
            p {
                "Authentication is handled by "
                a { href: "https://clerk.com", target: "_blank", rel: "noopener noreferrer", "Clerk" }
                " and is also subject to Clerk's "
                a { href: "https://clerk.com/legal/terms", target: "_blank", rel: "noopener noreferrer", "terms of service" }
                ". You are responsible for keeping your login credentials secure."
            }

            h2 { "Limitation of liability" }
            p { "To the fullest extent permitted by law, the author is not liable for any damages arising from your use of this demo." }

            h2 { "Contact" }
            p {
                "Questions? Reach out at "
                a { href: "mailto:dioxus-clerk-demo@sagikazarmark.com", "dioxus-clerk-demo@sagikazarmark.com" }
                "."
            }
        }
    }
}
