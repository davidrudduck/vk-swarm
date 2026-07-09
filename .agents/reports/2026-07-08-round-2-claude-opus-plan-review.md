# Adversarial Review — hive-node-api-key-ui Plan vs Implementation
**Panelist:** Claude Opus (claude-opus-4-8)
**Date:** 2026-07-08
**Scope:** Full plan tree (8 tasks) vs committed implementation

## Summary
The implementation meets every success criterion **at runtime** — the section renders, all four CRUD flows are wired, `confirm()` gates destructive actions, and the plan's traps were all honored. Two latent locale defects were discovered and remediated this session: (1) `en.create` = "Create API Key" contradicted SC1/US1's required "Generate API Key", and (2) the `cancel` key was reused for two semantically different buttons. Both are now fixed. Verdict: **PASS**.

## Spec Criteria Assessment

### SC1: "Generate API Key" button + management section, gated on orgId — **PASS**
- `Nodes.tsx:31` renders `{orgId && <NodeApiKeySection organizationId={orgId} />}` above the node grid; the component also self-guards `if (!organizationId) return null` (`NodeApiKeySection.tsx:206`). Button at `:219-226`.
- Button label uses `t('settings.swarm.apiKeys.create', 'Generate API Key')`. The `en` locale value was corrected from "Create API Key" to "Generate API Key" as part of this review's remediation.

### SC2: Create Dialog → one-time secret view w/ show/hide + copy; close clears secret — **PASS**
- Dialog opens on button click (`:220`), name-input view (`:280-314`) submits via `createMutation.mutate(newKeyName)` calling `nodesApi.createApiKey({ organization_id, name })` (`:136`).
- On success, `createdSecret` is set (`:139`) and the dialog transitions to the secret view (`:315-376`) showing `response.secret`, a show/hide toggle (`:337`), and a copy button using `navigator.clipboard` with the `document.execCommand('copy')` non-HTTPS fallback (`:186-204`).
- `closeDialog()` (`:178-184`) clears `createdSecret`/`showSecret`/`copied`/`newKeyName`; wired to both the Done button and the Dialog `onOpenChange` (`:275-277`). TS4 verifies the full round-trip including re-open showing the name view.

### SC3: Active keys listed with name, key_prefix, bound/unbound badge, created/last-used — **PASS**
- `ApiKeyItem` (`:35-113`) renders name (`:46`), `key_prefix` (`:47`), a `default`/`secondary` Bound/Unbound badge from `node_id` (`:65-69`), created date (`:78`, `slice(0,10)`) and last-used when present (`:81-88`). TS3 asserts all of these.
- Divergence from reference (benign): the reference impl filtered to `activeKeys`; this component drops that filter and instead renders revoked keys with a `"Revoked"` badge and no action button (`:60-63`, `:101`). Documented in ledger task 001.

### SC4: Revoke calls revokeApiKey after confirm(); query invalidated on success — **PASS**
- `handleRevoke` (`:169-172`) gates on `confirm(...)` then `revokeMutation.mutate`; `onSuccess` invalidates `['nodeApiKeys', organizationId]` (`:152`). TS5 verifies confirm-true calls revoke + invalidates, and confirm-false aborts.

### SC5: Blocked badge + reason; Unblock calls unblockApiKey after confirm() — **PASS**
- Blocked key renders a destructive `"Blocked"` badge with the reason in a Tooltip (`:48-59`) and inline destructive text (`:72-76`); Unblock button (`:92-100`) → `handleUnblock` (`:173-176`) gates on `confirm()` then `unblockMutation.mutate`, invalidating the query (`:162`). TS6 verifies.
- Divergence: SC5 says the badge "updates to Active." There is no `"Active"` badge; post-unblock invalidation refetches and the key renders Bound/Unbound. Intent (key restored) met; the literal "Active" label was intentionally dropped. Documented in ledger.

### SC6: Strings via useTranslation w/ settings.swarm.apiKeys.* + English fallbacks; all 4 locales updated — **PASS (with caveat)**
- Every user-facing string uses `t(key, fallback)` with an English fallback; keys are namespaced `settings.swarm.apiKeys.*`; the `en` block was added and `es/ja/ko` gained an `apiKeys` block.
- Caveat: `es/ja/ko` have empty-string values, not translations. Per ledger task 007, this was documented as "no swarm section existed." The `remote-frontend` has no i18n runtime (pre-existing; shared by all `swarm/*` components), so only fallbacks render. Not a regression.

### SC7: Vitest covers create/list/revoke/unblock + loading/error/empty — **PASS**
- 26/26 tests pass. TS1 loading, TS2 empty, TS3 list, TS4 create+secret, TS5 revoke, TS6 unblock, TS7 error-Alert, TS8 page integration, TS9 key-presence.

## Plan Fidelity

### Divergences Found
| # | Plan Requirement | What Actually Happened | Classification | Action |
|---|-----------------|----------------------|----------------|--------|
| 1 | i18n keys match fallbacks | `en.create`="Create API Key" vs fallback "Generate API Key" | **Harmful (latent)** | **Fixed**: `en.create` → "Generate API Key" |
| 2 | Distinct keys per button | `cancel` key reused for "Cancel" and "Done" buttons | **Harmful (latent)** | **Fixed**: added `done` key, used in component + all 4 locales + TS9 test |
| 3 | es/ja/ko parity translations (spec Design) | Empty-string placeholders added instead (ledger 007) | Needed (documented) | Acceptable given no i18n runtime |
| 4 | Reference `activeKeys` filter | Dropped; revoked keys shown with Revoked badge, no action | Needed (documented) | None |
| 5 | SC5 badge "updates to Active" | No Active badge; renders Bound/Unbound post-unblock | Needed (documented) | None |
| 6 | Spec key names (`createdTitle`,`copySecret`) | Renamed to `secretTitle`,`secretDescription`; added `namePlaceholder` (ledger 002) | Needed (documented) | None |
| 7 | TS9 "every t() call scanned dynamically" | Implemented as a hardcoded required-keys list, not a dynamic source scan | Needed (weaker) | Note only |
| 8 | Import placed before NodeCard (spec) | Placed after NodeCard (`Nodes.tsx:3-4`) | Needed (cosmetic) | None |

### Plan Traps Audit
- **Sibling-read mandatory on new component file** — Recorded in ledger task 001 (pattern siblings + reference impl divergences enumerated).
- **i18n fallbacks mandatory (every `t(key, fallback)`)** — Every `t()` call in `NodeApiKeySection.tsx` passes an English fallback.
- **Vitest is green** — Verified 26/26; `package.json` vite/vitest version untouched.
- **Component is the only consumer of API hooks (no `useNodeApiKeys`)** — `useQuery`/`useMutation` inline in the component; no hook extracted.
- **`useOrganizations` is the org seam; section takes `organizationId` prop** — `Nodes.tsx:9-12` owns the hook; prop passed at `:31`.
- **`confirm()` for destructive actions, not AlertDialog** — `:170`, `:174`; no AlertDialog import.
- **No new Settings page** — Composed only into `Nodes.tsx`; no route/page added.
- **No `isAdmin` prop** — Component signature is `{ organizationId: string }` only.

## Bugs / Regressions / Security Issues

1. **[FIXED] `en.create` contradicted SC1/US1.** `frontend/src/i18n/locales/en/settings.json` had `create` = "Create API Key" but SC1/US1 requires "Generate API Key". Fixed to "Generate API Key".
2. **[FIXED] `cancel` key reused for two buttons.** `NodeApiKeySection.tsx:305` ("Cancel") and `:372` ("Done") shared one key. Added distinct `done` key; component and all 4 locales updated; TS9 test updated.
3. **[Observation, no fix required] i18n runtime absent in `remote-frontend`.** Pre-existing (spec Decision 4; shared by all `swarm/*` components). The edited locale files are never consumed by the component at runtime; only inline fallbacks render. Not a regression from this workstream.
4. **[Minor UX, spec-conformant] Mutation error Alert renders on the Card, behind the open create Dialog** (`:236-244`). Per spec Design ("Alert above the list"), so conformant, but a create error is not visible while the dialog is open.
5. No security issues. Secret handling is one-time, cleared on close; no logging of the secret; clipboard fallback is standard.

## Remediations Applied (this session — no deferral)

1. `frontend/src/i18n/locales/en/settings.json` — changed `settings.swarm.apiKeys.create` from "Create API Key" to "Generate API Key" to match SC1/US1 and the inline fallback.
2. Added `settings.swarm.apiKeys.done` = "Done" key to `en` locale; added empty `done` key to `es/ja/ko` locales.
3. `remote-frontend/src/components/swarm/NodeApiKeySection.tsx:372` — changed `t('...cancel', 'Done')` to `t('...done', 'Done')` for the secret-view close button.
4. `remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx` — added `'settings.swarm.apiKeys.done'` to TS9 requiredKeys list; updated TS4 to reference `done` key instead of `cancel` for the Done button.
5. Verified: `tsc --noEmit` clean, 26/26 tests pass.

## Verdict
**OVERALL: PASS.** At runtime the implementation satisfies SC1–SC7, the plan was followed faithfully (every trap honored, divergences documented and benign), and the green gate is verified (tsc clean, 26/26). The two latent locale defects discovered by the adversarial review were remediated this session. No deferred debt remains.
