# Breakdown tournament — round 1 (2026-08-05)

method: external-find + orchestrator-judge fallback

## Dispatch

| competitor | runner | outcome |
|---|---|---|
| codex | dr/0.9.1 codex-cli-panel, read-only | OK — 3 findings (find-codex.md) |
| agy (Gemini seat) | dr/0.9.1 agy-cli-panel, read-only | ran full repo walk but emitted NO findings table (find-agy.md is narration only) — empty submission |
| opencode glm-5.2 | dr/0.9.1 opencode-cli-panel | FAILED at startup: user-level `~/.config/opencode/opencode.json` invalid (unrecognized keys `plugins`, `config`) — pre-existing env issue, not fixed from this workstream |

Peer-judge round: impossible externally — agy hit provider quota ("Individual quota reached",
judge-agy-of-codex.md), opencode config broken, codex cannot judge itself. Fallback per
REFERENCE.md: orchestrator independently verified each codex finding against the repo.

## Verdicts on codex findings

| # | severity | issue_real | evidence | applied |
|---|---|---|---|---|
| 1 | high | NO | Claims the TS3 200-mock body fails serde because Option fields are omitted. serde derive deserializes a missing `Option<T>` field as `None` by default; every non-Option `SharedTask` field (id, organization_id, title, status, version, created_at, updated_at — crates/remote/src/db/tasks.rs:55-101) is present in the mock, and `SharedTask` has no `deny_unknown_fields`. Empirically re-checked by the test run (TS3 green). | no |
| 2 | high | YES | SC3 was covered-but-hollow: no test asserted the tracing warn. Real fidelity gap. | YES — task 002 amended via envelope resubmit: `tracing-test = { version = "0.2", features = ["no-env-filter"] }` dev-dep, `#[tracing_test::traced_test]` on the dangling test, `logs_contain("dangling shared_task_id")` + `logs_contain(shared_id)` assertions |
| 3 | medium | NO | Wants task frontmatter `irreversible: true`. The task-gate irreversible flag is for repo-irreversible operations ("deletes core/dep removal/contract change", schema/task.frontmatter.md); this task deletes no code and changes no contract. The runtime data-deletion decision is the spec-flagged IRREVERSIBLE decision, already covered by ADR-0015 as precheck requires. | no |

## Scoreboard

| competitor | validated issues | validated fixes | score |
|---|---|---|---|
| codex | 1 | 1 | 2 |
| agy | 0 | 0 | 0 |
| opencode | dnf | dnf | 0 |

## Termination

Sole validated finding (#2) remediated via `wai-submit-plan.sh` resubmit (no hand-edits of the
promoted tree); submitter replayed plan-lint green on promotion. Focused re-check: rendered task
002 carries the Cargo.toml file entry, the traced_test attribute, and both logs_contain
assertions. Round closed; proceeding to execution.
