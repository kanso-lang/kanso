; Keypad. FX0A waits for three keys, each pressed and released, and draws
; them left to right. Then SKP waits for key 5 to go down, and V3 counts the
; trips round a SKNP loop until it comes back up.
        CLS
        LD   V6, 2
        LD   V7, 2
again:  LD   V0, K
        LD   F, V0
        DRW  V6, V7, 5
        ADD  V6, 6
        SE   V6, 20
        JP   again
        LD   V1, 5
poll:   SKP  V1
        JP   poll
held:   ADD  V3, 1
        SKNP V1
        JP   held
end:    JP   end
