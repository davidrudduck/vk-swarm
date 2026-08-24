---
id: "017"
phase: 4
title: "Make browser sign-out and Hive disconnect visibly distinct actions in the UI"
status: ready
depends_on: ["016"]
parallel: false
conflicts_with: []
files:
  - "frontend/src/components/layout/Navbar.tsx"
  - "frontend/src/pages/settings/SwarmSettings.tsx"
  - "frontend/src/components/layout/__tests__/NavbarAuthActions.test.tsx"
siblings: ["frontend/src/components/layout/__tests__/BottomNav.test.tsx"]
irreversible: false
scope_test: "frontend/src/components/layout/__tests__/NavbarAuthActions.test.tsx"
allowed_change: mixed
forbid_after: ["handleOAuthLogout"]
covers_criteria: []
covers_tests: []
---
## Failing test (write first)
File: `frontend/src/components/layout/__tests__/NavbarAuthActions.test.tsx` — create executable tests with existing `fireEvent` (no user-event dependency).

**Navbar harness.** Mock `useUserSystem` as logged in and provide only the fields Navbar reads; mock translations to return defaults/keys; mock project/search hooks, router navigation, and heavy child components. Render `<Navbar />` under `MemoryRouter`. Open the Radix **Main navigation** menu with `fireEvent.click`, then click `navbar-sign-out`. Spy on `browserAuthApi.logout`, `disconnectHive`, and a controllable `window.location.reload`. Assert logout exactly once, disconnect zero, reload exactly once after resolved logout. Add rejection coverage proving reload is zero on failure.

**SwarmSettings harness.** Mock `useAuth`, organization/query hooks, and organization/label/template/swarm section children to minimal inert components. Render `<SwarmSettings />`. Stub `window.confirm` true, click `hive-disconnect`, assert `disconnectHive` exactly once, `logout` zero, and reload once after resolution. Assert `window.confirm` was called with `expect.stringContaining('EVERY browser')` (this kills the drop-EVERY mutant; the STOP trigger makes that copy load-bearing). In a separate test stub confirmation false and assert disconnect/reload both zero.

Use `fireEvent`, `render`, `screen`, `waitFor`, and `MemoryRouter` only. Do not recreate the production provider stack; mocks must be minimal but return every destructured field so failures exercise the handlers rather than missing context.

Locked test fixtures (do not invent others):
- `browserAuthApi` via `vi.hoisted` + `vi.mock('@/lib/api', () => ({ browserAuthApi }))` (or `vi.mock('@/lib/api/browserAuth', ...)` if the component import is the module path — match the production import `@/lib/api`).
- Reload: `const reloadSpy = vi.fn()`; `beforeEach` does `Object.defineProperty(window, 'location', { configurable: true, value: { ...window.location, reload: reloadSpy } })`; `afterEach` restores the original descriptor.
- Navbar mocks: `useUserSystem` → `{ loginStatus: { status: 'loggedin' }, reloadSystem: vi.fn() }`; `useProject` → `{ projectId: 'p1', project: { id: 'p1', name: 'P' } }`; `useSearch` → `{ query: '', setQuery: vi.fn(), active: false, clear: vi.fn(), registerInputRef: vi.fn() }`; `useOpenProjectInEditor` → `vi.fn()`; `useLocation`/`useSearchParams` via `MemoryRouter` (do not mock all of `react-router-dom` — Navbar needs `Link`/`MemoryRouter`). Inert `vi.mock` for `SearchBar`, `ActivityFeed`, `ProjectSwitcher`, `ThemeToggle`, `OpenInIdeButton`, `OAuthDialog`, `VKSLogo`. i18n `t` returns the string default or the key.
- SwarmSettings mocks: `useAuth` → `{ isSignedIn: true, isLoaded: true }`; `useUserOrganizations` → `{ data: { organizations: [{ id: 'o1', name: 'Org' }] }, isLoading: false, error: null }`; `useOrganizationSelection` → `{ selectedOrgId: 'o1', selectedOrg: { id: 'o1', name: 'Org' }, handleOrgSelect: vi.fn() }`. Inert `() => null` mocks for `SwarmProjectsSection`, `NodeProjectsSection`, `SwarmLabelsSection`, `SwarmTemplatesSection`, `NodeTemplatesSection`, `LoginRequiredPrompt`.
- Open the navbar menu via `fireEvent.click(screen.getByRole('button', { name: 'Main navigation' }))` then `fireEvent.click(screen.getByTestId('navbar-sign-out'))`.


## Change
**File:** `frontend/src/components/layout/Navbar.tsx`
**Anchor 1:** `const handleOAuthLogout = async () => { ... }` at L125-132.
**Before:**
```tsx
  const handleOAuthLogout = async () => {
    try {
      await oauthApi.logout();
      await reloadSystem();
    } catch (err) {
      console.error('Error logging out:', err);
    }
  };
```
**After:**
```tsx
  // Sign out THIS BROWSER only. The node stays connected to the hive and other browsers keep
  // their sessions; disconnecting the node is a separate, deliberately harder action in
  // Settings -> Swarm.
  const handleBrowserSignOut = async () => {
    try {
      await browserAuthApi.logout();
      window.location.reload();
    } catch (err) {
      console.error('Failed to sign out of this browser:', err);
    }
  };
```
`window.location.reload()` (rather than `reloadSystem()`) is deliberate: the session cookie is gone, so every protected query must be torn down, and a full reload is the simplest correct teardown.

**Anchor 2:** the dropdown item at L320-323.
**Before:**
```tsx
                    <DropdownMenuItem onSelect={handleOAuthLogout}>
                      <LogOut className="mr-2 h-4 w-4" />
                      {t('common:signOut')}
                    </DropdownMenuItem>
```
**After:**
```tsx
                    <DropdownMenuItem
                      data-testid="navbar-sign-out"
                      onSelect={handleBrowserSignOut}
                    >
                      <LogOut className="mr-2 h-4 w-4" />
                      {t('common:signOut')}
                    </DropdownMenuItem>
```
Also update the import: `oauthApi` is no longer used for logout here.

**File:** `frontend/src/pages/settings/SwarmSettings.tsx`
**Anchor:** the end of the returned tree (L168-173).
**Before:**
```tsx
      {/* Node Templates Section - Promote local templates to swarm */}
      {selectedOrg && <NodeTemplatesSection organizationId={selectedOrg.id} />}
    </div>
  );
}
```
**After:** the same, with a disconnect card inserted before `</div>`:
```tsx
      {/* Node Templates Section - Promote local templates to swarm */}
      {selectedOrg && <NodeTemplatesSection organizationId={selectedOrg.id} />}

      <Card>
        <CardHeader>
          <CardTitle>{t('settings.swarm.disconnectTitle', 'Disconnect this node from Hive')}</CardTitle>
          <CardDescription>
            {t('settings.swarm.disconnectHelper',
              'Signs out EVERY browser, stops synchronisation and removes this node\u2019s Hive \
credentials. The node stays owned by your account, so no one else can claim it. To sign out just \
this browser, use Sign out in the menu.')}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Button variant="destructive" data-testid="hive-disconnect" onClick={handleDisconnect}>
            {t('settings.swarm.disconnectAction', 'Disconnect from Hive')}
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
```
`handleDisconnect` is locked as:
```ts
const handleDisconnect = async () => {
  if (
    !window.confirm(
      t(
        'settings.swarm.disconnectConfirm',
        "This signs out EVERY browser, stops synchronisation, and removes this node's Hive credentials. Continue?"
      )
    )
  ) {
    return;
  }
  try {
    await browserAuthApi.disconnectHive();
    window.location.reload();
  } catch (err) {
    console.error('Failed to disconnect from Hive:', err);
  }
};
```
Import `browserAuthApi` from `@/lib/api` in both files (the 016 barrel export). In Navbar replace `import { oauthApi } from '@/lib/api'` with `import { browserAuthApi } from '@/lib/api'` — `oauthApi` has no other use in Navbar. In SwarmSettings add `import { Button } from '@/components/ui/button'` (not currently imported) and `import { browserAuthApi } from '@/lib/api'`.

**Sibling alignment (rubric 9).** Copy the i18n idiom already used in THIS file — `t('settings.swarm.selectOrgHelper', 'Select the organization ...')`, an inline English default — so no locale JSON file (outside this task's `files:`) needs to change and `scripts/check-i18n.sh` stays happy. Use the `Card`/`CardHeader`/`CardTitle`/`CardDescription`/`CardContent` imports already at the top of the file; add only `Button` if it is not yet imported.

**Symbol grounding:** This task introduces `handleBrowserSignOut()` in `Navbar.tsx` (replacing the existing `handleOAuthLogout()`) and `handleDisconnect()` in `SwarmSettings.tsx`. It calls `browserAuthApi.logout()` and `browserAuthApi.disconnectHive()`, both defined by task 016. `oauthApi.logout()` remains defined in `frontend/src/lib/api/oauth.ts` but must have no call site left after this task (enforced by `forbid_after`).

**Executable test requirement.** Replace the prior comment-stub body with the two concrete lightweight harnesses above. The test file covers Navbar success/error and SwarmSettings confirm/cancel. Radix interaction must open the visible “Main navigation” menu before selecting `navbar-sign-out`; direct handler invocation is not accepted. Reload is controlled and asserted, not allowed to navigate jsdom.



## Allowed moves
[
  "Rename handleOAuthLogout to handleBrowserSignOut, repoint it at browserAuthApi.logout, add the data-testid, and fix the import line.",
  "Append one disconnect Card plus its handler to SwarmSettings.tsx.",
  "Create the test file.",
  "Do not change any other navbar item, any other settings section, or oauthApi itself."
]


## STOP triggers
[
  "The navbar sign-out reaching /api/auth/logout (the disconnect endpoint) by any path — that is the exact confusion D5 exists to prevent.",
  "The disconnect action lacking a confirmation, or its copy failing to say that it affects every browser.",
  "Adding keys to frontend/src/i18n/locales/** — not in files:; use the inline-default t() idiom already in SwarmSettings.tsx.",
  "`git grep -F 'oauthApi.logout' -- frontend/src` still matching after the change — the node-frontend call site must be gone. Historical reports, this task's Before snippet, plan.md, and remote-frontend (hive) callers are out of scope and must remain. Frontmatter forbid_after is `handleOAuthLogout` (the renamed Navbar symbol): the gate scans the whole commit tree excluding only docs/plans/<topic>/, so `oauthApi.logout` can never pass while remote-frontend and tournament notes exist.",
  "Deleting `logout` from frontend/src/lib/api/oauth.ts — removing a shipped API namespace member is a contract change outside this plan's irreversible budget; leave the definition, remove only the call site.",
  "Leaving render/user interaction as comments or calling handlers directly.",
  "Mounting the full application provider/dependency stack instead of minimal useUserSystem/router/project/search/translation/heavy-child mocks.",
  "Using userEvent or adding a testing dependency; existing fireEvent is sufficient.",
  "Omitting confirmation-false coverage or leaving window.location.reload uncontrolled."
]


Declared decision points (from the spec; do not edit here):
- DP1: Execution reaches the additive SQLite migration task and requires explicit approval before applying an irreversible production schema migration.  [codes: human_gate_required]


## Manual verification (record in decisions-ledger)
1. `cd frontend && npx vitest run src/components/layout/__tests__/NavbarAuthActions.test.tsx` green.
2. `WAI_ROOT="$HOME/.agents/wai"; test -x "$WAI_ROOT/scripts/task-gate.sh"; WAI_TYPECHECK_CMD="cd frontend && npx tsc --noEmit" WAI_TEST_CMD='(s={scope}; cd frontend && npx vitest run "${s#frontend/}")' bash "$WAI_ROOT/scripts/task-gate.sh" local-node-browser-oauth 017` exits 0.
3. `cd frontend && npm run lint && npx tsc --noEmit && npx vitest run` green; `bash scripts/check-i18n.sh` green.
4. Manual on a running node with two browsers: Sign out in browser A returns A to the login shell while B stays signed in; Settings -> Swarm -> Disconnect from Hive returns BOTH to the login shell.


## Done when
`WAI_TYPECHECK_CMD="cd <dir> && <typecheck>" WAI_TEST_CMD="cd <dir> && <test>" bash "$HOME/.agents/wai/scripts/task-gate.sh" local-node-browser-oauth 017` exits 0
