# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added
- Activity API now accepts `swarm_project_id` parameter for swarm project queries
- New `ensure_swarm_project_access()` function for swarm project access control
- Database columns: `swarm_project_id` added to `shared_tasks`, `activity`, `project_activity_counters`
- New `fetch_since_by_swarm_project()` method to ActivityRepository for efficient swarm project queries
- API documentation for activity endpoint with migration guide
- Default `PLAN` variant for Codex agent profiles
- Codex runtime capability discovery for native collaboration modes
- Codex experimental API negotiation during app-server initialization
- Append-only execution-log timeline utility for sanitizing streamed log updates
- Localized Codex runtime-capability status messaging in agent settings

### Deprecated
- Activity API `project_id` parameter deprecated in favor of `swarm_project_id`
- Will be removed in version 2.0 (Q3 2026)

### Changed
- Activity queries can now filter by swarm project instead of legacy project
- Access control uses `swarm_projects` table instead of `projects`
- Architecture documentation updated to reflect `swarm_projects` as canonical source of truth
- Codex executor now uses the v2 app-server thread/turn protocol instead of the legacy conversation API
- Codex collaboration mode settings are now runtime-discovered instead of statically exposed in normal profiles
- Codex falls back to standard mode with an explicit system notice when a requested native collaboration mode is unavailable or cannot be verified
- Execution log rendering now preserves previously written streamed content instead of rewriting earlier rows during live updates
