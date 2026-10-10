#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* count the lines and the bytes on standard input */
static int count_lines(FILE *in, long *bytes)
{
    int c, n = 0;
    *bytes = 0;
    while ((c = getc(in)) != EOF) {
        (*bytes)++;
        if (c == '\n')
            n++;
    }
    return n;
}

int main(void)
{
    long bytes;
    int n = count_lines(stdin, &bytes);
    printf("%d %ld\n", n, bytes);
    return 0;
}
