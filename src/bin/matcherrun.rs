//! matcherrun — e2e runner: apply an engine command stream, emit the canonical
//! tagged event journal to stdout, optionally write a snapshot at the end.
//! Part of the spec/JOURNAL.md recovery loop (with matcherrecover).
//!
//!   matcherrun <engine.cmd.jsonl> [--snap <path>]

use matcher::jsonflat::{get_u64, parse_command, parse_header};
use matcher::*;
use std::io::Write as _;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: matcherrun <cmd.jsonl> [--snap <path>]");
    let mut snap_path: Option<String> = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--snap" => snap_path = Some(args.next().expect("--snap needs a path")),
            _ => panic!("unknown arg {a}"),
        }
    }

    let text = std::fs::read_to_string(&path).unwrap();
    let mut lines = text.lines();
    let (pmin, pmax, max_orders, index) = parse_header(lines.next().unwrap_or("{}"));
    let cfg = BookConfig {
        price_min: pmin,
        price_max: pmax,
        max_orders,
        index,
    };
    let mut eng = Engine::new(cfg);

    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    let mut scratch = String::new();
    for (lineno, line) in lines.enumerate() {
        if line.is_empty() {
            continue;
        }
        let sym = get_u64(line, "symbol").unwrap_or(0) as Symbol;
        let cmd = parse_command(line)
            .unwrap_or_else(|| panic!("{}:{}: malformed command: {line}", path, lineno + 2));
        eng.submit_tagged(sym, cmd, &mut |s, seq, ev| {
            journal::journal_event(s, seq, ev, &mut out, &mut scratch);
        });
    }
    out.flush().unwrap();

    if let Some(p) = snap_path {
        let mut snap = String::new();
        snapshot::write_engine(&eng, &mut snap);
        std::fs::write(p, snap).unwrap();
    }
}
