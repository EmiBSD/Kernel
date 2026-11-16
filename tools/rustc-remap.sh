#!/bin/sh
#
# Copyright (c) 2026 Emilio Navarrete Lineros <enavarre@outlook.com>
#
# Permission to use, copy, modify, and distribute this software for any
# purpose with or without fee is hereby granted, provided that the above
# copyright notice and this permission notice appear in all copies.
#
# THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
# WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
# MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
# ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
# WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
# ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
# OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
#
# RUSTC_WRAPPER for every build the justfile runs: cargo calls it as
# `rustc-remap.sh <rustc> <args...>`. It runs DevTools' rustc with the same arguments plus
# two path remappings, so the paths of the build machine never land in a binary (panic
# locations, debug info): DevTools' toolchain directory (its `core`, `alloc` and the rest of
# rust-src) becomes /emibsd/toolchains, and cargo's registry under the builder's home becomes
# /cargo. The tree's own sources are already relative (cargo passes workspace paths that way).
# Additive: RUSTFLAGS and .cargo/config.toml's rustflags reach rustc as before.
# Userland's report of 2026-11-15: a release's BOOTAA64.EFI carried the builder's home path.

: "${EMIBSD_TOOLCHAIN_DIR:?rustc-remap.sh: EMIBSD_TOOLCHAIN_DIR is not set (run through just)}"
exec "$@" \
    "--remap-path-prefix=${EMIBSD_TOOLCHAIN_DIR}=/emibsd/toolchains" \
    "--remap-path-prefix=${HOME}/.cargo=/cargo"
