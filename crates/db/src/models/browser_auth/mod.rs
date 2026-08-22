//! Local browser-authorization records: the pinned Hive owner, browser-bound OAuth handoffs,
//! and hashed opaque browser sessions.
//!
//! Every timestamp in this module is unix-epoch MILLISECONDS supplied by the caller
//! (`now_millis`), never `datetime('now')`. See 20260821000000_add_browser_auth.sql for why.
//!
//! This module never generates or hashes a secret. Callers pass pre-computed `token_hash` /
//! `binding_hash` values (lowercase hex SHA-256), which keeps `crates/db` free of crypto/RNG
//! dependencies and keeps the hashing seam in one place
//! (`crates/server/src/auth/seams.rs::hash_token`).

mod handoff;
mod owner;

pub use handoff::{BrowserHandoff, HANDOFF_TTL_MILLIS, claim_handoff, create_handoff};
pub use owner::{NodeOwner, get_owner, pin_or_verify_owner};

/// Errors that are not plain database failures.
#[derive(Debug, thiserror::Error)]
pub enum BrowserAuthError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    /// A different Hive subject attempted to authorize an already-owned node.
    #[error("node is owned by a different hive subject")]
    OwnerMismatch,
}
