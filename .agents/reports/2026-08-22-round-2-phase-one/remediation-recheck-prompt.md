# Phase-1 remediation recheck

Read-only focused re-review of the `local-node-browser-oauth` integrated phase-1 remediation.

Repository under review: the supplied detached worktree at commit `fa0612ee`.

Original integrated target: `41f55c4b..ae5ee15f`, governed by
`docs/superpowers/specs/2026-08-21-local-node-browser-oauth.md`, phase-1 task files 001–005 and 022,
and `docs/plans/local-node-browser-oauth/decisions-ledger.md`.

Review the remediation range `ae5ee15f..fa0612ee`, with emphasis on two confirmed panel findings:

1. Task 022's startup fixture could select the production macOS Keychain backend. Verify
   `OAuthCredentials::new_file_backed()` is explicit/path-scoped, production detection is unchanged,
   every task-022 fixture that saves credentials uses it, its regression test is discriminating,
   and task 022's plan/status/gate evidence are coherent.
2. The promised `sqlite-busy-snapshot-calibration-stability` scope split was missing and `cargo test
   -p db` failed probabilistically. Verify the workstream exists, both negative controls now force a
   genuine WAL read-snapshot/intervening-commit/write-upgrade schedule, require extended code 517,
   retain live production write-first tests, and do not hide failures with ignores or gate changes.

Also inspect this remediation range for regressions, contract/scope contradictions, unsafe test
side effects, hollow assertions, and misleading ledger/report claims. Confirm the original accepted
O8 crash window and pending route work 009–012 have not been silently declared complete.

Use exact file:line citations and command evidence. Run focused tests/clippy if useful. Set
`DISABLE_WORKTREE_ORPHAN_CLEANUP=1` for local-deployment tests, and use a root-filesystem TMPDIR if
`/tmp` quota fails. Do not edit, restore, reset, clean, stash, commit, or remove any worktree.

End exactly with `VERDICT: APPROVE` if no blocking/should-fix finding remains, otherwise
`VERDICT: REJECT`.
