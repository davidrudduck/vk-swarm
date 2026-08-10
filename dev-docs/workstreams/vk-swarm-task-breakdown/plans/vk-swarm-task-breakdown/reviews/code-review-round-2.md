# Code Review — Round 2

**Target:** `wai/vk-swarm-task-breakdown` (PR #475)   **Range:** `da97cb29..b5209c33`   **Effort:** high

Two scopes this round:

1. **The round-1 remediation diff** (`b5209c33`) — new code, new surface. Reviewed adversarially
   for defects *introduced or left open* by the fixes, not re-reviewing the wider PR.
2. **The two axes round 1 recorded as uncovered**: dialog query-state gating across the
   settled/loading/error/refetch matrix, and the double-click window on mutation-disabled buttons.

The ledger's `## Post-review known issues` (now including round-1 items A–F) was passed in as
context, so adjudicated items were not re-derived (SC3b).

**Baseline at review time:** `cargo test --workspace` → exit 0, 58 suites, 1190 passed, 0 failed.

## Findings

| # | File:line | Severity | Category | Finding | Confidence | Actionable? |
|---|-----------|----------|----------|---------|-----------|-------------|
| 1 | `frontend/src/components/dialogs/tasks/BreakdownReviewDialog.tsx:88`, `frontend/src/hooks/useBreakdown.ts:62` | medium | correctness | **The running state never resolves.** `isRunning` renders *Generating breakdown...* for a draft with an execution process and no items — the normal state on opening the dialog during a run. A run completes server-side with **nothing pushed to the client**: the only `invalidateQueries({queryKey:['breakdown',taskId]})` in the codebase is in the dialog's own mutations (`useBreakdown.ts:88`), and neither consumer passed `refetchInterval`. With `main.tsx:13-14` setting `staleTime: 5min` and `refetchOnWindowFocus: false`, the spinner sits there indefinitely — and **closing and reopening the dialog does not help**, because the query stays fresh in cache for five minutes. Only a full reload or waiting out `staleTime` recovers. The same stale cache backs the `TaskCard` badge. This is the feature's core happy path. | high | yes — **fixed this round** |
| 2 | `crates/server/src/routes/tasks/handlers/core.rs:301` | low | quality | The comment introduced above the auto-trigger block still described the stage-2 spawn as "fire-and-forget" after `b5209c33` made it supervised — the exact class of intent-vs-behaviour drift round 1 filed five findings against, reintroduced by the fix for one of them. | high | yes — **fixed this round** |

Both were fixed before this record was finalised; see "Remediation" below.

## Non-actionable

| # | File:line | Severity | Category | Finding | Confidence | Why non-actionable |
|---|-----------|----------|----------|---------|-----------|---------------------|
| G | `BreakdownReviewDialog.tsx:281` | low | quality | Query settled with `proposal === null` falls through to the item-list branch and renders an **empty dialog** — no items, no message, and a Discard button that is enabled but whose handler returns early on `!proposal`, so clicking it does nothing silently. | high | Effectively unreachable: the dialog opens only from the badge, which `TaskCard.tsx:292` renders only when a draft proposal exists. Reachable only by a race against another client's discard. A "no proposal" empty state is worth adding when the N+1 fetch is reworked (tracked item 1 in `task-breakdown-followups`), not as a standalone edit. |
| H | `BreakdownReviewDialog.tsx:243` | low | quality | The **Reload** button gives no in-flight feedback. After an error the query's status stays `error` (not `pending`), so `isLoading` remains false during the refetch and the error banner does not change; a user may click repeatedly and fire several refetches. | high | Harmless — react-query dedupes in-flight fetches for the same key, so the extra clicks cost nothing. Cosmetic. |
| I | `crates/db/src/models/task_breakdown/queries.rs:298` | low | quality | The CAS `RowNotFound → Protocol` remap means a proposal row **deleted** between the read and the write (cascade delete of the parent task) now surfaces as 409 rather than 404. | high | Semantically defensible — a row vanishing mid-update is a concurrency conflict, which is what 409 means. The nonexistent-proposal case is unaffected: `update_status` still returns `RowNotFound` → 404 from its initial `find_by_id`, which the remap does not touch. |

## Verified sound (remediation diff)

Each checked against the actual merged code, not the commit message.

- **CAS does not break the no-op arm or 404 semantics.** `is_legal_transition`'s `(a, b) if a == b`
  arm still passes, and CAS matches because `current == status`. The initial
  `find_by_id(...).ok_or(sqlx::Error::RowNotFound)` is untouched, so a genuinely absent proposal
  still yields 404 via `map_proposal_error:228`; the remap applies only to the UPDATE's zero-row
  case. Verified by reading `map_proposal_error` (`routes/breakdown.rs:226-232`) against every
  `update_status` call site.
- **No caller holds a transaction.** All six `task_breakdown::update_status` call sites pass
  `&pool` (`routes/breakdown.rs:97,284,520,531,660`, `services/breakdown.rs:317`), so the
  single-statement CAS needs no signature change and cannot nest.
- **Dedupe ordering is correct in both layers.** Range and self-reference checks precede dedupe,
  which precedes the cycle check — `breakdown.rs:223-236` and `queries.rs:136-160`. Dedupe cannot
  mask an out-of-range or self-referential index because those are rejected first.
- **`persist_result` sees the deduped values.** `parse_breakdown_result` mutates and returns the
  result, and `persist_result` maps from `subtask.depends_on` on that same value; `sort_order` is
  the enumeration index and is unaffected by dedupe of a *different* field.
- **Panic-path status write cannot clobber.** If the proposal is no longer `Draft` when the
  supervisor marks it `Failed` (e.g. the user discarded meanwhile), CAS rejects the write and it is
  logged at warn — the new supervision cannot overwrite a user decision. The two fixes compose
  correctly rather than fighting.
- **Every compose invocation carries `-p`.** The only `docker compose` occurrences in
  `e2e-test.sh` are `dc()` itself (`:66`) and an informational `log` string (`:100`) — no raw
  invocation survives. The `ss` guard (`:163-171`) precedes `dc up` and sets `SKIP_DOCKER=true`
  before `exit 1`, which `cleanup()` (`:90`) honours by returning early, so the trap tears nothing
  down. `COMPOSE_PROJECT_NAME` is assigned at `:40`, before `dc()` is defined or called.
- **Locale files changed by addition only.** Parsed both revisions of all four files and diffed
  key-by-key: `breakdown.loadFailed`, `breakdown.loading`, `breakdown.reload` added to each; **zero**
  keys removed, **zero** values changed. Confirms the en `sharedTask` duplicate collapse is
  invisible to any parser and behaviourally inert.

## The two axes carried from round 1

**Dialog query-state gating — covered, one finding.** Walked the full matrix. Loading, error, and
failed states each gate correctly, and `isQuerySettled` properly hides the item list and disables
Accept while the query is unsettled, which is the "pending fetch is indistinguishable from no
proposal" hazard the code comment names. The gaps are finding 1 (running state never resolves) and
non-actionable G (settled-with-null renders blank).

**Double-click window — refuted as a defect.** Checked at both layers. Client: Discard, Retry and
Accept are each disabled on their own `isPending`, which react-query sets in the same render pass
as `mutate`, so there is no window a human can hit. Server, independently: a second Discard is
absorbed by the `a == b` transition arm plus CAS; a second Retry hits the one-draft-per-task
partial unique index and 409s (`retry_impl` creates a new row rather than re-drafting); a second
Accept is rejected by `accept_proposal`'s re-read on the transaction handle (`queries.rs:318`).
Every path is defended server-side regardless of client state, which is the property that matters.

## Coverage gaps (honest reporting)

- **Findings 1/2/3/6 from round 1 were validated read-only and remain so.** The docker/compose
  execution ban stands (a prior version of `e2e-test.sh` came within one run of destroying the live
  hive). Verification was `bash -n`, a YAML parse, grepping for un-`dc()`'d compose calls, and
  Compose's documented `-p` > env > dirname precedence. **Nobody has executed the fixed script.**
  That is the honest limit of this round's evidence for those four.
- **Finding 7's supervision (round 1) has no test.** The block sits inside the `create_task` axum
  handler, which needs a full `DeploymentImpl` with a real git repo; injecting a panic to assert a
  log line would test tokio, not this feature. Stated rather than papered over.
- **A dispatched adversarial reviewer for the remediation diff did not report** (same failure mode
  as round 1's frontend finder). Its five axes were therefore covered directly by the orchestrator
  and are itemised under "Verified sound" above with their evidence, so the axes are covered even
  though the independent second opinion is missing. This is a real reduction in independence and
  is recorded as such.

## Remediation applied this round

- **Finding 1** — `useBreakdownProposal`'s `refetchInterval` option widened to accept a predicate
  over the current data; the dialog passes `runningPollInterval`, which polls at 3s **only** while
  the proposal is a draft with an execution process and zero items, and returns `false` otherwise,
  so an idle or settled dialog does not poll. `TaskCard` deliberately unchanged — polling per card
  would multiply the N+1 fetch already tracked as followups item 1.
  Pinned by six new vitest cases (`runningPollInterval` describe block). Anti-hollow verified:
  forcing the predicate to return `false` turns `polls while a run is in flight` RED; mutation
  reverted and the file confirmed additive-only.
- **Finding 2** — comment corrected to "detached from the request, but supervised".

## Gates (after remediation)

`cargo fmt --all -- --check`, `cargo clippy --all --all-targets --all-features -- -D warnings`,
`cargo test --workspace` (exit 0, 1190 passed), `frontend` lint + tsc + `format:check` + vitest
(**535** passed, was 528), `remote-frontend` lint + tsc + vitest (426 passed) — all green.

## Verdict: Request changes

Both actionable findings have already been fixed (above), but the round is recorded with them
**open**, not as `[]`. Recording `Actionable: []` on a round whose own Findings table lists two
actionable items would be self-contradictory, and would let the fixes graduate without any review
pass having examined them — which is precisely the gap this loop exists to close. Round 3 reviews
the round-2 remediation diff and is the round that may converge.

Actionable: [1,2]
