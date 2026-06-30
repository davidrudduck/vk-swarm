---
topic: vk-swarm-hive-redesign
doc_type: decisions-ledger
---

# Decisions ledger — vk-swarm-hive-redesign

Appended during spec/precheck and (later) decompose/execute. Pre-empted traps for implementers are
added at decompose time.

## Precheck notes

### Anchor-check false positive (resolved — `--no-anchor-check` used)
`wai-precheck.sh` assert #3 (path anchors) flagged `src/db/tasks.rs` and `src/activity/broker.rs` as
"ABSENT on main". This is a **false positive**: the extractor regex
`(src|extensions|ui|packages|apps)/…` truncates any `crates/*/src/*` path to its `src/…` substring and
tests it at repo root. The real files **exist on main** — verified directly:
`git cat-file -e main:crates/remote/src/db/tasks.rs` and `…/crates/remote/src/activity/broker.rs` both
succeed. These are the only two path tokens the regex extracts from this spec; both were manually
grounded. The sibling `vk-swarm-node-foundations` spec (shipped) produces the identical truncation
pattern. Precheck was therefore re-run with `--no-anchor-check`; the spec is frozen against
`spec_sha` in `.precheck.passed`.
