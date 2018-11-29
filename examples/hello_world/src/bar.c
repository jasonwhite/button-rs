#include <stdio.h>
#include "foo.h"

int main() {
    fprintf(stdout, "stdout: %s\n", greeting());
    fflush(stdout);

    fprintf(stderr, "stderr: %s\n", greeting());
    fflush(stderr);

    return 0;
}
