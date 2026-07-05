# Tournament Round 1 — Adversarial Review Report

This report presents an adversarial review of the changes on branch `fix/preexisting-gate-failures` (PR #452, target: `davidrudduck/vk-swarm`). 
The target diff was evaluated through mechanical correctness and compliance with workspace standards (specifically `AGENTS.md` and `CLAUDE.md`).

---

## Findings

### Finding F001 (Severity: High)
* **Issue:** Non-standard and broken markdown syntax used in 37 doctests across the workspace.
* **Citation:** `crates/remote/src/db/swarm_projects.rs:259` (and 36 other locations)
* **Details:** 
  The author changed 37 doctest code blocks (31 in the `remote` crate and 6 in the `services` crate) to use the syntax `` ```,ignore `` to bypass rustdoc failures. 
  While `rustdoc`'s custom parser successfully splits on commas and ignores the test, standard Markdown parsers (such as those used by GitHub, crates.io, GitLab, and IDEs like VS Code) strictly treat the entire info string (`,ignore`) as the language name. 
  Because `,ignore` is not a valid language name, these code blocks render with absolutely no syntax highlighting (displayed as unformatted plain text) on GitHub/crates.io.
  Furthermore, blanket-suppressing all doctests in this manner defeats the purpose of the doctest quality gate, leading to documentation rot. This also violates the newly added rule in `AGENTS.md` which mandates using standard per-item attributes (`rust,ignore` or `ignore` or `no_run`).
* **Remediation:** 
  Replace the non-standard `` ```,ignore `` with the standard `` ```rust,ignore `` or `` ```ignore ``:
  ```rust
  /// # Examples
  ///
  /// ```rust,ignore
  /// # async fn doc_example(pool: &sqlx::PgPool) -> Result<(), Box<dyn std::error::Error>> {
  /// use uuid::Uuid;
  /// ...
  /// ```
  ```

---

### Finding F002 (Severity: Low)
* **Issue:** Duplicate production database schema for `node_outbox` in tests.
* **Citation:** `crates/services/tests/electric_task_sync.rs:102`
* **Details:**
  To remediate a missing schema failure (F6), the author hardcoded and duplicated the SQL schema for the `node_outbox` table and its corresponding index in `electric_task_sync.rs`. 
  This introduces schema redundancy and increases the risk of schema drift. If future database migrations modify the `node_outbox` column names or types, the test suite's duplicate setup will fall out of sync, leading to false-positive test failures or masking true regressions.
* **Remediation:** 
  Extract schema setup to a shared helper or programmatically apply migrations during testing instead of copy-pasting raw SQL table structures inside test functions:
  ```rust
  // Suggested fix:
  // Programmatically apply migrations to the test database
  sqlx::migrate!("../db/migrations")
      .run(&pool)
      .await
      .expect("Failed to run migrations");
  ```

---

## Conclusion
The target diff represents a major step forward in stabilizing the workspace's gate checks, particularly around serializing parallel integration tests to prevent DB locks. However, the blanket suppression of doctests via non-standard syntax represents a quality debt carry-forward that degrades the documentation quality and should be corrected using standard markdown and rustdoc practices.
