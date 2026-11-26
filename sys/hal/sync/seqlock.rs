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
//! `StrSlot`: a `&'static str` (or none) that one writer replaces and any CPU reads without a
//! lock: a thread's sleep message (`p_wmesg`), written by the sleeping thread under
//! `sched_lock` and read unlocked by `sysctl`'s `kinfo_proc`, ddb and the tty's status line.
//!
//! Not an OpenBSD file. The C stores a `const char *` and readers load it whenever; a Rust
//! `&str` is two words (address, length), so a reader racing the writer could pair the
//! address of one message with the length of another. The slot keeps the two words behind a
//! generation counter (a seqlock, as `pc_lock`'s producer and consumers): the writer makes the
//! generation odd, stores the words, makes it even; a reader that sees the same even
//! generation before and after loading the words has the two words of one `set`.
//!
//! ## Redesign
//! - LZ's `Cell<Option<&'static str>>` under an `unsafe impl Sync` whose comment names
//!   `sched_lock` -> a slot that is `Sync` by its own protocol: no reader needs the lock, and a
//!   second writer (which the C never has: `sched_lock` excludes it) is `panic(9)`, not a torn
//!   value.

use core::sync::atomic::{AtomicPtr, AtomicU32, AtomicUsize, Ordering, fence};

use crate::assert::hal_panic;

/// A `&'static str` or none, one writer at a time, readers without a lock.
pub struct StrSlot {
    /// Odd while a `set` is storing the words.
    seq: AtomicU32,
    /// The message's first byte, null for none.
    ptr: AtomicPtr<u8>,
    /// The message's length.
    len: AtomicUsize,
}

impl StrSlot {
    /// No message.
    pub const fn new() -> Self {
        Self {
            seq: AtomicU32::new(0),
            ptr: AtomicPtr::new(core::ptr::null_mut()),
            len: AtomicUsize::new(0),
        }
    }

    /// Stores `s`. A `set` that finds another under way on some CPU is `panic(9)`.
    pub fn set(&self, s: Option<&'static str>) {
        let g = self.seq.load(Ordering::Relaxed);
        if g & 1 != 0
            || self
                .seq
                .compare_exchange(g, g.wrapping_add(1), Ordering::Relaxed, Ordering::Relaxed)
                .is_err()
        {
            hal_panic(format_args!("StrSlot::set: two writers"));
        }
        fence(Ordering::Release);
        let (p, n) = s.map_or((core::ptr::null_mut(), 0), |s| {
            (s.as_ptr().cast_mut(), s.len())
        });
        self.ptr.store(p, Ordering::Relaxed);
        self.len.store(n, Ordering::Relaxed);
        self.seq.store(g.wrapping_add(2), Ordering::Release);
    }

    /// The message stored last.
    pub fn get(&self) -> Option<&'static str> {
        loop {
            let g = self.seq.load(Ordering::Acquire);
            if g & 1 != 0 {
                core::hint::spin_loop();
                continue;
            }
            let p = self.ptr.load(Ordering::Relaxed);
            let n = self.len.load(Ordering::Relaxed);
            fence(Ordering::Acquire);
            if self.seq.load(Ordering::Relaxed) != g {
                continue;
            }
            if p.is_null() {
                return None;
            }
            let bytes = core::ptr::slice_from_raw_parts(p.cast_const(), n);
            // SAFETY: `p` and `n` were loaded between two reads of the same even generation,
            // with the fences of the seqlock protocol (the writer's Release fence after making
            // it odd, its Release store of the next even value; this reader's Acquire load and
            // Acquire fence), so no `set` stored either word in between: both are the parts of
            // the one `&'static str` the last `set` before the first read stored (`as_ptr` and
            // `len` of it, the pointer with its provenance). Such a string lives for ever and is
            // valid UTF-8, so the reference made from its parts is valid for `'static` and the
            // bytes need no check.
            return Some(unsafe { core::str::from_utf8_unchecked(&*bytes) });
        }
    }
}

impl Default for StrSlot {
    fn default() -> Self {
        Self::new()
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::thread;

    use super::*;

    #[test]
    fn a_message_is_read_whole_while_another_cpu_writes() {
        let slot = Arc::new(StrSlot::new());
        assert_eq!(slot.get(), None);
        let stop = Arc::new(AtomicBool::new(false));
        let (s2, st2) = (Arc::clone(&slot), Arc::clone(&stop));
        let w = thread::spawn(move || {
            let msgs = [
                Some("nanoslp"),
                Some("wait"),
                None,
                Some("a much longer message"),
            ];
            let mut i = 0;
            while !st2.load(Ordering::Relaxed) {
                s2.set(msgs[i % msgs.len()]);
                i += 1;
            }
        });
        for _ in 0..200_000 {
            match slot.get() {
                None | Some("nanoslp" | "wait" | "a much longer message") => {}
                Some(other) => panic!("torn read: {other:?}"),
            }
        }
        stop.store(true, Ordering::Relaxed);
        w.join().unwrap();
    }

    #[test]
    #[should_panic(expected = "two writers")]
    fn a_second_writer_is_refused() {
        let slot = StrSlot::new();
        slot.seq.store(1, Ordering::Relaxed); // a set under way elsewhere
        slot.set(Some("x"));
    }
}
/* </TESTS> */
