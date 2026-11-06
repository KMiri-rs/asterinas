// SPDX-License-Identifier: MPL-2.0

// #3896 触发器的用户态驱动。它经下面这个调试 syscall 进内核，所以乱序 guard drop 和
// RCU 回收都发生在一个正常启动、由用户态拉起的内核上。syscall 还会把内核记录的事件
// 时间线拷回一个用户缓冲区，于是完整的插桩顺序在这里就能观测，既不靠 KMiri，也不靠任何
// 内核内打印。

#define _GNU_SOURCE
#include <sched.h>
#include <signal.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <sys/wait.h>
#include <unistd.h>

// 私有调试 syscall 号，注册在 `kernel/core/src/syscall/arch/x86.rs`。
#define SYS_KMIRI_3896 900

// 内核 `Ev` 码对应的名字（见 ostd/src/kmiri_3896.rs）。下标 0 是空槽；事件是 1..=10，按发生顺序。
static const char *EV_NAME[] = {
	"-",
	"OuterNew",
	"InnerNew",
	"ReadWith",
	"OuterDrop",
	"IrqEnabledWhileWitnessLive",
	"MightSleepReturned",
	"Update",
	"Reclaim",
	"Use",
	"InnerDrop",
};

int main(void)
{
	// 握手管道：对端进程真正跑起来后写一个字节，这样父进程进 syscall 前就能确定本 CPU 运行
	// 队列里有一个可运行的对端。没有它的话，父进程有时会抢先进 syscall，内核里的 yield_now
	// 找不到可切换的任务，宽限期就推不动。
	int sync_pipe[2];
	if (pipe(sync_pipe) < 0) {
		perror("pipe");
		return 1;
	}

	pid_t peer = fork();
	if (peer < 0) {
		perror("fork");
		return 1;
	}
	if (peer == 0) {
		close(sync_pipe[0]);
		// 宣告对端已被调度、正在运行，然后永远保持可运行，让每一次内核里的 yield_now 都有任务可切。
		if (write(sync_pipe[1], "x", 1) != 1)
			_exit(1);
		for (;;)
			sched_yield();
	}

	// 阻塞直到对端被调度、进入它的 yield 循环。
	close(sync_pipe[1]);
	char b;
	if (read(sync_pipe[0], &b, 1) != 1) {
		perror("read");
		return 1;
	}

	unsigned int tl[64] = { 0 };
	long code = syscall(SYS_KMIRI_3896, tl, 64);

	kill(peer, SIGKILL);
	waitpid(peer, NULL, 0);

	// bit 0：RCU 在过期引用被使用之前就回收了对象。
	// bits 8..15：通过那个过期引用读到的值。
	// bits 16..31：从 retire 到真正回收经过了多少次 yield；有可运行对端时这个值很小
	//（update 之后一次真实切换就足以在 SMP=1 下完成宽限期）。
	printf("[kmiri-3896-user] code=%#lx reclaim_before_use=%ld stale_val=%#lx yields_until_reclaim=%ld\n",
	       code, code & 1, (code >> 8) & 0xff, (code >> 16) & 0xffff);

	// 内核记录的事件顺序，直接来自内核的环形缓冲区。
	// `Reclaim` 出现在 `Use` 之前，就是不依赖 KMiri 的 UAF 证明。
	printf("[kmiri-3896-user] timeline:");
	for (int i = 0; i < 64 && tl[i] != 0; i++) {
		unsigned int ev = tl[i];
		if (ev <= 10)
			printf(" %s", EV_NAME[ev]);
		else
			printf(" ev%u", ev);
	}
	printf("\n");

	// 只有观测到 UAF 窗口时才返回 0，这样 runner 能统计 N 次里有多少次真正触发。
	return (code & 1) ? 0 : 2;
}
