use dioxus::prelude::*;
use dioxus_clerk::{
    AuthButtonMode, AuthRequirement, AuthStatus, ClerkError, ClerkFailed, ClerkLoaded,
    ClerkLoading, ClerkOptions, ClerkProvider, CreateOrganization, CreateOrganizationOptions,
    GetTokenOptions, OrganizationList, OrganizationListOptions, OrganizationProfile,
    OrganizationProfileOptions, OrganizationSwitcher, OrganizationSwitcherOptions, Protect,
    RedirectOptions, RedirectToSignIn, RedirectToSignUp, ReverificationLevel, Routing, Session,
    SignIn, SignInButton, SignInOptions, SignOutButton, SignOutOptions, SignUp, SignUpButton,
    SignUpOptions, SignedIn, SignedOut, SignedOutWhenLoaded, User, UserAvatar, UserButton,
    UserButtonOptions, UserProfile, UserProfileMode, UserProfileOptions, Waitlist, WaitlistOptions,
    use_auth, use_clear_clerk_error, use_clerk, use_clerk_error, use_reverification, use_session,
    use_user,
};

#[test]
fn crate_root_exposes_normal_app_facing_surface() {
    let mut dom = VirtualDom::new(AppFacingSurface);

    dom.rebuild_in_place();
}

#[test]
fn readme_quickstart_snippet_compiles() {
    let mut dom = VirtualDom::new(ReadmeQuickstartSnippet);

    dom.rebuild_in_place();
}

#[test]
fn option_builder_component_options_compile() {
    let mut dom = VirtualDom::new(OptionBuilderSurface);

    dom.rebuild_in_place();
}

#[component]
fn AppFacingSurface() -> Element {
    rsx! {
        ClerkProvider {
            publishable_key: "pk_test_public",
            sign_in_url: "/sign-in",
            sign_up_url: "/sign-up",
            waitlist_url: "/waitlist",
            user_profile_url: "/user",
            sign_in_fallback_redirect_url: "/app",
            sign_up_force_redirect_url: "/onboarding",
            prefetch_ui: false,
            allowed_redirect_origins: vec!["https://example.com".to_string()],
            options: ClerkOptions::new()
                .sign_in_url("/raw-sign-in")
                .after_switch_session_url("/switched")
                .prefetch_ui(false),
            HookProbe {}
            RedirectToSignIn {}
            RedirectToSignUp { fallback_redirect_url: "/" }
            ClerkLoading { span { "loading" } }
            ClerkLoaded { span { "loaded" } }
            ClerkFailed { span { "failed" } }
            SignedIn {
                UserButton {
                    id: "user-button-host",
                    class: "widget-host",
                    show_name: true,
                    default_open: false,
                    after_switch_session_url: "/switched",
                    sign_in_url: "/sign-in",
                    user_profile_mode: UserProfileMode::Modal,
                    fallback: rsx! { span { "loading user button" } },
                }
                UserAvatar {
                    class: "avatar",
                    fallback: rsx! { span { "avatar fallback" } },
                }
                UserProfile {
                    id: "user-profile-host",
                    class: "widget-host",
                    routing: Routing::Hash,
                    fallback: rsx! { span { "loading user profile" } },
                }
                OrganizationSwitcher { id: "organization-switcher-host", class: "widget-host" }
                OrganizationProfile {
                    id: "organization-profile-host",
                    class: "widget-host",
                    routing: Routing::Hash,
                }
                SignOutButton {
                    class: "btn",
                    redirect_url: "/",
                }
            }
            SignedOut {
                SignIn {
                    id: "sign-in-host",
                    class: "widget-host",
                    routing: Routing::Hash,
                    sign_up_url: "/sign-up",
                    waitlist_url: "/waitlist",
                    transferable: true,
                    fallback: rsx! { span { "loading sign in" } },
                }
                SignUp {
                    id: "sign-up-host",
                    class: "widget-host",
                    routing: Routing::Hash,
                    sign_in_url: "/sign-in",
                    waitlist_url: "/waitlist",
                    fallback: rsx! { span { "loading sign up" } },
                }
                CreateOrganization { id: "create-organization-host", class: "widget-host", routing: Routing::Hash }
                OrganizationList { id: "organization-list-host", class: "widget-host" }
                Waitlist { id: "waitlist-host", class: "widget-host" }
                SignInButton {
                    class: "btn",
                    id: "sign-in",
                    disabled: false,
                    aria_label: "Sign in",
                    sign_up_url: "/sign-up",
                    fallback_redirect_url: "/",
                    "Sign in"
                }
                SignUpButton {
                    mode: AuthButtonMode::Redirect,
                    class: "btn",
                    sign_in_url: "/sign-in",
                    fallback_redirect_url: "/",
                    "Sign up"
                }
            }
            SignedOutWhenLoaded {
                fallback: rsx! { span { "checking auth" } },
                SignInButton { class: "btn", "Sign in after load" }
            }
            Protect {
                role: "admin",
                fallback: rsx! { span { "denied" } },
                span { "protected" }
            }
            Protect {
                permission: "org:read",
                fallback: rsx! { span { "hidden" } },
                span { "shown" }
            }
        }
    }
}

#[component]
fn ReadmeQuickstartSnippet() -> Element {
    rsx! {
        ClerkProvider { publishable_key: "pk_test_public",
            SignedOut { SignInButton { class: "btn" } }
            SignedIn { UserButton {} }
        }
    }
}

#[component]
fn OptionBuilderSurface() -> Element {
    rsx! {
        ClerkProvider {
            publishable_key: "pk_test_public",
            options: ClerkOptions::new()
                .sign_in_url("/sign-in")
                .sign_up_url("/sign-up")
                .appearance(serde_json::json!({ "variables": { "colorPrimary": "blue" } }))
                .localization(serde_json::json!({ "locale": "en-US" })),
            SignIn {
                options: SignInOptions::new()
                    .routing(Routing::Hash)
                    .path("/sign-in")
                    .fallback_redirect_url("/dashboard")
                    .initial_values(serde_json::json!({ "emailAddress": "ada@example.com" })),
            }
            SignUp {
                options: SignUpOptions::new()
                    .routing(Routing::Hash)
                    .path("/sign-up")
                    .fallback_redirect_url("/dashboard"),
            }
            UserButton {
                options: UserButtonOptions::new()
                    .show_name(true)
                    .user_profile_mode(UserProfileMode::Modal)
                    .appearance(serde_json::json!({ "elements": { "avatarBox": "h-8 w-8" } })),
            }
            UserProfile {
                options: UserProfileOptions::new()
                    .routing(Routing::Hash)
                    .path("/user"),
            }
            CreateOrganization {
                options: CreateOrganizationOptions::new()
                    .routing(Routing::Hash)
                    .after_create_organization_url("/dashboard"),
            }
            OrganizationSwitcher {
                options: OrganizationSwitcherOptions::new()
                    .create_organization_url("/organizations/new")
                    .organization_profile_url("/organization"),
            }
            OrganizationProfile {
                options: OrganizationProfileOptions::new()
                    .routing(Routing::Hash)
                    .path("/organization"),
            }
            OrganizationList {
                options: OrganizationListOptions::new()
                    .after_create_organization_url("/dashboard")
                    .after_select_organization_url("/dashboard"),
            }
            Waitlist {
                options: WaitlistOptions::new().after_join_waitlist_url("/"),
            }
            RedirectToSignIn {
                options: RedirectOptions::new().fallback_redirect_url("/sign-in"),
            }
            SignInButton {
                options: SignInOptions::new().fallback_redirect_url("/dashboard"),
            }
            SignUpButton {
                options: SignUpOptions::new().fallback_redirect_url("/dashboard"),
            }
            SignOutButton {
                options: SignOutOptions::new().redirect_url("/"),
            }
        }
    }
}

#[component]
fn HookProbe() -> Element {
    let auth = use_auth();
    let user = use_user();
    let session = use_session();
    let error = use_clerk_error();
    let clear_error = use_clear_clerk_error();
    let clerk = use_clerk();
    let reverify = use_reverification();

    let _ = auth.is_signed_in();
    let _ = matches!(auth.status(), AuthStatus::SignedIn);
    let _ = auth.is_signed_out();
    let _ = auth.has_role("admin");
    let _ = auth.org_slug();
    let _ = auth.has(&AuthRequirement::permission("org:read"));
    let _ = auth.require_signed_in().ok();
    assert_future(auth.get_token_with_options(GetTokenOptions::new().template("api")));
    auth.sign_out();
    auth.sign_out_with_options(SignOutOptions::new().redirect_url("/"));
    assert_future(auth.try_sign_out());
    assert_future(auth.try_sign_out_with_options(SignOutOptions::new().redirect_url("/")));
    let _ = user.user().as_ref().map(|user| user.id.as_str());
    let _user_from_root: Option<User> = user.user();
    let _ = user.is_loaded();
    let _ = session
        .session()
        .as_ref()
        .map(|session| session.id.as_str());
    let _session_from_root: Option<Session> = session.session();
    let _ = session.is_signed_in();
    let _ = error.read().as_ref().map(|error| error.to_string());
    let _ = clear_error;
    clerk.open_sign_in();
    clerk.open_sign_up_with_options(SignUpOptions::new().routing(Routing::Hash));
    clerk.redirect_to_sign_in_with_options(RedirectOptions::new().fallback_redirect_url("/"));
    clerk.sign_out_with_options(SignOutOptions::new().redirect_url("/"));
    assert_future(clerk.try_sign_out());
    // Guard a sensitive action behind step-up reverification; a `#[server]`
    // action would map a clerk hint via `ClerkError::from_reverification_hint`.
    assert_future(reverify.guard(|| async { Ok::<(), ClerkError>(()) }));
    let _ = ReverificationLevel::SecondFactor.as_str();
    let _ = GetTokenOptions::new()
        .template("api")
        .organization_id("org_2ghi")
        .leeway_in_seconds(30);

    rsx! {}
}

fn assert_future<T>(_future: T) {}
