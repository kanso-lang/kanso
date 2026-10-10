; Decimal display through FX33 (BCD) and FX65 (load registers): prints
; 0, 7, 42, 137 and 255 as three decimal digits each, one number per line.
        CLS
        LD   V7, 1              ; y
        LD   V3, 0
        CALL number
        LD   V3, 7
        CALL number
        LD   V3, 42
        CALL number
        LD   V3, 137
        CALL number
        LD   V3, 255
        CALL number
end:    JP   end

; Draw V3 in decimal at (2, V7), then move down a line.
number: LD   I, scratch
        LD   B, V3
        LD   V2, [I]            ; V0..V2 = hundreds, tens, ones
        LD   V6, 2
        LD   F, V0
        DRW  V6, V7, 5
        ADD  V6, 5
        LD   F, V1
        DRW  V6, V7, 5
        ADD  V6, 5
        LD   F, V2
        DRW  V6, V7, 5
        ADD  V7, 6
        RET

scratch: DS 3
