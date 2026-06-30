# ADR-0009 — Assignment via atomic checkout + lease, made partition-safe by fencing tokens

- **Status:** accepted
- **Date:** 2026-06-30
- **Workstream:** vk-swarm-hive-redesign
- **Discharges:** node-foundations **D7** / [ADR-0001](./0001-crash-recovery-fence-then-resume.md)
  (`mark_orphaned_as_failed` matching foreign nodes' rows — punted to this workstream)

## Context

SC3 requires that a **network partition cannot cause double execution** of the same task. The paperclip
reference gives the proven *shape* — an atomic conditional checkout
(`UPDATE issues SET checkoutRunId=… WHERE status IN (…) RETURNING`, `paperclip/services/issues.ts:5533-5728`)
and an escalate-don't-kill silence watchdog — but paperclip runs each agent as a **server-owned local
subprocess**, so it has **no remote-worker network hop**: its "lease" is derived from a locally-observed
process, not a timed lease, and it has no defence against a *partitioned-but-still-alive* worker.

vk-swarm's hive↔node boundary **is** that network hop. **Leases + heartbeats alone are not enough:** if
the hive leases task T to node A, A partitions but keeps running, the lease expires, and the hive
reclaims T to node B — then A and B both execute T. Liveness detection, reclaim, and idempotent claim do
not stop the partitioned-but-alive writer.

node-foundations [ADR-0001](./0001-crash-recovery-fence-then-resume.md) already established a
process-group fence (PID + start-time fingerprint, defeating PID reuse) and explicitly punted
cross-node ownership (D7) to this workstream.

## Decision

1. **Atomic checkout (idempotent claim).** The hive assigns via a conditional CAS —
   `UPDATE node_task_assignments SET node_id=?, lease_expires_at=?, fencing_token=<next> WHERE <task
   available> RETURNING` — so two nodes can never both win a claim (port of paperclip's
   `UPDATE … RETURNING`).
2. **Real lease with expiry + heartbeat renewal.** Each grant has a `lease_expires_at`; the node
   renews via a periodic heartbeat carrying its node id and in-flight assignment ids. A missed
   heartbeat lets the lease expire; the hive may then reclaim/reassign with a **new, strictly higher
   fencing token**.
3. **Fencing tokens (the partition-safety mechanism), scoped to hive-assigned tasks.** Every lease
   grant carries a monotonic fencing token. The node stamps every report-up op
   ([ADR-0008](./0008-node-hive-ordered-ack-outbox.md)) **against a hive-assigned task** with that
   task's current token; the hive **rejects any commit whose token is older than the assignment's
   current token**. So a partitioned node A, after its lease is reassigned to B (higher token), has its
   late writes **bounced** → **at-most-once commit effect**. **Node-owned work** (locally-created tasks
   and their attempts/execs/logs) has no assignment and no lease — those ops carry **no fencing token**
   and are committed under the node's **ownership identity** (the node is the sole authority for its own
   tasks), so the stale-token check simply does not apply to them.
4. **Node self-fencing (bounds the overlap).** A node that cannot renew within the lease TTL **halts
   the agent** — the process-group kill from [ADR-0001](./0001-crash-recovery-fence-then-resume.md) —
   so execution overlap is **bounded**, not unbounded.

**Guarantee stated precisely:** at-most-once commit *effect* (fencing token rejection) + bounded-overlap
execution (node self-fencing). This is what "a partition cannot cause double execution" means in a
network system — not "we have leases".

## Consequences

- Satisfies SC3 and **discharges node-foundations D7**: foreign-row disambiguation on the hive is now
  lease/ownership-based, not the single-node `!= current_instance` heuristic.
- Adds a heartbeat/renewal path and a `fencing_token` column on assignments and on every op
  (wire-format change, irreversible).
- Self-fencing reuses an existing mechanism (ADR-0001), so the only new node behaviour is the
  renew-deadline watchdog.

## Alternatives considered

- **Lease + heartbeat without fencing** — rejected: does not stop the partitioned-but-alive writer
  (the exact SC3 failure).
- **Paperclip's terminal-status-derived lock** (no timed lease) — rejected: depends on the server
  directly observing the agent process, which the hive cannot do across the network.
- **Distributed lock service (etcd/Consul)** — rejected: heavy new dependency; a Postgres CAS + fencing
  token meets the guarantee within the existing store.
