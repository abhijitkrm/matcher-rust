//! snapdump — apply an engine command stream, print the snapshot to stdout.
//! Cross-language snapshot parity check: every implementation must emit
//! byte-identical output for the same input (spec/JOURNAL.md).
//!   matchersnap <engine.cmd.jsonl>

use matcher::jsonflat::{get_u64, parse_command};
use matcher::*;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: matchersnap <cmd.jsonl>");
    let text = std::fs::read_to_string(&path).unwrap();
    let mut lines = text.lines();
    let header = lines.next().unwrap();
    let cfg = BookConfig {
        price_min: matcher::jsonflat::get_i64(header, "pmin").unwrap_or(0),
        price_max: matcher::jsonflat::get_i64(header, "pmax").unwrap_or(1_000_000),
        max_orders: get_u64(header, "max_orders").unwrap_or(65_536) as usize,
        index: if matcher::jsonflat::get_str(header, "index") == Some("tree") {
            IndexKind::Tree
        } else {
            IndexKind::Ladder
        },
    };
    let mut eng = Engine::new(cfg);
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let sym = get_u64(line, "symbol").unwrap_or(0) as Symbol;
        let cmd = parse_command(line).unwrap();
        eng.submit_tagged(sym, cmd, &mut |_, _, _| {});
    }
    let mut snap = String::new();
    snapshot::write_engine(&eng, &mut snap);
    print!("{snap}");
}
