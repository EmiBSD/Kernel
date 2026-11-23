# EmiBSD's own test program (see thrtest.c): an OpenBSD-style Makefile that
# tools/xtask/src/userland.rs evaluates like those of the reference tree.
# `just smoke-thrsleep` runs it from the ramdisk.

PROG=	thrtest
BINDIR=	/usr/bin
LDSTATIC=	-static
NOMAN=	yes
DPADD+=	${LIBPTHREAD}
LDADD+=	-lpthread

.include <bsd.prog.mk>
