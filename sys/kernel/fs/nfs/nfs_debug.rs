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
 * Copyright (c) 2009 Thordur I. Bjornsson. <thib@openbsd.org>
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
//! The NFS client's `ddb(4)` printers in the kernel: `show all nfsreqs` and `show all
//! nfsnodes` print every outstanding request and every node; [`nfs_request_print`] and
//! [`nfs_node_print`] print one of them (`full` adds the second line).
//!
//! Upstream: sys/nfs/nfs_debug.c @ 3ce1f3f79392
//! LZ: sys/nfs/nfs_debug.rs@e8249449d905
//!
//! ## Deviations
//! - The C walks the pools `nfsreqpl` and `nfs_node_pool` (`pool_walk`): every item in use.
//!   Here `show all nfsreqs` walks every NFS mount's `nm_reqsq` and `show all nfsnodes` the
//!   node hash that stands for every mount's `nm_ntree`: the same items (a request is on its
//!   mount's queue for as long as it waits, a node on the hash from `nfs_nget` to
//!   `nfs_reclaim`), but for a request between its `pool_get` and its queueing or after its
//!   dequeue, and a node in `nfs_nget`'s race window, which the C prints and this does not.
//! - `show nfsreq addr` and `show nfsnode addr` print the request or the node at `addr` only
//!   when one is there (on a mount's queue, on the node hash); the C reads whatever lies at
//!   the address as one.
//! - `r_procp` is printed as the requester's thread id (`proc 0x..`, 0 for none), `r_nmp` as
//!   the mount the request waits on, `r_md` as the reply's first mbuf (the
//!   reply is a chain and the bytes consumed, `nfs`'s Redesign); the C prints pointers.
//!
//! ## Redesign
//! - LZ's printers read an item through a raw pointer (`pool_walk`'s `void *`, two
//!   `unsafe`); here they take the request or the node by reference and write to a
//!   `core::fmt::Write` (`so_print`'s form, D-9). The walks are the kernel lock's: the
//!   request's chains are read under its exclusive witness, the node hash under a reader
//!   (ddb holds the kernel lock by fiat); the two address commands find their item by its
//!   address on those walks, so no address is read as an item (LZ's casts go, D-9's two
//!   casts in `bsd`'s ddb are not needed).

use core::fmt;
use core::sync::atomic::Ordering;

use hal::klock;
use hal::rwlock::Exclusive;

use super::nfs::NfsReq;
use super::nfs_node::nfs_nhash_foreach;
use super::nfsmount::NfsMount;
use super::nfsnode::{NfsNode, nfstov};
use crate::fs::mount::mountlist;
use crate::proc::ProcGlue;

/// `nfs_request_print(v, full, pr)`: prints the request `rep` of the mount `nmp` (`None`
/// when unknown), its chains read under `w`.
pub fn nfs_request_print<G: ProcGlue, W: Exclusive + ?Sized>(
    rep: &NfsReq<G>,
    nmp: Option<&NfsMount<G>>,
    full: bool,
    w: &W,
    pr: &mut dyn fmt::Write,
) -> fmt::Result {
    writeln!(
        pr,
        "xid {:#x} flags {:#x} rexmit {} procnum {} proc {:#x}",
        rep.r_xid,
        rep.r_flags.load(Ordering::Relaxed),
        rep.r_rexmit.load(Ordering::Relaxed),
        rep.r_procnum,
        rep.r_procp.unwrap_or(0)
    )?;

    if full {
        let ch = rep.r_m.get(w);
        let mreq = ch
            .r_mreq
            .as_deref()
            .map_or(0, |m| core::ptr::from_ref(m).addr());
        let mrep = ch
            .r_mrep
            .as_ref()
            .map_or(0, |r| core::ptr::from_ref(r.chain()).addr());
        writeln!(
            pr,
            "mreq {:#x} mrep {:#x} md {:#x} nfsmount {:#x} vnode {:#x} timer {} rtt {}",
            mreq,
            mrep,
            mrep,
            nmp.map_or(0, |n| core::ptr::from_ref(n).addr()),
            rep.r_vp,
            rep.r_timer,
            rep.r_rtt.load(Ordering::Relaxed)
        )?;
    }
    Ok(())
}

/// `db_show_all_nfsreqs(expr, haddr, count, modif)`: `show all nfsreqs[/f]`: every request
/// waiting on an NFS mount of the glue `G`, in full with `full` (`/f`).
pub fn db_show_all_nfsreqs<G: ProcGlue>(full: bool, pr: &mut dyn fmt::Write) -> fmt::Result {
    for mp in mountlist::<G>(false) {
        let Some(nmp) = mp.node_arc::<NfsMount<G>>() else {
            continue;
        };
        let w = klock::witness();
        for rep in nmp.nm_reqsq.iter(&w) {
            nfs_request_print(rep, Some(&nmp), full, &w, pr)?;
        }
    }
    Ok(())
}

/// `show nfsreq[/f] addr`: the request at the address `addr`, found on its mount's queue
/// (nothing printed for an address that is no waiting request; the module's Deviations).
pub fn db_nfsreq_print<G: ProcGlue>(
    addr: usize,
    full: bool,
    pr: &mut dyn fmt::Write,
) -> fmt::Result {
    for mp in mountlist::<G>(false) {
        let Some(nmp) = mp.node_arc::<NfsMount<G>>() else {
            continue;
        };
        let w = klock::witness();
        if let Some(rep) = nmp
            .nm_reqsq
            .iter(&w)
            .find(|r| core::ptr::from_ref(*r).addr() == addr)
        {
            return nfs_request_print(rep, Some(&nmp), full, &w, pr);
        }
    }
    Ok(())
}

/// `nfs_node_print(v, full, pr)`: prints the node `np`.
pub fn nfs_node_print<G: ProcGlue>(
    np: &NfsNode<G>,
    full: bool,
    pr: &mut dyn fmt::Write,
) -> fmt::Result {
    let ld = |a: &core::sync::atomic::AtomicI64| a.load(Ordering::Relaxed);
    writeln!(
        pr,
        "size {} flag {} vnode {:p} accstamp {}",
        np.n_size.load(Ordering::Relaxed),
        np.n_flag.load(Ordering::Relaxed),
        nfstov(np),
        ld(&np.n_accstamp)
    )?;

    if full {
        writeln!(
            pr,
            "pushedlo {} pushedhi {} pushlo {} pushhi {}",
            ld(&np.n_pushedlo) as u64,
            ld(&np.n_pushedhi) as u64,
            ld(&np.n_pushlo) as u64,
            ld(&np.n_pushhi) as u64
        )?;
        writeln!(
            pr,
            "commitflags {}",
            np.n_commitflags.load(Ordering::Relaxed)
        )?;
    }
    Ok(())
}

/// `show nfsnode[/f] addr`: the node at the address `addr`, found on the node hash (nothing
/// printed for an address that is no node; the module's Deviations).
pub fn db_nfsnode_print<G: ProcGlue>(
    addr: usize,
    full: bool,
    pr: &mut dyn fmt::Write,
) -> fmt::Result {
    let r = klock::reader();
    let mut res = Ok(());
    nfs_nhash_foreach::<G>(&r, |np| {
        if res.is_ok() && core::ptr::from_ref(np).addr() == addr {
            res = nfs_node_print(np, full, pr);
        }
    });
    res
}

/// `db_show_all_nfsnodes(expr, haddr, count, modif)`: `show all nfsnodes[/f]`: every node of
/// the glue `G`'s NFS mounts, in full with `full` (`/f`).
pub fn db_show_all_nfsnodes<G: ProcGlue>(full: bool, pr: &mut dyn fmt::Write) -> fmt::Result {
    let r = klock::reader();
    let mut res = Ok(());
    nfs_nhash_foreach::<G>(&r, |np| {
        if res.is_ok() {
            res = nfs_node_print(np, full, pr);
        }
    });
    res
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the printers: a request and a node, short and full, and the walks over a
    // mount's queue and the node hash.

    use std::string::String;

    use super::*;
    use crate::fs::nfs::nfs_node::{test_mount, test_node};
    use crate::fs::testfs::TestGlue as G;
    use crate::fs::vnode::VREG;
    use crate::net::mbuf::{M_WAIT, MT_DATA, m_get};

    /// The short line has the xid, flags, retransmissions, procedure and thread; the full
    /// one adds the chains, the mount, the vnode, the timer and the RTT.
    #[test]
    fn request_print_short_and_full() {
        let tm = test_mount(0);
        let m = m_get(M_WAIT, MT_DATA).expect("an mbuf");
        let rep = NfsReq::<G>::new(6, 0x1234_5678, m, Some(42), 0x1000);
        rep.r_flags.store(0x22, Ordering::Relaxed);
        rep.r_rexmit.store(3, Ordering::Relaxed);
        rep.r_rtt.store(7, Ordering::Relaxed);
        hal::klock::kernel_lock();
        let w = klock::witness();
        let mut s = String::new();
        nfs_request_print(&rep, None, false, &w, &mut s).unwrap();
        assert_eq!(
            s,
            "xid 0x12345678 flags 0x22 rexmit 3 procnum 6 proc 0x2a\n"
        );
        let mut s = String::new();
        nfs_request_print(&rep, Some(&tm.nmp), true, &w, &mut s).unwrap();
        let lines: std::vec::Vec<&str> = s.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[1].starts_with("mreq 0x"), "{s}");
        assert!(lines[1].contains("mrep 0x0 md 0x0 nfsmount 0x"), "{s}");
        assert!(lines[1].ends_with("vnode 0x1000 timer 0 rtt 7"), "{s}");
        drop(w);
        hal::klock::kernel_unlock();
    }

    /// The short line has the size, the flags, the vnode and the access stamp; the full one
    /// the commit ranges and flags.
    #[test]
    fn node_print_short_and_full() {
        let tm = test_mount(0);
        let (vp, np) = test_node(tm.mp, VREG, &[1, 2, 3, 4]);
        np.n_size.store(4096, Ordering::Relaxed);
        np.n_flag.store(4, Ordering::Relaxed);
        np.n_accstamp.store(-1, Ordering::Relaxed);
        np.n_pushedlo.store(1, Ordering::Relaxed);
        np.n_pushedhi.store(2, Ordering::Relaxed);
        np.n_pushlo.store(3, Ordering::Relaxed);
        np.n_pushhi.store(4, Ordering::Relaxed);
        np.n_commitflags.store(3, Ordering::Relaxed);
        let mut s = String::new();
        nfs_node_print(&np, true, &mut s).unwrap();
        assert_eq!(
            s,
            std::format!(
                "size 4096 flag 4 vnode {vp:p} accstamp -1\n\
                 pushedlo 1 pushedhi 2 pushlo 3 pushhi 4\ncommitflags 3\n"
            )
        );
        // The walk of the hash finds the node once it is hashed, and not after.
        hal::klock::kernel_lock();
        crate::fs::nfs::nfs_node::nfs_ninsert(np.as_pin()).expect("hashed");
        let mut s = String::new();
        db_show_all_nfsnodes::<G>(false, &mut s).unwrap();
        assert!(s.contains(&std::format!("vnode {vp:p}")), "{s}");
        // `show nfsnode addr`: the node at its address, nothing at another.
        let at = core::ptr::from_ref::<NfsNode<G>>(&np).addr();
        let mut s = String::new();
        db_nfsnode_print::<G>(at, false, &mut s).unwrap();
        assert!(s.starts_with("size 4096 flag 4 vnode"), "{s}");
        let mut s = String::new();
        db_nfsnode_print::<G>(at + 8, false, &mut s).unwrap();
        assert!(s.is_empty(), "{s}");
        crate::fs::nfs::nfs_node::nfs_nremove(&np);
        let mut s = String::new();
        db_show_all_nfsnodes::<G>(false, &mut s).unwrap();
        assert!(!s.contains(&std::format!("vnode {vp:p}")), "{s}");
        hal::klock::kernel_unlock();
    }
}
/* </TESTS> */
