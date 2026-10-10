; Subroutines nest sixteen deep. The seventeenth CALL overflows the stack
; and the run stops with a fault; V0 counts the calls that fitted.
        LD   V0, 0
deeper: ADD  V0, 1
        CALL deeper
