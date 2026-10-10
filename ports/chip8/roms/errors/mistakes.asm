; Every line below has something wrong with it, and the assembler should
; report all of them, each with its line number.
start:  CLS
        MOV  V1, V2             ; no such mnemonic
        LD   V1, 300            ; too big for a byte
        JP   nowhere            ; no such label
        LD   VG, 1              ; not a register
        DRW  V1, V2             ; DRW wants a height
start:  RET                     ; label defined twice
        DB   1, 256             ; too big for DB
9lives: CLS                     ; not a label name
        LD   V1, ST             ; nothing reads the sound timer
        SE   V1, #1G            ; not a number
