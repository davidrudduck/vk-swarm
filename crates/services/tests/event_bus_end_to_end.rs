//! End-to-end integration suite for the event bus seam: `commit -> tailer -> broadcast ->
//! subscribe_from`.
//!
//! Task 013's and 016's suites (277 lib tests, ten adversarial panels) each prove one half of
//! this path in isolation: every `subscribe_from` test in `event_bus/mod.rs` hand-drives
//! `sender()` with a fabricated `SequencedEvent`, and every `tailer.rs` test observes a raw
//! `sender().subscribe()` receiver. Nothing drives the real, whole path — a committed row
//! reaching a subscriber purely because it was written through the model API and read back
//! through the public streaming API. That is this file's only job.
//!
//! **Binding constraint, honoured by every test below:** no test here ever constructs a
//! `SequencedEvent` by hand, and none ever calls `sender().send(..)`. Events enter ONLY via
//! [`event_journal::append`] inside a real transaction that is (or is not, for the rollback
//! test) committed on the real pool, and are observed ONLY via [`EventBus::subscribe_from`].
//!
//! **Determinism without fixed sleeps.** `EventBus::subscribe_from`'s state machine
//! (`crates/services/src/services/event_bus/mod.rs`) reads the journal directly in its
//! `Initializing` arm; after that read is exhausted (`ReplayingJournal` with
//! `index == events.len()`), the ONLY remaining path to the subscriber is the live broadcast
//! channel fed by the background tailer — UNLESS the channel overruns (`Lagged`), which
//! re-enters a journal read via the refill arm. That refill arm cannot fire in this suite: every
//! bus here uses capacity 64 against at most 10 events per test, far below what it takes to
//! overrun the channel. So, in this suite specifically, once the replay window is drained there
//! is exactly one way forward. Several tests below exploit this structurally: they drain a
//! "warm-up" commit first (via `expect_next_seq`) to provably exhaust the replay window, so that
//! the event under test is *guaranteed* to travel through the tailer rather than through
//! subscribe_from's own direct read — deterministically, not by scheduling luck. See the
//! decisions-ledger for task 017 for the full reasoning.
//!
//! All waits are deadline-based (`tokio::time::timeout` against a generous, fixed budget), never
//! a bare `sleep` used as the pass condition — this module has a documented history of flakiness
//! from fixed-sleep timing (see the ledger for tasks 013/016).

use std::time::Duration;

use db::models::{
    event::{NodeEvent, SequencedEvent},
    event_journal,
    task::TaskStatus,
};
use futures::stream::{BoxStream, StreamExt};
use services::services::event_bus::{EventBus, EventBusError};
use sqlx::SqlitePool;
use tokio::time::Instant;
use uuid::Uuid;

/// The concrete stream type `EventBus::subscribe_from` returns.
type EventStream = BoxStream<'static, Result<SequencedEvent, EventBusError>>;

/// Generous bound for "this must eventually arrive" waits. The tailer polls every 75ms (see the
/// task 013 ledger entry pinning `TAIL_INTERVAL`); 10s is over 130 poll cycles of headroom.
const DEADLINE: Duration = Duration::from_secs(10);

/// Bound for a single live-delivery wait — after a commit that must travel through the tailer to
/// reach a live subscriber. Matches the 30s deadline `event_bus/mod.rs`'s own
/// `the_bus_publishes_a_committed_row_exactly_once` and
/// `event_bus_tailer_health_tracks_the_bus_s_own_tailer` already use for the equivalent class of
/// wait, rather than inventing a new magic number.
///
/// **Task 018 retires this constant's former split from a narrower, retry-based budget.** Before
/// task 018, a single live-delivery wait right after `EventBus::new()` was unsound at any fixed
/// deadline: `new()` dropped the tailer's readiness receiver, so a row committed immediately
/// afterward could legitimately land before the tailer's initial `high_water_mark` read resolved
/// and be silently, permanently dropped — the file used a separate retry-based probe
/// (`prove_tailer_is_live`, since deleted) to paper over exactly that window. `EventBus::new()`
/// now awaits the tailer's readiness signal (bounded by its own internal timeout) before
/// returning at all, so the tailer's cursor is fixed before `new()` ever hands back control — see
/// the decisions-ledger for task 018. With that race closed at the source, one deadline serves
/// every live-delivery wait in this suite, first commit after construction or otherwise.
const WARM_LIVE_DEADLINE: Duration = Duration::from_secs(30);

/// Bound for "this must NOT arrive" checks — long enough that a still-active publisher has no
/// plausible excuse for silence (many multiples of `TAIL_INTERVAL`), short enough not to make a
/// failing suite slow. Matches the window `shutdown_stops_the_tailer` in `event_bus/mod.rs` uses
/// for the same kind of negative assertion.
const QUIET_WINDOW: Duration = Duration::from_secs(2);

/// Commits one `TaskCreated` event via the real model API inside a real transaction and returns
/// its assigned seq plus the `task_id` and `project_id` that identify its body. This is the ONLY
/// way an event enters the journal in this suite — see the file header's binding constraint.
///
/// Both id fields are returned, not just `task_id` (F4, panel 11 on task 017): the sibling
/// assertion in `tailer.rs` checks both deliberately, "so a mutation that fabricates only one of
/// the two fields has nowhere to hide either" — this suite diverged from that without recording
/// why, and this task fixes the divergence.
async fn commit_task_created(pool: &SqlitePool) -> (i64, Uuid, Uuid) {
    let mut tx = pool.begin().await.unwrap();
    let task_id = Uuid::new_v4();
    let project_id = Uuid::new_v4();
    let event = NodeEvent::TaskCreated {
        task_id,
        project_id,
    };
    let seq = event_journal::append(&mut *tx, &event).await.unwrap();
    tx.commit().await.unwrap();
    (seq, task_id, project_id)
}

/// Awaits the NEXT item `stream` produces and asserts it carries `expected_seq`.
///
/// Deliberately does NOT skip mismatched events the way a "wait for this seq eventually" helper
/// would. Every test in this suite drives a fully deterministic, known sequence of commits, so
/// an unexpected seq (or an error, or a closed stream, or a timeout) is always a real defect —
/// silently skipping past it would be exactly the kind of hollow assertion this task exists to
/// avoid (in particular it would make the "no history replay" half of test 4 vacuous).
async fn expect_next_seq(
    stream: &mut EventStream,
    expected_seq: i64,
    deadline: Instant,
) -> SequencedEvent {
    let remaining_at_start = deadline.saturating_duration_since(Instant::now());
    match tokio::time::timeout(remaining_at_start, stream.next()).await {
        Ok(Some(Ok(ev))) => {
            assert_eq!(
                ev.seq, expected_seq,
                "expected seq {expected_seq} next on the stream, got seq {} instead",
                ev.seq
            );
            ev
        }
        Ok(Some(Err(e))) => panic!("expected seq {expected_seq}, but the stream errored: {e:?}"),
        Ok(None) => panic!("expected seq {expected_seq}, but the stream closed"),
        Err(_) => panic!("timed out after {remaining_at_start:?} waiting for seq {expected_seq}"),
    }
}

/// Asserts `stream` stays silent for `window` — proving no further event (in particular no
/// duplicate, and no leaked rolled-back/history row) arrives after the point already checked.
async fn assert_quiet(stream: &mut EventStream, window: Duration) {
    match tokio::time::timeout(window, stream.next()).await {
        Err(_) => {}   // timed out: silence, as required
        Ok(None) => {} // stream closed: also fine, nothing further can arrive
        Ok(Some(Ok(ev))) => panic!(
            "expected silence for {window:?}, but seq {} arrived unexpectedly",
            ev.seq
        ),
        Ok(Some(Err(e))) => {
            panic!("expected silence for {window:?}, but the stream errored: {e:?}")
        }
    }
}

/// Asserts `ev` is a `TaskCreated` event carrying `expected_task_id` AND `expected_project_id` —
/// the "full body intact" check for the single-variant tests.
///
/// Both fields are asserted (F4, panel 11 on task 017): asserting `task_id` alone left
/// `project_id` free to be fabricated, matching neither the sibling assertion in `tailer.rs`
/// (`delivered`/`RowId`, which pins both) nor this file's own binding constraint that a committed
/// row's FULL body is what these tests exist to verify.
fn assert_task_created_body(
    ev: &SequencedEvent,
    expected_task_id: Uuid,
    expected_project_id: Uuid,
) {
    match &ev.event {
        NodeEvent::TaskCreated {
            task_id,
            project_id,
        } => {
            assert_eq!(
                *task_id, expected_task_id,
                "seq {} carried a body whose task_id does not match what was committed",
                ev.seq
            );
            assert_eq!(
                *project_id, expected_project_id,
                "seq {} carried a body whose project_id does not match what was committed",
                ev.seq
            );
        }
        other => panic!(
            "expected a TaskCreated event at seq {}, got {other:?}",
            ev.seq
        ),
    }
}

/// A committed row reaches a subscriber that is already live (subscribed before the commit).
///
/// The warm-up commit/receive pair is not part of the property under test — it exists solely to
/// provably exhaust `subscribe_from`'s one-time journal replay window (see the file header)
/// before the row under test is committed, so that row is GUARANTEED to travel
/// `commit -> tailer -> broadcast -> subscribe_from`'s live arm rather than
/// `subscribe_from`'s own direct read, deterministically. The row itself is then a single
/// deterministic commit consumed with the strict, exact-seq `expect_next_seq` — not a retry loop
/// (task 018 deleted `prove_tailer_is_live`, the retry helper this test used to call here):
/// `EventBus::new()` itself now awaits the tailer's readiness signal before returning, so the
/// tailer's cursor is already fixed by the time `new()` hands back control, well before this
/// test's first commit. There is no startup race left for a retry to paper over.
#[tokio::test]
async fn a_committed_row_reaches_a_live_subscriber() {
    let (pool, _temp_dir) = db::test_utils::create_test_pool_with_migrations().await;
    let bus = EventBus::new(pool.clone(), 64).await;
    let mut stream: EventStream = bus.subscribe_from(0).unwrap();

    let (warm_seq, _warm_task_id, _warm_project_id) = commit_task_created(&pool).await;
    expect_next_seq(&mut stream, warm_seq, Instant::now() + DEADLINE).await;

    let (seq, task_id, project_id) = commit_task_created(&pool).await;
    let ev = expect_next_seq(&mut stream, seq, Instant::now() + WARM_LIVE_DEADLINE).await;
    assert_task_created_body(&ev, task_id, project_id);

    bus.shutdown().await;
}

/// A subscriber that joins AFTER events already exist replays them from its cursor, then hands
/// off to live delivery with no gap and no duplicate across the boundary.
///
/// **Task 018 retires this test's declared residual.** Before task 018, this test's soundness for
/// the handoff commit rested on `EventBus::new()` having had three prior commits' worth of real
/// elapsed time to establish its tailer — correctly documented here as "improbable, not
/// structurally immune" to the same startup race `prove_tailer_is_live` existed to paper over
/// elsewhere in this file. That residual is now retired outright, not merely reduced:
/// `EventBus::new()` itself awaits the tailer's readiness signal before returning, so the
/// tailer's cursor is fixed before ANY commit in this test happens, elapsed time or not. The
/// handoff below uses a single deterministic commit with the strict, exact-seq `expect_next_seq`,
/// and does so structurally now rather than probabilistically. Given the generous
/// [`WARM_LIVE_DEADLINE`] this keeps the task's dictated "no gap" property exact rather than
/// weakened to "eventually, at or after."
#[tokio::test]
async fn a_subscriber_that_joins_late_replays_from_its_cursor_then_goes_live() {
    let (pool, _temp_dir) = db::test_utils::create_test_pool_with_migrations().await;
    let bus = EventBus::new(pool.clone(), 64).await;

    // Commit three events BEFORE subscribing at all.
    let mut committed = Vec::with_capacity(3);
    for _ in 0..3 {
        committed.push(commit_task_created(&pool).await);
    }

    let mut stream: EventStream = bus.subscribe_from(0).unwrap();

    // Replay must return exactly the three, strictly in seq order.
    let replay_deadline = Instant::now() + DEADLINE;
    for (expected_seq, expected_task_id, expected_project_id) in &committed {
        let ev = expect_next_seq(&mut stream, *expected_seq, replay_deadline).await;
        assert_task_created_body(&ev, *expected_task_id, *expected_project_id);
    }

    // Handoff: this commit happens strictly AFTER the 3-item replay batch has been fully
    // drained. subscribe_from never re-reads the journal past its one Initializing read (see
    // the file header), so the ONLY remaining path to the subscriber for this row is
    // commit -> tailer -> broadcast -> live arm — this IS the handoff this test exists to prove.
    let (seq4, task_id4, project_id4) = commit_task_created(&pool).await;
    let ev = expect_next_seq(&mut stream, seq4, Instant::now() + WARM_LIVE_DEADLINE).await;
    assert_task_created_body(&ev, task_id4, project_id4);

    // No gap (already proven: seq4 arrived as the immediate next item, not something later).
    // This does NOT additionally prove "no duplicate across the boundary" (F2, panel 11 on task
    // 017) — `subscribe_from`'s Live arm drops anything with `ev.seq <= state.last`
    // (`event_bus/mod.rs:200`), so a duplicate is consumed before the stream ever yields it and
    // this assertion cannot observe one either way. What this silence DOES prove is that nothing
    // further — in particular no belated replay of seqs 1..=4 — arrives after the handoff. The
    // exactly-once property genuinely lives in `the_bus_publishes_a_committed_row_exactly_once`
    // (`event_bus/mod.rs`), which subscribes directly to the broadcast channel rather than through
    // `subscribe_from`'s dedupe.
    assert_quiet(&mut stream, QUIET_WINDOW).await;

    bus.shutdown().await;
}

/// A rolled-back transaction reaches no subscriber. Journal-first means an uncommitted event
/// must be invisible END TO END (through the real `subscribe_from` API), not merely absent from
/// the raw journal table — the latter is already covered by task 004's `rollback_journals_nothing`.
#[tokio::test]
async fn a_rolled_back_transaction_reaches_no_subscriber() {
    let (pool, _temp_dir) = db::test_utils::create_test_pool_with_migrations().await;
    let bus = EventBus::new(pool.clone(), 64).await;

    // Append inside a transaction, then roll back by dropping it without committing — the same
    // idiom `crates/db/src/models/event_journal/mod.rs`'s `rollback_journals_nothing` uses.
    {
        let mut tx = pool.begin().await.unwrap();
        let event = NodeEvent::TaskCreated {
            task_id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
        };
        let _ = event_journal::append(&mut *tx, &event).await.unwrap();
        // `tx` drops here without `.commit()` — rollback.
    }

    // Commit a different, real event.
    let (seq, task_id, project_id) = commit_task_created(&pool).await;

    // Subscribe AFTER both operations, so the very first thing observed must be the committed
    // one — proving the rolled-back write left no trace anywhere subscribe_from can see it.
    let mut stream: EventStream = bus.subscribe_from(0).unwrap();
    let ev = expect_next_seq(&mut stream, seq, Instant::now() + DEADLINE).await;
    assert_task_created_body(&ev, task_id, project_id);

    // Nothing else arrives — in particular no belated appearance of the rolled-back row.
    assert_quiet(&mut stream, QUIET_WINDOW).await;

    bus.shutdown().await;
}

/// A new `EventBus` on the same pool resumes without replaying history: a subscriber started at
/// the pre-restart high-water mark sees only events committed after the restart, at their
/// correct absolute seq (not renumbered).
///
/// Two post-restart commits are involved, not one — each defends a different half of the
/// property, and each is established differently on purpose:
///
/// - The FIRST is a single deterministic commit, consumed with the strict, exact-seq
///   `expect_next_seq` — same device as test 1's warm-up. `bus2.subscribe_from(high_water)`
///   hasn't been polled yet at this point, so this commit is guaranteed to be picked up by
///   `subscribe_from`'s own one-shot `Initializing` read (`read_range(high_water, high_water+1)`),
///   independent of whether the tailer has started yet. Its EXACT seq (`high_water + 1`, not
///   merely "greater than") is the defence against vacuity: if a regression made the replay read
///   use the wrong lower bound, the first item received would be seq 1 or 2, and the assertion
///   fails loudly with that value. (This does NOT guard against a tailer that starts publishing
///   from 0 instead of the high-water mark — that is unobservable through `subscribe_from` by
///   construction, because its Live arm drops anything with `ev.seq <= state.last`; that property
///   belongs to, and is covered by, task 013's `tailer_resumes_from_its_high_water_on_restart`.)
///   Consuming it exhausts `subscribe_from`'s replay window (see the file header), which is what
///   makes the second commit below provable.
/// - The SECOND is established via a plain commit and `expect_next_seq` at
///   [`WARM_LIVE_DEADLINE`], exactly like the first — not the retry-based `prove_tailer_is_live`
///   this test used before task 018 deleted it. `bus2 = EventBus::new(...)` already awaited the
///   RESTARTED tailer's readiness before returning, so there is no startup race left for a retry
///   to paper over even on a freshly-constructed bus. With the replay window already exhausted by
///   the first commit, this is the half that actually proves the RESTARTED bus's tailer resumes
///   live delivery.
#[tokio::test]
async fn a_new_bus_on_the_same_pool_resumes_without_replaying_history() {
    let (pool, _temp_dir) = db::test_utils::create_test_pool_with_migrations().await;

    let bus1 = EventBus::new(pool.clone(), 64).await;
    let _pre_restart_1 = commit_task_created(&pool).await; // seq 1
    let _pre_restart_2 = commit_task_created(&pool).await; // seq 2

    let high_water = event_journal::high_water_mark(&pool).await.unwrap();
    assert_eq!(
        high_water, 2,
        "two pre-restart commits should leave the high-water mark at 2"
    );

    // Stop bus1's tailer explicitly before spawning bus2 on the same pool. A bare `drop` only
    // DETACHES the tailer's background task rather than stopping it (see the task 013
    // decisions-ledger); `shutdown()` is the documented way to stop it, and this avoids leaving
    // an orphaned tailer polling the pool for the rest of the test process. (bus1's tailer
    // publishes onto bus1's OWN broadcast channel, a separate object from bus2's, so this is
    // hygiene rather than a correctness requirement for what follows.)
    bus1.shutdown().await;

    let bus2 = EventBus::new(pool.clone(), 64).await;
    let mut stream: EventStream = bus2.subscribe_from(high_water).unwrap();

    let (first_post_seq, first_post_task_id, first_post_project_id) =
        commit_task_created(&pool).await;
    assert_eq!(
        first_post_seq,
        high_water + 1,
        "the first post-restart commit should continue the seq sequence, not reset it"
    );
    let first = expect_next_seq(&mut stream, first_post_seq, Instant::now() + DEADLINE).await;
    assert_task_created_body(&first, first_post_task_id, first_post_project_id);

    let (second_seq, second_task_id, second_project_id) = commit_task_created(&pool).await;
    assert!(
        second_seq > first_post_seq,
        "the second post-restart event (seq {second_seq}) must be strictly after the first (seq \
         {first_post_seq})"
    );
    let second =
        expect_next_seq(&mut stream, second_seq, Instant::now() + WARM_LIVE_DEADLINE).await;
    assert_task_created_body(&second, second_task_id, second_project_id);

    // No stray replay of the two pre-restart events sneaks in afterward. A duplicate of either
    // post-restart event is separately, and more strongly, ruled out already: `subscribe_from`'s
    // Live arm drops anything with `ev.seq <= state.last` (`event_bus/mod.rs:200`), so a
    // duplicate is consumed before the stream ever yields it and cannot arrive here to be
    // observed either way (F2, panel 11 on task 017) — the exactly-once property genuinely lives
    // in `the_bus_publishes_a_committed_row_exactly_once` (`event_bus/mod.rs`).
    assert_quiet(&mut stream, QUIET_WINDOW).await;

    bus2.shutdown().await;
}

/// Every `NodeEvent` variant survives the full round trip — journal write, through
/// `subscribe_from` — with its body intact, compared as full serialized JSON per the task's own
/// instruction (`NodeEvent` does not derive `PartialEq`).
///
/// The unit suite (`crates/db/src/models/event.rs`) proves the tailer's serde contract in
/// isolation (`event_type_matches_serde_tag_for_every_variant`); this proves nothing between the
/// journal and the subscriber narrows or corrupts any variant on the real path.
#[tokio::test]
async fn every_event_variant_survives_the_full_round_trip() {
    let (pool, _temp_dir) = db::test_utils::create_test_pool_with_migrations().await;
    let bus = EventBus::new(pool.clone(), 64).await;

    let variants = vec![
        NodeEvent::TaskCreated {
            task_id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
        },
        NodeEvent::TaskStatusChanged {
            task_id: Uuid::new_v4(),
            old_status: TaskStatus::Todo,
            new_status: TaskStatus::InProgress,
        },
        NodeEvent::TaskDeleted {
            task_id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
        },
        NodeEvent::AttemptStarted {
            task_id: Uuid::new_v4(),
            attempt_id: Uuid::new_v4(),
            execution_process_id: Uuid::new_v4(),
            executor: "claude".into(),
        },
        NodeEvent::AttemptFinished {
            task_id: Uuid::new_v4(),
            attempt_id: Uuid::new_v4(),
            execution_process_id: Uuid::new_v4(),
            executor: "claude".into(),
            exit_code: 0,
        },
        NodeEvent::AttemptFailed {
            task_id: Uuid::new_v4(),
            attempt_id: Uuid::new_v4(),
            execution_process_id: Uuid::new_v4(),
            executor: "claude".into(),
            reason: "boom".into(),
        },
        NodeEvent::HiveConnected {},
        NodeEvent::HiveDisconnected {
            reason: "network".into(),
        },
        NodeEvent::ReconcileCompleted { entity_count: 42 },
    ];
    assert_eq!(
        variants.len(),
        9,
        "a NodeEvent variant was added without extending this table"
    );

    let seqs: Vec<i64> = {
        let mut tx = pool.begin().await.unwrap();
        let mut seqs = Vec::with_capacity(variants.len());
        for event in &variants {
            seqs.push(event_journal::append(&mut *tx, event).await.unwrap());
        }
        tx.commit().await.unwrap();
        seqs
    };

    let mut stream: EventStream = bus.subscribe_from(0).unwrap();
    let deadline = Instant::now() + DEADLINE;
    for (expected_seq, expected_event) in seqs.iter().zip(variants.iter()) {
        let ev = expect_next_seq(&mut stream, *expected_seq, deadline).await;
        assert_eq!(
            serde_json::to_value(&ev.event).unwrap(),
            serde_json::to_value(expected_event).unwrap(),
            "seq {expected_seq} round-tripped with a body that does not match what was committed"
        );
    }

    bus.shutdown().await;
}
