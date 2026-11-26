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
 * Copyright (c) 2005 David Gwynne <dlg@openbsd.org>
 * Copyright (c) 2006 Constantine A. Murenin <cnst+openbsd@bugmail.mojo.ru>
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
 * Copyright (c) 2003, 2004 Alexander Yurchenko <grange@openbsd.org>
 * Copyright (c) 2006 Constantine A. Murenin <cnst+openbsd@bugmail.mojo.ru>
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
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
 * THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

/*-
 * Copyright (c) 1982, 1986, 1989, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * Mike Karels at Berkeley Software Design, Inc.
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
 *	@(#)kern_sysctl.c	8.4 (Berkeley) 4/14/94
 */
/* </LICENSES> */

/* <CODE> */
//! Hardware sensors: `kern/kern_sensors.c` and the kernel half of `<sys/sensors.h>`
//! (`struct ksensor`, `struct ksensordev`), with the sysctl node `hw.sensors`. Drivers embed a
//! [`Ksensordev`] and its [`Ksensor`]s, attach them ([`sensor_attach`],
//! [`sensordev_install`]) and write the values; [`sensor_task_register`] runs a driver's
//! refresh function every `period` seconds on the `sensors` task queue; `sysctl hw.sensors`
//! copies a device or a sensor out as `struct sensordev` or `struct sensor`.
//!
//! Upstream: sys/kern/kern_sensors.c @ 3ce1f3f79392
//! Upstream: sys/sys/sensors.h @ 3ce1f3f79392
//! Upstream: sys/kern/kern_sysctl.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_sensors.rs@e8249449d905
//! LZ: sys/sys/sensors.rs@e8249449d905
//! LZ: sys/kern/kern_sysctl.rs@e8249449d905
//!
//! ## Deviations
//! - `hotplug_device_attach`/`hotplug_device_detach` (`NHOTPLUG > 0`; `dev/hotplug.c` is not
//!   ported) are reported, as `subr_autoconf.rs` does, when the machine is not `cold` (LZ's).
//! - `sensordev_get` and `sensor_find` return the device or sensor, lent for the hold of the
//!   sensors' lock, instead of filling a `struct ksensordev **` (LZ's, with the lock).
//! - `sensor_task_register` returns `None` where the C returns `NULL` (an allocation
//!   failure) (LZ's).
//! - `hw.sensors.<dev>.<type>.<numt>` with a `type` outside `enum sensor_type` finds no
//!   sensor (`ENOENT`, after the device lookup's own errors), as the C's comparison would
//!   (LZ's `sysctl_sensors`).
//!
//! ## Redesign
//! - LZ's `SlistHead`s behind `unsafe impl Sync` ("at `splhigh`", "one CPU") -> `PinList`s
//!   under one [`Mutexed`] (`SENSORS_LOCK`, the C's kernel lock and `splhigh()` around the
//!   changes), held for the list steps only; the sysctl copies the members it read after
//!   releasing it. Devices and sensors are linked through `Pin<&'static T>`: their owner's
//!   detach unlinks them before the memory goes, as the C requires.
//! - `struct ksensor` and `struct ksensordev`: LZ's `Cell` members -> relaxed atomics with the
//!   same `get`/`set` (`kernel::net::if_var`'s `AtomicCell` and `AtomicBytes`, and
//!   [`TypeCell`], [`StatusCell`], [`TimevalCell`]), so the records are `Sync` by their
//!   members and valid all zero, as the zeroed softcs that embed them need (`docs/IDIOMS.md`,
//!   a statistics record drivers embed). A driver writes a member with no lock, as in the C:
//!   `desc` and `tv`, several words, can tear under a concurrent read, as the C's copy can.
//! - `sensordev_install` and `sensor_attach` keep the C's ordered walk; the C's
//!   `SLIST_INSERT_AFTER` of the element the walk found becomes a splice at that place (the
//!   elements after it moved aside, the new one appended, the rest put back), because a
//!   `PinList` lends its elements only for the witness's borrow, which the insert needs
//!   mutably. Each install or attach is O(n) as the C's walk is; the insert itself is O(n)
//!   where the C's is O(1). Attach time only.
//! - `struct sensor_task`: LZ's `malloc`ed record of raw pointers, freed by its own run, with
//!   an `unsafe fn sensor_task_unregister` -> a counted block ([`SensorTaskRef`], `M_DEVBUF`)
//!   that its timeout and its task hold (`set_shared`); `period` sits in a `Rwlocked` named
//!   `"sensor"`, held for writing across the driver's call as the C's `st->lock`; the run that
//!   finds the period 0 clears the timeout and the task, and the block goes with the last
//!   handle (the runner's, or the driver's if it kept one). The tick and the run each take
//!   one count on the block (a refcount pair) to pin it, where the C follows its pointer.
//! - The driver's function is the legacy `fn(*mut c_void)` and its argument a word, the
//!   pointer's exposed address, handed back and never read through (`docs/IDIOMS.md`, a
//!   legacy callback's `void *`).
//! - `hw.sensors` is the kernel's [`SensorsNode`] over `sysctl_sensors`, which comes from
//!   `kern_sysctl.c` (LZ's `kern_sysctl.rs`, whose notice the licence zone carries).

use core::ffi::c_void;
use core::pin::Pin;
use core::sync::atomic::{AtomicI32, AtomicI64, AtomicU32, Ordering};

use hal::cpu::intr::{IPL_HIGH, IPL_NONE};
use hal::ds::pin::{pin_static, project};
use hal::ds::pinlist::{PinAdapter, PinEntry, PinList};
use hal::mem::malloc::{M_NOWAIT, MArc};
use hal::mem::pool::AnyArc;
use hal::sync::mutex::{Mutexed, MutexedGuard};
use hal::sync::once::OnceStatic;
use hal::sync::rwlock::Rwlocked;
use hal::sync::sleep::{INFSLP, cold, sleep_finish, sleep_setup, wakeup};
use hal::sys::errno::Errno;
pub use hal::sys::sensors::*;
use hal::user::bytes_of;

use crate::assert::kpanic;
use crate::clock::Timeval;
use crate::net::if_var::{AtomicBytes, AtomicCell};
use crate::prf::unported;
use crate::sched::PZERO;
use crate::sysctl::{SysctlNode, SysctlReq, rdstruct};
use crate::task::{SYSTQ, Task, Taskq, TaskqRef, task_add, taskq_create};
use crate::timeout::{KCLOCK_NONE, Timeout};

/// `M_DEVBUF`: device driver memory (`<sys/malloc.h>`).
const M_DEVBUF: i32 = 2;

/// An `enum sensor_type` member: a relaxed atomic word with `Cell`'s `get` and `set`, valid
/// all zero (`SENSOR_TEMP`).
pub struct TypeCell(AtomicI32);

impl TypeCell {
    /// `SENSOR_TEMP`.
    pub const fn new() -> Self {
        Self(AtomicI32::new(0))
    }

    /// The type.
    pub fn get(&self) -> SensorType {
        SensorType::from_i32(self.raw()).unwrap_or(SENSOR_TEMP)
    }

    /// Stores `t`.
    pub fn set(&self, t: SensorType) {
        self.0.store(t as i32, Ordering::Relaxed);
    }

    /// The C's value.
    fn raw(&self) -> i32 {
        self.0.load(Ordering::Relaxed)
    }
}

impl Default for TypeCell {
    fn default() -> Self {
        Self::new()
    }
}

/// An `enum sensor_status` member: a relaxed atomic word with `Cell`'s `get` and `set`, valid
/// all zero (`SENSOR_S_UNSPEC`).
pub struct StatusCell(AtomicI32);

impl StatusCell {
    /// `SENSOR_S_UNSPEC`.
    pub const fn new() -> Self {
        Self(AtomicI32::new(0))
    }

    /// The state.
    pub fn get(&self) -> SensorStatus {
        SensorStatus::from_i32(self.raw()).unwrap_or(SENSOR_S_UNSPEC)
    }

    /// Stores `s`.
    pub fn set(&self, s: SensorStatus) {
        self.0.store(s as i32, Ordering::Relaxed);
    }

    /// The C's value.
    fn raw(&self) -> i32 {
        self.0.load(Ordering::Relaxed)
    }
}

impl Default for StatusCell {
    fn default() -> Self {
        Self::new()
    }
}

/// A `struct timeval` member: its two words, each a relaxed atomic, with `Cell`'s `get` and
/// `set` (a read racing a write can pair the words of two values, as the C's copy can).
/// Valid all zero.
pub struct TimevalCell {
    /// `tv_sec`.
    sec: AtomicI64,
    /// `tv_usec`.
    usec: AtomicI64,
}

impl TimevalCell {
    /// `{ 0, 0 }`.
    pub const fn new() -> Self {
        Self {
            sec: AtomicI64::new(0),
            usec: AtomicI64::new(0),
        }
    }

    /// The time.
    pub fn get(&self) -> Timeval {
        Timeval::new(
            self.sec.load(Ordering::Relaxed),
            self.usec.load(Ordering::Relaxed),
        )
    }

    /// Stores `tv`.
    pub fn set(&self, tv: Timeval) {
        self.sec.store(tv.tv_sec, Ordering::Relaxed);
        self.usec.store(tv.tv_usec, Ordering::Relaxed);
    }
}

impl Default for TimevalCell {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct ksensor`: the kernel's sensor data. The driver writes the values; `numt` and the
/// link change under the sensors' lock. Valid all zero.
pub struct Ksensor {
    /// `list`: device-scope list.
    list: PinEntry<Ksensor>,
    /// `desc`: sensor description, may be empty.
    pub desc: AtomicBytes<32>,
    /// `tv`: sensor value last change time.
    pub tv: TimevalCell,
    /// `value`: current value.
    pub value: AtomicCell<i64>,
    /// `type`: sensor type.
    pub r#type: TypeCell,
    /// `status`: sensor status.
    pub status: StatusCell,
    /// `numt`: sensor number of `.type` type.
    pub numt: AtomicCell<i32>,
    /// `flags`: sensor flags, ie. `SENSOR_FINVALID`.
    pub flags: AtomicCell<i32>,
}

impl Ksensor {
    /// A zeroed sensor (the C's `M_ZERO`/`bzero`ed `struct ksensor`).
    pub const fn new() -> Self {
        Self {
            list: PinEntry::new(),
            desc: AtomicBytes::new(),
            tv: TimevalCell::new(),
            value: AtomicCell::new(),
            r#type: TypeCell::new(),
            status: StatusCell::new(),
            numt: AtomicCell::new(),
            flags: AtomicCell::new(),
        }
    }

    /// Whether the sensor is attached to a device.
    pub fn is_attached(&self) -> bool {
        self.list.is_linked()
    }
}

impl Default for Ksensor {
    fn default() -> Self {
        Self::new()
    }
}

hal::pin_adapter!(
    /// `SLIST_HEAD(ksensors_head, ksensor)`: a device's sensors, through `list`.
    pub KsensorList: Ksensor, list
);

/// `struct ksensordev`: the kernel's sensor device data. Its sensors and its link change
/// under the sensors' lock; the driver writes `xname` before installing it. Valid all zero.
pub struct Ksensordev {
    /// `list`: the link in `sensordev_list`.
    list: PinEntry<Ksensordev>,
    /// `num`: sensordev number.
    pub num: AtomicCell<i32>,
    /// `xname`: unix device name, NUL-terminated.
    pub xname: AtomicBytes<16>,
    /// `maxnumt`: per type, one past the highest `numt` attached.
    pub maxnumt: [AtomicCell<i32>; SENSOR_MAX_TYPES],
    /// `sensors_count`.
    pub sensors_count: AtomicCell<i32>,
    /// `sensors_list`, in type then number order.
    sensors_list: PinList<KsensorList>,
}

impl Ksensordev {
    /// A zeroed sensor device.
    pub const fn new() -> Self {
        Self {
            list: PinEntry::new(),
            num: AtomicCell::new(),
            xname: AtomicBytes::new(),
            maxnumt: [const { AtomicCell::new() }; SENSOR_MAX_TYPES],
            sensors_count: AtomicCell::new(),
            sensors_list: PinList::new(),
        }
    }

    /// Whether the device is installed.
    pub fn is_installed(&self) -> bool {
        self.list.is_linked()
    }
}

impl Default for Ksensordev {
    fn default() -> Self {
        Self::new()
    }
}

hal::pin_adapter!(
    /// `SLIST_HEAD(, ksensordev) sensordev_list`: the installed devices, through `list`.
    pub KsensordevList: Ksensordev, list
);

/// `struct sensor_task`: a driver's refresh function, run every `period` seconds on the
/// `sensors` queue. Held by its timeout and its task, and by the driver's handle.
pub struct SensorTask {
    /// `func`: the driver's refresh function.
    func: fn(*mut c_void),
    /// `arg`: its argument's exposed address.
    arg: usize,
    /// `period` under `lock` (`"sensor"`): seconds between runs; 0 marks a dying task.
    period: Rwlocked<u32>,
    /// `timeout`: schedules the next run.
    timeout: Timeout,
    /// `task`: the run on the `sensors` queue.
    task: Task,
}

/// The driver's handle on its refresh task (the C's `struct sensor_task *`).
pub type SensorTaskRef = MArc<SensorTask>;

/// The sensors' lock: `sensordev_list`, each device's sensors, the device count.
static SENSORS_LOCK: Mutexed<()> = Mutexed::new(IPL_NONE, ());
/// `sensordev_list`, sorted by `num`.
static SENSORDEV_LIST: PinList<KsensordevList> = PinList::new();
/// `sensordev_count`, under [`SENSORS_LOCK`].
static SENSORDEV_COUNT: AtomicI32 = AtomicI32::new(0);
/// `sensors_taskq`: the queue the refresh tasks run on, `None` for `systq` (made by the
/// first registration).
static SENSORS_TASKQ: OnceStatic<Option<TaskqRef>> = OnceStatic::new();
/// `sensors_quiesced`.
static SENSORS_QUIESCED: AtomicI32 = AtomicI32::new(0);
/// `sensors_running`: refresh functions running now; also the quiesce's wait channel.
static SENSORS_RUNNING: AtomicU32 = AtomicU32::new(0);

/// The sensors' lock, for [`sensordev_get`] and [`sensor_find`] (the C's kernel lock).
pub fn sensors_lock() -> MutexedGuard<'static, ()> {
    SENSORS_LOCK.lock()
}

/// The C's `SLIST_INSERT_AFTER` of `e` after the `k`-th element of `list` (0-based): the
/// elements after it go aside, `e` is appended, they come back.
fn insert_after_nth<A: PinAdapter>(
    list: &PinList<A>,
    w: &mut MutexedGuard<'_, ()>,
    k: usize,
    e: Pin<&A::Elem>,
) {
    let rest = PinList::<A>::new();
    let mut i = 0usize;
    list.move_where(
        w,
        usize::MAX,
        |_| {
            let after = i > k;
            i += 1;
            after
        },
        &rest,
    );
    list.insert_tail(w, e);
    list.concat(w, &rest);
}

/// `sensordev_install`: gives `sensdev` the lowest free number and links it into
/// `sensordev_list`, which stays sorted.
pub fn sensordev_install(sensdev: &'static Ksensordev) {
    {
        let mut w = SENSORS_LOCK.lock();
        // The first device followed by a gap in the numbers, or the last one.
        let mut at = None;
        if let Some(mut v) = SENSORDEV_LIST.first(&w) {
            let mut k = 0;
            while let Some(nv) = SENSORDEV_LIST.next(&w, v) {
                if nv.num.get() - v.num.get() > 1 {
                    break;
                }
                v = nv;
                k += 1;
            }
            at = Some((k, v.num.get() + 1));
        }
        match at {
            None => {
                sensdev.num.set(0);
                SENSORDEV_LIST.insert_head(&mut w, pin_static(sensdev));
            }
            Some((k, num)) => {
                sensdev.num.set(num);
                insert_after_nth(&SENSORDEV_LIST, &mut w, k, pin_static(sensdev));
            }
        }
        SENSORDEV_COUNT.fetch_add(1, Ordering::Relaxed);
    }

    // NHOTPLUG > 0
    if !cold() {
        let _ = unported!("hotplug_device_attach (dev/hotplug.c)");
    }
}

/// `sensor_attach`: links `sens` into `sensdev`'s sensors after the others of its type,
/// giving it the lowest free number of that type (`numt`).
pub fn sensor_attach(sensdev: &Ksensordev, sens: &'static Ksensor) {
    let sh = &sensdev.sensors_list;
    let ty = sens.r#type.raw();

    let mut w = SENSORS_LOCK.lock();
    if sensdev.sensors_count.get() == 0 {
        for m in &sensdev.maxnumt {
            m.set(0);
        }
        sens.numt.set(0);
        sh.insert_head(&mut w, pin_static(sens));
    } else {
        // The last sensor of this type before a gap in its numbers or another type, or
        // the last sensor.
        let mut at = None;
        if let Some(mut v) = sh.first(&w) {
            let mut k = 0;
            while let Some(nv) = sh.next(&w, v) {
                let vt = v.r#type.raw();
                if vt == ty && (vt != nv.r#type.raw() || nv.numt.get() - v.numt.get() > 1) {
                    break;
                }
                v = nv;
                k += 1;
            }
            // sensors of the same type go after each other
            let numt = if v.r#type.raw() == ty {
                v.numt.get() + 1
            } else {
                0
            };
            at = Some((k, numt));
        }
        match at {
            Some((k, numt)) => {
                sens.numt.set(numt);
                insert_after_nth(sh, &mut w, k, pin_static(sens));
            }
            None => {
                sens.numt.set(0);
                sh.insert_head(&mut w, pin_static(sens));
            }
        }
    }
    // we only increment maxnumt[] if the sensor was added to the last position of sensors
    // of this type
    if let Some(m) = maxnumt_of(sensdev, ty)
        && m.get() == sens.numt.get()
    {
        m.set(m.get() + 1);
    }
    sensdev.sensors_count.set(sensdev.sensors_count.get() + 1);
}

/// `sensdev->maxnumt[type]`.
fn maxnumt_of(sensdev: &Ksensordev, ty: i32) -> Option<&AtomicCell<i32>> {
    usize::try_from(ty)
        .ok()
        .and_then(|t| sensdev.maxnumt.get(t))
}

/// `sensordev_deinstall`: unlinks `sensdev` from `sensordev_list`.
pub fn sensordev_deinstall(sensdev: &Ksensordev) {
    {
        let mut w = SENSORS_LOCK.lock();
        SENSORDEV_COUNT.fetch_sub(1, Ordering::Relaxed);
        SENSORDEV_LIST.remove(&mut w, sensdev);
    }

    // NHOTPLUG > 0
    if !cold() {
        let _ = unported!("hotplug_device_detach (dev/hotplug.c)");
    }
}

/// `sensor_detach`: unlinks `sens` from `sensdev`'s sensors.
pub fn sensor_detach(sensdev: &Ksensordev, sens: &Ksensor) {
    let mut w = SENSORS_LOCK.lock();
    sensdev.sensors_count.set(sensdev.sensors_count.get() - 1);
    sensdev.sensors_list.remove(&mut w, sens);
    // we only decrement maxnumt[] if this is the tail sensor of this type
    if let Some(m) = maxnumt_of(sensdev, sens.r#type.raw())
        && sens.numt.get() == m.get() - 1
    {
        m.set(m.get() - 1);
    }
}

/// `sensordev_get`: the installed device numbered `num`, lent for the hold `w` of the
/// sensors' lock; `ENXIO` when the number is a gap, `ENOENT` past the last device.
pub fn sensordev_get<'a>(w: &'a MutexedGuard<'_, ()>, num: i32) -> Result<&'a Ksensordev, Errno> {
    for sd in SENSORDEV_LIST.iter(w) {
        if sd.num.get() == num {
            return Ok(sd);
        }
        if sd.num.get() > num {
            return Err(Errno::ENXIO);
        }
    }
    Err(Errno::ENOENT)
}

/// `sensor_find`: sensor `numt` of type `type` on device `dev`, lent for the hold `w`.
pub fn sensor_find<'a>(
    w: &'a MutexedGuard<'_, ()>,
    dev: i32,
    r#type: SensorType,
    numt: i32,
) -> Result<&'a Ksensor, Errno> {
    let sensdev = sensordev_get(w, dev)?;
    sensdev
        .sensors_list
        .iter(w)
        .find(|s| s.r#type.raw() == r#type as i32 && s.numt.get() == numt)
        .ok_or(Errno::ENOENT)
}

/// The queue the refresh tasks run on.
fn sensors_taskq() -> &'static Taskq {
    match SENSORS_TASKQ.get() {
        Some(Some(tq)) => tq,
        _ => &SYSTQ,
    }
}

/// `sensor_task_register`: runs `func(arg)` now and then every `period` seconds on the
/// `sensors` task queue; `arg` is the exposed address (`expose_provenance`) of what `func` is
/// called with. `None` when the task cannot be allocated.
pub fn sensor_task_register(
    arg: usize,
    func: fn(*mut c_void),
    period: u32,
) -> Option<SensorTaskRef> {
    #[cfg(feature = "diagnostic")]
    if period == 0 {
        kpanic(format_args!("sensor_task_register: period is 0"));
    }

    if !SENSORS_TASKQ.is_set() {
        // A second registration racing this one makes a queue that stays unused, as the
        // C's would.
        let _ = SENSORS_TASKQ.set(taskq_create(b"sensors", 1, IPL_HIGH, 0));
    }

    let st = MArc::new(
        M_DEVBUF,
        M_NOWAIT,
        SensorTask {
            func,
            arg,
            period: Rwlocked::new("sensor", period),
            timeout: Timeout::unset(),
            task: Task::unset(),
        },
    )?;
    st.timeout
        .set_shared(sensor_task_tick, MArc::erase(st.clone()), KCLOCK_NONE, 0);
    st.task
        .set_shared(sensor_task_work, MArc::erase(st.clone()));

    sensor_task_tick_on(&st);

    Some(st)
}

/// `sensor_task_unregister`: marks the task as dying; its next run frees it.
///
/// We can't reliably `timeout_del` or `task_del` because there's a window between when they
/// come off the lists and the timeout or task code actually runs the respective handlers for
/// them. Mark the sensor task as dying by setting period to 0 and let `sensor_task_work` mop
/// up.
pub fn sensor_task_unregister(st: &SensorTaskRef) {
    *st.period.write() = 0;
}

/// The block an erased handle of the timeout or the task names.
fn sensor_task_of(arg: &AnyArc) -> SensorTaskRef {
    match arg.clone().downcast_arc::<SensorTask>() {
        Some(st) => st,
        None => kpanic(format_args!("sensor_task: not a sensor task")),
    }
}

/// `sensor_task_tick`: the timeout queues the task.
fn sensor_task_tick(arg: &AnyArc) {
    sensor_task_tick_on(&sensor_task_of(arg));
}

/// `sensor_task_tick(st)`.
fn sensor_task_tick_on(st: &SensorTaskRef) {
    let _ = task_add(sensors_taskq(), project(st.as_pin(), |s| &s.task));
}

/// `sensor_quiesce`: stops the refresh functions and waits for the running ones.
///
/// The C checks `sensors_running` and sleeps under the kernel lock, which the runs' wakeup
/// also holds. Here the interlock is the sleep queue's own: the thread is on the channel's
/// queue (`sleep_setup`) before it reads the count, so a run that drops the count to 0 after
/// the read finds it there and wakes it (its `wakeup` takes the same queue lock); and the
/// flag and the count are `SeqCst` on both sides, so either the last run sees the flag (and
/// wakes) or this read sees its decrement (and does not sleep).
pub fn sensor_quiesce() {
    SENSORS_QUIESCED.store(1, Ordering::SeqCst);
    loop {
        sleep_setup(&SENSORS_RUNNING, PZERO, "sensorpause");
        let busy = SENSORS_RUNNING.load(Ordering::SeqCst) > 0;
        let _ = sleep_finish(INFSLP, busy);
        if !busy {
            break;
        }
    }
}

/// `sensor_restart`.
pub fn sensor_restart() {
    SENSORS_QUIESCED.store(0, Ordering::SeqCst);
}

/// `sensor_task_work`: one run: calls the driver's function unless the task is dying or the
/// sensors are quiesced, then rearms the timeout, or lets a dying task go.
fn sensor_task_work(arg: &AnyArc) {
    let st = sensor_task_of(arg);

    SENSORS_RUNNING.fetch_add(1, Ordering::SeqCst);
    let period = {
        let p = st.period.write();
        if *p > 0 && SENSORS_QUIESCED.load(Ordering::SeqCst) == 0 {
            (st.func)(core::ptr::with_exposed_provenance_mut(st.arg));
        }
        *p
    };
    if SENSORS_RUNNING.fetch_sub(1, Ordering::SeqCst) == 1
        && SENSORS_QUIESCED.load(Ordering::SeqCst) != 0
    {
        wakeup(&SENSORS_RUNNING);
    }

    if period == 0 {
        // free(st): the timeout is not pending (only this run arms it) and the task is
        // running (its runner holds its own handle for the call): both give theirs back, and
        // the block goes with the last handle.
        st.timeout.clear();
        st.task.clear();
    } else {
        let secs = i32::try_from(period).unwrap_or(i32::MAX);
        let _ = project(st.as_pin(), |s| &s.timeout).add_sec(secs);
    }
}

/// `sysctl_sensors`: `hw.sensors.<dev>` (a `struct sensordev`) and
/// `hw.sensors.<dev>.<type>.<numt>` (a `struct sensor`), copies without the kernel's links.
fn sysctl_sensors(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    let (dev, sensor) = match *name {
        [dev] => (dev, None),
        [dev, ty, numt] => (dev, Some((ty, numt))),
        _ => return Err(Errno::ENOTDIR),
    };

    match sensor {
        None => {
            // Grab a copy, to clear the kernel pointers
            let usd = {
                let w = SENSORS_LOCK.lock();
                let ksd = sensordev_get(&w, dev)?;
                let mut xname = [0u8; 16];
                libkern::strlcpy(&mut xname, &ksd.xname.get());
                Sensordev {
                    num: ksd.num.get(),
                    xname,
                    maxnumt: core::array::from_fn(|t| ksd.maxnumt[t].get()),
                    sensors_count: ksd.sensors_count.get(),
                }
            };
            rdstruct(req, bytes_of(core::slice::from_ref(&usd)))
        }
        Some((ty, numt)) => {
            // Grab a copy, to clear the kernel pointers
            let us = {
                let w = SENSORS_LOCK.lock();
                // `(enum sensor_type)name[1]`: a value outside the enum matches no sensor.
                let ks = match SensorType::from_i32(ty) {
                    Some(t) => sensor_find(&w, dev, t, numt)?,
                    None => {
                        sensordev_get(&w, dev)?;
                        return Err(Errno::ENOENT);
                    }
                };
                let tv = ks.tv.get();
                Sensor {
                    desc: ks.desc.get(),
                    tv: hal::sys::net_if::IfTimeval {
                        tv_sec: tv.tv_sec,
                        tv_usec: tv.tv_usec,
                    },
                    value: ks.value.get(),
                    r#type: ks.r#type.raw(),
                    status: ks.status.raw(),
                    numt: ks.numt.get(),
                    flags: ks.flags.get(),
                }
            };
            rdstruct(req, bytes_of(core::slice::from_ref(&us)))
        }
    }
}

/// `hw.sensors` (`HW_SENSORS`): `sysctl_sensors`.
pub struct SensorsNode;

impl SysctlNode for SensorsNode {
    fn sysctl(&self, name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
        sysctl_sensors(name, req)
    }
}

/// Forgets every device an earlier test installed (unlinked, not freed: their sensors stay
/// on their own lists), the count and the quiesce, for the host tests (`testing.md`).
#[cfg(any(test, feature = "doubles"))]
pub fn sensors_test_reset() {
    let mut w = SENSORS_LOCK.lock();
    while SENSORDEV_LIST.remove_first(&mut w) {}
    SENSORDEV_COUNT.store(0, Ordering::Relaxed);
    SENSORS_QUIESCED.store(0, Ordering::Relaxed);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The device numbers and the sensors' order and numbers as the C's walks give them,
    // hw.sensors's two structures byte for byte and its refusals, the refresh task's runs,
    // quiesce and end.

    use std::sync::{Mutex, MutexGuard};
    use std::vec::Vec;

    use hal::user::UserPtr;

    use super::*;
    use crate::testutil;

    /// The tests share the process's one device list.
    fn setup() -> MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        testutil::install();
        sensors_test_reset();
        g
    }

    fn nums() -> Vec<i32> {
        SENSORDEV_LIST
            .iter(&sensors_lock())
            .map(|d| d.num.get())
            .collect()
    }

    /// The (type, numt) of `sd`'s sensors, in list order.
    fn order(sd: &Ksensordev) -> Vec<(SensorType, i32)> {
        sd.sensors_list
            .iter(&sensors_lock())
            .map(|s| (s.r#type.get(), s.numt.get()))
            .collect()
    }

    fn bytes16(s: &[u8]) -> [u8; 16] {
        let mut b = [0u8; 16];
        b[..s.len()].copy_from_slice(s);
        b
    }

    fn bytes32(s: &[u8]) -> [u8; 32] {
        let mut b = [0u8; 32];
        b[..s.len()].copy_from_slice(s);
        b
    }

    #[test]
    fn device_numbers_reuse_the_lowest_gap() {
        let _g = setup();
        static D: [Ksensordev; 4] = [const { Ksensordev::new() }; 4];
        sensordev_install(&D[0]);
        sensordev_install(&D[1]);
        sensordev_install(&D[2]);
        assert_eq!(nums(), [0, 1, 2]);
        sensordev_deinstall(&D[1]);
        assert!(!D[1].is_installed());
        sensordev_install(&D[3]);
        assert_eq!(D[3].num.get(), 1);
        assert_eq!(nums(), [0, 1, 2]);
        {
            let w = sensors_lock();
            assert!(core::ptr::eq(sensordev_get(&w, 1).unwrap(), &D[3]));
            assert_eq!(sensordev_get(&w, 3).err(), Some(Errno::ENOENT));
        }
        sensordev_deinstall(&D[0]);
        {
            // A gap before the first device: ENXIO, as the C's sorted walk answers.
            let w = sensors_lock();
            assert_eq!(sensordev_get(&w, 0).err(), Some(Errno::ENXIO));
        }
        // The C's walk fills gaps after the first device only: a gap before it stays.
        sensordev_install(&D[0]);
        assert_eq!(D[0].num.get(), 3);
        assert_eq!(nums(), [1, 2, 3]);
        for d in [&D[0], &D[2], &D[3]] {
            sensordev_deinstall(d);
        }
        assert!(nums().is_empty());
    }

    #[test]
    fn sensors_keep_type_then_number_order() {
        let _g = setup();
        static SD: Ksensordev = Ksensordev::new();
        static S: [Ksensor; 6] = [const { Ksensor::new() }; 6];
        let types = [
            SENSOR_TEMP,
            SENSOR_TEMP,
            SENSOR_FANRPM,
            SENSOR_TEMP,
            SENSOR_VOLTS_DC,
            SENSOR_TEMP,
        ];
        for (s, t) in S.iter().zip(types).take(4) {
            s.r#type.set(t);
            sensor_attach(&SD, s);
        }
        assert_eq!(
            order(&SD),
            [
                (SENSOR_TEMP, 0),
                (SENSOR_TEMP, 1),
                (SENSOR_TEMP, 2),
                (SENSOR_FANRPM, 0)
            ]
        );
        assert_eq!(SD.maxnumt[SENSOR_TEMP as usize].get(), 3);
        assert_eq!(SD.maxnumt[SENSOR_FANRPM as usize].get(), 1);
        assert_eq!(SD.sensors_count.get(), 4);

        // A sensor of a new type goes last; a detached middle number is taken again.
        S[4].r#type.set(types[4]);
        sensor_attach(&SD, &S[4]);
        sensor_detach(&SD, &S[1]);
        assert_eq!(SD.maxnumt[SENSOR_TEMP as usize].get(), 3);
        S[5].r#type.set(types[5]);
        sensor_attach(&SD, &S[5]);
        assert_eq!(S[5].numt.get(), 1);
        assert_eq!(
            order(&SD),
            [
                (SENSOR_TEMP, 0),
                (SENSOR_TEMP, 1),
                (SENSOR_TEMP, 2),
                (SENSOR_FANRPM, 0),
                (SENSOR_VOLTS_DC, 0)
            ]
        );
        assert_eq!(SD.maxnumt[SENSOR_TEMP as usize].get(), 3);
        // The tail sensor of a type leaving lowers maxnumt.
        sensor_detach(&SD, &S[3]);
        assert_eq!(SD.maxnumt[SENSOR_TEMP as usize].get(), 2);
        assert_eq!(SD.sensors_count.get(), 4);
        for s in [&S[0], &S[5], &S[2], &S[4]] {
            sensor_detach(&SD, s);
        }
        assert_eq!(SD.sensors_count.get(), 0);
    }

    #[test]
    fn hw_sensors_copies_the_two_structures_byte_for_byte() {
        let _g = setup();
        static SD: Ksensordev = Ksensordev::new();
        static S: [Ksensor; 2] = [const { Ksensor::new() }; 2];
        SD.xname.set(bytes16(b"viomb0"));
        S[0].desc.set(bytes32(b"desired"));
        S[0].r#type.set(SENSOR_INTEGER);
        sensor_attach(&SD, &S[0]);
        S[0].value.set(268_435_456);
        S[1].desc.set(bytes32(b"current"));
        S[1].r#type.set(SENSOR_INTEGER);
        sensor_attach(&SD, &S[1]);
        S[1].value.set(-5);
        S[1].tv.set(Timeval::new(1_791_030_896, 250_000));
        S[1].status.set(SENSOR_S_WARN);
        S[1].flags.set(SENSOR_FUNKNOWN);
        sensordev_install(&SD);
        let dev = SD.num.get();
        let u = testutil::USER_BASE + 0xC000;

        // struct sensordev: num, xname[16], maxnumt[23], sensors_count.
        let mut want = Vec::new();
        want.extend_from_slice(&dev.to_ne_bytes());
        want.extend_from_slice(&bytes16(b"viomb0"));
        for t in 0..SENSOR_MAX_TYPES {
            let m: i32 = if t == SENSOR_INTEGER as usize { 2 } else { 0 };
            want.extend_from_slice(&m.to_ne_bytes());
        }
        want.extend_from_slice(&2i32.to_ne_bytes());
        assert_eq!(want.len(), 116);
        let mut len = 116;
        let mut q = SysctlReq {
            oldp: u,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(SensorsNode.sysctl(&[dev], &mut q), Ok(()));
        assert_eq!(len, 116);
        assert_eq!(testutil::user_bytes(u, 116), Some(want));

        // struct sensor: desc[32], tv, value, type, status, numt, flags.
        let mut want = Vec::new();
        want.extend_from_slice(&bytes32(b"current"));
        want.extend_from_slice(&1_791_030_896i64.to_ne_bytes());
        want.extend_from_slice(&250_000i64.to_ne_bytes());
        want.extend_from_slice(&(-5i64).to_ne_bytes());
        for v in [
            SENSOR_INTEGER as i32,
            SENSOR_S_WARN as i32,
            1,
            SENSOR_FUNKNOWN,
        ] {
            want.extend_from_slice(&v.to_ne_bytes());
        }
        assert_eq!(want.len(), 72);
        let mut len = 72;
        let mut q = SysctlReq {
            oldp: u,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        let name = [dev, SENSOR_INTEGER as i32, 1];
        assert_eq!(SensorsNode.sysctl(&name, &mut q), Ok(()));
        assert_eq!(testutil::user_bytes(u, 72), Some(want));

        // The refusals: (name, room, a new value, the answer).
        let newp = u + 0x200;
        UserPtr::<i32>::new(newp).copyout(&0).unwrap();
        let ty = SENSOR_INTEGER as i32;
        let cases: &[(&[i32], usize, bool, Result<(), Errno>)] = &[
            (&[], 116, false, Err(Errno::ENOTDIR)),
            (&[dev, ty], 72, false, Err(Errno::ENOTDIR)),
            (&[dev, ty, 0, 0], 72, false, Err(Errno::ENOTDIR)),
            (&[dev + 7], 116, false, Err(Errno::ENOENT)),
            (&[dev, ty, 2], 72, false, Err(Errno::ENOENT)),
            (&[dev, 99, 0], 72, false, Err(Errno::ENOENT)),
            (&[dev, -1, 0], 72, false, Err(Errno::ENOENT)),
            (&[dev + 7, 99, 0], 72, false, Err(Errno::ENOENT)),
            (&[dev + 7, ty, 0], 72, false, Err(Errno::ENOENT)),
            (&[dev], 115, false, Err(Errno::ENOMEM)),
            (&[dev, ty, 0], 71, false, Err(Errno::ENOMEM)),
            (&[dev], 116, true, Err(Errno::EPERM)),
            (&[dev, ty, 0], 72, true, Err(Errno::EPERM)),
        ];
        for (i, (name, room, new, want)) in cases.iter().enumerate() {
            let mut len = *room;
            let mut q = SysctlReq {
                oldp: u,
                oldlenp: &mut len,
                newp: if *new { newp } else { 0 },
                newlen: if *new { 4 } else { 0 },
            };
            assert_eq!(
                SensorsNode.sysctl(name, &mut q),
                *want,
                "case {i}: {name:?}"
            );
        }

        sensordev_deinstall(&SD);
        sensor_detach(&SD, &S[0]);
        sensor_detach(&SD, &S[1]);
    }

    /// The runs of [`refresh`], by the argument's address.
    static RUNS: Mutex<Vec<usize>> = Mutex::new(Vec::new());

    fn refresh(arg: *mut c_void) {
        RUNS.lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(arg.addr());
    }

    fn runs_of(arg: usize) -> usize {
        RUNS.lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|&&a| a == arg)
            .count()
    }

    #[test]
    fn a_task_runs_stops_when_quiesced_and_ends_after_unregister() {
        let _g = setup();
        // An hour's period: no other test's clock reaches the rearmed timeout.
        let arg = 0x5E50_0000usize;
        let st = sensor_task_register(arg, refresh, 3600).unwrap();
        // The first run is the queue thread's when the host runs one; otherwise the task is
        // taken off the queue (freed while queued, its entry would stop the test).
        let t0 = std::time::Instant::now();
        while st.task.pending() && t0.elapsed() < std::time::Duration::from_secs(2) {
            std::thread::yield_now();
        }
        if st.task.pending() {
            let _ = crate::task::task_del(sensors_taskq(), &st.task);
        }
        // The task, the timeout and this handle (a runner's own given back).
        while MArc::count(&st) > 3 {
            std::thread::yield_now();
        }
        assert_eq!(MArc::count(&st), 3);
        let run = || sensor_task_work(&MArc::erase(st.clone()));
        let before = runs_of(arg);
        run();
        assert_eq!(runs_of(arg), before + 1);
        assert!(st.timeout.pending());

        sensor_quiesce();
        run();
        assert_eq!(runs_of(arg), before + 1);
        sensor_restart();
        run();
        assert_eq!(runs_of(arg), before + 2);

        let _ = st.timeout.del();
        sensor_task_unregister(&st);
        run();
        assert_eq!(runs_of(arg), before + 2);
        assert!(!st.timeout.pending());
        // The timeout and the task gave their handles back: the block goes with this one
        // (or with a queue thread's run still holding its own).
        while MArc::count(&st) > 1 {
            std::thread::yield_now();
        }
    }
}
/* </TESTS> */
