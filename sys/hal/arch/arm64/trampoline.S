/*
 * Copyright (c) 2018 Mark Kettenis <kettenis@openbsd.org>
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
 * The trampoline vectors of trampoline.S (see exception.rs): the EL0 vector
 * tables, one per Spectre-BHB mitigation, that VBAR_EL1 holds while user
 * code runs, their way into the kernel's handlers and tramp_return, the way
 * out. Pulled in after exception.S, in the same assembly; its vempty and
 * vector macros are this file's own, so exception.S's are purged first.
 *
 * Not here: the switch of TTBR1_EL1 between the kernel's tables and the
 * trampoline's (pmap_tramp, which maps nothing of the kernel but this page),
 * whose pmap half is not configured (sys/hal/arch/arm64/pmap/backend.rs,
 * deviations): the kernel stays mapped while user code runs, and the vectors
 * keep the BHB mitigations and the vector switch; N4.8d adds it (the user's
 * decision of 2026-10-23). PSCI's tables are always assembled (NPSCI is 1 in
 * this kernel).
 */

	.equ	SMCCC_ARCH_WORKAROUND_3, 0x80003fff

	.purgem	vempty
	.purgem	vector

	.text

/*
 * The next page contains everything that needs to be "exposed" to
 * userland.  This is the exception vector table that contains the
 * entries for EL0, and the tail end of the EL0 exception handlers.
 * This code is deliberately laid out in a way that avoids leaking
 * kernel virtual addresses.
 */
	.align 12

.macro	spectre_bhb_none_early
.endm
.macro	spectre_bhb_none_late
.endm

.macro	spectre_bhb_loop_8_early
	spectre_bhb_loop 8
.endm
.macro	spectre_bhb_loop_8_late
.endm

.macro	spectre_bhb_loop_11_early
	spectre_bhb_loop 11
.endm
.macro	spectre_bhb_loop_11_late
.endm

.macro	spectre_bhb_loop_24_early
	spectre_bhb_loop 24
.endm
.macro	spectre_bhb_loop_24_late
.endm

.macro	spectre_bhb_loop_32_early
	spectre_bhb_loop 32
.endm
.macro	spectre_bhb_loop_32_late
.endm

.macro	spectre_bhb_loop_132_early
	spectre_bhb_loop 132
.endm
.macro	spectre_bhb_loop_132_late
.endm

.macro	spectre_bhb_loop, cnt
	mov x18, #\cnt
1:
	b . + 4
	subs x18, x18, #1
	b.ne 1b
	dsb nsh
	isb
.endm

.macro	spectre_bhb_psci_hvc_early
.endm
.macro	spectre_bhb_psci_hvc_late
	stp	x0, x1, [sp, #-16]!
	stp	x2, x3, [sp, #-16]!
	mov	w0, #SMCCC_ARCH_WORKAROUND_3
	hvc	#0
	ldp	x2, x3, [sp], #16
	ldp	x0, x1, [sp], #16
.endm

.macro	spectre_bhb_psci_smc_early
.endm
.macro	spectre_bhb_psci_smc_late
	stp	x0, x1, [sp, #-16]!
	stp	x2, x3, [sp, #-16]!
	mov	w0, #SMCCC_ARCH_WORKAROUND_3
	smc	#0
	ldp	x2, x3, [sp], #16
	ldp	x0, x1, [sp], #16
.endm

/* clrbhb is hint #22: the hint form assembles whatever the target's features. */
.macro	spectre_bhb_clrbhb_early
	hint	#22
	isb
.endm
.macro	spectre_bhb_clrbhb_late
.endm

.macro	vempty
	.align 7
	brk	0xfff
	1: b	1b
.endm

/* The C's vector switches TTBR1_EL1 here, between the two halves (see above). */
.macro	vector	name, bhb
	.align 7
	msr	tpidrro_el0, x18
	spectre_bhb_\bhb\()_early
	spectre_bhb_\bhb\()_late
	b	tramp_\name
.endm

.macro	tramp_vector, bhb
	.align 11
	.globl trampoline_vectors_\bhb
trampoline_vectors_\bhb:
	vempty			/* Synchronous EL1t */
	vempty			/* IRQ EL1t */
	vempty			/* FIQ EL1t */
	vempty			/* Error EL1t */

	vempty			/* Synchronous EL1h */
	vempty			/* IRQ EL1h */
	vempty			/* FIQ EL1h */
	vempty			/* Error EL1h */

	vector el0_sync, \bhb	/* Synchronous 64-bit EL0 */
	vector el0_irq, \bhb	/* IRQ 64-bit EL0 */
	vector el0_fiq, \bhb	/* FIQ 64-bit EL0 */
	vector el0_error, \bhb	/* Error 64-bit EL0 */

	vempty			/* Synchronous 32-bit EL0 */
	vempty			/* IRQ 32-bit EL0 */
	vempty			/* FIQ 32-bit EL0 */
	vempty			/* Error 32-bit EL0 */
.endm

	.align 11
	.globl trampoline_vectors
trampoline_vectors:
	tramp_vector none
	tramp_vector loop_8
	tramp_vector loop_11
	tramp_vector loop_24
	tramp_vector loop_32
	tramp_vector loop_132
	tramp_vector psci_hvc
	tramp_vector psci_smc
	tramp_vector clrbhb

/* The C switches TTBR1_EL1 back to the trampoline's tables first (see above). */
	.align 11
	.globl tramp_return
tramp_return:
	mrs	x18, tpidrro_el0
	msr	tpidrro_el0, xzr
	eret
	dsb nsh
	isb

	.global trampoline_vectors_end
trampoline_vectors_end:

/*
 * The next page contains the start of the EL0 exception handlers.
 * This page is not "exposed" to userland, but should immediately
 * follow the page with the EL0 exception vector table such that
 * relative branches don't give away anything about the layout of our
 * kernel.
 */
	.align 12

/* exception_vectors by adrp/add, x18 being the one free register here. */
.macro	tramp_enter
	adrp	x18, exception_vectors
	add	x18, x18, :lo12:exception_vectors
	msr	vbar_el1, x18
	isb
	mrs	x18, tpidrro_el0
	msr	tpidrro_el0, xzr
.endm

tramp_el0_sync:
	tramp_enter
	b	handle_el0_sync

tramp_el0_irq:
	tramp_enter
	b	handle_el0_irq

tramp_el0_fiq:
	tramp_enter
	b	handle_el0_fiq

tramp_el0_error:
	tramp_enter
	b	handle_el0_error
