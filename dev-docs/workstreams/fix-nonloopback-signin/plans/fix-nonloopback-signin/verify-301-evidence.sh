#!/usr/bin/env bash
set -euo pipefail

LEDGER="docs/plans/fix-nonloopback-signin/decisions-ledger.md"

require_line() {
  local pattern="$1"
  if ! grep -Eq -- "$pattern" "$LEDGER"; then
    echo "missing acceptance evidence matching: $pattern" >&2
    exit 1
  fi
}

require_line '^## Acceptance evidence$'
require_line 'cd remote-frontend && npm run test:run -- src/pkce\.test\.ts src/AppRouter\.test\.tsx src/pages/InvitationPage\.test\.tsx src/pages/InvitationCompletePage\.test\.tsx.*PASS'
require_line 'cd remote-frontend && npm run test:run.*PASS'
require_line 'cd remote-frontend && npm run lint.*PASS'
require_line 'cd remote-frontend && npx tsc --noEmit.*PASS'
require_line 'cargo clippy --all --all-targets --all-features -- -D warnings.*PASS'
require_line 'cargo test --workspace.*PASS'
require_line 'cd frontend && npm run lint.*PASS'
require_line 'cd frontend && npx tsc --noEmit.*PASS'
require_line 'Normal login over `http://[^`]+/login`.*PASS'
require_line 'Invitation OAuth over `http://[^`]+/invitations/[^`]+/accept`.*PASS'

if grep -Eq 'PASS/FAIL|FAIL|unavailable|inconclusive|could not run|not run' "$LEDGER"; then
  echo "acceptance evidence contains a failing, unavailable, inconclusive, or placeholder result" >&2
  exit 1
fi
