#include <stdlib.h>

static unsigned allocations;

void *test_malloc(size_t bytes) {
    return ++allocations == 1 ? malloc(bytes) : NULL;
}

_Noreturn void test_abort(void) {
    _Exit(73);
}

void test_free(void *pointer) {
    (void)pointer;
    _Exit(74);
}
