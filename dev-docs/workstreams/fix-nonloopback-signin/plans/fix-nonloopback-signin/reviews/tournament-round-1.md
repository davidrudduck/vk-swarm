# Tournament round 1 — fix-nonloopback-signin

Method: three competitors performed find+remediate over the WAI breakdown, then non-self peer judges validated findings.

Competitors:

- Codex: `reviews/tournament-round-1/codex-find.md`
- Gemini: `reviews/tournament-round-1/gemini-find.md`
- OpenCode GLM-5.2: `reviews/tournament-round-1/opencode-find.md`

Peer judges:

- Codex judged Gemini: `reviews/tournament-round-1/codex-judge-gemini.md`
- Gemini judged OpenCode: `reviews/tournament-round-1/gemini-judge-opencode.md`
- OpenCode judged Codex: `reviews/tournament-round-1/opencode-judge-codex.md`

## Scoreboard

| competitor | findings submitted | validated issues | validated fixes | score |
|---|---:|---:|---:|---:|
| Codex | 3 | 2 | 2 | 4 |
| Gemini | 2 | 1 | 1 | 2 |
| OpenCode | 2 | 2 | 1 | 3 |

Notes:

- Codex finding 3 (`allowed_change: create` with read-only context files) was rejected by OpenCode. The task contract permits unchanged context files in `files:` and the lint advisory purpose was already documented.
- Gemini finding 2 (task 202 missing navigation mock) was rejected as a crash claim by Codex, but the broader missing navigation handling was independently validated by Codex and OpenCode findings. The applied remediation uses the peer-corrected safe approach: keep `initOAuth()` pending in jsdom unit tests and require real browser redirect proof in task 301.
- OpenCode finding 2's issue was validated, but Gemini rejected the proposed shell snippet because repeated `cd remote-frontend` would fail after the first `cd`. The applied remediation uses subshells.

## Validated findings and remediations

### V1 — Task 301 had a hollow acceptance gate

Validated by:

- Codex finding 1, judged real by OpenCode.
- OpenCode finding 2, judged real by Gemini with corrected shell mechanics.

Problem:

`phase-3/301-full-gates-manual-lan-verification.md` used `WAI_TYPECHECK_CMD="true" WAI_TEST_CMD="true"`, so `task-gate.sh` could pass without running automated gates or checking that manual LAN evidence was recorded.

Applied remediation:

- Added `docs/plans/fix-nonloopback-signin/verify-301-evidence.sh`.
- Updated task 301 `Done when` to run all targeted remote-frontend tests, the full remote-frontend suite, remote-frontend lint/typecheck, Rust clippy/test, frontend lint/typecheck, then the ledger evidence verifier through `WAI_TYPECHECK_CMD`.
- The verifier fails if `## Acceptance evidence` is missing, any automated gate PASS line is missing, either LAN PASS line is missing, or placeholders/failing/unavailable/inconclusive terms remain.

Focused re-check:

- `bash -n docs/plans/fix-nonloopback-signin/verify-301-evidence.sh` passed.
- `wai-plan-lint.sh fix-nonloopback-signin` passed.

### V2 — Route tests did not handle provider navigation safely

Validated by:

- Codex finding 2, judged real by OpenCode.
- OpenCode finding 1, judged real by Gemini.
- Gemini finding 1, judged real by Codex with a corrected remediation.

Problem:

Tasks 201 and 202 either prescribed an unsafe `vi.spyOn(window.location, 'assign')` pattern or omitted navigation handling. The peer-reviewed correction was to avoid brittle jsdom `window.location` redefinition in unit tests and keep `initOAuth()` pending after asserting challenge/storage behavior. The real redirect is then proven by task 301's mandatory LAN browser verification.

Applied remediation:

- Removed the `window.location.assign` spy instructions from task 201.
- Updated task 201's login test to make mocked `initOAuth()` return a pending promise after asserting provider/challenge/storage behavior.
- Updated task 202's invitation OAuth test to make mocked `initOAuth()` return a pending promise after asserting provider/challenge/storage behavior.
- Added explicit STOP triggers forbidding brittle jsdom `window.location` mocking unless a fresh repo-local descriptor proof exists and is recorded.
- Task 301 remains responsible for actual provider authorization URL reachability over non-loopback LAN HTTP.

Focused re-check:

- `wai-plan-lint.sh fix-nonloopback-signin` passed.

## Closure

All peer-validated findings were remediated. No validated remediation required changing the frozen spec. The tournament closes after the focused checks above per the termination rule.
