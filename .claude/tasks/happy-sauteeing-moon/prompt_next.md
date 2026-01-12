# Prompt Improvements for Implementation Sessions

Based on validation of the `reject_if_remote` implementation, these prompt additions address specific deviations observed.

---

## 1. Commit Message Format Rules

**Issue Observed:** Commit `a230f7cab` had "---" as the title instead of a proper conventional commit message.

**Add to Session Agent Prompt (STEP 7: COMMIT PROGRESS section):**

```markdown
## COMMIT MESSAGE RULES (CRITICAL)

**Format:** Always use conventional commit format: `type: description`

**Valid types:**
- `feat:` - New feature
- `fix:` - Bug fix
- `test:` - Adding/updating tests
- `docs:` - Documentation changes
- `chore:` - Maintenance tasks (formatting, deps)
- `refactor:` - Code restructuring without behavior change

**Examples:**
- `feat: add reject_if_remote helper function`
- `test: add test_reject_if_remote_rejects_remote_project (RED phase)`
- `docs: update swarm-api-patterns with middleware bypass pattern`
- `chore: apply rustfmt formatting`

**NEVER use as commit message:**
- `---` or YAML frontmatter
- Markdown headers (`#`, `##`)
- Multi-paragraph summaries as the title
- Empty or whitespace-only titles

**Commit command pattern:**
```bash
# CORRECT - Single line message
git commit -m "test: add test for remote rejection (RED phase)"

# CORRECT - Multi-line with proper title first
git commit -m "feat: implement reject_if_remote helper" -m "Adds remote project check for message queue handlers"

# WRONG - HEREDOC can cause malformed titles if first line is empty or frontmatter
git commit -m "$(cat <<EOF
---
# Summary
...
EOF
)"
```

**Verification (MANDATORY before moving on):**
```bash
git log --oneline -1
# Must show: abc1234 type: clear description
# If title is "---" or malformed, amend immediately:
git commit --amend -m "correct: commit message here"
```
```bash

---

## 2. Scope Discipline for Formatting

**Issue Observed:** Running `cargo fmt --all` formatted unrelated files in `tasks/` handlers.

**Add to CRITICAL RULES section:**

```markdown
## SCOPE DISCIPLINE (CRITICAL)

**Only modify files directly related to the current task.**

**For formatting commands:**
```bash
# WRONG - Formats entire codebase, may touch unrelated files
cargo fmt --all

# CORRECT - Format only files you modified
cargo fmt -- crates/server/src/routes/message_queue.rs

# CORRECT - Check first, then decide
cargo fmt --all -- --check
# Review output - if files outside your task scope need formatting:
# 1. Do NOT format them in this PR
# 2. Document as "Pre-existing formatting issue" in progress notes
# 3. Optionally create separate cleanup task
```

**For linting commands:**
```bash
# OK to run full clippy for verification
cargo clippy --all --all-targets --all-features -- -D warnings

# But ONLY fix warnings in files you're modifying for this task
# Ignore warnings in unrelated files
```

**Rationale:**
- Bundling unrelated changes makes code review harder
- Pollutes git blame history
- Can introduce merge conflicts
- Makes rollbacks more complex
```bash

---

## 3. Documentation Accuracy

**Issue Observed:** Documentation example showed `Path((attempt_id, message_id))` but actual implementation uses `Path(params): Path<MessageQueueParams>`.

**Add to Session 4 / Documentation tasks:**

```markdown
## DOCUMENTATION ACCURACY (CRITICAL)

**When writing documentation that includes code examples:**

1. **COPY-PASTE actual code** - Never type examples from memory
   ```bash
   # Before writing docs, read the actual implementation
   cat crates/server/src/routes/message_queue.rs | grep -A 10 "fn update_queued_message"
   ```

2. **Verify examples match reality**
   - Read the source file
   - Copy the exact function signature
   - If simplifying, note "simplified example" explicitly

3. **Cross-reference before committing**
   ```bash
   # Verify doc examples match implementation
   grep "Path<MessageQueueParams>" crates/server/src/routes/message_queue.rs
   grep "Path<MessageQueueParams>" docs/architecture/swarm-api-patterns.mdx
   # Both should show same pattern
   ```

**Common mistakes to avoid:**
- Writing `Path((attempt_id, message_id))` when code uses `Path(params): Path<MessageQueueParams>`
- Documenting function signatures from memory
- "Simplifying" examples in ways that don't compile
```

---

## 4. Plan Fidelity Reinforcement

**Add to CRITICAL RULES section:**

```markdown
## PLAN FIDELITY (CRITICAL)

**The approved plan is the specification. Implementation must match it exactly.**

**Before marking any acceptance criterion complete:**
1. Re-read the criterion from the task file
2. Compare your implementation line-by-line
3. If they differ:
   - Option A: Fix implementation to match plan
   - Option B: Document deviation with justification

**Acceptance criteria are IMMUTABLE:**
- You cannot check off a criterion by changing what it says
- You can only check it off by implementing what it originally specified
- If criterion is wrong/impossible, STOP and escalate

**Deviation documentation (when necessary):**
Add a `## Deviations` section to the task file:
```markdown
## Deviations

### Criterion: "Use tuple extraction for path parameters"
**Actual Implementation:** Used `MessageQueueParams` struct
**Justification:** Struct pattern matches existing codebase conventions and is more maintainable
**Impact:** Functionally equivalent, documentation example updated to match
```

**Red flags that indicate plan drift:**
- "I found a better way to do this"
- "The plan said X but I did Y because..."
- "This is essentially the same thing"
- Changing acceptance criteria checkboxes without doing the work
```bash

---

## 5. Model Selection Guidance

**Question:** Can Sonnet/Haiku be used instead of Opus 4.5 for implementation?

**Analysis of this plan's structure:**

| Aspect | Opus Needed? | Sonnet OK? | Haiku OK? |
|--------|--------------|------------|-----------|
| Task decomposition (9 clear tasks) | No | Yes | Yes |
| Code is pre-written in plan | No | Yes | Yes |
| Clear acceptance criteria | No | Yes | Yes |
| TDD phases labeled (RED/GREEN/REFACTOR) | No | Yes | Yes |
| File locations specified | No | Yes | Yes |
| Exact line numbers given | No | Yes | Yes |

**Recommendation:**

```markdown
## MODEL SELECTION BY TASK TYPE

**Use Opus 4.5 for:**
- Planning and architecture decisions
- Ambiguous requirements interpretation
- Complex debugging with multiple possible causes
- Validation and code review
- Tasks requiring judgment calls

**Use Sonnet 4 for:**
- Well-specified implementation tasks (like this plan)
- Tasks with pre-written code snippets to insert
- Clear acceptance criteria
- Straightforward refactoring
- Documentation updates

**Use Haiku 3.5 for:**
- Single-file, single-function changes
- Running pre-defined test suites
- Formatting and linting
- Simple search/replace operations
- Status checks and health verification

**This plan's tasks by recommended model:**

| Task | Description | Recommended Model |
|------|-------------|-------------------|
| 001 | Add test module structure | Haiku |
| 002-004 | Write specific test cases | Sonnet |
| 005 | Implement helper function | Sonnet |
| 006-007 | Integrate into handlers | Sonnet |
| 008 | Run formatter/linter | Haiku |
| 009 | Update documentation | Sonnet |
| Validation | Review entire implementation | Opus |
```

**Implementation in prompt:**
```markdown
## MODEL PARAMETER

This task should be executed with model: `sonnet` (or `haiku` for tasks 001, 008)

Specify in Task tool call:
```javascript
Task({
  subagent_type: "implementation",
  model: "sonnet",  // or "haiku" for simple tasks
  prompt: "Execute task 005..."
})
```
```

---

## Summary of Additions

| Section | Addition | Purpose |
|---------|----------|---------|
| STEP 7 | Commit message rules + verification | Prevent malformed commits |
| CRITICAL RULES | Scope discipline | Prevent unrelated changes |
| Session 4 / Docs | Documentation accuracy | Ensure examples match code |
| CRITICAL RULES | Plan fidelity | Prevent silent deviations |
| New section | Model selection | Cost/speed optimization |

---

## Cost/Performance Impact

Using the recommended model selection for this 9-task plan:

| Model | Tasks | Estimated Tokens | Cost Reduction |
|-------|-------|------------------|----------------|
| Opus 4.5 | Validation only | ~50K | Baseline |
| Sonnet 4 | 002-007, 009 | ~200K | ~60% cheaper |
| Haiku 3.5 | 001, 008 | ~20K | ~90% cheaper |

**Total estimated savings:** 40-50% vs all-Opus implementation

**Risk mitigation:**
- Use Opus for validation to catch any Sonnet/Haiku errors
- Sonnet handles the bulk of implementation safely given structured plans
- Haiku only for truly mechanical tasks
