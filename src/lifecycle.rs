//! Browser-side Clerk lifecycle seam.
//!
//! This module owns browser loadedness ordering for normal callers: loading
//! clerk-js, applying Auth state observations, and listener lifetime. Clerk
//! action dispatch (`crate::actions`) builds on this module's loadedness
//! primitives so browser actions never touch clerk-js before `Clerk.load()`
//! resolves.

use crate::bridge::{ClerkBridge, ListenerSubscription};
use crate::context::{ClerkContext, use_clerk_context};
use crate::core::{AuthObservation, AuthRuntimeState, ClerkError};
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;
use std::task::{Poll, Waker};

const DEFAULT_TIMEOUT_MS: u32 = 10_000;
const POLL_INTERVAL_MS: u32 = 50;
/// Upper bound on `Clerk.load()` settling. Without it a never-settling load
/// leaves no `load_error` and every awaited action waits forever.
const LOAD_SETTLE_TIMEOUT_MS: u32 = 60_000;
/// Upper bound on awaited actions waiting for lifecycle loadedness — a safety
/// net over the script-appear timeout plus the load-settle timeout.
const ACTION_WAIT_DEADLINE_MS: u32 = DEFAULT_TIMEOUT_MS + LOAD_SETTLE_TIMEOUT_MS + 5_000;

#[derive(Clone, Copy)]
enum ClerkScriptSource {
    PublishableKeyAvailable,
    ExternalOnly,
}

/// Inject clerk-js for a known publishable key.
pub(crate) fn inject_script(publishable_key: &str, options: &crate::loader::ScriptOptions) -> bool {
    crate::loader::inject_script(publishable_key, options)
}

/// Options passed to `Clerk.load(...)`.
#[derive(Clone)]
pub(crate) struct ClerkLoadOptions {
    pub(crate) value: serde_json::Value,
    pub(crate) router_push: Option<Callback<String>>,
    pub(crate) router_replace: Option<Callback<String>>,
}

/// Controls how this crate discovers clerk-js.
#[derive(Clone)]
pub(crate) struct ClerkScriptOptions {
    pub(crate) load_clerk_js: bool,
    pub(crate) script: crate::loader::ScriptOptions,
}

/// Signals owned by `ClerkProvider` and driven by the browser lifecycle.
#[derive(Clone, Copy)]
pub(crate) struct ClerkLifecycleSignals {
    pub(crate) auth: Signal<AuthRuntimeState>,
    pub(crate) load_error: Signal<Option<ClerkError>>,
}

/// Drive the full browser-side Clerk lifecycle from inside `ClerkProvider`.
///
/// `publishable_key` is a plain value, matching the provider's documented
/// read-once prop semantics — it is never written after mount.
///
/// The Clerk listener subscription is owned by the provider's scope and
/// unsubscribes on unmount. Keeping it alive past the provider would leave a
/// JS listener writing into dropped signals — a panic on the next Clerk event
/// after a route-layout provider unmounts.
pub(crate) fn use_drive_lifecycle(
    signals: ClerkLifecycleSignals,
    publishable_key: Option<String>,
    load_options: ClerkLoadOptions,
    script_options: ClerkScriptOptions,
) {
    use_hook({
        let publishable_key = publishable_key.clone();
        let script_options = script_options.clone();
        move || {
            if script_options.load_clerk_js {
                if let Some(key) = publishable_key {
                    inject_script(&key, &script_options.script);
                }
            }
        }
    });

    let subscription: Rc<RefCell<Option<ListenerSubscription>>> =
        use_hook(|| Rc::new(RefCell::new(None)));
    // clerk-js holds the router closures from the page's one Clerk.load() for
    // the life of the page, reading the page-scoped slots at call time. Each
    // provider mount re-populates the slots (a remount skips the second
    // load), and unmount clears them so a late navigation cannot call into
    // dropped scope-owned Callbacks (same bug class as the listener above).
    // The ownership token keeps a replacement remount safe when Dioxus mounts
    // the new provider before dropping the old one: the old provider's late
    // clear must not wipe the fresh callbacks.
    let router_slot_token = use_hook({
        let load_options = load_options.clone();
        move || {
            crate::bridge::set_router_callbacks(
                load_options.router_push,
                load_options.router_replace,
            )
        }
    });
    use_drop({
        let subscription = subscription.clone();
        move || {
            subscription.borrow_mut().take();
            crate::bridge::clear_router_callbacks(router_slot_token);
        }
    });

    // The load flow must run exactly once per provider instance, even if the
    // effect re-runs because a captured signal was written.
    let started = use_signal(|| false);

    use_effect({
        let mut auth = signals.auth;
        let mut load_error = signals.load_error;
        let has_publishable_key = publishable_key.is_some();
        let load_options = load_options.clone();
        let script_options = script_options.clone();
        move || {
            if *started.peek() {
                return;
            }
            let load_options = load_options.clone();
            let source = match (has_publishable_key, script_options.load_clerk_js) {
                (true, true) => ClerkScriptSource::PublishableKeyAvailable,
                (false, true) => {
                    load_error.set(Some(missing_publishable_key_error()));
                    wake_loadedness_waiters();
                    return;
                }
                (_, false) => ClerkScriptSource::ExternalOnly,
            };
            let mut started = started;
            started.set(true);
            let subscription = subscription.clone();
            spawn(async move {
                match load_global_with_source(DEFAULT_TIMEOUT_MS, load_options, source).await {
                    Ok(bridge) => {
                        apply_loaded_observation(&mut auth, bridge.observation());
                        *subscription.borrow_mut() = bridge.subscribe(move |observation| {
                            apply_loaded_observation(&mut auth, observation);
                        });
                    }
                    Err(e) => {
                        load_error.set(Some(e));
                        wake_loadedness_waiters();
                    }
                }
            });
        }
    });
}

fn apply_loaded_observation(auth: &mut Signal<AuthRuntimeState>, observation: AuthObservation) {
    let next = auth.read().apply_loaded_observation(observation);
    auth.set(next);
    wake_loadedness_waiters();
}

/// Memoize the lifecycle loadedness bit without making callers subscribe to
/// every Auth state change.
pub(crate) fn use_loadedness(ctx: ClerkContext) -> Memo<bool> {
    use_memo(move || ctx.auth.read().is_loaded())
}

/// Run a clerk-js action only after Auth state says the browser lifecycle is loaded.
///
/// Actions can defer completion for recoverable timing gaps such as a DOM host
/// not existing yet. Recoverable action failures are surfaced through
/// `use_clerk_error()`.
pub(crate) enum BridgeAction<T> {
    Deferred,
    Done(T),
}

thread_local! {
    /// Wakers of tasks awaiting [`wait_until_loaded`], woken whenever the
    /// lifecycle publishes progress (an observation or a load error). Waiting
    /// is event-driven instead of interval polling; spurious wakes just
    /// re-check the signals.
    static LOADEDNESS_WAKERS: RefCell<Vec<Waker>> = const { RefCell::new(Vec::new()) };
}

fn wake_loadedness_waiters() {
    let wakers = LOADEDNESS_WAKERS.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for waker in wakers {
        waker.wake();
    }
}

/// Resolves when the lifecycle reports loaded or a load error, waking on
/// lifecycle progress events rather than a poll interval.
struct LoadednessWatch {
    ctx: ClerkContext,
}

impl Future for LoadednessWatch {
    type Output = Result<(), ClerkError>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        // `peek`, not `read`: this future is polled inside the caller's
        // reactive scope (e.g. a `use_resource` awaiting `get_token`), and a
        // `read` would subscribe that scope to every auth signal write —
        // re-running it on each clerk-js emission instead of only on
        // loadedness changes. LOADEDNESS_WAKERS provides the wakeups.
        if let Some(error) = self.ctx.load_error.peek().clone() {
            return Poll::Ready(Err(error));
        }
        if self.ctx.auth.peek().is_loaded() {
            return Poll::Ready(Ok(()));
        }
        LOADEDNESS_WAKERS.with(|slot| slot.borrow_mut().push(cx.waker().clone()));
        Poll::Pending
    }
}

/// Wait for the browser Clerk lifecycle to finish loading before an awaited
/// caller touches the JS bridge layer.
///
/// Bounded by [`ACTION_WAIT_DEADLINE_MS`]: awaited actions must not hang
/// forever when the lifecycle can no longer make progress (for example a
/// provider that never started loading).
pub(crate) async fn wait_until_loaded(ctx: ClerkContext) -> Result<(), ClerkError> {
    use futures_util::future::{Either, select};
    let watch = std::pin::pin!(LoadednessWatch { ctx });
    let deadline = std::pin::pin!(TimeoutFuture::new(ACTION_WAIT_DEADLINE_MS));
    match select(watch, deadline).await {
        Either::Left((result, _)) => result,
        Either::Right(_) => Err(ClerkError::Timeout(format!(
            "browser Clerk lifecycle did not finish loading within {ACTION_WAIT_DEADLINE_MS} ms"
        ))),
    }
}

/// Run asynchronous JS bridge layer work after browser Clerk lifecycle loadedness.
pub(crate) async fn run_async_bridge_action_after_loaded<T, F>(
    ctx: ClerkContext,
    action: impl FnOnce(ClerkBridge) -> F,
) -> Result<T, ClerkError>
where
    F: Future<Output = Result<T, ClerkError>>,
{
    wait_until_loaded(ctx).await?;
    action(ClerkBridge::current()).await
}

enum LoadedAction<T> {
    Waiting,
    Deferred,
    Done(T),
    Failed,
}

fn loaded_bridge_action<T>(
    ctx: ClerkContext,
    is_loaded: &Memo<bool>,
    action: impl FnOnce(&ClerkBridge) -> Result<BridgeAction<T>, ClerkError>,
) -> LoadedAction<T> {
    if !*is_loaded.read() {
        return LoadedAction::Waiting;
    }

    let bridge = ClerkBridge::current();
    match action(&bridge) {
        Ok(BridgeAction::Deferred) => LoadedAction::Deferred,
        Ok(BridgeAction::Done(value)) => LoadedAction::Done(value),
        Err(err) => {
            let mut action_error = ctx.action_error;
            action_error.set(Some(err));
            LoadedAction::Failed
        }
    }
}

/// Schedule a clerk-js action after browser Auth loadedness, retrying deferred
/// results until the action can finish or the deferral deadline passes.
///
/// When the deadline passes, `deadline_error` is surfaced through
/// `use_clerk_error()` instead of retrying forever.
pub(crate) fn use_loaded_bridge_action<T: 'static, D: Dependency>(
    dependencies: D,
    should_run: impl Fn() -> bool + 'static,
    deferred_retry_ms: u32,
    deferred_deadline_ms: u32,
    deadline_error: ClerkError,
    action: impl Fn(&ClerkBridge) -> Result<BridgeAction<T>, ClerkError> + 'static,
    on_done: impl Fn(T) + 'static,
) {
    let ctx = use_clerk_context();
    let is_loaded = use_loadedness(ctx);
    let retry_tick = use_signal(|| 0_u32);
    let deferred_ms = use_signal(|| 0_u32);
    let max_deferred_ms = deferred_deadline_ms;

    use_effect(use_reactive(dependencies, move |_| {
        let _ = *retry_tick.read();
        if !should_run() {
            return;
        }

        match loaded_bridge_action(ctx, &is_loaded, |bridge| action(bridge)) {
            LoadedAction::Done(value) => {
                let mut deferred_ms = deferred_ms;
                deferred_ms.set(0);
                on_done(value);
            }
            LoadedAction::Deferred => {
                if *deferred_ms.peek() >= max_deferred_ms {
                    // Reset so a later deferral cycle (e.g. after an id or
                    // options change) gets a fresh deadline instead of
                    // erroring immediately off the saturated counter.
                    let mut deferred_ms = deferred_ms;
                    deferred_ms.set(0);
                    let mut action_error = ctx.action_error;
                    action_error.set(Some(deadline_error.clone()));
                    return;
                }
                let mut deferred_ms = deferred_ms;
                let next_deferred_ms = deferred_ms.peek().saturating_add(deferred_retry_ms);
                deferred_ms.set(next_deferred_ms);
                let mut retry_tick = retry_tick;
                spawn(async move {
                    TimeoutFuture::new(deferred_retry_ms).await;
                    let next = (*retry_tick.read()).wrapping_add(1);
                    retry_tick.set(next);
                });
            }
            LoadedAction::Failed => {
                // Reset so a later deferral cycle (e.g. after an id or
                // options change) gets a fresh deadline, matching Done.
                let mut deferred_ms = deferred_ms;
                deferred_ms.set(0);
            }
            LoadedAction::Waiting => {}
        }
    }));
}

/// Schedule a one-shot clerk-js action after browser Auth loadedness.
///
/// The effect observes the memoized loadedness bit, not the full Auth state, so
/// later listener updates do not turn the action into an ongoing watcher.
fn use_loaded_bridge_action_once(
    action: impl Fn(&ClerkBridge) -> Result<(), ClerkError> + 'static,
) {
    let ctx = use_clerk_context();
    let is_loaded = use_loadedness(ctx);
    let did_run = use_signal(|| false);

    use_effect(move || {
        if *did_run.read() || !*is_loaded.read() {
            return;
        }

        let bridge = ClerkBridge::current();
        let result = action(&bridge);

        let mut did_run = did_run;
        did_run.set(true);

        if let Err(err) = result {
            let mut action_error = ctx.action_error;
            action_error.set(Some(err));
        }
    });
}

/// Redirect to Clerk sign-in once the browser Clerk lifecycle has loaded.
///
/// clerk-js's redirect is an async navigation, so the one-shot effect spawns
/// the awaited call and routes any rejection to the provider error channel
/// rather than dropping it.
pub(crate) fn use_redirect_to_sign_in(options: serde_json::Value) {
    let ctx = use_clerk_context();
    use_loaded_bridge_action_once(move |bridge| {
        if bridge.is_signed_in() {
            return Ok(());
        }

        let options = options.clone();
        spawn(async move {
            if let Err(err) = ClerkBridge::current().redirect_to_sign_in(&options).await {
                let mut action_error = ctx.action_error;
                action_error.set(Some(err));
            }
        });
        Ok(())
    });
}

/// Redirect to Clerk sign-up once the browser Clerk lifecycle has loaded.
///
/// See [`use_redirect_to_sign_in`].
pub(crate) fn use_redirect_to_sign_up(options: serde_json::Value) {
    let ctx = use_clerk_context();
    use_loaded_bridge_action_once(move |bridge| {
        if bridge.is_signed_in() {
            return Ok(());
        }

        let options = options.clone();
        spawn(async move {
            if let Err(err) = ClerkBridge::current().redirect_to_sign_up(&options).await {
                let mut action_error = ctx.action_error;
                action_error.set(Some(err));
            }
        });
        Ok(())
    });
}

async fn load_global_with_source(
    timeout_ms: u32,
    options: ClerkLoadOptions,
    source: ClerkScriptSource,
) -> Result<ClerkBridge, ClerkError> {
    // When this crate injected the `@clerk/ui` bundle, clerk-js 6 needs its
    // constructor handed to `Clerk.load()` below, so also wait for
    // `window.__internal_ClerkUICtor`. Gate on the injected tag rather than the
    // source: a live `window.Clerk` already present (external load, or a mock)
    // skips injection, and must not wait for a UI global that will never
    // appear — a failed injection still surfaces via `script_load_error`.
    let need_ui = crate::loader::ui_script_injected();
    let mut waited_ms: u32 = 0;
    let bridge = loop {
        let bridge = ClerkBridge::current();
        if bridge.is_present() && (!need_ui || crate::bindings::clerk_ui_ctor().is_some()) {
            break bridge;
        }
        if let Some(message) = crate::loader::script_load_error() {
            return Err(ClerkError::ScriptLoad(message));
        }
        if waited_ms >= timeout_ms {
            return Err(timeout_error(source, timeout_ms));
        }
        TimeoutFuture::new(POLL_INTERVAL_MS).await;
        waited_ms += POLL_INTERVAL_MS;
    };

    // A remounted provider (or externally-driven clerk-js) finds Clerk already
    // loaded; calling `Clerk.load()` a second time is unsupported by clerk-js.
    if bridge.is_loaded() {
        return Ok(bridge);
    }

    // A load started by a previous provider instance may still be in flight:
    // the old provider's task was cancelled on unmount, but the JS promise
    // keeps running. Attach to it instead of issuing a second Clerk.load().
    if crate::bridge::load_in_flight() {
        let mut waited_ms: u32 = 0;
        while crate::bridge::load_in_flight() && waited_ms < LOAD_SETTLE_TIMEOUT_MS {
            TimeoutFuture::new(POLL_INTERVAL_MS).await;
            waited_ms += POLL_INTERVAL_MS;
        }
        let bridge = ClerkBridge::current();
        if bridge.is_loaded() {
            return Ok(bridge);
        }
        if crate::bridge::load_in_flight() {
            let message = format!(
                "a previous Clerk.load() did not settle within {LOAD_SETTLE_TIMEOUT_MS} ms"
            );
            // Record the timeout as terminal so later remounts fail fast on the
            // recorded failure instead of re-entering this branch and waiting
            // another full settle timeout against the same hung promise.
            crate::bridge::mark_load_timed_out(message.clone());
            return Err(ClerkError::ScriptLoad(message));
        }
        // The previous load settled; fall through to the failure gate below.
    }

    // A previous Clerk.load() on this same live singleton settled with a
    // rejection. clerk-js does not support re-loading a live instance, so
    // surface the recorded failure instead of issuing a second Clerk.load().
    // The failure is only recoverable by a fresh clerk-js (a full page reload,
    // or the script-tag-failure path that re-injects and never reaches here),
    // both of which reset the marker.
    if let Some(message) = crate::bridge::load_failure() {
        return Err(ClerkError::ScriptLoad(message));
    }

    // Bound Clerk.load() settling: a hung load must surface as a load error
    // instead of leaving the provider silently loading forever.
    use futures_util::future::{Either, select};
    {
        let load = std::pin::pin!(bridge.load(&options.value));
        let deadline = std::pin::pin!(TimeoutFuture::new(LOAD_SETTLE_TIMEOUT_MS));
        match select(load, deadline).await {
            Either::Left((result, _)) => result?,
            Either::Right(_) => {
                let message =
                    format!("Clerk.load() did not settle within {LOAD_SETTLE_TIMEOUT_MS} ms");
                // The JS promise is still pending; mark the timeout terminal so
                // remounts short-circuit instead of re-waiting the full timeout.
                crate::bridge::mark_load_timed_out(message.clone());
                return Err(ClerkError::ScriptLoad(message));
            }
        }
    }
    Ok(bridge)
}

fn timeout_error(source: ClerkScriptSource, timeout_ms: u32) -> ClerkError {
    match source {
        ClerkScriptSource::PublishableKeyAvailable => ClerkError::ScriptLoad(format!(
            "window.Clerk / @clerk/ui did not become ready within {timeout_ms} ms; check the network tab for clerk.browser.js or ui.browser.js load failures and the dashboard's allowed-origins config",
        )),
        ClerkScriptSource::ExternalOnly => ClerkError::ScriptLoad(format!(
            "window.Clerk did not appear within {timeout_ms} ms; load_clerk_js is false so dioxus-clerk expected clerk-js to be loaded externally",
        )),
    }
}

fn missing_publishable_key_error() -> ClerkError {
    ClerkError::InvalidConfig(
        "missing Clerk publishable key; provide ClerkProvider publishable_key, an SSR initial state publishable key, or set load_clerk_js to false when loading clerk-js externally".into(),
    )
}
