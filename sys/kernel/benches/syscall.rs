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
//! Host microbenchmarks of the system call path (N4.8b, decision 43: the syscall entry and
//! `copyin`/`copyout` are hot paths), over the host's doubles (`hal`'s user-copy double since
//! N4.8b, `kernel::testutil`'s for the rest). Never part of a kernel image.
//!
//! `cargo bench -p kernel --features doubles --bench syscall` (release) or with `--profile
//! dev`. Arguments: the names of the benches to run (all by default).
//!
//! What an op is (the "before" of each `Bench:` line is the same loop at the parent commit,
//! whose copies went through the `UserCopy` bridge to the kernel's double):
//! - `prim.copyin-64`: 64 bytes copied in from user memory (`UserPtr::copyin_slice`).
//! - `prim.copyout-64`: 64 bytes copied out.
//! - `prim.copyinstr-16`: a 16-byte path (15 characters and the NUL) copied in.
//!
//! The copies themselves are the host's double (a lock over a byte array, the bytes checked
//! against its bounds), not `copy.S`: the numbers measure the path into the machine's copy,
//! which the arm64 kernel shares, not the copy's own cost there.
//!
//! N4.8b b4's prologue (`kernel::syscall`), over a thread of this bench's glue whose libc pin
//! table names the call and whose `MAP_STACK` cache covers its stack pointer (the steady
//! state: the check is the cache compare, the map is not locked):
//! - `prim.pin-check`: `pin_check` of `getpid` from its pinned site.
//! - `prim.mi-syscall-prologue`: `mi_syscall` of `getpid`, pledge off, `SY_NOLOCK`, an empty
//!   body: `refreshcreds`, the `MAP_STACK` check, `pin_check`.
//! - `prim.inentry-hit`: the `MAP_STACK` check alone (`uvm_map_inentry` on a cache hit), the
//!   part of the prologue LZ reported `unported!` and skipped (gap 1, the user's decision 5).

#![forbid(unsafe_code)]

use std::boxed::Box;
use std::hint::black_box;
use std::time::{Duration, Instant};
use std::vec::Vec;

use core::pin::Pin;

use hal::malloc::{M_WAITOK, MArray};
use hal::sys::errno::Errno;
use hal::sys::syscall::SYS_getpid;
use hal::syscall::Register;
use hal::user::UserPtr;
use kernel::proc::{self, PinTable, ProcGlue, ProcTables, Process, Session, Thread};
use kernel::testutil;
use kernel::vm::map::{InentryCache, Vmspace, VmspaceRef};

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
// The processes' glue of this bench: the facts the prologue asks.

/// The bench's glue.
pub struct BenchGlue;

kernel::proc_glue_links!(BenchGlue);
kernel::test_file_glue!(BenchFiles, BenchGlue);

static TABLES: ProcTables<BenchGlue> = ProcTables::new();
static SCHED: kernel::sched::SchedTables<BenchGlue> = kernel::sched::SchedTables::new();

/// The bench thread's address space.
static VM: std::sync::OnceLock<VmspaceRef> = std::sync::OnceLock::new();

/// The bench thread's `PROC_PC`: the call instruction `pin_check` finds pinned.
const PC: usize = 0x2000_0040;
/// Its stack pointer, inside the `MAP_STACK` range its cache holds.
const SP: usize = 0x7f00_0000;

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
    fn pledge_syscall(_p: &Thread<Self>, _code: i32) -> Result<(), (Errno, u64)> {
        Ok(())
    }
    fn pledge_sysctl(_p: &Thread<Self>, _mib: &[i32], _new: usize) -> Result<(), Errno> {
        Ok(())
    }
    fn process_ref(pr: &Process<Self>) -> proc::ProcessRef<Self> {
        match hal::pool::PoolArc::from_ref(pr) {
            Some(a) => proc::StaticOrPooled::Pooled(a),
            None => std::process::exit(1),
        }
    }
    fn proc_pc(_p: &Thread<Self>) -> usize {
        PC
    }
    fn proc_stack(_p: &Thread<Self>) -> usize {
        SP
    }
    fn user_vmspace(_p: &Thread<Self>) -> Option<&Vmspace> {
        VM.get().map(|v| &**v)
    }
}

/// The bench's process, static.
static PR: Process<BenchGlue> = Process::new(42, ());

/// Where the bench's user buffers are in the user-copy double (a range no test uses).
const UBUF: usize = testutil::USER_BASE + 0xD000;

fn main() {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| a != "--bench")
        .collect();
    testutil::install();
    // The prologue's thread: an address space (the map pools over the doubles' pages), a
    // libc pin table naming getpid at PC, a MAP_STACK cache over SP.
    let _pages = testutil::pages_install(256 * 4096);
    testutil::init_shared_once();
    hal::pmap::init();
    kernel::vm::map::uvm_map_init();
    let vm = kernel::vm::map::uvmspace_alloc(
        hal::arch::current::vmparam::VM_MIN_ADDRESS,
        hal::arch::current::vmparam::VM_MAXUSER_ADDRESS,
        true,
        false,
    );
    let serial = vm.vm_map.sserial();
    let _ = VM.set(vm);
    let t: &'static Thread<BenchGlue> = Box::leak(Box::new(Thread::new_static(&PR, ())));
    let p: Pin<&'static Thread<BenchGlue>> = hal::pin::pin_static(t);
    let mut pins = std::vec![0u32; hal::sys::syscall::SYS_MAXSYSCALL];
    pins[SYS_getpid as usize] = 0x40;
    let Some(pins) = MArray::new(kernel::vm::mmap::M_PINSYSCALL, M_WAITOK, pins.len(), |i| {
        pins[i]
    }) else {
        eprintln!("bench: no pin table");
        std::process::exit(1);
    };
    drop(PR.ps_libcpin.replace(PinTable {
        start: 0x2000_0000,
        end: 0x2000_1000,
        pins: Some(pins),
    }));
    let cache = InentryCache {
        start: 0x7e00_0000,
        end: 0x8000_0000,
        serial,
    };
    t.p_spinentry.set(cache);
    let code = SYS_getpid as Register;
    let sargs = [0 as Register; 6];
    let mut rval = [0 as Register; 2];
    if kernel::syscall::mi_syscall(p, code, false, &sargs, &mut rval, |_, _| Ok(())).is_err() {
        eprintln!("bench: the prologue refused the bench's call");
        std::process::exit(1);
    }
    let vm_ref: &'static Vmspace = VM
        .get()
        .map(|v| &**v)
        .unwrap_or_else(|| std::process::exit(1));

    let path = *b"/usr/lib/libc.\0\0";
    if UserPtr::<u8>::new(UBUF + 0x100)
        .copyout_slice(&path)
        .is_err()
    {
        eprintln!("bench: the bench's buffer is not in the user-copy double");
        std::process::exit(1);
    }
    let src = [0x5au8; 64];
    let mut dst = [0u8; 64];
    let mut name = [0u8; 64];

    let mut benches: Vec<Bench<'_>> = Vec::new();
    benches.push(Bench {
        name: "prim.copyin-64",
        ops: PRIM,
        f: Box::new(prim(|| {
            let r = black_box(UserPtr::<u8>::new(black_box(UBUF))).copyin_slice(&mut dst);
            let _ = black_box(r);
        })),
    });
    benches.push(Bench {
        name: "prim.copyout-64",
        ops: PRIM,
        f: Box::new(prim(|| {
            let r = black_box(UserPtr::<u8>::new(black_box(UBUF))).copyout_slice(&src);
            let _ = black_box(r);
        })),
    });
    benches.push(Bench {
        name: "prim.copyinstr-16",
        ops: PRIM,
        f: Box::new(prim(|| {
            let r = black_box(UserPtr::<u8>::new(black_box(UBUF + 0x100))).copyinstr(&mut name);
            let _ = black_box(r);
        })),
    });

    benches.push(Bench {
        name: "prim.pin-check",
        ops: PRIM,
        f: Box::new(prim(|| {
            let _ = black_box(kernel::syscall::pin_check(black_box(p), black_box(code)));
        })),
    });
    benches.push(Bench {
        name: "prim.mi-syscall-prologue",
        ops: PRIM,
        f: Box::new(prim(|| {
            let r = kernel::syscall::mi_syscall(
                black_box(p),
                black_box(code),
                false,
                &sargs,
                &mut rval,
                |_, r| {
                    r[0] = 7;
                    Ok(())
                },
            );
            let _ = black_box(r);
        })),
    });
    benches.push(Bench {
        name: "prim.inentry-hit",
        ops: PRIM,
        f: Box::new(prim(|| {
            let mut ie = t.p_spinentry.get();
            let ok = kernel::vm::map::uvm_map_inentry(
                &vm_ref.vm_map,
                &mut ie,
                black_box(SP),
                "sp",
                "not MAP_STACK",
                kernel::vm::map::uvm_map_inentry_sp,
                vm_ref.vm_map.sserial(),
            );
            t.p_spinentry.set(ie);
            let _ = black_box(ok);
        })),
    });

    let reps = 7;
    let target = Duration::from_millis(300);
    println!("# bench min median max ns/op, spread (max-min)/median, calls/rep, reps {reps}");
    for bench in benches.iter_mut() {
        if !args.is_empty() && !args.iter().any(|x| x == bench.name) {
            continue;
        }
        let (min, med, max, calls) = measure(bench, reps, target);
        println!(
            "{:<30} {:>12.1} {:>12.1} {:>12.1} {:>6.1}% {:>8}",
            bench.name,
            min,
            med,
            max,
            (max - min) / med * 100.0,
            calls
        );
    }
    std::process::exit(0);
}
/* </CODE> */
