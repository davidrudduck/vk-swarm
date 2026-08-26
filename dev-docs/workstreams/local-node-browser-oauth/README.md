---
workstream: local-node-browser-oauth
doc_type: readme
status: draft
title: "local-node-browser-oauth"
staging_pointers:
  - docs/superpowers/specs/2026-08-21-local-node-browser-oauth.md
---

# local-node-browser-oauth

local-node-browser-oauth

## Acceptance evidence (TS7)

Date: 2026-08-26. Feature-branch commit `b776159b` (`gentle-mongoose`). Isolated node `HOST=0.0.0.0` `BACKEND_PORT=9012` LAN `http://10.69.96.233:9012` (not `127.0.0.1`). Assets `/tmp/opencode/vlnbo-ts7/` so live `/home/david/Tools/vk-swarm` DB was not migrated. Real Hive `https://vkswarm.thedoctor.raverx.net`. Same-host via LAN IP: human browsers A/C plus Playwright clean profile B. User production node on `:9002` (`main` `1f2caaea`) left running.

### A. Full repository gates

1. `cargo fmt --all -- --check` — EXIT 0 (stable-channel rustfmt option warnings only). 2026-08-26.
2. `cargo clippy --all --all-targets --all-features -- -D warnings` — EXIT 0 after a 927-crate rebuild (`TMPDIR=$PWD/.cargo-tmp`, `DISABLE_WORKTREE_ORPHAN_CLEANUP=1`). 2026-08-26.
3. `cargo test --workspace` — EXIT 101 once: only `crates/services/tests/normalize_sync_test.rs::test_fast_execution_no_lost_logs` (`Expected at least 1 JsonPatch entry for fast execution, got 0`). Tracked pre-existing flake `F-2026-08-04-02` / `dev-docs/workstreams/services-normalize-flaky-test/`. Isolated re-run `cargo test -p services --test normalize_sync_test test_fast_execution_no_lost_logs`: 1 passed, 0.61s. All other workspace suites green on the full run. 2026-08-26.
4. `npm run generate-types:check` — EXIT 0; `JSON schemas generated. 9 schemas created.`; `shared/types.ts is up to date.` 2026-08-26.
5. `cd frontend && npm run lint && npm run format:check && npx tsc --noEmit && npx vitest run` — first `format:check` failed on 16 files; `20b8ee3d` prettier-format (8 this workstream, 8 pre-existing). After that: lint EXIT 0, format:check EXIT 0, tsc EXIT 0, vitest 50 files / 560 passed. 2026-08-26.
6. `cd remote-frontend && npm run lint && npx tsc --noEmit && npx vitest run` — incomplete `node_modules` first; `pnpm install --filter ./remote-frontend --frozen-lockfile` then lint/tsc/vitest EXIT 0 (54 files / 426 passed). 2026-08-26.
7. `bash scripts/check-i18n.sh` — EXIT 0 (prints missing keys in `ko/tasks`; warning-style). 2026-08-26.
8. `bash scripts/test-verify-local-node-browser-oauth.sh` — EXIT 0, `ALL VERIFIER TESTS PASSED`. 2026-08-26.

### B. Trusted-LAN acceptance

1. **Two-browser isolation.** A (human) completed GitHub OAuth and reached the board. B (Playwright, `http://10.69.96.233:9012/`): button `Log in`, `document.cookie` empty, both Storages empty, `GET /api/info` 401, `/api/auth/state` `{authorized:false,oauth_available:true}`. Replaying claimed handoff `GET /api/auth/handoff/complete?handoff_id=96d50416-5358-4e1a-afc1-130ffc2af4ba&app_code=replayed-from-browser-B` → HTTP 400; B still login-shell, still 401 `/api/info`. 2026-08-26.
2. **Restart persistence.** Isolated pid 1160670 killed; relaunch pid 1596616, same sqlite. Health `git_commit=b776159b`. Four live `browser_sessions` survived. User A reloaded 9012: still signed in, no re-auth. 2026-08-26.
3. **Transient Hive outage.** `docker stop remote-remote-server-1` → Hive `https://vkswarm.thedoctor.raverx.net/v1/health` HTTP 502. A: projects/task still work. B: `POST /api/auth/handoff/init` → 502; stayed login-shell. `docker start remote-remote-server-1` → health 200 in ~1s. sqlite live=4 revoked=0 (nothing revoked by the outage). Live logs/SSE were not separately watched in that window. 2026-08-26.
4. **Logout scope.** A Navbar Sign out (not Disconnect). sqlite: session `D9BE529A` revoked 2026-08-26 03:34:49; three others still live. A login-shell; C still on the board. 2026-08-26.
5. **Disconnect scope.** C Settings → Swarm → Disconnect from Hive, EVERY-browser confirm. Three remaining sessions revoked together at 2026-08-26 03:38:12. A and C both login-shell. `node_owner.slot=1` hive_user `D3CB771A124B4F98AAD32C9C39792862` still pinned at 2026-08-26 01:41:13. Two new sessions at 03:38:34 / 03:38:52 are post-disconnect re-logins. A second Hive identity was not available, so live different-account refusal was not exercised here (SC6 already covered by 011/018). 2026-08-26.
6. **Deployed verifier** from LAN URL (full output kept for ledger D):

```
$ bash scripts/verify-local-node-browser-oauth.sh http://10.69.96.233:9012
PASS health is public
PASS auth state is public
PASS auth state has the exact minimal shape
PASS info is protected
PASS projects are protected
PASS status is protected
PASS events SSE is protected
PASS live logs are protected
PASS unknown api path is 404
PASS unknown api path is not SPA html
All browser-authorization boundary checks passed
```

VERIFIER_EXIT=0. 2026-08-26.

7. **Non-disclosure.** Playwright B: `document.cookie` empty; Storages empty; body only `Log in`. Signed-in C DevTools: `document.cookie` does not contain `vks_browser_session`; no Hive access/refresh token in storage/URLs/DOM. 2026-08-26.

In-session 016 repair (not 021 files): popup-close aborted poll before `getState`; plan `fc684953` + `b776159b`; user: "working now". Login-shell markup stayed the locked one-button shell.

### C. Sidecar A6

Probed 2026-08-26: real Hive + non-loopback LAN browsers were available. Frozen `.decisions.json` not edited.
