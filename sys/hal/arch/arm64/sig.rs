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
 * Copyright (c) 1990 The Regents of the University of California.
 * All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * William Jolitz and Don Ahn.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgement:
 *      This product includes software developed by the University of
 *      California, Berkeley and its contributors.
 * 4. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 */
/*
 * Copyright (c) 2001 Opsycon AB  (www.opsycon.se)
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
//! arm64's signal frame: `sig_machdep.c`'s `sendsig` and the machine half of
//! `sys_sigreturn`, on the running thread's user context (`trap.rs`'s `UserFrame`), behind
//! `hal::sig`.
//!
//! Upstream: sys/arch/arm64/arm64/sig_machdep.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/sig_machdep.rs@3c62ede68802
//!
//! `sendsig` places the frame (`struct sigframe`: the signal number, the context, the
//! `siginfo`) below the user stack pointer, or below the top of the alternate stack when the
//! action asks for it and the thread is not on it already, aligned to 16 bytes; saves the
//! registers and the mask in the context with its cookie (the context's address XOR the
//! process's cookie); copies the frame out; then points the user context at the handler:
//! `x0` the signal, `x1` the `siginfo` (or 0), `x2` the context, the link register the
//! handler, the stack pointer the frame, the pc the trampoline (`sigcode`), the branch type
//! cleared. `sigreturn` copies the context in, checks its cookie (a mismatch is the kernel's
//! `sigexit(p, SIGILL)`), clears the cookie in user memory so the context cannot be used
//! again, refuses a mode other than EL0 or masked interrupts (`EINVAL`), and restores the
//! registers; the mask goes back to the kernel, which installs it.
//!
//! ## Deviations
//! - The floating-point context is not saved here, as in C ("XXX"): the trampoline
//!   (`sigcode`, `locore.S`) saves and restores the `q` registers on the user stack.
//! - The 4-clause licence (advertising clause) of the first block was accepted by the user
//!   at M2 for this project.
//!
//! ## Redesign
//! - N4.8b: from `bsd` into `hal`. LZ's `sendsig` read `curproc`'s `p_sigstk`, `ps_sigcode`
//!   and `ps_sigcookie` and took the frame through a raw `pcb_tf` dereference (its own
//!   `unsafe`, alive across the copy); here the kernel hands those members in a
//!   `hal::sig::SigDelivery`, and the frame is the running thread's `UserFrame`, borrowed to
//!   read the registers and again to point them at the handler, never across `copyout`
//!   (whose fault may sleep in `uvm_fault`): no `unsafe` of its own.
//! - `sys_sigreturn` is split at the machine's edge: the `PROC_PC` check against
//!   `ps_sigcoderet`, the `sigexit` of a bad cookie and the mask (`& ~sigcantmask`) are the
//!   kernel's `sys_sigreturn` (`kernel::sig`); the copy, the cookie and mode checks and the
//!   registers are [`sigreturn`] here, which answers the mask or why it refused
//!   (`hal::sig::SigreturnError`).
//! - The frame's arithmetic and the checks are functions of the frame and the context
//!   alone ([`signal_frame`], [`enter_handler`], [`check_cookie`], [`check_mode`],
//!   [`restore`]), so the host tests run them on arm64's layouts.

use core::mem::offset_of;

use super::include::_types::_STACKALIGNBYTES;
use super::include::armreg::{PSR_BTYPE, PSR_F, PSR_I, PSR_M_EL0t, PSR_M_MASK};
use super::include::frame::{Sigframe, Trapframe};
use super::include::param::PAGE_MASK;
use super::include::signal::Sigcontext;
use crate::sig::{SigDelivery, SigreturnError};
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use crate::sys::errno::Errno;
use crate::sys::signal::SS_DISABLE;
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use crate::sys::signal::Sigset;
use crate::sys::types::Register;
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use crate::user::UserPtr;

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
use super::trap::UserFrame;

/// `STACKALIGN(p)`: a stack pointer rounded down to 16 bytes.
const fn stackalign(p: usize) -> usize {
    p & !_STACKALIGNBYTES
}

/// `sendsig`'s frame for the user context `tf`: where it goes (`fp`) and what it holds.
fn signal_frame(tf: &Trapframe, d: &SigDelivery<'_>) -> (usize, Sigframe) {
    let ss = &d.sigstk;
    let sp = tf.tf_sp as usize;
    // sigonstack(tf->tf_sp)
    let on_sigstack = ss.ss_flags & SS_DISABLE == 0 && sp.wrapping_sub(ss.ss_sp) < ss.ss_size;

    // Allocate space for the signal handler context.
    let mut fp = if ss.ss_flags & SS_DISABLE == 0 && !on_sigstack && d.onstack {
        ss.ss_sp.wrapping_add(ss.ss_size) & !PAGE_MASK
    } else {
        sp
    };

    // make room on the stack
    fp = fp.wrapping_sub(size_of::<Sigframe>());

    // make the stack aligned
    fp = stackalign(fp);

    // Build stack frame for signal trampoline.
    let mut frame = Sigframe::zeroed();
    frame.sf_signum = d.sig;

    // Save register context.
    for (sc, &x) in frame.sf_sc.sc_x.iter_mut().zip(tf.tf_x.iter()) {
        *sc = x as u64;
    }
    frame.sf_sc.sc_sp = tf.tf_sp as u64;
    frame.sf_sc.sc_lr = tf.tf_lr as u64;
    frame.sf_sc.sc_elr = tf.tf_elr as u64;
    frame.sf_sc.sc_spsr = tf.tf_spsr as u64;

    // Save signal mask.
    frame.sf_sc.sc_mask = d.mask as i32;

    // XXX Save floating point context

    if let Some(si) = d.info {
        frame.sf_si = *si;
    }

    let scp = fp.wrapping_add(offset_of!(Sigframe, sf_sc));
    frame.sf_sc.sc_cookie = (scp as i64) ^ (d.cookie as i64);
    (fp, frame)
}

/// The user context `tf` pointed at the handler, once its frame is at `fp`. We invoke the
/// handler directly, only returning via the trampoline.
fn enter_handler(tf: &mut Trapframe, d: &SigDelivery<'_>, fp: usize) {
    let sip = match d.info {
        Some(_) => fp.wrapping_add(offset_of!(Sigframe, sf_si)),
        None => 0,
    };
    tf.tf_x[0] = d.sig as Register;
    tf.tf_x[1] = sip as Register;
    tf.tf_x[2] = fp.wrapping_add(offset_of!(Sigframe, sf_sc)) as Register;
    tf.tf_lr = d.catcher as Register;
    tf.tf_sp = fp as Register;

    tf.tf_elr = d.sigcode as Register;
    tf.tf_spsr &= !(PSR_BTYPE as Register);
}

/// The context copied in from `scp` carries the cookie `sendsig` wrote for that address.
fn check_cookie(ksc: &Sigcontext, scp: usize, cookie: u64) -> Result<(), SigreturnError> {
    if ksc.sc_cookie != (scp as i64) ^ (cookie as i64) {
        return Err(SigreturnError::Cookie);
    }
    Ok(())
}

/// Make sure the processor mode has not been tampered with and interrupts have not been
/// disabled.
fn check_mode(ksc: &Sigcontext) -> Result<(), SigreturnError> {
    if ksc.sc_spsr & PSR_M_MASK != PSR_M_EL0t || ksc.sc_spsr & (PSR_I | PSR_F) != 0 {
        return Err(SigreturnError::Mode);
    }
    Ok(())
}

/// Restore register context.
fn restore(tf: &mut Trapframe, ksc: &Sigcontext) {
    for (x, &sc) in tf.tf_x.iter_mut().zip(ksc.sc_x.iter()) {
        *x = sc as Register;
    }
    tf.tf_sp = ksc.sc_sp as Register;
    tf.tf_lr = ksc.sc_lr as Register;
    tf.tf_elr = ksc.sc_elr as Register;
    tf.tf_spsr = ksc.sc_spsr as Register;
}

/// `sendsig`: send an interrupt to the running thread. The stack is set up to allow
/// `sigcode` to call the handler, followed by a system call to `sigreturn`; `EFAULT` when
/// the frame cannot be written (or no user context runs).
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub(crate) fn sendsig(d: &SigDelivery<'_>) -> Result<(), Errno> {
    let Some(frame) = UserFrame::current() else {
        return Err(Errno::EFAULT);
    };
    let (fp, sf) = frame.with(|tf| signal_frame(tf, d));
    // The copy is outside the borrow: its fault may take the page fault path.
    UserPtr::<Sigframe>::new(fp).copyout(&sf)?;
    frame.with(|tf| enter_handler(tf, d, fp));
    Ok(())
}

/// `sys_sigreturn`'s machine half: reset the user context from the one `sendsig` left at
/// `scp`, after checking carefully that the user has not modified the mode to gain improper
/// privileges or to cause a machine fault. The mask the context carries is the answer.
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub(crate) fn sigreturn(scp: usize, cookie: u64) -> Result<Sigset, SigreturnError> {
    let mut ksc = UserPtr::<Sigcontext>::new(scp)
        .copyin()
        .map_err(|_| SigreturnError::Fault)?;

    check_cookie(&ksc, scp, cookie)?;

    // Prevent reuse of the sigcontext cookie
    ksc.sc_cookie = 0;
    let _ = UserPtr::<i64>::new(scp.wrapping_add(offset_of!(Sigcontext, sc_cookie)))
        .copyout(&ksc.sc_cookie);

    check_mode(&ksc)?;

    // XXX Restore floating point context

    let Some(frame) = UserFrame::current() else {
        return Err(SigreturnError::Fault);
    };
    frame.with(|tf| restore(tf, &ksc));

    Ok(ksc.sc_mask as Sigset)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The frame's arithmetic and the context checks on arm64's layouts (the host compiles
    // this file with arm64's headers, `arch/host/mod.rs`).
    use super::*;
    use crate::sys::siginfo::Siginfo;
    use crate::sys::signal::{SIGSEGV, SIGUSR1, Sigaltstack};

    const SIGCODE: usize = 0x7f7f_f000;
    const COOKIE: u64 = 0x5a5a_1234_dead_beef;
    const CATCHER: usize = 0x2000_0abc;

    fn user_frame(sp: usize) -> Trapframe {
        let mut tf = Trapframe::new();
        for (i, x) in tf.tf_x.iter_mut().enumerate() {
            *x = 0x1000 + i as Register;
        }
        tf.tf_sp = sp as Register;
        tf.tf_lr = 0x4000_0010;
        tf.tf_elr = 0x4000_0020;
        tf.tf_spsr = (PSR_BTYPE | 0x2000_0000) as Register; // BTYPE and a condition flag
        tf
    }

    fn delivery(info: Option<&Siginfo>, onstack: bool, sigstk: Sigaltstack) -> SigDelivery<'_> {
        SigDelivery {
            catcher: CATCHER,
            sig: SIGUSR1,
            mask: 0x0000_8005,
            info,
            onstack,
            sigstk,
            sigcode: SIGCODE,
            cookie: COOKIE,
        }
    }

    fn no_altstack() -> Sigaltstack {
        Sigaltstack {
            ss_sp: 0,
            ss_size: 0,
            ss_flags: SS_DISABLE,
            _pad: 0,
        }
    }

    /// The frame goes below the stack pointer, 16-byte aligned, and saves every register,
    /// the mask and a cookie bound to the context's address.
    #[test]
    fn the_frame_saves_the_context_below_the_stack() {
        let sp = 0x7f00_0f38; // not 16-byte aligned
        let tf = user_frame(sp);
        let d = delivery(None, false, no_altstack());
        let (fp, sf) = signal_frame(&tf, &d);
        assert_eq!(fp, (sp - size_of::<Sigframe>()) & !15);
        assert_eq!(fp % 16, 0);
        assert_eq!(sf.sf_signum, SIGUSR1);
        assert_eq!(sf._pad, 0);
        for i in 0..30 {
            assert_eq!(sf.sf_sc.sc_x[i], 0x1000 + i as u64);
        }
        assert_eq!(sf.sf_sc.sc_sp, sp as u64);
        assert_eq!(sf.sf_sc.sc_lr, 0x4000_0010);
        assert_eq!(sf.sf_sc.sc_elr, 0x4000_0020);
        assert_eq!(sf.sf_sc.sc_spsr, PSR_BTYPE | 0x2000_0000);
        assert_eq!(sf.sf_sc.sc_mask, 0x8005);
        let scp = fp + offset_of!(Sigframe, sf_sc);
        assert_eq!(sf.sf_sc.sc_cookie, (scp as i64) ^ (COOKIE as i64));
        assert_eq!(sf.sf_si, Siginfo::zeroed(), "no siginfo without SA_SIGINFO");
    }

    /// `SA_ONSTACK` with an enabled alternate stack the thread is not on: the frame goes
    /// below the stack's top, rounded down to a page; on it already, or with the stack
    /// disabled, or without `SA_ONSTACK`, below the stack pointer.
    #[test]
    fn the_alternate_stack_is_used_when_asked_and_not_on_it() {
        let sp = 0x7f00_0f40;
        let alt = Sigaltstack {
            ss_sp: 0x5000_0000,
            ss_size: 0x2_0800, // its top is not page-aligned
            ss_flags: 0,
            _pad: 0,
        };
        let top = (0x5000_0000 + 0x2_0800) & !PAGE_MASK;
        let below = |from: usize| (from - size_of::<Sigframe>()) & !15;

        let (fp, _) = signal_frame(&user_frame(sp), &delivery(None, true, alt));
        assert_eq!(fp, below(top));

        let (fp, _) = signal_frame(&user_frame(sp), &delivery(None, false, alt));
        assert_eq!(fp, below(sp), "not asked");

        let on_it = 0x5000_1000;
        let (fp, _) = signal_frame(&user_frame(on_it), &delivery(None, true, alt));
        assert_eq!(fp, below(on_it), "on the alternate stack already");

        let disabled = Sigaltstack {
            ss_flags: SS_DISABLE,
            ..alt
        };
        let (fp, _) = signal_frame(&user_frame(sp), &delivery(None, true, disabled));
        assert_eq!(fp, below(sp), "disabled");
    }

    /// The handler's entry: the signal, the `siginfo` and the context in `x0`..`x2`, the
    /// handler in the link register, the frame as the stack, the trampoline as the pc, the
    /// branch type cleared; the other registers as they were.
    #[test]
    fn the_handler_is_entered_with_the_frame_and_the_trampoline() {
        let mut si = Siginfo::zeroed();
        si.si_signo = SIGSEGV;
        si.si_code = 2;
        let sp = 0x7f00_0f40;
        let mut tf = user_frame(sp);
        let d = delivery(Some(&si), false, no_altstack());
        let (fp, sf) = signal_frame(&tf, &d);
        assert_eq!(sf.sf_si, si, "SA_SIGINFO copies the siginfo");
        enter_handler(&mut tf, &d, fp);
        assert_eq!(tf.tf_x[0], SIGUSR1 as Register);
        assert_eq!(tf.tf_x[1], (fp + offset_of!(Sigframe, sf_si)) as Register);
        assert_eq!(tf.tf_x[2], (fp + offset_of!(Sigframe, sf_sc)) as Register);
        assert_eq!(tf.tf_x[3], 0x1003, "the other registers are untouched");
        assert_eq!(tf.tf_lr, CATCHER as Register);
        assert_eq!(tf.tf_sp, fp as Register);
        assert_eq!(tf.tf_elr, SIGCODE as Register);
        assert_eq!(tf.tf_spsr, 0x2000_0000, "BTYPE cleared, the flags kept");

        let mut tf = user_frame(sp);
        enter_handler(&mut tf, &delivery(None, false, no_altstack()), fp);
        assert_eq!(tf.tf_x[1], 0, "no siginfo: x1 is 0");
    }

    /// What `sendsig` wrote passes `sigreturn`'s checks and restores the registers; a
    /// context at another address, a changed cookie, a mode other than EL0t or masked
    /// interrupts are refused.
    #[test]
    fn sigreturn_checks_the_cookie_and_the_mode_then_restores() {
        let sp = 0x7f00_0f40;
        let tf0 = user_frame(sp);
        let d = delivery(None, false, no_altstack());
        let (fp, sf) = signal_frame(&tf0, &d);
        let scp = fp + offset_of!(Sigframe, sf_sc);
        let mut ksc = sf.sf_sc;
        ksc.sc_spsr = PSR_M_EL0t; // what the handler's frame returns with

        assert_eq!(check_cookie(&ksc, scp, COOKIE), Ok(()));
        assert_eq!(
            check_cookie(&ksc, scp + 16, COOKIE),
            Err(SigreturnError::Cookie),
            "the context moved"
        );
        assert_eq!(
            check_cookie(&ksc, scp, COOKIE ^ 1),
            Err(SigreturnError::Cookie),
            "another process's cookie"
        );
        let mut zeroed = ksc;
        zeroed.sc_cookie = 0;
        assert_eq!(
            check_cookie(&zeroed, scp, COOKIE),
            Err(SigreturnError::Cookie),
            "a context already returned through (its cookie cleared)"
        );

        assert_eq!(check_mode(&ksc), Ok(()));
        for bad in [
            1,     // EL1t
            5,     // EL1h
            PSR_I, // IRQs masked
            PSR_F, // FIQs masked
            PSR_M_MASK,
        ] {
            let mut k = ksc;
            k.sc_spsr = bad;
            assert_eq!(check_mode(&k), Err(SigreturnError::Mode), "spsr {bad:#x}");
        }

        let mut tf = user_frame(0x1234_5670);
        tf.tf_x = [0; 30];
        restore(&mut tf, &ksc);
        assert_eq!(&tf.tf_x[..], &tf0.tf_x[..]);
        assert_eq!(tf.tf_sp, tf0.tf_sp);
        assert_eq!(tf.tf_lr, tf0.tf_lr);
        assert_eq!(tf.tf_elr, tf0.tf_elr);
        assert_eq!(tf.tf_spsr, PSR_M_EL0t as Register);
    }
}
/* </TESTS> */
