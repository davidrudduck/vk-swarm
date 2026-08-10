# Code Review — Round 3

**Target:** `wai/vk-swarm-task-breakdown` (PR #475)   **Range:** `b5209c33..c8c0c39f` (+ this round's fix)   **Effort:** high

Scope: the **round-2 remediation diff** — the `refetchInterval` predicate plumbing in
`useBreakdown.ts`, the `runningPollInterval` predicate and its wiring in `BreakdownReviewDialog.tsx`,
the corrected auto-trigger comment, and the six new vitest cases. This is the third and final round
under the loop's cap.

The ledger's `## Post-review known issues` (round-1 items A–F, round-2 items G–I) was passed in as
context, so adjudicated items were not re-derived (SC3b).

**Baseline at review time:** `cargo test --workspace` → exit 0, 58 suites, 1190 passed, 0 failed;
frontend vitest 535 passed.

## Findings

| # | File:line | Severity | Category | Finding | Confidence | Actionable? |
|---|-----------|----------|----------|---------|-----------|-------------|
| 1 | `frontend/src/hooks/useBreakdown.ts:72-82` | low | test-coverage | The round-2 fix was pinned only at the **predicate** level — six cases exercised `runningPollInterval` as a pure function. The **plumbing** was unpinned: whether a function-valued `refetchInterval` actually reaches react-query in the shape its v5 callback expects (the callback receives the `Query`, not the data, so the option is unwrapped via `query.state.data`). A shape mismatch there would silently disable polling — restoring the exact defect round 2 fixed — while every predicate test stayed green and `tsc` stayed clean at the call site. | high | yes — **fixed this round** |

## Non-actionable

None new this round.

## Verified sound (round-2 remediation diff)

- **The predicate's running condition matches the render condition it drives.**
  `runningPollInterval` tests `status === 'draft' && items.length === 0 && !!execution_process_id`,
  which is exactly `isRunning`'s condition at `BreakdownReviewDialog.tsx:213-218` minus the
  `isQuerySettled`/`isFailed` gating (correctly absent — those are render concerns, not
  poll-lifetime concerns). Polling therefore starts and stops in lockstep with the spinner it
  serves; there is no state where the spinner shows and polling is off, or vice versa.
- **Polling cannot run on an idle or closed dialog.** `enabled: modal.visible` gates the query
  entirely, and the predicate returns `false` for every non-running shape — `failed`, `accepted`,
  `discarded`, a draft that was never spawned (`execution_process_id === null`), a draft with items,
  and `null`/`undefined` data. Each is covered by a test case.
- **No polling amplification of the known N+1.** `TaskCard` was deliberately left un-polled;
  verified by grep that it still calls `useBreakdownProposal(task.id)` with no options. Had the
  poll been added at the card layer, an N-task board would have issued N requests every 3s against
  the fetch pattern already tracked as followups item 1.
- **The comment fix is accurate.** `core.rs:301` now reads "detached from the request, but
  supervised", which matches the inner-`tokio::spawn`-plus-`await` structure directly below it.
- **Both new test bodies are genuine, not fixtures agreeing with themselves.** Verified by
  mutation, twice: forcing `runningPollInterval` to return `false` turns
  `polls while a run is in flight` RED; forcing the hook to ignore the function form of
  `refetchInterval` turns the new plumbing test RED. Both mutations reverted and the working tree
  confirmed identical to the committed state afterwards.

## Remediation applied this round

**Finding 1** — two plumbing tests added to `useBreakdown.test.ts`, driving the real hook through a
real `QueryClient`: one asserts a function-valued `refetchInterval` produces a second `breakdownApi.get`
call, the other asserts a predicate returning `false` produces none. These exercise the option's
passage into react-query rather than the predicate's arithmetic, which is where the untested risk
actually sat.

## Coverage gaps (honest reporting)

Carried forward unchanged and still true — stated so graduation does not imply more than was done:

- **Round-1 findings 1/2/3/6 (the E2E script, its docs, and the compose banner) were validated
  read-only and have never been executed.** The docker/compose execution ban stands; a prior
  version of `e2e-test.sh` came within one run of destroying the live hive. Evidence is `bash -n`,
  a YAML parse, an exhaustive grep for compose invocations bypassing `dc()`, and Compose's
  documented `-p` > env > directory-name precedence. It is static evidence, and that is its limit.
- **Round-1 finding 7's supervision has no test.** The block sits inside the `create_task` axum
  handler, which needs a full `DeploymentImpl` with a real git repo; injecting a panic to assert a
  log line would test tokio, not this feature.
- **Independent adversarial review was absent for rounds 2 and 3.** The dispatched reviewer for the
  remediation diff never reported (the same failure mode as round 1's frontend finder). Its axes
  were covered directly by the orchestrator with cited evidence, so coverage holds, but a genuinely
  independent second opinion on the remediation diff is missing from this loop. Recorded rather
  than smoothed over.

## Gates (after remediation)

`cargo fmt --all -- --check`, `cargo clippy --all --all-targets --all-features -- -D warnings`,
`cargo test --workspace` (exit 0, 58 suites, 1190 passed, 0 failed), `npm run generate-types:check`,
`frontend` lint + tsc + `format:check` + vitest (**537** passed), `remote-frontend` lint + tsc +
vitest (426 passed) — all green.

## Verdict: Approve

Round 3's single finding was a test-coverage gap in round 2's own fix, closed here with tests
verified RED under mutation. No actionable finding survives, and no finding this round touched
production behaviour.

Actionable: []
