#include <stdio.h>
#include <stdlib.h>

/* count the lines on standard input */
static int count_lines(FILE *in)
{
    int c, n = 0;
    while ((c = getc(in)) != EOF)
        if (c == '\n')
            n++;
    return n;
}

int main(void)
{
    int n = count_lines(stdin);
    printf("%d\n", n);
    return 0;
}
