; The delay timer counts down at 60 Hz: wait for it, counting how many times
; the loop ran, then sound a beep. The counter lands in V1 (high) and V2
; (low), and the report's `tone` counts the frames the beep sounded.
        LD   V0, 20
        LD   DT, V0
        LD   V1, 0
        LD   V2, 0
wait:   ADD  V2, 1
        SE   V2, 0
        JP   check
        ADD  V1, 1
check:  LD   V3, DT
        SE   V3, 0
        JP   wait
        LD   V4, DT             ; reads 0
        LD   V0, 6
        LD   ST, V0             ; no instruction reads ST back, so
        LD   DT, V0             ; the delay timer times the beep
beep:   LD   V5, DT
        SE   V5, 0
        JP   beep
end:    JP   end
