use dioxus::dioxus_core::AttributeValue;
use dioxus::prelude::*;

#[cfg(clerk_client)]
const HOST_RETRY_MS: u32 = 16;
/// How long to retry a missing host element before surfacing an error instead
/// of polling the DOM forever (e.g. on an SSR/client host-id mismatch).
#[cfg(clerk_client)]
const HOST_DEADLINE_MS: u32 = 10_000;

/// Mountable Clerk widget vocabulary, shared between the SSR host rendering
/// (host-id prefixes compile on every target) and the JS bridge layer's
/// clerk-js mount/unmount method tables (browser client only).
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Widget {
    SignIn,
    SignUp,
    UserButton,
    UserProfile,
    CreateOrganization,
    OrganizationProfile,
    OrganizationSwitcher,
    OrganizationList,
    Waitlist,
    TaskSetupMfa,
}

impl Widget {
    fn host_id_prefix(self) -> &'static str {
        match self {
            Self::SignIn => "clerk-signin",
            Self::SignUp => "clerk-signup",
            Self::UserButton => "clerk-userbutton",
            Self::UserProfile => "clerk-userprofile",
            Self::CreateOrganization => "clerk-createorganization",
            Self::OrganizationProfile => "clerk-organizationprofile",
            Self::OrganizationSwitcher => "clerk-organizationswitcher",
            Self::OrganizationList => "clerk-organizationlist",
            Self::Waitlist => "clerk-waitlist",
            Self::TaskSetupMfa => "clerk-tasksetupmfa",
        }
    }

    #[cfg(clerk_client)]
    pub(crate) fn mount_method(self) -> &'static str {
        match self {
            Self::SignIn => "mountSignIn",
            Self::SignUp => "mountSignUp",
            Self::UserButton => "mountUserButton",
            Self::UserProfile => "mountUserProfile",
            Self::CreateOrganization => "mountCreateOrganization",
            Self::OrganizationProfile => "mountOrganizationProfile",
            Self::OrganizationSwitcher => "mountOrganizationSwitcher",
            Self::OrganizationList => "mountOrganizationList",
            Self::Waitlist => "mountWaitlist",
            // v6 casing: `MFA`, not `Mfa`.
            Self::TaskSetupMfa => "mountTaskSetupMFA",
        }
    }

    #[cfg(clerk_client)]
    pub(crate) fn unmount_method(self) -> &'static str {
        match self {
            Self::SignIn => "unmountSignIn",
            Self::SignUp => "unmountSignUp",
            Self::UserButton => "unmountUserButton",
            Self::UserProfile => "unmountUserProfile",
            Self::CreateOrganization => "unmountCreateOrganization",
            Self::OrganizationProfile => "unmountOrganizationProfile",
            Self::OrganizationSwitcher => "unmountOrganizationSwitcher",
            Self::OrganizationList => "unmountOrganizationList",
            Self::Waitlist => "unmountWaitlist",
            // v6 casing: `MFA`, not `Mfa`.
            Self::TaskSetupMfa => "unmountTaskSetupMFA",
        }
    }

    #[cfg(clerk_client)]
    fn mount(
        self,
        bridge: &crate::bridge::ClerkBridge,
        element: &web_sys::Element,
        options: &serde_json::Value,
    ) -> Result<(), crate::core::ClerkError> {
        bridge.mount_widget(self, element, options)
    }

    #[cfg(clerk_client)]
    fn unmount(self, bridge: &crate::bridge::ClerkBridge, element: &web_sys::Element) {
        bridge.unmount_widget(self, element);
    }
}

#[derive(Clone, Default, PartialEq)]
struct HostProps {
    id: Option<String>,
    attributes: Vec<Attribute>,
}

#[derive(Clone)]
struct MountedWidget {
    id: String,
    options: serde_json::Value,
    /// The DOM element clerk-js actually mounted into. Unmount paths must use
    /// this element — after an `id` prop change a fresh document lookup finds
    /// the wrong (or no) element, leaving clerk-js bookkeeping behind.
    #[cfg(clerk_client)]
    element: web_sys::Element,
}

impl PartialEq for MountedWidget {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.options == other.options
    }
}

pub(super) struct WidgetProps {
    options: serde_json::Value,
    fallback: Element,
    host: HostProps,
}

impl WidgetProps {
    pub(super) fn new(
        options: serde_json::Value,
        fallback: Element,
        id: Option<String>,
        attributes: Vec<Attribute>,
    ) -> Self {
        Self {
            options,
            fallback,
            host: HostProps { id, attributes },
        }
    }
}

pub(super) fn render(widget: Widget, props: WidgetProps) -> Element {
    let WidgetProps {
        options,
        fallback,
        host,
    } = props;
    rsx! { WidgetHost { widget, options, fallback, host } }
}

#[component]
fn WidgetHost(
    widget: Widget,
    options: serde_json::Value,
    fallback: Element,
    host: HostProps,
) -> Element {
    // The clerk-js mount target id, in precedence order: the explicit `id`
    // prop, then an `id` from the attribute spread, then a generated
    // scope-stable id. clerk-js mounts into this element by id, so the widget
    // must know it — the explicit prop stays authoritative, but a spread `id`
    // is honored as a fallback rather than silently dropped.
    let id = host
        .id
        .or_else(|| spread_id(&host.attributes))
        .unwrap_or_else(|| generated_host_id(widget));
    // The resolved id is rendered explicitly on the host `<div>` below, so drop
    // any `id` from the spread to keep a single authoritative `id` attribute.
    let mut attributes = host.attributes;
    attributes.retain(|attribute| attribute.name != "id");
    let mounted_widget = use_signal(|| None::<MountedWidget>);
    let is_mounted_with_current_options = mounted_widget
        .read()
        .as_ref()
        .is_some_and(|mounted| mounted.id == id && mounted.options == options);
    // The fallback renders as a sibling of the host div: clerk-js owns the
    // host element's children once mounted, so Dioxus must not manage nodes
    // inside it.
    rsx! {
        if !is_mounted_with_current_options {
            {fallback}
        }
        div { id: "{id}", ..attributes,
            WidgetLifecycle { id: id.clone(), widget, options, mounted_widget }
        }
    }
}

#[component]
fn WidgetLifecycle(
    id: String,
    widget: Widget,
    options: serde_json::Value,
    mounted_widget: Signal<Option<MountedWidget>>,
) -> Element {
    #[cfg(clerk_client)]
    {
        use crate::bridge::ClerkBridge;
        use crate::lifecycle::BridgeAction;

        let id_for_mount = id.clone();
        let id_for_guard = id.clone();
        let id_for_done = id.clone();
        let widget_local = widget;
        let options_for_guard = options.clone();
        let options_for_mount = options.clone();
        let mounted_widget_guard = mounted_widget;
        let mounted_widget_for_action = mounted_widget;
        let mounted_widget_done = mounted_widget;
        crate::lifecycle::use_loaded_bridge_action(
            (&id, &options),
            move || {
                mounted_widget_guard.read().as_ref().is_none_or(|mounted| {
                    mounted.id != id_for_guard || mounted.options != options_for_guard
                })
            },
            HOST_RETRY_MS,
            HOST_DEADLINE_MS,
            crate::core::ClerkError::Js(format!(
                "clerk widget host element #{id} did not appear within {HOST_DEADLINE_MS} ms; the widget cannot mount"
            )),
            move |bridge| {
                let Some(el) = host_element(&id_for_mount) else {
                    return Ok(BridgeAction::Deferred);
                };
                // Unmount the previously mounted element (which may differ
                // from `el` after an id change) before mounting fresh.
                if let Some(mounted) = mounted_widget_for_action.read().as_ref() {
                    widget_local.unmount(bridge, &mounted.element);
                }
                widget_local.mount(bridge, &el, &options_for_mount)?;
                Ok(BridgeAction::Done(MountedWidget {
                    id: id_for_done.clone(),
                    options: options_for_mount.clone(),
                    element: el,
                }))
            },
            move |mounted| {
                let mut mounted_widget = mounted_widget_done;
                mounted_widget.set(Some(mounted));
            },
        );

        // Unmount using the element the widget actually mounted into; the
        // `id` prop may have changed (or the element may already be detached)
        // since this drop closure was created.
        use_drop(move || {
            let Some(element) = mounted_widget
                .peek()
                .as_ref()
                .map(|mounted| mounted.element.clone())
            else {
                return;
            };
            let bridge = ClerkBridge::current();
            widget.unmount(&bridge, &element);
        });
    }
    #[cfg(not(clerk_client))]
    let _ = (&options, mounted_widget);
    rsx! {}
}

/// The `id` value carried by the attribute spread, if any, so a caller can set
/// the host id through the `GlobalAttributes` spread as well as the explicit
/// `id` prop. Later attributes win, matching how the DOM resolves duplicates.
fn spread_id(attributes: &[Attribute]) -> Option<String> {
    attributes.iter().rev().find_map(|attribute| {
        if attribute.name != "id" {
            return None;
        }
        match &attribute.value {
            AttributeValue::Text(value) => Some(value.clone()),
            _ => None,
        }
    })
}

#[cfg(clerk_client)]
fn host_element(id: &str) -> Option<web_sys::Element> {
    let window = web_sys::window()?;
    let doc = window.document()?;
    doc.get_element_by_id(id)
}

fn generated_host_id(widget: Widget) -> String {
    // Scope ids reset with each VirtualDom tree, unlike process-global counters
    // that drift between SSR requests and browser hydration.
    generated_host_id_from_scope(widget, dioxus::dioxus_core::current_scope_id())
}

fn generated_host_id_from_scope(widget: Widget, scope_id: dioxus::prelude::ScopeId) -> String {
    format!("{}-{}", widget.host_id_prefix(), scope_id.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::prelude::ScopeId;
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());
    static RECORDED_IDS: Mutex<Vec<String>> = Mutex::new(Vec::new());

    #[test]
    fn generated_host_id_is_stable_for_the_same_widget_scope() {
        let first = generated_host_id_from_scope(Widget::SignIn, ScopeId(42));
        let second = generated_host_id_from_scope(Widget::SignIn, ScopeId(42));

        assert_eq!(first, second);
        assert_eq!(first, "clerk-signin-42");
    }

    #[test]
    fn task_setup_mfa_has_stable_generated_host_id() {
        let id = generated_host_id_from_scope(Widget::TaskSetupMfa, ScopeId(42));
        assert_eq!(id, "clerk-tasksetupmfa-42");
    }

    #[test]
    fn spread_id_reads_text_id_attribute() {
        let attributes = vec![Attribute::new("id", "custom-host", None, false)];
        assert_eq!(spread_id(&attributes).as_deref(), Some("custom-host"));
    }

    #[test]
    fn spread_id_is_none_without_an_id_attribute() {
        let attributes = vec![Attribute::new("class", "mx-auto", None, false)];
        assert_eq!(spread_id(&attributes), None);
    }

    #[test]
    fn spread_id_prefers_the_last_id_attribute() {
        let attributes = vec![
            Attribute::new("id", "first", None, false),
            Attribute::new("id", "second", None, false),
        ];
        assert_eq!(spread_id(&attributes).as_deref(), Some("second"));
    }

    #[test]
    fn generated_host_id_distinguishes_widget_instances_by_scope() {
        assert_ne!(
            generated_host_id_from_scope(Widget::SignIn, ScopeId(42)),
            generated_host_id_from_scope(Widget::SignIn, ScopeId(43))
        );
    }

    #[test]
    fn generated_host_id_resets_with_each_virtual_dom_tree() {
        let _guard = TEST_LOCK.lock().unwrap();

        let first = generated_host_id_from_fresh_dom();
        let second = generated_host_id_from_fresh_dom();

        assert_eq!(first, second);
    }

    fn generated_host_id_from_fresh_dom() -> String {
        RECORDED_IDS.lock().unwrap().clear();
        let mut dom = VirtualDom::new(IdProbe);

        dom.rebuild_in_place();

        RECORDED_IDS
            .lock()
            .unwrap()
            .first()
            .cloned()
            .expect("IdProbe records a generated id")
    }

    #[component]
    fn IdProbe() -> Element {
        RECORDED_IDS
            .lock()
            .unwrap()
            .push(generated_host_id(Widget::SignIn));
        rsx! {}
    }
}

// The clerk-js mount/unmount method names are only compiled for the browser
// client, so they are verified in the wasm suite. The v6 `MFA` casing is
// load-bearing: a wrong case is a silent no-op mount.
#[cfg(all(test, clerk_client))]
mod client_tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn task_setup_mfa_uses_v6_method_names() {
        assert_eq!(Widget::TaskSetupMfa.mount_method(), "mountTaskSetupMFA");
        assert_eq!(Widget::TaskSetupMfa.unmount_method(), "unmountTaskSetupMFA");
    }
}
