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
//! The kernel image. The boot path lives in `stand/` (boot(8)'s `bootarg` hand-off).

#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

#[cfg(target_os = "none")]
mod stand;

// The crate that classifies a trap from userland (decisions 56 and 58): named here, by the image
// alone, so the crate is linked; hal's assembly's `bl entry_user_trap` pulls its object in.
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
extern crate entry as _;

/// Host build of the kernel image: there is nothing to run on macOS or Linux.
#[cfg(not(target_os = "none"))]
fn main() {
    eprintln!(
        "bsd: this is a kernel image. Build it with `just build`, boot it with `just run-<arch>`."
    );
    std::process::exit(2);
}
/* </CODE> */
