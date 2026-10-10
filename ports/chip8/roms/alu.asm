; Arithmetic and flag checks. Each check leaves a result in V0 and a flag in
; VF, and `record` draws them as two hex bytes, result then flag, five lines
; to a column. Expected under the VIP quirks:
;
;   10 01   F0 + 20 carries          20 01   SUBN: 50 - 30
;   30 00   10 + 20                  42 01   SHR of 85 (from VY)
;   E0 00   SUB 30 - 50 borrows      0A 01   SHL of 85 (from VY)
;   20 01   SUB 50 - 30              FF 00   OR, VF cleared
;   E0 00   SUBN: 30 - 50 borrows    F0 00   XOR, VF cleared
;
; Two more leave their answers in registers: VA = FF + 2 = 01 with VF
; untouched (7XNN sets no flag), and VB = the flag of ADD VF, V1, which
; overwrites the sum because the flag is written last.
        CLS
        LD   V6, 0              ; cursor x
        LD   V7, 0              ; cursor y
        LD   VC, 0              ; column start
        LD   V0, #F0
        LD   V1, #20
        ADD  V0, V1
        CALL record
        LD   V1, #20
        ADD  V0, V1
        CALL record
        LD   V0, #30
        LD   V1, #50
        SUB  V0, V1
        CALL record
        LD   V0, #50
        LD   V1, #30
        SUB  V0, V1
        CALL record
        LD   V0, #50
        LD   V1, #30
        SUBN V0, V1
        CALL record
        LD   V0, #30
        LD   V1, #50
        SUBN V0, V1
        CALL record
        LD   V0, 0
        LD   V1, #85
        SHR  V0, V1
        CALL record
        LD   V0, 0
        SHL  V0, V1
        CALL record
        LD   VF, #77
        LD   V0, #F0
        LD   V1, #0F
        OR   V0, V1
        CALL record
        LD   VF, #77
        LD   V0, #FF
        XOR  V0, V1
        CALL record
        LD   VF, #77
        LD   VA, #FF
        ADD  VA, 2
        LD   VD, VF
        LD   VF, #FF
        LD   V1, 1
        ADD  VF, V1
        LD   VB, VF
end:    JP   end

; Draw V0 and VF at the cursor, then move the cursor down one line.
record: LD   V8, VF
        LD   V5, V0
        CALL show
        ADD  V6, 2
        LD   V5, V8
        CALL show
        LD   V6, VC
        ADD  V7, 6
        SE   V7, 30
        RET
        LD   V7, 0
        LD   VC, 32
        LD   V6, VC
        RET

; Draw V5 as two hex digits at (V6, V7), moving V6 past them.
show:   LD   V9, V5
        SHR  V9
        SHR  V9
        SHR  V9
        SHR  V9
        LD   F, V9
        DRW  V6, V7, 5
        ADD  V6, 5
        LD   V9, #0F
        AND  V9, V5
        LD   F, V9
        DRW  V6, V7, 5
        ADD  V6, 5
        RET
