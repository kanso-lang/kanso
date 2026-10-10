# kv: bitcask in kanso

A port of Bitcask, the log-structured key-value store that Basho wrote in
Erlang as a storage backend for Riak. The design is described in Justin
Sheehy and David Smith's 2010 paper, *Bitcask: A Log-Structured Hash Table
for Fast Key/Value Data*. This port was written from that description and
from the documented file layout of the Erlang implementation; no Bitcask
source was copied.

## What it does

A store is a directory of numbered data files. Every write appends one
record to the newest file:

    crc32 (4) | stamp (4) | key size (2) | value size (4) | key | value

All fields are big-endian, and the CRC-32 (zlib's) covers every byte after
itself. A delete appends a record whose value size is `0xffffffff` and which
carries no value. An in-memory key directory maps each key to the file,
offset and size of its newest record, so a get reads one record.

The port covers:

- **Append-only data files** with a per-record checksum and timestamp.
- **The key directory**, rebuilt on open by reading files oldest first.
- **Tombstones** for deletes.
- **Rolling over** to a new data file when a record would carry the active
  file past `--max-file-size`. Each process that opens the store starts a
  new active file, as Bitcask does.
- **Merge**, which copies the newest record of every live key into new
  files, drops overwritten records, deletes and expired records, writes a
  hint file beside each output file, and then removes the files it read,
  oldest first, so that a crash at any point leaves a store that reads the
  same.
- **Hint files**: per live key, its stamp, key size, record size and
  offset, ending in an entry that holds a CRC-32 of the rest. Opening reads
  a data file's hint file instead of the data file when one is there and
  checks out, and falls back to the data file when it does not.
- **Crash recovery.** A short or checksum-failing record at the end of the
  newest data file is a torn write: the file is cut back to the last whole
  record and the store opens. Damage in an older file is reported and left
  in place. A get checks the record's checksum before answering.
- **Expiry**: with `--expiry SECONDS`, records older than that read as
  absent and are dropped by a merge.
- **A CLI** with `put`, `get`, `delete`, `list`, `merge`, `status` and
  `dump`, run one command at a time or as a script on standard input.

## Running it

    KANSO=/tmp/claude-0/kanso-main/kanso
    $KANSO run . -- db put tea sencha
    $KANSO run . -- db get tea
    printf 'put a 1\nput b 2\ndelete a\nmerge\nstatus\n' | $KANSO run . -- db

    usage: kv [OPTIONS] DIR [COMMAND [ARGS...]]
    options: --max-file-size BYTES (default 2 GiB), --expiry SECS
    commands: put KEY VALUE, get KEY, delete KEY, list, merge, status, dump
    with no command, kv reads commands from standard input

`get` of a missing key prints `not found: KEY` and exits 1. A command-line
mistake exits 2. A script is parsed whole before any of it runs.

Timestamps come from `time/now`; set `KANSO_NOW` (milliseconds) to pin them,
which is how the fixtures get identical bytes on every engine.

`sh check.sh` runs the unit tests and then every fixture in
`fixtures/cases/` on the interpreter, a dev binary and a release binary, and
fails if any output differs from the expected file or between engines.
`sh check.sh --bless` rewrites the expected files from the interpreter.
Several fixtures print data and hint files with `od`, so the on-disk bytes
are checked as well as the program's view of them.

## Layout

    main.kso            entry: hands the arguments to cli/main
    bitcask/            the store
      bytes.kso         big-endian fields, hex, byte lists
      crc.kso           CRC-32
      format.kso        record and hint encodings
      scan.kso          reading a data file to its last good record
      disk.kso          writing bytes, removing files (through sh; see below)
      store.kso         opening, the key directory, recovery
      write.kso         put, delete, get
      merge.kso         merge and hint files
      report.kso        status and dump
    cli/                argument and script parsing, running commands
    fixtures/cases/     NAME.cmd scripts and their NAME.out
    bugs/               reductions of the engine problems found
    FRICTION.md         the journal of what got in the way

## Where it differs from Bitcask

- **Writing goes through `sh`.** kanso's `std/os` has no way to write a
  byte above 0x7f, and no append, remove or rename (FRICTION.md F1, F2).
  `bitcask/disk.kso` writes every chunk of bytes with
  `sh -c 'printf "$1" >> "$2"'`, passing the bytes as octal escapes in a
  positional parameter, and removes files with `rm`. The file format is
  unaffected, but each put starts a process.
- **A get reads the whole data file**, because there is no positional read
  (F20). The key directory still decides which file and offset.
- **Tombstones** are a value size of `0xffffffff` rather than Erlang
  Bitcask's magic tombstone value, so any string can be stored.
- **Merge takes every file**, including the active one. Erlang Bitcask can
  merge a chosen subset and decides when a merge is worth doing from
  fragmentation and dead-byte thresholds; here `status` reports the dead
  bytes and the merge is run by hand.
- **Left out:** the write lock file and the single-writer protection it
  gives, read-only opens, sync strategies, the fold API over keys and
  values, and the keydir sharing between processes in one VM.
