//! matcherfuzz <corpus.cmd.jsonl> — replay a fuzzgen corpus and print the
//! canonical event stream (symbol-tagged for engine corpora) to stdout.
//! scripts/diffuzz.sh byte-diffs this output across implementations.
//! Debug builds run `check_invariants` after every command.

use std::io::{BufWriter, Write as _};

use matcher::jsonflat::{get_str, parse_command, parse_header};
use matcher::*;

fn main() {
    let path = std::env::args().nth(1).expect("usage: matcherfuzz <cmd.jsonl>");
    let text = std::fs::read_to_string(&path).unwrap();
    let mut lines = text.lines();
    let header_line = lines.next().unwrap();
    let (pmin, pmax, max_orders, index) = parse_header(header_line);
    let engine = get_str(header_line, "engine") == Some("true");

    let cfg = BookConfig {
        price_min: pmin,
        price_max: pmax,
        max_orders,
        index,
    };

    let stdout = std::io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let mut scratch = String::with_capacity(128);

    if engine {
        let mut eng = Engine::new(cfg);
        for line in lines {
            if line.is_empty() {
                continue;
            }
            let sym = matcher::jsonflat::get_u64(line, "symbol").unwrap_or(0) as Symbol;
            let cmd = parse_command(line).unwrap_or_else(|| panic!("bad command line: {line}"));
            eng.submit_tagged(sym, cmd, &mut |s, seq, ev| {
                Event::write_canonical_sym(seq, s, ev, &mut scratch);
                scratch.push('\n');
                out.write_all(scratch.as_bytes()).unwrap();
                scratch.clear();
            });
            #[cfg(debug_assertions)]
            for (_, b) in eng.books_iter() {
                b.check_invariants();
            }
        }
    } else {
        let mut book = OrderBook::new(cfg);
        let mut sink = WriteSink {
            out: &mut out,
            scratch: &mut scratch,
        };
        for line in lines {
            if line.is_empty() {
                continue;
            }
            let cmd = parse_command(line).unwrap_or_else(|| panic!("bad command line: {line}"));
            book.apply(cmd, &mut sink);
            #[cfg(debug_assertions)]
            book.check_invariants();
        }
    }
    out.flush().unwrap();
}

struct WriteSink<'a> {
    out: &'a mut BufWriter<std::io::StdoutLock<'static>>,
    scratch: &'a mut String,
}

impl Sink for WriteSink<'_> {
    fn on_event(&mut self, seq: u64, ev: &Event) {
        Event::write_canonical(seq, ev, self.scratch);
        self.scratch.push('\n');
        self.out.write_all(self.scratch.as_bytes()).unwrap();
        self.scratch.clear();
    }
}
