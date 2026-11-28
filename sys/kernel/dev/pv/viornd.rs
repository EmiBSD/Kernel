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

/*
 * Copyright (c) 2014 Stefan Fritsch <sf@sfritsch.de>
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
//! `viornd(4)` in the kernel: the virtio entropy device (`viornd* at virtio?`), a source for
//! `rnd(4)`.
//!
//! Upstream: sys/dev/pv/viornd.c @ 3ce1f3f79392
//! LZ: sys/dev/pv/viornd.rs@e8249449d905
//!
//! The host may not have an unlimited supply of entropy, so the driver does not take all it
//! can: it asks for [`VIORND_BUFSIZE`] bytes once at boot (a timeout one tick after the
//! attach) and again every `15 * (1 << interval_shift)` seconds after each answer. The shift
//! comes from the second byte of the row's config flags (the lowest byte is the
//! transport's), 5 when it is 0; [`VIORND_ONESHOT`] asks only once. Each answer's whole
//! 32-bit words go to `enqueue_randomness` (`hal::random`, `rnd(4)`'s entropy input ring).
//! XXX There should be an API to check if we actually need more entropy.
//!
//! ## Deviations
//! - U13 (the user's decision of 2026-11-28): one `DmaBuf::alloc` stands for the C's
//!   `dma_alloc`, `bus_dmamap_create` and `bus_dmamap_load`, so `: Can't alloc dmamap` and
//!   `: Can't load dmamap` read `: Can't alloc dma buffer`. They print only when memory runs
//!   out during the attach.
//! - The buffer is a page of its own (`DmaBuf`'s granule) where the C takes 16 bytes of
//!   `dma_alloc` (P40); the device is told 16 bytes, as the C's map.
//! - A failed `virtio_attach_finish` deletes the request timeout before the child is marked
//!   failed; the C frees its buffer and destroys its map with the timeout still armed, which
//!   one tick later syncs the destroyed map and enqueues it (`viornd.c:137-150`: a slip of
//!   the C, for the user; nothing is reported upstream).
//! - A length of 2^31 or more in the used ring (negative as the C's `int`): the C's loop
//!   `(i + 1) * sizeof(int) <= len` compares it as a huge `size_t` and reads past its 16
//!   bytes; here, as in LZ, no word is read, and the next request is armed.
//! - The interval and the word count are the functions [`viornd_interval`] and
//!   [`viornd_nwords`], host-tested; the words go out through [`viornd_words`]. `VIORND_DEBUG`
//!   is 0 as in the C; its messages are compiled behind `if VIORND_DEBUG > 0`.
//!
//! ## Redesign
//! - LZ's `struct viornd_softc` was the C's (a `#[repr(C)]` softc of `Cell`s behind an
//!   `unsafe impl Softc`, `sc_buf` a raw pointer read volatile, the parent's softc cast from
//!   its header, `vq_owner` cast back to the child's); every member is now the core's,
//!   [`ViorndCore`], a counted block built whole at attach (P37) that the request timeout,
//!   the transport's `ChildRef` and the node's softc ([`Viornd`]) hold: the transport
//!   (`sc_virtio`, a `Box<dyn VirtioTransport>`), the interval, and the queue and the buffer
//!   (`sc_vq`, `sc_buf` with `sc_dmamap`: a `DmaBuf`) in a `Mutexed` at `IPL_NET`, the C's
//!   kernel lock and `splnet` (P33). The interrupt reads no set-once value.
//! - `viornd_ca`/`viornd_cd` -> [`VIORND`], a [`DriverOps`] whose probe reads the transport's
//!   [`VirtioArgs`] back from [`Aux::Typed`]; `cfdata[]`'s row 71 names it (`Cfdata::native`)
//!   with a stub `Cfdriver` of the name, `CD_COCOVM` (`bsd`'s `conf/ioconf.rs`).
//! - `viornd_vq_done` is [`ViorndCore`]'s `VirtioChild::vq_intr` (with `virtio_check_vq`'s
//!   test of the queue first), `viornd_tick` its `tick`, a shared timeout function.

use alloc::boxed::Box;

use hal::cpu::intr::IPL_NET;
use hal::dev::dma::{DmaBuf, DmaReq, DmaSeg, DmaSync};
use hal::mem::malloc::{M_NOWAIT, MArc, MBox};
use hal::mem::pool::AnyArc;
use hal::sync::mutex::Mutexed;
use hal::sync::once::OnceStatic;

use crate::assert::kpanic;
use crate::base::prf::printf;
use crate::dev::device::{
    Aux, CD_COCOVM, DV_DULL, DevRef, Devclass, DriverOps, Ident, M_DEVBUF, Match, UnitTable,
};
use crate::dev::pv::virtio::{
    ChildRef, VirtioArgs, VirtioChild, VirtioTransport, Virtqueue, virtio_attach_finish,
};
use crate::dev::pv::virtioreg::PCI_PRODUCT_VIRTIO_ENTROPY;
use crate::time::timeout::{KCLOCK_NONE, Timeout};

/// `VIORND_INTERVAL_SHIFT_DEFAULT`.
pub const VIORND_INTERVAL_SHIFT_DEFAULT: u32 = 5;
/// `VIORND_ONESHOT`: ask for entropy at boot only.
pub const VIORND_ONESHOT: i32 = 0x1000;
/// `VIORND_BUFSIZE`: the bytes asked for each time.
pub const VIORND_BUFSIZE: usize = 16;

/// `VIORND_DEBUG`.
const VIORND_DEBUG: i32 = 0;

/// `VIORND_INTERVAL_SHIFT(f)`: the interval's shift, from the flags' second byte.
#[inline]
pub const fn viornd_interval_shift(f: i32) -> u32 {
    ((f >> 8) & 0xf) as u32
}

/// The device's softc: its state, the core the attach builds (none when the attach failed
/// before it).
pub struct Viornd {
    /// The state the attach built.
    core: OnceStatic<MArc<ViorndCore>>,
}

/// `struct viornd_softc` but `sc_dev`: what the attach builds whole, which the request
/// timeout, the transport's interrupt and the node share.
pub struct ViorndCore {
    /// `sc_virtio`: the transport.
    transport: Box<dyn VirtioTransport>,
    /// `sc_interval`: seconds between requests, 0 for one request only.
    interval: u32,
    /// `sc_dev.dv_xname`, for the messages.
    xname: [u8; 16],
    /// \[`IPL_NET`\] the queue and the buffer (the C's kernel lock and `splnet`).
    st: Mutexed<ViorndState>,
    /// `sc_tick`: the next request.
    tick: MBox<Timeout>,
}

/// What the attach, the interrupt and the timeout change: `sc_vq`, `sc_buf` and its map.
pub struct ViorndState {
    /// `sc_vq`: the one queue, "Entropy request".
    vq: Virtqueue,
    /// `sc_buf` with `sc_dmamap`: [`VIORND_BUFSIZE`] bytes the device fills.
    buf: DmaBuf,
}

impl ViorndCore {
    /// `sc_dev.dv_xname`.
    fn xname(&self) -> &str {
        let len = self.xname.iter().position(|&b| b == 0).unwrap_or(16);
        core::str::from_utf8(&self.xname[..len]).unwrap_or("?")
    }

    /// `viornd_tick`: the buffer put on the queue for the device to fill.
    fn tick(&self) {
        let mut g = self.st.lock();
        let st = &mut *g;
        st.buf.sync(0, VIORND_BUFSIZE, DmaSync::PREREAD);
        let slot = match st.vq.enqueue_prep() {
            Ok(slot) if st.vq.enqueue_reserve(slot, 1).is_ok() => slot,
            _ => kpanic(format_args!("{}: virtqueue enqueue failed", self.xname())),
        };
        let seg = DmaSeg {
            addr: st.buf.seg().addr,
            len: VIORND_BUFSIZE,
        };
        st.vq.enqueue(slot, &[seg], false);
        st.vq.enqueue_commit(&*self.transport, Some(slot), true);
    }
}

/// `viornd_ca` with `viornd_cd`'s constants.
pub struct ViorndDriver;

/// The driver `cfdata[]`'s `viornd* at virtio?` row names (`Cfdata::native`).
pub static VIORND: ViorndDriver = ViorndDriver;

/// `viornd_cd`'s `cd_devs`.
static VIORND_UNITS: UnitTable = UnitTable::new();

impl VirtioChild for ViorndCore {
    /// `virtio_check_vq` of the queue, then `viornd_vq_done`: the device's answer handed to
    /// `rnd(4)` and the next request scheduled.
    fn vq_intr(&self, _index: usize) -> i32 {
        let mut g = self.st.lock();
        let st = &mut *g;
        if !st.vq.check() {
            return 0;
        }

        // viornd_vq_done
        let Ok((slot, used)) = st.vq.dequeue() else {
            return 0;
        };
        // The C's int.
        let len = used as i32;
        st.buf.sync(0, VIORND_BUFSIZE, DmaSync::POSTREAD);
        if len > VIORND_BUFSIZE as i32 {
            printf(format_args!(
                "{}: inconsistent descriptor length {} > {}\n",
                self.xname(),
                len,
                VIORND_BUFSIZE
            ));
        } else {
            if VIORND_DEBUG > 0 {
                printf(format_args!("viornd_vq_done: got {len} bytes of entropy\n"));
            }
            viornd_words(&st.buf, len, hal::random::enqueue_randomness);

            if self.interval != 0 {
                let _ = self.tick.as_pin().add_sec(self.interval as i32);
            }
        }

        // out:
        st.vq.dequeue_commit(slot);
        1
    }
}

/// `viornd_tick`'s timeout function: the core it holds asks the device again.
fn viornd_tick(arg: &AnyArc) {
    if let Some(core) = arg.downcast_ref::<ViorndCore>() {
        core.tick();
    }
}

/// `viornd_match`: the virtio entropy device; 0 for any other argument.
pub fn viornd_match(va: &VirtioArgs) -> i32 {
    i32::from(va.devid == PCI_PRODUCT_VIRTIO_ENTROPY)
}

/// The seconds between two requests for config flags `flags`: 0 with [`VIORND_ONESHOT`],
/// else `15 * (1 << shift)`, the shift [`VIORND_INTERVAL_SHIFT_DEFAULT`] when the flags give
/// none.
pub const fn viornd_interval(flags: i32) -> u32 {
    if flags & VIORND_ONESHOT != 0 {
        0
    } else {
        let mut shift = viornd_interval_shift(flags);
        if shift == 0 {
            shift = VIORND_INTERVAL_SHIFT_DEFAULT;
        }
        15 * (1 << shift)
    }
}

/// How many whole 32-bit words a `len`-byte answer holds: the C's loop bound,
/// `(i + 1) * sizeof(int) <= len`; none for a length below one word (a negative one
/// included: see the module's deviations).
pub const fn viornd_nwords(len: i32) -> usize {
    if len <= 0 {
        0
    } else {
        len as usize / size_of::<i32>()
    }
}

/// The answer's whole words, `sc_buf[i]` in order, each handed to `out` (the C's
/// `enqueue_randomness(sc->sc_buf[i])`), at most the buffer's four.
pub fn viornd_words(buf: &DmaBuf, len: i32, mut out: impl FnMut(u32)) {
    let n = viornd_nwords(len).min(VIORND_BUFSIZE / size_of::<u32>());
    for i in 0..n {
        out(buf.read::<u32>(i * size_of::<u32>()));
    }
}

/// `viornd_attach`: claim the device, negotiate, the interval from the row's flags, the
/// buffer and the queue, the first request a tick away, then `virtio_attach_finish`. Each
/// failure marks the child failed (the transport then prints "virtio configuration
/// failed"), after the C's message.
pub fn viornd_attach(parent: &DevRef, dev: &DevRef, va: &VirtioArgs) {
    let t = (va.transport)(parent);
    if !t.claim(dev, IPL_NET) {
        kpanic(format_args!("already attached to something else"));
    }

    // err: the child marked failed; what was made goes with its drop.
    let Ok(f) = t.negotiate_features(0, None) else {
        t.child_failed();
        return;
    };

    let interval = viornd_interval(dev.row().flags());
    if VIORND_DEBUG > 0 {
        printf(format_args!(": request interval: {interval}s\n"));
    }

    let req = DmaReq {
        size: VIORND_BUFSIZE,
        nowait: true,
        zero: true,
        ..DmaReq::default()
    };
    let Ok(buf) = DmaBuf::alloc(t.dma(), &req) else {
        printf(format_args!(": Can't alloc dma buffer\n"));
        t.child_failed();
        return;
    };

    let Ok(mut vq) = Virtqueue::alloc(&*t, &f, 0, 1, "Entropy request") else {
        printf(format_args!(": Can't alloc virtqueue\n"));
        t.child_failed();
        return;
    };

    let _ = vq.start_intr();
    let queue = vq.setup();
    let Some(tick) = MBox::new(M_DEVBUF, M_NOWAIT, Timeout::unset()) else {
        kpanic(format_args!(
            "viornd_attach: allocation of the softc failed"
        ));
    };
    let Some(core) = MArc::new(
        M_DEVBUF,
        M_NOWAIT,
        ViorndCore {
            transport: t,
            interval,
            xname: *dev.xname_bytes(),
            st: Mutexed::new(IPL_NET, ViorndState { vq, buf }),
            tick,
        },
    ) else {
        kpanic(format_args!(
            "viornd_attach: allocation of the softc failed"
        ));
    };
    core.tick
        .set_shared(viornd_tick, MArc::erase(core.clone()), KCLOCK_NONE, 0);
    let _ = core.tick.as_pin().add(1);
    if let Some(sc) = dev.softc().downcast_ref::<Viornd>() {
        let _ = sc.core.set(core.clone());
    }

    printf(format_args!("\n"));
    if virtio_attach_finish(&*core.transport, va, ChildRef::of(&core, dev), &[queue]).is_err() {
        // err2, err: the request first (the module's deviations), then the child failed.
        let _ = core.tick.del();
        core.tick.clear();
        core.transport.child_failed();
    }
}

impl DriverOps for ViorndDriver {
    fn name(&self) -> &'static [u8] {
        b"viornd"
    }
    fn class(&self) -> Devclass {
        DV_DULL
    }
    fn mode(&self) -> i32 {
        CD_COCOVM
    }
    fn units(&self) -> &'static UnitTable {
        &VIORND_UNITS
    }
    fn make_softc(&self, _id: &Ident<'_>) -> AnyArc {
        match MArc::new(
            M_DEVBUF,
            M_NOWAIT,
            Viornd {
                core: OnceStatic::new(),
            },
        ) {
            Some(sc) => MArc::erase(sc),
            None => kpanic(format_args!(
                "config_make_softc: allocation for device softc failed"
            )),
        }
    }
    fn probe(&self, _parent: Option<&DevRef>, _m: &Match, aux: &Aux<'_>) -> i32 {
        VirtioArgs::of_aux(aux).map_or(0, viornd_match)
    }
    fn attach(&self, parent: Option<&DevRef>, dev: &DevRef, aux: &Aux<'_>) {
        let (Some(parent), Some(va)) = (parent, VirtioArgs::of_aux(aux)) else {
            kpanic(format_args!("{}: not a virtio device", dev.xname()));
        };
        viornd_attach(parent, dev, va);
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of viornd(4): the request interval from the config flags, how many words of
    // an answer go to rnd(4), and the driver over a FakeTransport: a whole request cycle, the
    // one-shot flag, a length the buffer cannot hold, and each attach failure.

    use core::sync::atomic::{AtomicI32, Ordering};
    use std::vec::Vec;
    use std::{assert, assert_eq};

    use super::*;
    use crate::dev::device::ConfigRow;
    use crate::dev::pv::virtio::{FakeEvent, FakeTransport, fake_transport, test_node};
    use crate::dev::pv::virtioreg::{VIRTIO_CONFIG_DEVICE_STATUS_DRIVER_OK, VRING_DESC_F_WRITE};
    use crate::testutil;

    #[test]
    fn interval_from_flags() {
        // GENERIC's `viornd* at virtio?` has no flags: every 15 << 5 seconds.
        assert_eq!(viornd_interval(0), 480);
        // The lowest byte is the transport's.
        assert_eq!(viornd_interval(0x00ff), 480);
        assert_eq!(viornd_interval(0x0100), 30);
        assert_eq!(viornd_interval(0x0f00), 15 << 15);
        assert_eq!(viornd_interval(VIORND_ONESHOT), 0);
        assert_eq!(viornd_interval(VIORND_ONESHOT | 0x0300), 0);
    }

    #[test]
    fn whole_words_only() {
        assert_eq!(viornd_nwords(0), 0);
        assert_eq!(viornd_nwords(3), 0);
        assert_eq!(viornd_nwords(4), 1);
        assert_eq!(viornd_nwords(15), 3);
        assert_eq!(viornd_nwords(VIORND_BUFSIZE as i32), 4);
        assert_eq!(viornd_nwords(-1), 0);
        assert_eq!(viornd_nwords(i32::MIN), 0);
    }

    /// A `viornd* at virtio?` row with its flags.
    struct Row(AtomicI32);

    impl ConfigRow for Row {
        fn driver(&'static self) -> &'static dyn DriverOps {
            &VIORND
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
        fn flags(&self) -> i32 {
            self.0.load(Ordering::Relaxed)
        }
    }

    /// The node of a viornd attached by hand with `flags` over a fresh transport of a queue
    /// of `qsize`; the transport and the node's softc.
    struct Attached {
        t: &'static FakeTransport,
        dev: DevRef,
    }

    impl Attached {
        /// The core the attach built.
        fn core(&self) -> &MArc<ViorndCore> {
            let sc = self.dev.softc().downcast_ref::<Viornd>().expect("viornd");
            sc.core.get().expect("a core")
        }
    }

    impl Drop for Attached {
        /// The request taken off the wheel, so no other test's softclock runs it.
        fn drop(&mut self) {
            let sc = self.dev.softc().downcast_ref::<Viornd>();
            if let Some(core) = sc.and_then(|s| s.core.get()) {
                let _ = core.tick.del();
            }
        }
    }

    /// A transport of a queue of `qsize` and an unattached viornd node with `flags`.
    fn prepare(qsize: u16, flags: i32) -> (&'static FakeTransport, DevRef, DevRef, VirtioArgs) {
        testutil::install();
        let t = FakeTransport::make(qsize, 0);
        let row: &'static Row =
            std::boxed::Box::leak(std::boxed::Box::new(Row(AtomicI32::new(flags))));
        let parent = test_node(row, "virtio28");
        let xname = *b"viornd0\0\0\0\0\0\0\0\0\0";
        let id = Ident {
            class: DV_DULL,
            unit: 0,
            xname: &xname,
            row,
            parent: Some(&parent),
        };
        let dev = crate::dev::device::Device::new(&id, 0, VIORND.make_softc(&id));
        let va = VirtioArgs {
            devid: PCI_PRODUCT_VIRTIO_ENTROPY,
            nintr: 1,
            transport: fake_transport,
            legacy: 0,
        };
        (t, parent, dev, va)
    }

    fn attach(qsize: u16, flags: i32) -> Attached {
        let (t, parent, dev, va) = prepare(qsize, flags);
        VIORND.attach(Some(&parent), &dev, &Aux::Typed(&va));
        Attached { t, dev }
    }

    /// The device answers the request on the queue with `words` and `len`.
    fn answer(a: &Attached, words: [u32; 4], len: u32) {
        let core = a.core();
        let mut g = core.st.lock();
        let st = &mut *g;
        for (i, w) in words.iter().enumerate() {
            st.buf.write::<u32>(4 * i, *w);
        }
        let last = st.vq.device().avail_idx().wrapping_sub(1) & (st.vq.num() - 1);
        let slot = st.vq.device().avail_ring(last);
        st.vq.device().use_slot(u32::from(slot), len);
    }

    #[test]
    fn the_probe_answers_the_entropy_device_only() {
        testutil::install();
        let m = Match::Row(&ROW0);
        let mut va = VirtioArgs {
            devid: PCI_PRODUCT_VIRTIO_ENTROPY,
            nintr: 1,
            transport: fake_transport,
            legacy: 0,
        };
        assert_eq!(VIORND.probe(None, &m, &Aux::Typed(&va)), 1);
        va.devid = 5;
        assert_eq!(VIORND.probe(None, &m, &Aux::Typed(&va)), 0);
        assert_eq!(VIORND.probe(None, &m, &Aux::Legacy(0x1000)), 0);
        assert_eq!(VIORND.probe(None, &m, &Aux::Typed(&4i32)), 0);
        assert_eq!(VIORND.name(), b"viornd");
        assert_eq!(VIORND.mode(), CD_COCOVM);
    }

    static ROW0: Row = Row(AtomicI32::new(0));

    #[test]
    fn a_whole_request_cycle_feeds_rnd_four_words() {
        let a = attach(8, 0);
        let t = a.t;
        let (child, ipl) = t.claimed.lock().unwrap().clone().expect("claimed");
        assert!(MArc::ptr_eq(&child, &a.dev));
        assert_eq!(ipl, IPL_NET);
        let ev = t.events();
        assert_eq!(ev[0], FakeEvent::Negotiate(0));
        assert_eq!(ev[1], FakeEvent::AttachFinish(1));
        assert_eq!(ev[2], FakeEvent::SetupIntrs);
        assert!(matches!(ev[3], FakeEvent::SetupQueue(q) if q.index == 0 && q.num == 8));
        assert_eq!(
            ev[4],
            FakeEvent::SetStatus(VIRTIO_CONFIG_DEVICE_STATUS_DRIVER_OK)
        );
        assert!(!t.failed.load(Ordering::Relaxed));
        let core = a.core();
        assert_eq!(core.interval, 480);
        assert!(core.tick.pending(), "the first request a tick away");

        // The tick: one descriptor of 16 device-writable bytes, kicked.
        let _ = core.tick.del();
        viornd_tick(&MArc::erase(core.clone()));
        assert_eq!(t.kicks.load(Ordering::Relaxed), 1);
        {
            let mut g = core.st.lock();
            let st = &mut *g;
            let buf = st.buf.seg().addr;
            let d = st.vq.device();
            assert_eq!(d.avail_idx(), 1);
            let slot = d.avail_ring(0);
            let (addr, len, flags, _) = d.desc(slot);
            assert_eq!((len, flags), (16, VRING_DESC_F_WRITE));
            assert_eq!(addr, buf);
        }
        assert_eq!(t.interrupt(), 0, "nothing used yet");

        // The answer: four words to rnd(4), the next request 480 s away.
        let mut words = Vec::new();
        {
            let g = core.st.lock();
            viornd_words(&g.buf, 16, |w| words.push(w));
        }
        assert_eq!(words, [0, 0, 0, 0], "a zeroed buffer");
        let before = testutil::randomness_enqueued();
        answer(&a, [0x1111_1111, 0x2222_2222, 0x3333_3333, 0x4444_4444], 16);
        assert_eq!(t.interrupt(), 1);
        assert_eq!(testutil::randomness_enqueued() - before, 4);
        {
            let g = core.st.lock();
            words.clear();
            viornd_words(&g.buf, 16, |w| words.push(w));
            assert_eq!(g.vq.nfree(), 8, "the slot back");
        }
        assert_eq!(words, [0x1111_1111, 0x2222_2222, 0x3333_3333, 0x4444_4444]);
        assert!(core.tick.pending(), "rescheduled");
        assert_eq!(t.interrupt(), 0, "one answer, one call");

        // Fewer bytes: whole words only.
        let _ = core.tick.del();
        viornd_tick(&MArc::erase(core.clone()));
        let before = testutil::randomness_enqueued();
        answer(&a, [1, 2, 3, 4], 11);
        assert_eq!(t.interrupt(), 1);
        assert_eq!(testutil::randomness_enqueued() - before, 2);
    }

    #[test]
    fn a_length_above_the_buffer_prints_and_enqueues_nothing() {
        let a = attach(8, 0);
        let core = a.core();
        let _ = core.tick.del();
        viornd_tick(&MArc::erase(core.clone()));
        let before = testutil::randomness_enqueued();
        answer(&a, [1, 2, 3, 4], 20);
        assert_eq!(a.t.interrupt(), 1);
        assert_eq!(testutil::randomness_enqueued(), before);
        assert!(testutil::printf_count("viornd0: inconsistent descriptor length 20 > 16\n") >= 1);
        assert!(!core.tick.pending(), "no next request");
        assert_eq!(core.st.lock().vq.nfree(), 8, "the slot back all the same");
    }

    #[test]
    fn one_shot_asks_once() {
        let a = attach(8, VIORND_ONESHOT);
        let core = a.core();
        assert_eq!(core.interval, 0);
        let _ = core.tick.del();
        viornd_tick(&MArc::erase(core.clone()));
        let before = testutil::randomness_enqueued();
        answer(&a, [5, 6, 7, 8], 16);
        assert_eq!(a.t.interrupt(), 1);
        assert_eq!(testutil::randomness_enqueued() - before, 4);
        assert!(!core.tick.pending());
    }

    #[test]
    fn each_attach_failure_marks_the_child_failed() {
        // The negotiation.
        let (t, parent, dev, va) = prepare(8, 0);
        t.fail_negotiate.store(true, Ordering::Relaxed);
        VIORND.attach(Some(&parent), &dev, &Aux::Typed(&va));
        assert!(t.failed.load(Ordering::Relaxed));
        let a = Attached { t, dev };
        assert!(
            a.dev
                .softc()
                .downcast_ref::<Viornd>()
                .unwrap()
                .core
                .get()
                .is_none()
        );

        // The buffer.
        let (t, parent, dev, va) = prepare(8, 0);
        t.dma.fail.store(1, Ordering::Relaxed);
        VIORND.attach(Some(&parent), &dev, &Aux::Typed(&va));
        assert!(t.failed.load(Ordering::Relaxed));
        assert!(testutil::printf_count(": Can't alloc dma buffer\n") >= 1);
        assert_eq!(t.dma.frees.load(Ordering::Relaxed), 0);

        // The queue (the device has none): the buffer freed.
        let (t, parent, dev, va) = prepare(0, 0);
        VIORND.attach(Some(&parent), &dev, &Aux::Typed(&va));
        assert!(t.failed.load(Ordering::Relaxed));
        assert!(testutil::printf_count(": Can't alloc virtqueue\n") >= 1);
        assert_eq!(t.dma.frees.load(Ordering::Relaxed), 1, "the buffer back");

        // virtio_attach_finish: the request deleted before the child is failed.
        let (t, parent, dev, va) = prepare(8, 0);
        t.fail_finish.store(true, Ordering::Relaxed);
        VIORND.attach(Some(&parent), &dev, &Aux::Typed(&va));
        assert!(t.failed.load(Ordering::Relaxed));
        let a = Attached { t, dev };
        assert!(!a.core().tick.pending());
    }

    #[test]
    #[should_panic(expected = "already attached to something else")]
    fn a_second_child_panics() {
        let (t, parent, dev, va) = prepare(8, 0);
        assert!(t.claim(&parent, 0));
        VIORND.attach(Some(&parent), &dev, &Aux::Typed(&va));
    }
}
/* </TESTS> */
