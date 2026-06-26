---
id: "501"
phase: 5
title: Bound the ACP transcript event channel with drop-on-full
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - crates/executors/src/executors/acp/harness.rs
  - crates/executors/src/executors/acp/client.rs
irreversible: false
scope_test: "crates/executors/src/executors/acp/client.rs"
allowed_change: edit
covers_criteria: [SC7]
---
## Failing test (write first)

Add a unit test to the **bottom** of `crates/executors/src/executors/acp/client.rs` (the file
currently has no `#[cfg(test)]` module — add one). It proves the drop-on-full semantic: a flooded
channel does not panic or deadlock, and exactly `capacity` events survive.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn transcript_event_drops_when_channel_full_instead_of_blocking() {
        // Bounded event channel, capacity 2. send_event uses try_send, so a
        // third send into a full channel must drop (not block / not panic).
        let (tx, mut rx) = mpsc::channel::<AcpEvent>(2);
        let client = AcpClient::new(tx);

        client.record_user_prompt_event("a");
        client.record_user_prompt_event("b");
        // No recv yet; channel is full. Third call must not deadlock the test.
        client.record_user_prompt_event("c");

        let mut count = 0;
        while rx.try_recv().is_ok() {
            count += 1;
        }
        assert_eq!(
            count, 2,
            "exactly two transcript events should reach the receiver; the rest are dropped"
        );
    }
}
```

This test fails to compile against current code: `mpsc::channel::<AcpEvent>(2)` returns a bounded
`Sender`, but `AcpClient::new` today takes `mpsc::UnboundedSender<AcpEvent>`. After the change below
it compiles and passes.

## Change

For each file in `files:`:

### File 1 — `crates/executors/src/executors/acp/client.rs`

- **Anchor:** the `AcpClient` struct field `event_tx` (~L9-11) and `AcpClient::new` (~L13-17).
- **Before:**
```rust
/// ACP client that handles agent-client protocol communication
pub struct AcpClient {
    event_tx: mpsc::UnboundedSender<AcpEvent>,
}

impl AcpClient {
    /// Create a new ACP client
    pub fn new(event_tx: mpsc::UnboundedSender<AcpEvent>) -> Self {
        Self { event_tx }
    }
```
- **After:**
```rust
/// ACP client that handles agent-client protocol communication
pub struct AcpClient {
    event_tx: mpsc::Sender<AcpEvent>,
}

impl AcpClient {
    /// Create a new ACP client
    pub fn new(event_tx: mpsc::Sender<AcpEvent>) -> Self {
        Self { event_tx }
    }
```

- **Anchor:** the `send_event` method (~L23-28).
- **Before:**
```rust
    /// Send an event to the event channel
    fn send_event(&self, event: AcpEvent) {
        if let Err(e) = self.event_tx.send(event) {
            warn!("Failed to send ACP event: {}", e);
        }
    }
```
- **After:** (mirrors upstream `send_transcript_event`, vibe-kanban `crates/executors/src/executors/acp/client.rs:48-58`)
```rust
    /// Send a transcript-class event to the event channel.
    ///
    /// Uses `try_send` so producers are never blocked by a slow forwarder; the
    /// event is dropped with a `warn!` if the channel is full (drop-on-full per
    /// ADR-0004). On a closed channel (receiver dropped during shutdown) the
    /// send is a quiet no-op.
    fn send_event(&self, event: AcpEvent) {
        match self.event_tx.try_send(event) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                warn!("ACP event channel full; dropping transcript event");
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                // Receiver dropped — happens during shutdown. Quiet.
            }
        }
    }
```
(`use tokio::sync::mpsc;` is already present at L3, so `mpsc::error::TrySendError` needs no new import. `debug`/`warn` are already imported at L4.)

### File 2 — `crates/executors/src/executors/acp/harness.rs`

- **Anchor:** the typed-event channel construction inside `bootstrap_acp_connection` (~L256-259). Add a
  module-level capacity constant and switch construction to bounded.
- **Before:**
```rust
                        // Create event and raw channels
                        // Typed events available for future use; raw lines forwarded and persisted
                        let (event_tx, mut event_rx) =
                            mpsc::unbounded_channel::<crate::executors::acp::AcpEvent>();
```
- **After:** (mirrors upstream `mpsc::channel::<AcpEvent>(ACP_EVENT_CHANNEL_CAPACITY)`, vibe-kanban `harness.rs:352-355`)
```rust
                        // Create event and raw channels
                        // Typed events available for future use; raw lines forwarded and persisted
                        let (event_tx, mut event_rx) =
                            mpsc::channel::<crate::executors::acp::AcpEvent>(
                                ACP_EVENT_CHANNEL_CAPACITY,
                            );
```

- **Anchor:** add a module-level constant. Insert it immediately **after** the `use` block (after L23,
  the line `};`) and **before** the `/// Reusable harness for ACP-based conns` doc comment at L25.
- **Before:** (the boundary between the imports and the struct doc comment, L23-26)
```rust
    executors::{ExecutorError, ExecutorExitResult, SpawnContext, SpawnedChild, acp::AcpEvent},
};

/// Reusable harness for ACP-based conns (Gemini, Qwen, etc.)
pub struct AcpAgentHarness {
```
- **After:**
```rust
    executors::{ExecutorError, ExecutorExitResult, SpawnContext, SpawnedChild, acp::AcpEvent},
};

/// Capacity of the bounded ACP transcript-event channel (drop-on-full per ADR-0004).
///
/// Transcript events are display/diagnostic only; under flood the channel drops
/// the oldest-unsent lines rather than growing without bound (OOM guard).
const ACP_EVENT_CHANNEL_CAPACITY: usize = 1024;

/// Reusable harness for ACP-based conns (Gemini, Qwen, etc.)
pub struct AcpAgentHarness {
```

## Allowed moves

- ONLY: change `event_tx`'s type from `UnboundedSender<AcpEvent>` to `Sender<AcpEvent>` in
  `client.rs` (struct field + `new` parameter); rewrite `send_event`'s body to `try_send` with the
  drop-on-full match; add the `#[cfg(test)]` module at the bottom of `client.rs`.
- ONLY: in `harness.rs`, add the `ACP_EVENT_CHANNEL_CAPACITY` const and switch the `event_tx`/`event_rx`
  construction from `mpsc::unbounded_channel` to `mpsc::channel(ACP_EVENT_CHANNEL_CAPACITY)`.
- Do NOT touch the `log_tx` String channel at `harness.rs:175`
  (`mpsc::unbounded_channel::<String>()`) — ADR-0004 scopes this task to the `AcpEvent` channel only
  (`harness.rs:259`/`client.rs:10`); `log_tx` stays unbounded.
- Do NOT touch any control-flow channel (`shutdown_tx`/`shutdown_rx` watch channel, `exit_signal`
  oneshot). ADR-0004 requires those stay lossless.
- Do NOT change `record_user_prompt_event`, `session_notification`, or any `acp::Client` trait method
  body — they already call `send_event`, which now carries the new semantic.

## STOP triggers

- `client.rs` `event_tx` is not typed `mpsc::UnboundedSender<AcpEvent>` at the anchor, or `send_event`
  does not contain `self.event_tx.send(event)`.
- `harness.rs` does not contain `mpsc::unbounded_channel::<crate::executors::acp::AcpEvent>()` at ~L259.
- More than one send site for `event_tx` exists (verified: the only `.send` site is
  `client.rs:25`; the `event_tx` in `harness.rs` is only cloned into `AcpClient::new`). If a second
  real send site appears, STOP.
- The change would require editing any file not in `files:` (e.g. `acp/mod.rs` for `AcpEvent`).

(Note: `#[tokio::test]` is already in wide use in the `executors` crate — e.g.
`crates/executors/src/executors/opencode.rs:561` — so the new test needs no `Cargo.toml` change;
`executors`'s `tokio` already enables the test macros/runtime.)

## Sibling alignment

This is a forward-port of an existing upstream pattern, not a novel impl. Upstream
(`/home/david/Code/vibe-kanban`) split the channel into transcript-class (`try_send`, drop-on-full)
and control-class (`send().await`, backpressure) because upstream's `AcpEvent` channel **also**
carries control events (`ApprovalRequested`, `ApprovalResponse`, `RequestPermission`) that drive
cancellation. **The fork does NOT have that split, and does not need it:** in the fork's `client.rs`,
`request_permission` auto-approves inline (its outcome does not depend on the event being delivered),
and all control/lifecycle signals (`SessionStart`, `Done`, `Error`) travel on the separate `log_tx`
**String** channel, not on `AcpEvent`. The fork's `AcpEvent` channel is therefore **entirely
display-class**, so a single blanket bounded + drop-on-full satisfies ADR-0004's "control-flow stays
lossless" without importing upstream's transcript/control split, `LogSender`, approvals, or
cancellation machinery. Mirrored semantic source: vibe-kanban `client.rs:48-58` (`send_transcript_event`)
and `harness.rs:39,352-355` (`ACP_EVENT_CHANNEL_CAPACITY` const + `mpsc::channel`).

Capacity divergence (intentional, cited): ADR-0004 suggests `1024`; upstream shipped `2048`
(`harness.rs:39`). This task uses `1024` per the ADR. Either is acceptable; the value is not load-bearing.

## Done when

`WAI_TYPECHECK_CMD="cargo check -p executors" WAI_TEST_CMD="cargo test -p executors transcript_event_drops_when_channel_full_instead_of_blocking" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-node-foundations 501` exits 0
