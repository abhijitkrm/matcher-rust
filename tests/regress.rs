//! Regression: dense order map + churn. Before the `OrderMap::remove` fix,
//! backward-shift deletion could overwrite a live key; this stream then
//! rejected a cancel of live order 4 as `unknown_order_id` (event line 328).
//! Expected events agree with matcher-java and matcher-ts.

use matcher::jsonflat::{parse_command, parse_header};
use matcher::*;

#[test]
fn dense_map_churn() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/");
    let cmds = std::fs::read_to_string(format!("{dir}dense_map_churn.cmd.jsonl")).unwrap();
    let want = std::fs::read_to_string(format!("{dir}dense_map_churn.evt.jsonl")).unwrap();
    let mut lines = cmds.lines();
    let (pmin, pmax, max_orders, _) = parse_header(lines.next().unwrap());
    let cmds: Vec<Command> = lines.map(|l| parse_command(l).unwrap()).collect();
    for index in [IndexKind::Ladder, IndexKind::Tree] {
        let cfg = BookConfig {
            price_min: pmin,
            price_max: pmax,
            max_orders,
            index,
        };
        let mut book = OrderBook::new(cfg);
        let mut sink = LinesSink::new();
        for &c in &cmds {
            book.apply(c, &mut sink);
            #[cfg(debug_assertions)]
            book.check_invariants();
        }
        let want: Vec<&str> = want.lines().skip(1).collect();
        assert_eq!(sink.lines, want, "{index:?}");
    }
}
