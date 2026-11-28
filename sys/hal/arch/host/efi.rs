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
//! The host double of the UEFI runtime call (`hal::cpu::efi`'s machine part): no pmap switch
//! and no trampoline; the firmware is a fake per test thread ([`fake`]): bytes mapped at
//! addresses, services at addresses that write a time and answer a status, or fault.

extern crate std;

use std::cell::RefCell;
use std::vec::Vec;

use sysdefs::base::errno::Errno;
use sysdefs::dev::efi::{EfiStatus, EfiTime};

use crate::cpu::efi::{EfiLocked, below};
use crate::vm::pmap::Pmap;

/// arm64's `PMAP_DEVICE`, which the host's pmap keeps no record of.
pub(crate) const PMAP_DEVICE: usize = 0x2;

/// What a transaction may do, made only by [`transaction`] (arm64's `Inside`).
pub(crate) struct Inside {
    /// The EFI pmap's limit.
    limit: u64,
}

/// A fake service: its status and the time it writes, or a fault.
#[derive(Clone, Copy)]
enum Service {
    /// Answers the status, writing the time when there is one.
    Answer(EfiStatus, Option<EfiTime>),
    /// Faults (the trampoline's `EFAULT`).
    Fault,
}

/// The running thread's fake firmware.
#[derive(Default)]
struct Firmware {
    /// Bytes mapped at an address.
    memory: Vec<(u64, Vec<u8>)>,
    /// Services at an address.
    services: Vec<(u64, Service)>,
    /// The services called.
    calls: usize,
    /// The time last handed to a service.
    last: Option<EfiTime>,
}

std::thread_local! {
    static FIRMWARE: RefCell<Firmware> = RefCell::new(Firmware::default());
}

/// The test thread's fake firmware.
pub mod fake {
    use super::*;

    /// Forgets the thread's firmware.
    pub fn reset() {
        FIRMWARE.with(|f| *f.borrow_mut() = Firmware::default());
    }

    /// `bytes` mapped at `va` (replacing what was mapped there).
    pub fn map(va: u64, bytes: &[u8]) {
        FIRMWARE.with(|f| {
            let mut f = f.borrow_mut();
            f.memory.retain(|(a, _)| *a != va);
            f.memory.push((va, bytes.to_vec()));
        });
    }

    /// A service at `fn_va` answering `status`, writing `time` when given.
    pub fn service(fn_va: u64, status: EfiStatus, time: Option<EfiTime>) {
        set(fn_va, Service::Answer(status, time));
    }

    /// A service at `fn_va` that faults.
    pub fn fault(fn_va: u64) {
        set(fn_va, Service::Fault);
    }

    fn set(fn_va: u64, s: Service) {
        FIRMWARE.with(|f| {
            let mut f = f.borrow_mut();
            f.services.retain(|(a, _)| *a != fn_va);
            f.services.push((fn_va, s));
        });
    }

    /// The services called since the last [`reset`].
    pub fn calls() -> usize {
        FIRMWARE.with(|f| f.borrow().calls)
    }

    /// The time last handed to a service.
    pub fn last_time() -> Option<EfiTime> {
        FIRMWARE.with(|f| f.borrow().last)
    }
}

impl Inside {
    /// `kcopy` from the firmware: `EFAULT` outside the limit or the fake's mappings.
    pub(crate) fn read(&self, va: u64, buf: &mut [u8]) -> Result<(), Errno> {
        if !below(va, buf.len(), self.limit) {
            return Err(Errno::EFAULT);
        }
        FIRMWARE.with(|f| {
            let f = f.borrow();
            for (a, bytes) in &f.memory {
                if va >= *a {
                    let off = (va - *a) as usize;
                    if let Some(src) = bytes.get(off..off + buf.len()) {
                        buf.copy_from_slice(src);
                        return Ok(());
                    }
                }
            }
            Err(Errno::EFAULT)
        })
    }

    /// The service at `fn_va` called with `time`.
    pub(crate) fn time_call(&self, fn_va: u64, time: &mut EfiTime) -> Result<EfiStatus, Errno> {
        if !below(fn_va, 1, self.limit) {
            return Err(Errno::EFAULT);
        }
        FIRMWARE.with(|f| {
            let mut f = f.borrow_mut();
            let s = f
                .services
                .iter()
                .find(|(a, _)| *a == fn_va)
                .map(|(_, s)| *s);
            match s {
                Some(Service::Answer(status, written)) => {
                    f.calls += 1;
                    f.last = Some(*time);
                    if let Some(t) = written {
                        *time = t;
                    }
                    Ok(status)
                }
                Some(Service::Fault) | None => {
                    f.calls += usize::from(s.is_some());
                    Err(Errno::EFAULT)
                }
            }
        })
    }
}

/// `efi_enter`, `body`, `efi_leave`: on the host, `body` alone.
pub(crate) fn transaction<R>(
    _held: &EfiLocked<'_>,
    pm: &Pmap,
    body: impl FnOnce(&Inside) -> R,
) -> R {
    body(&Inside {
        limit: pm.va_limit().as_usize() as u64,
    })
}
/* </CODE> */
