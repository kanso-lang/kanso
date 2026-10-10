; A ball bounces inside a frame, one step per 60 Hz frame, timed by the
; delay timer. Every wall hit makes a short beep. Run it for a number of
; cycles and the report shows where the ball has got to.
        CLS
        CALL walls
        LD   V0, 5              ; ball x
        LD   V1, 9              ; ball y
        LD   V2, 1              ; dx
        LD   V3, 1              ; dy
        LD   I, ball
        DRW  V0, V1, 2
frame:  LD   V5, 1
        LD   DT, V5
tick:   LD   V5, DT
        SE   V5, 0
        JP   tick
        LD   I, ball
        DRW  V0, V1, 2          ; erase
        ADD  V0, V2
        ADD  V1, V3
        SNE  V0, 1
        CALL flip_x
        SNE  V0, 61
        CALL flip_x
        SNE  V1, 1
        CALL flip_y
        SNE  V1, 29
        CALL flip_y
        DRW  V0, V1, 2          ; draw
        JP   frame

; dx and dy are 1 or 255 (-1): negate by subtracting from 0.
flip_x: LD   V6, 0
        SUB  V6, V2
        LD   V2, V6
        JP   beep
flip_y: LD   V6, 0
        SUB  V6, V3
        LD   V3, V6
beep:   LD   V6, 2
        LD   ST, V6
        RET

; The frame: top and bottom rows of eight-pixel strips, then the sides.
walls:  LD   I, strip
        LD   V0, 0
across: LD   V1, 0
        DRW  V0, V1, 1
        LD   V1, 31
        DRW  V0, V1, 1
        ADD  V0, 8
        SE   V0, 64
        JP   across
        LD   I, dot
        LD   V1, 1
down:   LD   V0, 0
        DRW  V0, V1, 1
        LD   V0, 63
        DRW  V0, V1, 1
        ADD  V1, 1
        SE   V1, 31
        JP   down
        RET

ball:   DB   %11000000, %11000000
strip:  DB   #FF
dot:    DB   %10000000
