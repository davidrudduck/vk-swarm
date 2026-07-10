# Plan: Complete Tournament Round 2 for fix-nonloopback-signin

## Objective
Execute Round 2 of the adversarial tournament for the fix-nonloopback-signin workstream to achieve 2 consecutive clean rounds (0 valid issues) before PR creation.

## Current Status
- ✅ Round 1 completed with3 challengers (Opus, MiMo, GLM)
- ✅ Round 1 remediations applied and committed (commit 2b319169)
- ✅ Round 1 tournament summary written
- ✅ Round 2 analysis prompt prepared
- 🔄 Round 2 needs to be dispatched with 3 new challengers

## Step-by-Step Plan

### Step 1: Verify Round 1 Remediations (Already Done)
The Round 1 remediations have already been verified:
- `npm run test:run -- src/pkce.test.ts src/AppRouter.test.tsx src/pages/InvitationPage.test.tsx src/pages/InvitationCompletePage.test.tsx` — PASS 12/12
- `npm run test:invariants` — PASS 1/1
- `npx tsc --noEmit` — PASS

### Step 2: Create Proper Round 1 Tournament Report
The user requested the report at `.agents/reports/2026-07-08-round-1-tournament-results.md`, but we have it at `.agents/reports/2026-07-08-round-1-tournament-summary.md`. We need to:
1. Copy or rename the existing report to the requested location
2. Ensure it follows the standard tournament report format

### Step 3: Select 3 Random Challengers for Round 2
Pool: opus, gpt-5.5, deepseek-v4-pro, minimax-m3, mimo-v2.5-pro, glm-5.2, kimi-k2.7-code
Round 1 used: opus, mimo-v2.5-pro, glm-5.2
Available for Round 2: gpt-5.5, deepseek-v4-pro, minimax-m3, kimi-k2.7-code

Randomly select 3 from the available pool:
- Use a random selection method (e.g., random.org or simple algorithm)
- Document the selection process

### Step 4: Prepare Round 2 Tournament Directory
Create: `docs/plans/fix-nonloopback-signin/reviews/tournament-round-2/`
Structure:
- `find-prompt.md` (updated with Round 1 context)
- `challenger1-find.json` and `.md`
- `challenger2-find.json` and `.md`
- `challenger3-find.json` and `.md`
- `judge-assignments.json`
- `verdicts.json`
- `leaderboard.json`
- `tournament-record.md`

### Step 5: Dispatch Round 2 Challengers
For each selected challenger:
1. Use the appropriate subagent type:
   - gpt-5.5 → `subagent-chatgpt`
   - deepseek-v4-pro → `subagent-deepseek`
   - minimax-m3 → `subagent-minimax`
   - kimi-k2.7-code → `subagent-kimi`
2. Provide the analysis prompt (from `.agents/reports/round2-analysis-prompt.md`)
3. Have each write findings to `findings-<model>.json` in the tournament directory
4. Ensure findings follow the JSON contract: `{"model", "findings": [{id, severity, issue, citation, remediation}]}`

### Step 6: Adjudicate Round 2 Findings
1. Assign peer reviewers using `dr_tournament.py --mode assign`
2. Have peer reviewers validate findings and remediations
3. Have neutral decider (outside the roster) confirm verdicts
4. Collect verdicts into `verdicts.json`

### Step 7: Score Round 2
Run `dr_tournament.py --mode score` to generate:
- `leaderboard.json`
- `tournament-record.md`

### Step 8: Evaluate Round 2 Results
If Round 2 finds 0 valid issues:
- Tournament complete (2 consecutive clean rounds)
- Proceed to PR creation

If Round 2 finds valid issues:
- Remediate all validated findings
- Commit and push remediations
- Write Round 2 tournament report
- Start Round 3 (repeat Steps 3-8)

### Step 9: Write Final Tournament Report
Once 2 consecutive clean rounds are achieved:
- Write comprehensive tournament report to `.agents/reports/2026-07-08-tournament-final-results.md`
- Document all rounds, findings, remediations, and final status

## Verification Commands
After each round, verify:
```bash
cd remote-frontend && npm run test:run -- src/pkce.test.ts src/AppRouter.test.tsx src/pages/InvitationPage.test.tsx src/pages/InvitationCompletePage.test.tsx
cd remote-frontend && npm run test:invariants
cd remote-frontend && npx tsc --noEmit
cd remote-frontend && npm run lint
```

## Success Criteria
- 2 consecutive tournament rounds with 0 valid issues
- All remediations committed and pushed
- Tournament documentation complete
- Ready for PR creation

## Risk Mitigation
- If a challenger fails to run, select another from the pool
- If peer review is inconclusive, use neutral decider as tiebreaker
- If remediation breaks tests, revert and re-evaluate finding validity