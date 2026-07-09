---
doc_type: spec
status: shipped
workstream: hive-node-api-key-ui
change_kind: bugfix
---

# hive-node-api-key-ui — API key management UI on the Hive Nodes page

> **Origin:** Promoted from finding `F-2026-07-06-01` via `/wai:finding-promote`.
> The finding (high severity, discovered 2026-07-06, source `session/2026-07-06`)
> recorded that the Hive UI lacks a "Generate API key" button, blocking node
> onboarding. This spec captures the intent; `/wai:spec` will add the design.

## Intent (what / why)

Node onboarding requires an API key (`VK_NODE_API_KEY`) that the node presents to the
hive server to authenticate. Today the Hive UI (`remote-frontend/src/pages/Nodes.tsx:7-51`)
only lists connected nodes — there is no way to **generate, view, revoke, or unblock** an
API key from the hive console.

The gap is purely in the UI layer:

- **Backend:** the `/v1/nodes/api-keys` endpoints already exist and work (create, list,
  revoke, unblock) — served by `crates/remote`.
- **API client:** `remote-frontend/src/lib/api/nodes.ts:56-104` provides
  `listApiKeys`, `createApiKey`, `revokeApiKey`, `unblockApiKey` against `/v1/nodes/api-keys`.
- **Types:** `remote-frontend/src/types/nodes.ts:46-75` already defines `NodeApiKey`,
  `CreateNodeApiKeyRequest`, `CreateNodeApiKeyResponse` (with the `secret` one-time field).
- **Reference impl:** the main `frontend/` has a complete `NodeApiKeySection`
  (`frontend/src/components/org/NodeApiKeySection.tsx`) with full CRUD — but it lives in
  org settings and is not ported to the hive.

So every layer below the page is ready; only the `Nodes.tsx` page needs an API key
management section surfaced. Without it, an operator cannot onboard a new node from the
hive console at all.

## Users / who is affected

- **Operator (browser, hive console):** the human who connects nodes to the swarm. After
  this workstream they can generate an API key on the Nodes page, copy the one-time secret,
  and hand it to the node operator. They can also see which keys exist, which node each is
  bound to, and revoke/unblock keys.
- **Node operator (CLI):** the human who runs `vk-swarm-node` with `VK_NODE_API_KEY`. They
  consume the secret the hive operator generated; they are not directly a hive UI user.
- **Existing node-frontend users:** unaffected. The main `frontend/` keeps its own
  `NodeApiKeySection` in org settings; this workstream does not touch `frontend/`.

## User stories

- **US1:** As an operator, when I open the Nodes page in the hive console, I expect to see a
  "Generate API Key" button so I can create a key for onboarding a new node.
- **US2:** As an operator, when I create an API key, I expect the secret to be shown **once**
  with a copy button (and a show/hide toggle), so I can hand it to the node operator before
  the dialog closes and the secret is gone.
- **US3:** As an operator, when I view the Nodes page, I expect to see a list of active API
  keys — each showing name, key prefix, bound/unbound status, and created/last-used
  timestamps — so I can manage them.
- **US4:** As an operator, when I revoke an API key, I expect a confirmation dialog and the
  key to be removed from the active list, so nodes using it can no longer connect.
- **US5:** As an operator, when an API key is blocked (duplicate-key-use detection), I expect
  to see a "Blocked" badge with the reason and an "Unblock" action that restores the key.

## Success criteria

- **SC1:** The Nodes page (`remote-frontend/src/pages/Nodes.tsx`) renders a "Generate API
  Key" button and an API key management section alongside (or below) the existing node list,
  gated only on `orgId` being present (the existing org-load guard). → US1
- **SC2:** Clicking "Generate API Key" opens a `Dialog`; submitting a name calls
  `nodesApi.createApiKey({ organization_id, name })` and the dialog transitions to a
  one-time-secret view showing `response.secret` with a show/hide toggle and a copy button
  (using `navigator.clipboard` with the non-HTTPS `document.execCommand('copy')` fallback
  the frontend reference uses). Closing the dialog clears the secret from state. → US2
- **SC3:** Active (non-revoked) API keys are listed via `nodesApi.listApiKeys(orgId)` with
  each item showing: name, `key_prefix`, a "Bound"/"Unbound" badge (from `node_id`), created
  timestamp, and last-used timestamp (when present) — matching the `NodeApiKey` shape in
  `remote-frontend/src/types/nodes.ts:46-65`. → US3
- **SC4:** Revoking a key calls `nodesApi.revokeApiKey(keyId)` only after a `confirm()`
  dialog; on success the query `['nodeApiKeys', orgId]` is invalidated so the list updates
  reactively (the existing TanStack Query pattern used by `NodeApiKeySection`). → US4
- **SC5:** A blocked key (`blocked_at` set) renders a "Blocked" badge with `blocked_reason`;
  an "Unblock" button calls `nodesApi.unblockApiKey(keyId)` after `confirm()` and the badge
  updates to "Active" via query invalidation. → US5
- **SC6:** All user-facing strings go through `useTranslation(['settings', 'common'])` with
  keys namespaced under `settings.swarm.apiKeys.*` and English fallback strings, matching
  the pattern used by the existing `remote-frontend/src/components/swarm/*` components. The
  `en` locale file (`frontend/src/i18n/locales/en/settings.json`) gains the new
  keys; `es`/`ja`/`ko` locales are updated for parity. → US1, US2, US3, US4, US5
- **SC7:** Vitest tests cover the create/list/revoke/unblock flows and all loading,
  error, and empty states, building on the patterns at `remote-frontend/src/pages/Nodes.test.tsx`
  patterns (mocking `nodesApi` and `useOrganizations`, asserting on rendered output).
  → US1, US2, US3, US4, US5

## Constraints

- **Backend is done — no Rust changes.** The `/v1/nodes/api-keys` routes already exist and
  work. This workstream is `remote-frontend/`-only.
- **Reuse the API client and types.** The client functions
  (`nodesApi.listApiKeys`, `nodesApi.createApiKey`, `nodesApi.revokeApiKey`,
  `nodesApi.unblockApiKey`) live at `remote-frontend/src/lib/api/nodes.ts:56-104`;
  the `NodeApiKey` and `CreateNodeApiKey*` type definitions live at
  `remote-frontend/src/types/nodes.ts:46-75`. Do not duplicate them.
- **UI lives on the Nodes page.** No new Settings page or route (explicit scope decision).
  A new `NodeApiKeySection` component lands at
  `remote-frontend/src/components/swarm/NodeApiKeySection.tsx` (matching the
  `SwarmHealthSection` / `NodeProjectsSection` pattern) and is composed into
  `Nodes.tsx` above the node grid. See Design.
- **Follow the frontend reference for behavior.** `frontend/src/components/org/
  NodeApiKeySection.tsx` is the behavioral reference (create-once secret, confirm dialogs,
  query invalidation, unblock flow). Adapt it to the Nodes-page context; do not copy it
  verbatim where the hive's conventions differ.
- **Existing UI primitives available.** `remote-frontend/src/components/ui/` already has
  `dialog`, `card`, `button`, `input`, `label`, `alert`, `badge`, `tooltip` — use these,
  do not add new primitive libraries.
- **i18n pattern.** Remote-frontend Swarm components use `useTranslation(['settings',
  'common'])` + `t(key, fallback)`. Locale JSON files live in
  `frontend/src/i18n/locales/{en,es,ja,ko}/settings.json` under the `settings.swarm.*`
  namespace (the `settings.swarm.{projects,labels,templates,nodeProjects,nodeTemplates}`
  siblings already exist there). New keys go under `settings.swarm.apiKeys.*`.
- **No admin-role gating this pass.** The hive UI has no admin/role distinction yet; show
  all controls to any org member (the `orgId` guard from `useOrganizations` is the only
  gate). The frontend's `isAdmin` prop is **not** ported.
- **Bearer token auth via `localStorage`.** Established by `vk-swarm-hive-ui` tasks 101-102;
  `makeRequest` already sends the `Authorization: Bearer` header. No auth changes.
- **Routes nest under `/v1`.** The API client already uses `/v1/nodes/api-keys` — no change.
- **GitHub targeting:** PRs only against `davidrudduck/vk-swarm`.

## Out of scope

- **No new Settings page/route in `remote-frontend/`.** All API key management is surfaced
  on the existing Nodes page (explicit scope decision).
- **No backend / Rust changes.** The `/v1/nodes/api-keys` endpoints already work; this is a
  frontend-only workstream.
- **No changes to the main `frontend/`.** The `frontend/src/components/org/
  NodeApiKeySection.tsx` already has full CRUD in org settings and is not modified or
  backported — it is a behavioral reference only.
- **No admin-role visibility gating.** The hive UI has no role model yet; all org members
  see all controls. The `isAdmin` gating from the frontend reference is not ported.
- **No design-system (`.vks-*`) restyle.** This workstream uses the existing shadcn/ui
  primitives already in `remote-frontend/src/components/ui/`. The Midnight Terminal design
  system (`vk-swarm-design-system` workstream) is a separate, orthogonal effort.
- **No node-binding UX.** A key binds to a node on first connection (`node_id` is set
  server-side); this workstream only **displays** the bound/unbound state, it does not let
  the operator manually bind/unbind a key to a specific node.

## Approach

Three phases, sequentially dependent (each phase ends with a green test suite and a clean
`cd remote-frontend && npx tsc --noEmit`):

1. **Component + integration (foundation):** create
   `remote-frontend/src/components/swarm/NodeApiKeySection.tsx` and export it from
   `remote-frontend/src/components/swarm/index.ts`. Compose it into `Nodes.tsx` above the
   existing node grid. Wire `useQuery`/`useMutation` to the existing
   `nodesApi.{listApiKeys,createApiKey,revokeApiKey,unblockApiKey}`. Port the create dialog
   (with the one-time-secret reveal + clipboard copy + non-HTTPS fallback) and the
   `ApiKeyItem` list from the frontend reference (`frontend/src/components/org/
   NodeApiKeySection.tsx`), dropping the `isAdmin` prop. Cover with `NodeApiKeySection.test.tsx`
   and extend `Nodes.test.tsx`.

2. **i18n (localization):** add the `settings.swarm.apiKeys.*` namespace to all four locale
   files (`frontend/src/i18n/locales/{en,es,ja,ko}/settings.json`). Use English fallbacks
   inline via the existing `t(key, fallback)` pattern, so the UI works regardless of whether
   the locale JSON is present. Cover with a snapshot of the en locale confirming every
   `t()` call in `NodeApiKeySection.tsx` has a matching key.

3. **Verification:** run `cd remote-frontend && npx tsc --noEmit` and
   `cd remote-frontend && npx vitest run` to confirm the workstream is green. Then run the
   `wai-precheck` and `wai-decompose` gates per the WAI pipeline.

## Design / architecture

### Component tree (on the Nodes page)

```
Nodes.tsx                              (existing page, modified)
├── NodeApiKeySection organizationId   (NEW, above the node grid)
│   ├── Card > CardHeader              (title + "Generate API Key" button)
│   ├── CardContent
│   │   ├── Alert (error, conditional)
│   │   ├── Loader2 (loading, conditional)
│   │   ├── empty-state copy           ("No API keys found. Create one...")
│   │   └── ApiKeyItem[]               (active keys, one per row)
│   └── Dialog                         (create flow)
│       ├── name input + Create/Cancel
│       └── secret reveal + Copy + show/hide toggle
└── NodeCard[]                         (existing, unchanged)
```

`ApiKeyItem` is an internal subcomponent within the same file (mirroring the frontend
reference's inline `ApiKeyItem`). It renders name, `key_prefix`, bound/unbound badge,
created/last-used timestamps, and an action button (revoke when active, unblock when
blocked, no action when revoked).

### Data flow

- `useQuery(['nodeApiKeys', orgId], () => nodesApi.listApiKeys(orgId), { enabled: !!orgId })`
  — list, invalidated on every create/revoke/unblock mutation.
- `useMutation((name) => nodesApi.createApiKey({ organization_id: orgId, name }))` — on
  success, store `response.secret` in component state, transition the dialog to the
  reveal view, invalidate the list query.
- `useMutation((keyId) => nodesApi.revokeApiKey(keyId))` — invoked after `confirm()`;
  invalidates the list query.
- `useMutation((keyId) => nodesApi.unblockApiKey(keyId))` — invoked after `confirm()`;
  invalidates the list query.
- Errors from any mutation are caught in `onError` and stored in an `error` state shown
  via an `Alert variant="destructive"` above the list (same pattern the frontend uses).

### i18n key surface (proposed `settings.swarm.apiKeys.*`)

```
settings.swarm.apiKeys.title                    "Node API Keys"
settings.swarm.apiKeys.description              "API keys allow nodes to authenticate with the hive server"
settings.swarm.apiKeys.create                   "Create API Key"
settings.swarm.apiKeys.createTitle              "Create Node API Key"
settings.swarm.apiKeys.createDescription        "Give your API key a name..."
settings.swarm.apiKeys.createdTitle             "API Key Created"
settings.swarm.apiKeys.copySecret               "Copy this secret now. You won't be able to see it again."
settings.swarm.apiKeys.copyToClipboard          "Copy to Clipboard"
settings.swarm.apiKeys.copied                   "Copied!"
settings.swarm.apiKeys.envVarHint               "Use this key as the VK_NODE_API_KEY environment variable..."
settings.swarm.apiKeys.nameLabel                "Key Name"
settings.swarm.apiKeys.namePlaceholder          "e.g., MacBook Pro, Build Server"
settings.swarm.apiKeys.cancel                   "Cancel"
settings.swarm.apiKeys.createAction             "Create"
settings.swarm.apiKeys.loading                  "Loading API keys..."
settings.swarm.apiKeys.empty                    "No API keys found. Create one to allow nodes to connect."
settings.swarm.apiKeys.bound                    "Bound"
settings.swarm.apiKeys.boundTooltip             "Bound to node: {{id}}"
settings.swarm.apiKeys.unbound                  "Unbound"
settings.swarm.apiKeys.created                  "Created {{when}}"
settings.swarm.apiKeys.lastUsed                 "Last used {{when}}"
settings.swarm.apiKeys.active                   "Active"
settings.swarm.apiKeys.revoked                  "Revoked"
settings.swarm.apiKeys.blocked                  "Blocked"
settings.swarm.apiKeys.blockedTooltip           "{{reason}}"
settings.swarm.apiKeys.revoke                   "Revoke"
settings.swarm.apiKeys.revokeConfirm            "Are you sure you want to revoke this API key?..."
settings.swarm.apiKeys.unblock                  "Unblock"
settings.swarm.apiKeys.unblockConfirm           "Are you sure you want to unblock this API key?..."
settings.swarm.apiKeys.error                    "Failed: {{message}}"
```

All four locales (`en`, `es`, `ja`, `ko`) are extended with this set. The `en` set is the
canonical source; the other three carry parity translations.

## Decisions

1. **New `NodeApiKeySection` component in `components/swarm/`, not inlined into
   `Nodes.tsx`.** Rationale: matches the established `SwarmHealthSection` /
   `NodeProjectsSection` / `SwarmProjectsSection` pattern, keeps `Nodes.tsx` as a thin
   composition layer, and is independently testable. The `useOrganizations` → `orgId` guard
   stays in `Nodes.tsx`; the section is gated on receiving a non-empty `organizationId`.

2. **Render the section ABOVE the node grid.** Rationale: the onboarding flow is
   generate-key → give secret to node operator → node connects → appears in list. Surfacing
   the key-management section first puts the onboarding entry point at the top of the
   page. (A future redesign can move it into a side card without changing the component
   contract.)

3. **Port the frontend `NodeApiKeySection` behaviorally, not verbatim.** Rationale: the
   hive uses a different API client (`/v1/` prefix, `useQuery`/`useMutation` directly
   against `nodesApi`), different i18n namespace (`settings.swarm.*` instead of hardcoded
   strings), and no `isAdmin` prop. Copy the dialog flow and `ApiKeyItem` structure, but
   inline the keys/strings/hooks.

4. **i18n keys go in the frontend's locale files, not a new remote-frontend locale set.**
   Rationale: the remote-frontend's `useTranslation` calls read from the same namespace
   structure that already lives in `frontend/src/i18n/locales/`. Establishing a parallel
   locale set in the remote-frontend would duplicate the i18n config and the maintenance
   burden. The existing pattern is: define keys in frontend, use them with `t(key,
   fallback)` from remote-frontend. (This is a pre-existing architectural decision of the
   remote-frontend — not introduced by this workstream.)

5. **No admin-role gating in this pass.** Rationale: the hive UI has no role model. Adding
   one is out of scope; all org members see all controls. When a role model lands, the
   `isAdmin` prop can be added without a component-contract break (it's a new optional
   prop).

6. **Use `confirm()` for revoke/unblock, not the shadcn `AlertDialog`.** Rationale: the
   frontend reference uses `confirm()`; consistency is more valuable than visual polish
   for a destructive action. (The `AlertDialog` primitive is available and could be
   adopted later.)

7. **`useQuery`/`useMutation` directly in the component, not extracted into a
   `useNodeApiKeys` hook.** Rationale: the section is the only consumer; extracting a hook
   is premature. If a second consumer appears, the hook can be extracted without
   changing the section's contract.

Reversibility check: all seven decisions above are additive and reversible. None touch
contracts, public APIs, dependencies, or wire formats; all can be walked back by adjusting
the component, its placement, or the i18n keys. **No ADR required for this workstream.**

## Test strategy

- **TS1:** `NodeApiKeySection` renders without throwing when `organizationId` is set and
  the `nodeApiKeys` query is in a loading state — matches the `SwarmHealthSection`
  "renders without throwing" pattern in `components/swarm/index.test.tsx`. → SC1, SC7
- **TS2:** `NodeApiKeySection` renders the empty-state copy when the query returns an
  empty array — covers the "no keys yet" path before the first create. → SC3, SC7
- **TS3:** `NodeApiKeySection` renders one `ApiKeyItem` per active key returned by the
  query, each showing name, `key_prefix`, and a badge reflecting `node_id` bound/unbound
  state. → SC3, SC7
- **TS4:** Clicking the "Generate API Key" button opens the create `Dialog`; submitting a
  name calls `nodesApi.createApiKey` with `{ organization_id, name }` and the dialog
  transitions to the one-time-secret reveal view showing `response.secret` (mocked). The
  show/hide toggle flips the visibility; the Copy button calls the clipboard fallback
  when `navigator.clipboard` is unavailable. Closing the dialog clears the secret from
  state. → SC2, SC7
- **TS5:** Clicking Revoke on an active `ApiKeyItem` fires `window.confirm`; confirming
  calls `nodesApi.revokeApiKey(keyId)` and the query is invalidated (verified by
  `queryClient.invalidateQueries` mock). Declining the confirm aborts the mutation. → SC4, SC7
- **TS6:** A blocked `ApiKeyItem` (mock with `blocked_at` set) renders the "Blocked" badge
  with `blocked_reason`; clicking Unblock fires `window.confirm`; confirming calls
  `nodesApi.unblockApiKey(keyId)` and the query is invalidated. → SC5, SC7
- **TS7:** A mutation `onError` (e.g., `nodesApi.createApiKey` rejected) surfaces the
  error message in the destructive `Alert` above the list and the list itself does not
  refetch. Covers the error-state path for all three mutations. → SC2, SC4, SC5, SC7
- **TS8:** `Nodes.tsx` renders `<NodeApiKeySection />` when `orgId` is set and omits it
  when `orgId` is undefined (the "no organization" path) — extends the existing
  `Nodes.test.tsx` org-loading cases. → SC1, SC7
- **TS9:** Every `t(key, fallback)` call in `NodeApiKeySection.tsx` has a matching key in
  `frontend/src/i18n/locales/en/settings.json` under `settings.swarm.apiKeys.*` — a
  snapshot-style test that fails if a `t()` call references a key that does not exist in
  the en locale. → SC6, SC7

