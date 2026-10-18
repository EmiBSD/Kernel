# EmiBSD's own test program (see mmapstress.c): an OpenBSD-style Makefile that
# tools/xtask/src/userland.rs evaluates like those of the reference tree.
# `just smoke-mmap` runs it on the ramdisk, on tmpfs and on the root FFS.

PROG=	mmapstress
BINDIR=	/usr/bin
LDSTATIC=	-static
NOMAN=	yes

.include <bsd.prog.mk>
