# Focused Remediation Recheck — `fa0612ee..6a304873`

- **Scope:** read-only adversarial recheck of the two SHOULD-FIX findings (F-R2-01, F-R2-02) from the prior native Opus remediation review.
- **Worktree:** `/home/david/.cache/dr-panel-tmp/dr-panel-gentle-mongoose-6a304873-1372255`, HEAD `6a304873`, `git status --porcelain` empty before and after — nothing was edited, restored, reset, stashed, or cleaned.
- **Range contents:**
  - `3e128082` `test(auth): strengthen review evidence` — 6 files (comment rewrites in the two db test modules, the backend-inspection block, README/ledger/task-file prose).
  - `9d705438` `test(auth): guard keychain assertion ordering` — 1 line, a comment above the existing match.
  - `6a304873` `docs(wai): record strengthened task gate` — 13 lines appended to the decisions-ledger.

---

## F-R2-01 — SQLite evidence-layer accuracy: **CLOSED**

### What the prior review rejected

- The pre-remediation comments asserted the controls ran against the *identical* harness, and therefore "proved" the 200-iteration real-function stress tests were not silently toothless (`queries.rs` old text: "against the IDENTICAL harness"; `lifecycle.rs` old text: "prove the harness here is capable of reproducing the hazard rather than being silently toothless"). That was false — the controls force the schedule deterministically; the stress tests do not use the controls' schedule at all.

### What the four required artefacts now say

- `crates/db/src/models/execution_process/lifecycle.rs:1027-1032` — the stress test is now labelled a *"Scheduler-sensitive stress check"*, and the doc states the generator "is supplemental evidence rather than a deterministic proof against every read-before-write mutation."
- `crates/db/src/models/execution_process/lifecycle.rs:1103-1107` — "This in-tree control proves the database failure mode independently; **it does not calibrate** the timing-driven production stress generator above or deterministically mutation-test the real function."
- `crates/db/src/models/execution_process/queries.rs:1304-1309` — identical honest framing for `mark_orphaned_as_failed`'s stress test.
- `crates/db/src/models/execution_process/queries.rs:1363-1366` — retitled *"Deterministic hazard control"*; "This proves SQLite returns `SQLITE_BUSY_SNAPSHOT` for the hazardous shape; **it does not calibrate** the timing-driven stress generator above." The word "IDENTICAL" is gone from this docstring.
- `dev-docs/workstreams/sqlite-busy-snapshot-calibration-stability/README.md:40-45` — "the background-writer generator is not a deterministic mutation test and is not calibrated by the controls. This distinction replaces the previous inaccurate 'identical harness' claim."
- `docs/plans/local-node-browser-oauth/decisions-ledger.md:278-284` — records the rejection and the correction in the same terms.

### Independent verification that the claims are true

- **The "UPDATE is the transaction's first statement" premise is factually correct in both functions** — this is the load-bearing claim the new comments substitute for the retracted one:
  - `crates/db/src/models/execution_process/lifecycle.rs:91-117` — `pool.begin()` at `:91`, then the `UPDATE ... RETURNING` at `:104` is the first statement issued; the owner `SELECT` is at `:126-130`, strictly after.
  - `crates/db/src/models/execution_process/queries.rs:149-162` — `pool.begin()` at `:149`, `UPDATE ... RETURNING` at `:152` first; owner `SELECT` at `:168-172`, after.
- **Grep for surviving false claims** — `grep -rniE "identical harness|IDENTICAL|same harness|calibrat"` over `crates/db`, `docs/plans/local-node-browser-oauth`, and the workstream directory returns no residual "identical harness"/toothlessness claim. The remaining hits are unrelated (`lifecycle.rs:32/604/633/821` = "three identical Completed writes"; `queries.rs:125/167/1172/1182/1209/1281` = "identical hazard"/"identical helper" cross-references), or are the honest negations quoted above.
- **Focused runs** (`TMPDIR=/home/david/.cache/dr-panel-tmp DISABLE_WORKTREE_ORPHAN_CLEANUP=1`):

```text
cargo test -p db --lib -- --nocapture --test-threads=1 read_then_upgrade reproduces_busy_snapshot

test ...lifecycle::...::control_prior_status_read_reproduces_busy_snapshot ... ok
test ...lifecycle::...::update_completion_does_not_read_then_upgrade ...
  no_read_then_upgrade(update_completion, real write-first shape): 0/200 SQLITE_BUSY_SNAPSHOT, 0 other errors
ok
test ...queries::...::control_read_then_write_shape_reproduces_busy_snapshot ... ok
test ...queries::...::mark_orphaned_as_failed_does_not_read_then_upgrade ...
  no_read_then_upgrade(mark_orphaned_as_failed, real write-first shape): 0/200 SQLITE_BUSY_SNAPSHOT, 0 other errors
ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 305 filtered out; finished in 7.85s
```

- **Flake recheck of the two previously-flaky controls, three consecutive runs** — this is the one place a self-check could have diverged from the README's "ten consecutive focused runs" claim (`README.md:37-38`), so I ran it rather than trusting it:

```text
--- run 1 --- 2 passed; 0 failed  (1.13s)
--- run 2 --- 2 passed; 0 failed  (1.12s)
--- run 3 --- 2 passed; 0 failed  (1.09s)
```

### Judgement against the stated minimum acceptable remediation

- The minimum bar was: **stop claiming the controls calibrate the production stress tests, and state that the 200-iteration real-function tests are scheduler-sensitive supplemental evidence — without introducing a test-only production hook.** All four artefacts now say exactly that, in plain language, in the same place a reader encounters the test.
- **No test-only production hook was introduced.** `git diff fa0612ee..6a304873` touches only doc comments in the two db test modules (no `#[cfg(test)]` seam, no injectable hook, no production statement reordered). Production `update_completion` and `mark_orphaned_as_failed` bodies are byte-identical across the range.
- **No test was weakened, ignored, or deleted** to make the wording true — the same four tests still run and still assert `busy_snapshot_errors == 0` / extended code 517.
- F-R2-01 is closed.

---

## F-R2-02 — File-backend regression discrimination and safety: **CLOSED**

### The test now discriminates before it writes

`crates/services/src/services/oauth_credentials.rs:258-281`:

```rust
let path = temp_dir.path().join("credentials.json");
let credentials = OAuthCredentials::new_file_backed(path.clone());

// Inspect before saving so a backend regression cannot touch the production Keychain.
match &credentials.backend {                                              // :265
    Backend::File(backend) => assert_eq!(backend.path, path),             // :266
    #[cfg(target_os = "macos")]
    Backend::Keychain(_) => panic!("explicit file backend selected the macOS Keychain"), // :268
}

credentials.save(&Credentials { ... }).await.unwrap();                    // :271-278
assert!(path.exists());                                                    // :280
```

- **Inspects the private backend before saving** — the `match` at `:265-269` precedes the `save()` at `:271`. The test module is `mod tests` inside the defining module (`:254-256`, `use super::*`), so the private `OAuthCredentials::backend` field (`:43`) and the private `FileBackend::path` field (`:152`) are both legitimately reachable without any visibility widening. Confirmed by compilation — the test builds and runs.
- **Requires `Backend::File`** — `Backend` has exactly two variants (`:94-98`: `File`, and `Keychain` gated on `#[cfg(target_os = "macos")]`). The match is exhaustive on both platforms and admits no path in which a non-`File` backend reaches `save()`.
- **Verifies the exact supplied path** — `assert_eq!(backend.path, path)` at `:266` compares against the `tempfile` path handed to the constructor, so a constructor that silently substituted a different path (e.g. a default/production location) fails here.
- **Fails without touching Keychain on a regression** — on macOS, `Backend::Keychain(_)` at `:268` panics *before* `save()` is ever called, so no `set_generic_password` (`:239`) is issued against the fixed production slot `CARGO_PKG_NAME:oauth` / `default` (`:212-213`). This is exactly the hazard the original finding named.

### Production code confirmed unchanged from `ae5ee15f`

- `git diff ae5ee15f..6a304873 -- crates/services/src/services/oauth_credentials.rs` shows only two additive hunks: the `new_file_backed` constructor (`:55-60`) and the `#[cfg(test)] mod tests` block (`:254-282`).
- `OAuthCredentials::new()` (`:48-53`) and `Backend::detect()` (`:100-123`, including the `OAUTH_CREDENTIALS_BACKEND` env override and the `cfg!(debug_assertions)` default) appear in **no** hunk of that diff — they are byte-identical to `ae5ee15f`.

### Plan and ledger match the test

- `docs/plans/local-node-browser-oauth/phase-1/022-fence-browser-login-commit-against-explicit-disconnect.md:184-185` — "Its same-module regression test must inspect the private backend before saving, require `Backend::File`, and assert the exact configured path." Matches the implementation exactly, including the ordering requirement.
- Same file `:230-232` (manual-verification item 7) — "the constructor regression test verifies the backend variant and path before any persistent write." Matches.
- `docs/plans/local-node-browser-oauth/decisions-ledger.md:281-284` — "The credential regression test now inspects `OAuthCredentials`' private backend before saving, requires `Backend::File`, and verifies the exact configured path; on macOS a Keychain selection therefore fails without a Keychain write." Matches.
- STOP trigger at `:220` ("Any route edit or production credential-backend behavior change; the explicit file-backed test constructor must bypass detection without changing `OAuthCredentials::new`") is respected — verified by the `ae5ee15f` diff above.

### Independent run

```text
cargo test -p services --lib explicit_file_backend_is_path_scoped

running 1 test
test services::oauth_credentials::tests::explicit_file_backend_is_path_scoped ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 315 filtered out; finished in 0.01s
```

- F-R2-02 is closed.

---

## Repository gates re-run

| Gate | Command | Result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | clean (`FMT_CLEAN`; only the pre-existing nightly-only `imports_granularity`/`group_imports` warnings) |
| Focused tests (db) | `cargo test -p db --lib -- --nocapture --test-threads=1 read_then_upgrade reproduces_busy_snapshot` | 4 passed, 0 failed |
| Control flake recheck | same controls ×3 consecutive | 2 passed each run, 0 failed |
| Focused test (services) | `cargo test -p services --lib explicit_file_backend_is_path_scoped` | 1 passed, 0 failed |
| Clippy | `cargo clippy -p db -p services --all-targets --all-features -- -D warnings` | `Finished dev profile` — zero diagnostics |

All runs used `TMPDIR=/home/david/.cache/dr-panel-tmp` and `DISABLE_WORKTREE_ORPHAN_CLEANUP=1` as instructed.

---

## Non-blocking notes

These do not affect the verdict. None is a should-fix; I record them because this recheck is specifically about evidence-statement accuracy.

- **N-1 — stale section label contradicted by its own body.** `crates/db/src/models/execution_process/lifecycle.rs:1096` still opens "Calibration control for the test above", while `:1105-1107` of the same docstring says "it does not calibrate the timing-driven production stress generator above." The sibling in `queries.rs:1363` was retitled to "Deterministic hazard control"; `lifecycle.rs` was not. Read as one unit the docstring is honest — the operative statement is the explicit negation three lines down — so this is a wording nit, not a surviving false claim. Same for the assertion message "calibration control must reproduce `SQLITE_BUSY_SNAPSHOT`" at `lifecycle.rs:1146` and `queries.rs:1411`: those are failure strings, not evidence claims.
- **N-2 — the macOS arm is verified by inspection only.** `Backend::Keychain(_)` at `oauth_credentials.rs:267-268` is `#[cfg]`-ed out on this Linux host, so my green run exercised the single-arm form. It is not exercised anywhere in CI either: `.github/workflows/` contains no `cargo test` or `cargo clippy` step at all, and macOS appears only as a release build target (`pre-release.yml:185-190`, `x86_64-apple-darwin` / `aarch64-apple-darwin`). The guard is correct by construction and will fire on a macOS developer machine; my evidence for the macOS branch is source inspection plus exhaustiveness of the two-variant enum, not execution.
- **N-3 — the quoted gate block is not self-evidencing for the services test.** `decisions-ledger.md:285-297` presents a gate transcript whose test line reads `running tests for scope 'crates/db/src/models/browser_auth/handoff.rs'`. In `~/.agents/wai/scripts/task-gate.sh`, `run_scope_tests()` (`:168-186`) runs `WAI_TEST_CMD` when set but the surrounding `echo`/`note` at `:702` and `:715` always print the *scope*, never the override — so the block alone cannot show that `cargo test -p services explicit_file_backend_is_path_scoped` ran. The inference that an override was in effect is nevertheless sound (a `.rs` scope under the default runner would route to vitest/node and fail-or-be-unmatched, and no `docs/plans/local-node-browser-oauth/.wai-test-cmd` file channel exists), and I substituted a direct run of that test rather than relying on the transcript. The file-set line "only declared files changed (1 paths)" checks out: `9d705438` touched only `crates/services/src/services/oauth_credentials.rs`, which is in task 022's declared `files:` list.
- **N-4 — informational, out of range.** `decisions-ledger.md:255` cites `crates/db/src/models/execution_process/queries.rs:1437`; that file is now 1415 lines, so the anchor points past EOF. `git log -S` attributes that line to `7b5d6eff`, which predates `fa0612ee` — pre-existing relative to this range and describing the pre-fix state. Not touched by this remediation; noted only so it is not mistaken for new drift.
- **N-5 — cosmetic.** `9d705438`'s subject is "guard keychain assertion ordering", but the commit adds only a one-line explanatory comment (`oauth_credentials.rs:264`); the ordering guard itself landed in `3e128082`. No functional consequence.

---

## Conclusion

- **F-R2-01 closed.** The "identical harness"/"not silently toothless" claim is gone from both test modules, the workstream README, and the ledger, replaced by an accurate split: the controls force the SQLite hazard deterministically; the unchanged 200-iteration real-function tests are scheduler-sensitive supplemental stress evidence backed by the functions' verified UPDATE-first structure. This meets the stated minimum bar, and it was achieved with no test-only production hook, no weakened assertion, and no ignored test.
- **F-R2-02 closed.** `explicit_file_backend_is_path_scoped` inspects the private backend before any write, requires `Backend::File`, asserts the exact supplied path, and panics before `save()` on a macOS Keychain selection. `OAuthCredentials::new()` and `Backend::detect()` are byte-identical to `ae5ee15f`, and task 022's plan (`:184-185`, `:230-232`) and the ledger (`:281-284`) describe precisely what the test does.
- No new blocking or should-fix issue found in the range. Format, focused tests (including a three-run flake recheck of the two previously-flaky controls), and focused clippy are all green.

VERDICT: APPROVE