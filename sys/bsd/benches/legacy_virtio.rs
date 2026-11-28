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
//! Host microbenchmarks of the legacy virtio core (N7.3a, decision 43): the "before" of the
//! kernel's `benches/dev.rs`, the same loops over LZ's `dev/pv/virtio.rs` (`bsd`'s legacy
//! core, which its five legacy children keep until N7.3d), on a queue built by hand over
//! leaked host memory with a test transport, as that file's host tests build theirs. Never
//! part of a kernel image; it measures and changes nothing.
//!
//! `cargo bench -p bsd --bench legacy_virtio` (release: the `bench` profile) or with
//! `--profile dev`. Arguments: the names of the benches to run (all by default).
//!
//! What an op is (as in `kernel`'s `benches/dev.rs`):
//! - `virtq.cycle1`: `virtio_enqueue_prep`, `virtio_enqueue_reserve` of one descriptor,
//!   `virtio_enqueue` of a one-segment map, `virtio_enqueue_commit` with its kick, the
//!   device's answer written in the used ring, `virtio_check_vq` (its `vq_done` counts),
//!   `virtio_dequeue`, `virtio_dequeue_commit`.
//! - `virtq.cycle3-indirect`: the same with a three-segment map in the slot's indirect table.
//! - `virtio.intr-empty`: `virtio_check_vq` of a queue the device used nothing of.

#![forbid(unsafe_code)]

use std::boxed::Box;
use std::cell::Cell;
use std::ffi::c_void;
use std::hint::black_box;
use std::ptr;
use std::sync::atomic::AtomicI32;
use std::time::{Duration, Instant};
use std::vec::Vec;

use bsd::dev::pv::virtio::{
    virtio_check_vq, virtio_dequeue, virtio_dequeue_commit, virtio_enqueue, virtio_enqueue_commit,
    virtio_enqueue_prep, virtio_enqueue_reserve, virtio_init_vq,
};
use bsd::dev::pv::virtiovar::{
    VirtioAttachArgs, VirtioFeatureName, VirtioOps, VirtioSoftc, Virtqueue, VqEntry,
};
use bsd::machine::bus::{BusDmaSegment, BusDmaTag, BusDmamap};
use bsd::machine::cpu::CpuInfo;
use bsd::machine::intr::IntrFn;
use bsd::sys::device::{DV_DULL, Device};
use bsd::sys::errno::Errno;
use bsd::sys::queue::{SlistEntry, SlistHead};
use hal::sync::once::OnceStatic;

/// The calls of a primitive per timed call.
const PRIM: usize = 1000;

/// The queue size.
const QSIZE: usize = 256;

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

// The test transport: a kick is a counter, the rest does nothing.

static KICKS: AtomicI32 = AtomicI32::new(0);

fn t_kick(_sc: &VirtioSoftc, _idx: u16) {
    KICKS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}
fn t_r1(_sc: &VirtioSoftc, _o: i32) -> u8 {
    0
}
fn t_r2(_sc: &VirtioSoftc, _o: i32) -> u16 {
    0
}
fn t_r4(_sc: &VirtioSoftc, _o: i32) -> u32 {
    0
}
fn t_r8(_sc: &VirtioSoftc, _o: i32) -> u64 {
    0
}
fn t_w1(_sc: &VirtioSoftc, _o: i32, _v: u8) {}
fn t_w2(_sc: &VirtioSoftc, _o: i32, _v: u16) {}
fn t_w4(_sc: &VirtioSoftc, _o: i32, _v: u32) {}
fn t_w8(_sc: &VirtioSoftc, _o: i32, _v: u64) {}
fn t_qsize(_sc: &VirtioSoftc, _i: u16) -> u16 {
    QSIZE as u16
}
fn t_setup_queue(_sc: &VirtioSoftc, _vq: &Virtqueue, _addr: u64) {}
fn t_setup_intrs(_sc: &VirtioSoftc) {}
fn t_get_status(_sc: &VirtioSoftc) -> i32 {
    0
}
fn t_set_status(_sc: &VirtioSoftc, _s: i32) {}
fn t_neg(_sc: &VirtioSoftc, _n: Option<&[VirtioFeatureName]>) -> Result<(), Errno> {
    Ok(())
}
fn t_finish(_sc: &VirtioSoftc, _va: *mut VirtioAttachArgs) -> Result<(), Errno> {
    Ok(())
}
fn t_poll(_arg: *mut c_void) -> i32 {
    0
}
fn t_barrier(_sc: &VirtioSoftc) {}
fn t_establish(
    _sc: &VirtioSoftc,
    _va: *mut VirtioAttachArgs,
    _vec: i32,
    _ci: Option<&'static CpuInfo>,
    _f: IntrFn,
    _a: *mut c_void,
) -> Result<(), Errno> {
    Err(Errno::ENXIO)
}

static BENCH_OPS: VirtioOps = VirtioOps {
    kick: t_kick,
    read_dev_cfg_1: t_r1,
    read_dev_cfg_2: t_r2,
    read_dev_cfg_4: t_r4,
    read_dev_cfg_8: t_r8,
    write_dev_cfg_1: t_w1,
    write_dev_cfg_2: t_w2,
    write_dev_cfg_4: t_w4,
    write_dev_cfg_8: t_w8,
    read_queue_size: t_qsize,
    setup_queue: t_setup_queue,
    setup_intrs: t_setup_intrs,
    get_status: t_get_status,
    set_status: t_set_status,
    neg_features: t_neg,
    attach_finish: t_finish,
    poll_intr: t_poll,
    intr_barrier: t_barrier,
    intr_establish: t_establish,
};

/// A queue's `vq_done`: the work is the bench's.
fn done(_vq: &Virtqueue) -> i32 {
    1
}

/// A loaded map of one segment, or of `n` segments of 16 bytes (the host's map has one
/// segment slot: a three-segment request is three maps of one).
fn map(addr: usize, len: usize) -> &'static BusDmamap {
    Box::leak(Box::new(BusDmamap {
        dm_mapsize: Cell::new(len),
        dm_nsegs: Cell::new(1),
        segs: [Cell::new(BusDmaSegment {
            ds_addr: addr,
            ds_len: len,
        })],
    }))
}

/// A softc with one queue of `QSIZE` slots laid out as `virtio_alloc_vq` lays it out (no
/// event index; `maxnsegs` indirect descriptors per slot when above 1), and its memory.
fn fixture(
    maxnsegs: usize,
) -> (
    &'static VirtioSoftc,
    &'static Virtqueue,
    &'static [Cell<u64>],
) {
    let sc: &'static VirtioSoftc = Box::leak(Box::new(VirtioSoftc {
        sc_dev: Device {
            dv_class: Cell::new(DV_DULL),
            dv_cfdata: Cell::new(None),
            dv_unit: Cell::new(0),
            dv_xname: Cell::new(*b"virtio0\0\0\0\0\0\0\0\0\0"),
            dv_parent: Cell::new(None),
            dv_flags: Cell::new(0),
            dv_ref: AtomicI32::new(0),
        },
        sc_dmat: Cell::new(Some(BusDmaTag::default())),
        sc_ops: Cell::new(Some(&BENCH_OPS)),
        sc_ipl: Cell::new(0),
        sc_driver_features: Cell::new(0),
        sc_active_features: Cell::new(0),
        sc_indirect: Cell::new(i32::from(maxnsegs > 1)),
        sc_version_1: Cell::new(0),
        sc_nvqs: Cell::new(0),
        sc_vqs: Cell::new(ptr::null_mut()),
        sc_child: Cell::new(ptr::null()),
        sc_config_change: Cell::new(None),
        sc_native: OnceStatic::new(),
    }));

    let a1 = (16 * QSIZE + 2 * (2 + QSIZE)).next_multiple_of(4096);
    let a2 = (2 * 2 + 8 * QSIZE).next_multiple_of(4096);
    let a3 = if maxnsegs > 1 {
        16 * maxnsegs * QSIZE
    } else {
        0
    };
    let size = a1 + a2 + a3;
    let mem: &'static [Cell<u64>] = Vec::from_iter((0..size / 8).map(|_| Cell::new(0))).leak();
    let base = mem.as_ptr().cast::<u8>().cast_mut();
    let entries: &'static mut [VqEntry] = Vec::from_iter((0..QSIZE).map(|_| VqEntry {
        qe_list: SlistEntry::new(),
        qe_index: Cell::new(0),
        qe_next: Cell::new(0),
        qe_indirect: Cell::new(0),
        qe_vr_index: Cell::new(0),
        qe_desc_base: Cell::new(ptr::null_mut()),
    }))
    .leak();

    let vq: &'static Virtqueue = Box::leak(Box::new(Virtqueue {
        vq_owner: Cell::new(ptr::from_ref(sc)),
        vq_num: Cell::new(QSIZE as u32),
        vq_mask: Cell::new(QSIZE as u32 - 1),
        vq_index: Cell::new(0),
        vq_desc: Cell::new(base.cast()),
        vq_avail: Cell::new(base.wrapping_add(16 * QSIZE).cast()),
        vq_used: Cell::new(base.wrapping_add(a1).cast()),
        vq_indirect: Cell::new(if a3 > 0 {
            base.wrapping_add(a1 + a2).cast()
        } else {
            ptr::null_mut()
        }),
        vq_vaddr: Cell::new(base),
        vq_availoffset: Cell::new((16 * QSIZE) as i32),
        vq_usedoffset: Cell::new(a1 as i32),
        vq_indirectoffset: Cell::new(if a3 > 0 { (a1 + a2) as i32 } else { 0 }),
        vq_segs: [Cell::new(BusDmaSegment::default())],
        vq_bytesize: Cell::new(size as u32),
        vq_dmamap: Cell::new(Some(map(0x10_0000, size))),
        vq_maxnsegs: Cell::new(maxnsegs as i32),
        vq_entries: Cell::new(entries.as_mut_ptr()),
        vq_freelist: SlistHead::new(),
        vq_avail_idx: Cell::new(0),
        vq_used_idx: Cell::new(0),
        vq_queued: Cell::new(0),
        vq_done: Cell::new(Some(done)),
        vq_notify_off: Cell::new(0),
        vq_intr_vec: Cell::new(0),
    }));
    sc.sc_vqs.set(ptr::from_ref(vq).cast_mut());
    sc.sc_nvqs.set(1);
    virtio_init_vq(sc, vq);
    (sc, vq, mem)
}

/// The `u16` at byte `off` of `mem` (little-endian words).
fn peek16(mem: &[Cell<u64>], off: usize) -> u16 {
    (mem[off / 8].get() >> (8 * (off % 8))) as u16
}

/// `v` written at byte `off` of `mem`, `bytes` wide (2 or 4).
fn poke(mem: &[Cell<u64>], off: usize, bytes: usize, v: u64) {
    let shift = 8 * (off % 8);
    let mask = (u64::MAX >> (64 - 8 * bytes)) << shift;
    let w = &mem[off / 8];
    w.set((w.get() & !mask) | ((v << shift) & mask));
}

/// The device used `slot`: the used ring's next element, then its index.
fn device_uses(mem: &[Cell<u64>], slot: u32) {
    let used = (16 * QSIZE + 2 * (2 + QSIZE)).next_multiple_of(4096);
    let idx = peek16(mem, used + 2);
    let e = used + 4 + 8 * (usize::from(idx) % QSIZE);
    poke(mem, e, 4, u64::from(slot));
    poke(mem, e + 4, 4, 16);
    poke(mem, used + 2, 2, u64::from(idx.wrapping_add(1)));
}

/// One request of `maps` (one segment each) through `vq` and back.
fn cycle(sc: &VirtioSoftc, vq: &Virtqueue, mem: &[Cell<u64>], maps: &[&BusDmamap]) {
    let Ok(slot) = virtio_enqueue_prep(vq) else {
        return;
    };
    if virtio_enqueue_reserve(vq, slot, maps.len() as i32).is_err() {
        return;
    }
    for m in maps {
        virtio_enqueue(vq, slot, m, false);
    }
    virtio_enqueue_commit(sc, vq, slot, true);
    device_uses(mem, slot as u32);
    if virtio_check_vq(sc, vq) != 0
        && let Ok((s, len)) = virtio_dequeue(sc, vq)
    {
        black_box(len);
        virtio_dequeue_commit(vq, s);
    }
}

fn main() {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| a != "--bench")
        .collect();
    kernel::testutil::install();

    let (sc1, vq1, mem1) = fixture(1);
    let (sc3, vq3, mem3) = fixture(4);
    let (sce, vqe, _meme) = fixture(1);
    let one = [map(0x9000, 16)];
    let three = [map(0x9000, 16), map(0xa000, 512), map(0xb000, 1)];

    let benches: [(&str, &dyn Fn()); 3] = [
        ("virtq.cycle1", &|| cycle(sc1, vq1, mem1, &one)),
        ("virtq.cycle3-indirect", &|| cycle(sc3, vq3, mem3, &three)),
        ("virtio.intr-empty", &|| {
            black_box(virtio_check_vq(sce, vqe));
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
