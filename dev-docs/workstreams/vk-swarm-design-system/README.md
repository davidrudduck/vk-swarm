---
workstream: vk-swarm-design-system
doc_type: readme
status: draft
title: "VK-Swarm design system — Midnight Terminal component vocabulary + hive app UI kit"
staging_pointers:
  - docs/superpowers/specs/2026-07-04-vk-swarm-design-system.md
depends_on: []
adrs: []
---

# vk-swarm-design-system

**Origin:** Claude Design handoff bundle (preserved at
`dev-docs/designs/2026-07-04-vk-swarm-design-system/design-source/`). Ingested via
`/wai:design-handoff-ingest`; design spec at `design-spec.md`, gap analysis at
`gap-analysis.md` (31 findings: 1 A, 7 B, 23 C).

## What this workstream owns

Per the gap analysis, this workstream covers the **C-class un-built scope** from the design
handoff — the `.vks-*` component-class vocabulary, the refined token set (typography, spacing,
radius, motion, glows), the remaining texture utilities, and the hive app UI kit (BoardView,
Chrome, Panels/TaskDrawer) for `remote-frontend/`.

## Relationship to vk-swarm-hive-ui

`vk-swarm-hive-ui` (phase 1 tasks 100-106 done) shipped the hive auth shell — ProfileProvider
(Bearer/localStorage), oauthApi (bare JSON), useAuth, NormalLayout, AppRouter, root providers.
That shell is **reusable infrastructure** for this workstream's hive app UI kit. The
`vk-swarm-hive-ui` phase 2-3 tasks (202-308, the verbatim-copy approach) are **superseded** by
this design-system workstream: the design system replaces the "copy shadcn swarm components
verbatim" plan with a purpose-built `.vks-*` component vocabulary.

## Relationship to the program

Child of `vk-swarm-refactor`. Independent of `vk-swarm-hive-redesign` (renders over already-shipped
data plane). The node frontend (`frontend/`) is NOT modified by this workstream — it keeps its
shadcn/Tailwind UI as a HA fallback; the design system lands in `remote-frontend/` (hive).