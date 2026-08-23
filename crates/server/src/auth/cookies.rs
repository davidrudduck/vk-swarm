//! `Set-Cookie` construction and parsing for the two browser-authorization cookies.
//!
//! Hand-rolled on purpose (no `cookie` crate, no `axum-extra` cookie feature): the format here
//! is four fixed attribute strings plus a ~10-line reader, and a dependency change is outside
//! this task's file list.

use axum::http::HeaderMap;

/// The authorized browser-session cookie. Opaque 256-bit base64url token; only its SHA-256 hex
/// is stored server-side.
pub const SESSION_COOKIE: &str = "vks_browser_session";
/// The pre-auth handoff binding cookie. Present only between OAuth initiation and callback.
pub const BINDING_COOKIE: &str = "vks_browser_binding";

/// Five years in seconds (5 * 365 * 24 * 3600). Persistent across browser restart (SC5).
const SESSION_MAX_AGE_SECS: i64 = 157_680_000;
/// Ten minutes -- matches HANDOFF_TTL_MILLIS so a stale binding cookie cannot outlive its handoff.
const BINDING_MAX_AGE_SECS: i64 = 600;

/// Read one cookie value from a `Cookie:` header. Splits on ';', trims, matches `name=`.
pub fn read_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let header = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    let prefix = format!("{name}=");
    header
        .split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix(&prefix).map(|value| value.to_string()))
}

/// `Set-Cookie` for a new authorized session.
///
/// `Secure` is deliberately ABSENT (D9): the supported deployment is plain HTTP on a trusted LAN,
/// and a Secure cookie would simply never be sent. The plaintext-session risk is documented for
/// operators in docs/configuration-customisation/browser-authorization.mdx.
pub fn session_set_cookie(token: &str) -> String {
    format!(
        "{SESSION_COOKIE}={token}; HttpOnly; SameSite=Lax; Path=/; Max-Age={SESSION_MAX_AGE_SECS}"
    )
}

/// `Set-Cookie` that removes the session cookie from the presenting browser (Max-Age=0).
pub fn session_clear_cookie() -> String {
    format!("{SESSION_COOKIE}=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0")
}

/// `Set-Cookie` for the pre-auth binding secret.
///
/// `SameSite=Lax`, NEVER `Strict`: the hive OAuth callback arrives as a cross-site TOP-LEVEL GET
/// navigation. Lax sends the cookie on that navigation; Strict withholds it, and the handoff
/// claim would then fail for the RIGHTFUL browser -- indistinguishable from a wrong-browser
/// rejection.
pub fn binding_set_cookie(token: &str) -> String {
    format!(
        "{BINDING_COOKIE}={token}; HttpOnly; SameSite=Lax; Path=/; Max-Age={BINDING_MAX_AGE_SECS}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_cookie_attributes_are_exact() {
        let c = session_set_cookie("tok123");
        assert_eq!(
            c,
            "vks_browser_session=tok123; HttpOnly; SameSite=Lax; Path=/; Max-Age=157680000"
        );
        assert!(
            !c.contains("Secure"),
            "D9: plain-HTTP LAN deployment must not set Secure"
        );
    }
    #[test]
    fn binding_cookie_is_lax_and_short_lived() {
        let c = binding_set_cookie("bind123");
        assert_eq!(
            c,
            "vks_browser_binding=bind123; HttpOnly; SameSite=Lax; Path=/; Max-Age=600"
        );
        // MUST be Lax, never Strict: the hive callback is a cross-site top-level GET navigation and
        // Strict would withhold the cookie, making every login look like a wrong-browser rejection.
        assert!(c.contains("SameSite=Lax"));
    }
    #[test]
    fn clear_cookie_expires_immediately() {
        // Byte-exact: a `; Secure` mutant (D9 violation) must fail this assertion, not pass it.
        assert_eq!(
            session_clear_cookie(),
            "vks_browser_session=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0"
        );
        assert!(!session_clear_cookie().contains("Secure"));
    }
    #[test]
    fn read_cookie_picks_the_named_value_from_a_multi_cookie_header() {
        let mut h = axum::http::HeaderMap::new();
        h.insert(
            axum::http::header::COOKIE,
            "other=1; vks_browser_session=abc; vks_browser_binding=def"
                .parse()
                .unwrap(),
        );
        assert_eq!(read_cookie(&h, SESSION_COOKIE), Some("abc".to_string()));
        assert_eq!(read_cookie(&h, BINDING_COOKIE), Some("def".to_string()));
        assert_eq!(read_cookie(&h, "absent"), None);
        assert_eq!(
            read_cookie(&axum::http::HeaderMap::new(), SESSION_COOKIE),
            None
        );
    }
}
