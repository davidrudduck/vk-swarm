---
phase: 3
title: Replace MergedProject with ProjectWithStats
tasks: ["301","302","303"]
---

# Phase 3 — Replace MergedProject with ProjectWithStats

ADR-0014. Additive first (301), repoint (302), then delete (303) — so the board is never broken
between tasks.

The payload must stay behaviourally identical apart from the three hardcoded fields being
dropped: the board regression `a85f7d63` fixed (blank board, no task counts) must not return.
