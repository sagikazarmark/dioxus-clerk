//! Clerk options mapping: typed option builders for Clerk JS calls.
//!
//! The typed builders are the single surface translating Rust-side option
//! names into clerk-js JSON option keys — Clerk widget component props
//! delegate to them through the `maybe_*` setter variants, so each clerk-js
//! key is stated exactly once. Use [`JsonOptions::option`] or
//! `serde_json::Value` directly for Clerk options this crate has not named
//! yet.
//!
//! Each Clerk widget component's `options` prop is typed to *its own* builder,
//! so handing it the wrong builder is a compile error rather than silently
//! forwarding option keys the widget ignores:
//!
//! ```compile_fail
//! use dioxus_clerk::{ClerkOptions, SignInOptions};
//! // `ClerkOptions` cannot stand in for `SignInOptions`.
//! let _: SignInOptions = ClerkOptions::new().into();
//! ```
//!
//! A raw [`serde_json::Value`] is still accepted for any builder, so the
//! escape hatch for un-named Clerk options keeps working:
//!
//! ```
//! use dioxus_clerk::SignInOptions;
//! let _: SignInOptions = dioxus_clerk::serde_json::json!({ "signUpUrl": "/su" }).into();
//! ```

use serde_json::{Map, Value};

/// Clerk option keys shared by the typed builders, so each key is spelled
/// once. clerk-js silently ignores misspelled keys; a shared constant turns
/// drift into a compile error.
pub(crate) mod keys {
    pub(crate) const AFTER_CREATE_ORGANIZATION_URL: &str = "afterCreateOrganizationUrl";
    pub(crate) const AFTER_JOIN_WAITLIST_URL: &str = "afterJoinWaitlistUrl";
    pub(crate) const AFTER_MULTI_SESSION_SINGLE_SIGN_OUT_URL: &str =
        "afterMultiSessionSingleSignOutUrl";
    pub(crate) const AFTER_SELECT_ORGANIZATION_URL: &str = "afterSelectOrganizationUrl";
    pub(crate) const AFTER_SIGN_OUT_URL: &str = "afterSignOutUrl";
    pub(crate) const AFTER_SWITCH_SESSION_URL: &str = "afterSwitchSessionUrl";
    pub(crate) const ALLOWED_REDIRECT_ORIGINS: &str = "allowedRedirectOrigins";
    pub(crate) const ALLOWED_REDIRECT_PROTOCOLS: &str = "allowedRedirectProtocols";
    pub(crate) const APPEARANCE: &str = "appearance";
    pub(crate) const CREATE_ORGANIZATION_URL: &str = "createOrganizationUrl";
    pub(crate) const DEFAULT_OPEN: &str = "defaultOpen";
    pub(crate) const DOMAIN: &str = "domain";
    pub(crate) const FALLBACK_REDIRECT_URL: &str = "fallbackRedirectUrl";
    pub(crate) const FORCE_REDIRECT_URL: &str = "forceRedirectUrl";
    pub(crate) const INITIAL_VALUES: &str = "initialValues";
    pub(crate) const IS_SATELLITE: &str = "isSatellite";
    pub(crate) const LEEWAY_IN_SECONDS: &str = "leewayInSeconds";
    pub(crate) const LOCALIZATION: &str = "localization";
    pub(crate) const ORGANIZATION_ID: &str = "organizationId";
    pub(crate) const ORGANIZATION_PROFILE_URL: &str = "organizationProfileUrl";
    pub(crate) const PATH: &str = "path";
    pub(crate) const PREFETCH_UI: &str = "prefetchUI";
    pub(crate) const PROXY_URL: &str = "proxyUrl";
    pub(crate) const REDIRECT_URL: &str = "redirectUrl";
    pub(crate) const REDIRECT_URL_COMPLETE: &str = "redirectUrlComplete";
    pub(crate) const ROUTING: &str = "routing";
    pub(crate) const SATELLITE_AUTO_SYNC: &str = "satelliteAutoSync";
    pub(crate) const SESSION_ID: &str = "sessionId";
    pub(crate) const SHOW_NAME: &str = "showName";
    pub(crate) const SIGN_IN_FALLBACK_REDIRECT_URL: &str = "signInFallbackRedirectUrl";
    pub(crate) const SIGN_IN_FORCE_REDIRECT_URL: &str = "signInForceRedirectUrl";
    pub(crate) const SIGN_IN_URL: &str = "signInUrl";
    pub(crate) const SIGN_UP_FALLBACK_REDIRECT_URL: &str = "signUpFallbackRedirectUrl";
    pub(crate) const SIGN_UP_FORCE_REDIRECT_URL: &str = "signUpForceRedirectUrl";
    pub(crate) const SIGN_UP_URL: &str = "signUpUrl";
    pub(crate) const SKIP_CACHE: &str = "skipCache";
    pub(crate) const TASK_URLS: &str = "taskUrls";
    pub(crate) const TEMPLATE: &str = "template";
    pub(crate) const TRANSFERABLE: &str = "transferable";
    pub(crate) const USER_PROFILE_MODE: &str = "userProfileMode";
    pub(crate) const USER_PROFILE_PROPS: &str = "userProfileProps";
    pub(crate) const USER_PROFILE_URL: &str = "userProfileUrl";
    pub(crate) const WAITLIST_URL: &str = "waitlistUrl";
}

/// Generic JSON option builder used as the escape hatch for unsupported Clerk
/// options.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "builders are consuming; assign or chain the returned value"]
pub struct JsonOptions {
    value: Value,
}

impl Default for JsonOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonOptions {
    /// Start with an empty JSON object.
    pub fn new() -> Self {
        Self {
            value: Value::Object(Map::new()),
        }
    }

    /// Wrap an already-built JSON value.
    pub fn from_value(value: Value) -> Self {
        Self { value }
    }

    /// Set a raw clerk-js option by its JavaScript key.
    ///
    /// If the wrapped value is not a JSON object (for example a
    /// [`JsonOptions::from_value`] array or string), it is replaced with an
    /// empty object before the key is set.
    pub fn option(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        if !self.value.is_object() {
            self.value = Value::Object(Map::new());
        }
        if let Value::Object(map) = &mut self.value {
            map.insert(key.into(), value.into());
        }
        self
    }

    /// Convert into the raw JSON value passed to clerk-js.
    #[must_use = "into_value returns the built options; it does not send them anywhere"]
    pub fn into_value(self) -> Value {
        self.value
    }
}

impl From<JsonOptions> for Value {
    fn from(options: JsonOptions) -> Self {
        options.into_value()
    }
}

impl From<&JsonOptions> for Value {
    fn from(options: &JsonOptions) -> Self {
        options.value.clone()
    }
}

/// Routing mode for Clerk's embedded UI components.
///
/// This mirrors clerk-js's fixed `hash`/`path` routing set; it is not
/// `#[non_exhaustive]` so downstream `match`es need no wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Routing {
    /// Clerk manages embedded routing through the URL hash.
    Hash,
    /// Clerk manages embedded routing through the current path.
    Path,
}

impl Routing {
    /// The raw clerk-js routing string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hash => "hash",
            Self::Path => "path",
        }
    }
}

impl From<Routing> for Value {
    fn from(routing: Routing) -> Self {
        Value::String(routing.as_str().into())
    }
}

/// How `UserButton` opens the user profile UI.
///
/// This mirrors clerk-js's fixed `modal`/`navigation` set; it is not
/// `#[non_exhaustive]` so downstream `match`es need no wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UserProfileMode {
    /// Open the profile as a modal.
    Modal,
    /// Navigate to `user_profile_url`.
    Navigation,
}

impl UserProfileMode {
    /// The raw clerk-js user profile mode string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Modal => "modal",
            Self::Navigation => "navigation",
        }
    }
}

impl From<UserProfileMode> for Value {
    fn from(mode: UserProfileMode) -> Self {
        Value::String(mode.as_str().into())
    }
}

macro_rules! option_wrapper {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Default, PartialEq, Eq)]
        #[must_use = "builders are consuming; assign or chain the returned value"]
        pub struct $name {
            inner: JsonOptions,
        }

        impl $name {
            /// Start with an empty Clerk options object.
            pub fn new() -> Self {
                Self {
                    inner: JsonOptions::new(),
                }
            }

            /// Wrap an already-built JSON value.
            pub fn from_value(value: Value) -> Self {
                Self {
                    inner: JsonOptions::from_value(value),
                }
            }

            /// Set a raw clerk-js option by its JavaScript key.
            ///
            /// If the wrapped value is not a JSON object, it is replaced with
            /// an empty object before the key is set.
            pub fn option(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
                self.inner = self.inner.option(key, value);
                self
            }

            /// Convert into the raw JSON value passed to clerk-js.
            #[must_use = "into_value returns the built options; it does not send them anywhere"]
            pub fn into_value(self) -> Value {
                self.inner.into_value()
            }
        }

        impl From<$name> for Value {
            fn from(options: $name) -> Self {
                options.into_value()
            }
        }

        impl From<&$name> for Value {
            fn from(options: &$name) -> Self {
                options.inner.value.clone()
            }
        }

        // Raw-`Value` escape hatch: lets a widget's `options` prop accept a
        // `serde_json::Value` (via `#[props(into)]`) while still rejecting a
        // different builder type at compile time. Wrong builder in → no
        // `Into` impl → compile error.
        impl From<Value> for $name {
            fn from(value: Value) -> Self {
                Self::from_value(value)
            }
        }
    };
}

option_wrapper!(
    ClerkOptions,
    "Options forwarded to `Clerk.load(...)` by `ClerkProvider`."
);
option_wrapper!(SignInOptions, "Options for sign-in UI and modal flows.");
option_wrapper!(SignUpOptions, "Options for sign-up UI and modal flows.");
option_wrapper!(UserButtonOptions, "Options for the Clerk user button.");
option_wrapper!(UserProfileOptions, "Options for the Clerk user profile UI.");
option_wrapper!(
    CreateOrganizationOptions,
    "Options for the Clerk create-organization UI."
);
option_wrapper!(
    OrganizationProfileOptions,
    "Options for the Clerk organization profile UI."
);
option_wrapper!(
    OrganizationSwitcherOptions,
    "Options for the Clerk organization switcher UI."
);
option_wrapper!(
    OrganizationListOptions,
    "Options for the Clerk organization list UI."
);
option_wrapper!(WaitlistOptions, "Options for the Clerk waitlist UI.");
option_wrapper!(
    TaskSetupMFAOptions,
    "Options for the Clerk task setup-MFA UI."
);
option_wrapper!(RedirectOptions, "Options for Clerk redirect helpers.");
option_wrapper!(SignOutOptions, "Options for `Clerk.signOut(...)`.");
option_wrapper!(GetTokenOptions, "Options for `session.getToken(...)`.");

/// Generate the typed setter table for one option wrapper. Each entry expands
/// to the public setter plus a `pub(crate) maybe_*` variant used by the Clerk
/// widget component prop mappings, so a prop that is `None` leaves the
/// options untouched. Kinds: `string`, `bool`, `u64`, `value` (raw JSON),
/// `list` (string list), and `enum(Type)` for typed enums convertible to
/// [`Value`].
macro_rules! option_setters {
    ($owner:ident { $($rest:tt)* }) => {
        impl $owner {
            option_setters!(@items $($rest)*);
        }
    };
    (@items) => {};
    (@items $(#[$doc:meta])* string $name:ident / $maybe:ident => $key:path; $($rest:tt)*) => {
        $(#[$doc])*
        pub fn $name(self, value: impl Into<String>) -> Self {
            self.option($key, value.into())
        }

        #[allow(dead_code)]
        pub(crate) fn $maybe(self, value: Option<String>) -> Self {
            match value {
                Some(value) => self.$name(value),
                None => self,
            }
        }

        option_setters!(@items $($rest)*);
    };
    (@items $(#[$doc:meta])* bool $name:ident / $maybe:ident => $key:path; $($rest:tt)*) => {
        $(#[$doc])*
        pub fn $name(self, value: bool) -> Self {
            self.option($key, value)
        }

        #[allow(dead_code)]
        pub(crate) fn $maybe(self, value: Option<bool>) -> Self {
            match value {
                Some(value) => self.$name(value),
                None => self,
            }
        }

        option_setters!(@items $($rest)*);
    };
    (@items $(#[$doc:meta])* u64 $name:ident / $maybe:ident => $key:path; $($rest:tt)*) => {
        $(#[$doc])*
        pub fn $name(self, value: u64) -> Self {
            self.option($key, value)
        }

        #[allow(dead_code)]
        pub(crate) fn $maybe(self, value: Option<u64>) -> Self {
            match value {
                Some(value) => self.$name(value),
                None => self,
            }
        }

        option_setters!(@items $($rest)*);
    };
    (@items $(#[$doc:meta])* value $name:ident / $maybe:ident => $key:path; $($rest:tt)*) => {
        $(#[$doc])*
        pub fn $name(self, value: Value) -> Self {
            self.option($key, value)
        }

        #[allow(dead_code)]
        pub(crate) fn $maybe(self, value: Option<Value>) -> Self {
            match value {
                Some(value) => self.$name(value),
                None => self,
            }
        }

        option_setters!(@items $($rest)*);
    };
    (@items $(#[$doc:meta])* list $name:ident / $maybe:ident => $key:path; $($rest:tt)*) => {
        $(#[$doc])*
        pub fn $name(self, values: impl IntoIterator<Item = impl Into<String>>) -> Self {
            let values: Vec<String> = values.into_iter().map(Into::into).collect();
            self.option($key, serde_json::json!(values))
        }

        #[allow(dead_code)]
        pub(crate) fn $maybe(self, values: Option<Vec<String>>) -> Self {
            match values {
                Some(values) => self.$name(values),
                None => self,
            }
        }

        option_setters!(@items $($rest)*);
    };
    (@items $(#[$doc:meta])* enum($ty:ty) $name:ident / $maybe:ident => $key:path; $($rest:tt)*) => {
        $(#[$doc])*
        pub fn $name(self, value: $ty) -> Self {
            self.option($key, value)
        }

        #[allow(dead_code)]
        pub(crate) fn $maybe(self, value: Option<$ty>) -> Self {
            match value {
                Some(value) => self.$name(value),
                None => self,
            }
        }

        option_setters!(@items $($rest)*);
    };
}

option_setters!(ClerkOptions {
    /// Set the application sign-in URL.
    string sign_in_url / maybe_sign_in_url => keys::SIGN_IN_URL;
    /// Set the application sign-up URL.
    string sign_up_url / maybe_sign_up_url => keys::SIGN_UP_URL;
    /// Set the URL to redirect to after sign-in when no `redirect_url` is in play.
    string sign_in_fallback_redirect_url / maybe_sign_in_fallback_redirect_url => keys::SIGN_IN_FALLBACK_REDIRECT_URL;
    /// Set the URL to always redirect to after sign-in.
    string sign_in_force_redirect_url / maybe_sign_in_force_redirect_url => keys::SIGN_IN_FORCE_REDIRECT_URL;
    /// Set the URL to redirect to after sign-up when no `redirect_url` is in play.
    string sign_up_fallback_redirect_url / maybe_sign_up_fallback_redirect_url => keys::SIGN_UP_FALLBACK_REDIRECT_URL;
    /// Set the URL to always redirect to after sign-up.
    string sign_up_force_redirect_url / maybe_sign_up_force_redirect_url => keys::SIGN_UP_FORCE_REDIRECT_URL;
    /// Set the URL Clerk should use after sign-out.
    string after_sign_out_url / maybe_after_sign_out_url => keys::AFTER_SIGN_OUT_URL;
    /// Set the URL Clerk should use after signing out one account in multi-session apps.
    string after_multi_session_single_sign_out_url / maybe_after_multi_session_single_sign_out_url => keys::AFTER_MULTI_SESSION_SINGLE_SIGN_OUT_URL;
    /// Set the URL Clerk should use after switching sessions in multi-session apps.
    string after_switch_session_url / maybe_after_switch_session_url => keys::AFTER_SWITCH_SESSION_URL;
    /// Set the application waitlist URL.
    string waitlist_url / maybe_waitlist_url => keys::WAITLIST_URL;
    /// Set the application user profile URL.
    string user_profile_url / maybe_user_profile_url => keys::USER_PROFILE_URL;
    /// Set the application organization profile URL.
    string organization_profile_url / maybe_organization_profile_url => keys::ORGANIZATION_PROFILE_URL;
    /// Set the application create-organization URL.
    string create_organization_url / maybe_create_organization_url => keys::CREATE_ORGANIZATION_URL;
    /// Set Clerk's reverse-proxy URL.
    string proxy_url / maybe_proxy_url => keys::PROXY_URL;
    /// Set Clerk's satellite domain.
    string domain / maybe_domain => keys::DOMAIN;
    /// Set whether Clerk should treat this app as a satellite application.
    bool is_satellite / maybe_is_satellite => keys::IS_SATELLITE;
    /// Set whether satellite apps should automatically sync on initial page load.
    bool satellite_auto_sync / maybe_satellite_auto_sync => keys::SATELLITE_AUTO_SYNC;
    /// Set whether Clerk should prefetch its UI package when supported.
    bool prefetch_ui / maybe_prefetch_ui => keys::PREFETCH_UI;
    /// Set Clerk's `allowedRedirectOrigins` list.
    list allowed_redirect_origins / maybe_allowed_redirect_origins => keys::ALLOWED_REDIRECT_ORIGINS;
    /// Set Clerk's `allowedRedirectProtocols` list.
    list allowed_redirect_protocols / maybe_allowed_redirect_protocols => keys::ALLOWED_REDIRECT_PROTOCOLS;
    /// Forward Clerk's raw `appearance` object.
    value appearance / maybe_appearance => keys::APPEARANCE;
    /// Forward Clerk's raw `localization` object.
    value localization / maybe_localization => keys::LOCALIZATION;
    /// Map clerk-js session-task keys to the app URLs Clerk navigates to when a
    /// session has a pending task (e.g. `{ "setup-mfa": "/onboarding/mfa" }`).
    value task_urls / maybe_task_urls => keys::TASK_URLS;
});

option_setters!(SignInOptions {
    /// Set embedded routing mode, for example `"path"` or `"hash"`.
    enum(Routing) routing / maybe_routing => keys::ROUTING;
    /// Set the path used by embedded routing.
    string path / maybe_path => keys::PATH;
    /// Set the sign-up URL linked from the sign-in flow.
    string sign_up_url / maybe_sign_up_url => keys::SIGN_UP_URL;
    /// Set the waitlist URL linked from the sign-in flow.
    string waitlist_url / maybe_waitlist_url => keys::WAITLIST_URL;
    /// Always redirect here after sign-in.
    string force_redirect_url / maybe_force_redirect_url => keys::FORCE_REDIRECT_URL;
    /// Fallback redirect URL after sign-in.
    string fallback_redirect_url / maybe_fallback_redirect_url => keys::FALLBACK_REDIRECT_URL;
    /// Always redirect here after sign-up from sign-in.
    string sign_up_force_redirect_url / maybe_sign_up_force_redirect_url => keys::SIGN_UP_FORCE_REDIRECT_URL;
    /// Fallback redirect URL after sign-up from sign-in.
    string sign_up_fallback_redirect_url / maybe_sign_up_fallback_redirect_url => keys::SIGN_UP_FALLBACK_REDIRECT_URL;
    /// Forward Clerk's raw `initialValues` object.
    value initial_values / maybe_initial_values => keys::INITIAL_VALUES;
    /// Set whether sign-in attempts can transfer to sign-up when Clerk supports it.
    bool transferable / maybe_transferable => keys::TRANSFERABLE;
    /// Forward Clerk's raw `appearance` object.
    value appearance / maybe_appearance => keys::APPEARANCE;
});

option_setters!(SignUpOptions {
    /// Set embedded routing mode, for example `"path"` or `"hash"`.
    enum(Routing) routing / maybe_routing => keys::ROUTING;
    /// Set the path used by embedded routing.
    string path / maybe_path => keys::PATH;
    /// Set the sign-in URL linked from the sign-up flow.
    string sign_in_url / maybe_sign_in_url => keys::SIGN_IN_URL;
    /// Set the waitlist URL linked from the sign-up flow.
    string waitlist_url / maybe_waitlist_url => keys::WAITLIST_URL;
    /// Always redirect here after sign-up.
    string force_redirect_url / maybe_force_redirect_url => keys::FORCE_REDIRECT_URL;
    /// Fallback redirect URL after sign-up.
    string fallback_redirect_url / maybe_fallback_redirect_url => keys::FALLBACK_REDIRECT_URL;
    /// Always redirect here after sign-in from sign-up.
    string sign_in_force_redirect_url / maybe_sign_in_force_redirect_url => keys::SIGN_IN_FORCE_REDIRECT_URL;
    /// Fallback redirect URL after sign-in from sign-up.
    string sign_in_fallback_redirect_url / maybe_sign_in_fallback_redirect_url => keys::SIGN_IN_FALLBACK_REDIRECT_URL;
    /// Forward Clerk's raw `initialValues` object.
    value initial_values / maybe_initial_values => keys::INITIAL_VALUES;
    /// Forward Clerk's raw `appearance` object.
    value appearance / maybe_appearance => keys::APPEARANCE;
});

option_setters!(UserButtonOptions {
    /// Set the URL Clerk should use after sign-out.
    string after_sign_out_url / maybe_after_sign_out_url => keys::AFTER_SIGN_OUT_URL;
    /// Set the URL Clerk should use after switching sessions in multi-session apps.
    string after_switch_session_url / maybe_after_switch_session_url => keys::AFTER_SWITCH_SESSION_URL;
    /// Set the URL Clerk should use when adding another account.
    string sign_in_url / maybe_sign_in_url => keys::SIGN_IN_URL;
    /// Show the user's name next to the avatar when Clerk supports it.
    bool show_name / maybe_show_name => keys::SHOW_NAME;
    /// Open the user button menu by default on first render.
    bool default_open / maybe_default_open => keys::DEFAULT_OPEN;
    /// Set the user profile mode, for example `"modal"` or `"navigation"`.
    enum(UserProfileMode) user_profile_mode / maybe_user_profile_mode => keys::USER_PROFILE_MODE;
    /// Set the user profile URL for navigation mode.
    string user_profile_url / maybe_user_profile_url => keys::USER_PROFILE_URL;
    /// Forward options to the underlying `UserProfile` component.
    value user_profile_props / maybe_user_profile_props => keys::USER_PROFILE_PROPS;
    /// Forward Clerk's raw `appearance` object.
    value appearance / maybe_appearance => keys::APPEARANCE;
});

option_setters!(UserProfileOptions {
    /// Set embedded routing mode, for example `"path"` or `"hash"`.
    enum(Routing) routing / maybe_routing => keys::ROUTING;
    /// Set the path used by embedded routing.
    string path / maybe_path => keys::PATH;
    /// Forward Clerk's raw `appearance` object.
    value appearance / maybe_appearance => keys::APPEARANCE;
});

option_setters!(CreateOrganizationOptions {
    /// Set embedded routing mode.
    enum(Routing) routing / maybe_routing => keys::ROUTING;
    /// Set the path used by embedded routing.
    string path / maybe_path => keys::PATH;
    /// Set where Clerk redirects after creating an organization.
    string after_create_organization_url / maybe_after_create_organization_url => keys::AFTER_CREATE_ORGANIZATION_URL;
    /// Forward Clerk's raw `appearance` object.
    value appearance / maybe_appearance => keys::APPEARANCE;
});

option_setters!(OrganizationProfileOptions {
    /// Set embedded routing mode.
    enum(Routing) routing / maybe_routing => keys::ROUTING;
    /// Set the path used by embedded routing.
    string path / maybe_path => keys::PATH;
    /// Forward Clerk's raw `appearance` object.
    value appearance / maybe_appearance => keys::APPEARANCE;
});

option_setters!(OrganizationSwitcherOptions {
    /// Set where Clerk navigates to create an organization.
    string create_organization_url / maybe_create_organization_url => keys::CREATE_ORGANIZATION_URL;
    /// Set where Clerk redirects after creating an organization.
    string after_create_organization_url / maybe_after_create_organization_url => keys::AFTER_CREATE_ORGANIZATION_URL;
    /// Set where Clerk navigates for organization profile management.
    string organization_profile_url / maybe_organization_profile_url => keys::ORGANIZATION_PROFILE_URL;
    /// Forward Clerk's raw `appearance` object.
    value appearance / maybe_appearance => keys::APPEARANCE;
});

option_setters!(OrganizationListOptions {
    /// Set where Clerk redirects after creating an organization.
    string after_create_organization_url / maybe_after_create_organization_url => keys::AFTER_CREATE_ORGANIZATION_URL;
    /// Set where Clerk redirects after selecting an organization.
    string after_select_organization_url / maybe_after_select_organization_url => keys::AFTER_SELECT_ORGANIZATION_URL;
    /// Forward Clerk's raw `appearance` object.
    value appearance / maybe_appearance => keys::APPEARANCE;
});

option_setters!(WaitlistOptions {
    /// Set where Clerk redirects after joining the waitlist.
    string after_join_waitlist_url / maybe_after_join_waitlist_url => keys::AFTER_JOIN_WAITLIST_URL;
    /// Forward Clerk's raw `appearance` object.
    value appearance / maybe_appearance => keys::APPEARANCE;
});

option_setters!(TaskSetupMFAOptions {
    /// Set the URL Clerk navigates to after all pending session tasks resolve.
    string redirect_url_complete / maybe_redirect_url_complete => keys::REDIRECT_URL_COMPLETE;
    /// Forward Clerk's raw `appearance` object.
    value appearance / maybe_appearance => keys::APPEARANCE;
});

option_setters!(RedirectOptions {
    /// Always redirect here after the target auth flow.
    string force_redirect_url / maybe_force_redirect_url => keys::FORCE_REDIRECT_URL;
    /// Fallback redirect URL after the target auth flow.
    string fallback_redirect_url / maybe_fallback_redirect_url => keys::FALLBACK_REDIRECT_URL;
    /// Always redirect here after sign-up from sign-in.
    string sign_up_force_redirect_url / maybe_sign_up_force_redirect_url => keys::SIGN_UP_FORCE_REDIRECT_URL;
    /// Fallback redirect URL after sign-up from sign-in.
    string sign_up_fallback_redirect_url / maybe_sign_up_fallback_redirect_url => keys::SIGN_UP_FALLBACK_REDIRECT_URL;
    /// Always redirect here after sign-in from sign-up.
    string sign_in_force_redirect_url / maybe_sign_in_force_redirect_url => keys::SIGN_IN_FORCE_REDIRECT_URL;
    /// Fallback redirect URL after sign-in from sign-up.
    string sign_in_fallback_redirect_url / maybe_sign_in_fallback_redirect_url => keys::SIGN_IN_FALLBACK_REDIRECT_URL;
});

option_setters!(SignOutOptions {
    /// Full URL or path to navigate to after sign-out.
    string redirect_url / maybe_redirect_url => keys::REDIRECT_URL;
    /// Sign out a specific session id in multi-session applications.
    string session_id / maybe_session_id => keys::SESSION_ID;
});

option_setters!(GetTokenOptions {
    /// Use a named Clerk JWT template.
    string template / maybe_template => keys::TEMPLATE;
    /// Request a token scoped to a specific organization without changing the
    /// active organization in clerk-js.
    string organization_id / maybe_organization_id => keys::ORGANIZATION_ID;
    /// Allow Clerk to reuse a cached token for this many extra seconds when supported.
    u64 leeway_in_seconds / maybe_leeway_in_seconds => keys::LEEWAY_IN_SECONDS;
    /// Ask Clerk to bypass its token cache when supported.
    bool skip_cache / maybe_skip_cache => keys::SKIP_CACHE;
});

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn clerk_options_emit_clerk_keys() {
        let value = ClerkOptions::new()
            .sign_in_url("/si")
            .sign_up_url("/su")
            .sign_in_fallback_redirect_url("/sifb")
            .sign_in_force_redirect_url("/sif")
            .sign_up_fallback_redirect_url("/sufb")
            .sign_up_force_redirect_url("/suf")
            .after_sign_out_url("/aso")
            .after_multi_session_single_sign_out_url("/amsso")
            .after_switch_session_url("/ass")
            .waitlist_url("/wl")
            .user_profile_url("/up")
            .organization_profile_url("/op")
            .create_organization_url("/co")
            .proxy_url("/proxy")
            .domain("clerk.example.com")
            .is_satellite(true)
            .satellite_auto_sync(false)
            .prefetch_ui(true)
            .allowed_redirect_origins(["https://a.example"])
            .allowed_redirect_protocols(["https"])
            .appearance(json!({"variables": {}}))
            .localization(json!({"locale": "en-US"}))
            .into_value();

        assert_eq!(
            value,
            json!({
                "signInUrl": "/si",
                "signUpUrl": "/su",
                "signInFallbackRedirectUrl": "/sifb",
                "signInForceRedirectUrl": "/sif",
                "signUpFallbackRedirectUrl": "/sufb",
                "signUpForceRedirectUrl": "/suf",
                "afterSignOutUrl": "/aso",
                "afterMultiSessionSingleSignOutUrl": "/amsso",
                "afterSwitchSessionUrl": "/ass",
                "waitlistUrl": "/wl",
                "userProfileUrl": "/up",
                "organizationProfileUrl": "/op",
                "createOrganizationUrl": "/co",
                "proxyUrl": "/proxy",
                "domain": "clerk.example.com",
                "isSatellite": true,
                "satelliteAutoSync": false,
                "prefetchUI": true,
                "allowedRedirectOrigins": ["https://a.example"],
                "allowedRedirectProtocols": ["https"],
                "appearance": {"variables": {}},
                "localization": {"locale": "en-US"},
            })
        );
    }

    #[test]
    fn clerk_options_emit_session_task_urls() {
        // clerk-js v6 `taskUrls` maps session-task keys to the app URLs Clerk
        // navigates to when a session has a pending after-auth task.
        let value = ClerkOptions::new()
            .task_urls(json!({ "setup-mfa": "/onboarding/mfa" }))
            .into_value();

        assert_eq!(
            value,
            json!({ "taskUrls": { "setup-mfa": "/onboarding/mfa" } })
        );
    }

    #[test]
    fn sign_in_options_emit_clerk_keys() {
        let value = SignInOptions::new()
            .routing(Routing::Path)
            .path("/sign-in")
            .sign_up_url("/su")
            .waitlist_url("/wl")
            .force_redirect_url("/f")
            .fallback_redirect_url("/fb")
            .sign_up_force_redirect_url("/suf")
            .sign_up_fallback_redirect_url("/sufb")
            .initial_values(json!({"emailAddress": "a@example.com"}))
            .transferable(true)
            .appearance(json!({"variables": {}}))
            .into_value();

        assert_eq!(
            value,
            json!({
                "routing": "path",
                "path": "/sign-in",
                "signUpUrl": "/su",
                "waitlistUrl": "/wl",
                "forceRedirectUrl": "/f",
                "fallbackRedirectUrl": "/fb",
                "signUpForceRedirectUrl": "/suf",
                "signUpFallbackRedirectUrl": "/sufb",
                "initialValues": {"emailAddress": "a@example.com"},
                "transferable": true,
                "appearance": {"variables": {}},
            })
        );
    }

    #[test]
    fn sign_up_options_emit_clerk_keys() {
        let value = SignUpOptions::new()
            .routing(Routing::Hash)
            .path("/sign-up")
            .sign_in_url("/si")
            .waitlist_url("/wl")
            .force_redirect_url("/f")
            .fallback_redirect_url("/fb")
            .sign_in_force_redirect_url("/sif")
            .sign_in_fallback_redirect_url("/sifb")
            .initial_values(json!({"username": "a"}))
            .appearance(json!({}))
            .into_value();

        assert_eq!(
            value,
            json!({
                "routing": "hash",
                "path": "/sign-up",
                "signInUrl": "/si",
                "waitlistUrl": "/wl",
                "forceRedirectUrl": "/f",
                "fallbackRedirectUrl": "/fb",
                "signInForceRedirectUrl": "/sif",
                "signInFallbackRedirectUrl": "/sifb",
                "initialValues": {"username": "a"},
                "appearance": {},
            })
        );
    }

    #[test]
    fn user_button_options_emit_clerk_keys() {
        let value = UserButtonOptions::new()
            .after_sign_out_url("/aso")
            .after_switch_session_url("/ass")
            .sign_in_url("/si")
            .show_name(true)
            .default_open(false)
            .user_profile_mode(UserProfileMode::Navigation)
            .user_profile_url("/up")
            .user_profile_props(json!({"appearance": {}}))
            .appearance(json!({}))
            .into_value();

        assert_eq!(
            value,
            json!({
                "afterSignOutUrl": "/aso",
                "afterSwitchSessionUrl": "/ass",
                "signInUrl": "/si",
                "showName": true,
                "defaultOpen": false,
                "userProfileMode": "navigation",
                "userProfileUrl": "/up",
                "userProfileProps": {"appearance": {}},
                "appearance": {},
            })
        );
    }

    #[test]
    fn widget_options_emit_clerk_keys() {
        assert_eq!(
            UserProfileOptions::new()
                .routing(Routing::Path)
                .path("/profile")
                .appearance(json!({}))
                .into_value(),
            json!({"routing": "path", "path": "/profile", "appearance": {}})
        );
        assert_eq!(
            CreateOrganizationOptions::new()
                .routing(Routing::Hash)
                .path("/create-org")
                .after_create_organization_url("/org")
                .appearance(json!({}))
                .into_value(),
            json!({
                "routing": "hash",
                "path": "/create-org",
                "afterCreateOrganizationUrl": "/org",
                "appearance": {},
            })
        );
        assert_eq!(
            OrganizationProfileOptions::new()
                .routing(Routing::Path)
                .path("/org-profile")
                .appearance(json!({}))
                .into_value(),
            json!({"routing": "path", "path": "/org-profile", "appearance": {}})
        );
        assert_eq!(
            OrganizationSwitcherOptions::new()
                .create_organization_url("/co")
                .after_create_organization_url("/aco")
                .organization_profile_url("/op")
                .appearance(json!({}))
                .into_value(),
            json!({
                "createOrganizationUrl": "/co",
                "afterCreateOrganizationUrl": "/aco",
                "organizationProfileUrl": "/op",
                "appearance": {},
            })
        );
        assert_eq!(
            OrganizationListOptions::new()
                .after_create_organization_url("/aco")
                .after_select_organization_url("/aso")
                .appearance(json!({}))
                .into_value(),
            json!({
                "afterCreateOrganizationUrl": "/aco",
                "afterSelectOrganizationUrl": "/aso",
                "appearance": {},
            })
        );
        assert_eq!(
            WaitlistOptions::new()
                .after_join_waitlist_url("/ajw")
                .appearance(json!({}))
                .into_value(),
            json!({"afterJoinWaitlistUrl": "/ajw", "appearance": {}})
        );
    }

    #[test]
    fn task_setup_mfa_options_emit_clerk_keys() {
        assert_eq!(
            TaskSetupMFAOptions::new()
                .redirect_url_complete("/onboarding/done")
                .appearance(json!({}))
                .into_value(),
            json!({ "redirectUrlComplete": "/onboarding/done", "appearance": {} })
        );
    }

    #[test]
    fn action_options_emit_clerk_keys() {
        assert_eq!(
            RedirectOptions::new()
                .force_redirect_url("/f")
                .fallback_redirect_url("/fb")
                .sign_up_force_redirect_url("/suf")
                .sign_up_fallback_redirect_url("/sufb")
                .sign_in_force_redirect_url("/sif")
                .sign_in_fallback_redirect_url("/sifb")
                .into_value(),
            json!({
                "forceRedirectUrl": "/f",
                "fallbackRedirectUrl": "/fb",
                "signUpForceRedirectUrl": "/suf",
                "signUpFallbackRedirectUrl": "/sufb",
                "signInForceRedirectUrl": "/sif",
                "signInFallbackRedirectUrl": "/sifb",
            })
        );
        assert_eq!(
            SignOutOptions::new()
                .redirect_url("/after")
                .session_id("sess_1")
                .into_value(),
            json!({"redirectUrl": "/after", "sessionId": "sess_1"})
        );
        assert_eq!(
            GetTokenOptions::new()
                .template("supabase")
                .organization_id("org_1")
                .leeway_in_seconds(10)
                .skip_cache(true)
                .into_value(),
            json!({
                "template": "supabase",
                "organizationId": "org_1",
                "leewayInSeconds": 10,
                "skipCache": true,
            })
        );
    }

    #[test]
    fn maybe_setters_with_none_leave_options_untouched() {
        let value = SignInOptions::new()
            .maybe_routing(None)
            .maybe_path(None)
            .maybe_sign_up_url(None)
            .maybe_initial_values(None)
            .maybe_transferable(None)
            .into_value();

        assert_eq!(value, json!({}));

        let untouched = SignInOptions::from_value(json!({"signUpUrl": "/kept"}))
            .maybe_sign_up_url(None)
            .into_value();
        assert_eq!(untouched, json!({"signUpUrl": "/kept"}));
    }

    #[test]
    fn builders_accept_a_raw_value_via_from() {
        // The widget `options` props rely on `From<Value>` (through
        // `#[props(into)]`) to keep accepting a raw JSON escape hatch. It must
        // match `from_value` exactly.
        let value = json!({ "signUpUrl": "/kept", "path": "/p" });
        assert_eq!(
            SignInOptions::from(value.clone()).into_value(),
            SignInOptions::from_value(value).into_value(),
        );
    }

    #[test]
    fn explicit_props_win_over_raw_options() {
        let value = SignInOptions::from_value(json!({"signUpUrl": "/old", "path": "/kept"}))
            .sign_up_url("/new")
            .into_value();

        assert_eq!(value, json!({"signUpUrl": "/new", "path": "/kept"}));
    }

    #[test]
    fn raw_null_options_stay_null_when_no_setter_runs() {
        assert_eq!(
            SignInOptions::from_value(serde_json::Value::Null)
                .maybe_sign_up_url(None)
                .into_value(),
            serde_json::Value::Null
        );
        assert_eq!(
            SignInOptions::from_value(serde_json::Value::Null)
                .sign_up_url("/su")
                .into_value(),
            json!({"signUpUrl": "/su"})
        );
    }
}
