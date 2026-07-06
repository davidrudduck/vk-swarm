# Tournament Round 1 — remote-docker-build-fix

**Date:** 2026-07-05
**Target:** remote-docker-build-fix diff (Dockerfile, CI, package.json)
**Roster:** gemini (Gemini CLI), codex (Codex CLI), claude-sonnet-4-6 (Claude Code)

## Leaderboard

| Rank | Model | Score | Valid | Invalid | Remediations |
|------|-------|-------|-------|---------|--------------|
| 1 | Codex | 6.0 | 2 | 0 | 2/2 pass |
| 2 | Claude (Opus) | 3.5 | 3 | 2 | 2/3 pass (1 steal) |
| 3 | Gemini | 1.0 | 1 | 4* | 0/1 pass |

*4 Gemini findings rejected as hallucinations (invented spaces in `pnpm@` and garbled checkout path)

## Findings Detail

### Codex (winner — 6.0 pts)

| ID | Severity | Issue | Remediation | Pts |
|---|---|---|---|---|
| codex-F001 | blocking | No pipefail in `docker compose ... | tee` CI build step | `set -o pipefail` | 3.0 |
| codex-F002 | should-fix | `ARG PNPM_VERSION` in remote Dockerfile has no default | `ARG PNPM_VERSION=10.25.0` | 3.0 |

### Claude/Opus (3.5 pts)

| ID | Severity | Issue | Remediation | Pts |
|---|---|---|---|---|
| opus-F001 | should-fix | Root Dockerfile hardcodes `pnpm@10.25.0` — no drift guard | `ARG PNPM_VERSION` + variable ref | 3.0 |
| opus-F002 | should-fix | ARG PNPM_VERSION no default (STEAL from codex-F002) | — | 0.0 |
| opus-F003 | info | Missing trailing newline in remote-hive-build.yml | Add newline | 1.5 |
| opus-F004 | info | debug dep 4.4.3→4.4.1 in lockfile (side-effect, not ours) | — | -0.5 |
| opus-F005 | info | engines.node CI threshold "hardcoded to 22" (intentional floor) | — | -0.5 |

### Gemini (1.0 pts)

| ID | Severity | Issue | Remediation | Pts |
|---|---|---|---|---|
| gemini-F001 | should-fix | Pipefail missing in CI build step | `|| true` — REJECTED (conceals failures) | 1.0 |

## Decider Notes

- **gemini-F001 remediation rejected**: `|| true` after the pipeline would make ALL builds pass, masking real failures. The correct fix is `set -o pipefail`.
- **opus-F002** scored as steal (duplicate of codex-F002). Both panels ran concurrently; codex filed first alphabetically.
- **opus-F004** excluded: debug dependency downgrade is a transitive consequence of `pnpm install` during merge conflict resolution, not caused by our changes.
- **opus-F005** rejected: `[ "$MAJOR" -ge 22 ]` is the intended floor check matching engines.node `>=22.13`. Not a bug.

## Action Items

Valid remediations to implement in alignment round:
1. **codex-F001**: Add `set -o pipefail` before docker compose build pipeline in CI
2. **codex-F002/opus-F002**: Add `ARG PNPM_VERSION=10.25.0` default in `crates/remote/Dockerfile`
3. **opus-F001**: Convert root `Dockerfile` from hardcoded `pnpm@10.25.0` to `ARG PNPM_VERSION` + variable reference
4. **opus-F003**: Add trailing newline to `.github/workflows/remote-hive-build.yml`
