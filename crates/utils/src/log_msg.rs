use axum::{extract::ws::Message, response::sse::Event};
use json_patch::Patch;
use serde::{Deserialize, Serialize};

pub const EV_STDOUT: &str = "stdout";
pub const EV_STDERR: &str = "stderr";
pub const EV_JSON_PATCH: &str = "json_patch";
pub const EV_SESSION_ID: &str = "session_id";
pub const EV_FINISHED: &str = "finished";
pub const EV_REFRESH_REQUIRED: &str = "refresh_required";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LogMsg {
    Stdout(String),
    Stderr(String),
    JsonPatch(Patch),
    SessionId(String),
    Finished,
    RefreshRequired { reason: String },
}

impl LogMsg {
    pub fn name(&self) -> &'static str {
        match self {
            LogMsg::Stdout(_) => EV_STDOUT,
            LogMsg::Stderr(_) => EV_STDERR,
            LogMsg::JsonPatch(_) => EV_JSON_PATCH,
            LogMsg::SessionId(_) => EV_SESSION_ID,
            LogMsg::Finished => EV_FINISHED,
            LogMsg::RefreshRequired { .. } => EV_REFRESH_REQUIRED,
        }
    }

    pub fn to_sse_event(&self) -> Event {
        match self {
            LogMsg::Stdout(s) => Event::default().event(EV_STDOUT).data(s.clone()),
            LogMsg::Stderr(s) => Event::default().event(EV_STDERR).data(s.clone()),
            LogMsg::JsonPatch(patch) => {
                let data = serde_json::to_string(patch).unwrap_or_else(|_| "[]".to_string());
                Event::default().event(EV_JSON_PATCH).data(data)
            }
            LogMsg::SessionId(s) => Event::default().event(EV_SESSION_ID).data(s.clone()),
            LogMsg::Finished => Event::default().event(EV_FINISHED).data(""),
            LogMsg::RefreshRequired { reason } => Event::default()
                .event(EV_REFRESH_REQUIRED)
                .data(reason.clone()),
        }
    }

    /// Convert LogMsg to WebSocket message with proper error handling
    pub fn to_ws_message(&self) -> Result<Message, serde_json::Error> {
        let json = serde_json::to_string(self)?;
        Ok(Message::Text(json.into()))
    }

    /// Convert LogMsg to WebSocket message with fallback error handling
    ///
    /// This method mirrors the behavior of the original logmsg_to_ws function
    /// but with better error handling than unwrap().
    pub fn to_ws_message_unchecked(&self) -> Message {
        let json = match self {
            // Finished becomes JSON {finished: true}
            LogMsg::Finished => r#"{"finished":true}"#.to_string(),
            // RefreshRequired becomes JSON {refresh_required: true, reason: "..."}
            LogMsg::RefreshRequired { reason } => {
                format!(
                    r#"{{"refresh_required":true,"reason":"{}"}}"#,
                    reason.replace('"', "\\\"")
                )
            }
            _ => serde_json::to_string(self)
                .unwrap_or_else(|_| r#"{"error":"serialization_failed"}"#.to_string()),
        };

        Message::Text(json.into())
    }

    /// Rough size accounting for your byte‑budgeted history.
    pub fn approx_bytes(&self) -> usize {
        const OVERHEAD: usize = 8;
        match self {
            LogMsg::Stdout(s) => EV_STDOUT.len() + s.len() + OVERHEAD,
            LogMsg::Stderr(s) => EV_STDERR.len() + s.len() + OVERHEAD,
            LogMsg::JsonPatch(patch) => {
                let json_len = serde_json::to_string(patch).map(|s| s.len()).unwrap_or(2);
                EV_JSON_PATCH.len() + json_len + OVERHEAD
            }
            LogMsg::SessionId(s) => EV_SESSION_ID.len() + s.len() + OVERHEAD,
            LogMsg::Finished => EV_FINISHED.len() + OVERHEAD,
            LogMsg::RefreshRequired { reason } => {
                EV_REFRESH_REQUIRED.len() + reason.len() + OVERHEAD
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── LogMsg::name ───────────────────────────────────────────────────

    #[test]
    fn name_stdout() {
        let msg = LogMsg::Stdout("hello".into());
        assert_eq!(msg.name(), EV_STDOUT);
    }

    #[test]
    fn name_stderr() {
        let msg = LogMsg::Stderr("error".into());
        assert_eq!(msg.name(), EV_STDERR);
    }

    #[test]
    fn name_finished() {
        let msg = LogMsg::Finished;
        assert_eq!(msg.name(), EV_FINISHED);
    }

    #[test]
    fn name_session_id() {
        let msg = LogMsg::SessionId("abc123".into());
        assert_eq!(msg.name(), EV_SESSION_ID);
    }

    #[test]
    fn name_refresh_required() {
        let msg = LogMsg::RefreshRequired {
            reason: "config changed".into(),
        };
        assert_eq!(msg.name(), EV_REFRESH_REQUIRED);
    }

    // ── LogMsg::approx_bytes ───────────────────────────────────────────

    #[test]
    fn approx_bytes_stdout() {
        let msg = LogMsg::Stdout("hello".into());
        let bytes = msg.approx_bytes();
        assert!(bytes > "hello".len());
    }

    #[test]
    fn approx_bytes_finished() {
        let msg = LogMsg::Finished;
        let bytes = msg.approx_bytes();
        assert!(bytes > 0);
    }

    #[test]
    fn approx_bytes_refresh_required() {
        let msg = LogMsg::RefreshRequired {
            reason: "restart".into(),
        };
        let bytes = msg.approx_bytes();
        assert!(bytes > "restart".len());
    }

    #[test]
    fn approx_bytes_empty_stdout() {
        let msg = LogMsg::Stdout(String::new());
        let bytes = msg.approx_bytes();
        assert!(bytes > 0);
    }

    // ── LogMsg::to_sse_event ───────────────────────────────────────────

    #[test]
    fn to_sse_event_constructs_without_panic() {
        let msg = LogMsg::Finished;
        let _event = msg.to_sse_event();
    }

    #[test]
    fn to_sse_event_stdout() {
        let msg = LogMsg::Stdout("output".into());
        let event = msg.to_sse_event();
        let debug = format!("{event:?}");
        assert!(debug.contains("output"));
    }

    #[test]
    fn to_sse_event_refresh_required() {
        let msg = LogMsg::RefreshRequired {
            reason: "reload".into(),
        };
        let event = msg.to_sse_event();
        let debug = format!("{event:?}");
        assert!(debug.contains("reload"));
    }

    #[test]
    fn to_sse_event_session_id() {
        let msg = LogMsg::SessionId("sess-1".into());
        let event = msg.to_sse_event();
        let debug = format!("{event:?}");
        assert!(debug.contains("sess-1"));
    }

    #[test]
    fn to_sse_event_stderr() {
        let msg = LogMsg::Stderr("error output".into());
        let event = msg.to_sse_event();
        let debug = format!("{event:?}");
        assert!(debug.contains("error output"));
    }

    #[test]
    fn to_sse_event_json_patch() {
        let patch: json_patch::Patch = serde_json::from_str(r#"[{"op":"add","path":"/foo","value":1}]"#).unwrap();
        let msg = LogMsg::JsonPatch(patch);
        let event = msg.to_sse_event();
        let debug = format!("{event:?}");
        assert!(debug.contains("add"));
    }

    // ── LogMsg::to_ws_message ─────────────────────────────────────────

    #[test]
    fn to_ws_message_finished() {
        let msg = LogMsg::Finished;
        let result = msg.to_ws_message();
        assert!(result.is_ok());
        match result.unwrap() {
            Message::Text(s) => assert!(s.contains("Finished")),
            _ => panic!("expected Text message"),
        }
    }

    #[test]
    fn to_ws_message_stdout() {
        let msg = LogMsg::Stdout("data".into());
        let result = msg.to_ws_message();
        assert!(result.is_ok());
    }

    // ── LogMsg::to_ws_message_unchecked ───────────────────────────────

    #[test]
    fn to_ws_message_unchecked_finished() {
        let msg = LogMsg::Finished;
        match msg.to_ws_message_unchecked() {
            Message::Text(s) => assert!(s.contains("finished")),
            _ => panic!("expected Text message"),
        }
    }

    #[test]
    fn to_ws_message_unchecked_refresh_required() {
        let msg = LogMsg::RefreshRequired {
            reason: "config".into(),
        };
        match msg.to_ws_message_unchecked() {
            Message::Text(s) => {
                assert!(s.contains("refresh_required"));
                assert!(s.contains("config"));
            }
            _ => panic!("expected Text message"),
        }
    }

    #[test]
    fn to_ws_message_unchecked_refresh_required_escapes_quotes() {
        let msg = LogMsg::RefreshRequired {
            reason: r#"file "foo.txt" changed"#.into(),
        };
        match msg.to_ws_message_unchecked() {
            Message::Text(s) => {
                assert!(s.contains(r#"file \"foo.txt\" changed"#));
            }
            _ => panic!("expected Text message"),
        }
    }

    #[test]
    fn to_ws_message_unchecked_stdout() {
        let msg = LogMsg::Stdout("hello".into());
        match msg.to_ws_message_unchecked() {
            Message::Text(s) => assert!(s.contains("Stdout")),
            _ => panic!("expected Text message"),
        }
    }
}
