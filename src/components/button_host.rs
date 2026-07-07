//! Shared hosting for the unstyled auth button components.
//!
//! The host owns the native `<button>` chrome, the disabled guard, and the
//! click ordering: the caller's `onclick` runs first, then the Clerk action
//! dispatches (matching Clerk React). Each button component supplies only its
//! option mapping and dispatch closure.

use dioxus::prelude::*;

/// Native `<button>` chrome shared by every auth button component.
pub(super) struct ButtonChrome {
    pub(super) attributes: Vec<Attribute>,
    pub(super) disabled: bool,
    pub(super) onclick: Callback<MouseEvent>,
    pub(super) children: Element,
}

/// Render an unstyled auth button. When enabled, a click calls the caller's
/// `onclick` first and then `on_activate`, which dispatches the Clerk action.
pub(super) fn button_host(chrome: ButtonChrome, on_activate: impl Fn() + 'static) -> Element {
    let ButtonChrome {
        mut attributes,
        disabled,
        onclick,
        children,
    } = chrome;

    // Default to type="button" so a button inside a form does not submit it;
    // an explicit `r#type` attribute wins.
    if !attributes.iter().any(|attribute| attribute.name == "type") {
        attributes.push(Attribute::new("type", "button", None, false));
    }

    // The explicit `disabled` prop is authoritative and drives the click guard
    // below, so drop any `disabled` from the attribute spread to avoid a
    // conflicting duplicate whose rendered state could disagree with the guard.
    attributes.retain(|attribute| attribute.name != "disabled");

    let on_click = move |event: MouseEvent| {
        if disabled {
            return;
        }
        onclick.call(event);
        on_activate();
    };

    rsx! {
        button {
            disabled,
            onclick: on_click,
            ..attributes,
            {children}
        }
    }
}

#[cfg(all(test, not(clerk_client)))]
mod tests {
    use crate::actions::ClerkOperation;
    use crate::components::{
        AuthButtonMode, ClerkProvider, SignInButton, SignOutButton, SignUpButton,
    };
    use crate::context::use_clerk_context;
    use dioxus::core::{Event, Mutation, NoOpMutations};
    use dioxus::html::{PlatformEventData, SerializedHtmlEventConverter, SerializedMouseData};
    use dioxus::prelude::*;
    use std::any::Any;
    use std::cell::RefCell;
    use std::rc::Rc;

    thread_local! {
        static QUEUE: RefCell<Vec<ClerkOperation>> = const { RefCell::new(Vec::new()) };
        static QUEUE_LEN_AT_ONCLICK: RefCell<Option<usize>> = const { RefCell::new(None) };
    }

    /// Mirrors the provider dispatch queue into the test after every render.
    #[component]
    fn QueueProbe() -> Element {
        let ctx = use_clerk_context();
        QUEUE.with(|queue| *queue.borrow_mut() = ctx.pending.read().clone());
        rsx! {
            div {}
        }
    }

    /// Build the app, click the (single) rendered button, and return the
    /// provider dispatch queue afterwards.
    fn click_and_collect(app: fn() -> Element) -> Vec<ClerkOperation> {
        dioxus::html::set_event_converter(Box::new(SerializedHtmlEventConverter));
        QUEUE.with(|queue| queue.borrow_mut().clear());
        QUEUE_LEN_AT_ONCLICK.with(|len| *len.borrow_mut() = None);

        let mut dom = VirtualDom::new(app);
        let mutations = dom.rebuild_to_vec();
        let button = mutations
            .edits
            .iter()
            .find_map(|mutation| match mutation {
                Mutation::NewEventListener { name, id } if name.contains("click") => Some(*id),
                _ => None,
            })
            .expect("rendered button registers a click listener");

        let data = Rc::new(PlatformEventData::new(Box::new(
            SerializedMouseData::default(),
        ))) as Rc<dyn Any>;
        dom.runtime()
            .handle_event("click", Event::new(data, true), button);
        dom.render_immediate(&mut NoOpMutations);

        QUEUE.with(|queue| queue.borrow().clone())
    }

    #[component]
    fn RedirectSignInApp() -> Element {
        rsx! {
            ClerkProvider { publishable_key: "pk_test_buttons",
                SignInButton { force_redirect_url: "/dashboard" }
                QueueProbe {}
            }
        }
    }

    #[test]
    fn sign_in_button_defaults_to_redirect_dispatch_with_mapped_options() {
        let queue = click_and_collect(RedirectSignInApp);

        assert_eq!(
            queue,
            vec![ClerkOperation::RedirectToSignIn(serde_json::json!({
                "forceRedirectUrl": "/dashboard",
            }))],
        );
    }

    #[component]
    fn ModalSignInApp() -> Element {
        rsx! {
            ClerkProvider { publishable_key: "pk_test_buttons",
                SignInButton { mode: AuthButtonMode::Modal }
                QueueProbe {}
            }
        }
    }

    #[test]
    fn sign_in_button_modal_mode_opens_the_modal() {
        let queue = click_and_collect(ModalSignInApp);

        assert_eq!(
            queue,
            vec![ClerkOperation::OpenSignIn(serde_json::Value::Null)],
        );
    }

    #[component]
    fn ModalSignUpApp() -> Element {
        rsx! {
            ClerkProvider { publishable_key: "pk_test_buttons",
                SignUpButton { mode: AuthButtonMode::Modal, sign_in_url: "/sign-in" }
                QueueProbe {}
            }
        }
    }

    #[test]
    fn sign_up_button_modal_mode_opens_the_modal_with_mapped_options() {
        let queue = click_and_collect(ModalSignUpApp);

        assert_eq!(
            queue,
            vec![ClerkOperation::OpenSignUp(serde_json::json!({
                "signInUrl": "/sign-in",
            }))],
        );
    }

    #[component]
    fn SignOutApp() -> Element {
        rsx! {
            ClerkProvider { publishable_key: "pk_test_buttons",
                SignOutButton { redirect_url: "/", session_id: "sess_1" }
                QueueProbe {}
            }
        }
    }

    #[test]
    fn sign_out_button_dispatches_sign_out_with_mapped_options() {
        let queue = click_and_collect(SignOutApp);

        assert_eq!(
            queue,
            vec![ClerkOperation::SignOut(serde_json::json!({
                "redirectUrl": "/",
                "sessionId": "sess_1",
            }))],
        );
    }

    #[component]
    fn DisabledApp() -> Element {
        rsx! {
            ClerkProvider { publishable_key: "pk_test_buttons",
                SignInButton {
                    disabled: true,
                    onclick: move |_| {
                        QUEUE_LEN_AT_ONCLICK.with(|len| *len.borrow_mut() = Some(usize::MAX));
                    },
                }
                QueueProbe {}
            }
        }
    }

    #[test]
    fn disabled_button_neither_dispatches_nor_calls_onclick() {
        let queue = click_and_collect(DisabledApp);

        assert!(queue.is_empty());
        QUEUE_LEN_AT_ONCLICK.with(|len| assert_eq!(*len.borrow(), None));
    }

    #[component]
    fn OrderingApp() -> Element {
        let ctx = use_clerk_context();
        rsx! {
            SignInButton {
                onclick: move |_| {
                    let queued = ctx.pending.peek().len();
                    QUEUE_LEN_AT_ONCLICK.with(|len| *len.borrow_mut() = Some(queued));
                },
            }
        }
    }

    #[component]
    fn OrderingRoot() -> Element {
        rsx! {
            ClerkProvider { publishable_key: "pk_test_buttons",
                OrderingApp {}
                QueueProbe {}
            }
        }
    }

    #[test]
    fn caller_onclick_runs_before_the_clerk_dispatch() {
        let queue = click_and_collect(OrderingRoot);

        assert_eq!(queue.len(), 1);
        QUEUE_LEN_AT_ONCLICK.with(|len| {
            assert_eq!(*len.borrow(), Some(0), "onclick saw an empty queue");
        });
    }
}
