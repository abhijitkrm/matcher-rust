//! Snapshot write/restore (spec/JOURNAL.md §3, `matcher-snap/1`).
//!
//! A snapshot captures *resting* state in book order — restoring by direct
//! insertion reproduces exact FIFO position and `seq`, so the continuation
//! emits byte-identical events. Verify: `snap → restore → snap` must equal.

use std::fmt::Write as _;

use crate::book::OrderBook;
use crate::engine::Engine;
use crate::jsonflat::{get_i64, get_str, get_u64};
use crate::types::*;

/// Serialize one book's resting state as a `{"rec":"book"}` block + orders.
pub fn write_book(book: &OrderBook, sym: Symbol, out: &mut String) {
    let _ = writeln!(
        out,
        "{{\"rec\":\"book\",\"symbol\":{sym},\"seq\":{}}}",
        book.seq()
    );
    for o in book.resting_orders() {
        let _ = writeln!(
            out,
            "{{\"rec\":\"order\",\"order_id\":{},\"side\":\"{}\",\"otype\":\"limit\",\"tif\":\"{}\",\"price\":{},\"qty\":{}}}",
            o.order_id,
            o.side.as_str(),
            o.tif.as_str(),
            o.price,
            o.qty
        );
    }
}

/// Full engine snapshot: header (default config) + every book, sorted by
/// symbol for deterministic output.
pub fn write_engine(engine: &Engine, out: &mut String) {
    let cfg = engine.default_cfg();
    let _ = writeln!(
        out,
        "{{\"format\":\"matcher-snap/1\",\"pmin\":{},\"pmax\":{},\"max_orders\":{},\"index\":\"{}\"}}",
        cfg.price_min,
        cfg.price_max,
        cfg.max_orders,
        match cfg.index {
            IndexKind::Ladder => "ladder",
            IndexKind::Tree => "tree",
        }
    );
    let mut books: Vec<_> = engine.books_iter().collect();
    books.sort_by_key(|(s, _)| *s);
    for (sym, book) in books {
        write_book(book, sym, out);
    }
}

/// One parsed book block: symbol, last seq, resting orders in file order.
pub struct SnapshotBook {
    pub symbol: Symbol,
    pub seq: u64,
    pub orders: Vec<RestingOrder>,
}

/// Parsed snapshot: engine default config + book blocks in file order.
pub struct ParsedSnapshot {
    pub cfg: BookConfig,
    pub books: Vec<SnapshotBook>,
}

/// Parse a `matcher-snap/1` document. Panics on malformed input — snapshots
/// are trusted local artifacts, not adversarial input.
pub fn parse(text: &str) -> ParsedSnapshot {
    let mut lines = text.lines();
    let hdr = lines.next().expect("snapshot header");
    assert_eq!(get_str(hdr, "format"), Some("matcher-snap/1"), "bad format");
    let cfg = BookConfig {
        price_min: get_i64(hdr, "pmin").unwrap_or(0),
        price_max: get_i64(hdr, "pmax").unwrap_or(1_000_000),
        max_orders: get_u64(hdr, "max_orders").unwrap_or(65_536) as usize,
        index: match get_str(hdr, "index") {
            Some("tree") => IndexKind::Tree,
            _ => IndexKind::Ladder,
        },
    };
    let mut books: Vec<SnapshotBook> = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        match get_str(line, "rec").expect("rec field") {
            "book" => books.push(SnapshotBook {
                symbol: get_u64(line, "symbol").unwrap_or(0) as Symbol,
                seq: get_u64(line, "seq").unwrap_or(0),
                orders: Vec::new(),
            }),
            "order" => {
                let o = RestingOrder {
                    order_id: get_u64(line, "order_id").unwrap(),
                    side: match get_str(line, "side") {
                        Some("ask") => Side::Ask,
                        _ => Side::Bid,
                    },
                    tif: match get_str(line, "tif") {
                        Some("ioc") => Tif::Ioc,
                        Some("fok") => Tif::Fok,
                        Some("post_only") => Tif::PostOnly,
                        _ => Tif::Gtc,
                    },
                    price: get_i64(line, "price").unwrap(),
                    qty: get_u64(line, "qty").unwrap(),
                };
                books
                    .last_mut()
                    .expect("order line before book block")
                    .orders
                    .push(o);
            }
            r => panic!("bad rec {r}"),
        }
    }
    ParsedSnapshot { cfg, books }
}

/// Rebuild an engine from a parsed snapshot.
pub fn restore_engine(parsed: &ParsedSnapshot) -> Engine {
    let mut eng = Engine::new(parsed.cfg);
    for b in &parsed.books {
        eng.add_symbol(b.symbol, parsed.cfg);
        *eng.book_mut(b.symbol).unwrap() = OrderBook::restore(parsed.cfg, b.seq, &b.orders);
    }
    eng
}
