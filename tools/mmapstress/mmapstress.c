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
 * mmapstress: the vnode pager under two CPUs.
 *
 * EmiBSD's own test program, not OpenBSD code. A file of NPAGES pages is
 * mapped MAP_SHARED by a process and, after fork(2), by its child: the
 * parent dirties every page and msync(2)s the range with MS_SYNC (the
 * vnode pager's cluster write: a multi-page pager window, the object's lock
 * let go across VOP_WRITE) while the child madvise(2)s the same range
 * MADV_DONTNEED in a loop and reads it back (uvn_flush's PGO_DEACTIVATE:
 * pmap_page_protect(PROT_NONE) on pages the pager may hold busy in its
 * window, then faults). Each round both check their resident set size
 * (kinfo_proc's p_vm_rssize, what ps(1) prints) is not negative. At the end
 * the mapping and the file read back with read(2) must hold the last
 * round's pattern.
 *
 * usage: mmapstress file [rounds]
 * prints "mmapstress: ok <file> rounds=N" or "mmapstress: FAILED ..."
 * and exits 0 or 1.
 */

#include <sys/types.h>
#include <sys/mman.h>
#include <sys/sysctl.h>
#include <sys/wait.h>

#include <err.h>
#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#define NPAGES	16
#define WORDS	4		/* words written per page and round */

static long pagesz;

/* The pattern of word w of page p in round r. */
static uint32_t
pattern(int r, int p, int w)
{
	return (uint32_t)r * 0x10001u + (uint32_t)p * 0x101u + (uint32_t)w;
}

/* The offset in words of word w of a page. */
static size_t
wordoff(int w)
{
	return (size_t)w * (pagesz / sizeof(uint32_t) / WORDS);
}

/* The resident set size of process pid, in pages, or -1 if unknown. */
static int
rss_of(pid_t pid, long *rss)
{
	struct kinfo_proc kp;
	size_t len = sizeof(kp);
	int mib[6] = { CTL_KERN, KERN_PROC, KERN_PROC_PID, (int)pid,
	    sizeof(struct kinfo_proc), 1 };

	if (sysctl(mib, 6, &kp, &len, NULL, 0) == -1 || len != sizeof(kp))
		return -1;
	*rss = kp.p_vm_rssize;
	return 0;
}

/* Checks pid's rss; returns 1 when it is negative. */
static int
rss_negative(pid_t pid, const char *who)
{
	long rss;

	if (rss_of(pid, &rss) == 0 && rss < 0) {
		printf("mmapstress: FAILED %s rss %ld\n", who, rss);
		fflush(stdout);
		return 1;
	}
	return 0;
}

int
main(int argc, char *argv[])
{
	volatile int *done;
	uint32_t *m;
	char *buf;
	size_t len;
	pid_t child;
	int fd, r, p, w, rounds, status, bad = 0;

	if (argc < 2 || argc > 3) {
		fprintf(stderr, "usage: mmapstress file [rounds]\n");
		return 2;
	}
	rounds = argc == 3 ? atoi(argv[2]) : 200;
	pagesz = sysconf(_SC_PAGESIZE);
	len = (size_t)pagesz * NPAGES;

	if ((fd = open(argv[1], O_RDWR | O_CREAT | O_TRUNC, 0644)) == -1)
		err(1, "open %s", argv[1]);
	if (ftruncate(fd, (off_t)len) == -1)
		err(1, "ftruncate");
	m = mmap(NULL, len, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
	if (m == MAP_FAILED)
		err(1, "mmap");
	done = mmap(NULL, pagesz, PROT_READ | PROT_WRITE,
	    MAP_SHARED | MAP_ANON, -1, 0);
	if (done == MAP_FAILED)
		err(1, "mmap anon");
	*done = 0;

	switch ((child = fork())) {
	case -1:
		err(1, "fork");
	case 0:
		/* Deactivate and fault the pages back while the parent syncs. */
		for (r = 0; !*done; r++) {
			volatile uint32_t sum = 0;

			if (madvise(m, len, MADV_DONTNEED) == -1)
				err(1, "madvise");
			for (p = 0; p < NPAGES; p++)
				sum += m[p * (pagesz / sizeof(uint32_t))];
			(void)sum;
			if (rss_negative(getpid(), "child"))
				_exit(1);
		}
		_exit(0);
	}

	for (r = 1; r <= rounds && !bad; r++) {
		for (p = 0; p < NPAGES; p++)
			for (w = 0; w < WORDS; w++)
				m[p * (pagesz / sizeof(uint32_t)) + wordoff(w)] =
				    pattern(r, p, w);
		if (msync(m, len, MS_SYNC) == -1) {
			printf("mmapstress: FAILED msync round %d: %s\n", r,
			    strerror(errno));
			bad = 1;
		}
		bad |= rss_negative(getpid(), "parent");
		bad |= rss_negative(child, "child");
	}
	*done = 1;
	if (waitpid(child, &status, 0) == -1)
		err(1, "waitpid");
	if (!WIFEXITED(status) || WEXITSTATUS(status) != 0) {
		printf("mmapstress: FAILED child status %#x\n", status);
		bad = 1;
	}
	rounds = r - 1;

	/* The mapping, then the file, hold the last round's pattern. */
	for (p = 0; p < NPAGES && !bad; p++)
		for (w = 0; w < WORDS; w++)
			if (m[p * (pagesz / sizeof(uint32_t)) + wordoff(w)] !=
			    pattern(rounds, p, w)) {
				printf("mmapstress: FAILED mapping page %d "
				    "word %d\n", p, w);
				bad = 1;
				break;
			}
	if (munmap(m, len) == -1)
		err(1, "munmap");
	if ((buf = malloc(len)) == NULL)
		err(1, "malloc");
	if (pread(fd, buf, len, 0) != (ssize_t)len)
		err(1, "pread");
	for (p = 0; p < NPAGES && !bad; p++)
		for (w = 0; w < WORDS; w++) {
			uint32_t v;

			memcpy(&v, buf + (size_t)p * pagesz +
			    wordoff(w) * sizeof(uint32_t), sizeof(v));
			if (v != pattern(rounds, p, w)) {
				printf("mmapstress: FAILED file page %d "
				    "word %d\n", p, w);
				bad = 1;
				break;
			}
		}
	close(fd);

	if (bad)
		return 1;
	printf("mmapstress: ok %s rounds=%d\n", argv[1], rounds);
	return 0;
}
