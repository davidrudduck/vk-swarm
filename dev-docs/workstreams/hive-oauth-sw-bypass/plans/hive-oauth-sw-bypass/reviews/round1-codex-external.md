# Adversarial breakdown review — `hive-oauth-sw-bypass`

> Report creation was blocked by the read-only sandbox. The repository remains unchanged, and `codex-find.md` could not be written at the requested path.

Verdict: **REVISE**

| ID | Severity | Task section | Finding |
|---|---|---|---|
| F1 | blocker | All tasks, `Done when` | Literal `<dir>`, `<typecheck>`, and `<test>` placeholders make every task gate non-executable. Replace them with concrete scoped commands. |
| F2 | major | 202, `Failing test` | No viable NiceModal harness is specified. `OAuthDialog` uses `NiceModal.create`, `useModal()`, and `open={modal.visible}`; merely rendering its wrapper will not expose the GitHub button. Prescribe exact mocks for `defineModal`, `NiceModal.create`, and `useModal`, or an exact Provider/show sequence. |
| F3 | major | 202, test assertions | TS2 requires success-before-deadline coverage, but no such test is prescribed. Add a mutable authenticated status result and assert `reloadSystem`, `modal.resolve`, and `modal.hide`. Replace the hollow “no unmount warning” assertion with a `clearTimeout` spy asserting the deadline timer is cleared. |
| F4 | major | 301 manual verification; 102 frontmatter | SC1 also requires proving `api-cache` contains no `/v1/oauth/*` entries after sign-in. Only `sw.js` grep is prescribed. Add Cache Storage inspection to 301 and move SC1 ownership from 102 to 301. |
| F5 | blocker | 301, manual verification item 2 | “Note” a pre-existing `cargo test --workspace` failure contradicts the mandatory green gate and no-deferred-remediation policy. Require exit 0; otherwise fix, create a legitimate tracked scope split plus ledger entry, or escalate. |
| F6 | major | 301, manual verification item 6 | A post-fix trace cannot identify the old SW failure mechanism because `/v1/oauth` now bypasses Workbox. Prescribe a controlled pre-update trace, followed separately by rebuilt-SW success verification. If unavailable, record the mechanism as indeterminate. |
| F7 | minor | 101, `Change` / `Allowed moves` | It requires recording divergence in the decisions ledger while allowing only creation of two predicate files. Remove the ledger instruction or add the ledger to `files` and allow the append. |
| F8 | minor | 202 anchor 3; 201 | The existing error footer already has a retry button using `oauth.tryAgain`. “Relabel/add alongside” can create duplicate buttons. Require changing exactly `t('oauth.tryAgain')` to `t('oauth.retry')`, retaining `handleBack` and the close button. |

Concrete gate replacements:

- 101: remote-frontend `tsc --noEmit` plus `vitest run src/lib/swCachePredicate.test.ts`.
- 102: remote-frontend typecheck, scoped predicate test, Vite build, and fail-closed `grep -q 'v1/oauth' dist/sw.js`.
- 201: frontend typecheck plus JSON/key validation for all four locales.
- 202: frontend typecheck plus the scoped OAuthDialog test.
- 301: the exact mandatory AGENTS.md gate set, all required to exit zero.

Checked, clean:

- Both task-102 `Before` blocks match `remote-frontend/vite.config.ts` verbatim.
- Task 102 imports the exact module created by task 101.
- Tasks 201 and 202 agree on `oauth.timeoutError` and `oauth.retry`.
- Both frontend Vitest runners are configured for the prescribed paths.
- Task 102’s `forbid_after` literal occurs only in the targeted remote-frontend production source.
- Task 101’s cases honestly cover TS1.
- The proposed deadline effect clears its timer on cleanup.
- The current popup-closed-without-auth path already transitions to an error.
- No backend, schema, wire-format, shell-cache, or broader caching redesign drift was found.