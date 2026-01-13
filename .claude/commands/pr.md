Create a PR to merge this branch into origin main.

## STEP 0: SQUASH COMMITS (MANDATORY)

**Before creating PR, you MUST squash commits into logical units.**

### Why Squash?

Multiple commits with messages like "Task 001 complete" or development thoughts pollute git history.
**Goal:** 2-5 commits maximum, each representing a logical change with professional commit messages.

### Squash Process

**1. Review current commits:**
```bash
git log --oneline origin/main..HEAD
```

**2. Count commits to squash:**
```bash
COMMIT_COUNT=$(git rev-list --count origin/main..HEAD)
echo "Commits to squash: $COMMIT_COUNT"
```

**3. Interactive rebase:**
```bash
git rebase -i origin/main
```

**4. In the editor:**
- Keep first commit as `pick`
- Change all others to `squash` (or `s`)
- Save and exit

**5. Write new commit messages:**

Use conventional commits format for each logical unit:

**Example for feature implementation:**
```bash
feat: add custom template loading to TaskFollowUpSection

- Fetch custom templates from API when picker opens
- Add loading state and error handling
- Transform API response to PickerTemplate format
- Pass customTemplates, loading, and error props to TemplatePicker

Fixes template picker only showing system templates in follow-up section.
```

**Example for documentation:**
```bash
docs: update task-templates.mdx for follow-up section support

Note that template picker is available in both task creation modal
and task follow-up section.
```

**6. Verify squash:**
```bash
git log --oneline origin/main..HEAD
# Should show 2-5 commits max, all with professional messages
```

**7. Force push (branch has already been pushed):**
```bash
git push --force-with-lease
```

**ONLY proceed to PR creation after squashing is verified.**

---

## STEP 1: READ CONTEXT

- Read vks_progress.md
- Read the outline of the plan
- Read the validation report $TASKS/validation.md
- Read the git commits for this worktree (AFTER squashing)

---

## STEP 2: CREATE PR

- Create a detailed summary as below
- Confirm there are no conflicts after submitting the PR
- Address any merge conflicts
- .vk_progress.md and vks-progress.md should not be committed
- logs/ and .playwright-mcp/ should not be committed
- any example images used as part of the PR should be stored as .github/.images/[PR#]/[filename] and linked in the PR from that path

---

## STEP 3: STOP SERVICES

- Stop the services using the registered ports
  **Use `mcp__vkswarm__get_variables` to identify the PORTS used by this service**

---

= PR Summary =

# Summary 
- [Summarise the work completed]

# Key Features
- [detail the key features of the work completed]

# How It Works
- [detail how a user can use this feature]

# Files Changed
- [Details of file changes made]

# Bugs Fixed
- [bugs fixed]

# Testing
- [testing performed and results]
- [include screenshots from tests]

# Validation
- [validation results]
