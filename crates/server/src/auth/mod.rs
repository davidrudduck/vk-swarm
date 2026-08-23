//! Local browser-authorization for the node HTTP surface.
//!
//! `seams` holds the injected clock / token source / hash used by every browser-auth path so
//! expiry and token behaviour are deterministic in tests. Later tasks add `cookies`, `session`,
//! `node_token` and `login` here.

pub mod cookies;
pub mod node_token;
pub mod seams;
pub mod session;
