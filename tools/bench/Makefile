# EmiBSD's own benchmark program (see bench.c): an OpenBSD-style Makefile that
# tools/xtask/src/userland.rs evaluates like those of the reference tree.
# `cargo xtask bench` runs the same static binary on EmiBSD (from the ramdisk)
# and on a real OpenBSD VM (fetched over HTTP).

PROG=	bench
BINDIR=	/usr/bin
LDSTATIC=	-static
NOMAN=	yes

.include <bsd.prog.mk>
