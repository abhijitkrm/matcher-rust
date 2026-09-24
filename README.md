# matcher

[![ci](https://github.com/abhijitkrm/matcher-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/abhijitkrm/matcher-rust/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/matcher.svg)](https://crates.io/crates/matcher)
[![docs.rs](https://docs.rs/matcher/badge.svg)](https://docs.rs/matcher)
[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

Deterministic, zero-allocation FIFO limit order book and matching engine core —
measured at up to **~19M orders/sec** (see `spec/BENCH.md`).

Single-writer book per symbol, commands in, monotonically sequenced events out —
the same shape as real exchange matchers (CME Globex, Nasdaq INET). All I/O
hangs off the `Sink` seam; there is no networking, persistence, or clock
dependence in the core. `#![forbid(unsafe_code)]`, zero dependencies.

```rust
use matcher::*;

let mut book = OrderBook::new(BookConfig::default());
let mut sink = VecSink::new();

book.apply(Command::new(1, Side::Ask, 100, 10, Tif::Gtc), &mut sink);
book.apply(Command::new(2, Side::Bid, 100, 4, Tif::Gtc), &mut sink);

// order 2 filled 4 @100 against order 1 and closed; order 1 keeps 6 resting.
assert_eq!(book.order(1).unwrap().qty, 6);
assert!(book.order(2).is_none());
```

```toml
[dependencies]
matcher = "0.1"
```

## Features

- Limit + Market orders, New / Cancel / Replace
- GTC, IOC, FOK, Post-Only
- FIFO price-time priority, maker-price execution, partial fills, sweeps
- Pooled orders, intrusive FIFO price levels, bitmap ladder index
  (O(1) best-price) with `BTreeMap` fallback for unbounded prices
- Thin multi-symbol `Engine` router
- Deterministic event streams — verified byte-identically against the shared
  golden vector corpus (`vectors/`)

## Layout

```
src/       library (book, engine, index, pool, ordermap, level, sink, types)
src/bin/   matcher_bench benchmark harness
tests/     golden vector runner
vectors/   shared golden corpus (spec repo: github.com/abhijitkrm/matcher)
spec/      semantics contract (SPEC.md, SCHEMA.md, BENCH.md)
tools/     vectorgen — deterministic workload generator
```

## Test & bench

```bash
cargo test                  # unit tests + 41 golden vectors + doctest
cargo test --release
REGEN=1 cargo test          # regenerate vectors/*.evt.jsonl (auditing only)

# benchmark (workloads W1–W5, see spec/BENCH.md)
mkdir -p bench
cargo run --release --manifest-path tools/vectorgen/Cargo.toml -- \
  --workload w2 --n 200000 --setup-n 100000 --out bench/w2
cargo run --release --bin matcher_bench bench/w2
```

## License

MIT OR Apache-2.0
