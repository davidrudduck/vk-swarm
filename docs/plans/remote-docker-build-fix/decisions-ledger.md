# Decisions Ledger — remote-docker-build-fix

Run started: 2026-07-05

## Sibling alignment: task 004 — scripts/assert-dockerfile-node-match.sh

Warnings from `wai-plan-lint.sh`: `scripts/check-i18n.sh`, `scripts/clean-cargo-cache.sh`,
`scripts/dev-swarm-setup.sh` are unlisted siblings. Listed in `files:` and justified in task
frontmatter: none share guards, exclusions, or error-handling patterns with the assertion script.
`dev-swarm-setup.sh` is a full cluster bring-up orchestrator (swarm init, Docker Compose stacking);
`assert-dockerfile-node-match.sh` is a single deterministic grep assertion. No divergence to justify.

## Sibling alignment: task 005 — .github/workflows/remote-hive-build.yml

Warnings from `wai-plan-lint.sh`: `.github/workflows/pre-release.yml` and `.github/workflows/publish.yml`
are listed in `files:`. They share `runs-on: ubuntu-latest` and `actions/checkout@v4`. `pre-release.yml`
uses `permissions: contents: write` (publishes releases); our CI uses `permissions: contents: read` —
correct per GitHub least-privilege. Justified in task frontmatter.

- [Task 004] Used `set -euo pipefail` (adds `-u` for unbound variable detection) instead of sibling convention `set -eo pipefail` — assertion scripts benefit from catching unbound variables early — `scripts/assert-dockerfile-node-match.sh`

---## Task 004 — orchestrator amendment (panel finding)

- [Task 004 orchestrator] Added `|| true` guards to grep command substitutions — the adversarial panel found that `set -euo pipefail` kills the script on grep non-zero exit before the `-z` guard fires, making exit 2 (structural error) unreachable. Fix: `grep ... | sed ... || true` captures empty string on grep failure, allowing the `-z` check to branch. — `scripts/assert-dockerfile-node-match.sh`
