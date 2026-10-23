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
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
 * Copyright (c) 1982, 1986, 1989, 1993
 *      The Regents of the University of California.
 *
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the University nor the names of its contributors
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
 *      @(#)vm_meter.c  8.4 (Berkeley) 1/4/94
 * from: Id: uvm_meter.c,v 1.1.2.1 1997/08/14 19:10:35 chuck Exp
 */
/* </LICENSES> */

/* <CODE> */
//! UVM's statistics and its sysctl tree (`CTL_VM`): `uvm/uvm_meter.c`.
//!
//! Upstream: sys/uvm/uvm_meter.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_meter.rs@3c62ede68802
//!
//! `vm.loadavg`, `vm.vmmeter`, `vm.uvmexp`, `vm.nkmempages`, `vm.psstrings`,
//! `vm.anonmin`/`vtextmin`/`vnodemin`, `vm.maxslp`, `vm.uspace` and `vm.malloc_conf` are
//! served; [`uvmexp_print`] is the `ddb` printer.
//!
//! ## Deviations
//! - `UVM_SWAP_ENCRYPT` is not configured (as in the rest of the tree), so `vm.swapencrypt`
//!   is `EOPNOTSUPP` as the C's `#else` branch (LZ's).
//! - `struct loadavg` and `struct vmtotal` have padding in C; they are copied out as the C's
//!   byte image, built member by member with the holes zeroed (LZ's).
//! - `maxslp` keeps its lowercase C name: `MAXSLP` is the constant it starts from (LZ's).
//!
//! ## Redesign
//! - N4.6b: the sysctl helpers are `kernel::sysctl`'s (`rdint`, `int`, `rdstruct`,
//!   `string`), `averunnable` `kernel::sched`'s, the thread list the glue's tables
//!   (`allproc`), `nkmempages` the `Malloc` bridge's member; the request's user addresses
//!   travel as a [`SysctlReq`] (`kernel::sysctl`'s since N4.7b, re-exported here); the
//!   dispatch is `kernel::sysctl::sys_sysctl`'s (`CTL_VM`, vslocked). LZ's `VmSysctl` bridge
//!   (N3.5 to N4.6b) is gone. The dispatch of this tree, every value and every check (the
//!   three minimum percentages' sum) are here; `vm.uvmexp` is copied out as the bytes of
//!   `struct uvmexp` (`UvmexpCopy::to_ne_bytes`).
//! - `uvm_total` classifies the threads of `allproc` ([`ProcState`]: `p_stat` and whether the
//!   thread is its CPU's idle thread), as `uvm_meter.c` does.
//! - `malloc_conf` is a `Mutexed<[u8; 16]>` (a lock the C does not take: `sys_sysctl`'s
//!   `sysctl_lock` serialises every `CTL_VM` call already). `sysctl_string` copies in and out
//!   of user memory, which may sleep, so it works on a copy taken under the mutex and stores
//!   it back under it.
//! - `uvmexp_print` prints through a `&mut dyn FnMut(fmt::Arguments)` (`ddb`'s `pr`).

use core::fmt;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::intr::IPL_NONE;
use hal::mutex::Mutexed;
use hal::sys::errno::Errno;
use hal::sys::vmmeter::Vmtotal;
use hal::vm::uvmexp::{
    UVMEXP, UvmExpCounters, UvmexpCopy, VM_ANONMIN, VM_LOADAVG, VM_MALLOC_CONF, VM_MAXSLP,
    VM_METER, VM_NKMEMPAGES, VM_PSSTRINGS, VM_SWAPENCRYPT, VM_USPACE, VM_UVMEXP, VM_VNODEMIN,
    VM_VTEXTMIN, counters_read,
};

use super::init::machine;
use super::proc::vmproc;
use crate::proc::ProcGlue;
pub use crate::sysctl::SysctlReq;
use crate::sysctl::{int, rdint, rdstruct, string};

/// `MAXSLP`: the time for a process to be blocked before being very swappable. This is a
/// number of seconds which the system takes as being a non-trivial amount of real time. You
/// probably shouldn't change this; it is used in subtle ways (fractions and multiples of it
/// are, that is, like half of a "long time", almost a long time, etc.) It is related to human
/// patience and other factors which don't really change over time.
pub const MAXSLP: i32 = 20;

/// `SIDL`: thread being created by fork (`<sys/proc.h>`).
pub const SIDL: i32 = 1;
/// `SRUN`: currently runnable.
pub const SRUN: i32 = 2;
/// `SSLEEP`: sleeping on an address.
pub const SSLEEP: i32 = 3;
/// `SSTOP`: debugging or suspension.
pub const SSTOP: i32 = 4;
/// `SONPROC`: thread is currently on a CPU.
pub const SONPROC: i32 = 7;

/// What `uvm_total` reads of one thread of `allproc`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcState {
    /// `p_stat`.
    pub stat: i32,
    /// `p == p->p_cpu->ci_schedstate.spc_idleproc`.
    pub idle: bool,
}

/// `maxslp`: patchable.
#[allow(non_upper_case_globals)] // the C's name; `MAXSLP` is the constant
pub static maxslp: AtomicI32 = AtomicI32::new(MAXSLP);

/// `malloc_conf`: the configuration string for userland `malloc(3)`.
#[allow(non_upper_case_globals)] // the C's name, an array beside `maxslp`
pub static malloc_conf: Mutexed<[u8; 16]> = Mutexed::new(IPL_NONE, [0; 16]);

/// `uvm_sysctl`: the sysctl hook into UVM (`CTL_VM`), for the current process (the
/// threads of `G`'s tables for `vm.vmmeter`).
pub fn uvm_sysctl<G: ProcGlue>(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    let Some(&top) = name.first() else {
        return Err(Errno::ENOTDIR);
    };
    match top {
        // UVM_SWAP_ENCRYPT is not configured.
        VM_SWAPENCRYPT => return Err(Errno::EOPNOTSUPP),
        _ => {
            // all sysctl names at this level are terminal
            if name.len() != 1 {
                return Err(Errno::ENOTDIR); // overloaded
            }
        }
    }

    match top {
        VM_LOADAVG => rdstruct(
            req,
            &loadavg_bytes((crate::sched::averunnable(), i64::from(crate::sched::FSCALE))),
        ),

        VM_METER => rdstruct(req, &vmtotal_bytes(&uvm_total::<G>())),

        VM_UVMEXP => rdstruct(req, &uvmexp_read().to_ne_bytes()),

        VM_NKMEMPAGES => rdint(req, hal::malloc::nkmempages()),

        VM_PSSTRINGS => rdstruct(req, &(vmproc().ps_strings() as u64).to_ne_bytes()),

        VM_ANONMIN => uvm_minpct(
            req,
            &UVMEXP.anonminpct,
            &UVMEXP.anonmin,
            [&UVMEXP.vtextminpct, &UVMEXP.vnodeminpct],
        ),

        VM_VTEXTMIN => uvm_minpct(
            req,
            &UVMEXP.vtextminpct,
            &UVMEXP.vtextmin,
            [&UVMEXP.anonminpct, &UVMEXP.vnodeminpct],
        ),

        VM_VNODEMIN => uvm_minpct(
            req,
            &UVMEXP.vnodeminpct,
            &UVMEXP.vnodemin,
            [&UVMEXP.anonminpct, &UVMEXP.vtextminpct],
        ),

        VM_MAXSLP => rdint(req, maxslp.load(Ordering::Relaxed)),

        VM_USPACE => rdint(req, machine().uspace as i32),

        VM_MALLOC_CONF => {
            let mut conf = *malloc_conf.lock();
            let rv = string(req, &mut conf);
            if req.newp != 0 {
                *malloc_conf.lock() = conf;
            }
            rv
        }

        _ => Err(Errno::EOPNOTSUPP),
    }
}

/// The `VM_ANONMIN`, `VM_VTEXTMIN` and `VM_VNODEMIN` cases of `uvm_sysctl`: one of the three
/// minimum percentages, which together may not pass 95, and its threshold in 1/256ths.
fn uvm_minpct(
    req: &mut SysctlReq<'_>,
    pct: &AtomicI32,
    min: &AtomicI32,
    others: [&AtomicI32; 2],
) -> Result<(), Errno> {
    let t = AtomicI32::new(pct.load(Ordering::Relaxed));
    int(req, &t)?;
    minpct_set(t.into_inner(), pct, min, others)
}

/// The check and the update of [`uvm_minpct`] once `sysctl_int` gave `t`.
fn minpct_set(
    t: i32,
    pct: &AtomicI32,
    min: &AtomicI32,
    others: [&AtomicI32; 2],
) -> Result<(), Errno> {
    // The C's `int` sum, which wraps (a huge `t` from userland must not panic the kernel).
    let sum = others
        .iter()
        .fold(t, |acc, o| acc.wrapping_add(o.load(Ordering::Relaxed)));
    if sum > 95 || t < 0 {
        return Err(Errno::EINVAL);
    }
    pct.store(t, Ordering::Relaxed);
    min.store(t.wrapping_mul(256) / 100, Ordering::Relaxed);
    Ok(())
}

/// `struct loadavg` as the C lays it out: `fixpt_t ldavg[3]`, four bytes of padding, `long
/// fscale`.
fn loadavg_bytes((ldavg, fscale): ([u32; 3], i64)) -> [u8; 24] {
    let mut b = [0u8; 24];
    for (i, v) in ldavg.iter().enumerate() {
        b[4 * i..4 * i + 4].copy_from_slice(&v.to_ne_bytes());
    }
    b[16..].copy_from_slice(&fscale.to_ne_bytes());
    b
}

/// `struct vmtotal` as the C lays it out: five `u_int16_t`, two bytes of padding, nine
/// `u_int32_t`.
fn vmtotal_bytes(t: &Vmtotal) -> [u8; 48] {
    let mut b = [0u8; 48];
    let shorts = [t.t_rq, t.t_dw, t.t_pw, t.t_sl, t.t_sw];
    for (i, v) in shorts.iter().enumerate() {
        b[2 * i..2 * i + 2].copy_from_slice(&v.to_ne_bytes());
    }
    let ints = [
        t.t_vm, t.t_avm, t.t_rm, t.t_arm, t.t_vmshr, t.t_avmshr, t.t_rmshr, t.t_armshr, t.t_free,
    ];
    for (i, v) in ints.iter().enumerate() {
        b[12 + 4 * i..16 + 4 * i].copy_from_slice(&v.to_ne_bytes());
    }
    b
}

/// `uvm_total`: calculate the current state of the system (the threads of `G`'s
/// `allproc`, under the kernel lock's reader: `sysctl(2)` vslocked `CTL_VM`).
pub fn uvm_total<G: ProcGlue>() -> Vmtotal {
    let mut total = Vmtotal::default();

    // calculate process statistics
    let r = hal::klock::reader();
    for p in G::tables().allproc.iter(&r) {
        let state = ProcState {
            stat: i32::from(p.stat()),
            idle: crate::sched::is_idle_thread(p),
        };
        total_count(&mut total, state);
    }
    drop(r);

    // Calculate object memory usage statistics.
    let free = UVMEXP.free.load(Ordering::Relaxed);
    let npages = UVMEXP.npages.load(Ordering::Relaxed);
    let swpginuse = UVMEXP.swpginuse.load(Ordering::Relaxed);
    let active = UVMEXP.active.load(Ordering::Relaxed);
    total.t_free = free as u32;
    total.t_vm = (npages - free + swpginuse) as u32;
    total.t_avm = (active + swpginuse) as u32; // XXX
    total.t_rm = (npages - free) as u32;
    total.t_arm = active as u32;
    total.t_vmshr = 0; // XXX
    total.t_avmshr = 0; // XXX
    total.t_rmshr = 0; // XXX
    total.t_armshr = 0; // XXX
    total
}

/// One thread of `uvm_total`'s process statistics.
fn total_count(total: &mut Vmtotal, p: ProcState) {
    match p.stat {
        SSLEEP | SSTOP => total.t_sl += 1,
        SRUN | SONPROC if p.idle => {}
        SRUN | SONPROC | SIDL => total.t_rq += 1,
        _ => {}
    }
    // note active objects: #if 0 in C (XXXCDC: BOGUS! rethink this).
}

/// `uvmexp_read`: `uvmexp` with the per-CPU counters folded in.
pub fn uvmexp_read() -> UvmexpCopy {
    let mut uexp = UVMEXP.snapshot();

    let c = |which: UvmExpCounters| counters_read(which) as i32;

    // stat counters
    uexp.faults = c(UvmExpCounters::Faults);
    uexp.pageins = c(UvmExpCounters::Pageins);

    // fault subcounters
    uexp.fltnoram = c(UvmExpCounters::FltNoram);
    uexp.fltnoanon = c(UvmExpCounters::FltNoanon);
    uexp.fltnoamap = c(UvmExpCounters::FltNoamap);
    uexp.fltpgwait = c(UvmExpCounters::FltPgwait);
    uexp.fltpgrele = c(UvmExpCounters::FltPgrele);
    uexp.fltrelck = c(UvmExpCounters::FltRelck);
    uexp.fltnorelck = c(UvmExpCounters::FltNorelck);
    uexp.fltanget = c(UvmExpCounters::FltAnget);
    uexp.fltanretry = c(UvmExpCounters::FltAnretry);
    uexp.fltamcopy = c(UvmExpCounters::FltAmcopy);
    uexp.fltnamap = c(UvmExpCounters::FltNamap);
    uexp.fltnomap = c(UvmExpCounters::FltNomap);
    uexp.fltlget = c(UvmExpCounters::FltLget);
    uexp.fltget = c(UvmExpCounters::FltGet);
    uexp.flt_anon = c(UvmExpCounters::FltAnon);
    uexp.flt_acow = c(UvmExpCounters::FltAcow);
    uexp.flt_obj = c(UvmExpCounters::FltObj);
    uexp.flt_prcopy = c(UvmExpCounters::FltPrcopy);
    uexp.flt_przero = c(UvmExpCounters::FltPrzero);
    uexp.fltup = c(UvmExpCounters::FltUp);
    uexp.fltnoup = c(UvmExpCounters::FltNoup);
    uexp
}

/// `uvmexp_print`: `ddb` hook to print interesting uvm counters.
pub fn uvmexp_print(pr: &mut dyn FnMut(fmt::Arguments<'_>)) {
    let uexp = uvmexp_read();

    pr(format_args!("Current UVM status:\n"));
    pr(format_args!(
        "  pagesize={} (0x{:x}), pagemask=0x{:x}, pageshift={}\n",
        uexp.pagesize, uexp.pagesize, uexp.pagemask, uexp.pageshift
    ));
    pr(format_args!(
        "  {} VM pages: {} active, {} inactive, {} wired, {} free ({} zero)\n",
        uexp.npages, uexp.active, uexp.inactive, uexp.wired, uexp.free, uexp.zeropages
    ));
    pr(format_args!(
        "  freemin={}, free-target={}, inactive-target={}, wired-max={}\n",
        uexp.freemin, uexp.freetarg, uexp.inactarg, uexp.wiredmax
    ));
    pr(format_args!(
        "  faults={}, traps={}, intrs={}, ctxswitch={} fpuswitch={}\n",
        uexp.faults, uexp.traps, uexp.intrs, uexp.swtch, uexp.fpswtch
    ));
    pr(format_args!(
        "  softint={}, syscalls={}, kmapent={}\n",
        uexp.softs, uexp.syscalls, uexp.kmapent
    ));

    pr(format_args!("  fault counts:\n"));
    pr(format_args!(
        "    noram={}, noanon={}, noamap={}, pgwait={}, pgrele={}\n",
        uexp.fltnoram, uexp.fltnoanon, uexp.fltnoamap, uexp.fltpgwait, uexp.fltpgrele
    ));
    pr(format_args!(
        "    relocks={}({}), upgrades={}({}) anget(retries)={}({}), amapcopy={}\n",
        uexp.fltrelck,
        uexp.fltnorelck,
        uexp.fltup,
        uexp.fltnoup,
        uexp.fltanget,
        uexp.fltanretry,
        uexp.fltamcopy
    ));
    pr(format_args!(
        "    neighbor anon/obj pg={}/{}, gets(lock/unlock)={}/{}\n",
        uexp.fltnamap, uexp.fltnomap, uexp.fltlget, uexp.fltget
    ));
    pr(format_args!(
        "    cases: anon={}, anoncow={}, obj={}, prcopy={}, przero={}\n",
        uexp.flt_anon, uexp.flt_acow, uexp.flt_obj, uexp.flt_prcopy, uexp.flt_przero
    ));

    pr(format_args!("  daemon and swap counts:\n"));
    pr(format_args!(
        "    woke={}, revs={}, scans={}, obscans={}, anscans={}\n",
        uexp.pdwoke, uexp.pdrevs, uexp.pdscans, uexp.pdobscan, uexp.pdanscan
    ));
    pr(format_args!(
        "    busy={}, freed={}, reactivate={}, deactivate={}\n",
        uexp.pdbusy, uexp.pdfreed, uexp.pdreact, uexp.pddeact
    ));
    pr(format_args!(
        "    pageouts={}, pending={}, nswget={}\n",
        uexp.pdpageouts, uexp.pdpending, uexp.nswget
    ));
    pr(format_args!(
        "    nswapdev={}, swpskip={}\n",
        uexp.nswapdev, uexp.swpskip
    ));
    pr(format_args!(
        "    swpages={}, swpginuse={}, swpgonly={} paging={}\n",
        uexp.swpages, uexp.swpginuse, uexp.swpgonly, uexp.paging
    ));

    pr(format_args!("  kernel pointers:\n"));
    pr(format_args!(
        "    objs(kern)={:p}\n",
        super::aobj::kernel_object().map_or(core::ptr::null(), |o| o.as_ptr())
    ));
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vmtotal_has_the_c_layout() {
        let t = Vmtotal {
            t_rq: 1,
            t_sw: 5,
            t_vm: 6,
            t_free: 14,
            ..Vmtotal::default()
        };
        let b = vmtotal_bytes(&t);
        assert_eq!(u16::from_ne_bytes([b[0], b[1]]), 1);
        assert_eq!(u16::from_ne_bytes([b[8], b[9]]), 5);
        assert_eq!(&b[10..12], &[0, 0]);
        assert_eq!(u32::from_ne_bytes([b[12], b[13], b[14], b[15]]), 6);
        assert_eq!(u32::from_ne_bytes([b[44], b[45], b[46], b[47]]), 14);
    }

    #[test]
    fn uvmexp_read_folds_in_the_counters() {
        let before = uvmexp_read().fltanget;
        hal::vm::uvmexp::counters_inc(UvmExpCounters::FltAnget);
        assert!(uvmexp_read().fltanget > before);
    }

    #[test]
    fn loadavg_has_the_c_layout() {
        let b = loadavg_bytes(([1, 2, 3], 2048));
        assert_eq!(u32::from_ne_bytes([b[8], b[9], b[10], b[11]]), 3);
        assert_eq!(&b[12..16], &[0; 4], "the padding");
        assert_eq!(i64::from_ne_bytes(b[16..24].try_into().unwrap()), 2048);
    }

    /// `uvm_total`'s classification of `allproc`, case by case.
    #[test]
    fn the_run_queue_and_the_sleepers_are_counted_as_the_c_counts_them() {
        let mut t = Vmtotal::default();
        for (stat, idle) in [
            (0, false),
            (SIDL, false),
            (SRUN, false),
            (SONPROC, false),
            (SONPROC, true),
            (SRUN, true),
            (SSLEEP, false),
            (SSTOP, false),
            (5, false),
            (6, false),
        ] {
            total_count(&mut t, ProcState { stat, idle });
        }
        assert_eq!(
            t.t_rq, 3,
            "SIDL, SRUN and SONPROC, but not the idle threads"
        );
        assert_eq!(t.t_sl, 2, "SSLEEP and SSTOP");
    }

    #[test]
    fn the_minimum_percentages_stay_under_95() {
        let (pct, min) = (AtomicI32::new(10), AtomicI32::new(0));
        let (a, b) = (AtomicI32::new(40), AtomicI32::new(40));
        assert_eq!(minpct_set(16, &pct, &min, [&a, &b]), Err(Errno::EINVAL));
        assert_eq!(minpct_set(-1, &pct, &min, [&a, &b]), Err(Errno::EINVAL));
        assert_eq!(pct.load(Ordering::Relaxed), 10, "unchanged on EINVAL");
        // i32::MAX + 80 wraps negative in the C's int sum: accepted, as the C accepts it.
        assert_eq!(minpct_set(i32::MAX, &pct, &min, [&a, &b]), Ok(()));
        assert_eq!(pct.load(Ordering::Relaxed), i32::MAX);
        assert_eq!(minpct_set(15, &pct, &min, [&a, &b]), Ok(()));
        assert_eq!(pct.load(Ordering::Relaxed), 15);
        assert_eq!(min.load(Ordering::Relaxed), 15 * 256 / 100);
    }

    crate::test_file_glue!(Files, Procs);
    crate::test_proc_glue!(Procs, Files);

    /// `vm.*` through `uvm_sysctl` itself (LZ's bsd test of the `VmSysctl` bridge, which is
    /// gone): a size query of a leaf, a name below a terminal node, an unknown id.
    #[test]
    fn the_vm_sysctls_answer_as_the_c() {
        crate::testutil::install();
        let mut len = 0usize;
        let mut req = SysctlReq {
            oldp: 0,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(uvm_sysctl::<Procs>(&[VM_MAXSLP], &mut req), Ok(()));
        assert_eq!(len, core::mem::size_of::<i32>());
        let mut req = SysctlReq {
            oldp: 0,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(
            uvm_sysctl::<Procs>(&[VM_MAXSLP, 1], &mut req),
            Err(Errno::ENOTDIR)
        );
        assert_eq!(
            uvm_sysctl::<Procs>(&[hal::vm::uvmexp::VM_MAXID], &mut req),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(uvm_sysctl::<Procs>(&[], &mut req), Err(Errno::ENOTDIR));
        assert_eq!(
            uvm_sysctl::<Procs>(&[VM_SWAPENCRYPT], &mut req),
            Err(Errno::EOPNOTSUPP)
        );
        // vm.uvmexp's size is struct uvmexp's.
        assert_eq!(uvm_sysctl::<Procs>(&[VM_UVMEXP], &mut req), Ok(()));
        assert_eq!(len, core::mem::size_of::<UvmexpCopy>());
    }
}
/* </TESTS> */
