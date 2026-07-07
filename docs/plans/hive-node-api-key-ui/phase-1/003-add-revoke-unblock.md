---
id: "003"
phase: 1
title: Add revoke + unblock mutations with confirm() + query invalidation + TS5/TS6 tests
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
covers_criteria: [SC4, SC5, SC7]
covers_tests: [TS5, TS6]
---

## Failing test (write first)

Append the following two tests to `remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx`. Both must FAIL before the revoke/unblock mutations and `ApiKeyItem` action button wiring are added (the current `ApiKeyItem` button is a no-op). Both must PASS after the Change below lands.

```ts
it('revokes a key only after window.confirm; query is invalidated on success (TS5)', async () => {
  const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
  const invalidateSpy = vi.fn();
  // Wrap the QueryClient so we can observe invalidateQueries calls
  const localQueryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  localQueryClient.invalidateQueries = invalidateSpy;
  const keys: NodeApiKey[] = [{
    id: 'k1', organization_id: 'org-1', name: 'MacBook', key_prefix: 'vk_abc',
    created_by: null, last_used_at: null, revoked_at: null, created_at: '2026-01-01T00:00:00Z',
    node_id: 'n1', takeover_count: 0, takeover_window_start: null,
    blocked_at: null, blocked_reason: null,
  }];
  vi.mocked(nodesApi.listApiKeys).mockResolvedValue(keys);
  vi.mocked(nodesApi.revokeApiKey).mockResolvedValue();

  render(
    <QueryClientProvider client={localQueryClient}>
      <TooltipProvider><NodeApiKeySection organizationId="org-1" /></TooltipProvider>
    </QueryClientProvider>
  );
  const { default: user } = await import('@testing-library/user-event');
  const u = user.setup();
  const revokeBtn = await screen.findByRole('button', { name: 'settings.swarm.apiKeys.revoke' });
  await u.click(revokeBtn);
  expect(confirmSpy).toHaveBeenCalled();
  await waitFor(() => {
    expect(nodesApi.revokeApiKey).toHaveBeenCalledWith('k1');
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['nodeApiKeys', 'org-1'] });
  });

  // Declining the confirm aborts the mutation
  confirmSpy.mockReturnValue(false);
  vi.mocked(nodesApi.revokeApiKey).mockClear();
  await u.click(revokeBtn);
  expect(nodesApi.revokeApiKey).not.toHaveBeenCalled();
  confirmSpy.mockRestore();
});

it('renders Blocked badge with reason; Unblock calls confirm + unblockApiKey + invalidates (TS6)', async () => {
  const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
  const invalidateSpy = vi.fn();
  const localQueryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  localQueryClient.invalidateQueries = invalidateSpy;
  const keys: NodeApiKey[] = [{
    id: 'k2', organization_id: 'org-1', name: 'Compromised', key_prefix: 'vk_xyz',
    created_by: null, last_used_at: null, revoked_at: null, created_at: '2026-01-01T00:00:00Z',
    node_id: 'n1', takeover_count: 5, takeover_window_start: '2026-01-01T00:00:00Z',
    blocked_at: '2026-01-02T00:00:00Z', blocked_reason: 'Duplicate key use detected',
  }];
  vi.mocked(nodesApi.listApiKeys).mockResolvedValue(keys);
  vi.mocked(nodesApi.unblockApiKey).mockResolvedValue(keys[0]);

  render(
    <QueryClientProvider client={localQueryClient}>
      <TooltipProvider><NodeApiKeySection organizationId="org-1" /></TooltipProvider>
    </QueryClientProvider>
  );
  expect(await screen.findByText('settings.swarm.apiKeys.blocked')).toBeInTheDocument();
  expect(screen.getByText('Duplicate key use detected')).toBeInTheDocument();
  const { default: user } = await import('@testing-library/user-event');
  const u = user.setup();
  await u.click(screen.getByRole('button', { name: 'settings.swarm.apiKeys.unblock' }));
  await waitFor(() => {
    expect(nodesApi.unblockApiKey).toHaveBeenCalledWith('k2');
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ['nodeApiKeys', 'org-1'] });
  });
  confirmSpy.mockRestore();
});
```

## Change

- **File:** `remote-frontend/src/components/swarm/NodeApiKeySection.tsx`
- **Anchor:**
  - `ApiKeyItem` sub-component (currently renders a no-op `<Button>` labeled "Revoke")
  - `NodeApiKeySection` body (add two `useMutation` hooks for revoke and unblock)
- **Before:** (the `ApiKeyItem` action button as left by task 001)
  ```tsx
  <Button
    variant="ghost"
    size="sm"
    onClick={() => {/* task 003 wires this */}}
    className="text-destructive hover:text-destructive"
  >
    {t('settings.swarm.apiKeys.revoke', 'Revoke')}
  </Button>
  ```
- **After:**
  1. Inside `NodeApiKeySection` (after the `createMutation` from task 002), add two mutations that mirror the reference impl's structure:
     ```tsx
     const revokeMutation = useMutation({
       mutationFn: (keyId: string) => nodesApi.revokeApiKey(keyId),
       onSuccess: () => queryClient.invalidateQueries({ queryKey: ['nodeApiKeys', organizationId] }),
     });
     const unblockMutation = useMutation({
       mutationFn: (keyId: string) => nodesApi.unblockApiKey(keyId),
       onSuccess: () => queryClient.invalidateQueries({ queryKey: ['nodeApiKeys', organizationId] }),
     });
     ```
  2. Define handlers (also inside `NodeApiKeySection`):
     ```tsx
     const handleRevoke = (keyId: string) => {
       if (!confirm(t('settings.swarm.apiKeys.revokeConfirm', 'Are you sure you want to revoke this API key? Nodes using it will no longer be able to connect.'))) return;
       revokeMutation.mutate(keyId);
     };
     const handleUnblock = (keyId: string) => {
       if (!confirm(t('settings.swarm.apiKeys.unblockConfirm', 'Are you sure you want to unblock this API key? The node will be able to connect again.'))) return;
       unblockMutation.mutate(keyId);
     };
     ```
  3. Update the `ApiKeyItem` sub-component signature to accept `onRevoke` and `onUnblock` callbacks (typed as `(keyId: string) => void`). Inside `ApiKeyItem`:
     - Render the existing "Revoke" `<Button>` with `onClick={() => onRevoke(key.id)}` when the key is active (`blocked_at` is null AND `revoked_at` is null).
     - When the key has `blocked_at` set: render a `<Badge variant="destructive">` (or the closest destructive-style primitive in `@/components/ui/`) with text `t('settings.swarm.apiKeys.blocked', 'Blocked')` and `blocked_reason` next to it (use a `<Tooltip>` if available), and an "Unblock" `<Button onClick={() => onUnblock(key.id)}>`.
     - When the key has `revoked_at` set: render a `<Badge variant="secondary">{t('settings.swarm.apiKeys.revoked', 'Revoked')}</Badge>` and no action button.
  4. In the `NodeApiKeySection` JSX where `ApiKeyItem` is mapped:
     ```tsx
     {apiKeys.map((key) => (
       <ApiKeyItem
         key={key.id}
         apiKey={key}
         onRevoke={handleRevoke}
         onUnblock={handleUnblock}
       />
     ))}
     ```
  5. Add imports: `Trash2`, `Unlock`, `AlertTriangle` from `lucide-react`; `Tooltip`, `TooltipContent`, `TooltipTrigger` from `@/components/ui/tooltip` (the section is already wrapped in a `TooltipProvider` in the test harness per `index.test.tsx` precedent; the production mount in `Nodes.tsx` will need the same provider — task 006 handles that).

- **File:** `remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx`
- **Anchor:** append the two new `it(...)` blocks from "Failing test (write first" above to the existing `describe('NodeApiKeySection', ...)` block.

## Allowed moves

- Edit `NodeApiKeySection.tsx` per the Change above. No other files in this task.
- Edit `NodeApiKeySection.test.tsx` to add the TS5 and TS6 tests. No other test additions.
- Do NOT add the error-state Alert — that is task 004.
- Do NOT add the `confirm()` to the create flow (the reference impl does not confirm before create) — that is intentionally absent.
- Do NOT change the locale JSON files — that is task 007.
- Do NOT touch the `index.ts` barrel — that is task 005.

## STOP triggers

- The `window.confirm` mock leaks between tests — restore with `confirmSpy.mockRestore()` in every test that uses it.
- The `ApiKeyItem` component is no longer a sub-component of the same file (do not extract it to a separate file — keep the same-file pattern from task 001).
- The `Blocked` badge uses a non-existent variant — match the existing `Badge` variants in `@/components/ui/badge.tsx` (typically `default` / `secondary` / `destructive` / `outline`).
- The `invalidateQueries` spy does not fire because `queryClient` is destructured fresh per render — the test must wrap the same `localQueryClient` instance the component reads from.
- The `TooltipProvider` from `@/components/ui/tooltip` is missing in the test wrapper — required if the `Blocked` badge uses a `Tooltip` (the `index.test.tsx` precedent wraps everything in `TooltipProvider`).

## Done when

`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/components/swarm/NodeApiKeySection.test.tsx" bash /data/.cache/opencode/packages/agent-plugins@git+https:/github.com/ExpansionX/agent-plugins.git/node_modules/agent-plugins/plugins/wai/scripts/task-gate.sh hive-node-api-key-ui 003` exits 0.
