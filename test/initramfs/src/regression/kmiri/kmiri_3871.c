// SPDX-License-Identifier: MPL-2.0

// Userspace observer for asterinas#3871 (x86 BSP `LOCAL_TSS` is an immutable Rust static, but
// `syscall.S` writes TSS.sp0/sp1 into it on every return to / entry from userspace).
//
// Nothing here is a special trigger: any syscall performs the write. We read `/proc/kmiri_3871`
// (itself an ordinary `read` syscall) from two processes and check that
//   1. sp1 equals our own user rsp at that `read` syscall (the entry path wrote it), and
//   2. sp0 differs between parent and child (each return to userspace rewrites it with the
//      current task's kernel stack).

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/syscall.h>
#include <sys/wait.h>
#include <fcntl.h>
#include <unistd.h>

static char buf[4096];

// Issues the raw `read` syscall and returns the user rsp at the `syscall` instruction.
static long raw_read(int fd, char *dst, size_t len, uint64_t *rsp_at_syscall)
{
	long ret;
	uint64_t rsp;
	__asm__ volatile("mov %%rsp, %1\n\t"
			 "syscall"
			 : "=a"(ret), "=&r"(rsp)
			 : "a"((long)SYS_read), "D"((long)fd), "S"(dst), "d"(len)
			 : "rcx", "r11", "memory");
	*rsp_at_syscall = rsp;
	return ret;
}

static uint64_t field(const char *name)
{
	char key[64];
	snprintf(key, sizeof(key), "\n%s=", name);
	const char *p = strstr(buf, key);
	if (!p)
		return 0;
	return strtoull(p + strlen(key), NULL, 0);
}

static int snapshot(const char *who, uint64_t *sp0)
{
	int fd = open("/proc/kmiri_3871", O_RDONLY);
	if (fd < 0) {
		perror("open /proc/kmiri_3871");
		return -1;
	}
	uint64_t rsp;
	buf[0] = '\n';
	long n = raw_read(fd, buf + 1, sizeof(buf) - 2, &rsp);
	close(fd);
	if (n <= 0) {
		perror("read");
		return -1;
	}
	buf[n + 1] = '\0';
	printf("----- [%s pid=%d] /proc/kmiri_3871 -----%s", who, getpid(), buf);
	uint64_t sp1 = field("now_sp1");
	*sp0 = field("now_sp0");
	printf("[%s] user rsp at read syscall = 0x%llx, TSS.sp1 = 0x%llx -> %s\n", who,
	       (unsigned long long)rsp, (unsigned long long)sp1,
	       sp1 == rsp ? "MATCH (written by this syscall entry)" : "MISMATCH");
	return sp1 == rsp ? 0 : 1;
}

int main(void)
{
	uint64_t parent_sp0, child_sp0;
	int bad = 0;

	bad |= snapshot("parent", &parent_sp0) != 0;

	int pipefd[2];
	if (pipe(pipefd) < 0) {
		perror("pipe");
		return 1;
	}
	pid_t pid = fork();
	if (pid == 0) {
		uint64_t sp0;
		int r = snapshot("child", &sp0);
		if (write(pipefd[1], &sp0, sizeof(sp0)) != sizeof(sp0))
			r = 1;
		_exit(r == 0 ? 0 : 1);
	}
	int status = 0;
	if (read(pipefd[0], &child_sp0, sizeof(child_sp0)) != sizeof(child_sp0))
		bad = 1;
	waitpid(pid, &status, 0);
	bad |= !(WIFEXITED(status) && WEXITSTATUS(status) == 0);

	printf("[summary] parent TSS.sp0 = 0x%llx, child TSS.sp0 = 0x%llx -> %s\n",
	       (unsigned long long)parent_sp0, (unsigned long long)child_sp0,
	       parent_sp0 != child_sp0 ? "DIFFER (rewritten per task)" : "SAME");
	printf("[summary] plain Rust read of TSS.sp0 = 0x%llx, actual memory = 0x%llx -> %s\n",
	       (unsigned long long)field("rust_view_sp0"), (unsigned long long)field("now_sp0"),
	       field("rust_view_matches_memory") ? "CONSISTENT" : "STALE (compiler assumed the static never changes)");
	const char *v = strstr(buf, "verdict=");
	printf("KMIRI_3871_RESULT %s ok=%d\n", v ? v : "verdict=?\n", !bad);
	return bad;
}
