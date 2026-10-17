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
 * bench: micro-benchmarks for `cargo xtask bench`.
 *
 * EmiBSD's own program, not OpenBSD code. The same static binary runs on
 * EmiBSD (from the ramdisk) and on a real OpenBSD VM under the same QEMU
 * configuration, so what it measures is the ratio between the two kernels,
 * not an absolute speed (under TCG absolute numbers say little). Each
 * measurement repeats a fixed amount of work, timed with CLOCK_MONOTONIC,
 * and prints one line:
 *
 *	@@BENCH <name> <ns per op> <ops>
 *
 * usage: bench all FILE	every measurement; FILE is the cold file
 *				`fileread` reads
 *        bench NAME [FILE]	one measurement
 *        bench true		exit 0 at once: what `forkexec` executes
 *
 * The program must be run by its absolute path: `forkexec` executes
 * argv[0] (OpenBSD keeps true(1) in /usr/bin, dynamically linked, and the
 * ramdisk has none, so the same static binary is the trivial program on
 * both systems).
 *
 * Measurements (an op is):
 *	pagefault	the first touch of one page of a fresh anonymous
 *			mapping (mmap and munmap of the mapping included,
 *			spread over its pages)
 *	forkexit	fork(2), the child's _exit(2), the parent's waitpid(2)
 *	forkexec	the same, the child executing `bench true` first
 *	pipe		a one-byte round trip between two processes over two
 *			pipes
 *	fileread	read(2) of 64 KiB of FILE, from its start to its end
 *			(the caller makes it cold: a fresh mount)
 */

#include <sys/types.h>
#include <sys/mman.h>
#include <sys/wait.h>

#include <err.h>
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

#define FAULT_PAGES	1024	/* pages per mapping */
#define FAULT_ROUNDS	32	/* mappings */
#define FORKEXIT_OPS	500
#define FORKEXEC_OPS	200
#define PIPE_OPS	5000
#define READ_CHUNK	(64 * 1024)

static const char *self;

static unsigned long long
now_ns(void)
{
	struct timespec ts;

	if (clock_gettime(CLOCK_MONOTONIC, &ts) == -1)
		err(1, "clock_gettime");
	return (unsigned long long)ts.tv_sec * 1000000000ULL +
	    (unsigned long long)ts.tv_nsec;
}

static void
report(const char *name, unsigned long long ns, unsigned long long ops)
{
	if (ops == 0)
		errx(1, "%s: no operations", name);
	printf("@@BENCH %s %llu %llu\n", name, ns / ops, ops);
	fflush(stdout);
}

/* One mapping of `pages` pages, each touched once, then unmapped. */
static void
fault_round(size_t pages, size_t pagesz)
{
	volatile char *p;
	size_t i;

	p = mmap(NULL, pages * pagesz, PROT_READ | PROT_WRITE,
	    MAP_ANON | MAP_PRIVATE, -1, 0);
	if (p == MAP_FAILED)
		err(1, "mmap");
	for (i = 0; i < pages; i++)
		p[i * pagesz] = (char)i;
	if (munmap((void *)p, pages * pagesz) == -1)
		err(1, "munmap");
}

static void
bench_pagefault(void)
{
	unsigned long long t0;
	size_t pagesz;
	int r;

	pagesz = (size_t)sysconf(_SC_PAGESIZE);
	fault_round(FAULT_PAGES, pagesz);	/* warm-up */
	t0 = now_ns();
	for (r = 0; r < FAULT_ROUNDS; r++)
		fault_round(FAULT_PAGES, pagesz);
	report("pagefault", now_ns() - t0,
	    (unsigned long long)FAULT_PAGES * FAULT_ROUNDS);
}

/* fork(2); the child execs `bench true` when `exec` is set, else exits. */
static void
fork_one(int exec)
{
	char *const args[] = { (char *)self, "true", NULL };
	pid_t pid;
	int status;

	pid = fork();
	if (pid == -1)
		err(1, "fork");
	if (pid == 0) {
		if (exec) {
			execv(self, args);
			_exit(127);
		}
		_exit(0);
	}
	while (waitpid(pid, &status, 0) == -1)
		if (errno != EINTR)
			err(1, "waitpid");
	if (!WIFEXITED(status) || WEXITSTATUS(status) != 0)
		errx(1, "child %d: status %#x", (int)pid, status);
}

static void
bench_fork(const char *name, int exec, int ops)
{
	unsigned long long t0;
	int i;

	for (i = 0; i < ops / 10; i++)	/* warm-up */
		fork_one(exec);
	t0 = now_ns();
	for (i = 0; i < ops; i++)
		fork_one(exec);
	report(name, now_ns() - t0, (unsigned long long)ops);
}

static void
bench_pipe(void)
{
	unsigned long long t0;
	int to_child[2], to_parent[2];
	int i, status, total;
	pid_t pid;
	char c;

	if (pipe(to_child) == -1 || pipe(to_parent) == -1)
		err(1, "pipe");
	total = PIPE_OPS + PIPE_OPS / 10;
	pid = fork();
	if (pid == -1)
		err(1, "fork");
	if (pid == 0) {
		close(to_child[1]);
		close(to_parent[0]);
		for (i = 0; i < total; i++) {
			if (read(to_child[0], &c, 1) != 1)
				_exit(1);
			if (write(to_parent[1], &c, 1) != 1)
				_exit(1);
		}
		_exit(0);
	}
	close(to_child[0]);
	close(to_parent[1]);
	c = 'x';
	t0 = 0;
	for (i = 0; i < total; i++) {
		if (i == PIPE_OPS / 10)	/* the round trips before are warm-up */
			t0 = now_ns();
		if (write(to_child[1], &c, 1) != 1)
			err(1, "pipe write");
		if (read(to_parent[0], &c, 1) != 1)
			err(1, "pipe read");
	}
	report("pipe", now_ns() - t0, PIPE_OPS);
	while (waitpid(pid, &status, 0) == -1)
		if (errno != EINTR)
			err(1, "waitpid");
	if (!WIFEXITED(status) || WEXITSTATUS(status) != 0)
		errx(1, "pipe child: status %#x", status);
	close(to_child[1]);
	close(to_parent[0]);
}

static void
bench_fileread(const char *path)
{
	unsigned long long t0, ops;
	ssize_t n;
	char *buf;
	int fd;

	if ((buf = malloc(READ_CHUNK)) == NULL)
		err(1, "malloc");
	if ((fd = open(path, O_RDONLY)) == -1)
		err(1, "%s", path);
	ops = 0;
	t0 = now_ns();
	while ((n = read(fd, buf, READ_CHUNK)) > 0)
		ops++;
	if (n == -1)
		err(1, "read %s", path);
	report("fileread", now_ns() - t0, ops);
	close(fd);
	free(buf);
}

static void __dead
usage(void)
{
	fprintf(stderr, "usage: bench all FILE | bench pagefault | "
	    "bench forkexit | bench forkexec | bench pipe | "
	    "bench fileread FILE | bench true\n");
	exit(2);
}

int
main(int argc, char *argv[])
{
	const char *what;

	if (argc < 2)
		usage();
	what = argv[1];
	if (strcmp(what, "true") == 0)
		return 0;
	self = argv[0];
	if (strchr(self, '/') == NULL)
		errx(2, "run me by my absolute path (forkexec executes it)");
	if (strcmp(what, "all") == 0) {
		if (argc != 3)
			usage();
		bench_pagefault();
		bench_fork("forkexit", 0, FORKEXIT_OPS);
		bench_fork("forkexec", 1, FORKEXEC_OPS);
		bench_pipe();
		bench_fileread(argv[2]);
	} else if (strcmp(what, "pagefault") == 0)
		bench_pagefault();
	else if (strcmp(what, "forkexit") == 0)
		bench_fork("forkexit", 0, FORKEXIT_OPS);
	else if (strcmp(what, "forkexec") == 0)
		bench_fork("forkexec", 1, FORKEXEC_OPS);
	else if (strcmp(what, "pipe") == 0)
		bench_pipe();
	else if (strcmp(what, "fileread") == 0 && argc == 3)
		bench_fileread(argv[2]);
	else
		usage();
	return 0;
}
