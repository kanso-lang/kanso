# chip8: a CHIP-8 emulator, assembler and disassembler in kanso

CHIP-8 is a small interpreted language that Joseph Weisbecker designed for
the RCA COSMAC VIP in 1977. A CHIP-8 program runs on a 64x32 monochrome
screen with a 16-key hex keypad, sixteen 8-bit registers, a 12-bit index
register, a call stack and two 60 Hz timers. It has 35 two-byte
instructions. There is no single original implementation to port: the VIP's
own interpreter defines the behaviour, and later interpreters (most of all
SUPER-CHIP on the HP-48) changed a few details that programs came to rely
on. This port follows the VIP by default and offers the later behaviour as
a quirks profile.

The instruction set and the assembly syntax follow Cowgod's *Chip-8
Technical Reference*. The quirk list follows the one the community test
suites use. All the code here was written from those descriptions; none of
it is translated from another emulator.

## What is here

- `isa/`: the instruction set as values. One type per instruction; `decode`
  from a 16-bit opcode, `encode` back, `spell` to assembly text, and `parse`
  from a mnemonic and operand texts. Hex formatting and number parsing.
- `vm/`: the machine. Memory, registers, stack, the bit-packed 64x32
  display, the two timers, a scripted keypad, a seeded random generator for
  `CXNN`, the quirks profiles, the run loop and the text report.
- `asm/`: a two-pass assembler. Labels, `EQU` constants, `label+n` and
  `a-b` operands, `DB`, `DW` and `DS`, and every error reported with its
  line number.
- `cli/`: the `chip8` command.
- `roms/`: test ROMs, written in assembly for this project, plus two of them
  assembled to `.ch8` binaries.
- `fixtures/cases/`: command lines and their expected output.
- `bugs/`: the smallest programs that show the compiler and runtime bugs met
  on the way. `FRICTION.md` is the journal of everything that got in the way.

## Running it

```
KANSO=/tmp/claude-0/kanso-main/kanso

$KANSO run . -- run roms/digits.asm
$KANSO run . -- run roms/keys.asm --keys roms/keys.txt
$KANSO run . -- run roms/maze.asm --cycles 3000 --seed 7
$KANSO run . -- asm roms/alu.asm --list
$KANSO run . -- dis roms/alu.ch8
```

`kanso build .` makes a native `./chip8` that takes the same arguments.

### run

```
chip8 run ROM [--cycles N] [--per-frame N] [--keys FILE] [--seed N]
              [--quirks vip|modern] [--trace]
```

The ROM can be a `.ch8` binary, a `.hex` file of hex byte pairs, or a
`.asm` source, which is assembled first. The emulator runs at most
`--cycles` instructions (1000 by default). Every `--per-frame` instructions
(10 by default) is one 60 Hz frame: the timers count down and the key
script's events for that frame take effect. A run also stops early when the
program jumps to itself with no timer running and no key event still to
come, and when it faults: an unknown opcode, or a stack that overflows or
runs dry. Nothing depends on the clock, so a run gives the same output on
every engine and every machine.

The report gives the reason the run stopped, the cycle and frame counts, the
number of frames the sound timer held the tone on, the registers, the stack,
the keys held, and the screen, `#` for a lit pixel. `--trace` puts a line per
instruction before it.

A key script is one event per line, `<frame> down|up|tap <key>`, with `#`
comments. `tap` presses a key and releases it two frames later.

The quirks profiles:

| profile  | 8XY6/8XYE shift | FX55/FX65 | BNNN adds | 8XY1/2/3 VF |
|----------|-----------------|-----------|-----------|-------------|
| `vip`    | VY into VX      | advance I | V0        | cleared     |
| `modern` | VX in place     | leave I   | VX        | left alone  |

Sprites wrap their starting position and are clipped at the right and
bottom edges, as on the VIP. FX0A waits for a key to be pressed and
released.

### asm and dis

`chip8 asm SOURCE` prints the assembled bytes as hex, sixteen to a line;
`--list` prints a listing with each address and its bytes beside the source
line. `chip8 dis ROM` prints one instruction per two bytes, in syntax the
assembler reads back: `dis` followed by `asm` reproduces the ROM's bytes,
and a fixture checks that for seven of the test ROMs.

## Tests

```
sh check.sh
```

runs the `test_*` constants in each module (95 of them), then every fixture
in `fixtures/cases` on the interpreter, a dev-tier native binary and a
release binary, and fails if any output differs from the expected file.
`sh check.sh --bless` rewrites the expected files from the interpreter.

## What is left out

- SUPER-CHIP and XO-CHIP extensions: the 128x64 mode, scrolling, the large
  font, the extra registers and audio patterns.
- A window, a beeper and a real-time clock. The display is a text dump, the
  sound timer is a frame count, and time is counted in instructions.
- Writing `.ch8` files. kanso cannot write a binary file, so the assembler
  prints hex text; the emulator reads `.hex` files for that reason. The two
  `.ch8` files in `roms/` were made from that hex outside kanso.
- The VIP's display-wait behaviour (DXYN waiting for vertical blank), which
  only matters for timing against a real clock.
