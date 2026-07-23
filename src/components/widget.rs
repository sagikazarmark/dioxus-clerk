use dioxus::dioxus_core::AttributeValue;
use dioxus::prelude::*;
use std::rc::Rc;

#[cfg(clerk_client)]
const HOST_MOUNT_RETRY_MS: u32 = 16;
/// How long to wait for Dioxus to report the mounted host element instead of
/// polling forever when a renderer never supplies it.
#[cfg(clerk_client)]
const HOST_MOUNT_DEADLINE_MS: u32 = 10_000;

/// Mountable Clerk widget vocabulary shared by component rendering and the JS
/// bridge layer's clerk-js mount/unmount method tables.
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
    options: serde_json::Value,
    /// The DOM element clerk-js actually mounted into. Unmount paths must use
    /// this element even if the renderer later replaces or detaches it.
    #[cfg(clerk_client)]
    element: web_sys::Element,
}

impl PartialEq for MountedWidget {
    fn eq(&self, other: &Self) -> bool {
        self.options == other.options && {
            #[cfg(clerk_client)]
            {
                js_sys::Object::is(self.element.as_ref(), other.element.as_ref())
            }
            #[cfg(not(clerk_client))]
            {
                true
            }
        }
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
    // IDs are caller-owned DOM attributes, not lifecycle handles. Omitting an
    // ID avoids coupling SSR output to client scope allocation; Dioxus supplies
    // the actual mount element through `onmounted` below.
    let id = host.id.or_else(|| spread_id(&host.attributes));
    // The resolved id is rendered explicitly on the host `<div>` below, so drop
    // any `id` from the spread to keep a single authoritative `id` attribute.
    let mut attributes = host.attributes;
    attributes.retain(|attribute| attribute.name != "id");
    let mounted_widget = use_signal(|| None::<MountedWidget>);
    let host_element = use_signal(|| None::<Rc<MountedData>>);
    let is_mounted_with_current_options = mounted_widget
        .read()
        .as_ref()
        .is_some_and(|mounted| mounted.options == options);
    // The fallback renders as a sibling of the host div: clerk-js owns the
    // host element's children once mounted, so Dioxus must not manage nodes
    // inside it.
    rsx! {
        if !is_mounted_with_current_options {
            {fallback}
        }
        div {
            id,
            onmounted: move |event| {
                let mut host_element = host_element;
                host_element.set(Some(event.data()));
            },
            ..attributes,
        }
        // clerk-js owns every descendant of the host div. Keep Dioxus's empty
        // component placeholder outside that ownership boundary.
        WidgetLifecycle { widget, options, host_element, mounted_widget }
    }
}

#[component]
fn WidgetLifecycle(
    widget: Widget,
    options: serde_json::Value,
    host_element: Signal<Option<Rc<MountedData>>>,
    mounted_widget: Signal<Option<MountedWidget>>,
) -> Element {
    #[cfg(clerk_client)]
    {
        use crate::bridge::ClerkBridge;
        use crate::lifecycle::BridgeAction;

        let widget_local = widget;
        let options_for_guard = options.clone();
        let options_for_mount = options.clone();
        let host_element_guard = host_element;
        let host_element_for_action = host_element;
        let mounted_widget_guard = mounted_widget;
        let mounted_widget_for_action = mounted_widget;
        let mounted_widget_done = mounted_widget;
        crate::lifecycle::use_loaded_bridge_action(
            &options,
            move || {
                let Some(element) = mounted_host_element(host_element_guard) else {
                    return true;
                };
                mounted_widget_guard.read().as_ref().is_none_or(|mounted| {
                    mounted.options != options_for_guard
                        || !js_sys::Object::is(mounted.element.as_ref(), element.as_ref())
                })
            },
            HOST_MOUNT_RETRY_MS,
            HOST_MOUNT_DEADLINE_MS,
            crate::core::ClerkError::Js(format!(
                "clerk widget host element was not mounted within {HOST_MOUNT_DEADLINE_MS} ms; the widget cannot mount"
            )),
            move |bridge| {
                let Some(element) = mounted_host_element(host_element_for_action) else {
                    return Ok(BridgeAction::Deferred);
                };
                // Unmount the previously mounted element, which may differ
                // after a renderer replaces the host, before mounting fresh.
                let previously_mounted = { mounted_widget_for_action.read().as_ref().cloned() };
                if let Some(mounted) = previously_mounted {
                    widget_local.unmount(bridge, &mounted.element);
                    let mut mounted_widget = mounted_widget_for_action;
                    mounted_widget.set(None);
                }
                widget_local.mount(bridge, &element, &options_for_mount)?;
                Ok(BridgeAction::Done(MountedWidget {
                    options: options_for_mount.clone(),
                    element,
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
    let _ = (&options, host_element, mounted_widget);
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
fn mounted_host_element(host_element: Signal<Option<Rc<MountedData>>>) -> Option<web_sys::Element> {
    host_element
        .read()
        .as_ref()
        .and_then(|mounted| mounted.downcast::<web_sys::Element>())
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn widget_without_caller_id_omits_the_host_id_attribute() {
        let mut dom = VirtualDom::new(|| rsx! { crate::SignIn {} });
        let mutations = dom.rebuild_to_vec();

        assert!(!mutations.edits.iter().any(|edit| matches!(
            edit,
            dioxus::dioxus_core::Mutation::SetAttribute {
                name: "id",
                value,
                ..
            } if !matches!(value, AttributeValue::None)
        )));
    }

    #[test]
    fn widget_keeps_an_explicit_host_id() {
        let mut dom = VirtualDom::new(|| rsx! { crate::SignIn { id: "sign-in-host" } });
        let mutations = dom.rebuild_to_vec();

        assert!(mutations.edits.iter().any(|edit| matches!(
            edit,
            dioxus::dioxus_core::Mutation::SetAttribute {
                name: "id",
                value: AttributeValue::Text(value),
                ..
            } if value == "sign-in-host"
        )));
    }

    #[test]
    fn widget_keeps_a_spread_host_id() {
        let mut dom = VirtualDom::new(SpreadIdWidget);
        let mutations = dom.rebuild_to_vec();

        assert!(mutations.edits.iter().any(|edit| matches!(
            edit,
            dioxus::dioxus_core::Mutation::SetAttribute {
                name: "id",
                value: AttributeValue::Text(value),
                ..
            } if value == "spread-sign-in-host"
        )));
    }

    #[component]
    fn SpreadIdWidget() -> Element {
        render(
            Widget::SignIn,
            WidgetProps::new(
                serde_json::Value::Null,
                rsx! {},
                None,
                vec![Attribute::new("id", "spread-sign-in-host", None, false)],
            ),
        )
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
