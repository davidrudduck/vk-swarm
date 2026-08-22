-- Local browser-authorization schema (local-node-browser-oauth). Three additive tables; no
-- existing table is altered.
--
-- Divergence from every sibling table in this directory: all timestamps here are INTEGER
-- unix-epoch MILLISECONDS bound explicitly by the caller, not TEXT `datetime('now','subsec')`
-- defaults. Two reasons. (1) Handoff expiry is a stored-vs-bound comparison, and the
-- event_journal compaction regression (20260812000000, see
-- compact_keeps_same_day_rows_inside_the_retention_window) proved that comparing a TEXT
-- datetime() column against an RFC-3339 bind collates wrong ('T' > ' '). Here that failure mode
-- fails OPEN: an expired handoff would still be claimable. (2) A SQL DEFAULT bypasses the
-- injected test clock, and exact 10-minute expiry must be driven deterministically (TS1).

-- Exactly one owner. `slot` is pinned to 1 by CHECK, which makes the singleton structural.
-- First writer wins: the pin-or-compare upsert uses a no-op DO UPDATE so RETURNING yields the
-- EXISTING owner on conflict without replacing it.
CREATE TABLE IF NOT EXISTS node_owner (
    slot         INTEGER PRIMARY KEY CHECK (slot = 1),
    hive_user_id BLOB    NOT NULL,   -- UUID, stable Hive subject (ProfileResponse.user_id)
    pinned_at    INTEGER NOT NULL    -- unix epoch millis
);

-- Browser-bound OAuth handoffs. `binding_hash` is the SHA-256 hex of the pre-auth browser
-- cookie value; the raw value never reaches this table. `app_verifier` IS stored raw -- it is
-- the verifier the daemon must present to Hive at redemption, so it cannot be hashed. `state`
-- is terminal after 'claimed': redemption success AND redemption failure both leave the row
-- unclaimable, so replay can never mint a second session.
CREATE TABLE IF NOT EXISTS browser_oauth_handoffs (
    handoff_id   BLOB    PRIMARY KEY,  -- UUID issued by Hive
    provider     TEXT    NOT NULL,
    app_verifier TEXT    NOT NULL,
    binding_hash TEXT    NOT NULL,     -- lowercase hex SHA-256, 64 chars
    state        TEXT    NOT NULL DEFAULT 'pending' CHECK (state IN ('pending', 'claimed')),
    created_at   INTEGER NOT NULL,     -- unix epoch millis
    expires_at   INTEGER NOT NULL      -- unix epoch millis; claimable while expires_at > now
);

-- Opaque browser sessions. Only the SHA-256 hex of the 256-bit base64url token is stored; the
-- raw token exists only in the Set-Cookie header and the presenting browser. There is
-- deliberately NO expiry column: authorization is revocation-state only, never time-based
-- (D9/SC5).
CREATE TABLE IF NOT EXISTS browser_sessions (
    id           BLOB    PRIMARY KEY,  -- UUID v4
    token_hash   TEXT    NOT NULL UNIQUE,
    hive_user_id BLOB    NOT NULL,     -- the pinned owner subject
    created_at   INTEGER NOT NULL,     -- unix epoch millis
    revoked_at   INTEGER              -- NULL while live
);

-- Authentication is a point lookup on the hash for every protected request; the UNIQUE
-- constraint above already provides that index, so no extra index is created here.
