//! matcher_bench — spec/BENCH.md measurement protocol (v2).
//!
//!   matcher_bench <prefix> [--tag name] [--mode latency|throughput|scale]
//!                 [--threads N] [--reps R]
//!
//!   latency    (default) per-op timed `apply()`; row as BENCH.md §3.
//!   throughput untimed `apply_batch()` over the whole run; one warmup rep,
//!              then R reps (default 5): best / median ops/s, median ns/op.
//!   scale      N shard threads, each owning a private book replaying the
//!              corpus as its own symbol partition (SCALING.md); aggregate
//!              ops/s = N*ops / slowest shard's wall time.
//!
//! Loads `<prefix>.setup.cmd.jsonl` (untimed) + `<prefix>.run.cmd.jsonl`
//! (measured). Parsing happens before timing.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

use matcher::jsonflat::{parse_command, parse_header};
use matcher::*;

fn load(path: &str) -> (BookConfig, Vec<Command>) {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let mut lines = text.lines();
    let (pmin, pmax, max_orders, index) = parse_header(lines.next().unwrap_or(""));
    let cmds = lines
        .filter(|l| !l.trim().is_empty())
        .map(|l| parse_command(l).unwrap_or_else(|| panic!("bad command line: {l}")))
        .collect();
    (
        BookConfig {
            price_min: pmin,
            price_max: pmax,
            max_orders,
            index,
        },
        cmds,
    )
}

fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let i = ((sorted.len() as f64 - 1.0) * p).ceil() as usize;
    sorted[i.min(sorted.len() - 1)]
}

/// Ask the scheduler for a P-core (macOS has no affinity pinning; BENCH.md
/// §2.1). The only `unsafe` in this binary: one libc call with constant
/// arguments, no pointers. The library crate itself stays `forbid(unsafe)`.
#[cfg(target_os = "macos")]
fn high_qos() {
    extern "C" {
        fn pthread_set_qos_class_self_np(qos: u32, rel: i32) -> i32;
    }
    const QOS_CLASS_USER_INTERACTIVE: u32 = 0x21;
    // SAFETY: plain FFI call on the current thread; no memory is shared.
    unsafe {
        pthread_set_qos_class_self_np(QOS_CLASS_USER_INTERACTIVE, 0);
    }
}

#[cfg(not(target_os = "macos"))]
fn high_qos() {}

fn fresh_book(cfg: BookConfig) -> OrderBook {
    let mut book = OrderBook::new(cfg);
    book.prefault();
    book
}

/// One untimed batched replay on a fresh book; returns run wall seconds.
fn replay(cfg: BookConfig, setup: &[Command], run: &[Command], acc: &mut u64) -> f64 {
    let mut book = fresh_book(cfg);
    let mut sink = NullSink::new();
    book.apply_batch(setup, &mut sink);
    let t0 = Instant::now();
    book.apply_batch(run, &mut sink);
    let s = t0.elapsed().as_secs_f64();
    *acc = acc.wrapping_add(std::hint::black_box(sink.acc));
    s
}

fn latency_mode(tag: &str, cfg: BookConfig, setup: &[Command], run: &[Command]) {
    // Warmup: throwaway book, setup + first 10% of run.
    {
        let mut book = OrderBook::new(cfg);
        let mut sink = NullSink::new();
        for c in setup {
            book.apply(*c, &mut sink);
        }
        for c in run.iter().take(run.len() / 10) {
            book.apply(*c, &mut sink);
        }
        std::hint::black_box(sink.acc);
    }

    // Timed: fresh book, untimed setup, per-op timing on run.
    let mut book = fresh_book(cfg);
    let mut sink = NullSink::new();
    for c in setup {
        book.apply(*c, &mut sink);
    }
    let mut lat = Vec::with_capacity(run.len());
    let wall = Instant::now();
    for c in run {
        let t0 = Instant::now();
        book.apply(*c, &mut sink);
        lat.push(t0.elapsed().as_nanos() as u64);
    }
    let wall_ns = wall.elapsed().as_nanos() as u64;
    std::hint::black_box(sink.acc);

    lat.sort_unstable();
    let ops = run.len();
    let ops_s = ops as f64 / (wall_ns as f64 / 1e9);
    let mean = lat.iter().map(|v| *v as u128).sum::<u128>() / ops.max(1) as u128;

    println!(
        "| {tag} | {ops} | {ops_s:.0} | {mean} | {} | {} | {} | {} | {} |",
        percentile(&lat, 0.50),
        percentile(&lat, 0.90),
        percentile(&lat, 0.99),
        percentile(&lat, 0.999),
        lat.last().copied().unwrap_or(0),
    );
}

fn throughput_mode(tag: &str, cfg: BookConfig, setup: &[Command], run: &[Command], reps: usize) {
    high_qos();
    let mut acc = 0u64;
    replay(cfg, setup, run, &mut acc); // warmup
    let mut t: Vec<f64> = (0..reps).map(|_| replay(cfg, setup, run, &mut acc)).collect();
    std::hint::black_box(acc);
    t.sort_by(f64::total_cmp);
    let ops = run.len() as f64;
    let med = t[t.len() / 2];
    println!(
        "| {tag} | throughput | 1 | {} | {:.0} | {:.0} | {:.2} |",
        run.len(),
        ops / t[0],
        ops / med,
        1e9 * med / ops
    );
}

fn scale_mode(
    tag: &str,
    cfg: BookConfig,
    setup: &[Command],
    run: &[Command],
    threads: usize,
    reps: usize,
) {
    let mut agg = Vec::with_capacity(reps);
    for r in 0..=reps {
        // rep 0 is warmup
        let ready = AtomicUsize::new(0);
        let go = AtomicBool::new(false);
        let walls: Vec<f64> = std::thread::scope(|s| {
            let hs: Vec<_> = (0..threads)
                .map(|_| {
                    let (ready, go) = (&ready, &go);
                    s.spawn(move || {
                        high_qos();
                        // Book built on its own thread: first-touch places its pages.
                        let mut book = fresh_book(cfg);
                        let mut sink = NullSink::new();
                        book.apply_batch(setup, &mut sink);
                        ready.fetch_add(1, Ordering::SeqCst);
                        while !go.load(Ordering::Acquire) {
                            std::hint::spin_loop();
                        }
                        let t0 = Instant::now();
                        book.apply_batch(run, &mut sink);
                        let w = t0.elapsed().as_secs_f64();
                        std::hint::black_box(sink.acc);
                        w
                    })
                })
                .collect();
            while ready.load(Ordering::SeqCst) < threads {
                std::hint::spin_loop();
            }
            go.store(true, Ordering::Release);
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        if r > 0 {
            let slowest = walls.iter().copied().fold(0.0, f64::max);
            agg.push(threads as f64 * run.len() as f64 / slowest);
        }
    }
    agg.sort_by(f64::total_cmp);
    let med = agg[agg.len() / 2];
    println!(
        "| {tag} | scale | {threads} | {} | {:.0} | {:.0} | {:.0} |",
        run.len(),
        agg[agg.len() - 1],
        med,
        med / threads as f64
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "usage: matcher_bench <corpus-prefix> [--tag name] \
             [--mode latency|throughput|scale] [--threads N] [--reps R]"
        );
        std::process::exit(2);
    }
    let prefix = &args[1];
    let mut tag = prefix.rsplit('/').next().unwrap().to_string();
    let mut mode = "latency".to_string();
    let mut threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let mut reps = 5usize;
    let mut i = 2;
    while i + 1 < args.len() {
        let v = &args[i + 1];
        match args[i].as_str() {
            "--tag" => tag = v.clone(),
            "--mode" => mode = v.clone(),
            "--threads" => threads = v.parse::<usize>().unwrap_or(threads).max(1),
            "--reps" => reps = v.parse::<usize>().unwrap_or(reps).max(1),
            _ => {
                i += 1;
                continue;
            }
        }
        i += 2;
    }

    let (cfg, setup) = load(&format!("{prefix}.setup.cmd.jsonl"));
    let (_, run) = load(&format!("{prefix}.run.cmd.jsonl"));

    match mode.as_str() {
        "throughput" => throughput_mode(&tag, cfg, &setup, &run, reps),
        "scale" => scale_mode(&tag, cfg, &setup, &run, threads, reps),
        _ => latency_mode(&tag, cfg, &setup, &run),
    }
    eprintln!(
        "env: {} / {} / rust ({mode})",
        std::env::consts::ARCH,
        std::env::consts::OS
    );
}
