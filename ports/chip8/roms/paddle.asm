; A one-player paddle game. Keys 4 and 6 move the paddle along the bottom
; row; the ball bounces off the walls and the top, and the score in the
; corner goes up each time the paddle returns it. A ball that gets past
; the paddle is counted in VB and served again from the top.
;
; The paddle hit is found the CHIP-8 way: the ball is drawn, and if the draw
; turned a pixel off, VF says it landed on the paddle.
LEFT:     EQU 4
RIGHT:    EQU 6
FLOOR:    EQU 31
CEILING:  EQU 7
PADDLE_Y: EQU 30

        CLS
        LD   VA, 28             ; paddle x
        LD   I, paddle
        LD   V5, PADDLE_Y
        DRW  VA, V5, 1
        LD   V8, 0              ; score
        CALL score
        CALL serve
frame:  LD   V5, 1
        LD   DT, V5
tick:   LD   V5, DT
        SE   V5, 0
        JP   tick
        LD   I, ball
        DRW  V0, V1, 1          ; erase the ball
        LD   V5, LEFT
        SKNP V5
        CALL go_left
        LD   V5, RIGHT
        SKNP V5
        CALL go_right
        ADD  V0, V2
        ADD  V1, V3
        SNE  V0, 0
        CALL flip_x
        SNE  V0, 63
        CALL flip_x
        SNE  V1, CEILING
        CALL flip_y
        SNE  V1, FLOOR
        JP   missed
        LD   I, ball
        DRW  V0, V1, 1
        SE   VF, 0
        CALL hit
        JP   frame

missed: ADD  VB, 1
        CALL serve
        JP   frame

; Put the ball back at the top, heading down and right.
serve:  LD   V0, 20
        LD   V1, CEILING+1
        LD   V2, 1
        LD   V3, 1
        LD   I, ball
        DRW  V0, V1, 1
        RET

; The ball landed on the paddle: take it back off, send it up, score.
hit:    DRW  V0, V1, 1
        LD   V3, 255
        ADD  V1, V3
        ADD  V1, V3
        DRW  V0, V1, 1
        CALL score              ; erase the old score
        ADD  V8, 1
        CALL score              ; draw the new one
        RET

flip_x: LD   V6, 0
        SUB  V6, V2
        LD   V2, V6
        RET

flip_y: LD   V6, 0
        SUB  V6, V3
        LD   V3, V6
        RET

go_left: SNE VA, 0
        RET
        LD   I, paddle
        LD   V5, PADDLE_Y
        DRW  VA, V5, 1
        LD   V6, 2
        SUB  VA, V6
        DRW  VA, V5, 1
        RET

go_right: SNE VA, 56
        RET
        LD   I, paddle
        LD   V5, PADDLE_Y
        DRW  VA, V5, 1
        ADD  VA, 2
        DRW  VA, V5, 1
        RET

; Draw V8 as two decimal digits at the top left. Drawing the same digits
; twice erases them, so `score` both shows and clears.
score:  LD   I, saved
        LD   [I], V3            ; keep the ball's V0..V3
        LD   I, digits
        LD   B, V8
        LD   V2, [I]            ; V0..V2 = hundreds, tens, ones
        LD   V6, 1
        LD   V7, 1
        LD   F, V1
        DRW  V6, V7, 5
        LD   V6, 6
        LD   F, V2
        DRW  V6, V7, 5
        LD   I, saved
        LD   V3, [I]
        RET

paddle: DB   #FF
ball:   DB   %10000000
digits: DS   3
saved:  DS   4
