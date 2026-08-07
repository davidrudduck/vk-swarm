# Breakdown review — round 1 verdict

method: hybrid — 1 real external panel (Codex CLI, plan-review, read-only) + sub-agent-fallback
(3 sub-agents: mechanical-correctness, spec-fidelity, decomposition-quality). External dispatch
failed for 2 of 3 competitors: agy (Antigravity) individual quota exhausted (resets ~7 days);
opencode config invalid (`~/.config/opencode/opencode.json`: unrecognized keys plugins, config).
Per the decompose fallback rule, peer-judging was skipped; the orchestrator independently
verified every finding against the repo before applying.

## Scoreboard (validated findings, deduplicated across panels)

| finding | source(s) | verdict | remediation |
|---|---|---|---|
| generateSW serializes urlPattern via toString → imported predicate ships broken sw.js | mechanical F1 | CONFIRMED blocker; contradicted the frozen spec's Design | spec amended (inline self-contained predicate; module demoted to tested mirror + drift guards) and re-frozen via /wai:precheck; plan resubmitted |
| forbid_after literal false-positives on frozen spec + dev-docs (gate greps whole tree minus docs/plans/<topic>) | quality F1, fidelity F4 | CONFIRMED blocker (grep evidence) | forbid_after replaced with the repo-unique deleted comment literal "Shape requests bypass the SW cache." |
| Done-when gate lines non-executable (placeholders + wrong task-gate.sh path) | codex F1, mechanical F2 | CONFIRMED | all 5 Done-when lines filled with dynamic WAI_ROOT + concrete scoped commands |
| oauth.retry duplicates existing oauth.tryAgain; error branch already has retry button | codex F8, mechanical F3, quality F2/F3/F5, fidelity F6 | CONFIRMED | 201 adds only timeoutError (deterministic insertion after tryAgain, sibling+STOP updated); 202 makes NO error-branch change; tests assert oauth.tryAgain |
| SC1 runtime leg owned by wrong task; Cache Storage clause uncovered | codex F4, fidelity F1/F2, quality F4, mechanical F6 | CONFIRMED | SC1 moved to 301; 301 step 5 now includes api-cache DevTools inspection |
| SC4 runtime observation never performed live | fidelity F3 | CONFIRMED | SC4 moved to 301 with a stalled-flow live check (abandoned popup → polling ceases + timeout UI); 202 keeps TS2 |
| 202 harness underspecified / mock wiring wrong (onInitSuccess is a hook option, not a mutate arg) | codex F2, mechanical F5, fidelity F7 | CONFIRMED | full mock harness prescribed per TaskFormSheet sibling incl. options-capture factory, act() wrapping, raw-key t mock |
| missing success-before-deadline case; hollow unmount assertion (React 18) | codex F3, mechanical F4, fidelity F5 | CONFIRMED | success case + vi.getTimerCount()===0 cleanup assertion prescribed |
| deadline effect deps [isPolling, t] resets timer on language change | quality F6 | CONFIRMED | deps fixed to [isPolling] with eslint-disable rationale; also captured in amended spec Design |
| "note pre-existing cargo test failure" contradicts no-deferred-remediation | codex F5 | CONFIRMED | 301 requires exit 0; CLAUDE.md fix/split/escalate wording in STOP triggers |
| pre-fix mechanism trace may be impossible post-fix | codex F6 | CONFIRMED | 301 allows "mechanism: indeterminate — fix correct under both" |
| 101 ledger-write vs allowed_moves conflict | codex F7 | CONFIRMED (minor) | ledger instruction removed from 101 change text; sibling-read note kept |

Termination: all validated findings remediated in the v2 envelope resubmission + post-render
Done-when fill; focused re-check of the rendered tree passed (anchors re-verified, coverage map
SC1-4/TS1-3 each claimed exactly once). Round closed.
