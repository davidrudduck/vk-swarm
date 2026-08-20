//! Server-Sent Events (SSE) endpoint for event streaming.
//!
//! Provides a cursor-based replay-to-live subscription model:
//! - No cursor: stream live events only (not historical replay)
//! - With cursor: replay all events with seq > cursor, then stream live
//!
//! Each SSE frame carries the event's sequence number in the `id` field,
//! allowing clients to resume from `cursor=last_seen_seq`.
//!
//! Error handling: a failure BEFORE the stream starts is a 500-class HTTP response; a failure
//! MID-stream is emitted as a terminal `error` frame and the stream then ends. An `Err` item is
//! never yielded into the SSE body — axum surfaces a stream `Err` as an http_body error, which
//! makes hyper abort the chunked body so the client sees a silent close with no diagnostic
//! (axum-0.8.8 `sse.rs:130`). The item type is therefore [`Infallible`], which makes that
//! impossible by construction.

use std::convert::Infallible;

use axum::{
    Router,
    extract::{Query, State},
    response::{
        Sse,
        sse::{Event, KeepAlive},
    },
    routing::get,
};
use db::models::{event::SequencedEvent, event_journal};
use deployment::{Deployment, DeploymentError};
use futures_util::{
    StreamExt,
    stream::{BoxStream, unfold},
};
use serde::Deserialize;
use services::services::event_bus::EventBusError;
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

/// Map a failure that happens BEFORE the stream starts onto a 500-class `ApiError`.
///
/// `ApiError::Deployment(_)` maps unconditionally to `INTERNAL_SERVER_ERROR`
/// (`crates/server/src/error.rs`, `IntoResponse for ApiError`). `ApiError::Database` is
/// deliberately NOT used: it sub-matches `sqlx::Error::RowNotFound` onto 404, which is not
/// 500-class. `ApiError::BadRequest` would be wrong for the same reason in the other direction —
/// a journal or bus failure is a server fault, not a malformed request. A malformed `cursor` is
/// still a 400: it is rejected by axum's `Query` extractor before this handler runs.
fn internal_error<E>(e: E) -> ApiError
where
    E: std::error::Error + Send + Sync + 'static,
{
    ApiError::Deployment(DeploymentError::Other(anyhow::Error::new(e)))
}

/// State machine backing [`sse_stream`]: once a terminal error frame has been emitted the stream
/// is `Done` and yields nothing further.
enum StreamStage {
    Running(BoxStream<'static, Result<SequencedEvent, EventBusError>>),
    Done,
}

/// The terminal SSE frame emitted for a mid-stream failure.
fn terminal_error_frame(message: String) -> Event {
    Event::default().event("error").data(message)
}

/// Adapt the bus subscription onto SSE frames, emitting a terminal `error` frame and STOPPING on
/// the first failure (bus error or serialization failure).
///
/// `unfold` — the same construction `EventBus::subscribe_from` itself uses
/// (`crates/services/src/services/event_bus/mod.rs:207`) — is required here rather than
/// `scan`/`take_while`: those adapters only run their closure when the INNER stream yields, so a
/// transient fault would leave the stream alive and blocked in `rx.recv()` forever after the error
/// frame. `unfold` decides on the next POLL, so the body ends immediately.
fn sse_stream(
    inner: BoxStream<'static, Result<SequencedEvent, EventBusError>>,
) -> impl futures_util::Stream<Item = Result<Event, Infallible>> {
    unfold(StreamStage::Running(inner), |stage| async move {
        let mut inner = match stage {
            StreamStage::Running(inner) => inner,
            StreamStage::Done => return None,
        };

        match inner.next().await {
            None => None,
            Some(Ok(seq_event)) => match serde_json::to_string(&seq_event) {
                Ok(data) => {
                    let frame = Event::default().id(seq_event.seq.to_string()).data(data);
                    Some((Ok(frame), StreamStage::Running(inner)))
                }
                Err(e) => {
                    // A serialization failure is NOT swallowed to an empty payload: the client
                    // would silently receive a frame carrying nothing.
                    error!(
                        seq = seq_event.seq,
                        error = ?e,
                        "failed to serialize event for SSE; terminating stream"
                    );
                    // The full diagnostic is logged above; the client gets a stable, generic
                    // message so sqlx error text (SQL, table/column names) never leaves the server.
                    let frame = terminal_error_frame(format!(
                        "event serialization failed at seq {}",
                        seq_event.seq
                    ));
                    Some((Ok(frame), StreamStage::Done))
                }
            },
            Some(Err(e)) => {
                error!(error = ?e, "event bus stream error; terminating stream");
                let frame = terminal_error_frame("event stream error".to_string());
                Some((Ok(frame), StreamStage::Done))
            }
        }
    })
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
/// - `data`: JSON-serialized SequencedEvent (`{"seq":N,"event":{"type":"...",...}}`)
pub async fn events(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<EventsQuery>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let bus = deployment.event_bus();

    // Determine the starting cursor:
    // - None means live-only: start at the journal's current high-water mark so nothing already
    //   journaled is replayed. This is NOT cursor=0, which replays the whole journal.
    // - Some(n) means replay everything with seq > n, then go live.
    let cursor = match query.cursor {
        None => event_journal::high_water_mark(&deployment.db().pool)
            .await
            .map_err(|e| {
                error!(error = ?e, "failed to read event journal high-water mark");
                internal_error(e)
            })?,
        Some(cursor) if cursor < 0 => {
            return Err(ApiError::BadRequest(format!(
                "cursor must be non-negative, got {cursor}"
            )));
        }
        Some(cursor) => {
            // A cursor below (low_water - 1) predates retained history: compaction deleted
            // events the client never saw, and replay would silently skip them. Reject with
            // 410 Gone so the client knows to full-refresh and reconnect without a cursor.
            // A cursor of exactly low_water - 1 resumes gaplessly at the first retained row.
            // Best-effort read: a journal failure here is NOT a setup error on the cursor
            // path — the established contract (Test 7) is that cursor-path journal failures
            // surface inside the stream as a terminal error frame, so on Err we skip the
            // staleness check and let the subscription's own first read report the failure.
            let low_water = match event_journal::low_water_mark(&deployment.db().pool).await {
                Ok(mark) => mark,
                Err(e) => {
                    error!(error = ?e, "failed to read event journal low-water mark; deferring failure to the stream");
                    None
                }
            };
            if let Some(min) = low_water
                && cursor < min - 1
            {
                return Err(ApiError::Gone(format!(
                    "cursor {cursor} predates retained history (earliest retained seq is {min});                      refresh state and reconnect without a cursor"
                )));
            }
            cursor
        }
    };

    let stream = bus.subscribe_from(cursor).map_err(|e| {
        error!(error = ?e, "failed to subscribe to the event bus");
        internal_error(e)
    })?;

    Ok(Sse::new(sse_stream(stream)).keep_alive(KeepAlive::default()))
}

pub fn router(_: &DeploymentImpl) -> Router<DeploymentImpl> {
    let events_router = Router::new().route("/", get(events));
    Router::new().nest("/events", events_router)
}

#[cfg(test)]
mod tests {
    use axum::response::IntoResponse;
    use db::models::event_journal::EventJournalError;
    use futures_util::stream;

    use super::*;

    /// `internal_error` must land on the 500-class `ApiError::Deployment` arm: a journal or bus
    /// failure is a server fault. `ApiError::Database` would sub-match `RowNotFound` onto 404 and
    /// `BadRequest` would blame the client — both wrong (see the comment on `internal_error`).
    #[test]
    fn internal_error_maps_to_a_500_response() {
        let err = internal_error(EventJournalError::Database(sqlx::Error::PoolClosed));
        let resp = err.into_response();
        assert_eq!(
            resp.status(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "a journal/bus failure must surface as a 500, not a 404/400"
        );
    }

    /// An inner stream that ends cleanly must end the SSE stream — no frame, no hang.
    #[tokio::test]
    async fn sse_stream_ends_when_the_inner_stream_ends() {
        let inner = stream::empty::<Result<SequencedEvent, EventBusError>>().boxed();
        let mut s = Box::pin(sse_stream(inner));
        assert!(
            s.next().await.is_none(),
            "an exhausted inner stream must end the SSE stream"
        );
    }

    /// A mid-stream error yields exactly one terminal `error` frame and then ends: the
    /// `StreamStage::Done` arm must terminate the unfold on the very next poll (a stream that
    /// stays alive after the frame hangs the client on keep-alives forever).
    #[tokio::test]
    async fn sse_stream_emits_one_error_frame_then_ends() {
        let inner = stream::iter(vec![Err(EventBusError::Journal(
            EventJournalError::Database(sqlx::Error::PoolClosed),
        ))])
        .boxed();
        let mut s = Box::pin(sse_stream(inner));

        let first = s
            .next()
            .await
            .expect("the error must surface as a frame")
            .expect("SSE frames are infallible");
        let rendered = format!("{first:?}");
        assert!(
            rendered.contains("error"),
            "expected a terminal error frame, got: {rendered}"
        );

        assert!(
            s.next().await.is_none(),
            "the stream must END after the terminal error frame"
        );
    }
}
