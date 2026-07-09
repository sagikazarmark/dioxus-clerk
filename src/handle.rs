//! Friendly wrapper around the wasm-bindgen `Clerk` extern type.

use crate::bindings::{Clerk, clerk_singleton};
use crate::core::{ClerkError, ReverificationLevel, Session, SessionTask, User};
use crate::reverification::ReverificationOutcome;
use js_sys::{Array, Date, Function, Promise, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// Safety-net deadline for a step-up reverification prompt.
///
/// Reverification is user-driven (the user enters a fresh authentication
/// factor), so this is deliberately generous: it is not a UX cutoff. It exists
/// only to bound the pathological case where clerk-js tears the prompt down
/// without ever firing `afterVerification`/`afterVerificationCancelled`, which
/// would otherwise leave the awaiting future pending forever. Every other
/// awaited clerk-js interaction in this crate is bounded; this keeps the
/// reverification path from being the one exception.
const REVERIFICATION_SETTLE_TIMEOUT_MS: u32 = 10 * 60 * 1000;

/// Private raw adapter around the global `Clerk` singleton.
///
/// This adapter snapshots the current JS value and performs raw calls. Every
/// fallible method guards uniformly with `ClerkError::NotLoaded` when the
/// singleton is absent; `ClerkBridge` does not re-check presence.
#[derive(Clone)]
pub(crate) struct ClerkHandle {
    inner: Option<Clerk>,
}

impl ClerkHandle {
    /// Snapshot the current global state. Re-call to re-snapshot.
    pub(crate) fn current() -> Self {
        Self {
            inner: clerk_singleton(),
        }
    }

    /// Whether the global `Clerk` singleton has been instantiated. Cheaper
    /// than calling into JS loadedness, and answers a different question:
    /// "has clerk-js executed yet?" (vs. "has Clerk.load() resolved?").
    pub(crate) fn is_present(&self) -> bool {
        self.inner.is_some()
    }

    /// Whether there is an active session.
    ///
    /// Reads `Clerk.isSignedIn` dynamically because older or foreign clerk-js
    /// releases (reachable via a custom `clerk_js_url`) may not define it;
    /// those fall back to `Clerk.user != null`.
    pub(crate) fn is_signed_in(&self) -> bool {
        let Some(c) = self.inner.as_ref() else {
            return false;
        };
        match prop(c.as_ref(), "isSignedIn").and_then(|value| value.as_bool()) {
            Some(is_signed_in) => is_signed_in,
            None => prop(c.as_ref(), "user").is_some(),
        }
    }

    /// Read the current `User`, if any. A throwing getter (a foreign or older
    /// `window.Clerk`) reads as no user.
    pub(crate) fn user(&self) -> Option<User> {
        let raw = self.inner.as_ref()?.user().ok()?;
        if raw.is_null() || raw.is_undefined() {
            return None;
        }
        user_from_js(raw)
    }

    /// Read the current `Session`, if any. A throwing getter (a foreign or
    /// older `window.Clerk`) reads as no session.
    pub(crate) fn session(&self) -> Option<Session> {
        let raw = self.inner.as_ref()?.session().ok()?;
        if raw.is_null() || raw.is_undefined() {
            return None;
        }
        session_from_js(raw)
    }

    /// Start `Clerk.load(opts)` and return the raw promise, so callers can
    /// observe settlement independently of the awaiting Rust task (which may
    /// be cancelled by a provider unmount while the JS load keeps running).
    pub(crate) fn load_promise(&self, opts: JsValue) -> Result<js_sys::Promise, ClerkError> {
        let c = self.inner.as_ref().ok_or(ClerkError::NotLoaded)?;
        c.load(&opts).map_err(js_error)
    }

    /// Whether clerk-js reports `Clerk.loaded === true`, i.e. a previous
    /// `Clerk.load()` already resolved on this page.
    pub(crate) fn is_loaded_js(&self) -> bool {
        self.inner
            .as_ref()
            .and_then(|c| prop(c.as_ref(), "loaded"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    }

    /// Awaitable `Clerk.signOut(opts?)`.
    pub(crate) async fn sign_out(&self, opts: JsValue) -> Result<(), ClerkError> {
        let c = self.inner.as_ref().ok_or(ClerkError::NotLoaded)?;
        JsFuture::from(c.sign_out(&opts).map_err(js_error)?)
            .await
            .map(|_| ())
            .map_err(js_error)
    }

    /// Awaitable `Clerk.redirectToSignIn(opts?)`. Pass `JsValue::UNDEFINED` for
    /// default options. Awaits the navigation promise so a rejection surfaces
    /// instead of an unobserved unhandled rejection.
    pub(crate) async fn redirect_to_sign_in(&self, opts: JsValue) -> Result<(), ClerkError> {
        let c = self.inner.as_ref().ok_or(ClerkError::NotLoaded)?;
        JsFuture::from(c.redirect_to_sign_in(&opts).map_err(js_error)?)
            .await
            .map(|_| ())
            .map_err(js_error)
    }

    /// Awaitable `Clerk.redirectToSignUp(opts?)`. Pass `JsValue::UNDEFINED` for
    /// default options. See [`ClerkHandle::redirect_to_sign_in`].
    pub(crate) async fn redirect_to_sign_up(&self, opts: JsValue) -> Result<(), ClerkError> {
        let c = self.inner.as_ref().ok_or(ClerkError::NotLoaded)?;
        JsFuture::from(c.redirect_to_sign_up(&opts).map_err(js_error)?)
            .await
            .map(|_| ())
            .map_err(js_error)
    }

    /// Awaitable `Clerk.session.getToken(opts?)`.
    pub(crate) async fn get_token(&self, opts: JsValue) -> Result<Option<String>, ClerkError> {
        let c = self.inner.as_ref().ok_or(ClerkError::NotLoaded)?;
        let session = c.session().map_err(js_error)?;
        if session.is_null() || session.is_undefined() {
            return Ok(None);
        }

        let get_token = Reflect::get(&session, &JsValue::from_str("getToken"))
            .map_err(js_error)?
            .dyn_into::<Function>()
            .map_err(|_| ClerkError::Js("Clerk.session.getToken is not a function".into()))?;
        let promise = get_token
            .call1(&session, &opts)
            .map_err(js_error)?
            .dyn_into::<js_sys::Promise>()
            .map_err(|_| {
                ClerkError::Js("Clerk.session.getToken did not return a Promise".into())
            })?;
        let token = JsFuture::from(promise).await.map_err(token_error)?;

        if token.is_null() || token.is_undefined() {
            Ok(None)
        } else {
            // A non-string resolution is a broken clerk-js surface, not "no
            // session token"; make it visible instead of returning Ok(None).
            token.as_string().map(Some).ok_or_else(|| {
                ClerkError::Js("Clerk.session.getToken resolved to a non-string value".into())
            })
        }
    }

    /// Open clerk-js's step-up reverification UI for a required `level` and
    /// resolve once the user completes or cancels it.
    ///
    /// Calls `Clerk.__internal_openReverification({ level, afterVerification,
    /// afterVerificationCancelled })` (the internal entry point clerk-react's
    /// `useReverification` uses) and awaits a Promise the two callbacks settle:
    /// `afterVerification` resolves it as [`ReverificationOutcome::Completed`],
    /// `afterVerificationCancelled` as [`ReverificationOutcome::Cancelled`].
    pub(crate) async fn open_reverification(
        &self,
        level: Option<ReverificationLevel>,
    ) -> Result<ReverificationOutcome, ClerkError> {
        let c = self.inner.as_ref().ok_or(ClerkError::NotLoaded)?;
        let open = clerk_method(c, "__internal_openReverification")?;
        let clerk = c.clone();
        let level_js = level
            .map(|level| JsValue::from_str(level.as_str()))
            .unwrap_or(JsValue::UNDEFINED);

        // clerk-js drives the reverification UI asynchronously and reports the
        // outcome through the two callbacks, so bridge them to a Promise we can
        // await. The executor runs synchronously inside `Promise::new`; capture
        // any synchronous throw from `__internal_openReverification` here rather
        // than leaving the Promise forever pending.
        let mut open_error: Option<ClerkError> = None;
        let promise = Promise::new(&mut |resolve, _reject| {
            let props = js_sys::Object::new();
            let _ = Reflect::set(&props, &JsValue::from_str("level"), &level_js);

            // clerk-js fires exactly one of the two callbacks for a given
            // prompt, and may do so after the awaiting future is already gone
            // (e.g. the component unmounted mid-prompt). `Closure::once_into_js`
            // hands ownership to the JS side and keeps each closure alive until
            // it fires (so a late call stays safe) while reclaiming the memory
            // of the one that does fire. Only the unused callback is retained,
            // which any late-call-safe approach must do; this avoids the double
            // leak `Closure::forget` would incur on every prompt.
            let on_complete = resolve.clone();
            let after_verification = Closure::once_into_js(move || {
                let _ = on_complete.call1(&JsValue::UNDEFINED, &JsValue::TRUE);
            });
            let after_cancelled = Closure::once_into_js(move || {
                let _ = resolve.call1(&JsValue::UNDEFINED, &JsValue::FALSE);
            });
            let _ = Reflect::set(
                &props,
                &JsValue::from_str("afterVerification"),
                &after_verification,
            );
            let _ = Reflect::set(
                &props,
                &JsValue::from_str("afterVerificationCancelled"),
                &after_cancelled,
            );

            if let Err(err) = open.call1(clerk.as_ref(), &props) {
                open_error = Some(js_error(err));
            }
        });

        if let Some(err) = open_error {
            return Err(err);
        }

        // Race the settle against a generous safety-net deadline: a prompt torn
        // down without firing either callback (e.g. clerk-js unmounted by an
        // external route change) must not leave this future pending forever.
        use futures_util::future::{Either, select};
        let settle = std::pin::pin!(JsFuture::from(promise));
        let deadline = std::pin::pin!(gloo_timers::future::TimeoutFuture::new(
            REVERIFICATION_SETTLE_TIMEOUT_MS
        ));
        let settled = match select(settle, deadline).await {
            Either::Left((result, _)) => result.map_err(js_error)?,
            Either::Right(((), _)) => {
                return Err(ClerkError::Timeout(
                    "reverification prompt did not settle within the deadline".into(),
                ));
            }
        };
        // The two callbacks above settle the Promise with `true` from
        // `afterVerification` and `false` from `afterVerificationCancelled`;
        // decode that back into the outcome.
        Ok(if settled.as_bool() == Some(true) {
            ReverificationOutcome::Completed
        } else {
            ReverificationOutcome::Cancelled
        })
    }

    /// Fallible dynamic Clerk call used by mounted UI bridge behavior.
    pub(crate) fn try_call_method2(
        &self,
        method: &str,
        first: &JsValue,
        second: &JsValue,
    ) -> Result<(), ClerkError> {
        let c = self.inner.as_ref().ok_or(ClerkError::NotLoaded)?;
        let function = clerk_method(c, method)?;
        function
            .call2(c.as_ref(), first, second)
            .map(|_| ())
            .map_err(js_error)
    }

    /// Best-effort dynamic Clerk call used by mounted UI cleanup paths.
    pub(crate) fn call_method1(&self, method: &str, first: &JsValue) {
        let Some(c) = &self.inner else {
            return;
        };
        let Ok(function) = clerk_method(c, method) else {
            return;
        };
        let _ = function.call1(c.as_ref(), first);
    }

    /// Open the sign-in modal. Pass `JsValue::UNDEFINED` for default options.
    pub(crate) fn open_sign_in(&self, opts: &JsValue) -> Result<(), ClerkError> {
        let c = self.inner.as_ref().ok_or(ClerkError::NotLoaded)?;
        c.open_sign_in(opts).map_err(js_error)
    }

    /// Close the sign-in modal.
    pub(crate) fn close_sign_in(&self) -> Result<(), ClerkError> {
        let c = self.inner.as_ref().ok_or(ClerkError::NotLoaded)?;
        c.close_sign_in().map_err(js_error)
    }

    /// Open the sign-up modal. Pass `JsValue::UNDEFINED` for default options.
    pub(crate) fn open_sign_up(&self, opts: &JsValue) -> Result<(), ClerkError> {
        let c = self.inner.as_ref().ok_or(ClerkError::NotLoaded)?;
        c.open_sign_up(opts).map_err(js_error)
    }

    /// Close the sign-up modal.
    pub(crate) fn close_sign_up(&self) -> Result<(), ClerkError> {
        let c = self.inner.as_ref().ok_or(ClerkError::NotLoaded)?;
        c.close_sign_up().map_err(js_error)
    }

    /// Open the user-profile modal. Pass `JsValue::UNDEFINED` for default options.
    pub(crate) fn open_user_profile(&self, opts: &JsValue) -> Result<(), ClerkError> {
        let c = self.inner.as_ref().ok_or(ClerkError::NotLoaded)?;
        c.open_user_profile(opts).map_err(js_error)
    }

    /// Close the user-profile modal.
    pub(crate) fn close_user_profile(&self) -> Result<(), ClerkError> {
        let c = self.inner.as_ref().ok_or(ClerkError::NotLoaded)?;
        c.close_user_profile().map_err(js_error)
    }

    /// Add a state-change listener and get back a subscription you must keep
    /// alive for the lifetime of the listener. Returns `None` when the
    /// singleton is absent or `addListener` threw.
    pub(crate) fn add_listener(&self, cb: impl Fn(JsValue) + 'static) -> Option<ClerkListener> {
        let c = self.inner.as_ref()?;
        let closure = Closure::wrap(Box::new(move |v: JsValue| cb(v)) as Box<dyn FnMut(JsValue)>);
        let unsubscribe = c.add_listener(closure.as_ref().unchecked_ref()).ok()?;
        Some(ClerkListener {
            _closure: closure,
            unsubscribe: Some(unsubscribe),
        })
    }
}

fn user_from_js(raw: JsValue) -> Option<User> {
    Some(User {
        id: string_prop(&raw, &["id"])?,
        first_name: string_prop(&raw, &["firstName", "first_name"]),
        last_name: string_prop(&raw, &["lastName", "last_name"]),
        primary_email_address: primary_email_address(&raw),
        image_url: string_prop(&raw, &["imageUrl", "image_url"]),
    })
}

fn session_from_js(raw: JsValue) -> Option<Session> {
    Some(Session {
        id: string_prop(&raw, &["id"])?,
        status: string_prop(&raw, &["status"])?.into(),
        last_active_organization_id: string_prop(
            &raw,
            &["lastActiveOrganizationId", "last_active_organization_id"],
        ),
        last_active_at: millis_prop(&raw, &["lastActiveAt", "last_active_at"]),
        expire_at: millis_prop(&raw, &["expireAt", "expire_at"]),
        current_task: prop(&raw, "currentTask")
            .as_ref()
            .and_then(session_task_from_js),
        tasks: session_tasks_from_js(&raw),
    })
}

/// Read one clerk-js `SessionTask` (`{ key }`) into a typed [`SessionTask`].
fn session_task_from_js(raw: &JsValue) -> Option<SessionTask> {
    Some(SessionTask::new(string_prop(raw, &["key"])?))
}

/// Read clerk-js `Session.tasks` (`Array<SessionTask> | null`) into typed
/// tasks; a missing or non-array value reads as no tasks.
fn session_tasks_from_js(raw: &JsValue) -> Vec<SessionTask> {
    let Some(value) = prop(raw, "tasks") else {
        return Vec::new();
    };
    let Ok(array) = value.dyn_into::<Array>() else {
        return Vec::new();
    };
    array
        .iter()
        .filter_map(|task| session_task_from_js(&task))
        .collect()
}

fn primary_email_address(raw: &JsValue) -> Option<String> {
    let value = prop(raw, "primaryEmailAddress").or_else(|| prop(raw, "primary_email_address"))?;
    value
        .as_string()
        .or_else(|| string_prop(&value, &["emailAddress", "email_address"]))
}

fn string_prop(raw: &JsValue, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| prop(raw, key)?.as_string())
}

fn millis_prop(raw: &JsValue, keys: &[&str]) -> Option<i64> {
    keys.iter().find_map(|key| millis_value(&prop(raw, key)?))
}

fn millis_value(value: &JsValue) -> Option<i64> {
    if let Some(n) = value.as_f64() {
        return n.is_finite().then_some(n as i64);
    }

    // A numeric string (e.g. `"1720000000000"`) is epoch millis, not a date
    // string: `Date::new` would parse a short one as a bare year. Try an
    // integer parse before falling back to date-string parsing.
    if let Some(text) = value.as_string() {
        if let Ok(millis) = text.trim().parse::<i64>() {
            return Some(millis);
        }
    }

    let millis = Date::new(value).get_time();
    millis.is_finite().then_some(millis as i64)
}

fn prop(raw: &JsValue, key: &str) -> Option<JsValue> {
    let value = Reflect::get(raw, &JsValue::from_str(key)).ok()?;
    if value.is_null() || value.is_undefined() {
        None
    } else {
        Some(value)
    }
}

pub(crate) fn js_error(value: JsValue) -> ClerkError {
    ClerkError::Js(js_error_message(&value))
}

/// Map a `session.getToken()` rejection, distinguishing the v6
/// `ClerkOfflineError` (browser offline, a transient condition callers can
/// retry) from other JS throws. clerk-js 6 throws this where 5.x returned a
/// `null` token.
pub(crate) fn token_error(value: JsValue) -> ClerkError {
    if string_prop(&value, &["name"]).as_deref() == Some("ClerkOfflineError") {
        return ClerkError::Offline;
    }
    js_error(value)
}

pub(crate) fn js_error_message(value: &JsValue) -> String {
    let message = value
        .as_string()
        .or_else(|| string_prop(value, &["message"]));
    let name = string_prop(value, &["name"]);

    match (name.as_deref(), message) {
        (Some(name), Some(message)) if !name.is_empty() && name != "Error" => {
            format!("{name}: {message}")
        }
        (_, Some(message)) => message,
        (Some(name), None) if !name.is_empty() => name.to_owned(),
        _ => format!("{value:?}"),
    }
}

fn clerk_method(c: &Clerk, method: &str) -> Result<Function, ClerkError> {
    let value = Reflect::get(c.as_ref(), &JsValue::from_str(method)).map_err(js_error)?;
    value
        .dyn_into::<Function>()
        .map_err(|_| ClerkError::Js(format!("window.Clerk.{method} is not a function")))
}

/// Clerk listener subscription that unsubscribes before dropping its callback.
pub(crate) struct ClerkListener {
    _closure: Closure<dyn FnMut(JsValue)>,
    unsubscribe: Option<Function>,
}

impl Drop for ClerkListener {
    fn drop(&mut self) {
        if let Some(unsubscribe) = self.unsubscribe.take() {
            let _ = unsubscribe.call0(&JsValue::UNDEFINED);
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use js_sys::{Array, Error, Object};
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn user_from_js_reads_clerk_camel_case_fields() {
        let user = Object::new();
        set_prop(user.as_ref(), "id", &JsValue::from_str("user_2abc"));
        set_prop(user.as_ref(), "firstName", &JsValue::from_str("Ada"));
        set_prop(user.as_ref(), "lastName", &JsValue::from_str("Lovelace"));
        set_prop(
            user.as_ref(),
            "imageUrl",
            &JsValue::from_str("https://img/x.png"),
        );

        let email = Object::new();
        set_prop(
            email.as_ref(),
            "emailAddress",
            &JsValue::from_str("ada@example.com"),
        );
        set_prop(user.as_ref(), "primaryEmailAddress", email.as_ref());

        let user = user_from_js(user.into()).expect("valid Clerk user shape");

        assert_eq!(user.first_name.as_deref(), Some("Ada"));
        assert_eq!(user.last_name.as_deref(), Some("Lovelace"));
        assert_eq!(
            user.primary_email_address.as_deref(),
            Some("ada@example.com")
        );
        assert_eq!(user.image_url.as_deref(), Some("https://img/x.png"));
    }

    #[wasm_bindgen_test]
    fn session_from_js_reads_clerk_date_fields_as_unix_millis() {
        let session = Object::new();
        set_prop(session.as_ref(), "id", &JsValue::from_str("sess_2def"));
        set_prop(session.as_ref(), "status", &JsValue::from_str("active"));
        set_prop(
            session.as_ref(),
            "lastActiveAt",
            Date::new(&JsValue::from_f64(12_345.0)).as_ref(),
        );
        set_prop(
            session.as_ref(),
            "expireAt",
            Date::new(&JsValue::from_f64(99_999.0)).as_ref(),
        );

        let session = session_from_js(session.into()).expect("valid Clerk session shape");

        assert_eq!(session.last_active_at, Some(12_345));
        assert_eq!(session.expire_at, Some(99_999));
    }

    #[wasm_bindgen_test]
    fn session_from_js_reads_current_task_and_tasks() {
        use crate::core::SessionTaskKey;

        let session = Object::new();
        set_prop(session.as_ref(), "id", &JsValue::from_str("sess_2def"));
        set_prop(session.as_ref(), "status", &JsValue::from_str("pending"));

        let current = Object::new();
        set_prop(current.as_ref(), "key", &JsValue::from_str("setup-mfa"));
        set_prop(session.as_ref(), "currentTask", current.as_ref());

        let tasks = Array::new();
        tasks.push(current.as_ref());
        let choose = Object::new();
        set_prop(
            choose.as_ref(),
            "key",
            &JsValue::from_str("choose-organization"),
        );
        tasks.push(choose.as_ref());
        set_prop(session.as_ref(), "tasks", tasks.as_ref());

        let session = session_from_js(session.into()).expect("valid pending session shape");

        assert_eq!(
            session.current_task.map(|task| task.key),
            Some(SessionTaskKey::SetupMfa)
        );
        assert_eq!(
            session
                .tasks
                .iter()
                .map(|task| task.key.clone())
                .collect::<Vec<_>>(),
            vec![SessionTaskKey::SetupMfa, SessionTaskKey::ChooseOrganization]
        );
    }

    #[wasm_bindgen_test]
    fn session_from_js_without_tasks_reads_none_and_empty() {
        let session = Object::new();
        set_prop(session.as_ref(), "id", &JsValue::from_str("sess_2def"));
        set_prop(session.as_ref(), "status", &JsValue::from_str("active"));

        let session = session_from_js(session.into()).expect("valid active session shape");

        assert_eq!(session.current_task, None);
        assert!(session.tasks.is_empty());
    }

    #[wasm_bindgen_test]
    fn js_error_message_prefers_error_message_and_custom_name() {
        let error = Error::new("Publishable key is invalid");
        error.set_name("ClerkRuntimeError");

        assert_eq!(
            js_error_message(error.as_ref()),
            "ClerkRuntimeError: Publishable key is invalid"
        );
    }

    #[wasm_bindgen_test]
    fn js_error_message_uses_string_values_directly() {
        assert_eq!(
            js_error_message(&JsValue::from_str("network unavailable")),
            "network unavailable"
        );
    }

    #[wasm_bindgen_test]
    fn token_error_maps_clerk_offline_error_by_name() {
        let error = Error::new("Network is offline");
        error.set_name("ClerkOfflineError");

        assert_eq!(token_error(error.into()), ClerkError::Offline);
    }

    #[wasm_bindgen_test]
    fn token_error_falls_back_to_js_for_other_throws() {
        let error = Error::new("boom");
        error.set_name("TypeError");

        assert!(matches!(token_error(error.into()), ClerkError::Js(_)));
    }

    fn set_prop(target: &JsValue, key: &str, value: &JsValue) {
        Reflect::set(target, &JsValue::from_str(key), value).unwrap();
    }
}
