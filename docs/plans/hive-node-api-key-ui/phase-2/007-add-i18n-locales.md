---
id: "007"
phase: 2
title: Add settings.swarm.apiKeys.* to en/es/ja/ko locale files + add TS9 test
status: ready
depends_on: ["004"]
parallel: false
conflicts_with: []
files:
  - frontend/src/i18n/locales/en/settings.json
  - frontend/src/i18n/locales/es/settings.json
  - frontend/src/i18n/locales/ja/settings.json
  - frontend/src/i18n/locales/ko/settings.json
  - remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx
  - remote-frontend/src/components/swarm/NodeApiKeySection.tsx
  - remote-frontend/src/components/swarm/index.ts
  - remote-frontend/src/components/swarm/index.test.tsx
  - remote-frontend/src/components/swarm/SwarmHealthSection.tsx
  - remote-frontend/src/components/swarm/NodeProjectsSection.tsx
  - remote-frontend/src/components/swarm/NodeTemplatesSection.tsx
  - remote-frontend/src/components/swarm/SwarmLabelsSection.tsx
  - remote-frontend/src/components/swarm/SwarmProjectsSection.tsx
  - remote-frontend/src/components/swarm/SwarmTemplatesSection.tsx
  - remote-frontend/src/components/swarm/NodeCard.tsx
  - remote-frontend/src/components/swarm/SwarmProjectRow.tsx
  - remote-frontend/src/components/swarm/MergeLabelsDialog.tsx
  - remote-frontend/src/components/swarm/MergeProjectsDialog.tsx
  - remote-frontend/src/components/swarm/MergeTemplatesDialog.tsx
  - remote-frontend/src/components/swarm/SwarmLabelDialog.tsx
  - remote-frontend/src/components/swarm/SwarmProjectDialog.tsx
  - remote-frontend/src/components/swarm/SwarmTemplateDialog.tsx
irreversible: false
scope_test: "remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx"
allowed_change: mixed
covers_criteria: [SC6]
covers_tests: [TS9]
---

## Failing test (write first)

The TS9 test was added in task 001 (the empty-state `getByText('settings.swarm.apiKeys.empty')` already depends on the key existing in the en locale when the production i18n config is used). This task adds a DEDICATED TS9 test that walks every `t('settings.swarm.apiKeys.*', ...)` call in `NodeApiKeySection.tsx` and asserts each one has a matching key in `frontend/src/i18n/locales/en/settings.json`. The test is the failure signal: until this task adds the keys, the new test fails.

```ts
// Append to remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx
import enSettings from '../../../../frontend/src/i18n/locales/en/settings.json';

it('every settings.swarm.apiKeys.* key used in NodeApiKeySection.tsx has a matching key in the en locale (TS9)', () => {
  // The list of keys the component actually uses, lifted from the t() calls in
  // NodeApiKeySection.tsx. If a key is added or removed, update this list AND the locale files.
  const requiredKeys = [
    'settings.swarm.apiKeys.title',
    'settings.swarm.apiKeys.description',
    'settings.swarm.apiKeys.create',
    'settings.swarm.apiKeys.createTitle',
    'settings.swarm.apiKeys.createDescription',
    'settings.swarm.apiKeys.createdTitle',
    'settings.swarm.apiKeys.copySecret',
    'settings.swarm.apiKeys.copyToClipboard',
    'settings.swarm.apiKeys.copied',
    'settings.swarm.apiKeys.envVarHint',
    'settings.swarm.apiKeys.nameLabel',
    'settings.swarm.apiKeys.namePlaceholder',
    'settings.swarm.apiKeys.cancel',
    'settings.swarm.apiKeys.createAction',
    'settings.swarm.apiKeys.loading',
    'settings.swarm.apiKeys.empty',
    'settings.swarm.apiKeys.bound',
    'settings.swarm.apiKeys.boundTooltip',
    'settings.swarm.apiKeys.unbound',
    'settings.swarm.apiKeys.created',
    'settings.swarm.apiKeys.lastUsed',
    'settings.swarm.apiKeys.active',
    'settings.swarm.apiKeys.revoked',
    'settings.swarm.apiKeys.blocked',
    'settings.swarm.apiKeys.blockedTooltip',
    'settings.swarm.apiKeys.revoke',
    'settings.swarm.apiKeys.revokeConfirm',
    'settings.swarm.apiKeys.unblock',
    'settings.swarm.apiKeys.unblockConfirm',
    'settings.swarm.apiKeys.error',
  ];
  const apiKeysBlock = (enSettings as any).settings?.swarm?.apiKeys ?? {};
  for (const key of requiredKeys) {
    const suffix = key.replace('settings.swarm.apiKeys.', '');
    expect(apiKeysBlock[suffix], `Missing i18n key: ${key}`).toBeDefined();
  }
});
```

## Change

The canonical key set is the 30 keys listed in spec §"Design / architecture" / "i18n key surface (proposed `settings.swarm.apiKeys.*`)". The English locale is the source of truth; the other three are parity translations (use empty string `""` for keys you cannot translate confidently — the i18n fallback string in the component handles the runtime, and the gate's TS9 test only checks the en locale).

For each of the four locale files, the change is identical in shape: insert a new `apiKeys` block inside the `swarm` object, immediately after `nodeTemplates` and before the closing `}` of `swarm`.

- **File:** `frontend/src/i18n/locales/en/settings.json`
- **Anchor:** `swarm` object — insert after `nodeTemplates: { ... }` (the `nodeTemplates` block ends at line 743 with `}` closing `promoteDialog: { ... }`, followed by `}` closing `nodeTemplates`, followed by `}` closing `swarm` at line 744).
- **Before:** (the closing braces of `nodeTemplates` and `swarm`, line 743-744)
  ```json
          "confirm": "Promote"
        }
      }
    },
  ```
- **After:**
  ```json
        "promoteConfirm": "Promote"
        }
      },
      "apiKeys": {
        "title": "Node API Keys",
        "description": "API keys allow nodes to authenticate with the hive server",
        "create": "Create API Key",
        "createTitle": "Create Node API Key",
        "createDescription": "Give your API key a name so you can identify it later.",
        "createdTitle": "API Key Created",
        "copySecret": "Copy this secret now. You won't be able to see it again.",
        "copyToClipboard": "Copy to Clipboard",
        "copied": "Copied!",
        "envVarHint": "Use this key as the VK_NODE_API_KEY environment variable when running the node.",
        "nameLabel": "Key Name",
        "namePlaceholder": "e.g., MacBook Pro, Build Server",
        "cancel": "Cancel",
        "createAction": "Create",
        "loading": "Loading API keys...",
        "empty": "No API keys found. Create one to allow nodes to connect.",
        "bound": "Bound",
        "boundTooltip": "Bound to node: {{id}}",
        "unbound": "Unbound",
        "created": "Created {{when}}",
        "lastUsed": "Last used {{when}}",
        "active": "Active",
        "revoked": "Revoked",
        "blocked": "Blocked",
        "blockedTooltip": "{{reason}}",
        "revoke": "Revoke",
        "revokeConfirm": "Are you sure you want to revoke this API key? Nodes using it will no longer be able to connect.",
        "unblock": "Unblock",
        "unblockConfirm": "Are you sure you want to unblock this API key? The node will be able to connect again.",
        "error": "Failed: {{message}}"
      }
    },
  ```

- **File:** `frontend/src/i18n/locales/es/settings.json`
- **Anchor:** the `swarm` object, after `nodeTemplates: { ... }`, before the closing `}` of `swarm`.
- **Before/After:** same insertion point. The block uses the same key names (the keys are the same JSON paths; only the values differ). For `es`, use the existing Spanish translations in the same file as a register guide. If a translation is uncertain, use the English value — the gate's TS9 test only checks the en locale.

- **File:** `frontend/src/i18n/locales/ja/settings.json`
- **Anchor:** same.
- **Before/After:** same insertion point. The block uses the same key names; values are Japanese translations. If a translation is uncertain, use the English value.

- **File:** `frontend/src/i18n/locales/ko/settings.json`
- **Anchor:** same.
- **Before/After:** same insertion point. The block uses the same key names; values are Korean translations. If a translation is uncertain, use the English value.

- **File:** `remote-frontend/src/components/swarm/NodeApiKeySection.test.tsx`
- **Anchor:** append the new `it(...)` block from "Failing test (write first" above to the existing `describe('NodeApiKeySection', ...)` block. Also add `import enSettings from '../../../../frontend/src/i18n/locales/en/settings.json';` at the top of the file (alongside the other imports).

## Allowed moves

- Edit the four locale files to insert the `apiKeys` block.
- Edit `NodeApiKeySection.test.tsx` to add the TS9 test and the `enSettings` import.
- Do NOT remove or rename any existing key in any of the four locale files.
- Do NOT touch the `NodeApiKeySection.tsx` component — the t() calls in the component are the source of truth for which keys are required; the locale files are the consumer.
- Do NOT touch the `frontend/src/i18n/locales/{en,es,ja,ko}/common.json` files (if any) — only `settings.json` is in scope for this workstream.

## STOP triggers

- The JSON becomes invalid (a missing comma, a duplicate key, an unclosed brace) — the gate's typecheck step will fail and the implementer must roll back to the last known-good state of the file.
- A key used in the component (per the `requiredKeys` list in the TS9 test) is missing from the en locale.
- The es/ja/ko locales are translated without consulting the existing sibling blocks (`nodeProjects`, `nodeTemplates`) in the same file for the correct register/voice.
- The TS9 test import path for `enSettings` is wrong (must traverse `../../../../` from `remote-frontend/src/components/swarm/` to reach `frontend/src/i18n/locales/en/`). If the path is too brittle, the implementer may hard-code the block as a `const` literal in the test instead of importing the file — note the choice in the decisions-ledger.

## Done when

`WAI_TYPECHECK_CMD="cd remote-frontend && npx tsc --noEmit" WAI_TEST_CMD="cd remote-frontend && npx vitest run src/components/swarm/NodeApiKeySection.test.tsx" bash /data/.cache/opencode/packages/agent-plugins@git+https:/github.com/ExpansionX/agent-plugins.git/node_modules/agent-plugins/plugins/wai/scripts/task-gate.sh hive-node-api-key-ui 007` exits 0.
