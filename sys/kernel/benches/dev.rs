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
//! Host microbenchmarks of the virtio core's paths (N7.3a, decision 43: the virtqueue is on
//! the I/O path): a request through a queue, a transport's interrupt on an empty queue, and a
//! kernel-locked driver's state cell (`kernel::dev::pv::virtio`), over a fake transport and
//! leaked host pages (`FakeTransport`, `FakeDma`; feature `doubles`). Never part of a kernel
//! image; it measures and changes nothing.
//!
//! `cargo bench -p kernel --features doubles --bench dev` (release: the `bench` profile) or
//! with `--profile dev`. Arguments: the names of the benches to run (all by default).
//!
//! What an op is (the "before" of each `Bench:` line is `bsd`'s `benches/legacy_virtio.rs`,
//! the same loop over LZ's `virtio.rs`):
//! - `virtq.cycle1`: one request of one descriptor (`enqueue_prep`, `enqueue_reserve`,
//!   `enqueue` of one segment, `enqueue_commit` with its kick), the device's answer written
//!   in the used ring, `check`, `dequeue`, `dequeue_commit`: viornd's request.
//! - `virtq.cycle3-indirect`: the same with three segments in the slot's indirect table.
//! - `virtio.intr-empty`: the transport's interrupt for a queue the device used nothing of
//!   (`ChildRef::check_vq`: the erased call, the state's lock, `check`).
//! - `virtio.state-lock`: the state's `Mutexed` at `IPL_NET` taken and released (what a
//!   kernel-locked driver's `splnet` section costs here, P33).

#![forbid(unsafe_code)]

use std::cell::RefCell;
use std::hint::black_box;
use std::time::{Duration, Instant};
use std::vec::Vec;

use hal::cpu::intr::IPL_NET;
use hal::dev::dma::DmaSeg;
use hal::mem::malloc::{M_NOWAIT, MArc};
use hal::sync::mutex::Mutexed;
use kernel::dev::device::{ConfigRow, DriverOps};
use kernel::dev::pv::virtio::{
    ChildRef, FakeTransport, Features, VirtioChild, VirtioTransport, Virtqueue, test_node,
};
use kernel::dev::pv::virtioreg::VIRTIO_F_RING_INDIRECT_DESC;
use kernel::testutil;

/// The calls of a primitive per timed call.
const PRIM: usize = 1000;

/// `M_DEVBUF`.
const M_DEVBUF: i32 = 2;

/// ns per op of `reps` repetitions of about `target` of wall time each: min, median, max.
fn measure(mut f: impl FnMut(), reps: usize, target: Duration) -> (f64, f64, f64) {
    let mut calls = 1usize;
    loop {
        let t = Instant::now();
        for _ in 0..calls * PRIM {
            f();
        }
        let wall = t.elapsed();
        if wall >= target / 4 || calls >= 1 << 20 {
            let per = wall.as_secs_f64() / calls as f64;
            calls = ((target.as_secs_f64() / per.max(1e-9)) as usize).max(3);
            break;
        }
        calls *= 4;
    }
    let mut v = Vec::with_capacity(reps);
    for _ in 0..reps {
        let t = Instant::now();
        for _ in 0..calls * PRIM {
            f();
        }
        v.push(t.elapsed().as_nanos() as f64 / (calls * PRIM) as f64);
    }
    v.sort_by(f64::total_cmp);
    (v[0], v[v.len() / 2], v[v.len() - 1])
}

/// One request of `segs` through `vq` and back.
fn cycle(t: &dyn VirtioTransport, vq: &mut Virtqueue, segs: &[DmaSeg]) {
    let Ok(slot) = vq.enqueue_prep() else {
        return;
    };
    if vq.enqueue_reserve(slot, segs.len()).is_err() {
        return;
    }
    vq.enqueue(slot, segs, false);
    vq.enqueue_commit(t, Some(slot), true);
    vq.device().use_slot(u32::from(slot), 16);
    if vq.check()
        && let Ok((s, len)) = vq.dequeue()
    {
        black_box(len);
        vq.dequeue_commit(s);
    }
}

/// A child of one queue in a `Mutexed` at `IPL_NET`, as viornd's state.
struct BenchChild {
    vq: Mutexed<Virtqueue>,
}

impl VirtioChild for BenchChild {
    fn vq_intr(&self, _index: usize) -> i32 {
        let mut vq = self.vq.lock();
        if !vq.check() {
            return 0;
        }
        1
    }
}

/// The bench child's row: constants only.
struct BenchRow;

impl ConfigRow for BenchRow {
    fn driver(&'static self) -> &'static dyn DriverOps {
        unreachable!("the bench's node is never attached")
    }
    fn parents(&self) -> &'static [i16] {
        &[]
    }
    fn starunit1(&self) -> i16 {
        0
    }
    fn unit(&self) -> i16 {
        0
    }
    fn set_unit(&self, _unit: i16) {}
    fn fstate(&self) -> i16 {
        0
    }
    fn set_fstate(&self, _fstate: i16) {}
}

static BENCH_ROW: BenchRow = BenchRow;

fn main() {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| a != "--bench")
        .collect();
    testutil::install();

    let t = FakeTransport::make(256, VIRTIO_F_RING_INDIRECT_DESC);
    let plain = Features::default();
    let indirect = Features::of_active(VIRTIO_F_RING_INDIRECT_DESC);
    let one = RefCell::new(Virtqueue::alloc(&t, &plain, 0, 1, "bench").expect("a queue"));
    let three = RefCell::new(Virtqueue::alloc(&t, &indirect, 0, 4, "bench").expect("a queue"));
    let seg = [DmaSeg {
        addr: 0x9000,
        len: 16,
    }];
    let segs3 = [
        DmaSeg {
            addr: 0x9000,
            len: 16,
        },
        DmaSeg {
            addr: 0xa000,
            len: 512,
        },
        DmaSeg {
            addr: 0xb000,
            len: 1,
        },
    ];
    let child = MArc::new(
        M_DEVBUF,
        M_NOWAIT,
        BenchChild {
            vq: Mutexed::new(
                IPL_NET,
                Virtqueue::alloc(&t, &plain, 0, 1, "bench").expect("a queue"),
            ),
        },
    )
    .expect("the child's state");
    let node = test_node(&BENCH_ROW, "vbench0");
    let cref = ChildRef::of(&child, &node);
    let lock = Mutexed::new(IPL_NET, 0u64);

    let benches: [(&str, &dyn Fn()); 4] = [
        ("virtq.cycle1", &|| cycle(&t, &mut one.borrow_mut(), &seg)),
        ("virtq.cycle3-indirect", &|| {
            cycle(&t, &mut three.borrow_mut(), &segs3);
        }),
        ("virtio.intr-empty", &|| {
            black_box(cref.check_vq(0));
        }),
        ("virtio.state-lock", &|| {
            *lock.lock() += 1;
        }),
    ];

    let reps = 7;
    let target = Duration::from_millis(300);
    println!("# bench min median max ns/op, reps {reps}");
    for (name, f) in benches {
        if !args.is_empty() && !args.iter().any(|x| x == name) {
            continue;
        }
        let (min, med, max) = measure(f, reps, target);
        println!("{name:<30} {min:>12.1} {med:>12.1} {max:>12.1}");
    }
    std::process::exit(0);
}
/* </CODE> */
