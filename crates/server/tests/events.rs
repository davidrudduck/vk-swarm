//! TS5 — route tests for `GET /api/events`, the cursor-resumable SSE endpoint (SC4).
//!
//! Every test drives the REAL served router over real TCP (`common::HiveHarness`), and consumes
//! the SSE body as a raw byte stream: the harness's `get()` helper reads the body to completion,
//! which never happens on an SSE stream.
//!
//! Frame format (axum-0.8.8 `response/sse.rs:397-421`): each field is written as
//! `name: value\n` and each event is terminated by a further `\n`, so complete frames are
//! separated by a blank line. Keep-alive frames are bare `:` comments with no `data:` line.

mod common;

use std::collections::BTreeSet;

use db::models::{
    event::{NodeEvent, SequencedEvent},
    event_journal,
    project::{CreateProject, Project},
    task::{CreateTask, Task, TaskStatus},
};
use deployment::Deployment;
use futures_util::StreamExt;
use uuid::Uuid;

// ---------------------------------------------------------------------------------------------
// SSE frame parsing helpers
// ---------------------------------------------------------------------------------------------

/// Split an accumulated SSE byte buffer into COMPLETE frames.
///
/// `bytes_stream()` chunks are arbitrary, so the tail of the buffer is very likely a partial
/// frame — every read loop below breaks early. A trailing partial frame is therefore discarded
/// unless the buffer ends on a frame boundary.
fn complete_frames(buf: &str) -> Vec<&str> {
    let mut frames: Vec<&str> = buf.split("\n\n").collect();
    if !buf.ends_with("\n\n") {
        frames.pop();
    }
    frames
        .into_iter()
        .filter(|f| !f.trim().is_empty())
        .collect()
}

/// Complete frames that actually carry a payload (keep-alive comments excluded).
fn data_frames(buf: &str) -> Vec<&str> {
    complete_frames(buf)
        .into_iter()
        .filter(|f| f.lines().any(|l| l.starts_with("data:")))
        .collect()
}

/// The `id:` field of a frame, if present.
fn frame_id(frame: &str) -> Option<i64> {
    frame
        .lines()
        .find_map(|l| l.strip_prefix("id:"))
        .and_then(|v| v.trim().parse::<i64>().ok())
}

/// The concatenated `data:` payload of a frame.
fn frame_data(frame: &str) -> String {
    frame
        .lines()
        .filter_map(|l| l.strip_prefix("data:"))
        .map(|v| v.strip_prefix(' ').unwrap_or(v))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Ids of every data frame, in ARRIVAL ORDER (duplicates preserved).
fn frame_ids_in_order(buf: &str) -> Vec<i64> {
    data_frames(buf).into_iter().filter_map(frame_id).collect()
}

/// Journal `count` distinct `task_created` events in one committed transaction, returning the
/// assigned seqs. Tests 1-5 may journal directly; test 6 deliberately does not.
async fn journal_events(pool: &sqlx::SqlitePool, count: usize) -> Vec<i64> {
    let mut tx = pool.begin().await.unwrap();
    let mut seqs = Vec::with_capacity(count);
    for _ in 0..count {
        let event = NodeEvent::TaskCreated {
            task_id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
        };
        seqs.push(event_journal::append(&mut *tx, &event).await.unwrap());
    }
    tx.commit().await.unwrap();
    seqs
}

/// Test 1: events_without_cursor_streams_live_only
/// - Subscribe with no cursor
/// - Assert pre-existing journal rows are NOT replayed
/// - Assert a subsequently emitted event IS received
#[tokio::test]
#[serial_test::serial]
async fn events_without_cursor_streams_live_only() {
    let h = common::HiveHarness::hive_absent().await;

    // Seed some journal events BEFORE subscribing
    {
        let pool = &h.deployment().db().pool;
        let mut tx = pool.begin().await.unwrap();
        for _ in 0..3 {
            let event = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            let _ = event_journal::append(&mut *tx, &event).await.unwrap();
        }
        tx.commit().await.unwrap();
    }

    // Now subscribe to /api/events (no cursor param) — should get live-only
    let client = reqwest::Client::new();
    let url = format!("http://{}/api/events", h.addr());
    let res = client.get(&url).send().await.unwrap();

    assert_eq!(res.status(), 200, "SSE endpoint should return 200");
    let mut event_stream = res.bytes_stream();

    // Give the connection time to establish
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Emit a live event AFTER subscription
    let live_event = NodeEvent::TaskCreated {
        task_id: Uuid::new_v4(),
        project_id: Uuid::new_v4(),
    };
    let bus = h.deployment().event_bus();
    let sender = bus.sender();
    sender
        .send(db::models::event::SequencedEvent {
            seq: 4,
            event: live_event,
        })
        .ok();

    // Collect frames until we see the live event
    let mut collected = String::new();
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);

    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(tokio::time::Duration::from_millis(100), event_stream.next())
            .await
        {
            Ok(Some(Ok(bytes))) => {
                collected.push_str(&String::from_utf8_lossy(&bytes));
                if collected.contains("id: 4") {
                    break;
                }
            }
            Ok(Some(Err(_))) => break,
            Ok(None) => break,
            Err(_) => continue,
        }
    }

    // Assert that seq 4 arrived (the live event)
    assert!(
        collected.contains("id: 4"),
        "live event (seq 4) should have arrived; collected: {}",
        collected
    );

    // Assert that seqs 1-3 (pre-existing) are NOT in the stream
    // This is harder to prove negatively, so we check the minimal case:
    // the collected data should NOT be empty and SHOULD contain seq 4
    assert!(!collected.contains("id: 1"));
    assert!(!collected.contains("id: 2"));
    assert!(!collected.contains("id: 3"));
}

/// Test 2: events_with_cursor_replays_then_goes_live
/// - Journal 5 events
/// - Subscribe with cursor=2 (replay seqs 3,4,5)
/// - Journal a 6th event and let the TAILER publish it (no raw sender injection: a direct
///   `bus.sender().send()` races the subscriber's replay→live transition and can be lost)
/// - Assert seqs 3,4,5,6 all arrive and seqs 1,2 do not
#[tokio::test]
#[serial_test::serial]
async fn events_with_cursor_replays_then_goes_live() {
    let h = common::HiveHarness::hive_absent().await;

    let seeded = journal_events(&h.deployment().db().pool, 5).await;
    assert_eq!(
        seeded,
        vec![1, 2, 3, 4, 5],
        "harness journal should start empty; got {seeded:?}"
    );

    // Subscribe with cursor=2 (should replay 3,4,5)
    let client = reqwest::Client::new();
    let url = format!("http://{}/api/events?cursor=2", h.addr());
    let res = client.get(&url).send().await.unwrap();
    assert_eq!(res.status(), 200);

    let mut event_stream = res.bytes_stream();

    // The live leg: journal a 6th event. The journal tailer (75ms poll) publishes it to the bus,
    // which is the production delivery path — no test-only injection into the broadcast channel.
    let live = journal_events(&h.deployment().db().pool, 1).await;
    assert_eq!(live, vec![6]);

    // Collect SSE frames
    let mut collected = String::new();
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);

    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(tokio::time::Duration::from_millis(100), event_stream.next())
            .await
        {
            Ok(Some(Ok(bytes))) => {
                collected.push_str(&String::from_utf8_lossy(&bytes));
                if collected.contains("id: 6") {
                    break;
                }
            }
            Ok(Some(Err(_))) => break,
            Ok(None) => break,
            Err(_) => continue,
        }
    }

    let ids: BTreeSet<i64> = frame_ids_in_order(&collected).into_iter().collect();

    // Assert that seqs 3, 4, 5 arrived from replay and 6 arrived live
    assert!(ids.contains(&3), "seq 3 should arrive from replay: {ids:?}");
    assert!(ids.contains(&4), "seq 4 should arrive from replay: {ids:?}");
    assert!(ids.contains(&5), "seq 5 should arrive from replay: {ids:?}");
    assert!(
        ids.contains(&6),
        "seq 6 should arrive live, published by the journal tailer: {ids:?}"
    );

    // Assert that seqs 1, 2 did NOT arrive — cursor=2 means "everything above 2"
    assert!(
        !ids.contains(&1),
        "seq 1 is at or below the cursor: {ids:?}"
    );
    assert!(
        !ids.contains(&2),
        "seq 2 is at or below the cursor: {ids:?}"
    );
}

/// Test 3: each_sse_message_carries_seq
/// - Assert EVERY data frame carries an `id:` line
/// - Assert the exact set of ids equals the set of journaled seqs
/// - A stream that omits seq makes SC4 unimplementable client-side
#[tokio::test]
#[serial_test::serial]
async fn each_sse_message_carries_seq() {
    let h = common::HiveHarness::hive_absent().await;

    // Journal exactly three events. The assertion below also proves nothing else journaled
    // during harness construction, which is what makes the exact-set assertion sound.
    let seeded = journal_events(&h.deployment().db().pool, 3).await;
    assert_eq!(
        seeded,
        vec![1, 2, 3],
        "harness journal should contain only the three seeded events; got {seeded:?}"
    );

    // Subscribe from seq 0 to get all three
    let client = reqwest::Client::new();
    let url = format!("http://{}/api/events?cursor=0", h.addr());
    let res = client.get(&url).send().await.unwrap();
    assert_eq!(res.status(), 200);

    let mut event_stream = res.bytes_stream();

    // Collect until all three complete frames have arrived
    let mut collected = String::new();
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);

    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(tokio::time::Duration::from_millis(100), event_stream.next())
            .await
        {
            Ok(Some(Ok(bytes))) => {
                collected.push_str(&String::from_utf8_lossy(&bytes));
                if data_frames(&collected).len() >= 3 {
                    break;
                }
            }
            Ok(Some(Err(_))) => break,
            Ok(None) => break,
            Err(_) => continue,
        }
    }

    let frames = data_frames(&collected);
    assert_eq!(
        frames.len(),
        3,
        "expected the three journaled events; collected: {collected:?}"
    );

    // EVERY data frame must carry an id line — not merely "at least one".
    for frame in &frames {
        assert!(
            frame.lines().any(|l| l.starts_with("id: ")),
            "every SSE data frame must carry an `id:` line so a client can resume; \
             offending frame: {frame:?}"
        );
    }

    let ids: BTreeSet<i64> = frames.iter().copied().filter_map(frame_id).collect();
    assert_eq!(
        ids,
        BTreeSet::from([1, 2, 3]),
        "the ids must be exactly the journaled seqs; collected: {collected:?}"
    );
}

/// Test 4: reconnect_with_last_seen_cursor_skips_nothing
/// - Subscribe, observe events, derive the last-seen seq FROM THE FRAMES
/// - Actually disconnect (drop the response stream)
/// - Journal N events while disconnected
/// - Resubscribe with the observed last-seen cursor
/// - Assert every journaled seq above the cursor arrives in non-decreasing order, none skipped,
///   and nothing at or below the cursor is re-delivered (duplicates above it are tolerated)
#[tokio::test]
#[serial_test::serial]
async fn reconnect_with_last_seen_cursor_skips_nothing() {
    let h = common::HiveHarness::hive_absent().await;

    let before = journal_events(&h.deployment().db().pool, 3).await;
    assert_eq!(before, vec![1, 2, 3], "unexpected seeded seqs: {before:?}");

    // ---- connection 1 -------------------------------------------------------------------
    let client = reqwest::Client::new();
    let url = format!("http://{}/api/events?cursor=0", h.addr());
    let res = client.get(&url).send().await.unwrap();
    assert_eq!(res.status(), 200);

    let mut event_stream = res.bytes_stream();

    let mut collected = String::new();
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);

    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(tokio::time::Duration::from_millis(100), event_stream.next())
            .await
        {
            Ok(Some(Ok(bytes))) => {
                collected.push_str(&String::from_utf8_lossy(&bytes));
                if frame_ids_in_order(&collected).contains(&3) {
                    break;
                }
            }
            Ok(Some(Err(_))) => break,
            Ok(None) => break,
            Err(_) => continue,
        }
    }

    // The cursor a real client would resume from: the highest seq it actually SAW, not a literal.
    let last_seen = frame_ids_in_order(&collected)
        .into_iter()
        .max()
        .expect("connection 1 should have observed at least one sequenced frame");
    assert_eq!(
        last_seen, 3,
        "connection 1 should have caught up to the journal head; collected: {collected:?}"
    );

    // ---- disconnect ---------------------------------------------------------------------
    // Dropping the body stream drops the response, closing the TCP connection. Shadowing the
    // binding would NOT: the old stream would stay alive until end of scope.
    drop(event_stream);

    // ---- events journaled while disconnected ---------------------------------------------
    let during = journal_events(&h.deployment().db().pool, 5).await;
    assert_eq!(during, vec![4, 5, 6, 7, 8], "unexpected seqs: {during:?}");

    // ---- connection 2: resume from the observed cursor ------------------------------------
    let url = format!("http://{}/api/events?cursor={}", h.addr(), last_seen);
    let res = client.get(&url).send().await.unwrap();
    assert_eq!(res.status(), 200);

    let mut event_stream = res.bytes_stream();

    let mut reconnected = String::new();
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);

    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(tokio::time::Duration::from_millis(100), event_stream.next())
            .await
        {
            Ok(Some(Ok(bytes))) => {
                reconnected.push_str(&String::from_utf8_lossy(&bytes));
                let seen: BTreeSet<i64> = frame_ids_in_order(&reconnected).into_iter().collect();
                if during.iter().all(|s| seen.contains(s)) {
                    break;
                }
            }
            Ok(Some(Err(_))) => break,
            Ok(None) => break,
            Err(_) => continue,
        }
    }

    let arrival_order = frame_ids_in_order(&reconnected);
    let seen: BTreeSet<i64> = arrival_order.iter().copied().collect();

    // Nothing skipped: every seq journaled while disconnected arrived.
    for seq in &during {
        assert!(
            seen.contains(seq),
            "seq {seq} was journaled while disconnected and MUST arrive on resume; \
             arrival order: {arrival_order:?}"
        );
    }

    // Nothing at or below the cursor is re-delivered.
    assert!(
        arrival_order.iter().all(|id| *id > last_seen),
        "cursor={last_seen} must not replay anything at or below it; arrival order: \
         {arrival_order:?}"
    );

    // Ascending arrival order. Non-decreasing rather than strict: the dictate tolerates
    // duplicates (a replayed event may also arrive live), it forbids gaps and re-ordering.
    assert!(
        arrival_order.windows(2).all(|w| w[0] <= w[1]),
        "events must arrive in ascending seq order; arrival order: {arrival_order:?}"
    );
}

/// Test 5: removed_record_patch_route_is_gone — the TS5 guard.
///
/// Two halves:
/// (a) source-level — `Deployment::stream_events`, the trait method that produced the old
///     record-patch stream, no longer exists;
/// (b) shape-level — `GET /api/events` is a REGISTERED SSE route whose payload is a
///     `SequencedEvent`, and is NOT the old `LogMsg::to_sse_event()` shape
///     (`crates/utils/src/log_msg.rs:34-48`: named `event:` fields `json_patch`/`stdout`/
///     `stderr`/`session_id`/`finished`/`refresh_required`, with a JSON *array* body for
///     `json_patch`).
#[tokio::test]
#[serial_test::serial]
async fn removed_record_patch_route_is_gone() {
    // (a) The old stream's producer is gone from the deployment trait.
    const DEPLOYMENT_LIB: &str = include_str!("../../deployment/src/lib.rs");
    assert!(
        !DEPLOYMENT_LIB.contains("fn stream_events"),
        "Deployment::stream_events (the old record-patch stream source) must not exist"
    );

    let h = common::HiveHarness::hive_absent().await;
    let seeded = journal_events(&h.deployment().db().pool, 1).await;
    assert_eq!(seeded, vec![1]);

    // (b) Raw bounded read — the harness `get()` helper would hang here, because an SSE body
    // never ends.
    let client = reqwest::Client::new();
    let url = format!("http://{}/api/events?cursor=0", h.addr());
    let res = client.get(&url).send().await.unwrap();

    let status = res.status().as_u16();
    let content_type = res
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let mut event_stream = res.bytes_stream();
    let mut collected = String::new();
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);

    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(tokio::time::Duration::from_millis(100), event_stream.next())
            .await
        {
            Ok(Some(Ok(bytes))) => {
                collected.push_str(&String::from_utf8_lossy(&bytes));
                if !data_frames(&collected).is_empty() {
                    break;
                }
            }
            Ok(Some(Err(_))) => break,
            Ok(None) => break,
            Err(_) => continue,
        }
    }

    // The route is REGISTERED — it did not fall through to the SPA catch-all. This, not a
    // status code, is what proves registration in this codebase (common::Resp::assert_registered).
    let resp = common::Resp {
        status,
        body: collected.clone(),
        content_type: content_type.clone(),
    };
    resp.assert_registered();
    assert_eq!(status, 200);
    assert!(
        content_type
            .as_deref()
            .is_some_and(|c| c.starts_with("text/event-stream")),
        "GET /api/events must serve an SSE stream; content-type was {content_type:?}"
    );

    let frames = data_frames(&collected);
    assert!(
        !frames.is_empty(),
        "expected at least one payload frame; collected: {collected:?}"
    );

    for frame in &frames {
        let data = frame_data(frame);

        // The new shape.
        serde_json::from_str::<SequencedEvent>(&data).unwrap_or_else(|e| {
            panic!("frame payload must deserialize as SequencedEvent ({e}): {data:?}")
        });

        // NOT the old record-patch shape: no LogMsg event name...
        for name in [
            "json_patch",
            "stdout",
            "stderr",
            "session_id",
            "finished",
            "refresh_required",
        ] {
            assert!(
                !frame.contains(&format!("event: {name}")),
                "frame carries the removed LogMsg SSE event name {name:?}: {frame:?}"
            );
        }
        // ...and the payload is not a JSON patch array.
        assert!(
            serde_json::from_str::<Vec<serde_json::Value>>(&data).is_err(),
            "frame payload deserialized as a JSON-patch array — the removed record-patch \
             shape: {data:?}"
        );
    }
}

/// Test 6: sse_delivers_an_event_from_a_real_task_write
/// - Create a project and task via the REAL model write paths
/// - Subscribe to /api/events BEFORE the write
/// - Create the task
/// - Assert a `task_created` event carrying THAT task_id arrives on the SSE stream
/// - This is the full-path proof: model write → journal → tailer → bus → SSE
#[tokio::test]
#[serial_test::serial]
async fn sse_delivers_an_event_from_a_real_task_write() {
    let h = common::HiveHarness::hive_absent().await;

    // Create a project first
    let project_id = Uuid::new_v4();
    let pool = &h.deployment().db().pool;
    let create_project = CreateProject {
        name: "test-project".to_string(),
        git_repo_path: "/tmp/test-project".to_string(),
        use_existing_repo: true,
        clone_url: None,
        setup_script: None,
        dev_script: None,
        cleanup_script: None,
        copy_files: None,
    };
    Project::create(pool, &create_project, project_id)
        .await
        .expect("failed to create project");

    // Subscribe to events BEFORE writing the task
    let client = reqwest::Client::new();
    let url = format!("http://{}/api/events", h.addr());
    let res = client.get(&url).send().await.unwrap();
    assert_eq!(res.status(), 200);

    let mut event_stream = res.bytes_stream();

    // Give the subscription time to establish
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Now create a task via the real Model::create path
    let task_id = Uuid::new_v4();
    let create_task = CreateTask {
        project_id,
        title: "real-task".to_string(),
        description: Some("created via model layer".to_string()),
        status: Some(TaskStatus::Todo),
        parent_task_id: None,
        image_ids: None,
        shared_task_id: None,
    };
    Task::create(pool, &create_task, task_id)
        .await
        .expect("failed to create task");

    // The event we require: `task_created` carrying THIS task's id. Anything weaker (a frame
    // count, or "some id: line arrived") passes with the write path broken.
    let matches_write = |buf: &str| {
        data_frames(buf).into_iter().any(|frame| {
            match serde_json::from_str::<SequencedEvent>(&frame_data(frame)) {
                Ok(SequencedEvent {
                    event: NodeEvent::TaskCreated { task_id: id, .. },
                    ..
                }) => id == task_id,
                _ => false,
            }
        })
    };

    let mut collected = String::new();
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);

    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(tokio::time::Duration::from_millis(100), event_stream.next())
            .await
        {
            Ok(Some(Ok(bytes))) => {
                collected.push_str(&String::from_utf8_lossy(&bytes));
                if matches_write(&collected) {
                    break;
                }
            }
            Ok(Some(Err(_))) => break,
            Ok(None) => break,
            Err(_) => continue,
        }
    }

    assert!(
        matches_write(&collected),
        "a task_created event carrying task_id {task_id} must arrive on the SSE stream \
         (full-path proof: model write → journal → tailer → bus → SSE); collected: {collected:?}"
    );
}

/// Test 7: mid_stream_error_emits_terminal_error_frame_then_ends
///
/// Pins the R1 dictate: a failure that happens INSIDE the stream must reach the client as a
/// terminal `event: error` frame, after which the stream ENDS. An `Err` yielded into `SseBody`
/// would instead make hyper abort the chunked body (axum-0.8.8 `response/sse.rs:130`) — a silent
/// close; and a stream that emits the frame but stays alive would hang on keep-alives forever.
///
/// Fault injection uses this run's established table-rename technique
/// (`crates/services/src/services/event_bus/tailer.rs:581`): with `event_journal` renamed away,
/// the subscription's own `high_water_mark` read fails on its FIRST poll, inside the stream. The
/// `cursor=0` leg is what forces that read into the stream — the handler itself only reads the
/// journal on the no-cursor path, and its failure there is an HTTP error, not a frame.
#[tokio::test]
#[serial_test::serial]
async fn mid_stream_error_emits_terminal_error_frame_then_ends() {
    let h = common::HiveHarness::hive_absent().await;
    let pool = h.deployment().db().pool.clone();

    let seeded = journal_events(&pool, 3).await;
    assert_eq!(seeded, vec![1, 2, 3], "unexpected seeded seqs: {seeded:?}");

    // Poison the journal: the table the subscription reads no longer exists under that name.
    sqlx::query("ALTER TABLE event_journal RENAME TO event_journal_poisoned")
        .execute(&pool)
        .await
        .expect("failed to rename event_journal");

    let client = reqwest::Client::new();
    let url = format!("http://{}/api/events?cursor=0", h.addr());
    let res = client.get(&url).send().await.unwrap();

    // The response head is already on the wire before the stream is first polled, so the failure
    // cannot become a status code — it has to arrive as a frame.
    let status = res.status().as_u16();

    let mut event_stream = res.bytes_stream();
    let mut collected = String::new();
    let mut stream_ended = false;
    let mut body_error: Option<String> = None;

    // Bounded read. The frame budget is a guard, not an expectation: a route that emits the error
    // frame without terminating re-polls the failing journal read immediately and would otherwise
    // spin out frames for the whole deadline.
    const FRAME_BUDGET: usize = 8;
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(10);

    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(tokio::time::Duration::from_millis(100), event_stream.next())
            .await
        {
            Ok(Some(Ok(bytes))) => {
                collected.push_str(&String::from_utf8_lossy(&bytes));
                if data_frames(&collected).len() > FRAME_BUDGET {
                    break;
                }
            }
            Ok(Some(Err(e))) => {
                body_error = Some(e.to_string());
                break;
            }
            Ok(None) => {
                stream_ended = true;
                break;
            }
            Err(_) => continue,
        }
    }

    // Teardown BEFORE the assertions so a failing assertion cannot leave the DB poisoned. The
    // harness gives every test its own temp-dir database, so this is belt-and-braces: it keeps
    // the schema coherent for the deployment's own shutdown path (WAL checkpoint, pool close).
    sqlx::query("ALTER TABLE event_journal_poisoned RENAME TO event_journal")
        .execute(&pool)
        .await
        .expect("failed to restore event_journal");

    assert_eq!(
        status, 200,
        "the failure must arrive as a frame, not a status"
    );
    assert_eq!(
        body_error, None,
        "the SSE body must END cleanly, not abort — an Err through SseBody is the silent close \
         R1 forbids"
    );

    // The stream-end assertion is FIRST on purpose: it is the one the R1 `Done` transition owns,
    // and the one the dictated red proof must trip.
    assert!(
        stream_ended,
        "the stream must END after the terminal error frame; it was still open at the deadline \
         (a keep-alive-only hang is a failure). collected: {collected:?}"
    );

    let frames = data_frames(&collected);
    assert_eq!(
        frames.len(),
        1,
        "expected exactly one frame — the terminal error frame — and nothing after it; \
         collected: {collected:?}"
    );
    assert!(
        frames[0].lines().any(|l| l.trim_end() == "event: error"),
        "the terminal frame must be an `error` event; frame: {:?}",
        frames[0]
    );
    assert!(
        !frame_data(frames[0]).is_empty(),
        "the terminal error frame must carry a diagnostic payload; frame: {:?}",
        frames[0]
    );
}

/// Test 8: no_cursor_with_unreadable_journal_returns_500
///
/// The no-cursor path reads the journal high-water mark EAGERLY in the handler, so an unreadable
/// journal must surface as an HTTP 500 before any stream starts. This is the counterpart to
/// test 7: there the `cursor=0` leg forces the journal read INSIDE the stream (terminal error
/// frame); here the handler's own read fails and the client must get a status code, not a
/// connection that dies on first poll.
#[tokio::test]
#[serial_test::serial]
async fn no_cursor_with_unreadable_journal_returns_500() {
    let h = common::HiveHarness::hive_absent().await;
    let pool = h.deployment().db().pool.clone();

    sqlx::query("ALTER TABLE event_journal RENAME TO event_journal_poisoned")
        .execute(&pool)
        .await
        .expect("failed to rename event_journal");

    let client = reqwest::Client::new();
    let url = format!("http://{}/api/events", h.addr());
    let response = client.get(&url).send().await;

    // Restore the table BEFORE any assertion can panic, so harness teardown
    // always sees the schema it expects.
    let restore_result = sqlx::query("ALTER TABLE event_journal_poisoned RENAME TO event_journal")
        .execute(&pool)
        .await;

    let res = response.expect("failed to request /api/events");
    restore_result.expect("failed to restore event_journal");

    assert_eq!(
        res.status().as_u16(),
        500,
        "an unreadable journal on the no-cursor path must be an HTTP error, not a stream"
    );
}

/// Test 9: a negative cursor is rejected with 400 before any journal read or subscription.
/// A journal cursor cannot be negative; cursor=-1 previously behaved like a full
/// retained-history replay instead of an input-validation error.
#[tokio::test]
#[serial_test::serial]
async fn negative_cursor_returns_400() {
    let h = common::HiveHarness::hive_absent().await;

    let res = reqwest::Client::new()
        .get(format!("http://{}/api/events?cursor=-1", h.addr()))
        .send()
        .await
        .expect("failed to request /api/events");

    assert_eq!(
        res.status().as_u16(),
        400,
        "a negative cursor must be an input-validation error, not a replay"
    );
}
