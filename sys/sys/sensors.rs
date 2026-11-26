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
//! `<sys/sensors.h>`: hardware sensors (`hw.sensors`).
//!
//! Upstream: sys/sys/sensors.h @ 3ce1f3f79392
//!
//! Status: `ported`. Since N7.1b the user structures, types and flags are `sysdefs::dev::sensors`'s
//! and `struct ksensor`, `struct ksensordev` and `struct sensor_task` with their functions
//! `kernel::dev::sensors`'s, re-exported here for the drivers still in `bsd`
//! (`docs/SCAFFOLDING.md`).

pub use kernel::dev::sensors::{
    Ksensor, Ksensordev, SensorTask, SensorTaskRef, StatusCell, TimevalCell, TypeCell,
};
pub use sysdefs::dev::sensors::*;
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use kernel::dev::sensors::{
        sensor_attach, sensor_detach, sensordev_deinstall, sensordev_get, sensordev_install,
        sensors_lock,
    };

    use super::*;
    use crate::net::if_::tests::zeroed_static;

    #[test]
    fn zeroed_records_attach_install_and_detach() {
        kernel::testutil::install();
        // SAFETY: `Ksensordev` and `Ksensor` are valid all zero (their members are atomics,
        // hal's `PinEntry` and `PinList`, each 0 when empty), as the zeroed softcs that embed
        // them rely on.
        let sd: &'static Ksensordev = unsafe { zeroed_static() };
        // SAFETY: as above.
        let s: &'static Ksensor = unsafe { zeroed_static() };
        assert_eq!(s.r#type.get(), SENSOR_TEMP);
        assert_eq!(s.status.get(), SENSOR_S_UNSPEC);
        s.r#type.set(SENSOR_INTEGER);
        sensor_attach(sd, s);
        assert!(s.is_attached());
        sensordev_install(sd);
        let num = sd.num.get();
        assert!(core::ptr::eq(
            sensordev_get(&sensors_lock(), num).unwrap(),
            sd
        ));
        assert_eq!(sd.maxnumt[SENSOR_INTEGER as usize].get(), 1);
        s.value.set(42);
        assert_eq!(s.value.get(), 42);
        sensordev_deinstall(sd);
        sensor_detach(sd, s);
        assert!(!s.is_attached());
        assert!(!sd.is_installed());
    }
}
/* </TESTS> */
