---
id: "002"
phase: 1
title: "Add deterministic clock, token-source and hashing seams with public test fakes"
status: ready
depends_on: []
parallel: false
conflicts_with: ["006","007","011"]
files:
  - "Cargo.lock"
  - "crates/server/src/auth/mod.rs"
  - "crates/server/src/auth/seams.rs"
  - "crates/server/src/lib.rs"
  - "crates/server/Cargo.toml"
siblings: ["crates/server/src/routes/oauth.rs"]
irreversible: false
scope_test: "crates/server/src/auth/seams.rs"
allowed_change: mixed
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
File: `crates/server/src/auth/seams.rs` — colocated `#[cfg(test)] mod tests` written in the SAME commit as the interfaces.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_token_pins_the_stored_encoding() {
        // Value from OUTSIDE the implementation: SHA-256 of the empty string.
        assert_eq!(hash_token(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        let h = hash_token("abc");
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert_eq!(h, hash_token("abc"));
        assert_ne!(h, hash_token("abd"));
    }

    #[test]
    fn os_token_source_is_256_bit_base64url_unpadded() {
        let s = OsTokenSource;
        let a = s.generate_token();
        let b = s.generate_token();
        assert_eq!(a.len(), 43, "32 raw bytes base64url-unpadded is 43 chars: {a}");
        assert!(!a.contains('+') && !a.contains('/') && !a.contains('='), "not url-safe: {a}");
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
```


## Change
**File:** `crates/server/Cargo.toml`
**Anchor:** the dependency lines at L56-58 (verified).
**Before:**
```toml
url = "2.5"
rand = { version = "0.9", features = ["std"] }
sha2 = "0.10"
```
**After:**
```toml
url = "2.5"
rand = { version = "0.9", features = ["std"] }
sha2 = "0.10"
base64 = "0.22"
```
(`base64 = "0.22"` is the version already used by `crates/services` and `crates/remote`; keep it identical.) Touch ONLY the `[dependencies]` block: task 006 adds a `tokio-tungstenite` dev-dependency to the same file, which is why these two tasks declare each other in `conflicts_with`.

**File:** `Cargo.lock`
**Anchor:** the generated `server` package dependency list.
**Before:** the `server` package dependency list does not include `"base64"`.
**After:** regenerate the lockfile after adding the dependency; the only lockfile change is adding `"base64"` to the existing `server` package dependency list. The package/version already exists elsewhere in the lockfile, so no package resolution may change.

**File:** `crates/server/src/lib.rs`
**Anchor:** the `pub mod` block, L1-7 (verified).
**Before:**
```rust
pub mod error;
pub mod file_logging;
```
**After:**
```rust
pub mod auth;
pub mod error;
pub mod file_logging;
```
(alphabetical order preserved).

**File:** `crates/server/src/auth/mod.rs` — create.
**After:**
```rust
//! Local browser-authorization for the node HTTP surface.
//!
//! `seams` holds the injected clock / token source / hash used by every browser-auth path so
//! expiry and token behaviour are deterministic in tests. Later tasks add `cookies`, `session`,
//! `node_token` and `login` here.

pub mod seams;
```

**File:** `crates/server/src/auth/seams.rs` — create. Exact public interface:
```rust
/// Injected wall clock. Every timestamp written to the browser-auth tables comes from here, so
/// expiry behaviour is driven deterministically in tests rather than by SQL `datetime('now')`.
pub trait Clock: Send + Sync {
    /// Milliseconds since the Unix epoch, UTC.
    fn now_millis(&self) -> i64;
}

pub struct SystemClock;
impl Clock for SystemClock {
    fn now_millis(&self) -> i64 { chrono::Utc::now().timestamp_millis() }
}

/// Test fake. Public and unconditionally compiled: `crates/server/tests/` links the lib WITHOUT
/// `cfg(test)`, and `crates/server` has no `test-utils` feature to gate on.
pub struct FixedClock(std::sync::atomic::AtomicI64);
impl FixedClock {
    pub fn new(millis: i64) -> Self;
    pub fn set(&self, millis: i64);
    pub fn advance(&self, delta_millis: i64);
}
impl Clock for FixedClock { fn now_millis(&self) -> i64; }

/// Source of opaque secrets: session tokens and pre-auth binding-cookie values.
pub trait TokenSource: Send + Sync {
    /// 32 bytes (256 bits) of CSPRNG output, base64url-encoded WITHOUT padding (43 chars).
    fn generate_token(&self) -> String;
}

pub struct OsTokenSource;
impl TokenSource for OsTokenSource { /* rand::rng().random::<[u8; 32]>() -> URL_SAFE_NO_PAD */ }

/// Test fake returning a scripted sequence; panics when exhausted so a test can never silently
/// fall back to real randomness.
pub struct ScriptedTokenSource(std::sync::Mutex<std::collections::VecDeque<String>>);
impl ScriptedTokenSource { pub fn new(tokens: impl IntoIterator<Item = String>) -> Self; }
impl TokenSource for ScriptedTokenSource { fn generate_token(&self) -> String; }

/// Lowercase hex SHA-256, 64 chars. Deliberately a free function, not a trait: it must be
/// deterministic, so a fake could vary nothing useful, and a trait would let an implementation
/// drift from the encoding stored in the database.
pub fn hash_token(token: &str) -> String;
```

**Sibling alignment (rubric 9).** Read `crates/server/src/routes/oauth.rs:210-229` first: `generate_secret()` and `hash_sha256_hex()` already exist there. `hash_token` MUST produce byte-identical output to `hash_sha256_hex` (lowercase `{:02x}` hex of the SHA-256 digest) so a hash written by one path matches a hash recomputed by the other. Do NOT modify `routes/oauth.rs` in this task — the OAuth tasks (009/010/011) migrate its call sites.

**Symbol grounding:** This task introduces `hash_token()`, the `Clock` trait with `now_millis()`, `SystemClock`, `FixedClock` (`new()`, `set()`, `advance()`), the `TokenSource` trait with `generate_token()`, `OsTokenSource` and `ScriptedTokenSource`. `hash_token()` must produce byte-identical output to the pre-existing `hash_sha256_hex()` in `crates/server/src/routes/oauth.rs`, which this task does not modify.


## Allowed moves
[
  "Add exactly one dependency line (`base64 = \"0.22\"`) to crates/server/Cargo.toml.",
  "Regenerate Cargo.lock; add only the existing `base64` package to the `server` dependency list.",
  "Add exactly one `pub mod auth;` line to crates/server/src/lib.rs.",
  "Create crates/server/src/auth/mod.rs with only the doc comment and `pub mod seams;`.",
  "Create crates/server/src/auth/seams.rs with the interfaces above plus the colocated test module.",
  "Do not touch crates/server/src/routes/oauth.rs, and do not gate the fakes behind #[cfg(test)]."
]


## STOP triggers
[
  "`crates/server/src/auth/` already exists (a sibling task created it) — STOP; merge rather than create and re-check the conflict edge with 007/011.",
  "Any temptation to change routes/oauth.rs's generate_secret/hash_sha256_hex — out of scope here.",
  "Any temptation to put the fakes behind #[cfg(test)] — integration tests in crates/server/tests/ link the lib without cfg(test) and would not see them.",
  "`hash_token` output does not match `hash_sha256_hex` for the same input — STOP, the encodings must be identical.",
  "base64 0.22 fails to resolve at the workspace level — STOP rather than bumping other crates' versions.",
  "Editing the [dev-dependencies] block of crates/server/Cargo.toml — that belongs to task 006.",
  "Cargo.lock changes anything except adding `base64` to the existing `server` package dependency list — STOP rather than accepting unrelated resolution drift."
]


Declared decision points (from the spec; do not edit here):
- DP1: Execution reaches the additive SQLite migration task and requires explicit approval before applying an irreversible production schema migration.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
1. `WAI_ROOT="$HOME/.agents/wai"; test -x "$WAI_ROOT/scripts/task-gate.sh"; WAI_TYPECHECK_CMD="cargo fmt --all -- --check" WAI_TEST_CMD="cargo test -p server auth::seams" bash "$WAI_ROOT/scripts/task-gate.sh" local-node-browser-oauth 002` exits 0.
2. `cargo test -p server auth::seams` — 5 tests green.
3. `cargo clippy -p server --all-targets --all-features -- -D warnings` clean.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash "$HOME/.agents/wai/scripts/task-gate.sh" local-node-browser-oauth 002` exits 0
