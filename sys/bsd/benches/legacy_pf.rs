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
//! Host microbenchmark of pf's table lookup (decision 43, D-13, N6.7b1): what
//! `pfr_match_addr` costs a packet, the per-packet path of a `<table>` in a rule, over bsd's
//! pool(9) and malloc(9) on host memory and the kernel crate's doubles. Never part of a kernel
//! image; it measures and changes nothing. The same file runs before and after pf's tables
//! moved from bsd's `radix.rs` onto the kernel's radix tree (N6.7b1): it calls pf's ioctl
//! functions and `pfr_match_addr` alone.
//!
//! `cargo bench -p bsd --bench legacy_pf` (release: the `bench` profile) or with
//! `--profile dev`. Arguments: `list`; the names of the benches to run (all by default).
//!
//! What an op is:
//! - `pf.table.match`: `pfr_match_addr` of an IPv4 address inside one of the 1,000 /24
//!   networks of an active table (the key built, the radix match, the negation test, the
//!   match counter).

#![forbid(unsafe_code)]

use std::hint::black_box;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use std::vec::Vec;

use bsd::kern::kern_malloc::kmeminit;
use bsd::kern::subr_pool::pool_init;
use bsd::kern::vm_bsd::{uvm_page_init, uvm_page_physload};
use bsd::machine::Machine;
use bsd::machine::cons::consinit;
use bsd::machine::intr::IPL_SOFTNET;
use bsd::machine::pmap::Pmap;
use bsd::net::pf_ruleset::{PF_ANCHOR_PL, PF_ANCHORS, pf_init_ruleset, pf_main_ruleset};
use bsd::net::pf_table::{
    PFR_KTABLES, PfrBuf, pfr_add_addrs, pfr_add_tables, pfr_initialize, pfr_lookup_table,
    pfr_match_addr,
};
use bsd::net::pfvar::{PfAddr, PfAnchor, PfrAddr, PfrTable};
use bsd::netinet::in_::InAddr;
use bsd::sys::param::{PAGE_MASK, PAGE_SIZE};
use bsd::sys::socket::AF_INET;
use bsd::sys::systm::PHYSMEM;
use bsd::sys::types::Vaddr;
use hal::vm::param::atop;
use hal::vm::uvmexp::UVMEXP;
use libkern::string::strlcpy::strlcpy;

/// Ops per measurement.
const OPS: usize = 1_000_000;

/// Repetitions per bench.
const REPS: usize = 7;

/// The host memory loaded as the physical memory.
const BLOCK_BYTES: usize = 32 << 20;

/// The networks of the table.
const NETS: u32 = 1000;

/// ns per op of `REPS` repetitions of `f` (which returns the time of `ops` ops): min, median,
/// max.
fn measure(ops: usize, mut f: impl FnMut() -> Duration) -> (f64, f64, f64) {
    black_box(f());
    let mut v: Vec<f64> = (0..REPS)
        .map(|_| f().as_nanos() as f64 / ops as f64)
        .collect();
    v.sort_by(f64::total_cmp);
    (v[0], v[v.len() / 2], v[v.len() - 1])
}

/// A block of host memory as the physical memory, the page system and `kmeminit`, as bsd's
/// host tests bring them up (`setup_real_memory`), once for the process.
fn host_memory() {
    kernel::testutil::install();
    UVMEXP.pagesize.store(PAGE_SIZE as i32, Ordering::Relaxed);
    kernel::vm::page::uvm_setpagesize();
    let block: &'static mut [u8] = std::vec![0u8; BLOCK_BYTES + PAGE_SIZE].leak();
    let base = (block.as_ptr() as usize + PAGE_MASK) & !PAGE_MASK;
    let (start, end) = (atop(base), atop(base + BLOCK_BYTES));
    uvm_page_physload(start, end, start, end, 0);
    PHYSMEM.store(end - start, Ordering::Relaxed);
    let (mut s, mut e) = (Vaddr::new(0), Vaddr::new(0));
    uvm_page_init(&mut s, &mut e);
    <Machine as Pmap>::pmap_bootstrap_lazy();
    kmeminit();
    consinit();
}

/// The table description `name` in the main ruleset.
fn table(name: &str) -> PfrTable {
    let mut t = PfrTable::zeroed();
    strlcpy(&mut t.pfrt_name, name.as_bytes());
    t
}

/// An IPv4 address of the table's form.
fn pf4(a: [u8; 4]) -> PfAddr {
    PfAddr::from_v4(InAddr {
        s_addr: u32::from_ne_bytes(a),
    })
}

fn main() {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| a != "--bench")
        .collect();
    let names = ["pf.table.match"];
    if args.first().map(String::as_str) == Some("list") {
        for n in names {
            println!("{n}");
        }
        return;
    }
    let wanted = |n: &str| args.is_empty() || args.iter().any(|a| a == n);

    host_memory();
    pool_init(
        &PF_ANCHOR_PL,
        size_of::<PfAnchor>(),
        align_of::<PfAnchor>() as u32,
        IPL_SOFTNET,
        0,
        "pfanchor",
        None,
    );
    PF_ANCHORS.init();
    pf_init_ruleset(pf_main_ruleset());
    pfr_initialize();
    PFR_KTABLES.init();

    // One table of 1,000 /24s: 10.0.0.0/24 .. 10.3.231.0/24.
    let mut tables = std::vec![table("bench")];
    let mut nadd = 0;
    let added = pfr_add_tables(&PfrBuf::Kernel(&mut tables), 1, Some(&mut nadd), 0);
    let mut addrs: Vec<PfrAddr> = (0..NETS)
        .map(|i| {
            let [_, _, a, b] = i.to_be_bytes();
            PfrAddr {
                pfra_u: pf4([10, a, b, 0]),
                pfra_af: AF_INET,
                pfra_net: 24,
                ..PfrAddr::default()
            }
        })
        .collect();
    let mut tbl = table("bench");
    let mut naddr = 0;
    let filled = added.and_then(|()| {
        pfr_add_addrs(
            &mut tbl,
            &mut PfrBuf::Kernel(&mut addrs),
            NETS as i32,
            Some(&mut naddr),
            0,
        )
    });
    let Some(kt) = pfr_lookup_table(&tbl).filter(|_| filled.is_ok() && naddr == NETS as i32) else {
        eprintln!("legacy_pf: the table could not be made ({filled:?}, {nadd}, {naddr})");
        std::process::exit(1);
    };
    let inside = pf4([10, 2, 3, 77]);
    let outside = pf4([192, 168, 0, 1]);
    if !pfr_match_addr(kt, &inside, AF_INET) || pfr_match_addr(kt, &outside, AF_INET) {
        eprintln!("legacy_pf: the table answers wrongly");
        std::process::exit(1);
    }

    println!("# bench min median max ns/op, spread (max-min)/median, reps {REPS}");
    let report = |name: &str, (min, med, max): (f64, f64, f64)| {
        println!(
            "{:<30} {:>12.1} {:>12.1} {:>12.1} {:>6.1}%",
            name,
            min,
            med,
            max,
            (max - min) / med * 100.0
        );
    };

    if wanted(names[0]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                black_box(pfr_match_addr(kt, black_box(&inside), AF_INET));
            }
            t.elapsed()
        });
        report(names[0], r);
    }
}
/* </CODE> */
