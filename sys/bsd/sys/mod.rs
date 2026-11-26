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
//! Kernel-wide types and constants: OpenBSD `sys/sys/*.h`.
//!
//! Each header becomes one module here (`errno.h` → `errno.rs`, `proc.h` → `proc.rs`). Functions
//! that the corresponding `.c` file implements live in that file's module (`kern/`), as `impl`
//! blocks on the types defined here.

#[forbid(unsafe_code)]
pub mod _time;
pub use ::kernel::proc::acct;
pub mod ataio;
pub mod audioio;
pub mod buf;
pub mod cdio;
#[forbid(unsafe_code)]
pub mod clockintr;
#[forbid(unsafe_code)]
pub mod conf;
pub mod device;
pub use sysdefs::file::dirent;
pub mod disk;
pub mod disklabel;
#[forbid(unsafe_code)]
pub mod dkio;
#[forbid(unsafe_code)]
pub mod domain;
pub use sysdefs::base::endian;
pub use sysdefs::base::errno;
pub mod evcount;
#[forbid(unsafe_code)]
pub mod event;
#[forbid(unsafe_code)]
pub mod eventvar;
#[forbid(unsafe_code)]
pub mod exec;
pub use sysdefs::proc::exec_elf;
#[forbid(unsafe_code)]
pub mod exec_script;
#[forbid(unsafe_code)]
pub mod extent;
pub use sysdefs::file::fcntl;
#[forbid(unsafe_code)]
pub mod file;
#[forbid(unsafe_code)]
pub mod filedesc;
pub use sysdefs::file::filio;
pub use sysdefs::proc::futex;
pub mod gpio;
pub use sysdefs::base::ioccom;
pub use sysdefs::dev::intrmap;
pub mod ioctl;
pub use sysdefs::base::kernel;
pub use sysdefs::base::limits;
#[forbid(unsafe_code)]
pub mod lock;
#[forbid(unsafe_code)]
pub mod lockf;
pub mod malloc;
pub mod mbuf;
pub use sysdefs::vm::mman;
pub mod mount;
#[forbid(unsafe_code)]
pub mod mplock;
#[forbid(unsafe_code)]
pub mod mtio;
#[forbid(unsafe_code)]
pub mod mutex;
pub mod namei;
#[forbid(unsafe_code)]
pub mod param;
pub use sysdefs::time::pclock;
pub mod percpu;
pub use sysdefs::proc::pledge;
#[forbid(unsafe_code)]
pub mod poll;
pub mod pool;
pub mod proc;
pub mod protosw;
pub mod queue;
pub use sysdefs::proc::reboot;
pub mod refcnt;
pub mod resource;
pub mod resourcevar;
pub mod rwlock;
pub mod sched;
pub mod scsiio;
pub use sysdefs::file::select;
#[forbid(unsafe_code)]
pub mod selinfo;
pub mod sensors;
pub use sysdefs::sig::siginfo;
#[forbid(unsafe_code)]
pub mod sigio;
pub use sysdefs::sig::signal;
#[forbid(unsafe_code)]
pub mod signalvar;
pub mod smr;
pub mod socket;
pub mod socketvar;
#[forbid(unsafe_code)]
pub mod sockio;
pub use sysdefs::dev::softintr;
#[forbid(unsafe_code)]
pub mod specdev;
pub use sysdefs::file::stat;
pub mod staticcell;
#[forbid(unsafe_code)]
pub mod swap;
pub use sysdefs::proc::syscall;
#[forbid(unsafe_code)]
pub mod syscall_mi;
pub mod syscallargs;
#[forbid(unsafe_code)]
pub mod sysctl;
pub use sysdefs::base::syslimits;
#[forbid(unsafe_code)]
pub mod syslog;
pub mod systm;
pub mod task;
#[forbid(unsafe_code)]
pub mod termios;
#[forbid(unsafe_code)]
pub mod time;
#[forbid(unsafe_code)]
pub mod timeout;
pub mod timetc;
pub mod tree;
#[forbid(unsafe_code)]
pub mod tty;
#[forbid(unsafe_code)]
pub mod ttycom;
#[forbid(unsafe_code)]
pub mod ttydefaults;
pub use sysdefs::base::types;
#[forbid(unsafe_code)]
pub mod ucred;

#[forbid(unsafe_code)]
pub mod uio;
pub use sysdefs::net::un;
pub use sysdefs::proc::unistd;
#[forbid(unsafe_code)]
pub mod user;
pub use sysdefs::base::uuid;
pub use sysdefs::vm::vmmeter;
pub mod vnode;
pub use sysdefs::proc::wait;
/* </CODE> */
