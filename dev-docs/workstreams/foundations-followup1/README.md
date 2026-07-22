---
workstream: foundations-followup1
doc_type: readme
status: shipped
title: foundations-followup1
staging_pointers:
  - dev-docs/workstreams/foundations-followup1/plans/foundations-followup1
  - dev-docs/workstreams/foundations-followup1/spec/2026-06-27-foundations-followup1.md
---

# foundations-followup1

Close the three test coverage gaps documented in Phase 2a (vk-swarm-node-foundations):

1. **GAP 1 (SC1)** — End-to-end crash-resume integration test using `qa_mock` — verifies the
   full fence → classify → resume path in `cleanup_orphan_executions`.
2. **GAP 2 (D4)** — `MockProcessInspector` stubborn-PID mode + `fence_attempt_count` DB column
   + operator escalation warning for processes stuck in D-state.
3. **GAP 3 (D6/SC2)** — Boot-drain full call path integration test — calls
   `drain_queued_messages_on_boot` and asserts `start_queued_message_for_attempt` is reached
   against a real SQLite pool.

**Parent workstream:** `vk-swarm-node-foundations` (Phase 2a, shipped 2026-06-27).
