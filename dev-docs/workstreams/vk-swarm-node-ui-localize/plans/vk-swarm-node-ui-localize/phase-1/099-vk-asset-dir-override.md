---
id: "099"
phase: 1
title: "Make asset_dir() honour VK_ASSET_DIR so the asset root is configurable like its leaves"
status: passed
depends_on: []
parallel: false
conflicts_with: []
files:
  - crates/utils/src/assets.rs
  - scripts/setup-dev-environment.js
  - .env.example
  - docs/configuration-customisation/storage-configuration.mdx
siblings:
  - crates/utils/src/assets.rs
irreversible: false
scope_test: "N/A"
allowed_change: mixed
covers_criteria: [SC1, SC4]
---

## Why this task exists

`asset_dir()` is the root that `config_path()`, `credentials_path()`, `profiles_path()`, and the
default `database_path()` / `backup_dir()` all derive from — and it is the **only** one of them
with no environment override. `VK_DATABASE_PATH`, `VK_BACKUP_DIR`, `VK_WORKTREE_DIR`, and
`VK_LOG_DIR` already exist. This task completes that established pattern; it does not invent one.

Two concrete consequences today:

1. **Tests cannot reach the credential store.** Every hive proxy routes through
   `get_authed` → `require_oauth_token` → `auth_context.get_credentials()`
   (`crates/services/src/services/remote_client.rs:242-246`), which reads
   `credentials_path()` = `asset_dir()/credentials.json` (`crates/utils/src/assets.rs:35-36`).
   With no override, a test cannot supply credentials, so any proxy test observes `401`
   (`RemoteClientError::Auth` → `crates/server/src/error.rs:168`) instead of the frozen spec's
   required `200` + `success: true`. This blocks tasks 100, 101-104, and 301.
2. **`Deployment::new()` rewrites the developer's `config.json` on every run** —
   `save_config_to_file(&raw_config, &config_path).await?`
   (`crates/local-deployment/src/lib.rs:133`, commented "Always save config"). Running the test
   suite dirties working state.

In release builds `asset_dir()` is a single `ProjectDirs` path, so two production instances share
one config and credential store. This override also makes that separable.

**Scope discipline:** this task adds ONE env var to ONE function plus its documentation. It does
not refactor `asset_dir()`'s callers, does not touch `config_path`/`credentials_path`/
`profiles_path` (they inherit the fix for free), and adds no new capability.

## Failing test (write first)

Add to the EXISTING `#[cfg(test)] mod tests` block in `crates/utils/src/assets.rs`
(anchor: `mod tests {` at line 92). Match the surrounding style exactly — the neighbouring
`test_database_path_*` tests already use `#[serial]`, `unsafe { env::… }`, and the
`// SAFETY: Tests run serially via #[serial] attribute` comment. Reproduce all three.

```rust
    #[test]
    #[serial]
    fn test_asset_dir_env_override() {
        let tmp = tempfile::tempdir().expect("Failed to create temp dir");
        let custom = tmp.path().join("assets");
        // SAFETY: Tests run serially via #[serial] attribute
        unsafe { env::set_var("VK_ASSET_DIR", &custom) };
        let dir = asset_dir();
        unsafe { env::remove_var("VK_ASSET_DIR") };
        assert_eq!(dir, custom);
        assert!(custom.exists(), "asset_dir() must create the directory");
    }

    #[test]
    #[serial]
    fn test_asset_dir_env_override_reaches_derived_paths() {
        let tmp = tempfile::tempdir().expect("Failed to create temp dir");
        let custom = tmp.path().join("assets");
        // SAFETY: Tests run serially via #[serial] attribute
        unsafe { env::set_var("VK_ASSET_DIR", &custom) };
        let creds = credentials_path();
        let config = config_path();
        unsafe { env::remove_var("VK_ASSET_DIR") };
        // This is the whole point: the derived leaves must follow the root.
        assert_eq!(creds, custom.join("credentials.json"));
        assert_eq!(config, custom.join("config.json"));
    }

    #[test]
    #[serial]
    fn test_asset_dir_tilde_expansion() {
        // SAFETY: Tests run serially via #[serial] attribute
        unsafe { env::set_var("VK_ASSET_DIR", "~/vibe-kanban-assets-test") };
        let dir = asset_dir();
        unsafe { env::remove_var("VK_ASSET_DIR") };
        assert!(!dir.to_string_lossy().contains('~'));
        assert!(dir.is_absolute());
    }

    #[test]
    #[serial]
    fn test_asset_dir_default_unchanged_when_unset() {
        // SAFETY: Tests run serially via #[serial] attribute
        unsafe { env::remove_var("VK_ASSET_DIR") };
        let dir = asset_dir();
        // Debug builds resolve to the repo's dev_assets/; release to the platform data dir.
        // Either way it must be absolute and must exist after the call.
        assert!(dir.is_absolute());
        assert!(dir.exists());
    }
```

`test_asset_dir_tilde_expansion` creates `~/vibe-kanban-assets-test` as a side effect (the
function creates the directory it returns), mirroring the existing
`test_database_path_tilde_expansion`, which likewise expands into `$HOME`. That is accepted
existing behaviour — do NOT add cleanup logic the sibling tests do not have.

## Change

### `crates/utils/src/assets.rs` — `asset_dir()`

- **Anchor:** `pub fn asset_dir()` at line 6
- **Before:**
```rust
pub fn asset_dir() -> std::path::PathBuf {
    let path = if cfg!(debug_assertions) {
        std::path::PathBuf::from(PROJECT_ROOT).join("../../dev_assets")
    } else {
        ProjectDirs::from("ai", "bloop", "vibe-kanban")
            .expect("OS didn't give us a home directory")
            .data_dir()
            .to_path_buf()
    };
```
- **After:**
```rust
/// Get the root directory for all Vibe Kanban assets (config, credentials, profiles,
/// and the default database/backup locations).
///
/// Respects the `VK_ASSET_DIR` environment variable for custom locations.
/// Supports tilde expansion (e.g., `~/vibe-kanban`).
///
/// Default: `<project_root>/dev_assets` in debug builds, the platform data directory
/// in release builds.
///
/// The directory is created automatically if it does not exist.
pub fn asset_dir() -> std::path::PathBuf {
    let path = if let Ok(dir) = std::env::var("VK_ASSET_DIR") {
        crate::path::expand_tilde(&dir)
    } else if cfg!(debug_assertions) {
        std::path::PathBuf::from(PROJECT_ROOT).join("../../dev_assets")
    } else {
        ProjectDirs::from("ai", "bloop", "vibe-kanban")
            .expect("OS didn't give us a home directory")
            .data_dir()
            .to_path_buf()
    };
```

The rest of the function (the `if !path.exists() { create_dir_all }` block, the trailing comment
block, and the `path` return) is UNCHANGED — the existing creation logic already covers the new
branch, which is why no separate `create_dir_all` is added.

`crate::path::expand_tilde` is the same helper `database_path()` uses
(`crates/utils/src/path.rs:124`); it is already in scope via `crate::`, so **no new `use` is
needed**. Do not add one.

### `.env.example`

- **Anchor:** line 43-46, the `VK_DATABASE_PATH` block under "Override default storage locations"
- **Before:**
```text
# Database file path (default: platform-specific data directory)
# - Development: <project_root>/dev_assets/db.sqlite
# - Production: ~/.local/share/vibe-kanban/db.sqlite (Linux)
# VK_DATABASE_PATH=/custom/path/to/db.sqlite
```

- **After:** insert these six lines IMMEDIATELY BEFORE the `# Database file path` line, then ONE
  blank line separating them from it. Leave the existing block untouched:

```text
# Root directory for all Vibe Kanban assets: config.json, credentials.json,
# profiles.json, and the default database/backup locations.
# - Development: <project_root>/dev_assets
# - Production: ~/.local/share/vibe-kanban (Linux)
# Set this to run two instances with fully separate state.
# VK_ASSET_DIR=/custom/path/to/vibe-kanban
```

### `docs/configuration-customisation/storage-configuration.mdx`

- **Anchor:** `### Database Location` at line 32, under `## Configuration Options` (line 30)
- **Change:** insert a new subsection IMMEDIATELY BEFORE `### Database Location`, matching the
  existing `<ParamField>` + fenced-example shape used by its siblings. Apply Amendment R1.3 to
  the wording below before writing it.

  **The content to insert is indented by 4 spaces in THIS task file. Strip that 4-space indent
  when writing it into the `.mdx`.** The ` ```bash ` line and its closing ` ``` ` are LITERAL
  CONTENT of the `.mdx` file — they are not fences belonging to this task file. Do not nest or
  drop them.

      ### Asset Root Directory

      <ParamField path="VK_ASSET_DIR" type="string">
        Root directory for all Vibe Kanban state: `config.json`, `credentials.json`,
        `profiles.json`, and the default locations for the database and backups. The
        directory is created automatically if it doesn't exist. The more specific
        overrides below take precedence over it.
      </ParamField>

      ```bash
      # Example: keep all state under one directory
      VK_ASSET_DIR=~/vibe-kanban

      # Example: run a second, fully isolated instance
      VK_ASSET_DIR=/data/vibe-kanban-staging
      ```

The precedence sentence is REQUIRED and is accurate: `database_path()`
(`crates/utils/src/assets.rs:48-59`) and `backup_dir()` (`:72-81`) each read their own env var
FIRST and only fall back to `asset_dir()`, so `VK_DATABASE_PATH` does override `VK_ASSET_DIR`.

## Amendment R1 (orchestrator, 2026-07-30) — three defects found by the Stage-2 panel

Attempt 1 (commit `6c31aa1e`) implemented this task's text exactly, with an empty ledger, and
passed the Stage-1 gate. The adversarial panel then found three real defects **in this task's
text**. Attempt 2 must additionally do the following.

### R1.1 (BLOCKER) — add `VK_ASSET_DIR` to `productionOnlyVars`

`scripts/setup-dev-environment.js` maintains a deny-list whose stated purpose is: *"When executors
spawn worktree dev servers, they inherit production env vars. Dev servers should use their local
`dev_assets/` paths"* (`:307-310`). It already lists all four storage leaves — `VK_DATABASE_PATH`,
`VK_LOG_DIR`, `VK_BACKUP_DIR`, `VK_WORKTREE_DIR` (`:313-316`). Any `VK_*` var NOT on the list is
**actively re-exported** into the spawned dev server (`:342-347`).

`VK_ASSET_DIR` is the ROOT those four leaves default off. Left off the list, a production
`VK_ASSET_DIR` leaks into every worktree dev server — and because the script UNSETS
`VK_DATABASE_PATH` while forwarding `VK_ASSET_DIR`, `database_path()` falls back to
`asset_dir().join("db.sqlite")` (`crates/utils/src/assets.rs:72`) and the dev server opens the
**production database**. This is the exact hazard the deny-list exists to prevent.

- **Anchor:** `scripts/setup-dev-environment.js:312-316`
- **Before:**
```javascript
        const productionOnlyVars = [
          // Storage paths - dev uses local dev_assets/
          'VK_DATABASE_PATH',
```
- **After:**
```javascript
        const productionOnlyVars = [
          // Storage paths - dev uses local dev_assets/
          // VK_ASSET_DIR is the ROOT the others default off — it must be unset first,
          // or a production asset dir becomes the dev server's default database.
          'VK_ASSET_DIR',
          'VK_DATABASE_PATH',
```

### R1.2 (MAJOR) — treat an empty/whitespace `VK_ASSET_DIR` as unset

`VK_ASSET_DIR=` (exported but empty) currently produces `PathBuf::from("")`. `create_dir_all("")`
returns `Ok(())`, so the `if !path.exists()` guard never fires and never panics — and every
derived path becomes CWD-relative (`credentials.json`, `db.sqlite`, `backups/`). CWD differs
between `cargo watch` (repo root), the packaged binary, and `cargo test` (crate dir), so the same
env var silently yields different data locations. A stray `export VK_ASSET_DIR=` in a shell would
scatter config and credentials into whatever directory the server was launched from.

- **Change:** in `asset_dir()`, replace the override condition
  `if let Ok(dir) = std::env::var("VK_ASSET_DIR")` with a form that rejects blank values:
```rust
    let override_dir = std::env::var("VK_ASSET_DIR")
        .ok()
        .filter(|s| !s.trim().is_empty());

    let path = if let Some(dir) = override_dir {
        crate::path::expand_tilde(&dir)
    } else if cfg!(debug_assertions) {
```
  The remaining arms and the creation block are unchanged.

Add this test alongside the others:

```rust
    #[test]
    #[serial]
    fn test_asset_dir_empty_env_falls_back_to_default() {
        // SAFETY: Tests run serially via #[serial] attribute
        unsafe { env::remove_var("VK_ASSET_DIR") };
        let default_dir = asset_dir();
        unsafe { env::set_var("VK_ASSET_DIR", "   ") };
        let dir = asset_dir();
        unsafe { env::remove_var("VK_ASSET_DIR") };
        // A blank override must be ignored, NOT resolved relative to the CWD.
        assert_eq!(dir, default_dir);
        assert!(dir.is_absolute());
    }
```

### R1.3 (MAJOR) — the docs overpromise "fully separate state"

Two release instances with different `VK_ASSET_DIR` do NOT have fully separate state: the worktree
base dir is resolved from `VK_WORKTREE_DIR` or a shared temp dir, NOT from `asset_dir()`
(`crates/services/src/services/worktree_manager.rs:571-581`). Separate asset dirs mean separate
databases, so each instance's startup orphan sweep
(`crates/local-deployment/src/container.rs:319-383`, spawned at `:162`) sees the OTHER instance's
live worktrees as orphans and `remove_dir_all`s them — with no dirty-file guard on that path.

- In `.env.example`, replace the line
  `# Set this to run two instances with fully separate state.`
  with:
```text
# Set this to give a second instance its own config/credentials/database.
# NOTE: also set VK_WORKTREE_DIR per instance — worktrees are NOT under this
# directory, and two instances sharing one worktree dir will delete each
# other's worktrees on startup.
```
- In `storage-configuration.mdx`, replace the example comment
  `# Example: run a second, fully isolated instance`
  with `# Example: give a second instance its own config, credentials and database`
  and append this line inside the `<ParamField>` description, before `</ParamField>`:
```text
  Worktrees are not stored here — set `VK_WORKTREE_DIR` as well to isolate a second instance.
```

### R1.4 (MINOR) — make the tilde test actually bite

`test_asset_dir_tilde_expansion` passes even with the production change reverted (the default
`dev_assets` path is absolute and contains no `~`), so it asserts nothing about `VK_ASSET_DIR`.
Add this assertion at the end of that test:

```rust
        assert!(
            dir.ends_with("vibe-kanban-assets-test"),
            "tilde expansion must resolve the VK_ASSET_DIR value, got {dir:?}"
        );
```

## Amendment R2 (orchestrator, 2026-07-30) — the R1.2 guard is inconsistent with itself

The focused re-check found that R1.2 validates the TRIMMED string but then passes the UNTRIMMED
one to `expand_tilde`. So `VK_ASSET_DIR=" /tmp/foo "` survives the guard, and the leading space
makes the path RELATIVE — `asset_dir()`'s unconditional `create_dir_all` then creates a directory
literally named `" "` under the process CWD with the real path nested inside it, and config,
credentials and the database land there silently. That is the exact failure R1.2 was written to
prevent, reachable one space away.

### R2.1 — trim the value, then test it

- **Anchor:** `crates/utils/src/assets.rs:17-19`, the `override_dir` binding
- **Before:**
```rust
    let override_dir = std::env::var("VK_ASSET_DIR")
        .ok()
        .filter(|s| !s.trim().is_empty());
```
- **After:**
```rust
    // Trim BEFORE use, not just before the emptiness test: a value like " /tmp/foo "
    // is otherwise a RELATIVE path, and create_dir_all below would make a directory
    // literally named " " under the process CWD.
    let override_dir = std::env::var("VK_ASSET_DIR")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
```

Add this test alongside the others:

```rust
    #[test]
    #[serial]
    fn test_asset_dir_env_override_is_trimmed() {
        let tmp = tempfile::tempdir().expect("Failed to create temp dir");
        let custom = tmp.path().join("assets");
        let padded = format!("  {}  ", custom.display());
        // SAFETY: Tests run serially via #[serial] attribute
        unsafe { env::set_var("VK_ASSET_DIR", &padded) };
        let dir = asset_dir();
        unsafe { env::remove_var("VK_ASSET_DIR") };
        // Surrounding whitespace must not make the path relative.
        assert_eq!(dir, custom);
        assert!(dir.is_absolute(), "padded override must stay absolute, got {dir:?}");
    }
```

### R2.2 — the deny-list comment states a constraint that does not exist

`scripts/setup-dev-environment.js` emits every `unset` before any `export`, so `VK_ASSET_DIR`'s
position in the array is immaterial. The comment claiming it "must be unset first" is false and
will mislead the next maintainer.

- **Before:**
```javascript
          // VK_ASSET_DIR is the ROOT the others default off — it must be unset first,
          // or a production asset dir becomes the dev server's default database.
```
- **After:**
```javascript
          // VK_ASSET_DIR is the ROOT the others default off. Without it here, a
          // production asset dir would be re-exported below and become the dev
          // server's default database (database_path() falls back to it).
```

## Allowed moves

- Only the four files in `files:`.
- In `assets.rs`: the one `if let Ok(dir) = std::env::var("VK_ASSET_DIR")` branch, the doc comment
  above `asset_dir()`, and the four new tests appended inside the EXISTING `mod tests` block.

## STOP triggers

- If `crate::path::expand_tilde` does not exist at `crates/utils/src/path.rs:124` — STOP.
- If adding the branch changes the behaviour of any test that currently passes — STOP and report
  which. The default path when `VK_ASSET_DIR` is unset MUST be byte-identical to today's.
- If `mod tests` in `assets.rs` does not already import `serial_test::serial`, `std::env`, and use
  `tempfile` — STOP rather than adding dev-dependencies (they are already present in
  `crates/utils/Cargo.toml` `[dev-dependencies]`: `serial_test = "3.0"`, `tempfile = "3.10"`).
- Do NOT modify `config_path()`, `credentials_path()`, `profiles_path()`, `database_path()`,
  `backup_dir()`, or any caller of `asset_dir()`. They inherit the change for free. Touching them
  is out of scope.
- Do NOT add `VK_ASSET_DIR` handling to any other function.

## Manual verification (emit verbatim; the ORCHESTRATOR records it)

```bash
cargo test -p utils --lib assets
# Expected: the six new asset_dir tests pass (four original + the R1.2 blank-override test
#           + the R2.1 trimmed-override test), and every pre-existing test still passes

grep -n "VK_ASSET_DIR" scripts/setup-dev-environment.js
# Expected: a hit inside the productionOnlyVars array (Amendment R1.1)

node -e "process.env.VK_ASSET_DIR='/prod/assets'; require('child_process').execSync('node scripts/setup-dev-environment.js env',{stdio:'inherit'})" | grep -c "export VK_ASSET_DIR"
# Expected: 0 — a production VK_ASSET_DIR must NOT be re-exported into a dev server

cargo clippy -p utils --all-targets --all-features -- -D warnings
# Expected: clean

git diff --stat crates/
# Expected: only crates/utils/src/assets.rs

grep -n 'VK_ASSET_DIR' .env.example docs/configuration-customisation/storage-configuration.mdx
# Expected: hits in both files
```

## Done when

- `VK_ASSET_DIR` overrides the asset root, with tilde expansion, and the directory is created.
- `credentials_path()` and `config_path()` follow the override (proved by
  `test_asset_dir_env_override_reaches_derived_paths`).
- With `VK_ASSET_DIR` unset, behaviour is byte-identical to today.
- The env var is documented in `.env.example` and `storage-configuration.mdx` alongside its
  siblings.
- `cargo test -p utils --lib assets` and `cargo clippy -p utils --all-targets --all-features
  -- -D warnings` are both clean.
