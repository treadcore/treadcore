/* bigstack.c -- raise musl's default thread stack size before the first
 * thread is made. Built by configure.sh into build/bigstack.so on a musl
 * machine; run.sh preloads it into the window.
 *
 * Why: on Alpine the only Vulkan a VM has is lavapipe, Mesa's CPU
 * driver, which compiles every shader with LLVM's JIT on a thread of its
 * own. musl gives a thread 128 KiB of stack unless told otherwise, and
 * LLVM's code generator is deeper than that: the window segfaulted in
 * llvm::AsmPrinter::doFinalization on lavapipe's queue thread, every
 * time, before the first frame (13 October 2026, journal 0013). glibc
 * gives 8 MiB and never sees it. pthread_setattr_default_np is the
 * one call that changes the default for threads the program does not
 * create itself. No root, no rebuild of Mesa.
 */
#define _GNU_SOURCE
#include <pthread.h>

__attribute__((constructor)) static void bigstack(void) {
    pthread_attr_t a;
    pthread_attr_init(&a);
    pthread_attr_setstacksize(&a, 8u << 20);
    pthread_setattr_default_np(&a);
    pthread_attr_destroy(&a);
}
