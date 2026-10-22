# Security policy

EmiBSD is an experiment. It runs only in QEMU, and its crypto, network and storage code is
unaudited. Do not use it to protect real data or real networks. Security reports are still
welcome: OpenBSD's security properties are meant to survive the redesign, and a report that
shows one did not is a bug that matters.

## Supported versions

Only `main`. The `n<N>-<name>` tags mark closed milestones; they get no fixes of their own.

## Reporting a vulnerability

Report it privately, through GitHub:
[Report a vulnerability](https://github.com/EmiBSD/Kernel/security/advisories/new)
(the repository's **Security** tab, **Report a vulnerability**). If you cannot use GitHub,
write to enavarre@outlook.com with `EmiBSD security` in the subject.

Do not open a public issue for a vulnerability.

A useful report gives:

- the commit (`git rev-parse HEAD`) and the architecture (arm64; amd64 is frozen until
  milestone N10, see [docs/ROADMAP.md](../docs/ROADMAP.md));
- how it is reached: the system call, ioctl, sysctl, packet or file system image, and from
  which privilege (root, an unprivileged user, a pledged or unveiled process, the network);
- what happens: the panic message or serial output, or what a process can read or do that it
  should not;
- a reproducer, if you have one: a program or script to run inside the guest, or the QEMU
  command line.

## What happens next

One person maintains EmiBSD, so answers are best effort. The report is confirmed, fixed on
`main`, and credited in the fix's commit and the advisory unless you ask otherwise.

Most of the kernel derives from OpenBSD through [EmiBSD.LZ](https://github.com/EmiBSD/LandingZone),
the file-by-file port. If the bug is also in OpenBSD itself, please report it to OpenBSD as
well (security@openbsd.org); EmiBSD is not affiliated with the OpenBSD project.

## Scope

In scope: everything in this repository that runs in the kernel or the boot loaders, and any
way the kernel lets userland cross a boundary that OpenBSD enforces (privilege, `pledge(2)`,
`unveil(2)`, memory protection, the network stack, pf, IPsec, WireGuard, softraid CRYPTO).

Out of scope: OpenBSD's userland as shipped in the ramdisk (report those to OpenBSD), QEMU,
the firmware, and the host tooling under `tools/`, unless it puts the host at risk.
