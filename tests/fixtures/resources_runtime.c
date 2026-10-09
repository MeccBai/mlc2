#include <stdlib.h>

static unsigned drops;

void tracked_free(void *pointer) {
    if (pointer) {
        ++drops;
    }
    free(pointer);
}

__attribute__((destructor))
static void verify_resource_cleanup(void) {
    /* Nested/generic units and overwritten unit, plus direct resource cleanup. */
    if (drops != 12) {
        _Exit(90);
    }
}
