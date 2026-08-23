---
workstream: browser-auth-terminal-row-retention
doc_type: readme
status: ready
title: "Define retention for terminal browser-auth rows"
depends_on:
  - local-node-browser-oauth
adrs: []
staging_pointers:
  - docs/plans/local-node-browser-oauth/decisions-ledger.md
---

# browser-auth-terminal-row-retention

**Origin:** discovered during the integrated `local-node-browser-oauth` phase-1 adversarial
review on 2026-08-22.

## Finding

Phase 1 deliberately preserves claimed or invalidated OAuth handoffs and revoked browser sessions
as durable terminal rows. The task contracts prohibit deleting those rows because their state must
remain observable and revocation must survive restart. Without a later retention policy, however,
both tables can grow with login and browser-session churn.

## Required outcome

Settle a retention policy before implementing cleanup:

- define separate minimum retention windows for terminal handoffs and revoked sessions;
- preserve pending handoffs and live sessions unconditionally;
- choose an operator-visible trigger (scheduled compaction, existing maintenance, or explicit
  administration) rather than hiding deletion in authentication requests;
- prove retained revocations survive restart and cleanup never removes live authorization state;
- document operational impact and database observability.

No cleanup is included in phase 1 because choosing the retention window is a product/storage
decision and deletion was an explicit STOP trigger in tasks 004, 005 and 022.
