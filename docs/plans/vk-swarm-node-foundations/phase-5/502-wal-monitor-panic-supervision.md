---
id: "502"
phase: 5
title: Supervise the WAL-monitor background task against panic
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - crates/db/src/wal_monitor.rs
  - crates/db/Cargo.toml
irreversible: false
scope_test: "crates/db/src/wal_monitor.rs"
allowed_change: edit
covers_criteria: [SC7]
---
## Failing test (write first)

Add these tests inside the existing `#[cfg(test)] mod tests` block in
`crates/db/src/wal_monitor.rs` (the module already exists at the bottom of the file with
`test_default_config` / `test_get_wal_size_nonexistent`). They are ported verbatim from upstream
`vibe-kanban/crates/db/src/wal_monitor.rs:423-460`:

```rust
    #[tokio::test]
    async fn supervised_run_passes_through_normal_completion() {
        let result = supervised_run("test", async {
            // no-op
        })
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn supervised_run_catches_panic_and_reports_message() {
        let result = supervised_run("test", async {
            panic!("synthetic boom for test");
        })
        .await;
        assert!(matches!(result, Err(ref msg) if msg.contains("synthetic boom for test")));
    }

    #[tokio::test]
    async fn supervised_run_catches_non_string_panic_with_fallback_marker() {
        let result = supervised_run("test", async {
            std::panic::panic_any(123_u32);
        })
        .await;
        assert!(
            matches!(result, Err(ref msg) if msg.contains("<non-string panic>")),
            "non-string panic payload should yield the fallback marker, got {result:?}"
        );
    }
```

These fail to compile against current code (no `supervised_run` symbol exists). After adding the
function below they compile and pass. Note: the existing tests use `#[test]` (sync); the multi-thread
runtime for `#[tokio::test]` is already available via `tokio = { features = ["rt-multi-thread",
"macros"] }` in `[dev-dependencies]` (Cargo.toml:34).

## Change

For each file in `files:`:

### File 1 — `crates/db/Cargo.toml`

The fork's `db` crate has **no `futures` dependency** and there is no workspace-level `futures`.
`supervised_run` needs `futures::FutureExt::catch_unwind`. Add the dependency (mirroring the version
the `executors` crate already declares: `futures = "0.3.31"`).

- **Anchor:** the `[dependencies]` list, after the `rusqlite` line (Cargo.toml:21).
- **Before:**
```toml
rusqlite = { version = "0.32", features = ["bundled", "backup"] }

[features]
```
- **After:**
```toml
rusqlite = { version = "0.32", features = ["bundled", "backup"] }
futures = "0.3.31"

[features]
```

### File 2 — `crates/db/src/wal_monitor.rs`

**(a) Imports.** Add `std::panic::AssertUnwindSafe` and `futures::FutureExt` (mirroring upstream
`wal_monitor.rs:14-20`).

- **Anchor:** the import block (L15-21).
- **Before:**
```rust
use std::path::{Path, PathBuf};
use std::time::Duration;

use sqlx::SqlitePool;
use tokio::sync::mpsc;

use crate::DbMetrics;
```
- **After:**
```rust
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::time::Duration;

use futures::FutureExt;
use sqlx::SqlitePool;
use tokio::sync::mpsc;

use crate::DbMetrics;
```

**(b) Wrap the spawned task.** Inside `WalMonitor::spawn`, wrap `monitor.run(rx)` in `supervised_run`
(mirroring upstream `wal_monitor.rs:144-146`).

- **Anchor:** the `tokio::spawn(monitor.run(rx));` line in `WalMonitor::spawn` (L151).
- **Before:**
```rust
        tokio::spawn(monitor.run(rx));
        WalMonitorHandle { tx }
```
- **After:**
```rust
        tokio::spawn(async move {
            let _ = supervised_run("wal_monitor", monitor.run(rx)).await;
        });
        WalMonitorHandle { tx }
```

**(c) Add the `supervised_run` helper.** Insert it immediately **after** the `get_wal_size` function
and **before** the `#[cfg(test)] mod tests` block (ported verbatim from upstream
`wal_monitor.rs:369-391`).

- **Anchor:** the gap between `get_wal_size`'s closing `}` and `#[cfg(test)]` (L366-368).
- **Before:**
```rust
pub fn get_wal_size(db_path: impl AsRef<Path>) -> u64 {
    let wal_path = db_path.as_ref().with_extension("sqlite-wal");
    std::fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
```
- **After:**
```rust
pub fn get_wal_size(db_path: impl AsRef<Path>) -> u64 {
    let wal_path = db_path.as_ref().with_extension("sqlite-wal");
    std::fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0)
}

/// Run `fut` to completion, catching any panic and logging it at error level.
///
/// Returns `Ok(())` on normal completion, `Err(panic_message)` on panic. This
/// lets long-running background tasks fail noisily instead of being silently
/// swallowed by a dropped `JoinHandle`.
async fn supervised_run<F>(name: &'static str, fut: F) -> Result<(), String>
where
    F: std::future::Future<Output = ()>,
{
    match AssertUnwindSafe(fut).catch_unwind().await {
        Ok(()) => Ok(()),
        Err(panic) => {
            let msg = panic
                .downcast_ref::<&'static str>()
                .map(|s| (*s).to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "<non-string panic>".to_string());
            tracing::error!(task = name, panic = %msg, "background task panicked");
            Err(msg)
        }
    }
}

#[cfg(test)]
mod tests {
```

## Allowed moves

- ONLY: add `futures = "0.3.31"` to `crates/db/Cargo.toml` `[dependencies]`.
- ONLY: add the two `use` lines (`std::panic::AssertUnwindSafe`, `futures::FutureExt`) to `wal_monitor.rs`.
- ONLY: wrap the existing `tokio::spawn(monitor.run(rx))` body in the `async move { ... supervised_run ... }`.
- ONLY: add the `supervised_run` fn between `get_wal_size` and the test module.
- ONLY: add the three ported tests inside the existing `mod tests`.
- Do NOT change `WalMonitor::spawn`'s signature, `WalMonitor`'s fields (`metrics` stays — the fork has
  it; upstream removed it, that refactor is out of scope), `WalMonitor::run`, `run_checkpoint`, or any
  checkpoint logic. Upstream's unrelated refactors (`Pool<Sqlite>` for `SqlitePool`, `Option<Interval>`
  truncate timer) are explicitly OUT of scope — this task is one concern: panic supervision.
- Do NOT change "catch + log" into "restart". Upstream's `supervised_run` catches the panic, logs at
  error, returns `Err(msg)`; it does NOT restart the task (see "Spec correction" below).

## STOP triggers

- `wal_monitor.rs:151` is not exactly `tokio::spawn(monitor.run(rx));`.
- The import block at L15-21 does not match the Before text.
- `get_wal_size` is not immediately followed by `#[cfg(test)]`.
- `cargo check -p db` fails to resolve `futures` after the Cargo.toml edit (means the version pin is
  wrong — STOP and reconcile against `crates/executors/Cargo.toml`).
- The change would require editing any file not in `files:`.

## Sibling alignment

Grepped the fork for an existing panic-supervision / `catch_unwind` helper: **none exists** in the
workspace. This is therefore a faithful forward-port with **no sibling** — it mirrors upstream
`vibe-kanban/crates/db/src/wal_monitor.rs:375-391` (`supervised_run`) and its `spawn` wrapping at
`:144-146`, verbatim.

## Spec correction (record in decisions-ledger)

The Phase-5 task brief and the spec §5 say "panic-**supervising/restart** wrapper". Upstream's actual
shipped fix does NOT restart — `supervised_run` catches the panic, logs it at `error`, and returns
`Err(msg)`; the task then ends (it is not re-spawned). This task mirrors upstream reality (catch + log,
no restart), which is the faithful forward-port. SC7b ("WAL monitor survives an injected panic") is
satisfied: the panic no longer silently kills the process / is no longer swallowed by a dropped
`JoinHandle` — it is caught and surfaced. A genuine restart loop was not part of the upstream fix and
is out of scope here.

Also note (not a code change): the fork's `db` crate gains a **new `futures` dependency** that neither
the spec nor ADR-0004-adjacent docs mention; it is required because upstream's `supervised_run` uses
`futures::FutureExt::catch_unwind` and the fork's `db` crate did not previously depend on `futures`.

## Done when

`WAI_TYPECHECK_CMD="cargo check -p db" WAI_TEST_CMD="cargo test -p db supervised_run" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-node-foundations 502` exits 0
