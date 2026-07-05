// VK-Swarm UI kit — the remaining 7 settings panels (Projects, Organizations,
// Swarm, Agents, MCP, Webhooks, System). Each is a ({ draft, patch }) => cards
// of SettingsSection/SettingsRow. Registered on window.VKS_PANELS for
// settings.jsx to look up. Simple toggles/selects write to the shared draft so
// the dirty save-bar reacts; list add/remove use local ephemeral state.
const { useState: useStateP } = React;

function _sel(opts) { return opts.map((o) => (typeof o === 'string' ? { value: o, label: o } : o)); }
function DS() { return window.VKSwarmDesignSystem_067861; }

// ------------------------------------------------------------ Projects ------
function ProjectsPanel({ draft, patch }) {
  const { SettingsSection, SettingsRow, Select, Input, Switch, Checkbox } = DS();
  const p = draft.projects;
  const set = (k, v) => patch('projects', { [k]: v });
  return (
    <>
      <SettingsSection title="Worktrees" description="How isolated git worktrees are created for each attempt.">
        <SettingsRow label="Default base branch" htmlFor="p-branch" helper="Branch new worktrees are cut from.">
          <Input id="p-branch" mono value={p.defaultBranch} onChange={(e) => set('defaultBranch', e.target.value)} />
        </SettingsRow>
        <SettingsRow label="Worktree directory" htmlFor="p-wt" helper="Where worktrees live on each node.">
          <Input id="p-wt" mono value={p.worktreeBase} onChange={(e) => set('worktreeBase', e.target.value)} />
        </SettingsRow>
        <SettingsRow inline label="Clean up merged worktrees" htmlFor="p-clean" helper="Remove the worktree once its task reaches Done.">
          <Switch id="p-clean" checked={p.autoCleanup} onCheckedChange={(v) => set('autoCleanup', v)} />
        </SettingsRow>
        <SettingsRow inline label="Copy .env into worktree" htmlFor="p-env" helper="Copy the repo’s local env files into each worktree.">
          <Checkbox id="p-env" checked={p.copyEnv} onCheckedChange={(v) => set('copyEnv', v)} />
        </SettingsRow>
      </SettingsSection>
      <SettingsSection title="Setup script" description="Runs once after a worktree is created, before the agent starts.">
        <SettingsRow label="Command" htmlFor="p-setup" helper="e.g. install dependencies.">
          <textarea id="p-setup" className="vks-input vks-input--mono" rows={3} value={p.setupScript}
            onChange={(e) => set('setupScript', e.target.value)} style={{ height: 'auto' }} />
        </SettingsRow>
      </SettingsSection>
    </>
  );
}

// ------------------------------------------------------- Organizations ------
function OrganizationsPanel({ draft, patch }) {
  const { SettingsSection, SettingsRow, Select, Switch, Badge } = DS();
  const o = draft.organizations;
  const set = (k, v) => patch('organizations', { [k]: v });
  const members = [
    { name: 'david', hint: 'owner · you', role: 'Owner', variant: 'secondary' },
    { name: 'justX', hint: 'justX.raverx.net', role: 'Admin', variant: 'outline' },
    { name: 'ci-bot', hint: 'service account', role: 'Member', variant: 'outline' },
  ];
  return (
    <>
      <SettingsSection title="Organization" description="Settings apply to the selected organization.">
        <SettingsRow label="Active organization" htmlFor="o-org">
          <Select id="o-org" value={o.org} onValueChange={(v) => set('org', v)}
            options={_sel([{ value: 'raverx', label: 'raverx' }, { value: 'acme-labs', label: 'acme-labs' }])} />
        </SettingsRow>
      </SettingsSection>
      <SettingsSection title="Access" description="Defaults for new members and merges.">
        <SettingsRow label="Default member role" htmlFor="o-role">
          <Select id="o-role" value={o.defaultRole} onValueChange={(v) => set('defaultRole', v)}
            options={_sel([{ value: 'MEMBER', label: 'Member' }, { value: 'ADMIN', label: 'Admin' }, { value: 'VIEWER', label: 'Viewer' }])} />
        </SettingsRow>
        <SettingsRow inline label="Require review before merge" htmlFor="o-review" helper="A human must approve a diff before it can be merged.">
          <Switch id="o-review" checked={o.requireReview} onCheckedChange={(v) => set('requireReview', v)} />
        </SettingsRow>
        <SettingsRow inline label="Enforce SSO" htmlFor="o-sso" helper="Members must authenticate through your identity provider.">
          <Switch id="o-sso" checked={o.sso} onCheckedChange={(v) => set('sso', v)} />
        </SettingsRow>
      </SettingsSection>
      <SettingsSection title="Members" description="3 members in this organization.">
        <div className="vks-card" style={{ overflow: 'hidden' }}>
          {members.map((m, i) => (
            <div key={m.name} style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '10px 14px', borderBottom: i < members.length - 1 ? '1px solid var(--border)' : 0 }}>
              <span style={{ width: 28, height: 28, borderRadius: '50%', background: 'var(--surface-raised)', display: 'flex', alignItems: 'center', justifyContent: 'center', fontFamily: 'var(--font-code)', fontSize: 'var(--text-xs)', color: 'var(--text-muted)', flexShrink: 0 }}>{m.name.slice(0, 2)}</span>
              <span style={{ flex: 1, minWidth: 0 }}>
                <span style={{ display: 'block', fontFamily: 'var(--font-code)', fontSize: 'var(--text-sm)' }}>{m.name}</span>
                <span style={{ display: 'block', fontSize: 'var(--text-xs)', color: 'var(--text-dim)' }}>{m.hint}</span>
              </span>
              <Badge variant={m.variant}>{m.role}</Badge>
            </div>
          ))}
        </div>
      </SettingsSection>
    </>
  );
}

// -------------------------------------------------------------- Swarm -------
function SwarmPanel({ draft, patch }) {
  const { SettingsSection, SettingsRow, Select, Switch, Badge, Button } = DS();
  const s = draft.swarm;
  const set = (k, v) => patch('swarm', { [k]: v });
  const [labels, setLabels] = useStateP(['frontend', 'infra', 'urgent', 'agent:claude']);
  const [draftLabel, setDraftLabel] = useStateP('');
  const addLabel = () => { const v = draftLabel.trim(); if (v && !labels.includes(v)) setLabels((l) => [...l, v]); setDraftLabel(''); };
  return (
    <>
      <SettingsSection title="Swarm" description="Shared configuration synced across every node.">
        <SettingsRow label="Organization" htmlFor="sw-org">
          <Select id="sw-org" value={s.org} onValueChange={(v) => set('org', v)}
            options={_sel([{ value: 'raverx', label: 'raverx' }, { value: 'acme-labs', label: 'acme-labs' }])} />
        </SettingsRow>
        <SettingsRow inline label="Auto-sync" htmlFor="sw-sync" helper="Push shared projects, labels and templates to nodes as they change.">
          <Switch id="sw-sync" checked={s.autoSync} onCheckedChange={(v) => set('autoSync', v)} />
        </SettingsRow>
        <SettingsRow label="Conflict strategy" htmlFor="sw-conf" helper="What to do when a node’s local copy diverges.">
          <Select id="sw-conf" value={s.conflictStrategy} onValueChange={(v) => set('conflictStrategy', v)}
            options={_sel([{ value: 'MANUAL', label: 'Ask me' }, { value: 'HIVE_WINS', label: 'Hive wins' }, { value: 'NODE_WINS', label: 'Node wins' }])} />
        </SettingsRow>
      </SettingsSection>
      <SettingsSection title="Shared labels" description="Available on tasks across the whole swarm.">
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6 }}>
          {labels.map((l) => (
            <span key={l} style={{ display: 'inline-flex' }}>
              <Badge variant="outline">{l}
                <button onClick={() => setLabels((xs) => xs.filter((x) => x !== l))} aria-label={'Remove ' + l}
                  style={{ background: 'none', border: 0, color: 'var(--text-dim)', cursor: 'pointer', marginLeft: 6, padding: 0, lineHeight: 1 }}>×</button>
              </Badge>
            </span>
          ))}
        </div>
        <SettingsRow label="Add label" htmlFor="sw-newlabel">
          <div style={{ display: 'flex', gap: 8 }}>
            <div style={{ flex: 1 }}>
              <input id="sw-newlabel" className="vks-input vks-input--mono" placeholder="label name" value={draftLabel}
                onChange={(e) => setDraftLabel(e.target.value)} onKeyDown={(e) => e.key === 'Enter' && addLabel()} />
            </div>
            <Button variant="outline" size="sm" onClick={addLabel}>Add</Button>
          </div>
        </SettingsRow>
      </SettingsSection>
    </>
  );
}

// -------------------------------------------------------------- Agents ------
function AgentsPanel({ draft, patch }) {
  const { SettingsSection, SettingsRow, Select, Input, Checkbox, Button, Badge } = DS();
  const a = draft.agents;
  const set = (k, v) => patch('agents', { [k]: v });
  return (
    <>
      <SettingsSection title="Executor profiles" description="Per-agent configuration profiles used when running attempts.">
        <SettingsRow label="Agent" htmlFor="ag-exec">
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr auto', gap: 8, alignItems: 'end' }}>
            <Select id="ag-exec" value={a.executor} onValueChange={(v) => set('executor', v)}
              options={_sel([{ value: 'CLAUDE_CODE', label: 'Claude Code' }, { value: 'CODEX', label: 'Codex' }, { value: 'OPENCODE', label: 'OpenCode' }, { value: 'GEMINI', label: 'Gemini' }])} />
            <Select value={a.config} onValueChange={(v) => set('config', v)}
              options={_sel([{ value: 'DEFAULT', label: 'DEFAULT' }, { value: 'PLAN', label: 'PLAN' }, { value: 'YOLO', label: 'YOLO' }])} />
            <Button variant="destructive" size="sm"><window.Icon d={window.SICONS.trash} size={14} /></Button>
          </div>
        </SettingsRow>
        <SettingsRow inline label="Edit as raw JSON" htmlFor="ag-json" helper="Switch off the form editor to edit profiles.json directly.">
          <Checkbox id="ag-json" checked={!a.formEditor} onCheckedChange={(v) => set('formEditor', !v)} />
        </SettingsRow>

        {a.formEditor ? (
          <div className="vks-settings__body" style={{ gap: 'var(--space-4)', paddingTop: 4, borderTop: '1px dashed var(--border)' }}>
            <SettingsRow label="Model" htmlFor="ag-model">
              <Select id="ag-model" defaultValue="sonnet"
                options={_sel([{ value: 'sonnet', label: 'claude-sonnet-4.5' }, { value: 'opus', label: 'claude-opus-4' }, { value: 'haiku', label: 'claude-haiku-4' }])} />
            </SettingsRow>
            <SettingsRow label="Extra CLI args" htmlFor="ag-args" helper="Appended to the executor invocation.">
              <Input id="ag-args" mono defaultValue="--dangerously-skip-permissions" />
            </SettingsRow>
            <SettingsRow inline label="Sandbox" htmlFor="ag-sandbox" helper="Run the agent in a restricted filesystem sandbox.">
              <Checkbox id="ag-sandbox" defaultChecked />
            </SettingsRow>
          </div>
        ) : (
          <SettingsRow label="profiles.json" htmlFor="ag-raw" helper="~/.vk-swarm/profiles.json">
            <textarea id="ag-raw" className="vks-input vks-input--mono" rows={7} style={{ height: 'auto' }} defaultValue={'{\n  "executors": {\n    "CLAUDE_CODE": {\n      "DEFAULT": { "model": "claude-sonnet-4.5" }\n    }\n  }\n}'} />
          </SettingsRow>
        )}
      </SettingsSection>
    </>
  );
}

// ---------------------------------------------------------------- MCP -------
function McpPanel({ draft, patch }) {
  const { SettingsSection, SettingsRow, Select, Switch, Button, Badge } = DS();
  const m = draft.mcp;
  const set = (k, v) => patch('mcp', { [k]: v });
  const [servers, setServers] = useStateP([
    { id: 'fs', name: 'filesystem', cmd: 'npx @modelcontextprotocol/server-filesystem', on: true },
    { id: 'gh', name: 'github', cmd: 'npx @modelcontextprotocol/server-github', on: true },
    { id: 'pg', name: 'postgres', cmd: 'uvx mcp-server-postgres', on: false },
  ]);
  const toggle = (id) => setServers((s) => s.map((x) => x.id === id ? { ...x, on: !x.on } : x));
  return (
    <>
      <SettingsSection title="Model Context Protocol" description="MCP servers exposed to every agent in the swarm.">
        <SettingsRow label="Scope" htmlFor="mcp-scope" helper="Where these servers apply.">
          <Select id="mcp-scope" value={m.scope} onValueChange={(v) => set('scope', v)}
            options={_sel([{ value: 'GLOBAL', label: 'All projects' }, { value: 'PROJECT', label: 'This project only' }])} />
        </SettingsRow>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {servers.map((sv) => (
            <div key={sv.id} style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '12px 14px', border: '1px solid var(--border)', borderRadius: 'var(--radius-md)', background: 'var(--surface-card)' }}>
              <span style={{ minWidth: 0, flex: 1 }}>
                <span style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <span style={{ fontFamily: 'var(--font-code)', fontSize: 'var(--text-sm)', fontWeight: 500 }}>{sv.name}</span>
                  <Badge variant={sv.on ? 'secondary' : 'outline'} dot={sv.on}>{sv.on ? 'connected' : 'disabled'}</Badge>
                </span>
                <span style={{ display: 'block', fontFamily: 'var(--font-code)', fontSize: 'var(--text-xs)', color: 'var(--text-dim)', marginTop: 3, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{sv.cmd}</span>
              </span>
              <Switch checked={sv.on} onCheckedChange={() => toggle(sv.id)} />
            </div>
          ))}
        </div>
        <div>
          <Button variant="outline" size="sm"><window.Icon d={window.ICONS.plus} size={14} /> Add server</Button>
        </div>
      </SettingsSection>
    </>
  );
}

// ----------------------------------------------------------- Webhooks -------
function WebhooksPanel({ draft, patch }) {
  const { SettingsSection, SettingsRow, Select, Input, Switch, Checkbox, Button } = DS();
  const w = draft.webhooks;
  const set = (k, v) => patch('webhooks', { [k]: v });
  const events = [
    ['task.created', 'A task is added to the board'],
    ['attempt.finished', 'An agent attempt completes'],
    ['review.requested', 'A diff enters In Review'],
    ['task.merged', 'A task is merged and closed'],
  ];
  return (
    <>
      <SettingsSection title="Delivery" description="Send swarm events to an external endpoint.">
        <SettingsRow inline label="Enable webhooks" htmlFor="wh-on" helper="Deliver events over HTTPS POST.">
          <Switch id="wh-on" checked={w.enabled} onCheckedChange={(v) => set('enabled', v)} />
        </SettingsRow>
        <SettingsRow label="Endpoint URL" htmlFor="wh-url" helper="Receives a signed JSON payload per event.">
          <Input id="wh-url" mono defaultValue="https://hooks.raverx.net/vk-swarm" disabled={!w.enabled} />
        </SettingsRow>
        <SettingsRow label="Signing secret" htmlFor="wh-secret" helper="Used to verify the X-VK-Signature header.">
          <div style={{ display: 'flex', gap: 8 }}>
            <div style={{ flex: 1 }}><Input id="wh-secret" mono type="password" defaultValue="whsec_9a3f2c1b7e" disabled={!w.enabled} /></div>
            <Button variant="outline" size="sm" disabled={!w.enabled}><window.Icon d={window.SICONS.refresh} size={14} /> Rotate</Button>
          </div>
        </SettingsRow>
        <SettingsRow label="Retry attempts" htmlFor="wh-retry" helper="Failed deliveries are retried with backoff.">
          <Select id="wh-retry" value={w.retry} onValueChange={(v) => set('retry', v)}
            options={_sel([{ value: '0', label: 'No retries' }, { value: '3', label: '3 attempts' }, { value: '5', label: '5 attempts' }])} />
        </SettingsRow>
      </SettingsSection>
      <SettingsSection title="Events" description="Which events trigger a delivery.">
        {events.map(([id, desc], i) => (
          <SettingsRow key={id} inline label={<span style={{ fontFamily: 'var(--font-code)', fontSize: 'var(--text-sm)' }}>{id}</span>} htmlFor={'wh-ev-' + i} helper={desc}>
            <Checkbox id={'wh-ev-' + i} defaultChecked={i < 3} disabled={!w.enabled} />
          </SettingsRow>
        ))}
      </SettingsSection>
    </>
  );
}

// -------------------------------------------------------------- System ------
function SystemPanel({ draft, patch }) {
  const { SettingsSection, SettingsRow, Select, Switch, Button, Badge } = DS();
  const sy = draft.system;
  const set = (k, v) => patch('system', { [k]: v });
  return (
    <>
      <SettingsSection title="Backups" description="Snapshots of the hive database."
        footer={<Button variant="outline" size="sm"><window.Icon d={window.SICONS.database} size={14} /> Back up now</Button>}>
        <SettingsRow inline label="Automatic backups" htmlFor="sy-auto" helper="Snapshot the hive on a schedule.">
          <Switch id="sy-auto" checked={sy.autoBackup} onCheckedChange={(v) => set('autoBackup', v)} />
        </SettingsRow>
        {sy.autoBackup && (
          <SettingsRow nested label="Interval" htmlFor="sy-int">
            <Select id="sy-int" value={sy.backupInterval} onValueChange={(v) => set('backupInterval', v)}
              options={_sel([{ value: 'HOURLY', label: 'Hourly' }, { value: 'DAILY', label: 'Daily' }, { value: 'WEEKLY', label: 'Weekly' }])} />
          </SettingsRow>
        )}
        <SettingsRow label="Retention" htmlFor="sy-ret" helper="Older snapshots are pruned.">
          <Select id="sy-ret" value={sy.retention} onValueChange={(v) => set('retention', v)}
            options={_sel([{ value: '7', label: '7 days' }, { value: '30', label: '30 days' }, { value: '90', label: '90 days' }])} />
        </SettingsRow>
      </SettingsSection>

      <SettingsSection title="Hive sync" description="Connection to the central hive.">
        <div style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '12px 14px', border: '1px solid var(--border)', borderRadius: 'var(--radius-md)' }}>
          <span className="vks-node__pulse" style={{ flexShrink: 0 }} />
          <span style={{ flex: 1, minWidth: 0 }}>
            <span style={{ display: 'block', fontSize: 'var(--text-sm)', fontWeight: 500 }}>Connected · wss://hive.raverx.net</span>
            <span style={{ display: 'block', fontFamily: 'var(--font-code)', fontSize: 'var(--text-xs)', color: 'var(--text-dim)', marginTop: 2 }}>4 nodes synced · last sync 12s ago</span>
          </span>
          <Badge variant="secondary" dot>online</Badge>
        </div>
      </SettingsSection>

      <SettingsSection title="Build info" description="This VK-Swarm instance.">
        <div style={{ display: 'grid', gridTemplateColumns: 'auto 1fr', rowGap: 8, columnGap: 16, fontSize: 'var(--text-sm)' }}>
          {[['Version', 'v0.7.3'], ['Commit', '648692a'], ['Rust', '1.83.0'], ['Node registry', '4 nodes']].map(([k, v]) => (
            <React.Fragment key={k}>
              <span style={{ color: 'var(--text-muted)' }}>{k}</span>
              <span style={{ fontFamily: 'var(--font-code)', fontSize: 'var(--text-xs)', color: 'var(--foreground)' }}>{v}</span>
            </React.Fragment>
          ))}
        </div>
        <SettingsRow inline label="Send anonymous telemetry" htmlFor="sy-tel" helper="Share crash reports and usage metrics to improve VK-Swarm.">
          <Switch id="sy-tel" checked={sy.telemetry} onCheckedChange={(v) => set('telemetry', v)} />
        </SettingsRow>
      </SettingsSection>
    </>
  );
}

window.VKS_PANELS = {
  projects: ProjectsPanel,
  organizations: OrganizationsPanel,
  swarm: SwarmPanel,
  agents: AgentsPanel,
  mcp: McpPanel,
  webhooks: WebhooksPanel,
  system: SystemPanel,
};
