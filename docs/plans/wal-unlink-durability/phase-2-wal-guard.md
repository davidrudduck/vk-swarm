# Phase 2: wal-guard

WalGuard module in crates/db: dedicated connection holding the WAL wal-index in the ledger-recorded mode, with reconnect/read-mark API and the VK_WAL_GUARD kill-switch.
