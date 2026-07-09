//! Decode the Clerk Frontend API host from a publishable key.
//!
//! clerk-js is served from the instance's Frontend API host so the script and
//! the backend it talks to stay version-coordinated (matching Clerk's own
//! loaders). The host is encoded in the publishable key, so the default script
//! URL can be derived from it without extra configuration.
//!
//! Compiled on the browser client (where the loader uses it) and under `test`
//! (so the pure decoding logic is covered by host `cargo test`, not only the
//! CI-only wasm suite).

/// The Frontend API host encoded in a Clerk publishable key.
///
/// A publishable key is `pk_test_<b64>` / `pk_live_<b64>`, where `<b64>` is the
/// standard-base64 Frontend API host with a trailing `$` marker, for example
/// `pk_test_Zm9vLmNsZXJrLmFjY291bnRzLmRldiQ` decodes to `foo.clerk.accounts.dev$`,
/// yielding `foo.clerk.accounts.dev`. Returns `None` when the key is not shaped
/// like a publishable key or does not decode to a non-empty host.
pub(crate) fn frontend_api_host(publishable_key: &str) -> Option<String> {
    let encoded = publishable_key
        .strip_prefix("pk_test_")
        .or_else(|| publishable_key.strip_prefix("pk_live_"))?;
    let decoded = base64_decode(encoded)?;
    let text = String::from_utf8(decoded).ok()?;
    // Clerk appends a `$` marker after the host; tolerate its absence.
    let host = text.strip_suffix('$').unwrap_or(&text).trim();
    (!host.is_empty()).then(|| host.to_owned())
}

/// Minimal standard-base64 decoder (alphabet `A–Za–z0–9+/`, optional `=`
/// padding). Dependency-free and target-neutral so it decodes publishable keys
/// on the wasm client without pulling `ct-codecs`, which is server-gated.
/// Returns `None` on any character outside the alphabet.
fn base64_decode(input: &str) -> Option<Vec<u8>> {
    fn sextet(byte: u8) -> Option<u8> {
        match byte {
            b'A'..=b'Z' => Some(byte - b'A'),
            b'a'..=b'z' => Some(byte - b'a' + 26),
            b'0'..=b'9' => Some(byte - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }

    let input = input.trim_end_matches('=');
    let mut out = Vec::with_capacity(input.len() / 4 * 3 + 3);
    let mut accumulator: u32 = 0;
    let mut bits: u32 = 0;
    for &byte in input.as_bytes() {
        accumulator = (accumulator << 6) | u32::from(sextet(byte)?);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((accumulator >> bits) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn publishable_key(prefix: &str, host_marker: &str) -> String {
        format!("{prefix}{}", base64_encode(host_marker.as_bytes()))
    }

    /// Standard-base64 encoder used only to build test keys, so the tests stay
    /// free of the server-gated `ct-codecs` dependency and run under plain
    /// `cargo test`. Mirrors the alphabet `base64_decode` accepts.
    fn base64_encode(input: &[u8]) -> String {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

        let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
        for chunk in input.chunks(3) {
            let b0 = chunk[0];
            out.push(TABLE[(b0 >> 2) as usize] as char);
            match chunk {
                [_, b1, b2] => {
                    out.push(TABLE[(((b0 & 0b11) << 4) | (b1 >> 4)) as usize] as char);
                    out.push(TABLE[(((b1 & 0b1111) << 2) | (b2 >> 6)) as usize] as char);
                    out.push(TABLE[(b2 & 0b11_1111) as usize] as char);
                }
                [_, b1] => {
                    out.push(TABLE[(((b0 & 0b11) << 4) | (b1 >> 4)) as usize] as char);
                    out.push(TABLE[((b1 & 0b1111) << 2) as usize] as char);
                    out.push('=');
                }
                _ => {
                    out.push(TABLE[((b0 & 0b11) << 4) as usize] as char);
                    out.push_str("==");
                }
            }
        }
        out
    }

    #[test]
    fn decodes_frontend_api_host_from_test_key() {
        let key = publishable_key("pk_test_", "foo-bar-13.clerk.accounts.dev$");
        assert_eq!(
            frontend_api_host(&key).as_deref(),
            Some("foo-bar-13.clerk.accounts.dev")
        );
    }

    #[test]
    fn decodes_frontend_api_host_from_live_key() {
        let key = publishable_key("pk_live_", "clerk.example.com$");
        assert_eq!(
            frontend_api_host(&key).as_deref(),
            Some("clerk.example.com")
        );
    }

    #[test]
    fn tolerates_a_host_without_the_trailing_marker() {
        let key = publishable_key("pk_test_", "clerk.example.com");
        assert_eq!(
            frontend_api_host(&key).as_deref(),
            Some("clerk.example.com")
        );
    }

    #[test]
    fn rejects_a_key_without_a_publishable_prefix() {
        assert!(frontend_api_host("sk_test_secret").is_none());
        assert!(frontend_api_host("not-a-key").is_none());
    }

    #[test]
    fn rejects_a_key_whose_body_is_not_valid_base64() {
        assert!(frontend_api_host("pk_test_not*base64").is_none());
    }

    #[test]
    fn rejects_a_key_that_decodes_to_an_empty_host() {
        let key = publishable_key("pk_test_", "$");
        assert!(frontend_api_host(&key).is_none());
    }
}
