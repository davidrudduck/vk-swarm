# Integrated adversarial review — local-node browser OAuth phase 1, round 2

## Scope and governing intent

- **Initial target:** local committed range `41f55c4b..ae5ee15f`, covering phase-1 tasks
  001–005 and corrective task 022.
- **Remediation target:** the same range plus remediation through `d9551f8a`.
- **Governing intent:** `docs/superpowers/specs/2026-08-21-local-node-browser-oauth.md`, the
  phase-1 task files, and `docs/plans/local-node-browser-oauth/decisions-ledger.md`.
- **Push gate:** not used. The user explicitly prohibited pushing, so every challenger reviewed an
  explicit local commit range in a detached disposable worktree rather than a branch or PR.

## Isolation and resilience record

The initial Codex seat used detached worktree
`/home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-ae5ee15f-116053` at `ae5ee15f`.
The first Opus attempt caused the known pre-existing orphan-cleanup test hazard to delete that clean
worktree and timed out. The worktree was recreated as
`/home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-ae5ee15f-1106208`; the successful Opus
rerun set `DISABLE_WORKTREE_ORPHAN_CLEANUP=1`. Remediation review used
`/home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-fa0612ee-150627` at `fa0612ee`.
The final focused Opus recheck used
`/home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-6a304873-1372255` at `6a304873`.
All successful native JSON records have `status: ok`, empty `notes`, and
`stub_risk_accepted: false`.
Markdown reports and runner JSON records are retained as evidence; verbose transport raw logs were
intentionally omitted from the committed artifacts.

| Seat | Transport / identity | Initial verdict | Remediation result |
|---|---|---:|---:|
| Opus | native Claude Code 2.1.240, `opus`, high effort | REJECT | REJECT on evidence wording, then focused APPROVE |
| Codex | native codex-cli 0.149.0, `gpt-5.6-sol`, high effort | REJECT | APPROVE |
| Kimi state | OpenCode Task, Kimi K3 family | APPROVE | focused Kimi APPROVE |
| Kimi fidelity | OpenCode Task, Kimi K3 family | APPROVE | focused Kimi APPROVE |
| GLM SQL | OpenCode Task, GLM 5.3 family | APPROVE | focused GLM APPROVE |
| GLM concurrency | OpenCode Task, GLM 5.3 family | APPROVE | covered by focused remediation review |
| Grok integration | OpenCode Task, Grok 4.6 family | APPROVE | focused Grok APPROVE |
| Grok tests | OpenCode Task, Grok 4.6 family | APPROVE | test observations fixed and independently approved |

Both required native seats ran successfully and all six OpenCode seats returned substantive,
non-stub reports. The initial vote was 6/8 APPROVE (75%, above the two-thirds merge-vote floor),
but the two rejecting seats had confirmed findings, so the review remained open until remediation.

## Consolidated findings and disposition

| Issue | Tag | Accepted? | Impact if shipped | Remediation / evidence |
|---|---|---:|---|---|
| Startup fixture could select the fixed production macOS Keychain entry and save `test-refresh-token` | BLOCKING | Yes | A test could overwrite a developer's real Hive refresh token | `594d531c` added `OAuthCredentials::new_file_backed`; task fixtures use temporary paths. `3e128082`/`9d705438` made the same-module test inspect `Backend::File` and the exact path before any save. Production `new()`/detection remain unchanged. |
| Promised `sqlite-busy-snapshot-calibration-stability` split was absent and full DB tests were red at the probabilistic 0/200 control | BLOCKING / SHOULD-FIX | Yes | Mandatory DB gate could fail while the finding remained silently untracked | `7b5d6eff` created the workstream and deterministically forces read snapshot → intervening commit → write upgrade, asserting SQLite extended code 517. No test was ignored; full `cargo test -p db` passed. |
| Deterministic controls were inaccurately described as calibrating an identical production stress harness | SHOULD-FIX | Yes | Evidence could claim mutation resistance the scheduler-sensitive real-function stress tests do not provide | `3e128082`, `0cd57f5a`, `07ec2f8c`, and `d9551f8a` now distinguish deterministic hazard controls from supplemental scheduler-sensitive stress tests and accurately distinguish five-second test from thirty-second production timeout. |
| Explicit-file regression only asserted that a file existed after save | SHOULD-FIX | Yes | On common Linux/debug configurations the test would not prove backend selection and could discover a Keychain regression only after writing | The test now structurally matches the private backend and exact path before persistence, with a pre-save macOS Keychain panic arm. Native Opus focused recheck: APPROVE. |
| Task 022 startup diagnostics and comments drifted from injected/resolved configuration | INFO | Yes | Misleading logs and stale maintenance guidance | `547b373b` derives `has_shared_api` from resolved `api_base`, fixes the SAFETY comment, task examples, clippy scope, and generated MASTER status. |
| Terminal handoff/session retention lacked an explicit future policy | INFO | Yes | Unbounded terminal rows without a product/storage decision | Phase-1 deletion remains deliberately forbidden. Tracked scope split: `dev-docs/workstreams/browser-auth-terminal-row-retention/README.md`. |
| Tests did not distinguish terminal-state UPDATE from DELETE | INFO | Yes | A forbidden DELETE mutant satisfied the old assertions | `16be3a50` asserts the invalidated handoff row remains `claimed`; `4c80e19e` asserts both revoked session rows and exact timestamps remain. Task 022 and task 005 deterministic gates passed. |
| Configured-startup test leaves a node-cache task for runtime teardown | INFO | No — evidence-dismissed | Test-only resource hygiene concern, no production or cross-test leak | The task uses a private temporary DB; Tokio runtime teardown aborts remaining tasks; RemoteSync and event bus are explicitly shut down; no deployment-level node-cache stop handle exists. Adding a production API solely for the fixture would exceed scope. |
| `RemoteSync::spawn` URL parse could panic synchronously | INFO | No — evidence-dismissed | Potential startup panic if configuration were unparseable | `ShareConfig.api_base` is already a parsed `Url`; serializing it yields a parseable URL. No raw/sync same-byte invariant is claimed. |
| Task 022 omitted `siblings:` | INFO | No — evidence-dismissed | Planning metadata concern only | Task 022 edits existing modules, creates no sibling file, and plan lint emits no task-022 sibling advisory. |

## Verified phase-1 behavior

The panel independently confirmed the additive migration, singleton owner pin, atomic and
single-consumer handoff claim, non-expiring revocation-state sessions, clone-shared per-deployment
browser-auth epoch, configured-startup synchronous RemoteSync installation, raw API-base client
compatibility, exact task file scopes, and every phase-1 STOP trigger. Route-level SC8 is still
explicitly incomplete until tasks 009–012 consume these primitives. The O8 SQLite-versus-credential
store crash window remains the approved residual documented in the spec and ledger.

Final focused evidence included:

- `cargo test -p db browser_auth` — 17 passed.
- Full `cargo test -p db` after calibration repair — all unit, integration, and live doctests passed
  with only the pre-existing selectively ignored tests.
- Task 022 gate at `16be3a50` — `CONFORMS`, `GATE_FAIL_CHECK=none`.
- Task 005 gate at `4c80e19e` — `CONFORMS`, `GATE_FAIL_CHECK=none`.
- `cargo clippy -p db -p deployment -p services -p local-deployment --all-targets
  --all-features -- -D warnings` — passed.
- `cargo fmt --all -- --check`, plan lint, MASTER status check, and `git diff --check` — passed.
- Native Opus evidence recheck — `status: ok`, no notes/stub, `VERDICT: APPROVE`.
- Two independent final focused reviewers approved the terminal-row assertions and test-runtime
  hygiene dismissal; their only additional wording finding was fixed in `d9551f8a` and reverified.

## Final verdict

**APPROVE.** No confirmed BLOCKING or SHOULD-FIX finding remains. All concrete INFO findings were
fixed, tracked as legitimate scope splits, or dismissed with cited evidence. Phase 1 may advance to
task 006; this verdict does not claim completion of route-level SC8.

## Lessons learned

The cross-family pass caught two problems an implementation-only review could easily rationalise
away: a platform-specific test could mutate a real macOS Keychain even though it was harmless on
Linux, and a flaky calibration test could leave the mandatory DB gate red despite being unrelated
to browser auth. The remediation recheck also separated a true deterministic SQLite hazard proof
from a scheduler-sensitive stress test; accurate evidence matters as much as green output. Finally,
STOP-trigger semantics should be asserted positively: proving that terminal rows still exist is
stronger than merely proving they can no longer authenticate or be claimed.
