---
id: "401"
phase: 4
title: "Update AGENTS.md with remote-frontend mandatory gates"
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - AGENTS.md
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC7]
---
## Failing test (write first)
N/A — documentation change, not code.

## Change
- **File:** AGENTS.md
- **Anchor:** "Finish What We Start" section, the gate block (lines 10-15)
- **Before:**
```bash
cargo clippy --all --all-targets --all-features -- -D warnings
cargo test --workspace
cd frontend && npm run lint
cd frontend && npx tsc --noEmit
```
- **After:**
```bash
cargo clippy --all --all-targets --all-features -- -D warnings
cargo test --workspace
cd frontend && npm run lint
cd frontend && npx tsc --noEmit
cd remote-frontend && npm run lint
cd remote-frontend && npx tsc --noEmit
cd remote-frontend && npx vitest run
```

## Allowed moves
Add 3 lines to the gate block in AGENTS.md.

## STOP triggers
- If the gate block format changes unexpectedly
- If CLAUDE.md also needs updating (check and update if needed)

## Manual verification (record in decisions-ledger)
```bash
grep -A 10 "cargo clippy" AGENTS.md
# Expected: shows all 7 gates including remote-frontend
```

## Done when
- AGENTS.md has all 7 gates
- CLAUDE.md matches (if it has its own gate block)
