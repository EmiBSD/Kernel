/*
 * EmiBSD arm64 linker script. The same ELF boots from Limine and from OpenBSD's efiboot
 * (M14, track A3). The kernel is linked at 0xffffffff80000000 (Limine's higher half, which
 * arm64/pmap.rs adopts) with physical addresses from 0 (AT()): efiboot's LOADADDR keeps
 * the low 39 bits of p_paddr and adds its 64 MB block's address (efi_loadaddr), so the
 * image lands at the block's start. The ELF entry is the physical offset of locore0.S's
 * `_start` for the same reason (e_entry goes through LOADADDR too); Limine enters the
 * boot glue's `_start` (sys/stand/mod.rs) through its entry point request.
 *
 * The section bounds `__text_start`, `__rodata_start`, `_erodata` and `__data_start` are
 * OpenBSD's (kern.ldscript): hal's pmap_bootstrap maps the image by section with them
 * (text read-only executable, rodata read-only, the rest read-write), as pmap_map_stolen
 * does. They are symbols only: no section or segment moves.
 */
OUTPUT_FORMAT(elf64-littleaarch64)
ENTRY(__start_phys)

PHDRS
{
    text    PT_LOAD;
    rodata  PT_LOAD;
    data    PT_LOAD;
    /* boot(8) fills this segment with random data (loadfile_elf's PT_OPENBSD_RANDOMIZE). */
    openbsd_randomize PT_OPENBSD_RANDOMIZE;
}

__kernel_base = 0xffffffff80000000;

SECTIONS
{
    /* Higher half, where Limine loads non-relocatable kernels; physical = virtual - base. */
    . = __kernel_base;
    PROVIDE(__text_start = .);

    .text : AT(ADDR(.text) - __kernel_base) {
        /* locore0.S's _start first: efiboot cleans the cache from the entry point on. */
        KEEP(*(.text.locore0))
        *(.text .text.*)
    } :text

    PROVIDE(_etext = .);
    PROVIDE(etext = .);

    . = ALIGN(CONSTANT(MAXPAGESIZE));
    PROVIDE(__rodata_start = .);

    .rodata : AT(ADDR(.rodata) - __kernel_base) {
        *(.rodata .rodata.*)
    } :rodata

    /* dev/rnd.c's boot seed (entropy_pool0, rs_buf0) and init_main.c's __guard_local, which
     * boot(8) fills; read-only, inside the rodata segment, as in OpenBSD's kern.ldscript. */
    . = ALIGN(0x1000);
    .openbsd.randomdata : AT(ADDR(.openbsd.randomdata) - __kernel_base) {
        *(.openbsd.randomdata .openbsd.randomdata.*)
    } :openbsd_randomize :rodata
    . = ALIGN(0x1000);
    PROVIDE(_erodata = .);

    . = ALIGN(CONSTANT(MAXPAGESIZE));
    PROVIDE(__data_start = .);

    .data : AT(ADDR(.data) - __kernel_base) {
        *(.data .data.*)

        /* Limine requests: kept even though nothing references them by symbol. */
        KEEP(*(.requests_start_marker))
        KEEP(*(.requests))
        KEEP(*(.requests_end_marker))
    } :data

    .bss : AT(ADDR(.bss) - __kernel_base) {
        . = ALIGN(16);
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(16);
    } :data

    /* locore0.S zeroes __bss_start .. _end; locore.S's esym starts at end. hal's
     * pmap_bootstrap maps the image up to the page holding _end and nothing after it: an
     * allocated section placed after .bss (an orphan lld would put there) would be unmapped.
     * Only .text, .rodata, .openbsd.randomdata, .data and .bss exist today (llvm-objdump);
     * a new one goes inside one of the segments above. */
    _end = .;
    PROVIDE(end = .);

    /DISCARD/ : {
        *(.eh_frame*)
        *(.note .note.*)
    }
}

/* The section bounds the pmap maps by (see above) are where the sections are. */
ASSERT(__rodata_start == ADDR(.rodata), "__rodata_start is not .rodata's start")
ASSERT(_erodata >= ADDR(.openbsd.randomdata) + SIZEOF(.openbsd.randomdata),
    "_erodata is before the end of the read-only data")
ASSERT(__data_start == ADDR(.data) && _erodata <= __data_start,
    "__data_start is not .data's start")
ASSERT(_end == ADDR(.bss) + SIZEOF(.bss), "_end is not the end of .bss")

/* The ELF entry: _start's offset in the image (see above). */
__start_phys = _start - __kernel_base;
