mod common;

use db::models::task::TaskStatus;

#[tokio::test]
#[serial_test::serial]
async fn with_stats_returns_sorted_projects_with_derived_counts() {
    let h = common::HiveHarness::hive_absent().await; // no hive needed — this is local data

    // Insert TWO projects whose names arrive out of alphabetical order, so ordering is proven
    // rather than assumed, plus tasks spanning every status and at least one attempt.
    h.seed_project(
        "zeta",
        &[
            TaskStatus::Todo,
            TaskStatus::Todo,
            TaskStatus::Todo,
            TaskStatus::InProgress,
            TaskStatus::Done,
            TaskStatus::Done,
        ],
    )
    .await;
    h.seed_project("alpha", &[TaskStatus::Todo]).await;

    let res = h.get("/api/projects/with-stats").await;
    res.assert_registered();
    assert_eq!(res.status, 200, "body: {}", res.body);

    let v: serde_json::Value = serde_json::from_str(&res.body).unwrap();
    let projects = v["data"]["projects"].as_array().unwrap();

    // name-sorted (alpha before zeta), NOT insertion order
    assert_eq!(projects[0]["name"], "alpha");
    assert_eq!(projects[1]["name"], "zeta");

    // counts derived from the seeded tasks
    assert_eq!(projects[1]["task_counts"]["todo"], 3);
    assert_eq!(projects[1]["task_counts"]["done"], 2);

    // the attempt timestamp survives the mapping
    assert!(!projects[1]["last_attempt_at"].is_null());

    // the three dead fields are GONE (ADR-0014)
    assert!(projects[0].get("nodes").is_none());
    assert!(projects[0].get("has_local").is_none());
    assert!(projects[0].get("local_project_id").is_none());
}
