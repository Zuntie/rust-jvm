#include <stdlib.h>
#include <stdio.h>
#include <string.h>

void* alloc_stub(long size) {
    void *ptr = calloc(1, size);
    if (!ptr) {
        fprintf(stderr, "Memory allocation failed\n");
        exit(EXIT_FAILURE);
    }
    return ptr;
}

void null_pointer_exception() {
    printf("Runtime Error: NullPointerException\n");
    exit(EXIT_FAILURE);
}

void print_integer_stub(long value) {
    printf("%ld\n", value);
}

void print_string_stub(const char *str) {
    if (str) {
        printf("%s\n", str);
    } else {
        printf("null\n");
    }
}

void exit_stub(int code) {
    exit(code);
}