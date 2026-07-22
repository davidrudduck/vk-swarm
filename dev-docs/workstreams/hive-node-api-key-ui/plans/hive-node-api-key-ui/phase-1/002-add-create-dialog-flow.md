---
id: "002"
phase: 1
title: Add create-Dialog flow (name input, secret reveal, copy, show/hide) + TS4 test
status: ready
depends_on: ["001"]
parallel: false
conflicts_with: []
files:
  - remote-frontend/src/components/swarm/NodeApiKeySection.tsx
  - remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx
irreversible: false
scope_test: "remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx"
allowed_change: edit
covers_criteria: [SC2, SC7]
covers_tests: [TS4]
---

## Failing test (write first)

Append the following test to `remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx`. It must FAIL before the Dialog/secret/copy logic is added (the "Generate API Key" button currently has a no-op handler; the dialog never opens). It must PASS after the Change below lands.

```ts
it('opens the create Dialog, reveals the one-time secret, and supports show/hide + copy (TS4)', async () => {
  vi.mocked(nodesApi.listApiKeys).mockResolvedValue([]);
  vi.mocked(nodesApi.createApiKey).mockResolvedValue({
    api_key: {
      id: 'newk', organization_id: 'org-1', name: 'Test', key_prefix: 'vk_new',
      created_by: null, last_used_at: null, revoked_at: null, created_at: '2026-01-01T00:00:00Z',
      node_id: null, takeover_count: 0, takeover_window_start: null,
      blocked_at: null, blocked_reason: null,
    },
    secret: 'vk_SECRET_VALUE_DO_NOT_SHARE',
  });

  // Clipboard fallback: navigator.clipboard undefined (jsdom) → uses document.execCommand('copy').
  const execCommand = vi.fn(() => true);
  // @ts-expect-error — assigning a test double to read-only document
  document.execCommand = execCommand;
  // @ts-expect-error
  navigator.clipboard = undefined;

  const { fireEvent } = await import('@testing-library/react');
  renderWith(<NodeApiKeySection organizationId="org-1" />);

  // 1) Open the create Dialog from the "Generate API Key" button
  fireEvent.click(screen.getByRole('button', { name: 'settings.swarm.apiKeys.create' }));
  const nameInput = await screen.findByLabelText('settings.swarm.apiKeys.nameLabel');
  fireEvent.change(nameInput, { target: { value: 'Test Key' } });
  fireEvent.click(screen.getByRole('button', { name: 'settings.swarm.apiKeys.createAction' }));

  await waitFor(() => {
    expect(nodesApi.createApiKey).toHaveBeenCalledWith({ organization_id: 'org-1', name: 'Test Key' });
  });

  // 2) Secret-reveal view shows the secret. The component renders the secret text
  //    inside a <code> element; assert that the secret string is in the document.
  await waitFor(() => {
    expect(screen.getByText('vk_SECRET_VALUE_DO_NOT_SHARE')).toBeInTheDocument();
  });
  // The i18n t() mock does NOT apply the show/hide transformation to the secret
  // text — it just returns the literal "vk_..." string. The show/hide toggle flips
  // a CSS class on the wrapper. Assert the wrapper has the masked class initially.
  const secretWrapper = screen.getByText('vk_SECRET_VALUE_DO_NOT_SHARE').closest('[data-secret-wrapper]')!;
  expect(secretWrapper).toHaveAttribute('data-hidden', 'true');

  // 3) Show/hide toggle: click the Eye icon → wrapper's data-hidden flips to false
  fireEvent.click(screen.getByRole('button', { name: 'settings.swarm.apiKeys.showSecret' /* fallback in component: 'Show secret' */ }));
  expect(secretWrapper).toHaveAttribute('data-hidden', 'false');
  // Click again to hide
  fireEvent.click(screen.getByRole('button', { name: 'settings.swarm.apiKeys.hideSecret' /* fallback: 'Hide secret' */ }));
  expect(secretWrapper).toHaveAttribute('data-hidden', 'true');

  // 4) Copy button uses the non-HTTPS fallback
  fireEvent.click(screen.getByRole('button', { name: 'settings.swarm.apiKeys.copyToClipboard' }));
  expect(execCommand).toHaveBeenCalledWith('copy');

  // 5) Close + reopen the Dialog — the previous secret MUST be cleared from state.
  //    The Dialog's close button is the shadcn `DialogClose` rendered as a button
  //    with the `Cancel` label (the create dialog's footer).
  fireEvent.click(screen.getByRole('button', { name: 'settings.swarm.apiKeys.cancel' }));
  await waitFor(() => {
    expect(screen.queryByText('vk_SECRET_VALUE_DO_NOT_SHARE')).not.toBeInTheDocument();
  });
  // Reopen — should land on the name-input view, not the secret-reveal view
  fireEvent.click(screen.getByRole('button', { name: 'settings.swarm.apiKeys.create' }));
  await waitFor(() => {
    expect(screen.getByLabelText('settings.swarm.apiKeys.nameLabel')).toBeInTheDocument();
    expect(screen.queryByText('vk_SECRET_VALUE_DO_NOT_SHARE')).not.toBeInTheDocument();
  });
});
```

Use `fireEvent` from `@testing-library/react` (already in `remote-frontend/package.json`). Do NOT add `@testing-library/user-event` — the `fireEvent` API covers `click` and `change` for this test without introducing a new dependency. The component's `data-secret-wrapper` and `data-hidden` attributes are the implementation contract for the show/hide toggle; the implementer must add them in the Change step below.

## Change

- **File:** `remote-frontend/src/components/swarm/NodeApiKeySection.tsx`
- **Anchor:**
  - Imports block (top of file)
  - `NodeApiKeySection` function body (adds `useState` for dialog + secret)
  - "Generate API Key" `<Button>` (currently no-op — wire to `setShowCreateDialog(true)`)
  - Add a new `<Dialog>` after the `</Card>` closing tag
- **Before:** (the button's `onClick` is a no-op placeholder from task 001)
  ```tsx
  <Button onClick={() => setShowCreateDialog(true)}>
    <Plus className="h-4 w-4" />
    {t('settings.swarm.apiKeys.create', 'Create API Key')}
  </Button>
  ```
- **After:** (no change to the button itself; the state hook + Dialog are added — see below)

  1. Add `import { useState } from 'react';` if not already present.
  2. Add the imports: `Copy`, `Check`, `Eye`, `EyeOff` from `lucide-react`; `Dialog`, `DialogContent`, `DialogDescription`, `DialogFooter`, `DialogHeader`, `DialogTitle` from `@/components/ui/dialog`; `Input` from `@/components/ui/input`; `Label` from `@/components/ui/label`; `useMutation`, `useQueryClient` from `@tanstack/react-query`.
  3. Add state hooks at the top of `NodeApiKeySection`:
     ```tsx
     const [showCreateDialog, setShowCreateDialog] = useState(false);
     const [newKeyName, setNewKeyName] = useState('');
     const [createdSecret, setCreatedSecret] = useState<string | null>(null);
     const [showSecret, setShowSecret] = useState(false);
     const [copied, setCopied] = useState(false);
     ```
  4. Inside `NodeApiKeySection`, after the `useQuery`:
     ```tsx
     const queryClient = useQueryClient();
     const createMutation = useMutation({
       mutationFn: (name: string) => nodesApi.createApiKey({ organization_id: organizationId, name }),
       onSuccess: (response) => {
         setCreatedSecret(response.secret);
         setNewKeyName('');
         queryClient.invalidateQueries({ queryKey: ['nodeApiKeys', organizationId] });
       },
     });
     ```
  5. Add helper functions:
     ```tsx
     const handleCopySecret = async () => {
       if (!createdSecret) return;
       try {
         if (navigator.clipboard?.writeText) {
           await navigator.clipboard.writeText(createdSecret);
         } else {
           // Non-HTTPS fallback used by the reference impl
           const ta = document.createElement('textarea');
           ta.value = createdSecret;
           document.body.appendChild(ta);
           ta.select();
           document.execCommand('copy');
           document.body.removeChild(ta);
         }
         setCopied(true);
         setTimeout(() => setCopied(false), 2000);
       } catch (e) {
         console.error('Failed to copy secret', e);
       }
     };
     ```
  6. After the closing `</Card>` tag of the existing JSX, add the Dialog. Two views inside the same `Dialog`:
     - **Name-input view** (shown when `!createdSecret`): `<DialogHeader>` with `<DialogTitle>{t('settings.swarm.apiKeys.createTitle', 'Create Node API Key')}</DialogTitle>`, a `<Label htmlFor="key-name">{t('settings.swarm.apiKeys.nameLabel', 'Key Name')}</Label>`, an `<Input id="key-name" value={newKeyName} onChange={(e) => setNewKeyName(e.target.value)} />`, and a `<DialogFooter>` with a Cancel `<Button variant="outline" onClick={() => setShowCreateDialog(false)}>{t('settings.swarm.apiKeys.cancel', 'Cancel')}</Button>` and a Create `<Button onClick={() => createMutation.mutate(newKeyName)} disabled={createMutation.isPending || !newKeyName.trim()}>{t('settings.swarm.apiKeys.createAction', 'Create')}</Button>`.
     - **Secret-reveal view** (shown when `createdSecret` is non-null): `<DialogHeader>` with `<DialogTitle>{t('settings.swarm.apiKeys.createdTitle', 'API Key Created')}</DialogTitle>` and `<DialogDescription>{t('settings.swarm.apiKeys.copySecret', "Copy this secret now. You won't be able to see it again.")}</DialogDescription>`. Below: a `<code data-secret-wrapper data-hidden={!showSecret} className={showSecret ? '' : 'blur-sm select-none'}> {createdSecret}</code>` (the test asserts on `data-secret-wrapper` and `data-hidden` to verify the show/hide toggle), an Eye/EyeOff toggle `<Button aria-label={showSecret ? t('settings.swarm.apiKeys.hideSecret', 'Hide secret') : t('settings.swarm.apiKeys.showSecret', 'Show secret')} onClick={() => setShowSecret((v) => !v)}>`, and a Copy button calling `handleCopySecret()`. Closing the dialog (`onOpenChange={(_, open) => { if (!open) { setShowCreateDialog(false); setCreatedSecret(null); setNewKeyName(''); setShowSecret(false); }}}`) clears all four state values.
  7. Wire the `<Button>` for "Generate API Key" to `onClick={() => setShowCreateDialog(true)}` (already correct from task 001; the state hook from step 3 makes it functional).

- **File:** `remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx`
- **Anchor:** append the new `it(...)` block from "Failing test (write first" above to the existing `describe('NodeApiKeySection', ...)` block.

## Allowed moves

- Edit `NodeApiKeySection.tsx` per the Change above. No other files in this task.
- Edit `NodeApiKeySection.test.tsx` to add the TS4 test. No other test additions.
- Add the `data-secret-wrapper` and `data-hidden` attributes on the secret `<code>` element; the TS4 test asserts on them.
- Do NOT add the revoke / unblock mutations — that is task 003.
- Do NOT add the error-state Alert — that is task 004.
- Do NOT add the `ApiKeyItem` action button wiring — that is task 003.
- Do NOT change the locale JSON files — that is task 007.

## STOP triggers

- The `nodesApi.createApiKey` mock does not match the actual signature `(data: CreateNodeApiKeyRequest) => Promise<CreateNodeApiKeyResponse>` in `remote-frontend/src/lib/api/nodes.ts:67-79` and `remote-frontend/src/types/nodes.ts:67-75`.
- `navigator.clipboard` is not undefined in the test environment (jsdom usually does) — if it is defined, the test still passes because the code falls through to the `if` branch; record the result in the ledger either way.
- `fireEvent.click` and `fireEvent.change` from `@testing-library/react` are the only DOM-event primitives used in the test — do NOT add `@testing-library/user-event` as a dependency.
- The "Create" button submit handler fires without awaiting — the test must see the create call AND the secret; if only one fires, the test is wrong, not the implementation.

## Done when

`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/components/swarm/NodeApiKeySection.test.tsx" bash /data/.cache/opencode/packages/agent-plugins@git+https:/github.com/ExpansionX/agent-plugins.git/node_modules/agent-plugins/plugins/wai/scripts/task-gate.sh hive-node-api-key-ui 002` exits 0.
