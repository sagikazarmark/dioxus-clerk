fn main() {
    // env! reads the env var at compile time but cargo doesn't track
    // that dependency by default. Without this directive, changing the
    // env var (or setting it after a previous build) wouldn't invalidate
    // the cached binary.
    println!("cargo:rerun-if-env-changed=CLERK_PUBLISHABLE_KEY");
}
