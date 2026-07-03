---
id: "203"
phase: 2
title: Hive TaskAssignmentRepository::try_claim (atomic CAS) + renew_lease
status: done
depends_on: ["201"]
parallel: false
conflicts_with: ["209"]
files:
  - crates/remote/src/db/task_assignments.rs
irreversible: false
scope_test: "crates/remote/src/db/task_assignments.rs"
allowed_change: edit
covers_criteria: [SC3]
covers_tests: []
---
## Failing test (write first)
**PRECONDITION (Trap 2b — NON-NEGOTIABLE):** these are `#[tokio::test]`s that REQUIRE a live, migrated
Postgres (the `201` columns + `node_fencing_token_seq`). A run without `DATABASE_URL` returns early (skip)
= HOLLOW pass. Stand up Postgres, `sqlx::migrate!("./migrations")`, export `DATABASE_URL=postgres://…`
before the gate, or RAISE that CI Postgres is unavailable.

**The test MUST be a `#[cfg(test)] mod` INSIDE `task_assignments.rs`.** The repo methods use **runtime**
`sqlx::query_as::<_, T>` / `sqlx::query` (NOT the `query!` macro — verified across all 9 existing methods),
so the typecheck does NOT need the offline cache; only the test needs a live PG. **Sibling read (rubric
#9):** `crates/remote/tests/backfill_e2e.rs` — copy its `database_url()`/`skip_without_db!`/`create_pool()`
helpers verbatim, and its `create_test_organization`/`create_test_node` fixture style. There is NO shared
`common` module — inline the helpers. Seed a `node_task_assignments`-adjacent fixture as the existing
`TaskAssignmentRepository::create` test path would (an org, a node, a swarm task id, a node_project id).

Add to `task_assignments.rs`:
```rust
#[cfg(test)]
mod lease_tests {
    use super::*;

    fn database_url() -> Option<String> { std::env::var("DATABASE_URL").ok() }
    macro_rules! skip_without_db { () => {
        if database_url().is_none() { eprintln!("Skipping: DATABASE_URL not set"); return; }
    }; }
    async fn create_pool() -> sqlx::PgPool {
        sqlx::PgPool::connect(&database_url().unwrap()).await.expect("connect")
    }
    // … inline create_test_organization / create_test_node and seed a (task_id, node_project_id) exactly
    //    as backfill_e2e seeds them; produce a fresh `task_id: Uuid` per test for isolation.

    #[tokio::test]
    async fn try_claim_wins_when_available_and_assigns_monotonic_token() {
        skip_without_db!();
        let pool = create_pool().await;
        let repo = TaskAssignmentRepository::new(&pool);
        // seed org, node_a, task_id, node_project_id.
        let claim = repo.try_claim(task_id, node_a, node_project_id, chrono::Duration::seconds(30))
            .await.unwrap();
        let claim = claim.expect("claim should succeed on an available task");
        assert_eq!(claim.node_id, node_a);
        assert!(claim.fencing_token > 0, "a granted token is from nextval (>0)");
        assert!(claim.lease_expires_at > chrono::Utc::now(), "lease expiry is in the future");
    }

    #[tokio::test]
    async fn try_claim_fails_for_a_second_node_while_lease_is_live() {
        skip_without_db!();
        let pool = create_pool().await;
        let repo = TaskAssignmentRepository::new(&pool);
        // node_a claims with a long TTL.
        let first = repo.try_claim(task_id, node_a, np_id, chrono::Duration::seconds(300))
            .await.unwrap().expect("a wins");
        // node_b tries the SAME task while a's lease is live → None (no row updated, no double-claim).
        let second = repo.try_claim(task_id, node_b, np_id, chrono::Duration::seconds(300))
            .await.unwrap();
        assert!(second.is_none(), "a live lease blocks a second claimant");
        let _ = first;
    }

    #[tokio::test]
    async fn try_claim_reclaims_an_expired_lease_with_a_strictly_higher_token() {
        skip_without_db!();
        let pool = create_pool().await;
        let repo = TaskAssignmentRepository::new(&pool);
        // node_a claims with a NEGATIVE TTL so lease_expires_at is already in the past.
        let a = repo.try_claim(task_id, node_a, np_id, chrono::Duration::seconds(-1))
            .await.unwrap().expect("a wins");
        // node_b now claims the EXPIRED lease → succeeds with a strictly higher token.
        let b = repo.try_claim(task_id, node_b, np_id, chrono::Duration::seconds(300))
            .await.unwrap().expect("b reclaims expired");
        assert_eq!(b.node_id, node_b);
        assert!(b.fencing_token > a.fencing_token,
            "a reassigned lease MUST get a strictly higher fencing token (the SC3 basis)");
    }

    #[tokio::test]
    async fn renew_lease_extends_expiry_without_changing_the_token() {
        skip_without_db!();
        let pool = create_pool().await;
        let repo = TaskAssignmentRepository::new(&pool);
        let a = repo.try_claim(task_id, node_a, np_id, chrono::Duration::seconds(30))
            .await.unwrap().expect("a wins");
        let renewed = repo.renew_lease(a.assignment_id, node_a, chrono::Duration::seconds(120))
            .await.unwrap().expect("renew succeeds for the lease holder");
        assert_eq!(renewed.fencing_token, a.fencing_token, "renew does NOT bump the token");
        assert!(renewed.lease_expires_at > a.lease_expires_at, "renew extends the expiry");
        // a foreign node cannot renew someone else's lease.
        let stolen = repo.renew_lease(a.assignment_id, node_b, chrono::Duration::seconds(120))
            .await.unwrap();
        assert!(stolen.is_none(), "renew is scoped to the current lease holder");
    }
}
```

## Change
- **File:** `crates/remote/src/db/task_assignments.rs`
- **Anchor:** add two methods inside `impl<'a> TaskAssignmentRepository<'a>` — place `try_claim` after
  `create` (@28-68) and `renew_lease` after it. Add a small purpose struct `LeaseClaim` near the top of
  the file (after the `TaskAssignmentError` enum @8-16). **Do NOT widen `NodeTaskAssignment` (domain.rs).**
- **Sibling read (rubric #9):** the existing `create` (@28-68, an `INSERT … RETURNING` mapped via
  `sqlx::query_as::<_, NodeTaskAssignment>`) and `fail_node_assignments` (@329-349, an `UPDATE … WHERE
  completed_at IS NULL RETURNING` read with `.get("task_id")`) are the patterns. `try_claim`/`renew_lease`
  reuse the runtime `sqlx::query`/`.get(...)` style — read both before authoring. The "available" predicate
  mirrors `find_active_for_task`'s `completed_at IS NULL` (@121) plus the lease-expiry leg.
- **Add the purpose struct** (narrow result — avoids the `FromRow` column-list trap; the existing 9
  SELECTs that omit the new columns keep working because `NodeTaskAssignment` is NOT changed):
```rust
/// Narrow result of an atomic checkout / renew — only the lease fields the wire grant needs.
#[derive(Debug, Clone)]
pub struct LeaseClaim {
    pub assignment_id: Uuid,
    pub node_id: Uuid,
    pub task_id: Uuid,
    pub fencing_token: i64,
    pub lease_expires_at: DateTime<Utc>,
}
```
  (`DateTime` import: the file already imports `chrono::Utc`; add `chrono::DateTime` to that use, OR write
  `chrono::DateTime<chrono::Utc>` inline — pick the minimal change and keep it confined to this file.)
- **Add `try_claim`** — the atomic conditional CAS (port of paperclip's `UPDATE … RETURNING`, ADR-0009):
```rust
    /// Atomically claim (or reclaim an expired lease on) a task for a node — the partition-safe
    /// checkout (ADR-0009 SC3). Succeeds (returns Some) only when the task has NO live assignment:
    /// either no active row, or the active row's lease has expired. Each grant bumps the monotonic
    /// fencing token via nextval, so a reassigned lease always outranks any prior holder. Two nodes
    /// can never both win (the UPDATE … RETURNING is atomic under the row lock). Returns None when a
    /// live lease blocks the claim.
    pub async fn try_claim(
        &self,
        task_id: Uuid,
        node_id: Uuid,
        node_project_id: Uuid,
        lease_ttl: chrono::Duration,
    ) -> Result<Option<LeaseClaim>, TaskAssignmentError> {
        // CONTRACT (executor finalizes the exact statement(s) against the live schema):
        //  ATOMIC ORDER (do NOT reorder — the UPDATE-then-INSERT split is race-safe only in this
        //  sequence, guarded by the `idx_task_assignments_active` partial unique index):
        //  - Step 1: attempt a conditional UPDATE on the EXISTING active row IFF it is reclaimable:
        //      WHERE task_id = $1 AND completed_at IS NULL
        //            AND (lease_expires_at IS NULL OR lease_expires_at < now())
        //      SET node_id = $2, node_project_id = $3,
        //          lease_expires_at = now() + ($interval from lease_ttl),
        //          fencing_token = nextval('node_fencing_token_seq')
        //      RETURNING id, node_id, task_id, fencing_token, lease_expires_at
        //    If a row is returned → claim granted (reclaim path). Two nodes can never both win the
        //    UPDATE: it is atomic under the row lock, and only an expired/NULL lease matches.
        //  - Step 2: if Step 1 updated 0 rows, check whether an active row exists AT ALL:
        //      SELECT 1 FROM node_task_assignments WHERE task_id = $1 AND completed_at IS NULL
        //  - Step 3: if an active row exists (with a LIVE lease, since Step 1 did not match it) →
        //    return Ok(None) — a live lease blocks the claim; do NOT fall through to INSERT.
        //  - Step 4: ONLY when no active row exists at all → INSERT a fresh assignment with
        //    lease_expires_at = now()+ttl and fencing_token = nextval(...). The `idx_task_assignments_active`
        //    partial unique index guarantees atomicity here too: a concurrent INSERT from another node
        //    that also missed the UPDATE raises a unique-violation → treat that as Ok(None) (the other
        //    node won the insert race); do NOT retry the INSERT.
        // Map the RETURNING into LeaseClaim with .get(...) like fail_node_assignments. fetch_optional.
        todo!("executor finalizes the CAS statement(s) per the contract above")
    }
```
  > The body is intentionally specified as a contract, not literal SQL, because the available-vs-expired
  > split + INSERT-or-UPDATE must be finalized against the live `node_task_assignments` constraints (the
  > `idx_task_assignments_active` partial unique index that `create` already trips on @60 — a reclaim must
  > UPDATE the existing active row, not insert a duplicate). The four tests pin the observable contract.
- **Add `renew_lease`** — extend expiry for the CURRENT holder, token UNCHANGED:
```rust
    /// Renew (extend) a live lease for its current holder. Does NOT change the fencing token (renewal is
    /// not a reassignment). Returns None if the assignment is gone, completed, or held by a different
    /// node (a foreign node cannot renew someone else's lease).
    pub async fn renew_lease(
        &self,
        assignment_id: Uuid,
        node_id: Uuid,
        lease_ttl: chrono::Duration,
    ) -> Result<Option<LeaseClaim>, TaskAssignmentError> {
        // UPDATE node_task_assignments SET lease_expires_at = now() + ($ interval)
        //   WHERE id = $1 AND node_id = $2 AND completed_at IS NULL
        //   RETURNING id, node_id, task_id, fencing_token, lease_expires_at
        // fetch_optional → None when the holder/row check fails. Map into LeaseClaim.
        todo!("executor finalizes the renew UPDATE per the contract above")
    }
```
> **NOTE (implemented):** the shipped `renew_lease` (`crates/remote/src/db/task_assignments.rs:187-202`)
> adds `AND lease_expires_at > NOW()` to the WHERE predicate alongside `id`, `node_id`, and
> `completed_at IS NULL`. This closes the post-expiry extension gap: a lease that has already expired
> cannot be renewed (renewal is not a reclaim — an expired lease must be reclaimed via `try_claim`,
> which bumps the fencing token). The predicate is the load-bearing difference between "renew" (token
> unchanged, lease still live) and "reclaim" (token bumped, lease was expired). The four tests above
> still hold: the foreign-node renew test now also covers the expired-lease case (an expired lease
> returns `None` exactly like a foreign-holder case).

## Allowed moves
ONLY: add the `LeaseClaim` struct, the `try_claim` method, and the `renew_lease` method to
`task_assignments.rs`, plus the `#[cfg(test)] mod lease_tests`, plus (if needed) extend the existing
`use chrono::Utc;` to also import `DateTime`. Do NOT widen `NodeTaskAssignment` (domain.rs) or any of the
9 existing SELECT/RETURNING column lists. Do NOT run `cargo sqlx prepare` (these are runtime queries; no
offline cache entry is generated). Do NOT touch the migration (201), the WS protocol (202), or the sweep
(209 — it also touches this file; conflicts_with 209).

## STOP triggers
- A reclaim path would INSERT a SECOND active row for the same task → BUG: the `idx_task_assignments_active`
  partial unique index (the one `create` maps to `AlreadyAssigned` @60) forbids it. Reclaim MUST UPDATE the
  existing active row. Verify the index definition in `20251202000000_nodes_swarm.sql` before writing the CAS.
- `try_claim` returns `Some` for a task whose lease is still live (e.g. forgot the `lease_expires_at < now()`
  leg) → BUG: that is the double-claim SC3 forbids. The second test catches it.
- The reassigned token is not strictly higher (e.g. reused `fencing_token` instead of `nextval`) → BUG: the
  third test catches it; this is the partition-safety basis.
- `query!`/`query_as!` macro forms are tempting → DO NOT; match the file's runtime `sqlx::query`/`query_as::
  <_, T>` style so no offline-cache entry is needed (Trap 2b: macro forms WOULD need the cache or a live PG
  at compile time).
- Touching `domain.rs` to add columns to `NodeTaskAssignment` → STOP: that triggers the `FromRow`
  missing-column RUNTIME failure across the 9 omitting SELECTs (decisions-ledger judgment call: 203 uses
  narrow queries, not a struct widening). If a later phase genuinely needs the columns on the struct, that
  is its own task editing all 9 lists.

## Done when
`WAI_TYPECHECK_CMD="cargo check -p remote" WAI_TEST_CMD='test -n "$DATABASE_URL" && cargo test -p remote lease_tests' bash ~/.claude/wai/scripts/task-gate.sh vk-swarm-hive-redesign 203` exits 0
(run with `DATABASE_URL=postgres://…` pointed at a migrated Postgres — Trap 2b. The `test -n "$DATABASE_URL"
&&` prefix makes the gate FAIL-CLOSED: without `DATABASE_URL` the gate fails instead of `skip_without_db!`
reporting a hollow green.)
