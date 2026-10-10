; Two of the behaviours SUPER-CHIP changed. Run with --quirks vip and
; --quirks modern to see both.
;
; FX55 and FX65: on the VIP they leave I just past the last register.
; VA ends up as the byte at I after the store: 0 on the VIP (the byte after
; the four stored), 1 under SUPER-CHIP (I still points at the first).
;
; BNNN: the VIP jumps to NNN + V0; SUPER-CHIP reads it as BXNN and adds VX,
; X being the top nibble of the address. `table` sits in the #2xx page, so
; that is V2. VB records which entry the jump landed on.
        LD   V0, 1
        LD   V1, 2
        LD   V2, 3
        LD   V3, 4
        LD   I, buffer
        LD   [I], V3
        LD   V0, [I]
        LD   VA, V0
        LD   V0, 2
        LD   V2, 4
        JP   V0, table
landed: LD   I, buffer
        DRW  V4, V4, 4          ; the stored bytes, as a sprite
end:    JP   end

table:  JP   entry_0
        JP   entry_1
        JP   entry_2
entry_0: LD  VB, 0
        JP   landed
entry_1: LD  VB, 1
        JP   landed
entry_2: LD  VB, 2
        JP   landed

buffer: DS   8
