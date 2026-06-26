---
id: "201"
phase: 2
title: Forward-port qa_mock executor and wire into CodingAgent (single commit)
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - crates/executors/src/executors/qa_mock.rs
  - crates/executors/src/executors/mod.rs
  - crates/executors/src/profile.rs
  - crates/executors/src/mcp_config.rs
irreversible: false
scope_test: "crates/executors/src/executors/qa_mock.rs"
allowed_change: mixed
covers_criteria: [SC7]
---
## Failing test (write first)
In `crates/executors/src/executors/qa_mock.rs` `#[cfg(test)] mod tests` (port upstream's tests, then
ADD a selectability test proving the variant is wired into this fork's profile system):
```rust
#[test]
fn test_qa_mock_resolves_through_profile_system() {
    use crate::profile::{ExecutorConfigs, ExecutorProfileId};
    use crate::executors::{BaseCodingAgent, CodingAgent};
    let cfg = ExecutorConfigs::get_cached();
    let agent = cfg.get_coding_agent(&ExecutorProfileId::new(BaseCodingAgent::QaMock));
    assert!(matches!(agent, Some(CodingAgent::QaMock(_))));
}
```
(If this fork's default `ExecutorConfigs` does not auto-register every `BaseCodingAgent`, adjust the
test to construct the profile the same way the other executors' tests do — record the chosen path.)

## Change
This is a faithful forward-port of `qa_mock` from the upstream reference repo at
`/home/david/Code/vibe-kanban`, ADAPTED to this fork's diverged executor architecture (this fork
dispatches via `ExecutorConfigs::get_coding_agent` + profiles, NOT the per-variant match upstream uses;
`capabilities()` here is EXHAUSTIVE with no wildcard — see ledger Trap 3).

- **File:** `crates/executors/src/executors/qa_mock.rs` (NEW) — copy
  `/home/david/Code/vibe-kanban/crates/executors/src/executors/qa_mock.rs` (456 lines) and adapt to this
  fork's `StandardCodingAgentExecutor` trait signature (compare against a sibling, e.g. `opencode.rs` —
  see Sibling alignment). It must support a deterministic, kill-mid-run mode so Phase 3 can `kill -9`
  it (that is the whole reason for the port — see spec Test strategy / SC7d).
- **File:** `crates/executors/src/executors/mod.rs` — add (mirror upstream mod.rs:18/52/135/211, but at
  this fork's line numbers): `use crate::executors::qa_mock::QaMockExecutor;`, `pub mod qa_mock;`, the
  `QaMock(QaMockExecutor)` variant in `enum CodingAgent`, and a `Self::QaMock(_) => vec![]` arm in
  `capabilities()` (the exhaustive match at L167-180, mirror `Self::Opencode(_) | Self::Copilot(_)`).
- **File:** `crates/executors/src/profile.rs` — add `QaMock` to the `BaseCodingAgent` enum and wire it
  into `get_coding_agent` (L398) so a `QaMock` profile yields `CodingAgent::QaMock(QaMockExecutor)`, plus
  any `from_str`/kebab/serialization arm `BaseCodingAgent` requires. Read this file to place each arm.
- **File:** `crates/executors/src/mcp_config.rs` — add a `QaMock` arm if its `CodingAgent` match is
  exhaustive (upstream has one at mcp_config.rs:410: `CodingAgent::QaMock(_) => Passthrough`). If this
  fork's match has a `_ =>` wildcard, NO edit is needed — remove this file from `files:` and record why.

**Compiler-driven ripple (EXPECTED):** after the edits above, run `cargo check -p executors`. Adding the
variant will produce non-exhaustive-match errors at any OTHER exhaustive `match self { … }` on
`CodingAgent`/`BaseCodingAgent` (e.g. `no_context()` in mod.rs). For EACH, add a `Self::QaMock(_) => …`
arm mirroring the simplest existing executor's behaviour. If the compiler flags a file NOT in `files:`,
see the STOP trigger.

## Allowed moves
Add the qa_mock executor and the minimum wiring to make it compile and be selectable via the profile
system. Mirror upstream semantics; adapt structure to this fork. Do NOT change any other executor's
behaviour, and do NOT touch `crates/services/src/services/container.rs` (verified: this fork does not
match `CodingAgent` variants there — keep it out of this task so Phase 3 owns it cleanly).

## Sibling alignment
`qa_mock.rs` is a new executor implementing `StandardCodingAgentExecutor`. Read a simple sibling
(`crates/executors/src/executors/opencode.rs` or `copilot.rs`): list the trait methods it implements,
its spawn/follow-up shape, error handling, and any capability/no_context defaults. The ported qa_mock
must implement the SAME trait surface this fork requires (upstream's may differ). Record every
divergence from upstream's qa_mock.rs in the ledger.

## STOP triggers
- The compiler flags an exhaustive `CodingAgent`/`BaseCodingAgent` match in a file NOT listed in
  `files:` → this is the expected ripple: ADD that file to this task's `files:` list, add the
  `Self::QaMock(_)` arm, and record the addition in the ledger. (Do not silently edit an unlisted file —
  update the task file so the gate's file-allow-list stays honest.)
- `qa_mock.rs` from upstream does not compile against this fork's `StandardCodingAgentExecutor` trait
  (signature drift) → adapt to the local trait, recording each change; do NOT change the trait.
- A `BaseCodingAgent` config-version migration (`config/versions/v*.rs`) breaks exhaustively → STOP and
  record; historical config conversion of a brand-new test-only variant may warrant a wildcard/skip arm
  rather than a real migration path. Escalate if unclear.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p executors" WAI_TEST_CMD="cargo test -p executors qa_mock" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-node-foundations 201` exits 0
