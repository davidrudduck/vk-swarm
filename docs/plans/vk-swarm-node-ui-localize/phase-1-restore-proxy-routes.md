---
phase: 1
title: Restore the hive proxy route layer
tasks: ["101","102","103","104","105"]
---

# Phase 1 — Restore the hive proxy route layer

Ships the boundary: every URL in the spec's Intent table answers instead of 404.

Each of 101–104 restores ONE module verbatim from `35b378a5^` and registers it, so each task
leaves the tree with one more working surface. They are strictly sequential — all four edit
`crates/server/src/routes/mod.rs`.

105 proves reachability over HTTP against a running server (see plan.md, "Known limitation").
