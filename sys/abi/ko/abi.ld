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
 * abi-openbsd.ko, both architectures: a static position-independent ELF linked at 0,
 * which hal::module (sys/hal/module/load.rs) places, relocates and maps, segment by
 * segment, each on its own pages:
 *
 *   text    R X  the code;
 *   rodata  R    the module header first (the ELF entry address names it), then the
 *                read-only data, the GOT and .data.rel.ro, the dynamic section and the
 *                RELA relocations: the loader relocates them through its writable copy
 *                before it maps the pages read-only (hence -z notext in build.rs);
 *   data    R W  the data and the bss.
 *
 * Every segment starts on a page, so no page needs two protections; the loader refuses a
 * segment that does not, and one that is both writable and executable.
 */
ENTRY(abi_module_header)
PHDRS
{
	text PT_LOAD FLAGS(5);
	rodata PT_LOAD FLAGS(4);
	data PT_LOAD FLAGS(6);
	dynamic PT_DYNAMIC FLAGS(4);
}
SECTIONS
{
	. = 0;
	.text : {
		*(.text .text.*)
	} :text

	. = ALIGN(4096);
	.abi_module : {
		abi_module_header = .;
		KEEP(*(.data.rel.ro.*ABI_OPENBSD_MODULE_HEADER*))
		KEEP(*(.rodata.*ABI_OPENBSD_MODULE_HEADER*))
		KEEP(*(.data.*ABI_OPENBSD_MODULE_HEADER*))
	} :rodata
	.rodata : {
		*(.rodata .rodata.*)
		*(.data.rel.ro .data.rel.ro.*)
		*(.got .got.plt)
	} :rodata
	.dynamic : { *(.dynamic) } :rodata :dynamic
	.rela.dyn : { *(.rela.dyn .rela.*) } :rodata
	.dynsym : { *(.dynsym) } :rodata
	.dynstr : { *(.dynstr) } :rodata
	.hash : { *(.hash) } :rodata
	.gnu.hash : { *(.gnu.hash) } :rodata

	. = ALIGN(4096);
	.data : {
		*(.data .data.*)
	} :data
	.bss : {
		*(.bss .bss.*)
		*(COMMON)
	} :data

	/DISCARD/ : {
		*(.eh_frame*)
		*(.note .note.*)
		*(.comment)
		*(.interp)
	}
}
