# Integrated adversarial review — browser OAuth phase 1

## Scope

- Workstream: `local-node-browser-oauth`
- Phase: 1, tasks 001–005
- Diff: `41f55c4b..8fd674a8`
- Panel: Opus, GPT, Sonnet; all read-only
- Focus: interactions among migration, deterministic auth seams, owner pinning, handoff claim,
  session persistence/revocation, and the already-locked phase-2/3 composition.

## Verdicts

| Panelist | Verdict | Result |
|---|---|---|
| Opus | CONFORMS | No defect in the landed phase-1 primitives. |
| Sonnet | CONFORMS | No defect in the landed phase-1 primitives. |
| GPT | DEVIATES | One blocking integration race in the locked callback/disconnect composition. |

## Validated finding

`create_session` inserts a live row independently (`crates/db/src/models/browser_auth/session.rs`),
while `revoke_all_sessions` can update only rows that already exist. The original task 011 sequence
saved credentials and inserted a session after Hive I/O; task 012 revoked all existing sessions and
then cleared daemon state. An already-claimed callback could therefore pause in Hive I/O,
disconnect could return, and the callback could then recreate credentials/session/sync.

The review's exact SQLite ordering probe reported:

```text
revoke_all_rows=1 live_after_disconnect=['after']
```

This violates frozen SC8: when explicit disconnect returns, all browser sessions must be revoked,
sync stopped and daemon credentials absent.

Two related races were independently verified while designing the fix:

1. Existing `Deployment::spawn_remote_sync` installs the handle in a detached task, so disconnect
   can observe `None` and return before the detached task installs `Some`.
2. An in-flight token refresh can save credentials after disconnect clears them unless clear is
   serialized with the existing `AuthContext::refresh_guard`.

## Remediation decision

No second migration was authorized. A durable generation was therefore rejected. Corrective task
022 adds only reversible primitives:

- durable `invalidate_pending_handoffs`, reusing the existing terminal `claimed` state;
- one per-deployment `Arc<Mutex<u64>>` browser-auth commit epoch;
- synchronous login-path sync installation.

Tasks 009–012 were tightened before phase 2:

- initiation holds the epoch only around durable handoff insertion;
- claim and epoch capture share one short guard, with all Hive I/O unlocked;
- callback re-checks the epoch before saving credentials, creating a session and synchronously
  installing sync; credential save is also serialized with refresh;
- disconnect holds the epoch while incrementing it, invalidating pending handoffs, revoking all
  sessions, stopping sync and clearing credentials; refresh guard is acquired only immediately
  around clear, after `client.logout`, to avoid re-entrant deadlock.

Task 012 now requires deterministic barrier tests for callback-vs-disconnect, pending-handoff
invalidation, fresh login after disconnect, and in-flight refresh resurrection.

## Residual constraint

SQLite session revocation and file/Keychain credential deletion cannot share a transaction (O8).
A crash between them can leave an over-locked-out node with credentials still present. Closing
that crash window needs new durable state and another irreversible approval. Recovery is to retry
disconnect; the chosen revoke-first order never leaves an authorized browser after credential
removal.

## Pre-existing failure discovered

Panel runs intermittently failed the two execution-process `SQLITE_BUSY_SNAPSHOT` calibration
controls because they could not provoke the expected hazard in 200 attempts. The reviewed diff
does not touch those files and repeated identical runs oscillated. Per AGENTS.md this will be
resolved as the explicit tracked scope split `sqlite-busy-snapshot-calibration-stability` before
the session closes; it is not silently carried forward.

## Status

Plan correction passed `wai-plan-lint.sh`. This review remains open until task 022 passes its
deterministic gate and adversarial panel and the integrated focused re-review confirms the
remediation primitives. Route-level closure occurs through the amended tasks 009–012 in this same
execution workstream.
