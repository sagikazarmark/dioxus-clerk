//! Step-up reverification: wrapping a gated action so that a clerk-reported
//! "needs reverification" outcome triggers a re-auth prompt and, on success,
//! resumes the action.
//!
//! The wrap → prompt → retry control flow lives in [`run_with_reverification`]
//! as a pure combinator over two async closures — the gated action and the
//! prompt — so `cargo test` covers it on the host without a browser. The public
//! [`use_reverification`] hook wires the prompt to clerk-js's reverification UI.

use crate::context::{ClerkContext, use_clerk_context};
use crate::core::{ClerkError, ReverificationLevel};
use dioxus::prelude::*;
use std::future::Future;

/// The result of a step-up reverification prompt handed back to
/// [`run_with_reverification`].
///
/// Only the browser client (`handle.rs`) constructs these; off-client the
/// prompt short-circuits to [`ClerkError::UnsupportedTarget`], so allow the
/// variants to read as dead there.
#[cfg_attr(not(clerk_client), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReverificationOutcome {
    /// The user completed reverification; the gated action may retry.
    Completed,
    /// The user dismissed the prompt without completing it.
    Cancelled,
}

/// Run a gated action, and if it reports that step-up reverification is needed,
/// prompt for it and resume the action on success.
///
/// `fetcher` is the gated action; it may run twice (once, then again after a
/// successful reverification), so it takes no arguments and is called fresh
/// each time. `prompt` drives the reverification UI for a required level and is
/// invoked at most once, only when the first `fetcher` call reports
/// [`ClerkError::NeedsReverification`].
///
/// - action succeeds → its value, prompt untouched
/// - action needs reverification, prompt completes → the action's retry result
/// - action needs reverification, prompt cancels →
///   [`ClerkError::ReverificationCancelled`]
/// - any other action error, or a prompt error → propagated unchanged
pub(crate) async fn run_with_reverification<T, Fetch, FetchFut, Prompt, PromptFut>(
    fetcher: Fetch,
    prompt: Prompt,
) -> Result<T, ClerkError>
where
    Fetch: Fn() -> FetchFut,
    FetchFut: Future<Output = Result<T, ClerkError>>,
    Prompt: FnOnce(Option<ReverificationLevel>) -> PromptFut,
    PromptFut: Future<Output = Result<ReverificationOutcome, ClerkError>>,
{
    match fetcher().await {
        Err(ClerkError::NeedsReverification { level }) => match prompt(level).await? {
            ReverificationOutcome::Completed => fetcher().await,
            ReverificationOutcome::Cancelled => Err(ClerkError::ReverificationCancelled),
        },
        other => other,
    }
}

/// Drive clerk-js's reverification UI for a required level, after Clerk
/// lifecycle loadedness, and report whether the user completed or cancelled it.
///
/// Off the browser client clerk-js is unreachable, so a gated action that needs
/// reverification fails with [`ClerkError::UnsupportedTarget`] rather than
/// silently resolving.
#[cfg(clerk_client)]
async fn prompt_reverification(
    ctx: ClerkContext,
    level: Option<ReverificationLevel>,
) -> Result<ReverificationOutcome, ClerkError> {
    crate::lifecycle::run_async_bridge_action_after_loaded(ctx, move |bridge| async move {
        bridge.open_reverification(level).await
    })
    .await
}

#[cfg(not(clerk_client))]
async fn prompt_reverification(
    _ctx: ClerkContext,
    _level: Option<ReverificationLevel>,
) -> Result<ReverificationOutcome, ClerkError> {
    Err(ClerkError::UnsupportedTarget)
}

/// Reactive handle returned by [`use_reverification`], mirroring Clerk React's
/// `useReverification`.
///
/// The handle is `Copy`, so it can be captured into event handlers. Wrap a
/// sensitive action with [`UseReverification::guard`]; if the action reports
/// that step-up reverification is needed, clerk-js's reverification UI is shown
/// and the action resumes on success.
#[derive(Clone, Copy)]
pub struct UseReverification {
    ctx: ClerkContext,
}

impl std::fmt::Debug for UseReverification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UseReverification").finish_non_exhaustive()
    }
}

impl UseReverification {
    /// Run a sensitive `action`, guarding it with step-up reverification.
    ///
    /// `action` is the gated call. Typically it is a `#[server]` function that
    /// maps a clerk reverification hint onto [`ClerkError::NeedsReverification`]
    /// via [`ClerkError::from_reverification_hint`]; an `action` calling clerk-js
    /// directly instead constructs that variant from its own caught throw. When
    /// `action` reports that reverification is needed, clerk-js
    /// prompts the user for a
    /// fresh factor and `action` is retried on success. `action` may therefore
    /// run twice, so it takes no arguments and is invoked fresh each time.
    ///
    /// - action succeeds → its value
    /// - action needs reverification, user completes it → the retry's result
    /// - action needs reverification, user cancels →
    ///   [`ClerkError::ReverificationCancelled`]
    /// - any other action error → propagated unchanged
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dioxus::prelude::*;
    /// use dioxus_clerk::*;
    ///
    /// # async fn delete_account() -> Result<(), ClerkError> { Ok(()) }
    /// #[component]
    /// fn DangerZone() -> Element {
    ///     let reverify = use_reverification();
    ///
    ///     rsx! {
    ///         button {
    ///             onclick: move |_| async move {
    ///                 let _ = reverify.guard(|| delete_account()).await;
    ///             },
    ///             "Delete account"
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn guard<T, F, Fut>(&self, action: F) -> Result<T, ClerkError>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, ClerkError>>,
    {
        let ctx = self.ctx;
        run_with_reverification(action, move |level| prompt_reverification(ctx, level)).await
    }
}

/// Guard sensitive actions behind Clerk step-up reverification.
///
/// Returns a [`UseReverification`] handle whose [`guard`](UseReverification::guard)
/// method wraps an action so a clerk-reported "needs reverification" outcome
/// prompts the user for a fresh authentication factor and resumes the action on
/// success. Mirrors Clerk React's `useReverification`.
pub fn use_reverification() -> UseReverification {
    UseReverification {
        ctx: use_clerk_context(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ClerkError;
    use futures_util::FutureExt;
    use std::cell::Cell;

    #[test]
    fn success_returns_value_and_never_prompts() {
        let prompted = Cell::new(false);

        let result = run_with_reverification(
            || async { Ok::<_, ClerkError>(7) },
            |_level| async {
                prompted.set(true);
                Ok(ReverificationOutcome::Completed)
            },
        )
        .now_or_never()
        .expect("all futures resolve immediately");

        assert_eq!(result, Ok(7));
        assert!(!prompted.get(), "a successful action must not prompt");
    }

    // A fetcher that needs reverification on its first call and succeeds on the
    // retry: the prompt runs once (seeing the level), the action runs twice,
    // and the caller gets the retry's value.
    #[test]
    fn completed_prompt_retries_the_action() {
        let calls = Cell::new(0u32);
        let seen_level = Cell::new(None);

        let result = run_with_reverification(
            || async {
                let n = calls.get();
                calls.set(n + 1);
                if n == 0 {
                    Err(ClerkError::NeedsReverification {
                        level: Some(ReverificationLevel::SecondFactor),
                    })
                } else {
                    Ok(42)
                }
            },
            |level| async {
                seen_level.set(Some(level));
                Ok(ReverificationOutcome::Completed)
            },
        )
        .now_or_never()
        .expect("all futures resolve immediately");

        assert_eq!(result, Ok(42));
        assert_eq!(calls.get(), 2, "the action retries after reverification");
        assert_eq!(
            seen_level.take(),
            Some(Some(ReverificationLevel::SecondFactor)),
            "the prompt receives the required level"
        );
    }

    // A cancelled prompt maps to `ReverificationCancelled` and does not retry.
    #[test]
    fn cancelled_prompt_yields_cancelled_and_does_not_retry() {
        let calls = Cell::new(0u32);

        let result = run_with_reverification(
            || async {
                calls.set(calls.get() + 1);
                Err::<i32, _>(ClerkError::NeedsReverification { level: None })
            },
            |_level| async { Ok(ReverificationOutcome::Cancelled) },
        )
        .now_or_never()
        .expect("all futures resolve immediately");

        assert_eq!(result, Err(ClerkError::ReverificationCancelled));
        assert_eq!(calls.get(), 1, "a cancelled prompt must not retry");
    }

    // Any non-reverification error passes straight through, untouched, and the
    // prompt is never shown.
    #[test]
    fn other_errors_pass_through_without_prompting() {
        let prompted = Cell::new(false);

        let result = run_with_reverification(
            || async { Err::<i32, _>(ClerkError::Unauthenticated) },
            |_level| async {
                prompted.set(true);
                Ok(ReverificationOutcome::Completed)
            },
        )
        .now_or_never()
        .expect("all futures resolve immediately");

        assert_eq!(result, Err(ClerkError::Unauthenticated));
        assert!(
            !prompted.get(),
            "a non-reverification error must not prompt"
        );
    }

    // A prompt that fails to run (e.g. clerk-js not loaded) propagates its own
    // error rather than retrying or masking it as cancelled.
    #[test]
    fn prompt_error_propagates_without_retry() {
        let calls = Cell::new(0u32);

        let result = run_with_reverification(
            || async {
                calls.set(calls.get() + 1);
                Err::<i32, _>(ClerkError::NeedsReverification { level: None })
            },
            |_level| async { Err(ClerkError::NotLoaded) },
        )
        .now_or_never()
        .expect("all futures resolve immediately");

        assert_eq!(result, Err(ClerkError::NotLoaded));
        assert_eq!(calls.get(), 1, "a failed prompt must not retry the action");
    }
}
