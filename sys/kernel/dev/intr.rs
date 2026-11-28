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
 * Copyright (c) 2001-2004 Opsycon AB  (www.opsycon.se / www.opsycon.com)
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS
 * OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
 * WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY
 * DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 */
/* </LICENSES> */

/* <CODE> */
//! An interrupt handler, as every interrupt controller is handed one: the machine-independent
//! half of `<machine/intr.h>`'s establish interface.
//!
//! Upstream: sys/arch/arm64/include/intr.h @ 3ce1f3f79392
//! LZ: sys/arch/arm64/include/intr.rs@e8249449d905
//!
//! The C hands a controller `int (*func)(void *)` and its `void *arg`; a controller that
//! finds `arg == NULL` passes the interrupt's frame instead (`ampintc.c:675-706`,
//! `agintc.c:1113-1116`, amd64's `intr.c:564`): the clock's handler reads the frame that way.
//! [`IntrHandler`] carries the pair and [`IntrHandler::call`] applies that rule, so every
//! controller calls a handler the same way and none repeats the test. The rule is the same on
//! both machines, hence this module beside `kernel::dev::arm64` (amd64 adds its own policy
//! around the call: `intr_suspended`, `ci_handled_intr_level`; N10).
//!
//! ## Redesign
//! - `int (*func)(void *)` and `void *arg`, kept apart by every controller (`ih_func`,
//!   `ih_arg`, `ip_func`, `ip_arg`) -> one [`IntrHandler`] value. Its first arm is
//!   [`IntrHandler::Raw`], a handler of `bsd`'s, with its argument as an exposed address
//!   (scaffolding, `docs/SCAFFOLDING.md`: a `bsd` driver's softc is a raw pointer until the
//!   driver moves). N7.2a2 adds the other two arms of P11: [`IntrHandler::Frame`], a native
//!   handler of the C's `NULL` argument, which reads the frame (its first user is the
//!   generic timer, N7.2b2), and [`IntrHandler::Shared`], a native handler and its softc's
//!   counted handle (first user: `ampintc`'s IPI handler), which the handler names back by
//!   its type (`AnyArc::downcast_ref`). `raw_arg` went: a legacy controller takes the `Raw`
//!   arm's two members itself and refuses the others.
//! - N7.2a3: [`IntrHandler::call`] takes the dispatch's token (`hal::cpu::intr::InIrq`) and
//!   lends it to a [`IntrHandler::Shared`] handler: `agintc`'s IPI handler ends its interrupt
//!   itself before it halts the CPU (`agintc_ipi_halt`), and the end of interrupt asks for the
//!   token (`hal::cpu::icc::eoir1`). A `bsd` handler and a frame handler do not see it.
//! - LZ's `IntrFn` (`sys/arch/arm64/include/intr.rs`, the same as `<machine/intr.h>`'s of
//!   `sys/machine/intr.rs`) is this module's; `bsd`'s header re-exports it.

use core::ffi::c_void;
use core::ptr;

use hal::cpu::clock::ClockFrame;
use hal::cpu::intr::InIrq;
use hal::mem::pool::AnyArc;

/// An interrupt handler, `int (*)(void *)`: nonzero when the interrupt was the device's.
pub type IntrFn = fn(*mut c_void) -> i32;

/// What a controller calls when its interrupt arrives (`ih_func` and `ih_arg`).
#[derive(Clone)]
pub enum IntrHandler {
    /// A handler of `bsd`'s and its `void *` argument as an exposed address (`0` is `NULL`,
    /// which gets the frame's address, the C's rule).
    Raw(IntrFn, usize),
    /// A native handler established with a `NULL` argument: it reads the frame.
    Frame(fn(&ClockFrame) -> i32),
    /// A native handler and its softc's counted handle, named back by its type.
    Shared(fn(&AnyArc, &InIrq) -> i32, AnyArc),
}

impl IntrHandler {
    /// Runs the handler for an interrupt taken with `frame`, inside the dispatch `irq`:
    /// `(*ih_func)(ih_arg ? ih_arg : frame)`, a shared handler given the dispatch's token.
    /// Returns what the handler returned (nonzero: it was the device's).
    #[inline]
    pub fn call(&self, frame: &ClockFrame, irq: &InIrq) -> i32 {
        match self {
            IntrHandler::Raw(func, 0) => func(ptr::from_ref(frame).cast_mut().cast::<c_void>()),
            IntrHandler::Raw(func, arg) => func(ptr::with_exposed_provenance_mut::<c_void>(*arg)),
            IntrHandler::Frame(func) => func(frame),
            IntrHandler::Shared(func, arg) => func(arg, irq),
        }
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The C's rule: a `NULL` argument gets the frame, any other is passed as it is.

    use core::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    static SEEN: AtomicUsize = AtomicUsize::new(0);

    fn record(arg: *mut c_void) -> i32 {
        SEEN.store(arg.addr(), Ordering::Relaxed);
        1
    }

    #[test]
    fn a_null_argument_gets_the_frame_and_another_is_passed_as_it_is() {
        let frame = ClockFrame::default();
        let at = ptr::from_ref(&frame).addr();

        let irq = InIrq::host();
        assert_eq!(IntrHandler::Raw(record, 0).call(&frame, &irq), 1);
        assert_eq!(SEEN.load(Ordering::Relaxed), at);

        let softc = [0u64; 4];
        let arg = ptr::from_ref(&softc).expose_provenance();
        assert_eq!(IntrHandler::Raw(record, arg).call(&frame, &irq), 1);
        assert_eq!(SEEN.load(Ordering::Relaxed), arg);
    }

    fn by_frame(frame: &ClockFrame) -> i32 {
        SEEN.store(ptr::from_ref(frame).addr(), Ordering::Relaxed);
        2
    }

    fn by_softc(arg: &AnyArc, _irq: &InIrq) -> i32 {
        match arg.downcast_ref::<u64>() {
            Some(v) => *v as i32,
            None => -1,
        }
    }

    #[test]
    fn a_native_handler_gets_the_frame_or_its_softc() {
        use hal::mem::malloc::{M_WAITOK, MArc};

        crate::testutil::install();
        let frame = ClockFrame::default();
        let irq = InIrq::host();
        assert_eq!(IntrHandler::Frame(by_frame).call(&frame, &irq), 2);
        let softc = MArc::erase(MArc::new(127, M_WAITOK, 7u64).unwrap());
        let h = IntrHandler::Shared(by_softc, softc);
        assert_eq!(h.clone().call(&frame, &irq), 7);
        let other = MArc::erase(MArc::new(127, M_WAITOK, 7u32).unwrap());
        assert_eq!(IntrHandler::Shared(by_softc, other).call(&frame, &irq), -1);
    }
}
/* </TESTS> */
