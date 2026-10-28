# Changelog

## Unreleased

- New hot-path layout (`../matcher/docs/LAYOUT.md`): resting orders live
  inline in 128-byte per-level chunks instead of a pooled record with
  prev/next links; `OrderMap` gets a 1-byte tag array.
- `OrderBook::apply_batch()` and `prefault()`; optional `prefetch` feature
  (the crate's only `unsafe`, one audited hint helper) for the look-ahead
  pipeline once a book outgrows cache.
- `matcher_bench --mode latency|throughput|scale` (BENCH.md v2).
- Throughput, Apple M1: 94M ops/s on one core (1k live orders), 25M at
  100k live, 13M at 1M live; 110M ops/s aggregate on the mixed workload
  across 8 shards. See `../matcher/docs/RESULTS.md`.

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
