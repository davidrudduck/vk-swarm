---
workstream: hive-node-api-key-ui
doc_type: readme
status: shipped
title: "Hive node API key management UI — generate/revoke/unblock keys on the Nodes page"
staging_pointers:
  - docs/superpowers/specs/2026-07-07-hive-node-api-key-ui.md
depends_on: []
adrs: []
---

# hive-node-api-key-ui

**Origin:** Promoted from finding `F-2026-07-06-01` (high severity, discovered 2026-07-06)
via `/wai:finding-promote`. The finding recorded that the Hive UI lacks a "Generate API
key" button, blocking node onboarding.

## What this workstream owns

Surfacing API key management (create / list / revoke / unblock) on the Hive Nodes page
(`remote-frontend/src/pages/Nodes.tsx`). The backend (`/v1/nodes/api-keys`), the API client
(`remote-frontend/src/lib/api/nodes.ts`), and the types (`remote-frontend/src/types/nodes.ts`)
already exist — this is a `remote-frontend/`-only UI workstream. The main `frontend/`
`NodeApiKeySection` is a behavioral reference, not modified.

## Relationship to the program

Independent of `vk-swarm-design-system` (this workstream uses the existing shadcn/ui
primitives, not the `.vks-*` vocabulary). Consumes the node-auth API shipped by
`vk-swarm-node-foundations`. No backend dependency — all endpoints already work.
