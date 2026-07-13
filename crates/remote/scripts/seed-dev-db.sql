-- Dev seed script: creates test user, org, and API key for local E2E testing
-- Run after migrations complete

-- Test user
INSERT INTO users (id, email, first_name, last_name, username)
VALUES (
    '00000000-0000-0000-0000-000000000001',
    'dev@test.local',
    'Dev',
    'User',
    'devuser'
) ON CONFLICT (email) DO NOTHING;

-- Test organization (personal)
INSERT INTO organizations (id, name, slug, is_personal)
VALUES (
    '00000000-0000-0000-0000-000000000002',
    'Dev Test Org',
    'dev-test-org',
    true
) ON CONFLICT (slug) DO NOTHING;

-- Org membership (admin)
INSERT INTO organization_member_metadata (organization_id, user_id, role)
VALUES (
    '00000000-0000-0000-0000-000000000002',
    '00000000-0000-0000-0000-000000000001',
    'admin'
) ON CONFLICT (organization_id, user_id) DO NOTHING;

-- OAuth account (fake GitHub)
INSERT INTO oauth_accounts (id, user_id, provider, provider_user_id, email, username, display_name)
VALUES (
    '00000000-0000-0000-0000-000000000003',
    '00000000-0000-0000-0000-000000000001',
    'github',
    'dev-github-id-001',
    'dev@test.local',
    'devuser',
    'Dev User'
) ON CONFLICT (provider, provider_user_id) DO NOTHING;

-- Test node API key
-- Key: vk_dev-test-key-secret-12345678
-- Prefix: vk_dev-te
-- Hash: SHA256 of the key
INSERT INTO node_api_keys (id, organization_id, name, key_hash, key_prefix, created_by)
VALUES (
    '00000000-0000-0000-0000-000000000004',
    '00000000-0000-0000-0000-000000000002',
    'Dev Test Key',
    encode(sha256('vk_dev-test-key-secret-12345678'::bytea), 'hex'),
    'vk_dev-te',
    '00000000-0000-0000-0000-000000000001'
) ON CONFLICT DO NOTHING;

-- Second test API key (for revoke/unblock testing)
-- Key: vk_dev-test-key-revocable-abcdef
-- Prefix: vk_dev-te
INSERT INTO node_api_keys (id, organization_id, name, key_hash, key_prefix, created_by)
VALUES (
    '00000000-0000-0000-0000-000000000005',
    '00000000-0000-0000-0000-000000000002',
    'Dev Revocable Key',
    encode(sha256('vk_dev-test-key-revocable-abcdef'::bytea), 'hex'),
    'vk_dev-te',
    '00000000-0000-0000-0000-000000000001'
) ON CONFLICT DO NOTHING;

-- Report what was seeded
DO $$
BEGIN
    RAISE NOTICE '=== Seed complete ===';
    RAISE NOTICE 'User:   dev@test.local (id: 00000000-0000-0000-0000-000000000001)';
    RAISE NOTICE 'Org:    dev-test-org (id: 00000000-0000-0000-0000-000000000002)';
    RAISE NOTICE 'API Key: vk_dev-test-key-secret-12345678';
    RAISE Notice 'API Key: vk_dev-test-key-revocable-abcdef';
END $$;
