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
 * Copyright (c) 2007, 2008 Artur Grabowski <art@openbsd.org>
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
//! The run queues and CPU selection: `kern/kern_sched.c`, as `bsd`'s adapter of
//! `kernel::sched`.
//!
//! Upstream: sys/kern/kern_sched.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_sched.rs@3c62ede68802
//!
//! The run queues, the CPU sets, the CPU choice, stealing, the cost estimate, the idle loop,
//! the dead threads, pegging, the barrier and the secondary CPUs' start and stop are
//! `kernel::sched`'s (N4.5a), kept by CPU number. This module keeps LZ's names for `bsd`'s
//! callers, which name a CPU by its `struct cpu_info`: `sched_init_cpu` (the scheduler's state,
//! then the CPU's clock interrupts and its idle thread, which stay `bsd`'s), the idle thread's
//! creation through `fork1` (`sched_kthreads_create`), and `hw.ncpuonline`, `hw.smt` and
//! `hw.blockcpu`.
//!
//! ## Deviations
//! - `ci->ci_cputype` (`__HAVE_CPU_TOPOLOGY`) is `Cpu::ci_cputype`, whose default is 0 (no
//!   topology known) until the machines port their topology probes (amd64 `identcpu.c`
//!   `cpu_topology`, arm64 `cpu.c` `cpu_identify`): `hw.smt=0` and `hw.blockcpu` then block
//!   no CPU.
//!
//! ## Redesign
//! - LZ's run queues and CPU sets were `hal::list::List`s in each `cpu_info`'s
//!   `schedstate_percpu` and `struct cpuset`s of `cpu_info` pointers (`cpuset_infos`, a table
//!   behind an `unsafe impl Sync`), changed under the legacy `SCHED_LOCK` mutex. They are
//!   `kernel::sched`'s now: the queues `PinList`s under `hal::thread::SCHED_LOCK`'s hold, the
//!   sets bit sets of CPU numbers. A CPU's number is its `CPU_INFO_UNIT` (its `hal::cpu` core's
//!   number, `cpu_attach`), and the way back to its `cpu_info` is [`cpu_info_by_number`], a
//!   table of the CPUs [`sched_init_cpu`] saw.
//! - The idle thread's argument is its CPU's number (`fork1`'s `FORK_IDLE` places the thread
//!   on that CPU), so neither the thread nor the hatching CPU turns an address back into a
//!   `cpu_info`.

use core::ffi::c_void;
use core::ptr;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::once::OnceStatic;
use hal::thread::SchedHold;

use crate::kern::init_main::PROC0;
use crate::kern::kern_clock::statclock;
use crate::kern::kern_clockintr::clockintr_bind;
use crate::kern::kern_fork::fork1;
use crate::kern::kern_kthread::kthread_create_deferred;
use crate::kern::kern_sysctl::{sysctl_int_bounded, sysctl_rdstring, sysctl_string};
use crate::kern::kern_time::itimer_update;
use crate::kern::proc_glue::BsdProcGlue;
use crate::kern::sched_bsd::roundrobin;
use crate::kern::subr_prf::{panic, snprintf};
use crate::kern::subr_prof::profclock;
use crate::machine::Machine;
use crate::machine::cpu::{Cpu, CpuInfo, curcpu};
use crate::sys::errno::Errno;
use crate::sys::proc::{
    _MAXCOMLEN, FORK_IDLE, FORK_NOZOMBIE, FORK_SHAREFILES, FORK_SHAREVM, FORK_SYSTEM, Proc, ProcExt,
};
use crate::sys::sched::{CPUTYP_E, CPUTYP_L, CPUTYP_P, CPUTYP_SMT};

pub use kernel::sched::{
    SCHED_BLOCKCPU, SCHED_CHOOSE, SCHED_COST_PRIORITY, SCHED_COST_RESIDENT, SCHED_COST_RUNNABLE,
    SCHED_NMIGRATIONS, SCHED_NOIDLE, SCHED_NOMIGRATIONS, SCHED_STOLEN, SCHED_WASIDLE,
};

/// The glue every scheduler call here is made for.
type G = BsdProcGlue;

/// The `cpu_info` of each CPU [`sched_init_cpu`] made scheduler state for, by number (LZ's
/// `cpuset_infos`).
static CPU_INFOS: [OnceStatic<&'static CpuInfo>; hal::cpu::MAXCPUS] =
    [const { OnceStatic::new() }; hal::cpu::MAXCPUS];

/// The number the scheduler knows `ci` by (`CPU_INFO_UNIT(ci)`).
#[inline]
pub fn cpu_number_of(ci: &CpuInfo) -> usize {
    Machine::cpu_info_unit(ci) as usize
}

/// The `cpu_info` of CPU `n`: the one [`sched_init_cpu`] saw, else the running CPU's when `n`
/// is its number (the host, whose threads are CPUs of one `cpu_info`).
pub fn cpu_info_by_number(n: usize) -> Option<&'static CpuInfo> {
    match CPU_INFOS.get(n).and_then(|c| c.get().copied()) {
        Some(ci) => Some(ci),
        None => (hal::cpu::cpu_number() == n).then(curcpu),
    }
}

/// `sched_init`: called in `main()` before calling `sched_init_cpu(curcpu())`. Setup the
/// bare minimum to allow things like `setrunqueue()` to work even before the scheduler is
/// actually started.
pub fn sched_init() {
    kernel::sched::sched_init::<G>();
}

/// `sched_init_cpu`: called from `main()` for the boot cpu, then it's the responsibility
/// of the MD code (`cpu_attach`) to call it for all other cpus: the CPU's scheduler state,
/// its four clock interrupts, its SMR queue, and its idle thread through the deferred kthread
/// queue (at once once that queue has run).
pub fn sched_init_cpu(ci: &'static CpuInfo) {
    let n = cpu_number_of(ci);
    if CPU_INFOS[n].set(ci).is_err() {
        panic(format_args!("sched_init_cpu: cpu{n} twice"));
    }
    kernel::sched::sched_init_cpu::<G>(n);

    let spc = Machine::ci_schedstate(ci);
    clockintr_bind(&spc.spc_itimer, ci, itimer_update, ptr::null_mut());
    clockintr_bind(&spc.spc_profclock, ci, profclock, ptr::null_mut());
    clockintr_bind(&spc.spc_roundrobin, ci, roundrobin, ptr::null_mut());
    clockintr_bind(&spc.spc_statclock, ci, statclock, ptr::null_mut());

    kthread_create_deferred(sched_kthreads_create, ptr::without_provenance_mut(n));

    spc.spc_deferred.init();
}

/// `sched_kthreads_create`: creates the CPU's idle thread; its argument is the CPU's number.
pub fn sched_kthreads_create(v: *mut c_void) {
    static NUM: AtomicI32 = AtomicI32::new(0);

    let n = v.addr();
    let Ok(p) = fork1(
        &PROC0,
        FORK_SHAREVM | FORK_SHAREFILES | FORK_NOZOMBIE | FORK_SYSTEM | FORK_IDLE,
        sched_idle,
        v,
    ) else {
        panic(format_args!("fork idle"));
    };
    kernel::sched::set_idleproc::<G>(n, hal::pin::pin_static(p));

    // Name it as specified.
    let num = NUM.fetch_add(1, Ordering::Relaxed);
    let mut comm = [0u8; _MAXCOMLEN];
    let len = snprintf(&mut comm, format_args!("idle{num}")).min(_MAXCOMLEN - 1);
    p.process().set_comm(&comm[..len]);
}

/// `sched_idle`: the idle thread of the CPU whose number is its argument. It runs without the
/// kernel lock and never returns.
pub fn sched_idle(v: *mut c_void) {
    kernel::sched::sched_idle::<G>(v.addr())
}

/// `sched_exit`: the exiting thread `p` (the running one) goes to its CPU's dead threads and
/// the CPU to its idle thread, which hands it to the reaper.
pub fn sched_exit(p: &Proc) -> ! {
    kernel::sched::sched_exit::<G>(hal::pin::pin_static(p.as_static()))
}

/// `sched_toidle`: switches to the idle thread without saving the current context (the
/// current thread is dead, or none ran on this CPU yet).
pub fn sched_toidle() -> ! {
    kernel::sched::sched_toidle::<G>()
}

/// `setrunqueue`: puts `p` on CPU `cpu`'s run queue for `prio` (`None`: `sched_choosecpu`'s
/// choice), under the hold `h`.
pub fn setrunqueue(h: &mut SchedHold, cpu: Option<usize>, p: &Proc, prio: u8) {
    kernel::sched::setrunqueue(h, cpu, hal::pin::pin_static(p.as_static()), prio);
}

/// `remrunqueue`: takes `p` off its run queue.
pub fn remrunqueue(h: &mut SchedHold, p: &Proc) {
    kernel::sched::remrunqueue(h, p);
}

/// `sched_choosecpu_fork`: the number of the CPU a new thread starts on.
pub fn sched_choosecpu_fork(_parent: &Proc, _flags: i32) -> usize {
    kernel::sched::sched_choosecpu_fork::<G>()
}

/// `sched_peg_curproc`: peg the running thread to `ci`.
pub fn sched_peg_curproc(ci: &CpuInfo) {
    kernel::sched::sched_peg_curproc::<G>(cpu_number_of(ci));
}

/// `sched_unpeg_curproc`.
pub fn sched_unpeg_curproc() {
    kernel::sched::sched_unpeg_curproc::<G>();
}

/// `sched_start_secondary_cpus`: lets the running secondary CPUs take threads again.
pub fn sched_start_secondary_cpus() {
    kernel::sched::sched_start_secondary_cpus::<G>();
}

/// `sched_stop_secondary_cpus`: makes sure we stop the secondary CPUs: each hands its
/// threads over and parks in its idle loop (`SPCF_HALTED`); returns once all have.
pub fn sched_stop_secondary_cpus() {
    kernel::sched::sched_stop_secondary_cpus::<G>();
}

/// `sched_barrier`: returns once `ci` (the primary CPU for `None`) has gone through the
/// scheduler, i.e. every interrupt handler it was running has finished.
pub fn sched_barrier(ci: Option<&CpuInfo>) {
    kernel::sched::sched_barrier::<G>(ci.map(cpu_number_of));
}

/// `sysctl_hwncpuonline`.
pub fn sysctl_hwncpuonline() -> u32 {
    kernel::sched::ncpuonline::<G>()
}

/// `cpu_is_online`.
pub fn cpu_is_online(ci: &CpuInfo) -> bool {
    kernel::sched::cpu_is_online::<G>(cpu_number_of(ci))
}

/// `sched_cpuadjust` (`__HAVE_CPU_TOPOLOGY`): blocks the secondary CPUs whose type is in
/// `newblockcpu` and lets the others schedule again.
pub fn sched_cpuadjust(newblockcpu: i32) -> Result<(), Errno> {
    kernel::sched::sched_cpuadjust::<G>(newblockcpu);
    Ok(())
}

/// `sysctl_hwsmt`: emulate hw.smt temporarily.
pub fn sysctl_hwsmt(
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
) -> Result<(), Errno> {
    let newsmt = AtomicI32::new(1);

    if SCHED_BLOCKCPU.load(Ordering::Relaxed) & CPUTYP_SMT != 0 {
        newsmt.store(0, Ordering::Relaxed);
    }
    sysctl_int_bounded(oldp, oldlenp, newp, newlen, &newsmt, 0, 1)?;
    if newp == 0 {
        return Ok(());
    }
    let mut newblockcpu = SCHED_BLOCKCPU.load(Ordering::Relaxed);
    if newsmt.load(Ordering::Relaxed) != 0 {
        newblockcpu &= !CPUTYP_SMT;
    } else {
        newblockcpu |= CPUTYP_SMT;
    }
    sched_cpuadjust(newblockcpu)
}

/// The `hw.blockcpu` letters, in the C's order.
const BLOCKCPU_LETTERS: [(u8, i32); 4] = [
    (b'S', CPUTYP_SMT),
    (b'P', CPUTYP_P),
    (b'E', CPUTYP_E),
    (b'L', CPUTYP_L),
];

/// `sysctl_hwblockcpu`: the CPU types not to schedule on, as letters (`S`, `P`, `E`, `L`).
pub fn sysctl_hwblockcpu(
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
) -> Result<(), Errno> {
    let mut type_ = [0u8; 8];
    let mut n = 0;
    let blockcpu = SCHED_BLOCKCPU.load(Ordering::Relaxed);

    for (letter, typ) in BLOCKCPU_LETTERS {
        if blockcpu & typ != 0 {
            type_[n] = letter;
            n += 1;
        }
    }
    if newp == 0 {
        return sysctl_rdstring(oldp, oldlenp, newp, &type_);
    }

    sysctl_string(oldp, oldlenp, newp, newlen, &mut type_)?;
    let mut newblockcpu = 0;
    for &c in type_.iter().take_while(|&&c| c != 0) {
        let Some(&(_, typ)) = BLOCKCPU_LETTERS.iter().find(|(l, _)| *l == c) else {
            return Err(Errno::EINVAL);
        };
        newblockcpu |= typ;
    }
    sched_cpuadjust(newblockcpu)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_running_cpu_is_found_by_its_number() {
        let n = hal::cpu::cpu_number();
        assert!(cpu_info_by_number(n).is_some_and(|ci| ptr::eq(ci, curcpu())));
    }

    #[test]
    fn another_number_names_no_cpu_before_it_joined() {
        let n = (hal::cpu::cpu_number() + 1) % hal::cpu::MAXCPUS;
        if CPU_INFOS[n].get().is_none() {
            assert!(cpu_info_by_number(n).is_none());
        }
    }

    #[test]
    fn blockcpu_letters_keep_the_c_order() {
        let letters: [u8; 4] = BLOCKCPU_LETTERS.map(|(l, _)| l);
        assert_eq!(&letters, b"SPEL");
    }
}
/* </TESTS> */
