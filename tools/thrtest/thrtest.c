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
 * thrtest: __thrsleep(2) and __thrwakeup(2) for `just smoke-thrsleep`.
 *
 * EmiBSD's own test program, not OpenBSD code. It checks the calls'
 * return values (they return their error, errno is untouched), then
 * runs rounds of threads that create and join threads while they fight
 * over one FILE lock: libc's flockfile(3) sleeps in __thrsleep and
 * funlockfile(3) wakes in __thrwakeup when two threads contend, the
 * pattern that killed threaded programs with SIGSYS before the calls
 * were ported. Every check prints one line; the last is
 * "thrtest: all ok" or "thrtest: N failures".
 *
 * usage: thrtest [rounds]   (300 when not given)
 */

#include <sys/time.h>

#include <errno.h>
#include <pthread.h>
#include <sched.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

/* libc exports both; their prototypes are in libc's private thread_private.h. */
int	__thrsleep(const volatile void *, clockid_t, const struct timespec *,
	    void *, const int *);
int	__thrwakeup(const volatile void *, int);

#define LOCKED		1	/* _ATOMIC_LOCK_LOCKED on amd64 and arm64 */
#define UNLOCKED	0
#define SHARED		((const volatile void *)-1L)
#define NSLEEPERS	4
#define NWORKERS	4
#define NWRITES		50
#define TRIES		20000	/* 1 ms apart: 20 s before giving up */

static int failures;

static void
check(const char *what, int got, int want)
{
	if (got == want)
		printf("thrtest: %s ok\n", what);
	else {
		printf("thrtest: %s FAILED: %d (%s), want %d (%s)\n", what,
		    got, strerror(got), want, strerror(want));
		failures++;
	}
	fflush(stdout);
}

static void
pause_1ms(void)
{
	struct timespec ts = { 0, 1000000 };

	nanosleep(&ts, NULL);
}

static volatile int chan_one, chan_n;
static volatile int done;
static pthread_mutex_t done_mtx = PTHREAD_MUTEX_INITIALIZER;

struct sleeper {
	pthread_t		 t;
	const volatile void	*ident;
	int			 lock;
	int			 rv;
};

static void *
sleeper(void *arg)
{
	struct sleeper *s = arg;

	s->lock = LOCKED;
	s->rv = __thrsleep(s->ident, CLOCK_REALTIME, NULL, &s->lock, NULL);
	pthread_mutex_lock(&done_mtx);
	done++;
	pthread_mutex_unlock(&done_mtx);
	return NULL;
}

static int
done_count(void)
{
	int n;

	pthread_mutex_lock(&done_mtx);
	n = done;
	pthread_mutex_unlock(&done_mtx);
	return n;
}

static void
start(struct sleeper *s, const volatile void *ident)
{
	s->ident = ident;
	s->rv = -1;
	if (pthread_create(&s->t, NULL, sleeper, s) != 0) {
		printf("thrtest: pthread_create failed\n");
		exit(1);
	}
}

/* __thrwakeup(ident, 1) until it finds a sleeper: 0, or ESRCH after TRIES. */
static int
wake_one(const volatile void *ident)
{
	int i, r = ESRCH;

	for (i = 0; i < TRIES; i++) {
		if ((r = __thrwakeup(ident, 1)) != ESRCH)
			break;
		pause_1ms();
	}
	return r;
}

static void
calls(void)
{
	struct sleeper s[NSLEEPERS];
	struct timespec ts, t0, t1;
	int lock, abort_word, i, r, ok;

	check("thrsleep ident 0 EINVAL",
	    __thrsleep(NULL, CLOCK_REALTIME, NULL, NULL, NULL), EINVAL);
	check("thrwakeup ident 0 EINVAL", __thrwakeup(NULL, 1), EINVAL);
	check("thrwakeup no sleeper ESRCH", __thrwakeup(&chan_one, 1), ESRCH);
	check("thrwakeup -1 no sleeper 0", __thrwakeup(SHARED, 0), 0);

	ts.tv_sec = 0;
	ts.tv_nsec = 1000000000;
	check("thrsleep bad timespec EINVAL",
	    __thrsleep(&chan_one, CLOCK_REALTIME, &ts, NULL, NULL), EINVAL);

	/* a time already passed: the lock is still released */
	lock = LOCKED;
	ts.tv_sec = 0;
	ts.tv_nsec = 0;
	r = __thrsleep(&chan_one, CLOCK_REALTIME, &ts, &lock, NULL);
	check("thrsleep past EWOULDBLOCK", r, EWOULDBLOCK);
	check("thrsleep past unlocks", lock, UNLOCKED);

	/* a real timeout */
	clock_gettime(CLOCK_MONOTONIC, &t0);
	ts = t0;
	ts.tv_nsec += 50000000;
	if (ts.tv_nsec >= 1000000000) {
		ts.tv_sec++;
		ts.tv_nsec -= 1000000000;
	}
	r = __thrsleep(&chan_one, CLOCK_MONOTONIC, &ts, NULL, NULL);
	clock_gettime(CLOCK_MONOTONIC, &t1);
	check("thrsleep timeout EWOULDBLOCK", r, EWOULDBLOCK);
	ok = t1.tv_sec > ts.tv_sec ||
	    (t1.tv_sec == ts.tv_sec && t1.tv_nsec >= ts.tv_nsec);
	check("thrsleep timeout slept", ok, 1);
	check("thrwakeup after timeout ESRCH", __thrwakeup(&chan_one, 1), ESRCH);

	/* the abort word, read after the unlock */
	lock = LOCKED;
	abort_word = 1;
	r = __thrsleep(&chan_one, CLOCK_REALTIME, NULL, &lock, &abort_word);
	check("thrsleep abort EINTR", r, EINTR);
	check("thrsleep abort unlocks", lock, UNLOCKED);

	/* one sleeper, one wakeup */
	done = 0;
	start(&s[0], &chan_one);
	check("thrwakeup sleeper 0", wake_one(&chan_one), 0);
	pthread_join(s[0].t, NULL);
	check("thrsleep woken 0", s[0].rv, 0);
	check("thrsleep woken unlocked", s[0].lock, UNLOCKED);

	/* NSLEEPERS sleepers woken one at a time */
	done = 0;
	for (i = 0; i < NSLEEPERS; i++)
		start(&s[i], &chan_n);
	for (i = 0, ok = 1; i < NSLEEPERS; i++)
		if (wake_one(&chan_n) != 0)
			ok = 0;
	for (i = 0; i < NSLEEPERS; i++) {
		pthread_join(s[i].t, NULL);
		if (s[i].rv != 0)
			ok = 0;
	}
	check("thrwakeup n=1 each", ok, 1);
	check("thrwakeup n=1 none left ESRCH", __thrwakeup(&chan_n, 1), ESRCH);

	/* ident -1 wakes everyone; it answers 0 whether or not anyone slept */
	done = 0;
	for (i = 0; i < NSLEEPERS; i++)
		start(&s[i], SHARED);
	for (i = 0; i < TRIES && done_count() < NSLEEPERS; i++) {
		if (__thrwakeup(SHARED, 0) != 0)
			break;
		pause_1ms();
	}
	for (i = 0, ok = done_count() == NSLEEPERS; i < NSLEEPERS; i++) {
		pthread_join(s[i].t, NULL);
		if (s[i].rv != 0)
			ok = 0;
	}
	check("thrwakeup -1 broadcast", ok, 1);
}

static FILE *sink;
static pthread_mutex_t mtx = PTHREAD_MUTEX_INITIALIZER;
static long total;

static void *
child(void *arg)
{
	(void)arg;
	flockfile(sink);
	fputs("c", sink);
	funlockfile(sink);
	return NULL;
}

static void *
worker(void *arg)
{
	pthread_t c;
	int i, id = (int)(long)arg;

	for (i = 0; i < NWRITES; i++) {
		flockfile(sink);
		fprintf(sink, "worker %d line %d\n", id, i);
		if (i % 8 == 0)
			sched_yield();	/* hold the lock across a switch */
		funlockfile(sink);
		pthread_mutex_lock(&mtx);
		total++;
		pthread_mutex_unlock(&mtx);
	}
	if (pthread_create(&c, NULL, child, NULL) == 0)
		pthread_join(c, NULL);
	return NULL;
}

static void
stress(int rounds)
{
	pthread_t w[NWORKERS];
	char what[64];
	int r, i;

	if ((sink = fopen("/dev/null", "w")) == NULL) {
		printf("thrtest: /dev/null: %s\n", strerror(errno));
		exit(1);
	}
	for (r = 0; r < rounds; r++) {
		for (i = 0; i < NWORKERS; i++)
			if (pthread_create(&w[i], NULL, worker,
			    (void *)(long)i) != 0) {
				printf("thrtest: round %d: pthread_create failed\n",
				    r);
				exit(1);
			}
		for (i = 0; i < NWORKERS; i++)
			pthread_join(w[i], NULL);
		if ((r + 1) % 100 == 0) {
			printf("thrtest: %d rounds\n", r + 1);
			fflush(stdout);
		}
	}
	fclose(sink);
	snprintf(what, sizeof(what), "stress %d rounds", rounds);
	check(what, total == (long)rounds * NWORKERS * NWRITES, 1);
}

int
main(int argc, char *argv[])
{
	int rounds = 300;

	if (argc > 1)
		rounds = atoi(argv[1]);
	setvbuf(stdout, NULL, _IOLBF, 0);

	calls();
	stress(rounds);

	if (failures == 0)
		printf("thrtest: all ok\n");
	else
		printf("thrtest: %d failures\n", failures);
	return failures != 0;
}
