---
id: "004"
phase: 1
title: Add error-state Alert + TS7 test
status: ready
depends_on: ["002"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/components/swarm/NodeApiKeySection.tsx
  - remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx
irreversible: false
scope_test: "remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx"
allowed_change: edit
covers_criteria: [SC2, SC4, SC5, SC7]
covers_tests: [TS7]
---

## Failing test (write first)

Append the following test to `remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx`. It must FAIL before the error-state Alert is wired (no `error` state, no `Alert` rendered on mutation rejection). It must PASS after the Change below lands.

```ts
it('surfaces a destructive Alert when a mutation rejects; the list does not refetch (TS7)', async () => {
  vi.mocked(nodesApi.listApiKeys).mockResolvedValue([]);
  vi.mocked(nodesApi.createApiKey).mockRejectedValue(new Error('boom'));
  renderWith(<NodeApiKeySection organizationId="org-1" />);
  const { default: user } = await import('@testing-library/user-event');
  const u = user.setup();
  const listSpy = vi.mocked(nodesApi.listApiKeys);
  const callsBefore = listSpy.mock.calls.length;
  await u.click(screen.getByRole('button', { name: 'settings.swarm.apiKeys.create' }));
  const nameInput = await screen.findByLabelText('settings.swarm.apiKeys.nameLabel');
  await u.type(nameInput, 'X');
  await u.click(screen.getByRole('button', { name: 'settings.swarm.apiKeys.createAction' }));
  await waitFor(() => {
    // The Alert reads "Failed: {{message}}"; the key returns 'boom'.
    expect(screen.getByText('settings.swarm.apiKeys.error')).toBeInTheDocument();
    expect(screen.getByText(/boom/)).toBeInTheDocument();
  });
  // No new list fetch on mutation failure (the same `useQuery` is preserved)
  expect(listSpy.mock.calls.length).toBe(callsBefore);
});
```

## Change

- **File:** `remote-frontend/src/components/swarm/NodeApiKeySection.tsx`
- **Anchor:**
  - Imports block (add `Alert`, `AlertDescription`)
  - State hooks (add `const [error, setError] = useState<string | null>(null);`)
  - `useMutation` calls (add `onError` to `createMutation`, `revokeMutation`, `unblockMutation`)
  - JSX (render `<Alert variant="destructive">` above the list when `error` is set)
- **Before:** (the `createMutation` from task 002 with no `onError`)
  ```tsx
  const createMutation = useMutation({
    mutationFn: (name: string) => nodesApi.createApiKey({ organization_id: organizationId, name }),
    onSuccess: (response) => {
      setCreatedSecret(response.secret);
      setNewKeyName('');
      queryClient.invalidateQueries({ queryKey: ['nodeApiKeys', organizationId] });
    },
  });
  ```
- **After:**
  1. Add `import { Alert, AlertDescription } from '@/components/ui/alert';` (the `NodeProjectsSection` already imports this — same path).
  2. Add `const [error, setError] = useState<string | null>(null);` alongside the other state hooks.
  3. Add `onError` to all three mutations:
     ```tsx
     onError: (err) => {
       setError(err instanceof Error ? err.message : 'Failed');
     },
     ```
     The `setError` call captures the error string; the Alert reads it via the i18n key. Add a `setError(null)` at the start of each `onSuccess` to clear the previous error.
  4. Above the `<CardContent>` body, render:
     ```tsx
     {error && (
       <div className="px-6 pb-4">
         <Alert variant="destructive">
           <AlertDescription>
             {t('settings.swarm.apiKeys.error', 'Failed: {{message}}', { message: error })}
           </AlertDescription>
         </Alert>
       </div>
     )}
     ```
     Match the `NodeProjectsSection` precedent (`<div className="px-4 pb-4 sm:px-6">`); either is fine — the test only checks the text content, not the wrapper class.

- **File:** `remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx`
- **Anchor:** append the new `it(...)` block from "Failing test (write first" above to the existing `describe('NodeApiKeySection', ...)` block.

## Allowed moves

- Edit `NodeApiKeySection.tsx` per the Change above. No other files in this task.
- Edit `NodeApiKeySection.test.tsx` to add the TS7 test. No other test additions.
- Do NOT add a global error boundary — the section-local `error` state is sufficient.
- Do NOT add the error-state to `useQuery` (the spec says only the mutation `onError` path surfaces errors; the list itself is presumed to load successfully when the section renders, per SC3).
- Do NOT change the locale JSON files — that is task 007.
- Do NOT touch the `index.ts` barrel — that is task 005.

## STOP triggers

- The `Alert` component import path is wrong (must be `@/components/ui/alert` — already used by `NodeProjectsSection`).
- The `t('settings.swarm.apiKeys.error', 'Failed: {{message}}', { message: error })` interpolation is misspelled — the i18n key has `{{message}}` literally; the interpolation must pass `message: error`.
- The test passes by accident (e.g. the Alert is rendered even when there's no error because the `error` state is initialized to a string) — `useState<string | null>(null)` is the only correct initial value.
- `setError(null)` in `onSuccess` is missing — without it, a previously-shown error persists after a successful retry, contradicting SC2's "the list updates" intent.

## Done when

`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/components/swarm/NodeApiKeySection.test.tsx" bash /data/.cache/opencode/packages/agent-plugins@git+https:/github.com/ExpansionX/agent-plugins.git/node_modules/agent-plugins/plugins/wai/scripts/task-gate.sh hive-node-api-key-ui 004` exits 0.
