//! Clerk action dispatch: coordination of application-requested clerk-js
//! browser actions with the Clerk lifecycle.
//!
//! One dispatch queue lives on the provider-owned `ClerkContext`, so
//! fire-and-forget operations from every hook share a single FIFO order. An
//! operation never touches clerk-js before the lifecycle reports loaded.
//! [`schedule`] surfaces failures through the provider's error reporting;
//! [`try_run`] reports the outcome directly to the caller.
//!
//! The operation vocabulary is plain JSON and compiles on every target; only
//! executing an operation requires the browser client.

use crate::context::ClerkContext;
use crate::core::ClerkError;
use dioxus::prelude::*;

/// Clerk browser operation known to the dispatch queue.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ClerkOperation {
    OpenSignIn(serde_json::Value),
    CloseSignIn,
    OpenSignUp(serde_json::Value),
    CloseSignUp,
    OpenUserProfile(serde_json::Value),
    CloseUserProfile,
    SignOut(serde_json::Value),
    RedirectToSignIn(serde_json::Value),
    RedirectToSignUp(serde_json::Value),
}

/// Queue a fire-and-forget operation on the provider-owned dispatch queue.
///
/// Failures are surfaced through `use_clerk_error()` once the scheduler runs
/// the operation. Off the browser client operations still queue but nothing
/// ever drains them: clicks cannot happen there, native tests inspect the
/// queued operations, and the queue dies with the per-request context. If a
/// long-lived non-client context ever exists, this must discard instead.
pub(crate) fn schedule(ctx: ClerkContext, operation: ClerkOperation) {
    let mut pending = ctx.pending;
    pending.write().push(operation);
}

/// Run one operation after Clerk lifecycle loadedness and return the outcome.
#[cfg(clerk_client)]
pub(crate) async fn try_run(
    ctx: ClerkContext,
    operation: ClerkOperation,
) -> Result<(), ClerkError> {
    crate::lifecycle::run_async_bridge_action_after_loaded(ctx, move |bridge| async move {
        bridge.run(operation).await
    })
    .await
}

/// Awaited operations fail with `UnsupportedTarget` off the browser client:
/// clerk-js is never reachable there.
#[cfg(not(clerk_client))]
pub(crate) async fn try_run(
    _ctx: ClerkContext,
    _operation: ClerkOperation,
) -> Result<(), ClerkError> {
    Err(ClerkError::UnsupportedTarget)
}

/// Drain the dispatch queue in order once the Clerk lifecycle reports loaded.
///
/// Called once by `ClerkProvider`, so queued operations from every hook share
/// one FIFO order. When the load flow has failed, queued operations are
/// dropped: the load error is already surfaced and the operations could
/// never run.
#[cfg(clerk_client)]
pub(crate) fn use_action_scheduler(ctx: ClerkContext) {
    let is_loaded = crate::lifecycle::use_loadedness(ctx);
    let mut pending = ctx.pending;
    let draining = use_signal(|| false);

    use_effect(move || {
        if pending.read().is_empty() {
            return;
        }
        if ctx.load_error.read().is_some() {
            pending.set(Vec::new());
            return;
        }
        if !*is_loaded.read() {
            return;
        }
        // One drain task at a time: it re-checks the queue after every batch,
        // so operations queued while an async action is in flight still run
        // after it, preserving the single FIFO order.
        if *draining.peek() {
            return;
        }

        let mut draining = draining;
        draining.set(true);
        let mut action_error = ctx.action_error;
        spawn(async move {
            loop {
                let actions = std::mem::take(&mut *pending.write());
                if actions.is_empty() {
                    break;
                }
                for action in actions {
                    let bridge = crate::bridge::ClerkBridge::current();
                    if let Err(err) = bridge.run(action).await {
                        action_error.set(Some(err));
                    }
                }
            }
            draining.set(false);
        });
    });
}

#[cfg(all(test, not(clerk_client)))]
mod tests {
    use super::*;
    use crate::components::ClerkProvider;
    use crate::context::use_clerk_context;
    use std::cell::RefCell;

    thread_local! {
        static SEEN: RefCell<Vec<ClerkOperation>> = const { RefCell::new(Vec::new()) };
    }

    #[component]
    fn Probe() -> Element {
        let auth = crate::hooks::use_auth();
        let clerk = crate::hooks::use_clerk();
        use_hook(move || {
            auth.sign_out();
            clerk.open_sign_in();
            clerk.close_sign_up();
        });

        let ctx = use_clerk_context();
        SEEN.with(|seen| *seen.borrow_mut() = ctx.pending.peek().clone());
        rsx! { div {} }
    }

    #[component]
    fn App() -> Element {
        rsx! {
            ClerkProvider { publishable_key: "pk_test_queue", Probe {} }
        }
    }

    #[test]
    fn scheduled_operations_share_one_provider_queue_in_request_order() {
        SEEN.with(|seen| seen.borrow_mut().clear());

        let mut dom = VirtualDom::new(App);
        dom.rebuild_in_place();

        SEEN.with(|seen| {
            assert_eq!(
                *seen.borrow(),
                vec![
                    ClerkOperation::SignOut(serde_json::Value::Null),
                    ClerkOperation::OpenSignIn(serde_json::Value::Null),
                    ClerkOperation::CloseSignUp,
                ],
            );
        });
    }
}
