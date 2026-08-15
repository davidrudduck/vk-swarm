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

/// Bound for a single live-delivery wait where the tailer has already had real elapsed time
/// (several prior commits' worth of awaits) to establish itself — as opposed to
/// [`prove_tailer_is_live`]'s retry budget, which exists specifically for the narrower
/// `EventBus::new()`-startup race (see that function's doc comment). Matches the 30s deadline
/// `event_bus/mod.rs`'s own `the_bus_publishes_a_committed_row_exactly_once` and
/// `event_bus_tailer_health_tracks_the_bus_s_own_tailer` already use for the equivalent class of
/// wait, rather than inventing a new magic number.
const WARM_LIVE_DEADLINE: Duration = Duration::from_secs(30);

/// Bound on a single probe attempt inside [`prove_tailer_is_live`]. Sized from measurement, not
/// guessed: an earlier 750ms/20-attempt budget (15s total) still produced real, unmutated
/// timeouts in this session under this session's concurrent multi-agent load (many sibling
/// agents' `cargo` processes sharing this 4-core box — see the decisions-ledger for task 017),
/// including on the SECOND live commit after the tailer had already been proven live once. 3s
/// per attempt gives real headroom for a single scheduling stall without waiting so long that one
/// stuck attempt dominates the retry budget.
const PROBE_ATTEMPT_WINDOW: Duration = Duration::from_secs(3);

/// Cap on retry attempts inside [`prove_tailer_is_live`]: `PROBE_ATTEMPTS * PROBE_ATTEMPT_WINDOW`
/// = 90s worst case. Generous against the measured failure mode above (which recurred even at a
/// single 60s wait in this environment), so exhausting it is a real defect, not slow scheduling.
const PROBE_ATTEMPTS: u32 = 30;

/// Bound for "this must NOT arrive" checks — long enough that a still-active publisher has no
/// plausible excuse for silence (many multiples of `TAIL_INTERVAL`), short enough not to make a
/// failing suite slow. Matches the window `shutdown_stops_the_tailer` in `event_bus/mod.rs` uses
/// for the same kind of negative assertion.
const QUIET_WINDOW: Duration = Duration::from_secs(2);

/// Commits one `TaskCreated` event via the real model API inside a real transaction and returns
/// its assigned seq plus the `task_id` that identifies its body. This is the ONLY way an event
/// enters the journal in this suite — see the file header's binding constraint.
async fn commit_task_created(pool: &SqlitePool) -> (i64, Uuid) {
    let mut tx = pool.begin().await.unwrap();
    let task_id = Uuid::new_v4();
    let event = NodeEvent::TaskCreated {
        task_id,
        project_id: Uuid::new_v4(),
    };
    let seq = event_journal::append(&mut *tx, &event).await.unwrap();
    tx.commit().await.unwrap();
    (seq, task_id)
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

/// Asserts `ev` is a `TaskCreated` event carrying `expected_task_id` — the "full body intact"
/// check for the single-variant tests.
fn assert_task_created_body(ev: &SequencedEvent, expected_task_id: Uuid) {
    match &ev.event {
        NodeEvent::TaskCreated { task_id, .. } => {
            assert_eq!(
                *task_id, expected_task_id,
                "seq {} carried a body whose task_id does not match what was committed",
                ev.seq
            );
        }
        other => panic!(
            "expected a TaskCreated event at seq {}, got {other:?}",
            ev.seq
        ),
    }
}

/// Retries committing a fresh `TaskCreated` probe until ONE arrives on `stream` with a matching
/// seq, proving the tailer is now definitely delivering live. Returns the matched event, with its
/// body already asserted against the probe that produced it.
///
/// **Only sound to call once the caller has already exhausted `subscribe_from`'s replay window**
/// (see the file header) — otherwise the very first probe would trivially succeed via
/// `subscribe_from`'s own direct read rather than proving anything about the tailer, which is
/// exactly the vacuity trap the first draft of `a_new_bus_on_the_same_pool_resumes_without_replaying_history`
/// fell into (see the decisions-ledger).
///
/// **Why a retry loop, not a single commit-and-wait.** `EventBus::new()` deliberately drops the
/// tailer's own readiness signal (`tailer::spawn`'s doc comment in `event_bus/tailer.rs`) — "the
/// tailer's readiness receiver is dropped here... A row committed before the initial
/// `high_water_mark` resolves is CORRECTLY never published (property 1: start at the mark, not
/// 0)." `tokio::spawn` only SCHEDULES the tailer task; how long its first `high_water_mark()`
/// read takes to resolve is unbounded under real machine load (this session's concurrent
/// multi-agent load on a 4-core box measurably widens that window — see the decisions-ledger for
/// task 017), and a probe committed before it resolves is silently and permanently (and
/// correctly) dropped. Only a bounded retry — not a longer single wait — is sound against a
/// permanent, by-design drop.
///
/// **Only for waits that occur before the tailer has otherwise been demonstrated live.** Once a
/// live delivery has already succeeded once on a given stream, the tailer is proven running and a
/// single `expect_next_seq` at a generous deadline is sound (see `WARM_LIVE_DEADLINE` and test
/// 2's handoff, which stays on the exact-seq form for exactly this reason).
async fn prove_tailer_is_live(pool: &SqlitePool, stream: &mut EventStream) -> SequencedEvent {
    for _ in 0..PROBE_ATTEMPTS {
        let (probe_seq, probe_task_id) = commit_task_created(pool).await;
        let attempt_deadline = Instant::now() + PROBE_ATTEMPT_WINDOW;
        loop {
            let remaining = attempt_deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break; // this probe never arrived (tailer likely not ready yet) — retry fresh
            }
            match tokio::time::timeout(remaining, stream.next()).await {
                Ok(Some(Ok(ev))) if ev.seq == probe_seq => {
                    assert_task_created_body(&ev, probe_task_id);
                    return ev;
                }
                Ok(Some(Ok(_))) => continue, // an earlier straggling probe — keep waiting for THIS one
                _ => break,
            }
        }
    }
    panic!(
        "the tailer never went live after {PROBE_ATTEMPTS} probe attempts of \
         {PROBE_ATTEMPT_WINDOW:?} each"
    );
}

/// A committed row reaches a subscriber that is already live (subscribed before the commit).
///
/// The warm-up commit/receive pair is not part of the property under test — it exists solely to
/// provably exhaust `subscribe_from`'s one-time journal replay window (see the file header)
/// before the row under test is committed, so that row is GUARANTEED to travel
/// `commit -> tailer -> broadcast -> subscribe_from`'s live arm rather than
/// `subscribe_from`'s own direct read, deterministically. The row itself is then established via
/// [`prove_tailer_is_live`], which retries rather than committing exactly once — see that
/// function's doc comment for why a single commit right after `EventBus::new()` is not reliable.
#[tokio::test]
async fn a_committed_row_reaches_a_live_subscriber() {
    let (pool, _temp_dir) = db::test_utils::create_test_pool_with_migrations().await;
    let bus = EventBus::new(pool.clone(), 64);
    let mut stream: EventStream = bus.subscribe_from(0).unwrap();

    let (warm_seq, _warm_task_id) = commit_task_created(&pool).await;
    expect_next_seq(&mut stream, warm_seq, Instant::now() + DEADLINE).await;

    prove_tailer_is_live(&pool, &mut stream).await;

    bus.shutdown().await;
}

/// A subscriber that joins AFTER events already exist replays them from its cursor, then hands
/// off to live delivery with no gap and no duplicate across the boundary.
///
/// The handoff uses a single deterministic commit with the strict, exact-seq `expect_next_seq` —
/// not the retry-based [`prove_tailer_is_live`] — because by this point in the test the bus (and
/// its tailer) has already had three prior commits' worth of real elapsed awaits to establish
/// itself, which is a materially different situation from `EventBus::new()` immediately followed
/// by one commit (see [`prove_tailer_is_live`]'s doc comment for that narrower race). Given the
/// generous [`WARM_LIVE_DEADLINE`] this keeps the task's dictated "no gap" property exact rather
/// than weakened to "eventually, at or after."
#[tokio::test]
async fn a_subscriber_that_joins_late_replays_from_its_cursor_then_goes_live() {
    let (pool, _temp_dir) = db::test_utils::create_test_pool_with_migrations().await;
    let bus = EventBus::new(pool.clone(), 64);

    // Commit three events BEFORE subscribing at all.
    let mut committed = Vec::with_capacity(3);
    for _ in 0..3 {
        committed.push(commit_task_created(&pool).await);
    }

    let mut stream: EventStream = bus.subscribe_from(0).unwrap();

    // Replay must return exactly the three, strictly in seq order.
    let replay_deadline = Instant::now() + DEADLINE;
    for (expected_seq, expected_task_id) in &committed {
        let ev = expect_next_seq(&mut stream, *expected_seq, replay_deadline).await;
        assert_task_created_body(&ev, *expected_task_id);
    }

    // Handoff: this commit happens strictly AFTER the 3-item replay batch has been fully
    // drained. subscribe_from never re-reads the journal past its one Initializing read (see
    // the file header), so the ONLY remaining path to the subscriber for this row is
    // commit -> tailer -> broadcast -> live arm — this IS the handoff this test exists to prove.
    let (seq4, task_id4) = commit_task_created(&pool).await;
    let ev = expect_next_seq(&mut stream, seq4, Instant::now() + WARM_LIVE_DEADLINE).await;
    assert_task_created_body(&ev, task_id4);

    // No gap (already proven: seq4 arrived as the immediate next item, not something later) and
    // no duplicate across the boundary (proven here: nothing else — in particular no repeat of
    // 1..=4 — arrives afterward).
    assert_quiet(&mut stream, QUIET_WINDOW).await;

    bus.shutdown().await;
}

/// A rolled-back transaction reaches no subscriber. Journal-first means an uncommitted event
/// must be invisible END TO END (through the real `subscribe_from` API), not merely absent from
/// the raw journal table — the latter is already covered by task 004's `rollback_journals_nothing`.
#[tokio::test]
async fn a_rolled_back_transaction_reaches_no_subscriber() {
    let (pool, _temp_dir) = db::test_utils::create_test_pool_with_migrations().await;
    let bus = EventBus::new(pool.clone(), 64);

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
    let (seq, task_id) = commit_task_created(&pool).await;

    // Subscribe AFTER both operations, so the very first thing observed must be the committed
    // one — proving the rolled-back write left no trace anywhere subscribe_from can see it.
    let mut stream: EventStream = bus.subscribe_from(0).unwrap();
    let ev = expect_next_seq(&mut stream, seq, Instant::now() + DEADLINE).await;
    assert_task_created_body(&ev, task_id);

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
///   independent of whether the tailer has started yet — calling the tailer-dependent
///   [`prove_tailer_is_live`] here instead would violate ITS OWN documented precondition (it must
///   only be called once the replay window is already exhausted) and prove nothing, which is
///   exactly the mistake this test's own first draft made once already (see the decisions-ledger).
///   Its EXACT seq (`high_water + 1`, not merely "greater than") is the defence against vacuity:
///   if a regression made the replay read use the wrong lower bound, the first item received
///   would be seq 1 or 2, and the assertion fails loudly with that value. (This does NOT guard
///   against a tailer that starts publishing from 0 instead of the high-water mark — that is
///   unobservable through `subscribe_from` by construction, because its Live arm drops anything
///   with `ev.seq <= state.last`; that property belongs to, and is covered by, task 013's
///   `tailer_resumes_from_its_high_water_on_restart`.) Consuming it exhausts `subscribe_from`'s
///   replay window (see the file header), which is what makes the second commit below provable.
/// - The SECOND is established via [`prove_tailer_is_live`] (retried, not a single commit — see
///   that function's doc comment). With the replay window now exhausted, this is the half that
///   actually proves the RESTARTED bus's tailer resumes live delivery.
#[tokio::test]
async fn a_new_bus_on_the_same_pool_resumes_without_replaying_history() {
    let (pool, _temp_dir) = db::test_utils::create_test_pool_with_migrations().await;

    let bus1 = EventBus::new(pool.clone(), 64);
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

    let bus2 = EventBus::new(pool.clone(), 64);
    let mut stream: EventStream = bus2.subscribe_from(high_water).unwrap();

    let (first_post_seq, first_post_task_id) = commit_task_created(&pool).await;
    assert_eq!(
        first_post_seq,
        high_water + 1,
        "the first post-restart commit should continue the seq sequence, not reset it"
    );
    let first = expect_next_seq(&mut stream, first_post_seq, Instant::now() + DEADLINE).await;
    assert_task_created_body(&first, first_post_task_id);

    let second = prove_tailer_is_live(&pool, &mut stream).await;
    assert!(
        second.seq > first.seq,
        "the second post-restart event (seq {}) must be strictly after the first (seq {})",
        second.seq,
        first.seq
    );

    // No stray replay of the two pre-restart events, or a duplicate of either post-restart one,
    // sneaks in afterward.
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
    let bus = EventBus::new(pool.clone(), 64);

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
