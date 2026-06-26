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
  - crates/executors/default_profiles.json
irreversible: false
scope_test: "crates/executors/src/executors/qa_mock.rs"
allowed_change: mixed
covers_criteria: [SC7]
---
## Failing test (write first)
In `crates/executors/src/executors/qa_mock.rs` `#[cfg(test)] mod tests` (port upstream's tests, then ADD
a selectability test proving the variant resolves through THIS fork's profile system):
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
This passes ONLY if a `QA_MOCK` profile is present in `default_profiles.json` (see Change C).

## Change
Faithful forward-port of `qa_mock` from upstream `/home/david/Code/vibe-kanban`, **adapted to this
fork's `#[enum_dispatch]` convention** (verified: breakdown-review R3). Key fork facts:
- `CodingAgent` (`crates/executors/src/executors/mod.rs:101`) is an `#[enum_dispatch]` enum with **bare
  variants whose name equals the backing struct** (`ClaudeCode,` ↔ `pub struct ClaudeCode` in
  `claude.rs:55`; `Opencode,` ↔ `opencode.rs:123`). NOT upstream's `QaMock(QaMockExecutor)` tuple shape.
- `BaseCodingAgent` is **auto-derived** via `#[strum_discriminants(name(BaseCodingAgent))]`
  (`mod.rs:92`) — adding the `CodingAgent::QaMock` variant AUTOMATICALLY creates `BaseCodingAgent::QaMock`.
  You do NOT (and must not) hand-add it to a `BaseCodingAgent` enum, and `profile.rs` needs no edit.
- `ExecutorConfigs::get_coding_agent` (`profile.rs:398`) resolves a profile via `self.executors` (a
  `HashMap` loaded from `crates/executors/default_profiles.json` by `include_str!`) — so qa_mock needs a
  `QA_MOCK` entry there to be selectable.
- The MCP match is `get_mcp_config` in `mod.rs:117` with a `_ =>` wildcard at `:152` — a new variant
  falls through automatically; **`mcp_config.rs` needs no edit** (it has no `CodingAgent` match here).

- **File:** `crates/executors/src/executors/qa_mock.rs` (NEW) — copy upstream's `qa_mock.rs` (456 lines)
  and **rename `QaMockExecutor` → `QaMock`** throughout (variant-name == struct-name convention). Adapt
  to this fork's `StandardCodingAgentExecutor` trait signature (compare against `opencode.rs` — see
  Sibling alignment). It MUST support a deterministic, kill-mid-run mode so Phase 3 can `kill -9` it
  (the whole reason for the port — spec Test strategy / SC7d).
- **File:** `crates/executors/src/executors/mod.rs` — add `use crate::executors::qa_mock::QaMock;` (with
  the other executor `use`s ~L18), `pub mod qa_mock;` (with the other `pub mod`s ~L52), the bare
  **`QaMock,`** variant in `enum CodingAgent` (with the others, L102-113), and a `Self::QaMock(_) =>
  vec![]` arm in the EXHAUSTIVE `capabilities()` match (L167-180, mirror `Self::Opencode(_) |
  Self::Copilot(_)`).
- **File:** `crates/executors/default_profiles.json` — add a `QA_MOCK` executor key mirroring the
  simplest existing entry (e.g. `OPENCODE`)'s shape, so `get_coding_agent(BaseCodingAgent::QaMock)`
  resolves. READ the file's structure first; match the JSON shape exactly (the discriminant serializes
  as `QA_MOCK` per strum's SCREAMING_SNAKE convention — verify against an existing key).

**Compiler-driven ripple (EXPECTED):** after the edits, run `cargo check -p executors`. The new variant
will trigger non-exhaustive-match errors at any OTHER exhaustive `match self` on `CodingAgent` (e.g.
`no_context()` in mod.rs). For EACH, add a `Self::QaMock(_) => …` arm mirroring the simplest executor.

## Allowed moves
Add the qa_mock executor + the minimum wiring to compile and be selectable via the profile system.
Mirror upstream semantics; adapt naming/structure to this fork (bare variant, struct named `QaMock`).
Do NOT change any other executor's behaviour. Do NOT touch `crates/services/src/services/container.rs`
(verified: this fork does not match `CodingAgent` variants there — Phase 3 owns it). Do NOT edit
`profile.rs` (the discriminant auto-derives) or `mcp_config.rs` (wildcard covers it).

## Sibling alignment
`qa_mock.rs` is a new executor implementing `StandardCodingAgentExecutor`. Read a simple sibling
(`opencode.rs` or `claude.rs`): list the trait methods it implements, its `spawn`/`spawn_follow_up`
shape, the bare-variant↔struct-name convention, and capability/no_context defaults. The ported qa_mock
must implement the SAME trait surface this fork requires (upstream's may differ). Record every
divergence from upstream's `qa_mock.rs` in the ledger.

## STOP triggers
- The compiler flags an exhaustive `CodingAgent` match in a file NOT listed in `files:` → this is the
  expected ripple: ADD that file to this task's `files:`, add the `Self::QaMock(_)` arm, record it in
  the ledger. (Do not silently edit an unlisted file — keep the gate's allow-list honest.)
- Upstream `qa_mock.rs` does not compile against this fork's `StandardCodingAgentExecutor` (signature
  drift) → adapt to the local trait, recording each change; do NOT change the trait.
- `get_coding_agent(BaseCodingAgent::QaMock)` still returns `None` after adding the JSON entry → inspect
  how `default_profiles.json` keys map to `BaseCodingAgent` (strum serialize) and fix the key spelling;
  do NOT edit `profile.rs` logic.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p executors" WAI_TEST_CMD="cargo test -p executors qa_mock" bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-node-foundations 201` exits 0
