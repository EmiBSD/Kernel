# debugger: lessons across runs

One line per lesson, appended at the end; task state goes in HANDOFF.md
(`.claude/rules/subagents.md`, "Memory"). Committed with the tree.
- amd64 kernel_map is NX at the directory level (pmap_alloc_level sets pg_nx on L4/L3/L2, as pmap.c:3041): a leaf entered R|X there still faults with code 0x11; arm64 sets no table PXN, so the same code runs there. Check the whole walk, not only the leaf.
- Page-walk trick: a temporary kprintf of each level's entry via pmap_find_pte_direct's loop (amd64 pmap) settles NX/present questions in one boot; revert it after.
- A smoke whose `--screenshot-after` line comes right before a panic fails with `monitor.sock: No such file or directory` (QEMU exited and removed its socket before xtask's 100 ms poll) and prints no transcript: suspect a crash, not the socket.
