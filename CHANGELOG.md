# Changelog

## v0.1.0 — 2025-01-XX

Initial release — reference implementation.

- `OrderBook` + `Engine`, `Sink` event seam
- Limit/Market, New/Cancel/Replace, GTC/IOC/FOK/Post-Only
- FIFO price-time priority, maker-price execution
- Pooled orders, intrusive FIFO levels, bitmap ladder index with
  top-of-book cursor, `BTreeMap` fallback for unbounded prices
- `#![forbid(unsafe_code)]`, zero dependencies
- 41 golden vectors passing — byte-identical to matcher-go and matcher-cpp
