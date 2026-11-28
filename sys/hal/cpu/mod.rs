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
//! The running CPU and what is bound to it: the CPU's core, threads, traps, interrupts and the clock and signal frames (hal's groups, 2026-11-26). `curcpu` holds the CPU itself, re-exported here so `hal::cpu::cpu_number` and the other paths do not change.

#[forbid(unsafe_code)]
mod curcpu;
pub use curcpu::*;
pub mod thread;
// What the machine's trap code asks the kernel (N4.8b): wherever a machine's trap code is
// (arm64; amd64's is N10's) and on the host, where it has no entry.
pub mod intr;
#[forbid(unsafe_code)]
pub mod softintr;
#[cfg(any(not(target_os = "none"), target_arch = "aarch64"))]
#[forbid(unsafe_code)]
pub mod trap;
// The way back to user mode (decision 56): the exit gate the kernel's AST loop passes,
// wherever a machine's trap code is (arm64; amd64's is N10's) and on the host.
#[cfg(any(not(target_os = "none"), target_arch = "aarch64"))]
#[forbid(unsafe_code)]
pub mod usertrap;
// The clock frame (N4.8a): wherever a machine's frame is (arm64 and the host; amd64's
// `struct intrframe` at N10).
#[cfg(any(not(target_os = "none"), target_arch = "aarch64"))]
#[forbid(unsafe_code)]
pub mod clock;
// The signal frame of the running thread (N4.8b): wherever a machine's trap code is (arm64;
// amd64's is N10's) and on the host, where it is a double.
#[cfg(any(not(target_os = "none"), target_arch = "aarch64"))]
#[forbid(unsafe_code)]
pub mod sig;
// The firmware call of the SMC Calling Convention, its value class (N7.2b2a): arm64's
// conduit, and on the host a fake firmware.
#[cfg(any(not(target_os = "none"), target_arch = "aarch64"))]
#[forbid(unsafe_code)]
pub mod smccc;
/* </CODE> */
