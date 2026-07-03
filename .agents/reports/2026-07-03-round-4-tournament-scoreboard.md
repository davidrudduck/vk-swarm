# Tournament Adversarial Review — Round 4 (2026-07-03)

**Branch:** `vk-swarm-hive-redesign-p47` (HEAD before this round: `82d05b29`)
**Target:** Phases 4-7 hive-redesign diff + pre-existing debt fixes
**Governing intent:** frozen spec `docs/superpowers/specs/2026-06-26-vk-swarm-hive-redesign.md` (SHA `2ac86436…`), 7-phase plan, frozen WS CONTRACT

---

## Tournament format

Three challengers (Claude CLI / Codex CLI / Gemini CLI) independently reviewed the diff in a
discovery phase. Each finding can earn up to 3 points:
1. **+1** for a valid cited finding (verified against real code)
2. **+1** for a working remediation (concrete patch that compiles + passes tests)
3. **+1** if the remediation survives adversarial peer review by a different model

If the discoverer's remediation is invalid, a peer can steal all 3 points by providing a working
alternative the discoverer cannot disprove.

## Executors that ran

| Model | How | Status |
|-------|-----|--------|
| Claude CLI (Opus) | local checkout, `claude -p --permission-mode plan` | ✅ ran (discovery + peer review of CF1) |
| Codex CLI | local checkout, `codex` | ✅ ran (discovery) |
| Gemini CLI | local checkout, `gemini` | ✅ ran (discovery + peer review of CF2) |

No 2-of-3 resilience fallback was needed — all three ran cleanly.

---

## Discovery findings

### Claude (discovery) — 2 findings

| # | Issue | Tag | Valid? | Remediation? |
|---|-------|-----|--------|--------------|
| F1 | `resend_from_seq` false positive — two independent `.difference().is_some()` checks instead of one intersection | [SHOULD-FIX] | ✅ yes | ✅ `.any()` single-intersection |
| F2 | Missing trailing newline in 4 new files | [INFO] | ✅ yes | ✅ append `\n` |

### Codex (discovery) — 2 findings

| # | Issue | Tag | Valid? | Remediation? |
|---|-------|-----|--------|--------------|
| F1 | Digest re-stream drops fencing token — `restream_row_to_ws_op` copies `None` from the row; hive rejects as stale | [BLOCKING] | ✅ yes | ✅ `token_by_task` map from `active_assignments` |
| F2 | Same as Claude F1 (`resend_from_seq` false positive) | [BLOCKING] | ✅ yes | ✅ same `.any()` fix |

### Gemini (discovery) — 2 findings

| # | Issue | Tag | Valid? | Remediation? |
|---|-------|-----|--------|--------------|
| F1 | Duplicated test boilerplate across 7 integration test files | [INFO] | ✅ yes (real duplication) | extract to `tests/common/mod.rs` |
| F2 | `NodeTaskAttemptRepository` lacks transaction/executor support | [INFO] | ✅ yes (real limitation) | generic executor bound |

Gemini's verdict: **APPROVE** (no [BLOCKING]/[SHOULD-FIX]).

---

## Consolidation

After dedupe (merge same-location findings, keep strongest tag, discard uncited):

| # | Issue (cited) | Tag | Accepted? | Impact if shipped | Remediation |
|---|---------------|-----|-----------|-------------------|-------------|
| CF1 | `node_runner.rs:1179` — `restream_row_to_ws_op` copies `fencing_token: None` from the row; hive SC3 guard (`session.rs:2123` `None => true`) rejects every re-streamed task op as stale. Digest healing NEVER succeeds for assigned-task ops. | [BLOCKING] | yes | Anti-entropy heal path is broken for the exact case it exists to fix (assigned-task divergence) | Build `token_by_task: HashMap<Uuid, i64>` from `active_assignments` (mirroring `hive_sync.rs:231-241`), pass to `restream_row_to_ws_op`, re-stamp for `entity_type == "task"` |
| CF2 | `session.rs:2469-2475` — `resend_from_seq` tests two INDEPENDENT set differences (`node_ids - hive_ids` non-empty AND `node_ids - hive_deleted_ids` non-empty) instead of one intersection. Mixed tombstoned + in-sync → spurious `Some(1)` re-stream. | [SHOULD-FIX] | yes | Spurious re-streams (idempotent, no data corruption, but wasteful + semantically wrong) | Replace `&&` of two `.difference().is_some()` with single `.any(\|id\| !hive_ids.contains(id) && !hive_deleted_ids.contains(id))` |
| G-F1 | Duplicated test boilerplate | [INFO] | noted (debt) | Maintenance overhead | Extract to `tests/common/mod.rs` |
| G-F2 | `NodeTaskAttemptRepository` lacks txn support | [INFO] | noted (debt) | Forces raw SQL in test txn blocks | Generic executor bound |
| C-F2 | Missing trailing newlines in 4 files | [INFO] | noted (debt) | `cargo fmt --check` nit | Append `\n` |

---

## Peer review phase

| Finding | Discoverer | Peer reviewer | Verdict | Points |
|---------|-----------|---------------|---------|--------|
| CF1 | Codex | Claude | **CONFIRM** — "All four links in the CF1 causal chain are confirmed against actual code... The proposed remediation is correct and mirrors the normal send path exactly." | Codex: 3/3 |
| CF2 | Codex + Claude | Gemini | **CONFIRM** — "The analysis and counter-example are completely accurate... mathematically guarantee that the proposed `.any()`-based logic completely eliminates the false positive." | Codex: 3/3, Claude: 3/3 |

No steals (no DISPROVE verdicts; no alternative remediations needed).

---

## Scoreboard

| Challenger | Findings | Valid | Remediated | Peer-validated | **Total** |
|------------|----------|-------|------------|----------------|-----------|
| **Codex** | 2 | 2 | 2 | 2 | **6** (CF1 3pts + CF2 3pts) |
| **Claude** | 2 | 2 | 1 (CF2) | 1 (CF2) | **3** (CF2 3pts; F2 newline is INFO, no peer-review point) |
| **Gemini** | 2 | 2 | 0 peer-validated (both INFO, no peer review triggered) | 0 | **2** (2 valid INFO findings, 1pt each for valid+remediation, no 3rd point since no peer review) |

### Winner: **Codex** (6 points)

Codex uniquely discovered the [BLOCKING] CF1 fencing-token bug that both Claude and Gemini missed,
and independently co-discovered CF2 with an identical remediation.

---

## Remediation applied

Both confirmed findings were remediated in-session (per the No Deferred Remediation rule):

### CF1 fix (`crates/services/src/services/node_runner.rs`)
- Build `token_by_task: HashMap<Uuid, i64>` from `handle.state.read().active_assignments` before
  the re-stream loop (mirrors `hive_sync.rs:231-241`).
- `restream_row_to_ws_op` now takes `&HashMap<Uuid, i64>` and re-stamps `fencing_token` for
  `entity_type == "task"` ops (falls back to stored token for non-task ops or missing assignments).
- 3 unit tests added: re-stamp task op, preserve non-task stored token, fallback when no assignment.

### CF2 fix (`crates/remote/src/nodes/ws/session.rs`)
- Replaced two-independent-differences `&&` with single `.any(|id| !hive_ids.contains(id) && !hive_deleted_ids.contains(id))`.
- 1 integration test added: `digest_mixed_tombstoned_and_in_sync_no_spurious_restream` — seeds
  one tombstoned + one in-sync task, asserts `resend_from_seq == None`.

### INFO debt (accepted, not remediated)
- G-F1 (test boilerplate duplication): noted as standing debt.
- G-F2 (txn support in `NodeTaskAttemptRepository`): noted as standing debt.
- C-F2 (trailing newlines): noted; `cargo fmt` would fix but is cosmetic.

---

## Mandatory gate (post-remediation)

```
cargo clippy --all --all-targets --all-features -- -D warnings   → clean
cargo test -p db --lib                                            → 196 pass / 0 fail
cargo test -p remote --lib                                        → 94 pass / 0 fail
cargo test -p services --lib -- --skip terminal_session           → 205 pass / 0 fail
cd frontend && npm run lint                                       → clean
cd frontend && npx tsc --noEmit                                   → clean
```

---

## Lessons learned

1. **The fencing-token bug (CF1) is a cross-component interaction bug** — the kind per-task
   panels cannot catch (the enqueue path, the live-send path, the re-stream path, and the SC3
   guard are in 4 different files across 2 crates). The post-phase integrated reviews focused on
   each phase's diff in isolation and missed that the P5 heal branch copied a field that P4's
   enqueue path left as None. The tournament's cross-model discovery caught it.

2. **The `resend_from_seq` bug (CF2) survived because the existing test only exercised the
   all-deleted edge case**, not the mixed tombstoned + in-sync case. Two models independently
   spotted the same set-logic error — strong signal that the two-`&&` pattern reads as correct
   but is subtly wrong.

3. **Gemini's APPROVE with only INFO findings** shows a model-family blind spot: Gemini focused
   on structural/code-quality observations (DRY, txn support) rather than control-flow
   correctness. The cross-family discovery is what made the tournament effective.

4. **Codex's unique CF1 find** (the only [BLOCKING]) was the highest-value discovery of the
   round — a real defect that would have shipped a broken heal path. The tournament scoring
   (3pts for find + remediate + peer-validated) correctly rewards this.
