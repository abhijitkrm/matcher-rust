# Changelog

## v0.2.0 — 2026-10-10

- Journals and snapshots (`spec/JOURNAL.md`): command/event journal,
  `matcher-snap/1` snapshots, restore; `matcherrun`, `matcherrecover`,
  `matchersnap`; `matcherfuzz` differential-fuzz harness.
- `engine: true` golden vectors (multi-symbol `Engine`).
- Vector `edge/042_dense_map_churn` (43 golden vectors).
- Fix: `OrderMap::remove` dropped live keys during backward-shift deletion
  (found by orderer; pinned by `042_dense_map_churn`).
- `Engine` hashes symbols with one multiply instead of SipHash: about 2x
  on multi-symbol workloads.
- The price ladder finds the next best price through a summary bitmap,
  and an emptied side resets at once. A side whose last level emptied
  used to scan the whole ladder: W3 at 1M ops (the book drains) ran at
  0.6M and now runs at 13M orders/s. Output unchanged; a randomized
  test checks the cursor against a sorted reference.

## v0.1.0 — 2026-09-24

Initial release — reference implementation.

- `OrderBook` + `Engine`, `Sink` event seam
- Limit/Market, New/Cancel/Replace, GTC/IOC/FOK/Post-Only
- FIFO price-time priority, maker-price execution
- Pooled orders, intrusive FIFO levels, bitmap ladder index with
  top-of-book cursor, `BTreeMap` fallback for unbounded prices
- `#![forbid(unsafe_code)]`, zero dependencies
- 41 golden vectors passing — byte-identical to matcher-go and matcher-cpp
