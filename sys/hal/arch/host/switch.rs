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
//! The host's half of `hal::thread`'s switch: the host has no kernel stacks to switch
//! between, so a switch is refused and a thread's frames are nothing to build (as `bsd`'s
//! host double's `cpu_switchto` and `cpu_fork` were).

use core::ptr::NonNull;

use crate::thread::MachineSlot;

/// The host has one thread of execution per test thread and no kernel stacks to switch.
pub(crate) fn switch(
    _old: Option<&MachineSlot>,
    _new_slot: NonNull<MachineSlot>,
    _new_thread: NonNull<()>,
) {
    crate::assert::hal_panic(format_args!("host: cpu_switchto has no context switch"))
}

/// Nothing to set up: the host never switches to the thread.
pub(crate) fn thread_setup(
    _parent: &MachineSlot,
    _parent_runs: bool,
    _child: &MachineSlot,
    _stack: usize,
    _tcb: usize,
    _start: usize,
    _arg: usize,
) {
}

/// Nothing to rebuild: the host never switches to the thread.
pub(crate) fn thread_restart(_t: &MachineSlot, _start: usize, _arg: usize) {}
/* </CODE> */
