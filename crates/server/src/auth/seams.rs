//! Injected dependencies for deterministic browser-auth testing.
//!
//! These traits and fakes allow tests to control time and token generation, ensuring
//! browser-auth behavior is deterministic rather than dependent on system time or randomness.

use rand::Rng;

/// Injected wall clock. Every timestamp written to the browser-auth tables comes from here, so
/// expiry behaviour is driven deterministically in tests rather than by SQL `datetime('now')`.
pub trait Clock: Send + Sync {
    /// Milliseconds since the Unix epoch, UTC.
    fn now_millis(&self) -> i64;
}

pub struct SystemClock;
impl Clock for SystemClock {
    fn now_millis(&self) -> i64 {
        chrono::Utc::now().timestamp_millis()
    }
}

/// Test fake. Public and unconditionally compiled: `crates/server/tests/` links the lib WITHOUT
/// `cfg(test)`, and `crates/server` has no `test-utils` feature to gate on.
pub struct FixedClock(std::sync::atomic::AtomicI64);
impl FixedClock {
    pub fn new(millis: i64) -> Self {
        FixedClock(std::sync::atomic::AtomicI64::new(millis))
    }
    pub fn set(&self, millis: i64) {
        self.0.store(millis, std::sync::atomic::Ordering::SeqCst);
    }
    pub fn advance(&self, delta_millis: i64) {
        self.0
            .fetch_add(delta_millis, std::sync::atomic::Ordering::SeqCst);
    }
}
impl Clock for FixedClock {
    fn now_millis(&self) -> i64 {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// Source of opaque secrets: session tokens and pre-auth binding-cookie values.
pub trait TokenSource: Send + Sync {
    /// 32 bytes (256 bits) of CSPRNG output, base64url-encoded WITHOUT padding (43 chars).
    fn generate_token(&self) -> String;
}

pub struct OsTokenSource;
impl TokenSource for OsTokenSource {
    fn generate_token(&self) -> String {
        use base64::prelude::*;
        let mut rng = rand::rng();
        let random_bytes: [u8; 32] = rng.random();
        BASE64_URL_SAFE_NO_PAD.encode(random_bytes)
    }
}

/// Test fake returning a scripted sequence; panics when exhausted so a test can never silently
/// fall back to real randomness.
pub struct ScriptedTokenSource(std::sync::Mutex<std::collections::VecDeque<String>>);
impl ScriptedTokenSource {
    pub fn new(tokens: impl IntoIterator<Item = String>) -> Self {
        ScriptedTokenSource(std::sync::Mutex::new(tokens.into_iter().collect()))
    }
}
impl TokenSource for ScriptedTokenSource {
    fn generate_token(&self) -> String {
        let mut queue = self.0.lock().unwrap();
        queue.pop_front().expect("ScriptedTokenSource exhausted")
    }
}

/// Lowercase hex SHA-256, 64 chars. Deliberately a free function, not a trait: it must be
/// deterministic, so a fake could vary nothing useful, and a trait would let an implementation
/// drift from the encoding stored in the database.
pub fn hash_token(token: &str) -> String {
    use sha2::Digest;
    let mut output = String::with_capacity(64);
    let digest = sha2::Sha256::digest(token.as_bytes());
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(output, "{:02x}", byte);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_token_pins_the_stored_encoding() {
        // Value from OUTSIDE the implementation: SHA-256 of the empty string.
        assert_eq!(
            hash_token(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        let h = hash_token("abc");
        assert_eq!(h.len(), 64);
        assert!(
            h.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
        assert_eq!(h, hash_token("abc"));
        assert_ne!(h, hash_token("abd"));
    }

    #[test]
    fn os_token_source_is_256_bit_base64url_unpadded() {
        let s = OsTokenSource;
        let a = s.generate_token();
        let b = s.generate_token();
        assert_eq!(
            a.len(),
            43,
            "32 raw bytes base64url-unpadded is 43 chars: {a}"
        );
        assert!(
            !a.contains('+') && !a.contains('/') && !a.contains('='),
            "not url-safe: {a}"
        );
        assert_ne!(a, b);
    }

    #[test]
    fn fixed_clock_is_settable_and_additive() {
        let c = FixedClock::new(1_000);
        assert_eq!(c.now_millis(), 1_000);
        c.advance(600_000);
        assert_eq!(c.now_millis(), 601_000);
        c.set(7);
        assert_eq!(c.now_millis(), 7);
    }

    #[test]
    fn scripted_token_source_returns_in_order() {
        let s = ScriptedTokenSource::new(["t1".to_string(), "t2".to_string()]);
        assert_eq!(s.generate_token(), "t1");
        assert_eq!(s.generate_token(), "t2");
    }

    #[test]
    #[should_panic]
    fn scripted_token_source_panics_when_exhausted() {
        let s = ScriptedTokenSource::new([]);
        let _ = s.generate_token();
    }
}
