/*
 * Copyright (c) 2015 Dale Rahn <drahn@dalerahn.com>
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

	.text

/*
 * hal_cpu_switchto(struct machine_slot *oldslot, struct machine_slot *newslot,
 *     struct proc *newproc)
 * 	x0	the old thread's slot (its p_stat, p_cpu, p_addr), 0 when the
 *		old context is not saved (the thread is dead, or none ran)
 * 	x1	the new thread's slot
 *	x2	the new thread itself, what ci_curproc holds
 *      x3	- may contain switchframe pointer (new or old)
 *      x4	- may contain curcpu
 *	x5	- may contain PCB pointer (new or old)
 *	x6	- may contain user thread pointer (TCB)
 *
 * The C's cpu_switchto(oldproc, newproc) with the slot and the thread apart:
 * the slot's members are where the thread's struct proc had them (the thread
 * keeps its slot first), but hal reaches them through the slot it was given.
 * curcpu is TPIDR_EL1: the cpu_info, whose first member is the CpuCore whose
 * offsets CI_CURPCB and CI_CURPROC are.
 * The retguard cookie dance (x20/x15) is not here: see switch.rs.
 */
	.globl	hal_cpu_switchto
	.type	hal_cpu_switchto, %function
	.p2align 2
hal_cpu_switchto:
	// check if old context needs to be saved
	cmp	x0, #0
	beq	1f

	// create switchframe
	sub	sp, sp, #({SWITCHFRAME_SZ})
	mov	x3, sp
	stp	x19, x20,  [x3, #({SF_X19})]
	stp	x21, x22,  [x3, #({SF_X21})]
	stp	x23, x24,  [x3, #({SF_X23})]
	stp	x25, x26,  [x3, #({SF_X25})]
	stp	x27, x28,  [x3, #({SF_X27})]
	stp	x29, x30,  [x3, #({SF_X29})]

	mrs	x4, tpidr_el1			// load curcpu
	ldr	x5, [x4, #({CI_CURPCB})]
	str	x3, [x5, #({PCB_SP})]		// save to old pcb

1:
	mrs	x4, tpidr_el1			// load curcpu
	mov	w5, #{SONPROC}
	strb	w5, [x1, #({P_STAT})]		// Mark new on cpu
	str	x4, [x1, #({P_CPU})]
	ldr	x5, [x1, #({P_ADDR})]		// load new pcb
	str	x5, [x4, #({CI_CURPCB})]
	str	x2, [x4, #({CI_CURPROC})]

	ldr	x6, [x5, #({PCB_TCB})]
	msr	tpidr_el0, x6			// load user tls

	ldr	x19, [x5, #({PCB_SP})]		// load new stack pointer
	mov	x0, x2
	bl	pmap_setttb
	mov	x3, x19

	mov	sp, x3

	ldp	x19, x20,  [x3, #({SF_X19})]
	ldp	x21, x22,  [x3, #({SF_X21})]
	ldp	x23, x24,  [x3, #({SF_X23})]
	ldp	x25, x26,  [x3, #({SF_X25})]
	ldp	x27, x28,  [x3, #({SF_X27})]
	ldp	x29, x30,  [x3, #({SF_X29})]
	add	sp, sp, #({SWITCHFRAME_SZ})
	ret
	.size	hal_cpu_switchto, . - hal_cpu_switchto

/*
 * hal_proc_trampoline: where a thread built by hal::thread::thread_setup
 * starts, with the switch frame's sf_x19 (its start word) and sf_x20 (its
 * argument) restored. The Rust side (switch.rs's trampoline_run) adopts the
 * scheduler lock the switch kept and calls the registered start; when that
 * returns the thread leaves for user mode through syscall_return.
 */
	.globl	hal_proc_trampoline
	.type	hal_proc_trampoline, %function
	.p2align 2
hal_proc_trampoline:
	mov	x0, x19
	mov	x1, x20
	bl	{TRAMPOLINE_RUN}
	b	syscall_return
	.size	hal_proc_trampoline, . - hal_proc_trampoline
