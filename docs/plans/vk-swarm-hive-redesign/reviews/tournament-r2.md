# Adversarial breakdown tournament — Phases 2–6, Round 2

**Date:** 2026-06-30 · **Target:** `phase-2..6` (lease/fencing, status, inbound, anti-entropy, no-fanout)

## Method

3 real external CLI competitors via the dr panel runners, find-prompt `reviews/r2-find-prompt.md` (with an
anchor-verification caveat: P2–P6 build on not-yet-built P1 code, so verify against `main` OR the cited
upstream task's Before/After). **Codex** and **Gemini** produced structured findings; **opencode/GLM**
DNF (investigated but emitted no final table). Peer-validation = cross-competitor corroboration +
orchestrator repo-grounded verification (every finding reproduced against the real code before editing);
≥2 structured competitors met the floor.

## Findings, verdicts, remediations

| # | sev | task | finding | verdict | remediation |
|---|-----|------|---------|---------|-------------|
| F1 | high | 202,501 | The P2 lease-protocol and P5 digest-protocol tasks both edit the dual WS enums + `session.rs` but neither references the other → concurrent execution corrupts the enum tails | **CONFIRMED** (codex+gemini) | symmetric `conflicts_with` 202↔501 + `501 depends_on 202`; plan rows synced |
| F2 | **critical** | 205 | Fence resolves the task via `find_by_source_task_id(node_id, payload.id)` = the CREATOR node (`tasks.rs:352`); a task reassigned to a non-creator node returns `None` → fence skipped → **double-execution** (breaks SC3) | **CONFIRMED** — verified `Task.shared_task_id` exists (mod.rs:44) and the lookup keys on `source_node_id` | resolve via `payload.shared_task_id` directly; fall back to `find_by_source_task_id` only when null; test a reassigned-node stale-token reject |
| F3 | high | 303 | 303 enforces transitions in `handle_op_batch` (the 205-owned block) but `depends_on` only [301,302] — runs before the fencing seam exists | **CONFIRMED** | `303 depends_on += 205` |
| F4 | high | 503 | `resend_from_seq = MAX(seq)` never resends an acked-but-lost OLDER op (hive through seq 10, lost entity at seq 3) → SC5 self-heal fails | **CONFIRMED** | replay from the MIN seq among missing entities (or conservatively 1); TS4 seeds a below-max missing entity |
| F5 | high | 601,602 | Use the BIN name `vks_hive_server`/`vks-hive-server`; the package/lib crate is `remote` → `use`/`-p` fail to compile | **CONFIRMED** — Cargo.toml `[package] name="remote"`, `[[bin]] name="vks-hive-server"` | `use remote::…` + `cargo {check,test} -p remote` |
| F6 | med | 601 | 601 builds a THIRD exhaustive `HiveMessage` match but `depends_on [103]` only; 202/501 STOP-trigger on a third match → wedge if 601 runs first | **CONFIRMED** (codex+gemini) | `601 depends_on [103,202,501]`; classification arms made unconditional |
| F7 | med | 208 | Self-fence test requires the selector to pick `lease_expires_at=None`, but the Change calls that ambiguous (two impls, one makes the test wrong) | **CONFIRMED** | selector fences only `Some(exp)<now`; `LeaseRevoked` is a separate immediate-halt path + its own test |
| F8 | high | 210 | SC3 acceptance test asserts at the repo layer (`upsert_from_node`, no fencing) → bypasses the rejection → hollow | **CONFIRMED** | test `handle_op_batch` directly inside `session.rs` (or a `pub(crate)` apply helper); fail-closed PG gate kept |

## Scoreboard

| competitor | structured | validated | score | note |
|---|---|---|---|---|
| Codex | 6 | 6 | 12 | breadth + F4 replay bug + F5 crate name + the cross-phase ordering trio |
| Gemini | 4 | 4 | 8 | the CRITICAL F2 fence-bypass + F8 hollow-test; corroborated F1/F6 |
| opencode/GLM | 0 (DNF) | – | 0 | no final table |

**Winner: Gemini** on impact (the critical SC3-breaking fence bypass), Codex on breadth.

## Termination

8 peer-validated findings remediated in the task files; none collided with the frozen spec (F2/F4 are
correctness, the rest are task-metadata/test-placement). Focused re-check: `wai-plan-lint` PASS for
phases 2–6 (plan↔frontmatter consistent after the dep/conflict edits). Round closed — no second full round.
