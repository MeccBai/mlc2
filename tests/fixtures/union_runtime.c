#include <stdlib.h>

static unsigned drops;

void tracked_free(void *pointer) {
    if (pointer) {
        ++drops;
    }
    free(pointer);
}

__attribute__((destructor))
static void verify_cleanup(void) {
    if (drops != 5) {
        _Exit(90);
    }
}
