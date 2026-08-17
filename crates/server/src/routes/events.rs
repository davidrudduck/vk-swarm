//! Server-Sent Events (SSE) endpoint for event streaming.
//!
//! Provides a cursor-based replay-to-live subscription model:
//! - No cursor: stream live events only (not historical replay)
//! - With cursor: replay all events with seq > cursor, then stream live
//!
//! Each SSE frame carries the event's sequence number in the `id` field,
//! allowing clients to resume from `cursor=last_seen_seq`.

use axum::{
    BoxError, Router,
    extract::{Query, State},
    response::{
        Sse,
        sse::{Event, KeepAlive},
    },
    routing::get,
};
use deployment::Deployment;
use futures_util::StreamExt;
use serde::Deserialize;
use tracing::error;

use crate::DeploymentImpl;
use crate::error::ApiError;

/// Query parameters for SSE subscription.
#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    /// Cursor to start from. If provided, replays events with seq > cursor.
    /// If absent, streams live events only (NOT cursor=0).
    #[serde(default)]
    pub cursor: Option<i64>,
}

/// GET /api/events
///
/// Stream events as Server-Sent Events (SSE).
///
/// # Query Parameters
/// - `cursor` (optional): Starting sequence number. Events with seq > cursor are replayed first.
///   Absent cursor means live-only (no historical replay).
///
/// # Response
/// SSE stream with one event per journaled/live SequencedEvent. Each frame carries:
/// - `id`: The event's sequence number
/// - `data`: JSON-serialized SequencedEvent
pub async fn events(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<EventsQuery>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, BoxError>>>, ApiError> {
    let bus = deployment.event_bus();

    // Determine starting cursor:
    // - None means live-only (subscribe without replay)
    // - Some(n) means replay from n, then go live
    let stream = match query.cursor {
        None => {
            // Live-only: subscribe without any replay
            // We use cursor = high_water_mark to skip all history
            let mark = db::models::event_journal::high_water_mark(&deployment.db().pool)
                .await
                .map_err(|e| {
                    error!("Failed to get high water mark: {}", e);
                    ApiError::BadRequest(format!(
                        "Failed to get event journal high water mark: {}",
                        e
                    ))
                })?;
            bus.subscribe_from(mark).map_err(|e| {
                error!("Failed to subscribe to event bus: {}", e);
                ApiError::BadRequest(format!("Failed to subscribe to events: {}", e))
            })?
        }
        Some(cursor) => {
            // Replay + live: subscribe from the given cursor
            bus.subscribe_from(cursor).map_err(|e| {
                error!("Failed to subscribe to event bus: {}", e);
                ApiError::BadRequest(format!("Failed to subscribe to events: {}", e))
            })?
        }
    };

    // Map SequencedEvent to SSE Event
    let sse_stream = stream.map(|result| {
        match result {
            Ok(seq_event) => {
                let seq_str = seq_event.seq.to_string();
                // Serialize event to JSON
                let data = serde_json::to_string(&seq_event).unwrap_or_else(|_| "{}".to_string());
                Ok(Event::default().id(seq_str).data(data))
            }
            Err(e) => {
                // Terminal error frame
                error!("Event bus stream error: {}", e);
                Err::<Event, BoxError>(format!("Event bus error: {}", e).into())
            }
        }
    });

    Ok(Sse::new(sse_stream).keep_alive(KeepAlive::default()))
}

pub fn router(_: &DeploymentImpl) -> Router<DeploymentImpl> {
    let events_router = Router::new().route("/", get(events));
    Router::new().nest("/events", events_router)
}
