//! Concrete JS bridge layer for clerk-js browser interop.

use crate::actions::ClerkOperation;
use crate::core::{AuthObservation, ClerkError, ReverificationLevel, SessionStatus};
use crate::handle::ClerkHandle;
use crate::handle::ClerkListener;
use crate::reverification::ReverificationOutcome;
use dioxus::prelude::Callback;
use serde::Serialize;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::prelude::*;

thread_local! {
    /// Page-scoped slots for the router callbacks handed to `Clerk.load(...)`.
    ///
    /// clerk-js keeps the wrapped `routerPush`/`routerReplace` JS closures for
    /// the life of the page, while provider instances come and go (route-layout
    /// remounts) and a Dioxus `Callback` dies with its owning scope — calling
    /// one after its provider unmounts aborts the whole wasm app. The JS
    /// closures therefore read through these page-scoped slots: every provider
    /// mount re-populates them (so the closures installed by the first
    /// `Clerk.load()` keep working after a remount, which skips the second
    /// load), and unmount clears them so a late clerk-js navigation is a no-op
    /// instead of a call into a dropped `Callback`.
    static ROUTER_PUSH: Cell<Option<Callback<String>>> = const { Cell::new(None) };
    static ROUTER_REPLACE: Cell<Option<Callback<String>>> = const { Cell::new(None) };
    /// Which `set_router_callbacks` call currently owns the slots. Dioxus may
    /// mount a replacement provider before dropping the old one, so an
    /// unmounting provider must only clear the slots it still owns — otherwise
    /// its late `use_drop` would wipe the fresh provider's callbacks and
    /// clerk-js navigation would silently no-op for the rest of the page.
    static ROUTER_SLOT_OWNER: Cell<u64> = const { Cell::new(0) };
    /// True while a `Clerk.load()` started by this crate has not settled.
    /// Page-scoped like the router slots: a provider unmount cancels the
    /// awaiting Rust task, but the underlying JS promise keeps running, and
    /// clerk-js does not support a second `Clerk.load()`. Cleared by JS
    /// promise callbacks, so it tracks the real load regardless of which
    /// provider instances come and go.
    static LOAD_IN_FLIGHT: Cell<bool> = const { Cell::new(false) };
    /// The failure message from the most recent `Clerk.load()` that settled
    /// with a rejection on this page's clerk-js singleton. Set by the load
    /// promise's reject callback and cleared on a successful load, so it
    /// tracks the real outcome across provider remounts. The lifecycle reads it
    /// to avoid a second `Clerk.load()` on a live instance whose first load
    /// already failed — clerk-js does not support re-loading a live singleton,
    /// so that failure is terminal until a fresh clerk-js is injected (only the
    /// script-tag-failure path re-injects, and it never reaches `Clerk.load()`,
    /// so it never sets this marker).
    static LOAD_FAILED: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Whether a `Clerk.load()` started by this crate is still in flight.
pub(crate) fn load_in_flight() -> bool {
    LOAD_IN_FLIGHT.with(Cell::get)
}

/// The recorded failure message if the current singleton's `Clerk.load()`
/// settled with a rejection, else `None`.
pub(crate) fn load_failure() -> Option<String> {
    LOAD_FAILED.with(|slot| slot.borrow().clone())
}

/// Records a terminal `Clerk.load()` failure that the promise itself never
/// reported — specifically the lifecycle's settle-deadline firing while the JS
/// promise is still pending. Dropping the Rust future does not cancel the JS
/// promise, so without this marker the in-flight flag stays set forever and
/// every provider remount pays another full settle timeout before erroring.
/// Clears the in-flight flag so remounts short-circuit on [`load_failure`]
/// instead of re-waiting; a later successful settle still clears the marker.
pub(crate) fn mark_load_timed_out(message: String) {
    LOAD_IN_FLIGHT.with(|flag| flag.set(false));
    LOAD_FAILED.with(|slot| *slot.borrow_mut() = Some(message));
}

/// Reset the page-scoped `Clerk.load()` state. Only for the wasm test harness:
/// a mock whose load promise never settles leaves `LOAD_IN_FLIGHT` set, which
/// would make the next test's provider block in the in-flight wait loop.
pub(crate) fn reset_load_state() {
    LOAD_IN_FLIGHT.with(|flag| flag.set(false));
    LOAD_FAILED.with(|slot| *slot.borrow_mut() = None);
}

/// Slot-ownership token returned by [`set_router_callbacks`]; pass it back to
/// [`clear_router_callbacks`] on unmount.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct RouterSlotToken(u64);

/// Point the page-scoped router slots at the current provider's callbacks,
/// taking ownership of the slots.
pub(crate) fn set_router_callbacks(
    push: Option<Callback<String>>,
    replace: Option<Callback<String>>,
) -> RouterSlotToken {
    let token = ROUTER_SLOT_OWNER.with(|owner| {
        let next = owner.get().wrapping_add(1);
        owner.set(next);
        next
    });
    ROUTER_PUSH.with(|slot| slot.set(push));
    ROUTER_REPLACE.with(|slot| slot.set(replace));
    RouterSlotToken(token)
}

/// Disconnect clerk-js from the scope-owned callbacks — unless a newer
/// provider has already taken over the slots. Idempotent.
pub(crate) fn clear_router_callbacks(token: RouterSlotToken) {
    ROUTER_SLOT_OWNER.with(|owner| {
        if owner.get() == token.0 {
            ROUTER_PUSH.with(|slot| slot.set(None));
            ROUTER_REPLACE.with(|slot| slot.set(None));
        }
    });
}

fn read_router_push() -> Option<Callback<String>> {
    ROUTER_PUSH.with(Cell::get)
}

fn read_router_replace() -> Option<Callback<String>> {
    ROUTER_REPLACE.with(Cell::get)
}

/// Fresh bridge over the current `window.Clerk` value.
#[derive(Clone)]
pub(crate) struct ClerkBridge {
    handle: ClerkHandle,
}

impl ClerkBridge {
    /// Snapshot the current browser-global Clerk value.
    pub(crate) fn current() -> Self {
        Self {
            handle: ClerkHandle::current(),
        }
    }

    /// Whether clerk-js has created `window.Clerk`.
    pub(crate) fn is_present(&self) -> bool {
        self.handle.is_present()
    }

    /// Whether clerk-js reports a previous `Clerk.load()` already resolved.
    pub(crate) fn is_loaded(&self) -> bool {
        self.handle.is_loaded_js()
    }

    /// Whether clerk-js reports an active session.
    pub(crate) fn is_signed_in(&self) -> bool {
        self.handle.is_signed_in()
    }

    /// Call `Clerk.load(...)`, converting JSON options and the page-scoped
    /// router callbacks into the JS shapes clerk-js expects.
    ///
    /// The in-flight marker is set for the lifetime of the JS promise, not
    /// the Rust future: settlement callbacks clear it even when the awaiting
    /// provider task was cancelled mid-load.
    pub(crate) async fn load(&self, options: &serde_json::Value) -> Result<(), ClerkError> {
        let promise = self.handle.load_promise(load_options_js(options))?;

        LOAD_IN_FLIGHT.with(|flag| flag.set(true));
        let on_resolve = Closure::<dyn FnMut(JsValue)>::new(move |_value: JsValue| {
            LOAD_IN_FLIGHT.with(|flag| flag.set(false));
            LOAD_FAILED.with(|slot| *slot.borrow_mut() = None);
        });
        let on_reject = Closure::<dyn FnMut(JsValue)>::new(move |value: JsValue| {
            LOAD_IN_FLIGHT.with(|flag| flag.set(false));
            // Record the failure so a remounted provider surfaces it instead of
            // issuing a second Clerk.load() on this already-failed singleton.
            let message = crate::handle::js_error_message(&value);
            LOAD_FAILED.with(|slot| *slot.borrow_mut() = Some(message));
        });
        // `then2` here borrows `&Closure<dyn FnMut(JsValue)>` (not `&Function`),
        // so the closures must be `Closure::new` and outlive this Rust scope:
        // the JS promise settles them after the awaiting future may be gone.
        // `forget` leaks two closures per Clerk.load() (once-per-page), each
        // firing at most once — accepted over reworking the settle wiring.
        let _ = promise.then2(&on_resolve, &on_reject);
        on_resolve.forget();
        on_reject.forget();

        wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map(|_| ())
            .map_err(crate::handle::js_error)
    }

    /// Call `Clerk.session.getToken(opts?)`.
    pub(crate) async fn get_token(
        &self,
        options: &serde_json::Value,
    ) -> Result<Option<String>, ClerkError> {
        self.handle.get_token(js_options(options)).await
    }

    /// Open clerk-js's step-up reverification UI and await the user's outcome.
    ///
    /// Delegates to [`ClerkHandle::open_reverification`]; the reverification
    /// hook calls this after Clerk lifecycle loadedness.
    pub(crate) async fn open_reverification(
        &self,
        level: Option<ReverificationLevel>,
    ) -> Result<ReverificationOutcome, ClerkError> {
        self.handle.open_reverification(level).await
    }

    /// Execute one Clerk action dispatch operation against clerk-js.
    ///
    /// This is the single place mapping the operation vocabulary to JS calls;
    /// dispatch ordering and lifecycle timing stay with `crate::actions`.
    pub(crate) async fn run(&self, operation: ClerkOperation) -> Result<(), ClerkError> {
        match operation {
            ClerkOperation::OpenSignIn(options) => self.handle.open_sign_in(&js_options(&options)),
            ClerkOperation::CloseSignIn => self.handle.close_sign_in(),
            ClerkOperation::OpenSignUp(options) => self.handle.open_sign_up(&js_options(&options)),
            ClerkOperation::CloseSignUp => self.handle.close_sign_up(),
            ClerkOperation::OpenUserProfile(options) => {
                self.handle.open_user_profile(&js_options(&options))
            }
            ClerkOperation::CloseUserProfile => self.handle.close_user_profile(),
            ClerkOperation::SignOut(options) => self.handle.sign_out(js_options(&options)).await,
            ClerkOperation::RedirectToSignIn(options) => self.redirect_to_sign_in(&options).await,
            ClerkOperation::RedirectToSignUp(options) => self.redirect_to_sign_up(&options).await,
        }
    }

    /// Convert current Clerk user/session fields into an Auth state observation.
    pub(crate) fn observation(&self) -> AuthObservation {
        if self.handle.is_signed_in() {
            return match (self.handle.user(), self.handle.session()) {
                (Some(user), Some(session)) => AuthObservation::SignedIn { user, session },
                _ => AuthObservation::Loading,
            };
        }

        // `Clerk.isSignedIn` is `false` for a pending session (it bakes in
        // clerk-js's `treatPendingAsSignedOut` default), yet `Clerk.session`
        // still holds that pending session. Surface it as `Pending` so apps can
        // route on `current_task`; a genuinely signed-out client has no session.
        match (self.handle.user(), self.handle.session()) {
            (Some(user), Some(session)) if session.status == SessionStatus::Pending => {
                AuthObservation::Pending { user, session }
            }
            _ => AuthObservation::SignedOut,
        }
    }

    /// Subscribe to Clerk changes and emit converted observations.
    pub(crate) fn subscribe(
        &self,
        on_observation: impl FnMut(AuthObservation) + 'static,
    ) -> Option<ListenerSubscription> {
        let on_observation = Rc::new(RefCell::new(on_observation));
        // Defense-in-depth: a clerk event delivered while the callback borrow
        // is held is deferred and re-read once the outer callback returns,
        // instead of being silently dropped. (Direct reentrancy of the wasm
        // closure is already rejected by wasm-bindgen before reaching here.)
        let redispatch = Rc::new(Cell::new(false));
        self.handle
            .add_listener(move |_| match on_observation.try_borrow_mut() {
                Ok(mut callback) => {
                    callback(ClerkBridge::current().observation());
                    drop(callback);
                    while redispatch.take() {
                        let Ok(mut callback) = on_observation.try_borrow_mut() else {
                            redispatch.set(true);
                            break;
                        };
                        callback(ClerkBridge::current().observation());
                    }
                }
                Err(_) => redispatch.set(true),
            })
            .map(|listener| ListenerSubscription {
                _listener: listener,
            })
    }

    /// Redirect to Clerk sign-in using clerk-js option conversion.
    ///
    /// Named alongside [`ClerkBridge::run`] because it has a second caller
    /// shape: the lifecycle's one-shot redirect effects run it outside the
    /// dispatch queue. Awaits clerk-js's navigation promise so a rejection
    /// reaches the caller instead of being dropped.
    pub(crate) async fn redirect_to_sign_in(
        &self,
        options: &serde_json::Value,
    ) -> Result<(), ClerkError> {
        self.handle.redirect_to_sign_in(js_options(options)).await
    }

    /// Redirect to Clerk sign-up using clerk-js option conversion.
    ///
    /// Named alongside [`ClerkBridge::run`] because it has a second caller
    /// shape: the lifecycle's one-shot redirect effects run it outside the
    /// dispatch queue. Awaits clerk-js's navigation promise so a rejection
    /// reaches the caller instead of being dropped.
    pub(crate) async fn redirect_to_sign_up(
        &self,
        options: &serde_json::Value,
    ) -> Result<(), ClerkError> {
        self.handle.redirect_to_sign_up(js_options(options)).await
    }

    pub(crate) fn mount_widget(
        &self,
        widget: crate::components::widget::Widget,
        element: &web_sys::Element,
        options: &serde_json::Value,
    ) -> Result<(), ClerkError> {
        let opts = js_options(options);
        self.handle
            .try_call_method2(widget.mount_method(), element.as_ref(), &opts)
    }

    pub(crate) fn unmount_widget(
        &self,
        widget: crate::components::widget::Widget,
        element: &web_sys::Element,
    ) {
        self.handle
            .call_method1(widget.unmount_method(), element.as_ref());
    }
}

/// Listener subscription whose closure must stay alive while subscribed.
/// Dropping it unsubscribes from clerk-js and releases the closure.
pub(crate) struct ListenerSubscription {
    _listener: ClerkListener,
}

/// Convert JSON options to the shape clerk-js expects.
pub(crate) fn js_options(options: &serde_json::Value) -> JsValue {
    if options.is_null() {
        return JsValue::UNDEFINED;
    }
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    options.serialize(&serializer).unwrap_or_else(|error| {
        // Practically unreachable for serde_json::Value input, but dropping
        // the options must at least be visible in the console.
        web_sys::console::warn_1(&JsValue::from_str(&format!(
            "dioxus-clerk: failed to convert Clerk options to JS, passing undefined: {error}"
        )));
        JsValue::UNDEFINED
    })
}

/// Convert `Clerk.load(...)` options, wrapping router callbacks as JS
/// functions clerk-js can call for SPA-native navigation.
///
/// The JS closures read the page-scoped router slots at call time, so a
/// remounted provider's callbacks keep working through the closures installed
/// by the first (and only) `Clerk.load()` on this page.
fn load_options_js(options: &serde_json::Value) -> JsValue {
    let js_options = js_options(options);
    // clerk-js 6 ships UI components in the separate `@clerk/ui` bundle; hand
    // its constructor to `Clerk.load({ ui: { ClerkUI } })` or every `mountX`
    // throws "Clerk was not loaded with UI components". Absent when clerk-js is
    // loaded externally without the UI bundle, in which case we pass nothing.
    let ui_ctor = crate::bindings::clerk_ui_ctor();
    if ui_ctor.is_none() && read_router_push().is_none() && read_router_replace().is_none() {
        return js_options;
    }

    let object = if js_options.is_null() || js_options.is_undefined() || !js_options.is_object() {
        js_sys::Object::new().into()
    } else {
        js_options
    };

    if let Some(ctor) = ui_ctor {
        // Merge `ClerkUI` into any caller-supplied `ui` object (passed through
        // the raw `options` escape hatch) instead of replacing it.
        let ui = js_sys::Reflect::get(&object, &JsValue::from_str("ui"))
            .ok()
            .filter(|value| value.is_object())
            .unwrap_or_else(|| js_sys::Object::new().into());
        let _ = js_sys::Reflect::set(&ui, &JsValue::from_str("ClerkUI"), ctor.as_ref());
        let _ = js_sys::Reflect::set(&object, &JsValue::from_str("ui"), &ui);
    }

    if read_router_push().is_some() {
        set_router_callback(&object, "routerPush", read_router_push);
    }
    if read_router_replace().is_some() {
        set_router_callback(&object, "routerReplace", read_router_replace);
    }

    object
}

fn set_router_callback(target: &JsValue, key: &str, slot: fn() -> Option<Callback<String>>) {
    let closure = Closure::<dyn Fn(JsValue)>::wrap(Box::new(move |to: JsValue| {
        // The slot is cleared on provider unmount; a late clerk-js navigation
        // must not call into a dropped scope-owned Callback.
        let Some(callback) = slot() else {
            return;
        };
        if let Some(to) = to.as_string() {
            callback.call(to);
        }
    }));
    let _ = js_sys::Reflect::set(target, &JsValue::from_str(key), closure.as_ref());
    closure.forget();
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use js_sys::{Function, Object, Reflect};
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn window() -> web_sys::Window {
        web_sys::window().expect("wasm tests run in a browser window")
    }

    fn set_prop(target: &JsValue, key: &str, value: &JsValue) {
        Reflect::set(target, &JsValue::from_str(key), value).unwrap();
    }

    fn get_prop(target: &JsValue, key: &str) -> JsValue {
        Reflect::get(target, &JsValue::from_str(key)).unwrap()
    }

    fn number_prop(target: &JsValue, key: &str) -> f64 {
        get_prop(target, key).as_f64().unwrap_or(0.0)
    }

    fn clear_clerk() {
        let key = JsValue::from_str("Clerk");
        let _ = Reflect::delete_property(window().as_ref(), &key);
        Reflect::set(window().as_ref(), &key, &JsValue::UNDEFINED).unwrap();
    }

    fn install_listener_clerk() -> JsValue {
        let clerk = Object::new();
        let user = Object::new();
        let session = Object::new();

        set_prop(user.as_ref(), "id", &JsValue::from_str("user_2abc"));
        set_prop(session.as_ref(), "id", &JsValue::from_str("sess_2def"));
        set_prop(session.as_ref(), "status", &JsValue::from_str("active"));
        set_prop(clerk.as_ref(), "isSignedIn", &JsValue::FALSE);
        set_prop(clerk.as_ref(), "user", user.as_ref());
        set_prop(clerk.as_ref(), "session", session.as_ref());
        // clerk_singleton requires a Clerk-shaped global: `load` must exist.
        set_prop(
            clerk.as_ref(),
            "load",
            Function::new_no_args("return Promise.resolve();").as_ref(),
        );
        set_prop(
            clerk.as_ref(),
            "addListener",
            Function::new_no_args(
                r#"
                var clerk = this;
                this.lastListener = arguments[0];
                return function unsubscribe() {
                    clerk.unsubscribeCallCount = (clerk.unsubscribeCallCount || 0) + 1;
                    clerk.lastListener = undefined;
                };
                "#,
            )
            .as_ref(),
        );

        let clerk: JsValue = clerk.into();
        set_prop(window().as_ref(), "Clerk", &clerk);
        clerk
    }

    #[wasm_bindgen_test]
    fn bridge_subscription_drop_unsubscribes_listener() {
        clear_clerk();
        let clerk = install_listener_clerk();

        let bridge = ClerkBridge::current();
        let subscription = bridge
            .subscribe(|_| {})
            .expect("mock Clerk returns unsubscribe function");

        assert_eq!(number_prop(&clerk, "unsubscribeCallCount"), 0.0);

        drop(subscription);

        assert_eq!(number_prop(&clerk, "unsubscribeCallCount"), 1.0);
    }

    #[wasm_bindgen_test]
    fn stale_clear_does_not_wipe_newer_router_slot_owner() {
        use dioxus::prelude::*;

        // Remount with mount-before-drop ordering: provider B takes the slots
        // before provider A's use_drop runs. A's late clear must be a no-op.
        // Callbacks need a live runtime, so the dance runs inside a VirtualDom.
        fn app() -> Element {
            let token_a = set_router_callbacks(Some(Callback::new(|_: String| {})), None);
            let token_b = set_router_callbacks(Some(Callback::new(|_: String| {})), None);

            clear_router_callbacks(token_a);
            assert!(
                read_router_push().is_some(),
                "stale unmount cleared the new provider's router callbacks"
            );

            clear_router_callbacks(token_b);
            assert!(read_router_push().is_none());
            rsx! {}
        }

        let mut dom = VirtualDom::new(app);
        dom.rebuild_in_place();
    }

    #[wasm_bindgen_test(async)]
    async fn load_in_flight_flag_survives_rust_task_cancellation() {
        use futures_util::FutureExt;

        clear_clerk();
        let clerk = Object::new();
        // A load whose settlement the test controls via `clerk.resolveLoad`.
        set_prop(
            clerk.as_ref(),
            "load",
            Function::new_no_args(
                r#"
                var clerk = this;
                return new Promise(function (resolve) { clerk.resolveLoad = resolve; });
                "#,
            )
            .as_ref(),
        );
        let clerk: JsValue = clerk.into();
        set_prop(window().as_ref(), "Clerk", &clerk);

        assert!(!load_in_flight());

        // Poll the load once (sets the flag, attaches settle callbacks), then
        // drop it — the provider-unmount cancellation case.
        assert!(
            ClerkBridge::current()
                .load(&serde_json::Value::Null)
                .now_or_never()
                .is_none()
        );
        assert!(
            load_in_flight(),
            "cancelling the Rust task must not clear the in-flight marker"
        );

        // Settling the JS promise clears the marker even with no Rust waiter.
        let resolve: Function = get_prop(&clerk, "resolveLoad").unchecked_into();
        resolve.call0(&JsValue::UNDEFINED).unwrap();
        gloo_timers::future::TimeoutFuture::new(10).await;
        assert!(!load_in_flight());

        clear_clerk();
    }

    #[wasm_bindgen_test]
    fn observation_reports_pending_for_pending_session() {
        use crate::core::{AuthObservation, SessionStatus, SessionTaskKey};

        clear_clerk();
        let clerk = Object::new();
        let user = Object::new();
        let session = Object::new();
        set_prop(user.as_ref(), "id", &JsValue::from_str("user_2abc"));
        set_prop(session.as_ref(), "id", &JsValue::from_str("sess_2def"));
        set_prop(session.as_ref(), "status", &JsValue::from_str("pending"));
        let task = Object::new();
        set_prop(task.as_ref(), "key", &JsValue::from_str("setup-mfa"));
        set_prop(session.as_ref(), "currentTask", task.as_ref());
        // clerk-js reports a pending session as not-signed-in.
        set_prop(clerk.as_ref(), "isSignedIn", &JsValue::FALSE);
        set_prop(clerk.as_ref(), "user", user.as_ref());
        set_prop(clerk.as_ref(), "session", session.as_ref());
        set_prop(
            clerk.as_ref(),
            "load",
            Function::new_no_args("return Promise.resolve();").as_ref(),
        );
        let clerk: JsValue = clerk.into();
        set_prop(window().as_ref(), "Clerk", &clerk);

        match ClerkBridge::current().observation() {
            AuthObservation::Pending { user, session } => {
                assert_eq!(user.id, "user_2abc");
                assert_eq!(session.status, SessionStatus::Pending);
                assert_eq!(
                    session.current_task.map(|task| task.key),
                    Some(SessionTaskKey::SetupMfa)
                );
            }
            other => panic!("expected a Pending observation, got {other:?}"),
        }

        clear_clerk();
    }

    #[wasm_bindgen_test]
    fn observation_reports_signed_out_without_a_session() {
        use crate::core::AuthObservation;

        clear_clerk();
        let clerk = Object::new();
        set_prop(clerk.as_ref(), "isSignedIn", &JsValue::FALSE);
        set_prop(
            clerk.as_ref(),
            "load",
            Function::new_no_args("return Promise.resolve();").as_ref(),
        );
        let clerk: JsValue = clerk.into();
        set_prop(window().as_ref(), "Clerk", &clerk);

        assert!(matches!(
            ClerkBridge::current().observation(),
            AuthObservation::SignedOut
        ));

        clear_clerk();
    }

    // Install a fake Clerk whose `__internal_openReverification` records the
    // props it received and settles asynchronously (a microtask) by calling the
    // named outcome callback — mirroring how real clerk-js drives the UI.
    fn install_reverification_clerk(outcome_callback: &str) -> JsValue {
        let clerk = Object::new();
        set_prop(
            clerk.as_ref(),
            "load",
            Function::new_no_args("return Promise.resolve();").as_ref(),
        );
        let body = format!(
            r#"
            this.lastReverificationProps = arguments[0];
            var props = arguments[0];
            Promise.resolve().then(function () {{ props.{outcome_callback}(); }});
            "#
        );
        set_prop(
            clerk.as_ref(),
            "__internal_openReverification",
            Function::new_no_args(&body).as_ref(),
        );
        let clerk: JsValue = clerk.into();
        set_prop(window().as_ref(), "Clerk", &clerk);
        clerk
    }

    #[wasm_bindgen_test(async)]
    async fn open_reverification_resolves_completed_and_forwards_level() {
        use crate::core::ReverificationLevel;

        clear_clerk();
        let clerk = install_reverification_clerk("afterVerification");

        let outcome = ClerkBridge::current()
            .open_reverification(Some(ReverificationLevel::SecondFactor))
            .await
            .expect("reverification prompt resolves");

        assert_eq!(outcome, ReverificationOutcome::Completed);

        // clerk-js received the required level as its raw string.
        let props = get_prop(&clerk, "lastReverificationProps");
        assert_eq!(
            get_prop(&props, "level").as_string().as_deref(),
            Some("second_factor")
        );

        clear_clerk();
    }

    #[wasm_bindgen_test(async)]
    async fn open_reverification_resolves_cancelled_when_user_dismisses() {
        clear_clerk();
        install_reverification_clerk("afterVerificationCancelled");

        let outcome = ClerkBridge::current()
            .open_reverification(None)
            .await
            .expect("reverification prompt resolves");

        assert_eq!(outcome, ReverificationOutcome::Cancelled);

        clear_clerk();
    }

    #[wasm_bindgen_test(async)]
    async fn open_reverification_without_clerk_is_not_loaded() {
        clear_clerk();

        let err = ClerkBridge::current()
            .open_reverification(None)
            .await
            .expect_err("no clerk-js singleton");

        assert!(matches!(err, ClerkError::NotLoaded));
    }

    #[wasm_bindgen_test]
    fn bridge_mount_sign_in_returns_not_loaded_when_clerk_is_missing() {
        clear_clerk();
        let element = window().document().unwrap().create_element("div").unwrap();

        let err = ClerkBridge::current()
            .mount_widget(
                crate::components::widget::Widget::SignIn,
                &element,
                &serde_json::Value::Null,
            )
            .expect_err("missing Clerk should be visible to mounted UI");

        assert!(matches!(err, ClerkError::NotLoaded));
    }

    fn set_clerk_ui_ctor() {
        let key = JsValue::from_str("__internal_ClerkUICtor");
        Reflect::set(
            window().as_ref(),
            &key,
            Function::new_no_args("return null;").as_ref(),
        )
        .unwrap();
    }

    fn clear_clerk_ui_ctor() {
        let key = JsValue::from_str("__internal_ClerkUICtor");
        let _ = Reflect::delete_property(window().as_ref(), &key);
        Reflect::set(window().as_ref(), &key, &JsValue::UNDEFINED).unwrap();
    }

    // clerk-js 6 needs the `@clerk/ui` constructor handed to `Clerk.load()`.
    #[wasm_bindgen_test]
    fn load_options_js_passes_clerk_ui_ctor_when_present() {
        set_clerk_ui_ctor();

        let options = load_options_js(&serde_json::Value::Null);
        let ui = get_prop(&options, "ui");
        assert!(ui.is_object(), "load options must carry a `ui` object");
        assert!(
            get_prop(&ui, "ClerkUI").is_function(),
            "`ui.ClerkUI` must be the @clerk/ui constructor"
        );

        clear_clerk_ui_ctor();
    }

    // A caller passing their own `ui` object through the raw `options` escape
    // hatch keeps it; ClerkUI is merged in rather than overwriting it.
    #[wasm_bindgen_test]
    fn load_options_js_merges_clerk_ui_into_caller_supplied_ui() {
        set_clerk_ui_ctor();

        let options = load_options_js(&serde_json::json!({ "ui": { "custom": "kept" } }));
        let ui = get_prop(&options, "ui");
        assert_eq!(
            get_prop(&ui, "custom").as_string().as_deref(),
            Some("kept"),
            "caller-supplied `ui` fields must be preserved"
        );
        assert!(get_prop(&ui, "ClerkUI").is_function());

        clear_clerk_ui_ctor();
    }

    // Loaded externally without the UI bundle: no ctor, so no `ui` key — and
    // the real options still pass through untouched.
    #[wasm_bindgen_test]
    fn load_options_js_omits_ui_without_ctor() {
        clear_clerk_ui_ctor();

        let options = load_options_js(&serde_json::json!({ "signInUrl": "/si" }));
        assert!(
            get_prop(&options, "ui").is_undefined(),
            "no @clerk/ui ctor -> no `ui` key in load options"
        );
        assert_eq!(
            get_prop(&options, "signInUrl").as_string().as_deref(),
            Some("/si")
        );
    }
}
