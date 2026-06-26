export const meta = {
  name: 'vk-swarm-execute',
  description: 'WAI execute-tasks adapted for vk-swarm Rust/SQLx: DATABASE_URL injection, Done-when-driven gate',
  phases: [
    { title: 'Plan', detail: 'spec-freshness + plan-lint + load ready tasks in dependency order' },
    { title: 'Implement', detail: 'constrained implementer per task (Haiku → Codex → Opus escalation)' },
    { title: 'Validate', detail: 'Stage-1 gate reads Done-when for WAI_TYPECHECK_CMD/TEST_CMD + adversarial panel' },
  ],
}

// Fork of ~/.claude/plugins/cache/agent-plugins/wai/0.17.0/workflows/execute-tasks.mjs
// adapted for this Rust/SQLx workspace. Key diffs:
//   (a) DATABASE_URL + `sqlx migrate run` injected before every cargo step
//   (b) Gate agent reads task's "## Done when" to extract WAI_TYPECHECK_CMD/WAI_TEST_CMD
//       (the upstream gate auto-detects tsgo/vitest which is wrong for Rust .rs scope files)
//   (c) Implementer uses `git add … && git commit` — no `scripts/committer` in this repo
// Everything else (schemas, circuit breaker, adversarial panel, evidence check) is identical
// to execute-tasks.mjs.

const repoRoot = (args && args.repoRoot) || '/home/david/Code/vk-swarm'
const MAX_RETRIES = (args && args.maxRetries) || 3
// DATABASE_URL for the shared canonical dev DB (fresh, all migrations applied through
// 20260131000000_add_webhooks). Pre-seeded in .env as a commented precondition; we always
// set it explicitly here so cargo sqlx compile-time checks work for new query macros.
const DB_URL = 'sqlite:///home/david/Code/vk-swarm/dev_assets/db.sqlite'
// WAI scripts live in the plugin cache — ${WAI_SCRIPTS}/ does NOT exist on this
// machine (only hooks/ and launcher.active are there). Use the full path everywhere.
const WAI_SCRIPTS = '/home/david/.claude/plugins/cache/agent-plugins/wai/0.17.0/scripts'
const shq = (s) => `'${String(s).replace(/'/g, `'\\''`)}'`

// Topic: allow args.topic override; default to this plan.
let topic = (args && typeof args.topic === 'string' && args.topic.trim()) || 'vk-swarm-node-foundations'
if (!topic) {
  throw new Error('wf-execute: topic is empty. Pass args:{topic,repoRoot} or accept the default.')
}
const topicQ = shq(topic)

// ---- Schemas (identical to execute-tasks.mjs) ----

const TASK_LIST_SCHEMA = {
  type: 'object',
  required: ['tasks'],
  properties: {
    tasks: {
      type: 'array',
      items: {
        type: 'object',
        required: ['id', 'phase', 'file', 'status', 'irreversible'],
        properties: {
          id: { type: 'string' },
          phase: { type: 'number' },
          file: { type: 'string' },
          status: { type: 'string' },
          irreversible: { type: 'boolean' },
          depends_on: { type: 'array', items: { type: 'string' } },
        },
      },
    },
  },
}

const IMPL_SCHEMA = {
  type: 'object',
  required: ['outcome'],
  properties: {
    outcome: { type: 'string', enum: ['committed', 'stopped'] },
    commit: { type: 'string' },
    stopReason: { type: 'string' },
    ledgerLinesAdded: { type: 'number' },
    filesChanged: { type: 'array', items: { type: 'string' } },
  },
}

const GATE_SCHEMA = {
  type: 'object',
  required: ['conforms'],
  properties: {
    conforms: { type: 'boolean' },
    stderr: { type: 'string' },
  },
}

const VERDICT_SCHEMA = {
  type: 'object',
  required: ['verdict'],
  properties: {
    verdict: { type: 'string', enum: ['CONFORMS', 'DEVIATES'] },
    findings: {
      type: 'array',
      items: {
        type: 'object',
        required: ['claim', 'citation'],
        properties: { claim: { type: 'string' }, citation: { type: 'string' }, fix: { type: 'string' } },
      },
    },
  },
}

// ---- Implementer rules ----
// Cache-friendly: stable prefix first; variable task+history is the suffix per dispatch.
const IMPL_RULES =
  `You are a CONSTRAINED IMPLEMENTER for a Rust/Axum/SQLx Cargo workspace. Execute the given ` +
  `task file EXACTLY and nothing more. WIN CONDITION: a clean gate pass with an EMPTY ` +
  `decisions-ledger entry; every undictated choice loses points and will be hunted by an ` +
  `adversarial panel.\n` +
  `PRE-FLIGHT (do this BEFORE any edit; emit the evidence in your result): for EACH file in ` +
  `the task's \`files:\`, run \`grep -n '<anchor>' <file>\` (or Read the file) and confirm the ` +
  `task's Before-text matches the file byte-for-byte at that anchor. If ANY anchor / symbol / ` +
  `path is absent or differs from the task file → return outcome:"stopped" NOW with the ` +
  `grep/read output as stopReason. Do NOT edit anything until every anchor is confirmed (a ` +
  `half-applied edit is worse than a STOP).\n` +
  `RULES (non-negotiable): touch ONLY the files in the task's \`files:\`; make ONLY the ` +
  `described change; if a test fails for an unpredicted reason, or you must choose something ` +
  `the task did not specify → STOP (outcome:"stopped", stopReason). Write the failing test ` +
  `first if specified, then the minimal change. To commit:\n` +
  `  git add <each file from task files:>\n` +
  `  git commit -m "wai(<id>): <title from task frontmatter>"\n` +
  `  capture: git rev-parse HEAD\n` +
  `Log any undictated choice to docs/plans/${topic}/decisions-ledger.md under "## Per-task ` +
  `decisions". Report outcome, commit sha, filesChanged, ledgerLinesAdded. Report TERSELY ` +
  `(caveman style): fragments OK; file paths, test results, command output, stopReason EXACT.\n` +
  `\n` +
  `RUST/SQLX PRECONDITION — run BEFORE any cargo check or cargo test command:\n` +
  `  1. export DATABASE_URL='${DB_URL}'\n` +
  `  2. cd '${repoRoot}' && sqlx migrate run --source crates/db/migrations\n` +
  `This applies any new migration files committed by earlier tasks to the dev DB so sqlx ` +
  `compile-time query checks work. The dev DB at that path is isolated from production and ` +
  `safe to migrate. Do NOT run cargo sqlx prepare (it rewrites tracked .sqlx/ files the ` +
  `gate rejects — Trap 2 in docs/plans/${topic}/decisions-ledger.md). See also the task ` +
  `file's "## Done when" section for the exact WAI_TYPECHECK_CMD and WAI_TEST_CMD to run.`

// Implementer escalation by attempt — diversity on attempt 2, strength on attempt 3.
const IMPL_DISPATCH = [
  { model: 'haiku' },            // attempt 1: fast literal executor
  { agentType: 'codex:codex-rescue' }, // attempt 2: different model family
  { model: 'opus' },             // attempt 3: strongest model
]

// ---- Preflight ----
phase('Plan')

const fresh = await agent(
  `Run \`bash ${WAI_SCRIPTS}/wai-freshness.sh ${topicQ}\` from ${shq(repoRoot)}. ` +
    `Return ok=true iff it exits 0; include its stderr. Do not edit anything.`,
  { label: `freshness:${topic}`, phase: 'Plan', schema: { type: 'object', required: ['ok'], properties: { ok: { type: 'boolean' }, stderr: { type: 'string' } } } },
).catch(() => null)
if (!fresh || !fresh.ok) {
  throw new Error(
    `wf-execute: spec freshness FAILED. ${fresh ? fresh.stderr : 'no result'} ` +
      `Do NOT edit the spec to proceed; re-run /wai:precheck only to deliberately re-freeze.`,
  )
}

const planlint = await agent(
  `Run \`bash ${WAI_SCRIPTS}/wai-plan-lint.sh ${topicQ}\` from ${shq(repoRoot)}. ` +
    `Return ok=true iff it exits 0; include its stderr. Do not edit anything.`,
  { label: `plan-lint:${topic}`, phase: 'Plan', schema: { type: 'object', required: ['ok'], properties: { ok: { type: 'boolean' }, stderr: { type: 'string' } } } },
).catch(() => null)
if (!planlint || !planlint.ok) {
  throw new Error(
    `wf-execute: plan-lint FAILED. ${planlint ? planlint.stderr : 'no result'} ` +
      `Fix the cited mismatch in docs/plans/${topic}/ before executing.`,
  )
}

const plan = await agent(
  `Run \`bash ${WAI_SCRIPTS}/task-status.sh ${topicQ}\` from ${shq(repoRoot)}. ` +
    `Return ALL ready tasks (status "ready", not passed/blocked/rejected) sorted in topological ` +
    `dependency order: if task B depends_on task A, A must appear before B — even if A is also ` +
    `still "ready". Do NOT filter out tasks whose depends_on tasks are also "ready". For each ` +
    `task return id, phase, file path, status, irreversible (bool), depends_on. Do not implement.`,
  { label: `plan:${topic}`, phase: 'Plan', schema: TASK_LIST_SCHEMA },
)

const tasks = (plan && plan.tasks) || []
log(`${tasks.length} ready task(s) for topic ${topic}`)

// ---- Per-task loop ----
const results = []
for (const task of tasks) {
  // Human gate: irreversible tasks are not run unattended.
  if (task.irreversible) {
    log(
      `🚧 HUMAN GATE: task ${task.id} is irreversible — skipping in unattended run. ` +
        `Create docs/plans/${topic}/reviews/${task.id}.approved after reviewing the diff, then re-run.`,
    )
    results.push({ id: task.id, outcome: 'human-gate-skipped' })
    continue
  }

  let passed = false
  let attempt = 0
  const errorHistory = []
  const recordError = (stage, reason) => errorHistory.push(`[attempt ${attempt} ${stage}] ${reason}`)

  while (!passed && attempt < MAX_RETRIES) {
    attempt++

    const errorDigest = errorHistory.length
      ? `\n\nPRIOR ATTEMPTS (oldest first) — address the PATTERN, not just the last line:\n` +
        errorHistory.map((e) => `  - ${e}`).join('\n')
      : ''

    const dispatch = IMPL_DISPATCH[Math.min(attempt - 1, IMPL_DISPATCH.length - 1)]

    // --- Implement (constrained, fresh context) ---
    const impl = await agent(
      `${IMPL_RULES}\n\nTASK FILE: ${task.file}\nLEDGER: docs/plans/${topic}/decisions-ledger.md${errorDigest}`,
      { label: `impl:${task.id}#${attempt}`, phase: 'Implement', schema: IMPL_SCHEMA, ...dispatch },
    )

    if (!impl || impl.outcome === 'stopped') {
      recordError('impl', `STOPPED: ${impl ? impl.stopReason : 'no result'}`)
      log(`task ${task.id} attempt ${attempt}: implementer STOPPED — orchestrator must resolve; halting task`)
      break
    }

    // --- Stage 1: deterministic gate ---
    // Reads Done-when from the task file to extract WAI_TYPECHECK_CMD/WAI_TEST_CMD
    // (the upstream gate's fallback detection tries vitest/node for .rs files — wrong).
    // Also applies pending migrations before cargo so new query macros can be validated.
    const commitRef = shq(impl.commit || 'HEAD')
    const gate = await agent(
      `WAI Stage-1 gate for task ${task.id} (topic: ${topic}, commit: ${impl.commit || 'HEAD'}).\n` +
        `Follow these steps in order:\n` +
        `1. Read the task file: ${task.file}\n` +
        `2. Find the "## Done when" section. Locate the backtick code block — it looks like:\n` +
        `   \`WAI_TYPECHECK_CMD="..." WAI_TEST_CMD="..." bash ${WAI_SCRIPTS}/task-gate.sh ...\`\n` +
        `   Extract the WAI_TYPECHECK_CMD value (everything between its quotes).\n` +
        `   Extract the WAI_TEST_CMD value if present (some tasks omit it when scope_test is N/A).\n` +
        `3. Apply pending migrations (idempotent — safe to run even if already applied):\n` +
        `   cd '${repoRoot}' && DATABASE_URL='${DB_URL}' sqlx migrate run --source crates/db/migrations\n` +
        `4. Run the gate with the extracted values:\n` +
        `   cd '${repoRoot}' && DATABASE_URL='${DB_URL}' \\\n` +
        `     WAI_TYPECHECK_CMD="<extracted>" \\\n` +
        `     WAI_TEST_CMD="<extracted or omit if absent>" \\\n` +
        `     bash ${WAI_SCRIPTS}/task-gate.sh ${topicQ} ${shq(task.id)} --commit ${commitRef}\n` +
        `   (If WAI_TEST_CMD was absent from Done-when, omit the WAI_TEST_CMD= prefix entirely.)\n` +
        `5. Return conforms=true iff exit code is 0; include the full stderr verbatim. Do NOT fix anything.`,
      { label: `gate:${task.id}#${attempt}`, phase: 'Validate', schema: GATE_SCHEMA },
    )

    if (!gate || !gate.conforms) {
      recordError('gate', `Stage-1 REJECT: ${gate ? gate.stderr : 'no result'}`)
      log(`task ${task.id} attempt ${attempt}: Stage-1 gate REJECT`)
      continue
    }

    // --- Stage 2: adversarial panel ---
    // Model diversity escalates with attempt (Opus at 1, +Codex at 2, +Gemini at 3).
    const panelModels =
      attempt <= 1
        ? [{ t: undefined }]
        : attempt === 2
          ? [{ t: undefined }, { t: 'codex:codex-rescue' }]
          : [{ t: undefined }, { t: 'codex:codex-rescue' }, { t: 'cc-gemini-plugin:gemini-agent' }]

    const verdicts = await parallel(
      panelModels.map((p, i) => () =>
        agent(
          `ADVERSARIAL REVIEW of WAI task ${task.id} (commit ${impl.commit || 'HEAD'}) in ${repoRoot}. ` +
            `Try to PROVE the implementation deviated from the task file ${task.file}. Every finding MUST ` +
            `cite the exact command-output line proving it (run git show --name-status --find-renames, ` +
            `git show, git ls-tree, and the task's scope). Uncited findings are discarded. Return verdict ` +
            `CONFORMS or DEVIATES. If DEVIATES, include at least one finding with a non-empty citation — ` +
            `a DEVIATES verdict with no findings is treated as CONFORMS. ` +
            `Be terse (caveman style); cited command-output lines stay EXACT.`,
          { label: `panel:${task.id}#${attempt}.${i}`, phase: 'Validate', agentType: p.t, schema: VERDICT_SCHEMA },
        ).catch(() => null),
      ),
    )

    const validVerdicts = verdicts.filter(Boolean)
    if (validVerdicts.length === 0) {
      recordError('panel', 'all panel agents returned null — model error or schema rejection; NOT an impl rejection')
      log(`task ${task.id} attempt ${attempt}: panel FAIL — all agents null; halting for human check`)
      break
    }

    const citedDissent = validVerdicts
      .filter((v) => v.verdict === 'DEVIATES')
      .flatMap((v) => (v.findings || []).filter((f) => f.citation && f.citation.trim().length > 0))

    const uncitedDeviates = validVerdicts
      .filter((v) => v.verdict === 'DEVIATES')
      .filter((v) => !(v.findings || []).some((f) => f.citation && f.citation.trim().length > 0))
    if (uncitedDeviates.length > 0) {
      log(
        `task ${task.id} attempt ${attempt}: ${uncitedDeviates.length} DEVIATES verdict(s) with no ` +
          `cited findings — discarded per policy (uncited dissent is not evidence)`,
      )
    }

    if (citedDissent.length > 0) {
      recordError('panel', `cited dissent: ${citedDissent.map((f) => f.claim).join(' | ')}`)
      log(`task ${task.id} attempt ${attempt}: panel REJECT — ${citedDissent.map((f) => f.claim).join(' | ')}`)
      continue
    }

    passed = true
    log(`task ${task.id}: PASSED (attempt ${attempt}, ledger lines: ${impl.ledgerLinesAdded || 0})`)
    results.push({ id: task.id, outcome: 'passed', commit: impl.commit, attempts: attempt })
  }

  if (!passed) {
    const lastReason = errorHistory[errorHistory.length - 1] || 'no attempts ran'
    results.push({ id: task.id, outcome: 'halted', attempts: attempt, lastReason, errorHistory })
    log(
      `task ${task.id}: HALTED after ${attempt} attempt(s). History:\n` +
        errorHistory.map((e) => '  ' + e).join('\n'),
    )
    log(`Stopping run — orchestrator/human must resolve task ${task.id} before continuing.`)
    break
  }
}

// ---- Evidence postcheck (SC#10) ----
// For a behaviour spec, the ledger must carry a `## Reachability gate` section.
// The orchestrator/human writes that section; here we only warn (fail-closed lives in /wai:close).
const evidence = await agent(
  `Run \`bash ${WAI_SCRIPTS}/wai-evidence.sh ${topicQ}\` from ${shq(repoRoot)}. ` +
    `Return ok=true iff it exits 0; include its stderr.`,
  {
    label: `evidence:${topic}`,
    phase: 'Validate',
    schema: { type: 'object', required: ['ok'], properties: { ok: { type: 'boolean' }, stderr: { type: 'string' } } },
  },
).catch(() => null)

if (!evidence || !evidence.ok) {
  log(
    `⚠ evidence gate: ${evidence ? evidence.stderr : 'no result'} ` +
      `— record the '## Reachability gate' section in the decisions-ledger before /wai:close`,
  )
}

return {
  topic,
  ran: results.length,
  passed: results.filter((r) => r.outcome === 'passed').length,
  halted: results.filter((r) => r.outcome === 'halted').length,
  humanGate: results.filter((r) => r.outcome === 'human-gate-skipped').length,
  results,
}
