---
phase: 2
title: Remove the node's API-key surface (D3)
tasks: ["201","202","203"]
---

# Phase 2 — Remove the node's API-key surface

Decision D3 / ADR-0013: the hive owns node API-key management. The node's copy is deleted, not
reconnected, and `/nodes/api-keys*` is deliberately never restored in phase 1.

201 removes the UI; 202 removes the now-unreachable client methods.
