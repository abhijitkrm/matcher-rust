# Contributing

The contract that keeps this crate honest is `spec/` + `vectors/` (vendored
from the [matcher spec repo](https://github.com/abhijitkrm/matcher)):

- **Semantics changes** start upstream in `spec/SPEC.md` plus a golden vector
  (`vectors/**.cmd.jsonl`) with its canonical `.evt.jsonl`. The implementation
  must emit that stream byte-identically — in every index mode the vector
  declares.
- **Verify**: `cargo test` replays all golden vectors. `REGEN=1 cargo test`
  regenerates `.evt.jsonl` — audit diffs before committing.
- **Style**: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`.
  No `unsafe` — the crate is `#![forbid(unsafe_code)]`.
- **Performance**: no hot-path allocations; pooled orders, intrusive levels,
  direct-indexed ladder. Benchmarks use `tools/vectorgen` workloads per
  `spec/BENCH.md` — report CPU/OS/flags, no unattributed numbers.
