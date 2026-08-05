---
phase: 4
title: Hive-absent as a first-class state
tasks: ["401","402","403"]
---

# Phase 4 — Hive-absent as a first-class state

A node may run standalone (C4). Today `RemoteClientNotConfigured` collapses into
`ApiError::BadRequest("Remote client not configured")` — a generic 400 the frontend cannot
distinguish from a real bad request.

401 gives it a discriminable status; 402 renders it; 403 hardens the four remote stream hooks.
