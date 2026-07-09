//! `wasm-bindgen` extern bindings to the global `Clerk` singleton injected
//! by clerk-js into `window.Clerk` after the script tag loads.

use js_sys::{Function, Promise, Reflect};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// Mirror of `window.Clerk`.
    ///
    /// Every method binding is `catch`: `window.Clerk` may be an older
    /// clerk-js release (custom `clerk_js_url`) or a colliding foreign global,
    /// and a synchronous JS throw must become a `Result` instead of escaping
    /// into the calling scheduler tick.
    #[derive(Clone)]
    pub type Clerk;

    /// `Clerk.load(opts)` -> Promise<void>.
    #[wasm_bindgen(catch, method, js_name = "load")]
    pub fn load(this: &Clerk, opts: &JsValue) -> Result<Promise, JsValue>;

    /// `Clerk.signOut(opts?)` -> Promise<void>.
    #[wasm_bindgen(catch, method, js_name = "signOut")]
    pub fn sign_out(this: &Clerk, opts: &JsValue) -> Result<Promise, JsValue>;

    /// `Clerk.addListener(cb)` returns an unsubscribe function.
    #[wasm_bindgen(catch, method, js_name = "addListener")]
    pub fn add_listener(this: &Clerk, cb: &Function) -> Result<Function, JsValue>;

    /// Typed accessors on the singleton.
    #[wasm_bindgen(catch, method, getter)]
    pub fn user(this: &Clerk) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch, method, getter)]
    pub fn session(this: &Clerk) -> Result<JsValue, JsValue>;

    /// `Clerk.redirectToSignIn(opts?)` -> Promise. clerk-js navigates via an
    /// async `navigate()`, so the returned promise must be awaited for a
    /// rejection (blocked navigation, misconfigured redirect) to surface.
    #[wasm_bindgen(catch, method, js_name = "redirectToSignIn")]
    pub fn redirect_to_sign_in(this: &Clerk, opts: &JsValue) -> Result<Promise, JsValue>;

    /// `Clerk.redirectToSignUp(opts?)` -> Promise. See `redirect_to_sign_in`.
    #[wasm_bindgen(catch, method, js_name = "redirectToSignUp")]
    pub fn redirect_to_sign_up(this: &Clerk, opts: &JsValue) -> Result<Promise, JsValue>;

    /// `Clerk.openSignIn(opts?)`: opens the sign-in modal.
    #[wasm_bindgen(catch, method, js_name = "openSignIn")]
    pub fn open_sign_in(this: &Clerk, opts: &JsValue) -> Result<(), JsValue>;
    /// `Clerk.closeSignIn()`.
    #[wasm_bindgen(catch, method, js_name = "closeSignIn")]
    pub fn close_sign_in(this: &Clerk) -> Result<(), JsValue>;

    /// `Clerk.openSignUp(opts?)`: opens the sign-up modal.
    #[wasm_bindgen(catch, method, js_name = "openSignUp")]
    pub fn open_sign_up(this: &Clerk, opts: &JsValue) -> Result<(), JsValue>;
    /// `Clerk.closeSignUp()`.
    #[wasm_bindgen(catch, method, js_name = "closeSignUp")]
    pub fn close_sign_up(this: &Clerk) -> Result<(), JsValue>;

    /// `Clerk.openUserProfile(opts?)`: opens the user-profile modal.
    #[wasm_bindgen(catch, method, js_name = "openUserProfile")]
    pub fn open_user_profile(this: &Clerk, opts: &JsValue) -> Result<(), JsValue>;
    /// `Clerk.closeUserProfile()`.
    #[wasm_bindgen(catch, method, js_name = "closeUserProfile")]
    pub fn close_user_profile(this: &Clerk) -> Result<(), JsValue>;
}

/// Returns a clone of the singleton Clerk handle, or `None` if the global
/// hasn't been set yet (script not loaded) or is not Clerk-shaped.
///
/// Reads `window.Clerk` fresh on each call via `Reflect::get`. We intentionally
/// avoid `#[wasm_bindgen(thread_local_v2)] static CLERK` here: that pattern
/// caches the value in a `LazyCell` on first access, which means a poll
/// performed before clerk-js finished loading would capture `undefined` and
/// cache it forever, even after `window.Clerk` is set.
pub fn clerk_singleton() -> Option<Clerk> {
    let window = web_sys::window()?;
    let clerk = Reflect::get(&window, &JsValue::from_str("Clerk")).ok()?;
    if clerk.is_undefined() || clerk.is_null() {
        return None;
    }
    // A colliding non-Clerk global must read as "clerk-js has not executed",
    // so the load flow surfaces a timeout/ScriptLoad error instead of calling
    // methods on a foreign object.
    let load = Reflect::get(&clerk, &JsValue::from_str("load")).ok()?;
    if !load.is_function() {
        return None;
    }
    Some(clerk.unchecked_into())
}

/// The `@clerk/ui` component constructor that `ui.browser.js` installs on
/// `window.__internal_ClerkUICtor`.
///
/// clerk-js 6 split UI components into the separately-versioned `@clerk/ui`
/// bundle; the constructor must be handed to `Clerk.load({ ui: { ClerkUI } })`
/// or every `mountX` throws "Clerk was not loaded with UI components". Returns
/// `None` until the UI script has executed (or when clerk-js is loaded
/// externally without it).
pub fn clerk_ui_ctor() -> Option<Function> {
    let window = web_sys::window()?;
    let ctor = Reflect::get(&window, &JsValue::from_str("__internal_ClerkUICtor")).ok()?;
    ctor.dyn_into::<Function>().ok()
}
