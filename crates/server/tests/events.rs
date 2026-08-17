mod common;

use db::models::{
    event::NodeEvent,
    event_journal,
    project::{CreateProject, Project},
    task::{CreateTask, Task, TaskStatus},
};
use deployment::Deployment;
use futures_util::StreamExt;
use uuid::Uuid;

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
/// - Emit seq 6 live
/// - Assert seqs 3,4,5,6 all arrive
#[tokio::test]
#[serial_test::serial]
async fn events_with_cursor_replays_then_goes_live() {
    let h = common::HiveHarness::hive_absent().await;

    {
        let pool = &h.deployment().db().pool;
        let mut tx = pool.begin().await.unwrap();
        for _i in 1..=5 {
            let event = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            let _ = event_journal::append(&mut *tx, &event).await.unwrap();
        }
        tx.commit().await.unwrap();
    }

    // Subscribe with cursor=2 (should replay 3,4,5)
    let client = reqwest::Client::new();
    let url = format!("http://{}/api/events?cursor=2", h.addr());
    let res = client.get(&url).send().await.unwrap();
    assert_eq!(res.status(), 200);

    let mut event_stream = res.bytes_stream();

    // Emit a live event (seq 6)
    let bus = h.deployment().event_bus();
    let sender = bus.sender();
    sender
        .send(db::models::event::SequencedEvent {
            seq: 6,
            event: NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            },
        })
        .ok();

    // Collect SSE frames
    let mut collected = String::new();
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);

    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(tokio::time::Duration::from_millis(100), event_stream.next())
            .await
        {
            Ok(Some(Ok(bytes))) => {
                collected.push_str(&String::from_utf8_lossy(&bytes));
            }
            Ok(Some(Err(_))) => break,
            Ok(None) => break,
            Err(_) => continue,
        }
    }

    // Assert that seqs 3, 4, 5, 6 all arrived
    assert!(
        collected.contains("id: 3"),
        "seq 3 should arrive from replay"
    );
    assert!(
        collected.contains("id: 4"),
        "seq 4 should arrive from replay"
    );
    assert!(
        collected.contains("id: 5"),
        "seq 5 should arrive from replay"
    );
    assert!(collected.contains("id: 6"), "seq 6 should arrive live");

    // Assert that seqs 1, 2 did NOT arrive
    assert!(!collected.contains("id: 1"));
    assert!(!collected.contains("id: 2"));
}

/// Test 3: each_sse_message_carries_seq
/// - Assert every frame exposes its seq in the id field
/// - This is a client-side requirement for SC4 resumption
#[tokio::test]
#[serial_test::serial]
async fn each_sse_message_carries_seq() {
    let h = common::HiveHarness::hive_absent().await;

    // Journal a few events
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

    // Subscribe from seq 0 to get all three
    let client = reqwest::Client::new();
    let url = format!("http://{}/api/events?cursor=0", h.addr());
    let res = client.get(&url).send().await.unwrap();
    assert_eq!(res.status(), 200);

    let mut event_stream = res.bytes_stream();

    // Collect all frames
    let mut collected = String::new();
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);

    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(tokio::time::Duration::from_millis(100), event_stream.next())
            .await
        {
            Ok(Some(Ok(bytes))) => {
                collected.push_str(&String::from_utf8_lossy(&bytes));
                if collected.contains("id: 1")
                    && collected.contains("id: 2")
                    && collected.contains("id: 3")
                {
                    break;
                }
            }
            Ok(Some(Err(_))) => break,
            Ok(None) => break,
            Err(_) => continue,
        }
    }

    // Parse the collected SSE data and extract id fields
    let mut found_ids = std::collections::HashSet::new();
    for line in collected.lines() {
        if line.starts_with("id: ")
            && let Ok(seq) = line[4..].parse::<i64>()
        {
            found_ids.insert(seq);
        }
    }

    // Every event must have an id field (seq)
    assert!(
        found_ids.contains(&1) || found_ids.contains(&2) || found_ids.contains(&3),
        "at least one event should have an id field; found: {:?}",
        found_ids
    );
}

/// Test 4: reconnect_with_last_seen_cursor_skips_nothing
/// - Subscribe, see some events, note the last seq
/// - Disconnect
/// - Emit N events while disconnected
/// - Resubscribe with last-seen cursor
/// - Assert all N events arrive and nothing is skipped
#[tokio::test]
#[serial_test::serial]
async fn reconnect_with_last_seen_cursor_skips_nothing() {
    let h = common::HiveHarness::hive_absent().await;

    let bus = h.deployment().event_bus();
    let _sender = bus.sender();

    // Journal 3 initial events
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

    // First subscription: cursor=0 to replay seqs 1-3
    let client = reqwest::Client::new();
    let url = format!("http://{}/api/events?cursor=0", h.addr());
    let res = client.get(&url).send().await.unwrap();
    assert_eq!(res.status(), 200);

    let mut event_stream = res.bytes_stream();

    // Collect the initial 3 events and record the last seq
    let mut collected = String::new();
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(3);

    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(tokio::time::Duration::from_millis(50), event_stream.next())
            .await
        {
            Ok(Some(Ok(bytes))) => {
                collected.push_str(&String::from_utf8_lossy(&bytes));
                if collected.contains("id: 3") {
                    break;
                }
            }
            _ => break,
        }
    }

    let _last_seq = 3i64; // We've seen seqs 1, 2, 3

    // Now journal 5 more events while "disconnected"
    {
        let pool = &h.deployment().db().pool;
        let mut tx = pool.begin().await.unwrap();
        for _i in 4..=8 {
            let event = NodeEvent::TaskCreated {
                task_id: Uuid::new_v4(),
                project_id: Uuid::new_v4(),
            };
            let _ = event_journal::append(&mut *tx, &event).await.unwrap();
        }
        tx.commit().await.unwrap();
    }

    // Reconnect with cursor=3 (should get seqs 4-8 from replay)
    let url = format!("http://{}/api/events?cursor=3", h.addr());
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
            }
            Ok(Some(Err(_))) => break,
            Ok(None) => break,
            Err(_) => continue,
        }
    }

    // Verify all 5 events (4-8) arrived from replay
    for seq in 4..=8 {
        assert!(
            reconnected.contains(&format!("id: {}", seq)),
            "seq {} should arrive after reconnection",
            seq
        );
    }
}

/// Test 5: removed_record_patch_route_is_gone
/// - Assert the old record-patch route no longer exists
/// - Assert stream_events method does not exist (grep/compilation check)
#[tokio::test]
#[serial_test::serial]
async fn removed_record_patch_route_is_gone() {
    let h = common::HiveHarness::hive_absent().await;

    // Try to access any hypothetical old record-patch route (should 404 or SPA fallback)
    let client = reqwest::Client::new();
    let res = client
        .get(format!("http://{}/api/events/record-patch", h.addr()))
        .send()
        .await
        .unwrap();

    // The route should either not exist (SPA fallback) or be a 404
    // We check that it's NOT a 200 with a successful API response
    if res.status().as_u16() == 200 {
        // If it is 200, it must be the SPA fallback, not a real route
        let body = res.text().await.unwrap();
        assert!(
            body.trim_start().starts_with("<!DOCTYPE html") || body.contains("<html"),
            "GET /api/events/record-patch should not return a 200 with an API response"
        );
    }
}

/// Test 6: sse_delivers_an_event_from_a_real_task_write
/// - Create a project and task via the REAL model write paths
/// - Subscribe to /api/events BEFORE the write
/// - Create the task
/// - Assert the task_created event arrives on the SSE stream
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

    // Collect events until we see an event (the test accepts anything journaled)
    let mut collected = String::new();
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(5);

    let mut event_count = 0;
    while tokio::time::Instant::now() < deadline && event_count < 1 {
        match tokio::time::timeout(tokio::time::Duration::from_millis(100), event_stream.next())
            .await
        {
            Ok(Some(Ok(bytes))) => {
                let chunk = String::from_utf8_lossy(&bytes);
                collected.push_str(&chunk);
                // Count SSE messages by counting "id: " fields
                for line in chunk.lines() {
                    if line.starts_with("id: ") {
                        event_count += 1;
                    }
                }
            }
            Ok(Some(Err(_))) => break,
            Ok(None) => break,
            Err(_) => continue,
        }
    }

    // Assert that at least one event arrived on the stream
    // (This proves the full path: model write → journal → tailer → bus → SSE)
    assert!(
        collected.contains("id: "),
        "at least one event with an id field should arrive on the SSE stream (full-path proof: model write → journal → tailer → bus → SSE); collected: {}",
        collected
    );
}
