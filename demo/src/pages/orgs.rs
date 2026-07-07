use dioxus::prelude::*;
use dioxus_code::{code, Code};

use crate::examples::organizations::OrganizationsExample;
use crate::examples::waitlist::WaitlistExample;
use crate::ui::{snippet_theme, DocLink, ExampleSection, PageHeader, SetupCallout};

#[component]
pub fn Organizations() -> Element {
    rsx! {
        PageHeader {
            eyebrow: "Components",
            title: "Organizations",
            intro: "Clerk's organization widgets plus role-gated rendering. Without an active organization the widgets show their empty/create states.",
        }
        SetupCallout {
            title: "Requires Organizations enabled",
            dashboard_label: "Open Clerk Dashboard",
            dashboard_url: "https://dashboard.clerk.com",
            docs_label: "Organizations docs",
            docs_url: "https://clerk.com/docs/organizations/overview",
            "Enable Organizations under Configure → Organizations, then create an org and assign roles. "
            "The Protect example below looks for the "
            code { class: "rounded bg-base-200 px-1 py-0.5 text-xs", "org:admin" }
            " role — configure roles under "
            DocLink { href: "https://clerk.com/docs/organizations/roles-permissions", "Roles & permissions" }
            "."
        }
        ExampleSection {
            title: "Switcher, list, create, and Protect",
            intro: "OrganizationSwitcher, OrganizationList, CreateOrganization, and OrganizationProfile are hosted widgets. Protect renders its children only for the matching org role — fail-closed, so always enforce on the server too.",
            demo: rsx! { OrganizationsExample {} },
            code: rsx! { Code { src: code!("src/examples/organizations.rs"), theme: snippet_theme() } },
        }
    }
}

#[component]
pub fn WaitlistPage() -> Element {
    rsx! {
        PageHeader {
            eyebrow: "Components",
            title: "Waitlist",
            intro: "Clerk's waitlist form collects sign-ups before you open registration.",
        }
        SetupCallout {
            title: "Requires waitlist sign-up mode",
            dashboard_label: "Open Clerk Dashboard",
            dashboard_url: "https://dashboard.clerk.com",
            docs_label: "Waitlist docs",
            docs_url: "https://clerk.com/docs/components/waitlist",
            "The widget mounts regardless, but joining only works when the instance's sign-up mode is set to "
            b { "Waitlist" }
            " (Configure → Restrictions). Note this "
            b { "disables normal sign-up" }
            " on the same Clerk instance, so it conflicts with the embedded sign-up demo — use a separate instance to try it live."
        }
        ExampleSection {
            title: "<Waitlist>",
            intro: "A single hosted component. after_join_waitlist_url (and other options) are available as props.",
            demo: rsx! { WaitlistExample {} },
            code: rsx! { Code { src: code!("src/examples/waitlist.rs"), theme: snippet_theme() } },
        }
    }
}
