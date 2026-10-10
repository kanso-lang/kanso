; A random maze: every 4x4 cell gets one of two diagonals, chosen by CXNN.
; The generator is seeded with --seed, so a seed always draws one maze.
        LD   V0, 0              ; x
        LD   V1, 0              ; y
cell:   LD   I, rising
        RND  V2, 1
        SE   V2, 0
        LD   I, falling
        DRW  V0, V1, 4
        ADD  V0, 4
        SE   V0, 64
        JP   cell
        LD   V0, 0
        ADD  V1, 4
        SE   V1, 32
        JP   cell
end:    JP   end

rising:  DB %00010000, %00100000, %01000000, %10000000
falling: DB %10000000, %01000000, %00100000, %00010000
