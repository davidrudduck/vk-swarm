# ADR-0004 — ACP transcript channel bounded with drop-on-full

- **Status:** accepted
- **Date:** 2026-06-26
- **Workstream:** vk-swarm-node-foundations

## Context

The ACP (Agent-Code Protocol) executor uses `mpsc::unbounded_channel::<AcpEvent>()` for its
transcript event stream (`crates/executors/src/executors/acp/harness.rs:259`; the sender is stored as
`event_tx: mpsc::UnboundedSender<AcpEvent>` in `acp/client.rs:10`). An unbounded channel provides
backpressure-free delivery but at the cost of unbounded memory growth — a flooded or stalled consumer
can let the channel accumulate without limit, eventually causing OOM. This pattern was already caught
and fixed upstream (vibe-kanban), where it was vetted with a bounded channel + drop policy for transcript
events. vk-swarm has not yet forward-ported that fix.

## Decision

**Replace `mpsc::unbounded_channel` with `mpsc::channel(N)` and drop-on-full** for the ACP transcript
event channel:

- `harness.rs:259` — change channel construction to `mpsc::channel(N)` where `N` is a compile-time
  constant (suggested: 1024; configurable via a const or env var at wire-up).
- `client.rs:10` — change `event_tx` type to `mpsc::Sender<AcpEvent>` (bounded sender).
- The send site uses `try_send`; on `Err(TrySendError::Full(_))` the event is **silently dropped**
  (drop-on-full). On disconnect `TrySendError::Closed(_)` is a no-op (executor is stopping).

This is a **lossy-delivery** semantic: a burst of transcript events from a fast agent may cause
visible lines to be missing from the UI log. This is acceptable because:
1. Transcript events are display/diagnostic — not control flow, not persisted state.
2. The bounded channel still drains faster than any real agent can produce under normal conditions.
3. Upstream already made and shipped this tradeoff, so the risk surface is already well-understood.

**This decision DOES NOT apply to control-flow channels** (stop signals, process lifecycle). Those must
remain lossless (use `Receiver::recv` / unbounded, or separate channels with proper error handling).

## Consequences

- Memory growth under a stalled/flood consumer is bounded (SC7a).
- Under extreme burst, some transcript log lines may be silently dropped — the UI log is no longer
  100% complete. This is a known tradeoff, not a regression; upstream has shipped with this policy.
- The `try_send` call site must be updated; any callers asserting `send` success must use `if let`.
- A test for the drop-on-full path (channel full → no panic, no deadlock, counter/log) is required per
  the spec's test strategy (SC7).

## Alternatives considered

- **Keep unbounded** — rejected: OOM risk under flood; upstream already fixed this for good reason.
- **Block on full (backpressure)** — rejected: would stall the ACP harness task, blocking the agent
  driver loop while the consumer catches up; the harness must never block waiting on UI delivery.
- **Separate lossless + lossy channels** — over-engineering for transcript-only events; unnecessary
  complexity. Keep it simple: one bounded channel, drop-on-full, exactly as upstream did.
