---
id: "502"
phase: 5
title: "Pin the hive-absent 503 exactly; the four route tests only asserted != 500"
status: passed
depends_on: ["501"]
parallel: false
conflicts_with: []
files:
  - crates/server/tests/nodes_routes.rs
  - crates/server/tests/swarm_projects_routes.rs
  - crates/server/tests/swarm_labels_routes.rs
  - crates/server/tests/swarm_templates_routes.rs
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC4]
---

## Why this task exists (created mid-run at 501 close, not at decomposition)

Found by the orchestrator while independently working attack vector 4 of the Stage-2 panel
("does any assertion still use a vacuous predicate that would pass against a broken server?").

The four hive-absent tests carried this:

```rust
assert_ne!(
    res.status, 500,
    "hive-absent must be the specific HiveNotConfigured 503 (task 401), never an unhandled 500"
);
```

The **message claims** the assertion pins `503`. The **assertion** only excludes `500`. It passes
for `200`, `400`, `401`, `404` — any status but 500. This is the fourth instance in this run of the
same defect class (`assert_ne!(status, 404)`, `status >= 500`, the over-broad content-type guard),
and like the others it is invisible to every gate: Stage 1 is mechanical, and the suite is green
either way.

**Why this is BLOCKING rather than a nice-to-have.** Task 501's ledger states that SC4's
`503 HiveNotConfigured` path is *"not live-observable on this host by construction, and remains
covered by the in-process registration tests."* That claim was **false as written** — the in-process
tests did not pin 503. The evidence doc would have shipped asserting coverage that did not exist.
Fixed in-session per CLAUDE.md "No Deferred Remediation".

## Change

In each of the four files, replace the `assert_ne!(res.status, 500, ...)` block with:

```rust
assert_eq!(
    res.status, 503,
    "hive-absent must be the specific HiveNotConfigured 503 (task 401), never an unhandled 500 \
     and never a silently-different status; body: {}",
    res.body
);
```

No product code changes. The product was already correct — only the test was hollow.

## Manual verification (recorded verbatim in the ledger)

Green after the fix, mutation-killed under a deliberate regression, and reverted clean.

## Done when

- All four files assert `assert_eq!(res.status, 503)`.
- A mutation of `ApiError::HiveNotConfigured`'s status kills **all four** tests, not just the first.
- `cargo fmt` / `clippy --all --all-targets --all-features -D warnings` / `cargo test --workspace`
  are all exit 0.
