# debugger: lessons across runs

One line per lesson, appended at the end; task state goes in HANDOFF.md
(`.claude/rules/subagents.md`, "Memory"). Committed with the tree.
- amd64 kernel_map is NX at the directory level (pmap_alloc_level sets pg_nx on L4/L3/L2, as pmap.c:3041): a leaf entered R|X there still faults with code 0x11; arm64 sets no table PXN, so the same code runs there. Check the whole walk, not only the leaf.
- Page-walk trick: a temporary kprintf of each level's entry via pmap_find_pte_direct's loop (amd64 pmap) settles NX/present questions in one boot; revert it after.
- A smoke whose `--screenshot-after` line comes right before a panic fails with `monitor.sock: No such file or directory` (QEMU exited and removed its socket before xtask's 100 ms poll) and prints no transcript: suspect a crash, not the socket.
- Host profiling on the Mac without root: `xctrace record --template 'Time Profiler' --launch -- BIN`, `xctrace export --xpath '.../table[@schema="time-profile"]'`, then `atos -o BIN -l <load-addr> -i` per frame expands inlined frames (run `dsymutil` on a dev binary first or atos gives no lines); `sample <pid>` is function-level only. Rust v0 symbols need a demangler (no rustfilt; legacy mangling is gone from stable).
- Re-analysing a profile after `cargo build` changed the binary resolves garbage: atos maps addresses against the current file; record and analyse against the same build.
- bsd's legacy host tests ran VMMAP_DEBUG tree sanity on every map lock and uvm_pmr_assertvalid under cfg(test): time LZ-era uvm only with those cfgs turned back to their features. kernel::testutil's pool/malloc doubles leak a zeroed 4 KiB block per item: a timing loop needs recycling doubles registered first.
- N3 perf (decision 42): fork/exec/map cost is dominated by map.rs re-finding entries by key (`ent(&st.addr, key)`, ~12-15 tree searches per cloned entry) with hal::owned's per-node ownership check, plus MapState::owners() rebuilt per call; host fork70 8.4 -> 19.5 us vs LZ at 20035fd0.
