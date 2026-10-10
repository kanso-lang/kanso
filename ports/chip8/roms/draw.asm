; DXYN: collisions, clipping and wrapping.
;
; A block drawn on a blank screen sets VF to 0 (saved in VA); drawn again
; on itself it erases and sets VF to 1 (VB). Drawn at (60, 28) it is cut off
; at the right and bottom edges. Drawn at (70, 40) it wraps to (6, 8),
; because the start position wraps while the sprite does not.
        LD   I, block
        LD   V0, 20
        LD   V1, 10
        DRW  V0, V1, 8
        LD   VA, VF
        DRW  V0, V1, 8
        LD   VB, VF
        LD   V0, 60
        LD   V1, 28
        DRW  V0, V1, 8
        LD   VC, VF
        LD   V0, 70
        LD   V1, 40
        DRW  V0, V1, 8
        LD   VD, VF
end:    JP   end

block:  DB   #FF, #81, #81, #81, #81, #81, #81, #FF
