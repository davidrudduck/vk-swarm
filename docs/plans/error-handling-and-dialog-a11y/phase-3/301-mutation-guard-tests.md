---
id: "301"
phase: 3
title: "Add mutation guard tests for createAttemptRef and orgIdRef"
status: ready
depends_on: ["202"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx
irreversible: false
scope_test: "remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx"
allowed_change: edit
covers_criteria: [SC4, SC5]
---
## Failing test (write first)
These tests exercise guards that prevent stale mutation callbacks from executing. The tests
verify the guard logic directly — not the org-change effect (which also clears state).

## Change
- **File:** remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx
- **Anchor:** end of `describe('NodeApiKeySection', ...)` block (after last existing test)
- **Before:** (end of existing test suite)
- **After:** Add these 4 tests before the closing `});`:

```typescript
  it('createAttemptRef: onSuccess does not call setCreatedSecret when dialog was closed before resolve (SC4)', async () => {
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue([]);
    let resolveCreate: (v: unknown) => void;
    const createPromise = new Promise((resolve) => { resolveCreate = resolve; });
    vi.mocked(nodesApi.createApiKey).mockReturnValue(createPromise as ReturnType<typeof nodesApi.createApiKey>);
    const { fireEvent } = await import('@testing-library/react');
    renderWith(<NodeApiKeySection organizationId="org-1" />);
    fireEvent.click(screen.getByRole('button', { name: 'Generate API Key' }));
    const nameInput = await screen.findByLabelText('Key Name');
    fireEvent.change(nameInput, { target: { value: 'Test' } });
    fireEvent.click(screen.getByRole('button', { name: 'Create' }));
    // Close dialog BEFORE mutation resolves — createAttemptRef increments in closeDialog
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    // Verify dialog is closed (no form visible)
    expect(screen.queryByLabelText('Key Name')).not.toBeInTheDocument();
    // Resolve the mutation — onSuccess should be a no-op due to createAttemptRef guard
    resolveCreate!({
      api_key: { id: 'newk', organization_id: 'org-1', name: 'Test', key_prefix: 'vk_new',
        created_by: null, last_used_at: null, revoked_at: null, created_at: '2026-01-01T00:00:00Z',
        node_id: null, takeover_count: 0, takeover_window_start: null,
        blocked_at: null, blocked_reason: null },
      secret: 'vk_STALE_AFTER_CLOSE',
    });
    // Guard prevents setCreatedSecret — secret must NOT appear
    await waitFor(() => {
      expect(screen.queryByText('vk_STALE_AFTER_CLOSE')).not.toBeInTheDocument();
    });
    // Dialog must remain closed (guard didn't reopen it)
    expect(screen.queryByLabelText('Key Name')).not.toBeInTheDocument();
  });

  it('createAttemptRef: onSuccess does not set createdSecret when org changed before resolve (SC4)', async () => {
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue([]);
    let resolveCreate: (v: unknown) => void;
    const createPromise = new Promise((resolve) => { resolveCreate = resolve; });
    vi.mocked(nodesApi.createApiKey).mockReturnValue(createPromise as ReturnType<typeof nodesApi.createApiKey>);
    const { fireEvent } = await import('@testing-library/react');
    const result = renderWith(<NodeApiKeySection organizationId="org-1" />);
    fireEvent.click(screen.getByRole('button', { name: 'Generate API Key' }));
    const nameInput = await screen.findByLabelText('Key Name');
    fireEvent.change(nameInput, { target: { value: 'Test' } });
    fireEvent.click(screen.getByRole('button', { name: 'Create' }));
    // Org changes — createAttemptRef increments in the org-change effect
    result.rerender(
      <QueryClientProvider client={queryClient}>
        <TooltipProvider><NodeApiKeySection organizationId="org-2" /></TooltipProvider>
      </QueryClientProvider>
    );
    // Resolve the mutation — onSuccess should be a no-op due to createAttemptRef guard
    resolveCreate!({
      api_key: { id: 'newk', organization_id: 'org-1', name: 'Test', key_prefix: 'vk_new',
        created_by: null, last_used_at: null, revoked_at: null, created_at: '2026-01-01T00:00:00Z',
        node_id: null, takeover_count: 0, takeover_window_start: null,
        blocked_at: null, blocked_reason: null },
      secret: 'vk_STALE_AFTER_ORG_CHANGE',
    });
    // Guard prevents setCreatedSecret — secret must NOT appear
    await waitFor(() => {
      expect(screen.queryByText('vk_STALE_AFTER_ORG_CHANGE')).not.toBeInTheDocument();
    });
  });

  it('orgIdRef: revoke onError does not call setError when org changed before reject (SC4/SC5)', async () => {
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    try {
      const keys: NodeApiKey[] = [{
        id: 'k1', organization_id: 'org-1', name: 'Active', key_prefix: 'vk_a',
        created_by: null, last_used_at: null, revoked_at: null, created_at: '2026-01-01T00:00:00Z',
        node_id: 'n1', takeover_count: 0, takeover_window_start: null,
        blocked_at: null, blocked_reason: null,
      }];
      let rejectRevoke: (v: unknown) => void;
      const revokePromise = new Promise((_, reject) => { rejectRevoke = reject; });
      vi.mocked(nodesApi.listApiKeys).mockResolvedValue(keys);
      vi.mocked(nodesApi.revokeApiKey).mockReturnValue(revokePromise as ReturnType<typeof nodesApi.revokeApiKey>);
      const result = renderWith(<NodeApiKeySection organizationId="org-1" />);
      const { fireEvent } = await import('@testing-library/react');
      fireEvent.click(await screen.findByRole('button', { name: 'Revoke' }));
      // Org changes — orgIdRef.current becomes "org-2"
      result.rerender(
        <QueryClientProvider client={queryClient}>
          <TooltipProvider><NodeApiKeySection organizationId="org-2" /></TooltipProvider>
        </QueryClientProvider>
      );
      // Reject the mutation — onError should be a no-op due to orgIdRef guard
      rejectRevoke!(new Error('revoke failed'));
      // Guard prevents setError — no error alert should appear
      await waitFor(() => {
        expect(screen.queryByRole('alert')).not.toBeInTheDocument();
      });
    } finally {
      confirmSpy.mockRestore();
    }
  });

  it('orgIdRef: create onError does not call setError when org changed before reject (SC5)', async () => {
    vi.mocked(nodesApi.listApiKeys).mockResolvedValue([]);
    let rejectCreate: (v: unknown) => void;
    const createPromise = new Promise((_, reject) => { rejectCreate = reject; });
    vi.mocked(nodesApi.createApiKey).mockReturnValue(createPromise as ReturnType<typeof nodesApi.createApiKey>);
    const { fireEvent } = await import('@testing-library/react');
    const result = renderWith(<NodeApiKeySection organizationId="org-1" />);
    fireEvent.click(screen.getByRole('button', { name: 'Generate API Key' }));
    const nameInput = await screen.findByLabelText('Key Name');
    fireEvent.change(nameInput, { target: { value: 'Test' } });
    fireEvent.click(screen.getByRole('button', { name: 'Create' }));
    // Org changes — orgIdRef.current becomes "org-2"
    result.rerender(
      <QueryClientProvider client={queryClient}>
        <TooltipProvider><NodeApiKeySection organizationId="org-2" /></TooltipProvider>
      </QueryClientProvider>
    );
    // Reject the mutation — onError should be a no-op due to orgIdRef guard
    rejectCreate!(new Error('create failed'));
    // Guard prevents setError — no error alert should appear
    await waitFor(() => {
      expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    });
  });
```

## Allowed moves
Add 4 new test cases to `NodeApiKeySection.test.tsx` before the closing `});`.

## STOP triggers
- If any of the 4 new tests pass without the existing guards in NodeApiKeySection.tsx (hollow tests)
- If any of the 36 existing tests break

## Manual verification (record in decisions-ledger)
```bash
cd remote-frontend && npx vitest run src/components/swarm/NodeApiKeySection.test.tsx
# Expected: 40 tests pass (36 existing + 4 new)
```

## Done when
- 4 new tests added
- All 40 tests pass (36 existing + 4 new)
- SC4 (createAttemptRef: tests 1-2) and SC5 (orgIdRef: tests 3-4) satisfied
