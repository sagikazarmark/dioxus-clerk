//! Emits the `clerk_client` cfg alias: true when compiling for the browser
//! client (wasm32 without the `worker` feature), where clerk-js is reachable.
//! Use `#[cfg(clerk_client)]` / `#[cfg(not(clerk_client))]` instead of
//! repeating `all(target_arch = "wasm32", not(feature = "worker"))`.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(clerk_client)");

    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let worker = std::env::var("CARGO_FEATURE_WORKER").is_ok();
    if target_arch == "wasm32" && !worker {
        println!("cargo::rustc-cfg=clerk_client");
    }
}
