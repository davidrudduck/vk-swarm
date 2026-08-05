# Decisions Ledger

## Submission
Plan accepted from submit envelope.

## Pre-execution (decompose)

### 2026-08-05 — breakdown review round 1 (hybrid external + sub-agent-fallback)
See `reviews/round1-verdict.md`. Two CONFIRMED blockers forced a deliberate spec amendment
(Workbox generateSW toString serialization → predicate must be inline; spec re-frozen via
/wai:precheck after amendment) and a forbid_after literal swap. All other validated findings
applied via plan envelope v2 resubmission.

### Done-when placeholder fill
The plan submitter renders `Done when` lines with `<dir>/<typecheck>/<test>` scaffold
placeholders (no envelope field exists for them). Filled post-render with dynamic WAI_ROOT
resolution + concrete scoped commands per task (tournament codex-F1/mechanical-F2). This
completes the scaffold; it does not alter any gate-checked artifact.

### Precheck anchor-check false positives (recorded per no-deferred-remediation)
`--no-anchor-check` used on both precheck runs: the extractor truncates
`crates/remote/src/auth/handoff.rs` / `crates/server/src/routes/oauth.rs` to `src/...`
(both verified present on main via `git cat-file -e`), and
`remote-frontend/src/lib/swCachePredicate.ts` is a to-create file (verified absent on main).
