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
/* </LICENSES> */

/* <CODE> */
//! `<sys/sensors.h>`'s ABI half: the `struct sensor` and `struct sensordev` that
//! `sysctl hw.sensors` copies out, the sensor types and states, and the flags.
//!
//! Upstream: sys/sys/sensors.h @ 3ce1f3f79392
//! LZ: sys/sys/sensors.rs@e8249449d905
//!
//! ## Deviations
//! - `enum sensor_type` and `enum sensor_status` are `#[repr(i32)]` Rust enums with the C's
//!   names and values ([`SensorType`], [`SensorStatus`]); the user structures keep plain `i32`
//!   members (`r#type`, `status`), valid for every bit pattern (LZ's).
//! - `sensor_type_s[]` is `#ifndef _KERNEL` (userland only) and is not part of the kernel
//!   (LZ's).
//!
//! ## Redesign
//! - N7.1b: the user structures move from `bsd`'s `sys/sys/sensors.rs` to `hal::sys` (one
//!   layout for both machines: `i32`, `i64`, byte arrays and `struct timeval`'s image,
//!   [`IfTimeval`]), beside the types and flags the kernel's `struct ksensor` uses
//!   (`kernel::sensors`). `bsd`'s header re-exports them.

use crate::sys::net_if::IfTimeval;

/// `enum sensor_type`: sensor types.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)] // OpenBSD names, verbatim, for grep-ability
pub enum SensorType {
    /// temperature (uK)
    SENSOR_TEMP = 0,
    /// fan revolution speed
    SENSOR_FANRPM,
    /// voltage (uV DC)
    SENSOR_VOLTS_DC,
    /// voltage (uV AC)
    SENSOR_VOLTS_AC,
    /// resistance
    SENSOR_OHMS,
    /// power (uW)
    SENSOR_WATTS,
    /// current (uA)
    SENSOR_AMPS,
    /// power capacity (uWh)
    SENSOR_WATTHOUR,
    /// power capacity (uAh)
    SENSOR_AMPHOUR,
    /// boolean indicator
    SENSOR_INDICATOR,
    /// generic integer value
    SENSOR_INTEGER,
    /// percent (m%)
    SENSOR_PERCENT,
    /// illuminance (ulx)
    SENSOR_LUX,
    /// disk
    SENSOR_DRIVE,
    /// system time error (nSec)
    SENSOR_TIMEDELTA,
    /// humidity (m%RH)
    SENSOR_HUMIDITY,
    /// frequency (uHz)
    SENSOR_FREQ,
    /// angle (uDegrees)
    SENSOR_ANGLE,
    /// distance (uMeter)
    SENSOR_DISTANCE,
    /// pressure (mPa)
    SENSOR_PRESSURE,
    /// acceleration (u m/s^2)
    SENSOR_ACCEL,
    /// velocity (u m/s)
    SENSOR_VELOCITY,
    /// energy (uJ)
    SENSOR_ENERGY,
}

pub use SensorType::*;

/// `SENSOR_MAX_TYPES`: the number of sensor types.
pub const SENSOR_MAX_TYPES: usize = SENSOR_ENERGY as usize + 1;

impl SensorType {
    /// Every type, in the C's order (index = value).
    pub const ALL: [SensorType; SENSOR_MAX_TYPES] = [
        SENSOR_TEMP,
        SENSOR_FANRPM,
        SENSOR_VOLTS_DC,
        SENSOR_VOLTS_AC,
        SENSOR_OHMS,
        SENSOR_WATTS,
        SENSOR_AMPS,
        SENSOR_WATTHOUR,
        SENSOR_AMPHOUR,
        SENSOR_INDICATOR,
        SENSOR_INTEGER,
        SENSOR_PERCENT,
        SENSOR_LUX,
        SENSOR_DRIVE,
        SENSOR_TIMEDELTA,
        SENSOR_HUMIDITY,
        SENSOR_FREQ,
        SENSOR_ANGLE,
        SENSOR_DISTANCE,
        SENSOR_PRESSURE,
        SENSOR_ACCEL,
        SENSOR_VELOCITY,
        SENSOR_ENERGY,
    ];

    /// The type whose C value is `v` (`(enum sensor_type)v`); `None` outside the enum.
    pub fn from_i32(v: i32) -> Option<Self> {
        usize::try_from(v)
            .ok()
            .and_then(|i| Self::ALL.get(i).copied())
    }
}

/// `SENSOR_DRIVE_EMPTY`.
pub const SENSOR_DRIVE_EMPTY: i64 = 1;
/// `SENSOR_DRIVE_READY`.
pub const SENSOR_DRIVE_READY: i64 = 2;
/// `SENSOR_DRIVE_POWERUP`.
pub const SENSOR_DRIVE_POWERUP: i64 = 3;
/// `SENSOR_DRIVE_ONLINE`.
pub const SENSOR_DRIVE_ONLINE: i64 = 4;
/// `SENSOR_DRIVE_IDLE`.
pub const SENSOR_DRIVE_IDLE: i64 = 5;
/// `SENSOR_DRIVE_ACTIVE`.
pub const SENSOR_DRIVE_ACTIVE: i64 = 6;
/// `SENSOR_DRIVE_REBUILD`.
pub const SENSOR_DRIVE_REBUILD: i64 = 7;
/// `SENSOR_DRIVE_POWERDOWN`.
pub const SENSOR_DRIVE_POWERDOWN: i64 = 8;
/// `SENSOR_DRIVE_FAIL`.
pub const SENSOR_DRIVE_FAIL: i64 = 9;
/// `SENSOR_DRIVE_PFAIL`.
pub const SENSOR_DRIVE_PFAIL: i64 = 10;

/// `enum sensor_status`: sensor states.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)] // OpenBSD names, verbatim, for grep-ability
pub enum SensorStatus {
    /// status is unspecified
    SENSOR_S_UNSPEC = 0,
    /// status is ok
    SENSOR_S_OK,
    /// status is warning
    SENSOR_S_WARN,
    /// status is critical
    SENSOR_S_CRIT,
    /// status is unknown
    SENSOR_S_UNKNOWN,
}

pub use SensorStatus::*;

impl SensorStatus {
    /// Every state, in the C's order (index = value).
    pub const ALL: [SensorStatus; 5] = [
        SENSOR_S_UNSPEC,
        SENSOR_S_OK,
        SENSOR_S_WARN,
        SENSOR_S_CRIT,
        SENSOR_S_UNKNOWN,
    ];

    /// The state whose C value is `v`; `None` outside the enum.
    pub fn from_i32(v: i32) -> Option<Self> {
        usize::try_from(v)
            .ok()
            .and_then(|i| Self::ALL.get(i).copied())
    }
}

/// `SENSOR_FINVALID`: sensor is invalid.
pub const SENSOR_FINVALID: i32 = 0x0001;
/// `SENSOR_FUNKNOWN`: sensor value is unknown.
pub const SENSOR_FUNKNOWN: i32 = 0x0002;

/// `struct sensor`: sensor data as `sysctl(2)` returns it. New fields should be added at the
/// end to encourage backwards compat.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sensor {
    /// `desc`: sensor description, may be empty.
    pub desc: [u8; 32],
    /// `tv`: sensor value last change time.
    pub tv: IfTimeval,
    /// `value`: current value.
    pub value: i64,
    /// `type`: sensor type (an `enum sensor_type` value).
    pub r#type: i32,
    /// `status`: sensor status (an `enum sensor_status` value).
    pub status: i32,
    /// `numt`: sensor number of `.type` type.
    pub numt: i32,
    /// `flags`: sensor flags.
    pub flags: i32,
}

// `#[repr(C)]` of a byte array, a `struct timeval` (two `i64`s) and integers, laid out
// without holes (`abi_pod!` checks it).
crate::abi_pod!(Sensor {
    desc: [u8; 32],
    tv: IfTimeval,
    value: i64,
    r#type: i32,
    status: i32,
    numt: i32,
    flags: i32,
});

/// `struct sensordev`: sensor device data as `sysctl(2)` returns it. New fields should be
/// added at the end to encourage backwards compat.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sensordev {
    /// `num`: sensordev number.
    pub num: i32,
    /// `xname`: unix device name.
    pub xname: [u8; 16],
    /// `maxnumt`: per type, one past the highest `numt` attached.
    pub maxnumt: [i32; SENSOR_MAX_TYPES],
    /// `sensors_count`.
    pub sensors_count: i32,
}

// `#[repr(C)]` of `i32`s and a byte array, without holes (`abi_pod!` checks it).
crate::abi_pod!(Sensordev {
    num: i32,
    xname: [u8; 16],
    maxnumt: [i32; SENSOR_MAX_TYPES],
    sensors_count: i32,
});

const _: () = {
    assert!(core::mem::size_of::<Sensor>() == 32 + 16 + 8 + 4 * 4);
    assert!(core::mem::offset_of!(Sensor, tv) == 32);
    assert!(core::mem::offset_of!(Sensor, value) == 48);
    assert!(core::mem::size_of::<Sensordev>() == 4 + 16 + 4 * SENSOR_MAX_TYPES + 4);
    assert!(SENSOR_MAX_TYPES == 23);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensor_type_round_trip() {
        for (i, t) in SensorType::ALL.iter().enumerate() {
            assert_eq!(*t as usize, i);
            assert_eq!(SensorType::from_i32(i as i32), Some(*t));
        }
        assert_eq!(SensorType::from_i32(-1), None);
        assert_eq!(SensorType::from_i32(SENSOR_MAX_TYPES as i32), None);
        for (i, s) in SensorStatus::ALL.iter().enumerate() {
            assert_eq!(*s as usize, i);
            assert_eq!(SensorStatus::from_i32(i as i32), Some(*s));
        }
        assert_eq!(SensorStatus::from_i32(5), None);
    }

    #[test]
    fn user_layouts() {
        assert_eq!(core::mem::size_of::<Sensor>(), 72);
        assert_eq!(core::mem::size_of::<Sensordev>(), 116);
        assert_eq!(core::mem::offset_of!(Sensor, r#type), 56);
        assert_eq!(core::mem::offset_of!(Sensor, flags), 68);
        assert_eq!(core::mem::offset_of!(Sensordev, xname), 4);
        assert_eq!(core::mem::offset_of!(Sensordev, maxnumt), 20);
        assert_eq!(core::mem::offset_of!(Sensordev, sensors_count), 112);
    }
}
/* </TESTS> */
