-- Comprehensive seed script for E2E testing against the real Docker environment
-- Creates: users, orgs, projects, tasks, nodes, API keys, labels, assignments

-- =============================================================================
-- USERS
-- =============================================================================

INSERT INTO users (id, email, first_name, last_name, username) VALUES
    ('00000000-0000-0000-0000-000000000001', 'admin@test.local', 'Admin', 'User', 'adminuser'),
    ('00000000-0000-0000-0000-000000000011', 'member@test.local', 'Member', 'User', 'memberuser')
ON CONFLICT (email) DO NOTHING;

-- =============================================================================
-- ORGANIZATIONS
-- =============================================================================

INSERT INTO organizations (id, name, slug, is_personal) VALUES
    ('00000000-0000-0000-0000-000000000002', 'E2E Test Org', 'e2e-test-org', true)
ON CONFLICT (slug) DO NOTHING;

-- =============================================================================
-- ORG MEMBERSHIPS
-- =============================================================================

INSERT INTO organization_member_metadata (organization_id, user_id, role) VALUES
    ('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000001', 'admin'),
    ('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000011', 'member')
ON CONFLICT (organization_id, user_id) DO NOTHING;

-- =============================================================================
-- OAUTH ACCOUNTS (fake GitHub)
-- =============================================================================

INSERT INTO oauth_accounts (id, user_id, provider, provider_user_id, email, username, display_name) VALUES
    ('00000000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-000000000001', 'github', 'admin-github-001', 'admin@test.local', 'adminuser', 'Admin User'),
    ('00000000-0000-0000-0000-000000000013', '00000000-0000-0000-0000-000000000011', 'github', 'member-github-001', 'member@test.local', 'memberuser', 'Member User')
ON CONFLICT (provider, provider_user_id) DO NOTHING;

-- =============================================================================
-- NODE API KEYS
-- =============================================================================

INSERT INTO node_api_keys (id, organization_id, name, key_hash, key_prefix, created_by) VALUES
    ('00000000-0000-0000-0000-000000000004', '00000000-0000-0000-0000-000000000002', 'E2E Primary Key',
     encode(sha256('vk_e2e-primary-key-12345678'::bytea), 'hex'), 'vk_e2e-p',
     '00000000-0000-0000-0000-000000000001'),
    ('00000000-0000-0000-0000-000000000005', '00000000-0000-0000-0000-000000000002', 'E2E Revocable Key',
     encode(sha256('vk_e2e-revocable-key-abcdef'::bytea), 'hex'), 'vk_e2e-r',
     '00000000-0000-0000-0000-000000000001'),
    ('00000000-0000-0000-0000-000000000006', '00000000-0000-0000-0000-000000000002', 'E2E Blocked Key',
     encode(sha256('vk_e2e-blocked-key-xyz'::bytea), 'hex'), 'vk_e2e-b',
     '00000000-0000-0000-0000-000000000001')
ON CONFLICT DO NOTHING;

-- Block the third key
UPDATE node_api_keys SET blocked_at = NOW(), blocked_reason = 'E2E test: rate limit exceeded'
WHERE id = '00000000-0000-0000-0000-000000000006' AND blocked_at IS NULL;

-- =============================================================================
-- NODES (simulated connected nodes)
-- =============================================================================

INSERT INTO nodes (id, organization_id, name, machine_id, hostname, os_info, ip_address, status, last_heartbeat_at) VALUES
    ('00000000-0000-0000-0000-000000000007', '00000000-0000-0000-0000-000000000002', 'e2e-node-alpha', 'machine-alpha-001', 'alpha.local', 'Linux x86_64', '192.168.1.10', 'online', NOW()),
    ('00000000-0000-0000-0000-000000000008', '00000000-0000-0000-0000-000000000002', 'e2e-node-beta', 'machine-beta-001', 'beta.local', 'macOS arm64', '192.168.1.11', 'online', NOW()),
    ('00000000-0000-0000-0000-000000000009', '00000000-0000-0000-0000-000000000002', 'e2e-node-gamma', 'machine-gamma-001', 'gamma.local', 'Windows x86_64', '192.168.1.12', 'offline', NOW() - INTERVAL '2 hours')
ON CONFLICT DO NOTHING;

-- =============================================================================
-- SHARED TASKS (the main task list)
-- =============================================================================

INSERT INTO shared_tasks (id, organization_id, title, body, status, priority, created_by) VALUES
    ('00000000-0000-0000-0000-000000000020', '00000000-0000-0000-0000-000000000002', 'Implement login page', 'Build the OAuth login flow with GitHub and Google', 'done', 'high', '00000000-0000-0000-0000-000000000001'),
    ('00000000-0000-0000-0000-000000000021', '00000000-0000-0000-0000-000000000002', 'Add API key management', 'CRUD interface for node API keys', 'in-progress', 'high', '00000000-0000-0000-0000-000000000001'),
    ('00000000-0000-0000-0000-000000000022', '00000000-0000-0000-0000-000000000002', 'Write unit tests', 'Add test coverage for mutation guards', 'todo', 'medium', '00000000-0000-0000-0000-000000000001'),
    ('00000000-0000-0000-0000-000000000023', '00000000-0000-0000-0000-000000000002', 'Fix dialog accessibility', 'Replace custom dialog with Radix', 'in-review', 'high', '00000000-0000-0000-0000-000000000001'),
    ('00000000-0000-0000-0000-000000000024', '00000000-0000-0000-0000-000000000002', 'Deploy to staging', 'Push latest to staging environment', 'todo', 'low', '00000000-0000-0000-0000-000000000001'),
    ('00000000-0000-0000-0000-000000000025', '00000000-0000-0000-0000-000000000002', 'Update documentation', 'Sync docs with latest API changes', 'cancelled', 'low', '00000000-0000-0000-0000-000000000001')
ON CONFLICT DO NOTHING;

-- =============================================================================
-- NODE TASK ASSIGNMENTS (kanban board data)
-- =============================================================================

INSERT INTO node_task_assignments (id, task_id, node_id, organization_id, execution_status) VALUES
    ('00000000-0000-0000-0000-000000000030', '00000000-0000-0000-0000-000000000021', '00000000-0000-0000-0000-000000000007', '00000000-0000-0000-0000-000000000002', 'in_progress'),
    ('00000000-0000-0000-0000-000000000031', '00000000-0000-0000-0000-000000000022', '00000000-0000-0000-0000-000000000008', '00000000-0000-0000-0000-000000000002', 'pending'),
    ('00000000-0000-0000-0000-000000000032', '00000000-0000-0000-0000-000000000020', '00000000-0000-0000-0000-000000000007', '00000000-0000-0000-0000-000000000002', 'completed'),
    ('00000000-0000-0000-0000-000000000033', '00000000-0000-0000-0000-000000000023', '00000000-0000-0000-0000-000000000009', '00000000-0000-0000-0000-000000000002', 'failed')
ON CONFLICT DO NOTHING;

-- =============================================================================
-- LABELS
-- =============================================================================

INSERT INTO labels (id, organization_id, name, color) VALUES
    ('00000000-0000-0000-0000-000000000040', '00000000-0000-0000-0000-000000000002', 'bug', '#ef4444'),
    ('00000000-0000-0000-0000-000000000041', '00000000-0000-0000-0000-000000000002', 'feature', '#22c55e'),
    ('00000000-0000-0000-0000-000000000042', '00000000-0000-0000-0000-000000000002', 'docs', '#3b82f6'),
    ('00000000-0000-0000-0000-000000000043', '00000000-0000-0000-0000-000000000002', 'urgent', '#f97316')
ON CONFLICT DO NOTHING;

-- Assign labels to tasks
INSERT INTO shared_task_labels (task_id, label_id) VALUES
    ('00000000-0000-0000-0000-000000000020', '00000000-0000-0000-0000-000000000041'),
    ('00000000-0000-0000-0000-000000000021', '00000000-0000-0000-0000-000000000041'),
    ('00000000-0000-0000-0000-000000000021', '00000000-0000-0000-0000-000000000043'),
    ('00000000-0000-0000-0000-000000000022', '00000000-0000-0000-0000-000000000040'),
    ('00000000-0000-0000-0000-000000000023', '00000000-0000-0000-0000-000000000041')
ON CONFLICT DO NOTHING;

-- =============================================================================
-- NODE TASK OUTPUT LOGS (for log viewer testing)
-- =============================================================================

INSERT INTO node_task_output_logs (id, assignment_id, node_id, stream, chunk_index, content) VALUES
    ('00000000-0000-0000-0000-000000000050', '00000000-0000-0000-0000-000000000030', '00000000-0000-0000-0000-000000000007', 'stdout', 0, 'Starting API key management implementation...'),
    ('00000000-0000-0000-0000-000000000051', '00000000-0000-0000-0000-000000000030', '00000000-0000-0000-0000-000000000007', 'stdout', 1, 'Created NodeApiKeySection.tsx component'),
    ('00000000-0000-0000-0000-000000000052', '00000000-0000-0000-0000-000000000030', '00000000-0000-0000-0000-000000000007', 'stderr', 2, 'Warning: missing type for blocked_at field'),
    ('00000000-0000-0000-0000-000000000053', '00000000-0000-0000-0000-000000000030', '00000000-0000-0000-0000-000000000007', 'stdout', 3, 'All 36 tests passing'),
    ('00000000-0000-0000-0000-000000000054', '00000000-0000-0000-0000-000000000032', '00000000-0000-0000-0000-000000000007', 'stdout', 0, 'Login page implementation complete. OAuth flow verified.')
ON CONFLICT DO NOTHING;

-- =============================================================================
-- NODE TASK PROGRESS EVENTS
-- =============================================================================

INSERT INTO node_task_progress_events (id, assignment_id, node_id, event_type, message) VALUES
    ('00000000-0000-0000-0000-000000000060', '00000000-0000-0000-0000-000000000030', '00000000-0000-0000-0000-000000000007', 'started', 'Beginning implementation'),
    ('00000000-0000-0000-0000-000000000061', '00000000-0000-0000-0000-000000000030', '00000000-0000-0000-0000-000000000007', 'checkpoint', 'Component created, writing tests'),
    ('00000000-0000-0000-0000-000000000062', '00000000-0000-0000-0000-000000000032', '00000000-0000-0000-0000-000000000007', 'completed', 'All tests green')
ON CONFLICT DO NOTHING;

-- =============================================================================
-- REPORT
-- =============================================================================

DO $$
BEGIN
    RAISE NOTICE '=== E2E Seed Complete ===';
    RAISE NOTICE 'Users:    2 (admin@test.local, member@test.local)';
    RAISE NOTICE 'Org:      1 (e2e-test-org)';
    RAISE NOTICE 'API Keys: 3 (primary, revocable, blocked)';
    RAISE NOTICE 'Nodes:    3 (alpha online, beta online, gamma offline)';
    RAISE NOTICE 'Tasks:    5 (done, in-progress, todo, in-review, cancelled)';
    RAISE NOTICE 'Labels:   4 (bug, feature, docs, urgent)';
    RAISE NOTICE 'Logs:     5 output log entries';
    RAISE NOTICE '';
    RAISE NOTICE 'API Keys:';
    RAISE NOTICE '  Primary:    vk_e2e-primary-key-12345678';
    RAISE NOTICE '  Revocable:  vk_e2e-revocable-key-abcdef';
    RAISE NOTICE '  Blocked:    vk_e2e-blocked-key-xyz';
    RAISE NOTICE '========================';
END $$;
