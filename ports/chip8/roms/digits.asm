; Draws the sixteen built-in hex digits in two rows of eight.
        CLS
        LD   V0, 0          ; digit
        LD   V1, 2          ; x
        LD   V2, 4          ; y
next:   LD   F, V0
        DRW  V1, V2, 5
        ADD  V0, 1
        ADD  V1, 8
        SE   V0, 8
        JP   same_row
        LD   V1, 2
        LD   V2, 14
same_row:
        SE   V0, 16
        JP   next
done:   JP   done
