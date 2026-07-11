use dioxus::prelude::*;
use dioxus_code::{Code, code};

use crate::examples::org_create::OrgCreateExample;
use crate::examples::org_list::OrgListExample;
use crate::examples::org_profile::OrgProfileExample;
use crate::examples::org_protect::OrgProtectExample;
use crate::examples::org_switcher::OrgSwitcherExample;
use crate::examples::waitlist::WaitlistExample;
use crate::ui::{DocLink, ExampleSection, InlineCode, PageHeader, SetupCallout, snippet_theme};

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
            InlineCode { "org:admin" }
            " role, configure roles under "
            DocLink { href: "https://clerk.com/docs/organizations/roles-permissions", "Roles & permissions" }
            "."
        }
        ExampleSection {
            title: "Active-org switcher",
            intro: rsx! {
                InlineCode { "OrganizationSwitcher" }
                " shows the active organization and lets the user switch, create, or manage one from a compact dropdown, ideal for a header."
            },
            demo: rsx! { OrgSwitcherExample {} },
            code: rsx! { Code { src: code!("src/examples/org_switcher.rs"), theme: snippet_theme() } },
        }
        ExampleSection {
            title: "Organization list",
            stacked: true,
            intro: rsx! {
                InlineCode { "OrganizationList" }
                " is the embedded pick-or-create surface: every org the user belongs to, plus a create action, as a full-width view rather than a dropdown."
            },
            demo: rsx! { OrgListExample {} },
            code: rsx! { Code { src: code!("src/examples/org_list.rs"), theme: snippet_theme() } },
        }
        ExampleSection {
            title: "Create organization",
            stacked: true,
            intro: rsx! {
                InlineCode { "CreateOrganization" }
                " is the standalone create-an-org form, the same step the switcher and list expose, mounted on its own route."
            },
            demo: rsx! { OrgCreateExample {} },
            code: rsx! { Code { src: code!("src/examples/org_create.rs"), theme: snippet_theme() } },
        }
        ExampleSection {
            title: "Organization profile",
            stacked: true,
            intro: rsx! {
                InlineCode { "OrganizationProfile" }
                " mounts the full management UI for the active organization, members, invitations, roles, and settings, inline. It needs an active org to show anything."
            },
            demo: rsx! { OrgProfileExample {} },
            code: rsx! { Code { src: code!("src/examples/org_profile.rs"), theme: snippet_theme() } },
        }
        ExampleSection {
            title: "Role-gated rendering",
            intro: rsx! {
                InlineCode { "Protect" }
                " renders its children only for a matching org role or permission, checked against server-verified claims. It is fail-closed, so always enforce the same rule on the server too."
            },
            demo: rsx! { OrgProtectExample {} },
            code: rsx! { Code { src: code!("src/examples/org_protect.rs"), theme: snippet_theme() } },
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
            " on the same Clerk instance, so it conflicts with the embedded sign-up demo; use a separate instance to try it live."
        }
        ExampleSection {
            title: "<Waitlist>",
            stacked: true,
            intro: rsx! {
                "A single hosted component. "
                InlineCode { "after_join_waitlist_url" }
                " (and other options) are available as props."
            },
            demo: rsx! { WaitlistExample {} },
            code: rsx! { Code { src: code!("src/examples/waitlist.rs"), theme: snippet_theme() } },
        }
    }
}
