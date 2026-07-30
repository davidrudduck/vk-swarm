---
id: "401"
phase: 4
title: "Give RemoteClientNotConfigured a discriminable ApiError variant and status"
status: ready
depends_on: []
parallel: false
conflicts_with: []
files:
  - crates/server/src/error.rs
irreversible: false
scope_test: "N/A"
allowed_change: edit
covers_criteria: [SC4]
---

## Failing test (write first)

N/A — Rust error mapping with no unit-test seam in this crate. Proven over HTTP in task 402's
verification and in the phase-5 deploy evidence.

## Why this task exists

Today every hive-absent failure collapses into one line:

```rust
impl From<RemoteClientNotConfigured> for ApiError {
    fn from(_: RemoteClientNotConfigured) -> Self {
        ApiError::BadRequest("Remote client not configured".to_string())
    }
}
```

That renders as a generic `400 BadRequest`, indistinguishable from a malformed request. The
frontend's `ApiError` class (`frontend/src/lib/api/utils.ts`) carries `status`, so a distinct
status code is the cleanest thing for the UI to branch on (SC4).

## Change

### 1. Add the variant — `crates/server/src/error.rs`

- **Anchor:** the `pub enum ApiError` body, immediately after the `RemoteClient` variant (~line 66)
- **Before:**
```rust
    #[error(transparent)]
    RemoteClient(#[from] RemoteClientError),
```
- **After:**
```rust
    #[error(transparent)]
    RemoteClient(#[from] RemoteClientError),
    #[error("This node is not connected to a hive")]
    HiveNotConfigured,
```

### 2. Repoint the `From` impl

- **Anchor:** line 103-107
- **Before:**
```rust
impl From<RemoteClientNotConfigured> for ApiError {
    fn from(_: RemoteClientNotConfigured) -> Self {
        ApiError::BadRequest("Remote client not configured".to_string())
    }
}
```
- **After:**
```rust
impl From<RemoteClientNotConfigured> for ApiError {
    fn from(_: RemoteClientNotConfigured) -> Self {
        ApiError::HiveNotConfigured
    }
}
```

### 3. Map it in `IntoResponse`

- **Anchor:** the `match &self` arm list, immediately after the `ApiError::RemoteClient(err) => …`
  arm's closing `},` and before `ApiError::Unauthorized => …` (~line 197)
- **After:** insert:
```rust
            ApiError::HiveNotConfigured => {
                (StatusCode::SERVICE_UNAVAILABLE, "HiveNotConfigured")
            }
```

`503` + the `"HiveNotConfigured"` error type give the frontend two independent discriminators.

## Allowed moves

- Only `crates/server/src/error.rs`: one variant, one `From` body, one match arm.

## STOP triggers

- **This is a deliberate status-code change (400 → 503) on every route that calls
  `deployment.remote_client()?`,** which includes the already-live `/api/organizations*` routes.
  Decomposition found no frontend code branching on `status === 400`
  (`grep -rn 'status === 400' frontend/src` → no matches). Re-run that grep before you start; if
  it now returns a hit, STOP and report rather than proceeding.
- If the `match` is non-exhaustive after adding the variant, you have put the arm in the wrong
  block — STOP rather than adding a catch-all `_ =>`.

## Manual verification (record in decisions-ledger)

```bash
cargo clippy -p server --all-targets --all-features -- -D warnings
# Expected: clean (a missing match arm would fail here)

grep -rn 'status === 400\|statusCode === 400' frontend/src remote-frontend/src
# Expected: NO output (nothing depends on the old 400)

# With the dev server running and NO hive configured:
curl -s -o /dev/null -w '%{http_code}\n' \
  "http://127.0.0.1:${PORT}/api/nodes?organization_id=00000000-0000-0000-0000-000000000000"
# Expected: 503

curl -s "http://127.0.0.1:${PORT}/api/nodes?organization_id=00000000-0000-0000-0000-000000000000"
# Expected: body contains "HiveNotConfigured"
```

## Done when

- `ApiError::HiveNotConfigured` exists and is what `RemoteClientNotConfigured` converts into.
- A hive-absent request returns `503` with error type `HiveNotConfigured`.
- Clippy is clean.
