# diff and patch, in kanso

A port of the core of [GNU diffutils](https://www.gnu.org/software/diffutils/)
`diff`, together with a unified-diff `patch` in the manner of
[GNU patch](https://savannah.gnu.org/projects/patch/), written in kanso. Both
GNU programs are by the Free Software Foundation and their contributors (diff
originally by Paul Eggert, Mike Haertel, David Hayes, Richard Stallman and Len
Tower; patch originally by Larry Wall). This port was written from an
understanding of how those programs behave and of the algorithms they
document; it contains none of their source.

It is one program with two tools, chosen by the first argument:

    kanso run . -- diff [OPTION]... FILE1 FILE2
    kanso run . -- patch [OPTION]... [ORIGFILE [PATCHFILE]]

or, after `kanso build . --release`, `./diff diff ...` and `./diff patch ...`.

## What it covers

**diff**

- The comparison is GNU diff's: identical leading and trailing lines are set
  aside, lines that match nothing in the other file are discarded
  (`discard_confusing_lines`, including its provisional discards), Myers'
  linear-space O(ND) search with forward and backward passes meeting at a
  middle snake (`diag`/`compareseq`), and `shift_boundaries` to slide runs of
  changes into the conventional places. On the inputs tried, the output is
  byte-identical to GNU diff 3.10.
- Output formats: normal (default), unified (`-u`, `-U N`, `--unified[=N]`)
  and context (`-c`, `-C N`, `--context[=N]`), with "\ No newline at end of
  file" where it belongs, and hunks merged when their context would overlap.
- `-q`/`--brief`, `-s`/`--report-identical-files`, `-i`, `-b`, `-w`,
  `-N`/`--new-file`, `--label`, `--normal`, `-` for standard input.
- Directories: `diff DIR DIR` compares the files one level down and reports
  "Only in", "Common subdirectories" and file/directory mismatches; `-r`
  recurses; `-rN` compares a file or a whole directory present on one side
  with nothing. A file and a directory compare the file with the same name in
  the directory. Binary files (a NUL byte, or bytes that are not UTF-8) are
  reported as "Binary files ... differ".
- Exit status 0 for no differences, 1 for some, 2 for trouble.

**patch**

- Reads one or more unified file patches from standard input, `-i FILE` or a
  second operand, skipping and remembering any text between them.
- Applies each hunk without fuzz. It looks where the hunk says, then one line
  down, one up, two down and so on, following GNU patch's `locate_hunk`
  (including its rule that a hunk with uneven context sits against the top or
  bottom of the file). Reports "Hunk #N succeeded at L (offset K lines)." and
  "Hunk #N FAILED at L.", writes failed hunks to `FILE.rej` with their line
  numbers adjusted the way GNU does, and keeps `FILE.orig` when a patch did
  not apply exactly.
- Creates files (`/dev/null`, an epoch timestamp or a `-0,0` range on the old
  side), deletes them (the same on the new side), and creates the directories
  a new file needs.
- Detects a reversed or already-applied patch and skips it, as GNU patch does
  when it has no terminal to ask; `-N` skips without asking, `-R` applies in
  reverse. `-pN`/`--strip=N` (default: the base name), `-o FILE`,
  `--dry-run`, `-s`.
- Exit status 0 when everything applied, 1 when some hunk failed or was
  skipped, 2 for trouble.

## What it leaves out

- Timestamps. `std/os` cannot read a file's modification time, so unified and
  context headers name the file and stop, where GNU appends the time. A file
  that `-N` stands in for still gets GNU's epoch stamp, because that is how
  patch learns a file is to be created or deleted.
- diff's heuristics for very large inputs (`--speed-large-files`, the
  "too expensive" cutoff), `--minimal`, `-p`/`-F` function headers,
  `-y`/side-by-side, `-e`/ed scripts, `-B`, `-I`, `--ignore-file-name-case`,
  `-x`/`-X` exclusions, `--from-file`/`--to-file`, and tab expansion.
- patch's fuzz (`-F`), context and normal diff input, `-b`/backup options,
  `-E`, `--merge`, `-t`/`-f`, interactive questions (the answers are always
  GNU's defaults), git extensions, and quoted file names.
- One known difference from GNU patch 2.7.6: when a rejected hunk's last line
  has no newline, GNU writes the line without a newline and without the "\ No
  newline" marker, which leaves a `.rej` that is not a valid patch. This port
  writes the marker.

## Layout

    main.kso        entry: hands the arguments to cli
    cli/            picks the tool
    lines/          files as lines, the "no newline" bit, -i/-b/-w keys
    myers/          discard, the Myers search, shift_boundaries
    report/         change blocks, hunks, normal/unified/context output
    difftool/       diff's options and driver (files, directories, binary)
    patch/          parsing unified diffs, locating and applying hunks
    patchtool/      patch's options and driver (messages, .rej, .orig)
    fixtures/       54 end-to-end cases: cmd, in/, optional stdin, expected
    bugs/           minimal reproductions of compiler and runtime bugs
    FRICTION.md     the journal

## Running the checks

    sh check.sh

runs the 58 unit tests (`kanso test` on each module), then every fixture on
the interpreter, the dev build and the release build, and fails if any of the
three transcripts differs from another or from the fixture's `expected`.
`sh check.sh --update` rewrites the expected files from the interpreter. The
compiler is found at `$KANSO`, by default `/tmp/claude-0/kanso-main/kanso`.

Each fixture's expected output was also compared with GNU diff 3.10 and GNU
patch 2.7.6 run with `-F0`; they agree except for the timestamps described
above. Randomized comparisons against the same tools, run on the final
release build, covered 10,080 diff cases (normal, unified and context output,
several context sizes, `-i`, `-b`, `-w`, files up to 60 lines) and 2,000
patch cases (offsets, failures, `-R`, `-N`, `-s`, missing newlines), with no
differences.
