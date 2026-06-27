# Task 301: Executor resume-capability audit (SC1)

## Purpose
Map each CodingAgent/BaseCodingAgent variant to its crash-recovery branch for task 304.
When an executor process crashes during execution, we need to know whether it can:
1. Resume the same session (recover state)
2. Cold-respawn from scratch (stateless re-entry)
3. Must be marked failed (no recovery path)

## Capability table

| Variant | Supports `spawn_follow_up(--resume session_id)`? | How (evidence: file:line) | Recovery branch | Resume-prompt semantics | Notes |
|---------|--------------------------------------------------|--------------------------|-----------------|------------------------|-------|
| ClaudeCode | YES | claude.rs:264-267 (--fork-session --resume flags) | resume | Re-send original prompt from executor_sessions.prompt | Claude implements full session fork with `--fork-session --resume {session_id}`. Session state fully recoverable. Index repair happens first (claude.rs:259). |
| Amp | YES | amp.rs:105-109 (threads fork {session_id}) | resume | Re-send original prompt from executor_sessions.prompt | Amp uses stateful thread fork via `threads fork {session_id}`. Synchronous fork obtains new thread ID, then spawns follow-up. |
| Gemini | YES | gemini.rs:140 + acp/harness.rs:136 (existing_session param) | resume | Re-send original prompt from executor_sessions.prompt | Gemini delegates to AcpAgentHarness with session recovery. ACP harness passes session_id to bootstrap_acp_connection via existing_session field. |
| QwenCode | YES | qwen.rs:68 + acp/harness.rs:136 (existing_session param) | resume | Re-send original prompt from executor_sessions.prompt | QwenCode uses ACP harness with "qwen_sessions" namespace (qwen.rs:66). Session recovery via existing_session parameter to ACP bootstrap. |
| Codex | YES | codex.rs:233,247 (session_id param to spawn_internal) | resume | Re-send original prompt from executor_sessions.prompt | Codex passes session_id through spawn_internal. AppServerClient manages session state internally. Protocol peer available for live message injection. |
| CursorAgent | YES | cursor.rs:128 (--resume {session_id}) | resume | Re-send original prompt from executor_sessions.prompt | CursorAgent CLI supports native `--resume` flag. Session state managed by Cursor agent itself. |
| Opencode | YES | opencode.rs:220 (--session {session_id}) | resume | Re-send original prompt from executor_sessions.prompt | Opencode uses `--session` flag (not --resume) to resume session. Session recovery mechanism built into Opencode CLI. |
| Copilot | YES | copilot.rs:165 (--resume {session_id}) | resume | Re-send original prompt from executor_sessions.prompt | Copilot CLI supports `--resume` flag. Temp log directory created per follow-up (copilot.rs:162). |
| Droid | YES | droid.rs:171-178 (fork_session + --session-id) | resume | Re-send original prompt from executor_sessions.prompt | Droid implements session forking via fork_session() helper (droid.rs:171). Forked session ID passed via `--session-id` flag. |

## Variant count reconciliation
- `enum CodingAgent` variants found: 9
  1. ClaudeCode
  2. Amp
  3. Gemini
  4. Codex
  5. Opencode
  6. CursorAgent
  7. QwenCode
  8. Copilot
  9. Droid

- Table rows: 9 (matches)
- Note: `QaMock` not yet present in codebase (forward-ported by task 201). Once task 201 lands,
  QaMock is a test-only mock executor → classify as **cold-respawn** (stateless, no session state).

## Recovery branch classification
All 9 variants classified as **resume**:
- ClaudeCode: YES (--fork-session --resume)
- Amp: YES (threads fork)
- Gemini: YES (ACP harness)
- QwenCode: YES (ACP harness with namespace)
- Codex: YES (AppServerClient session state)
- CursorAgent: YES (--resume flag)
- Opencode: YES (--session flag)
- Copilot: YES (--resume flag)
- Droid: YES (fork_session + --session-id)

No variants classified as **cold-respawn** or **mark-failed**.

## Resume-prompt recommendation
For **resume-class executors**: Re-send the original prompt from `executor_sessions.prompt`

**Reasoning:**
- All 9 variants have session recovery mechanisms that preserve conversation state
- The session ID maps to complete conversation history in the executor's storage
- Sending the original prompt upon resume will make agents aware of context and allow them to resume intelligently
- None of the executors attempt to auto-continue without a new prompt (all implement spawn_follow_up with prompt parameter)
- Minimal continuation prompt (e.g., "Continue from where you left off") would lose task context for stateless agents

**Default for task 303:** Re-send original `executor_sessions.prompt` value on crash recovery.

**Alternative considered:** Minimal continuation prompt
- Rejected because: Loses original task context, may cause agents to re-start instead of resume, reduces effectiveness of state recovery

## STOP triggers triggered
None. All source files were successfully read and classified. No blockers encountered.

## Implementation notes for task 304
1. On executor crash detection:
   - Look up executor_sessions record by task_attempt_id
   - Extract session_id from executor_sessions.session_id
   - Extract original prompt from executor_sessions.prompt
   
2. Call spawn_follow_up with:
   - current_dir: worktree path
   - prompt: executor_sessions.prompt (re-send original)
   - session_id: executor_sessions.session_id

3. Log recovery attempt before spawn_follow_up call for observability

4. Verify that all executor implementations accept spawn_follow_up calls with recovered session IDs (all do per this audit).
