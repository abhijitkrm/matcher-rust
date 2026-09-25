//! matcherrecover — e2e recovery: load a matcher-snap/1 snapshot, replay a
//! command journal tail, emit the canonical tagged event journal to stdout.
//! Pair with matcherrun:
//!   matcherrun cmds > full.evt
//!   head -n N cmds > prefix.cmd; matcherrun prefix.cmd --snap s.snap > prefix.evt
//!   tail -n +N+1 cmds > tail.cmd;  matcherrecover s.snap tail.cmd > recov.evt
//!   cat prefix.evt recov.evt | diff - full.evt        # must be identical
//!
//! Exits nonzero on a malformed journal line (truncated tail write).

use matcher::jsonflat::{get_u64, parse_command};
use matcher::*;
use std::io::Write as _;

fn main() {
    let mut args = std::env::args().skip(1);
    let snap_path = args
        .next()
        .expect("usage: matcherrecover <snap.jsonl> <cmd-tail.jsonl>");
    let tail_path = args
        .next()
        .expect("usage: matcherrecover <snap.jsonl> <cmd-tail.jsonl>");

    let snap_text = std::fs::read_to_string(&snap_path).unwrap();
    let mut eng = snapshot::restore_engine(&snapshot::parse(&snap_text));

    let text = std::fs::read_to_string(&tail_path).unwrap();
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    let mut scratch = String::new();
    for (lineno, line) in text.lines().enumerate() {
        if line.is_empty() || line.contains("\"format\"") {
            continue;
        }
        let sym = get_u64(line, "symbol").unwrap_or(0) as Symbol;
        let cmd = parse_command(line).unwrap_or_else(|| {
            eprintln!("{tail_path}:{}: malformed journal line: {line}", lineno + 1);
            std::process::exit(2);
        });
        eng.submit_tagged(sym, cmd, &mut |s, seq, ev| {
            journal::journal_event(s, seq, ev, &mut out, &mut scratch);
        });
    }
    out.flush().unwrap();
}
