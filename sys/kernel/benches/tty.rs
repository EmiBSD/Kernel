/* <LICENSES> */
/*
 * Copyright (c) 2026 Emilio Navarrete Lineros <enavarre@outlook.com>
 *
 * Permission to use, copy, modify, and distribute this software for any
 * purpose with or without fee is hereby granted, provided that the above
 * copyright notice and this permission notice appear in all copies.
 *
 * THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
 * WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
 * MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
 * ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
 * WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
 * ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
 * OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
 */
/* </LICENSES> */

/* <CODE> */
//! Host microbenchmarks of N4.7b's hot paths (decision 43): the clists (`kernel::clist`)
//! and, from the tty's commit on, the line discipline's per-character input, its output
//! processing and the tty's lock, over the kernel's host doubles (`kernel::testutil`,
//! feature `doubles`). Never part of a kernel image; it measures and changes nothing.
//!
//! `cargo bench -p kernel --features doubles --bench tty` (release: the `bench` profile) or
//! with `--profile dev`. Arguments: `list`; the names of the benches to run (all by default).
//!
//! What an op is (the "before" of each `Bench:` line is the same loop over LZ's functions, a
//! scratch test of `bsd` at the parent commit):
//! - `prim.clist-putc-getc`: one `putc` then one `getc` on a 1,024-character quoted queue.
//! - `prim.clist-b_to_q-64`: `b_to_q` of 64 bytes then `q_to_b` of them back.
//! - `tty.input-char-raw`: `ttyinput` of one character with `ICANON` and `ECHO` off (64 in
//!   a row, the raw queue emptied after them, which is timed too).
//! - `tty.input-char-canon-echo`: `ttyinput` of one character of a 64-character line (63
//!   'a' and '\n') with `ICANON|ECHO`, the echo drained by the driver's start routine (an
//!   inherited driver's `t_oproc`), the canonical queue emptied after each line.
//! - `tty.write-512-onlcr`: `ttwrite` of 512 bytes (a '\n' every 64) with `OPOST|ONLCR`,
//!   the output drained by the driver's start routine.
//! - `prim.tty-lock`: a `Mutexed` at `IPL_TTY` taken and given back (a queue's lock, the
//!   tty's); LZ's "before" is a `spltty()`/`splx()` pair, what its queues paid instead.
//! - `prim.spltty`: `splraise(IPL_TTY)` and its restore, the level alone.

#![forbid(unsafe_code)]

use std::boxed::Box;
use std::hint::black_box;
use std::string::String;
use std::time::{Duration, Instant};
use std::vec::Vec;

use hal::sys::errno::Errno;
use kernel::clist::Clist;
use kernel::proc::{self, ProcGlue, ProcTables, Process, Session, Thread};
use kernel::termios::{ECHO, ICANON, TTYDEF_CFLAG, TTYDEF_IFLAG, TTYDEF_LFLAG, TTYDEF_OFLAG};
use kernel::testutil;
use kernel::tty::{TS_CARR_ON, TS_ISOPEN, Tty, TtyRef, ttsetwater, ttychars, ttyinput, ttymalloc};
use kernel::uio::UioMove;

// ---------------------------------------------------------------------------------------
// The harness (vm.rs's).

/// One bench: its name, the ops one call of `f` makes, and `f`, which returns the time of
/// the part it measures.
struct Bench<'a> {
    name: &'static str,
    ops: usize,
    f: Box<dyn FnMut() -> Duration + 'a>,
}

/// The calls of a primitive per timed call.
const PRIM: usize = 1000;

/// Times `PRIM` calls of `f`.
fn prim(mut f: impl FnMut()) -> impl FnMut() -> Duration {
    move || {
        let t = Instant::now();
        for _ in 0..PRIM {
            f();
        }
        t.elapsed()
    }
}

/// ns per op of `reps` repetitions of about `target` of wall time each (after a
/// calibration that is the warm-up): min, median, max, and the calls per repetition.
fn measure(b: &mut Bench<'_>, reps: usize, target: Duration) -> (f64, f64, f64, usize) {
    let mut calls = 1usize;
    loop {
        let t = Instant::now();
        for _ in 0..calls {
            black_box((b.f)());
        }
        let wall = t.elapsed();
        if wall >= target / 4 || calls >= 1 << 24 {
            let per = wall.as_secs_f64() / calls as f64;
            calls = ((target.as_secs_f64() / per.max(1e-9)) as usize).max(3);
            break;
        }
        calls *= 4;
    }
    let mut v = Vec::with_capacity(reps);
    for _ in 0..reps {
        let mut t = Duration::ZERO;
        for _ in 0..calls {
            t += (b.f)();
        }
        v.push(t.as_nanos() as f64 / (calls * b.ops) as f64);
    }
    v.sort_by(f64::total_cmp);
    (v[0], v[v.len() / 2], v[v.len() - 1], calls)
}

// ---------------------------------------------------------------------------------------
// The glue of this bench: no member of bsd's.

/// The bench's glue.
pub struct BenchGlue;

kernel::proc_glue_links!(BenchGlue);
kernel::test_file_glue!(BenchFiles, BenchGlue);

static TABLES: ProcTables<BenchGlue> = ProcTables::new();
static SCHED: kernel::sched::SchedTables<BenchGlue> = kernel::sched::SchedTables::new();

impl ProcGlue for BenchGlue {
    type ProcessG = ();
    type ThreadG = ();
    type PgrpG = ();
    type SessionG = ();
    type FileGlue = BenchFiles;
    kernel::proc_glue_link_types!();

    fn tables() -> &'static ProcTables<Self> {
        &TABLES
    }

    fn sched() -> &'static kernel::sched::SchedTables<Self> {
        &SCHED
    }

    fn session_hangup(_sess: &Session<Self>) {}
    fn session_placed(_sess: &Session<Self>) {}
    fn session_destroy(_sess: &Session<Self>) {}
    fn with_curproc<R>(f: impl FnOnce(Option<&Thread<Self>>) -> R) -> R {
        f(None)
    }
    fn exit1(_p: &Thread<Self>, _xexit: i32, xsig: i32, _flags: i32) -> ! {
        std::panic!("exit1 in a bench (signal {xsig})")
    }
    fn pledge_fail(_p: &Thread<Self>, error: Errno, _code: u64) -> Errno {
        error
    }
    fn process_ref(pr: &Process<Self>) -> proc::ProcessRef<Self> {
        match hal::pool::PoolArc::from_ref(pr) {
            Some(a) => proc::StaticOrPooled::Pooled(a),
            None => std::process::exit(1),
        }
    }
}

/// A tty set up as a driver's open would: default termios at 9600, open, carrier on.
fn bench_tty() -> TtyRef<BenchGlue> {
    let tp = ttymalloc::<BenchGlue>(9600);
    ttychars(&tp);
    tp.set_t_iflag(TTYDEF_IFLAG);
    tp.set_t_oflag(TTYDEF_OFLAG);
    tp.set_t_lflag(TTYDEF_LFLAG);
    tp.set_t_cflag(TTYDEF_CFLAG);
    tp.set_t_ispeed(9600);
    tp.set_t_ospeed(9600);
    ttsetwater(&tp);
    tp.t_state_set(TS_ISOPEN | TS_CARR_ON);
    tp
}

/// The driver's start routine of the benches: the output queue drained.
fn drain_out(tp: &Tty<BenchGlue>) {
    let n = tp.t_outq.c_cc.get();
    if n > 0 {
        tp.t_outq.ndflush(n);
    }
}

/// A `UIO_SYSSPACE`-like write source over a buffer.
struct Src<'a> {
    data: &'a [u8],
    pos: usize,
}

impl UioMove for Src<'_> {
    fn resid(&self) -> usize {
        self.data.len() - self.pos
    }
    fn uiomove(&mut self, buf: &mut [u8]) -> Result<(), Errno> {
        let n = buf.len().min(self.resid());
        buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
        self.pos += n;
        Ok(())
    }
    fn nonblock(&self) -> bool {
        false
    }
    fn is_write(&self) -> bool {
        true
    }
    fn set_resid(&mut self, resid: usize) {
        self.pos = self.data.len().saturating_sub(resid);
    }
}

/// A queue of `size` characters, set up by `clalloc`, for the run.
fn queue(size: i32, quot: bool) -> &'static Clist {
    let q: &'static Clist = Box::leak(Box::new(Clist::new()));
    q.clalloc(size, quot);
    q
}

fn main() {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| a != "--bench")
        .collect();
    testutil::install();

    let q = queue(1024, true);
    let q2 = queue(1024, true);
    let src = [b'x'; 64];
    let mut dst = [0u8; 64];

    let raw = bench_tty();
    raw.t_lflag_clr(ICANON | ECHO);
    let canon = bench_tty();
    canon.set_oproc(Some(drain_out));
    let wr = bench_tty();
    wr.set_oproc(Some(drain_out));
    let line = [b'a'; 64];
    let mut cline = [b'a'; 64];
    cline[63] = b'\n';
    let mut msg = [b'w'; 512];
    for i in 0..8 {
        msg[i * 64 + 63] = b'\n';
    }

    let mut benches: Vec<Bench<'_>> = Vec::new();
    benches.push(Bench {
        name: "tty.input-char-raw",
        ops: 64,
        f: Box::new(|| {
            let t = Instant::now();
            for &c in &line {
                ttyinput(i32::from(c), &raw);
            }
            raw.t_rawq.ndflush(raw.t_rawq.c_cc.get());
            t.elapsed()
        }),
    });
    benches.push(Bench {
        name: "tty.input-char-canon-echo",
        ops: 64,
        f: Box::new(|| {
            let t = Instant::now();
            for &c in &cline {
                ttyinput(i32::from(c), &canon);
            }
            canon.t_canq.ndflush(canon.t_canq.c_cc.get());
            t.elapsed()
        }),
    });
    benches.push(Bench {
        name: "tty.write-512-onlcr",
        ops: 100,
        f: Box::new(|| {
            // A hundred writes per timing, as LZ's "before" loop (one clock read pair per
            // write would add ~45 ns of the host's clock to each).
            let t = Instant::now();
            for _ in 0..100 {
                let mut src = Src { data: &msg, pos: 0 };
                let _ = black_box(kernel::tty::ttwrite(&wr, &mut src, 0));
                drain_out(&wr);
            }
            t.elapsed()
        }),
    });
    benches.push(Bench {
        name: "prim.clist-putc-getc",
        ops: PRIM,
        f: Box::new(prim(|| {
            black_box(q.putc(black_box(0x61)));
            black_box(q.getc());
        })),
    });
    benches.push(Bench {
        name: "prim.clist-b_to_q-64",
        ops: PRIM,
        f: Box::new(prim(|| {
            black_box(q2.b_to_q(black_box(&src)));
            black_box(q2.q_to_b(black_box(&mut dst)));
        })),
    });

    // The primitives the ops are made of: the queue's lock (a `Mutexed` at IPL_TTY), and
    // what LZ's queues paid instead (`spltty()`/`splx()`, a raise of the level alone).
    static TTY_MTX: hal::mutex::Mutexed<u64> = hal::mutex::Mutexed::new(hal::intr::IPL_TTY, 0);
    benches.push(Bench {
        name: "prim.tty-lock",
        ops: PRIM,
        f: Box::new(prim(|| {
            *TTY_MTX.lock() += 1;
        })),
    });
    benches.push(Bench {
        name: "prim.spltty",
        ops: PRIM,
        f: Box::new(prim(|| {
            let s = hal::intr::splraise(hal::intr::IPL_TTY);
            drop(black_box(s));
        })),
    });

    if args.first().map(String::as_str) == Some("list") {
        for b in &benches {
            println!("{}", b.name);
        }
        std::process::exit(0);
    }

    if args.first().map(String::as_str) == Some("--profile") {
        // Loops one bench for SECS seconds, for a sampling profiler attached to the process
        // (`sample <pid>`), as vm.rs.
        let name = args.get(1).cloned().unwrap_or_default();
        let secs: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10);
        let Some(b) = benches.iter_mut().find(|b| b.name == name.as_str()) else {
            eprintln!("tty bench: no bench {name}");
            std::process::exit(2);
        };
        eprintln!(
            "pid {} profiling {} for {secs} s",
            std::process::id(),
            b.name
        );
        let end = Instant::now() + Duration::from_secs(secs);
        let mut calls = 0u64;
        let mut t = Duration::ZERO;
        while Instant::now() < end {
            t += (b.f)();
            calls += 1;
        }
        println!(
            "{} {:.1} ns/op over {calls} calls",
            b.name,
            t.as_nanos() as f64 / (calls as f64 * b.ops as f64)
        );
        std::process::exit(0);
    }

    let reps = 7;
    let target = Duration::from_millis(300);
    println!("# bench min median max ns/op, spread (max-min)/median, calls/rep, reps {reps}");
    for b in benches.iter_mut() {
        if !args.is_empty() && !args.iter().any(|a| a == b.name) {
            continue;
        }
        let (min, med, max, calls) = measure(b, reps, target);
        println!(
            "{:<30} {:>12.1} {:>12.1} {:>12.1} {:>6.1}% {:>8}",
            b.name,
            min,
            med,
            max,
            (max - min) / med * 100.0,
            calls
        );
    }
}
/* </CODE> */
