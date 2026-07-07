# Decisions Ledger — hive-node-api-key-ui

Append-only. Implementers record any choice the task did not dictate. Empty = perfect.

## Pre-execution (decompose)

### Decomposition choices (made by the decomposer, not the implementer)

- **8 tasks across 3 phases** rather than fewer-bigger tasks, because the
  single component file has 4 distinct behaviors (list, create, revoke/unblock,
  error) and bundling them into one edit task would make the diff unreadable
  in one screen. Each behavior is its own red-green pair.
- **Task 005 (barrel) depends only on 001** (the component skeleton), not on
  002-004. The barrel export needs only the component symbol; the section's
  behaviors are irrelevant to the import path.
- **Task 006 (Nodes.tsx composition) depends on 002 + 005**, not on 003 + 004.
  The TS8 test only asserts "renders section when orgId set" — it works with
  the create flow in place but does not require revoke/unblock or the error
  Alert. The lint's dep graph is the minimum that satisfies ordering; an
  implementer who finds 003/004 needed at compose time should escalate.
- **Task 007 (i18n) depends on 004**, so all `t()` calls in the component are
  stable before the locale JSON is amended. The TS9 test in 007 walks every
  `settings.swarm.apiKeys.*` key the component uses and asserts the en locale
  has a matching entry.
- **Task 008 is manual verification** with no source changes. The two
  commands are recorded in the decisions-ledger when the implementer runs
  them; the executor does not gate on the test pass in this task.
- **Same-directory sibling list in task 001 and 007 is exhaustive** of the
  `remote-frontend/src/components/swarm/` directory, even for files that are
  not pattern siblings (the dialogs). This is a one-line cost to silence the
  lint's `W:` advisory; the SC4 cross-directory sibling (the reference impl)
  is also listed.

### Plan-lint advisory W: warnings (acknowledged, not blocking)

After expanding the `files:` lists, the lint reports zero W: lines. The
historical W: lines (Merge*Dialog.tsx, NodeCard.tsx, index.ts) were the lint
picking the first unlisted same-extension sibling in the directory; listing
every file in the directory silences them.

### Sibling-read decisions (recorded so the implementer can verify)

The implementer of task 001 MUST read every file listed in `files:` as a
sibling and record the structural choices in the "## Execution" section of
this ledger. The cross-directory reference impl is
`frontend/src/components/org/NodeApiKeySection.tsx`; the in-tree pattern
siblings are the six `*Section.tsx` files in `remote-frontend/src/components/swarm/`.
The dialogs (Merge*, SwarmLabel*, SwarmProject*, SwarmTemplate*) are NOT
pattern siblings — they are modals that operate on a selected entity, not
list/section patterns. The implementer should record this non-pattern
classification in their ledger entry.

### Lint regex patch (none required this round)

The pre-existing lint regex split for the `\bTODO\b` deferral marker (see
`docs/plans/vk-swarm-design-system/decisions-ledger.md`) is sufficient; no
further patches are needed.

## Execution (tasks 001-008)

The implementer appends one section per task here, recording any choice the
task did not dictate, any divergence from the sibling patterns, and the
decisions needed to satisfy the failing-test-first red step.

### Task 001
*(appended by the implementer)*

### Task 002
*(appended by the implementer)*

### Task 003
*(appended by the implementer)*

### Task 004
*(appended by the implementer)*

### Task 005
*(appended by the implementer)*

### Task 006
*(appended by the implementer)*

### Task 007
*(appended by the implementer)*

### Task 008
*(appended by the implementer — full stdout+stderr of the two verification commands)*
