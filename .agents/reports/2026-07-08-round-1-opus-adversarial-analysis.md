I've completed the adversarial review. Note: the harness forced **plan mode**, which blocked writing to the requested path `.agents/reports/2026-07-08-round-1-opus-adversarial-analysis.md`. I persisted the full report to the only writable location (`/home/david/.claude/plans/do-not-modify-files-snug-harbor.md`) and reproduce it below. **Approve/exit plan mode and I'll write it to the intended path verbatim.**

---

# Adversarial Analysis — fix-nonloopback-signin

## Verification I actually ran (not just read)

- **Fallback SHA-256 is byte-exact.** I extracted `sha256Fallback` and compared it to Node's `crypto.createHash('sha256')` across lengths `0,1,3,55,56,63,64,65,100,200,1000` (every padding/block boundary) plus a multi-byte UTF-8 string — **all match**. No correctness bug in the hash.
- **Frontend↔backend contract is consistent.** Backend verifies `hash_sha256_hex(verifier)` and requires 64-char lowercase hex (`crates/remote/src/auth/handoff.rs:332-334,482-485`); frontend emits exactly that via `bytesToHex`. Hex (not base64url) is intentional and required.
- **12/12 tests pass**, **eslint exits 0** on all 5 changed files.

## Issues Found

### Issue 1 — `no-push-invariant` node-test orphaned from all automated execution
- **Severity:** medium · **File:line:** `remote-frontend/vite.config.ts:74`
- Adding `scripts/**` to Vitest's `exclude` correctly stops Vitest mis-collecting the `node:test` file — but nothing else runs it. No CI workflow runs `npm run test:run` for `remote-frontend` or `node --test scripts/no-push-invariant.test.mjs`, and `package.json` has no script for it. The only "wiring" is prose in a *different* workstream's doc. This is the exact silent-disable scenario CLAUDE.md warns against (category went from 1 live test → 0, no follow-up created this session).
- **Fix:** add `"test:invariants": "node --test scripts/no-push-invariant.test.mjs"` to `package.json` and invoke it from `remote-hive-build.yml` (or open a tracked follow-up). **~20 min.**

### Issue 2 — Multi-block SHA-256 path has no value-asserting test
- **Severity:** low · **File:line:** `remote-frontend/src/pkce.test.ts:37-54`
- Exact-value tests only use single-block inputs (`""`, `"abc"`); the 43-byte case checks only a regex. The multi-block loop/length-encoding is never value-checked (I verified it correct out-of-band). **Fix:** add a fixed two-block NIST vector + a pinned 43-byte-verifier value. **~15 min.**

### Issue 3 — Inconsistent crypto guarding
- **Severity:** low · **File:line:** `pkce.ts:3` vs `pkce.ts:15`
- `sha256()` uses `globalThis.crypto?.subtle` defensively; `generateVerifier()` uses bare `crypto.getRandomValues`. Not a runtime bug (getRandomValues exists in insecure contexts), but asymmetric. **Fix:** guard or comment the assumption. **~5 min.**

### Issue 4 — Trivially-true test assertion
- **Severity:** low · **File:line:** `AppRouter.test.tsx:115`, `InvitationPage.test.tsx:76`
- `expect(resolveInitOAuth).toBeTypeOf('function')` is always true and asserts nothing. **Fix:** drop or replace with `initOAuth` call-count / verifier round-trip. **~5 min.**

## Dismissed non-issues
Hex encoding (correct per backend), `data as BufferSource` cast (documented TS quirk), tests keeping `initOAuth` pending (intentional; LAN nav proven by task-301 Playwright), bitwise-overflow/negative rotate (normalized by `>>> 0`, empirically exact).

## Summary
**4 findings: 1 medium, 3 low. No correctness or security defects.** The fallback is a correct, verified SHA-256 and the challenge contract is consistent end-to-end. Only Issue 1 has real teeth and should be closed this session. The workstream is well-implemented and the ledger's acceptance evidence holds up.