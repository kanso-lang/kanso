# inflate: puff.c in kanso

A DEFLATE decoder in the style of Mark Adler's `puff.c`, the small reference
inflater that ships in zlib's `contrib/puff` directory, with the gzip and zlib
wrappers around it and a small compressor so that round trips can be tested.
The code here was written from RFC 1950, 1951 and 1952 and from reading how
puff structures the problem; none of puff's source is copied.

## What it covers

- **DEFLATE decoding** (`deflate/`): stored, fixed-Huffman and
  dynamic-Huffman blocks, with puff's canonical code construction (`counts`
  and `symbols`), puff's bit-at-a-time decode, and a nine-bit lookup table in
  front of it for the common short codes, as zlib's inflate has. Every error
  puff reports is reported here, with the byte offset where it was found:
  invalid block type, stored length that does not match its complement,
  too many length or distance codes, an incomplete or over-subscribed
  code-length code, a repeat with no previous length, too many code lengths,
  no end-of-block code, invalid code lengths (with puff's exception for a
  single one-bit code), an invalid length or distance symbol, a distance too
  far back, and input that runs out.
- **gzip** (`container/gzip.kso`, RFC 1952): every header field (FTEXT,
  FHCRC with its check, FEXTRA, FNAME, FCOMMENT, the time stamp and OS),
  reserved-flag and method checks, the CRC-32 and length trailer, several
  members in one file, and junk after the last member.
- **zlib** (`container/zlib.kso`, RFC 1950): the method, window size and
  header check, a clear refusal for a preset dictionary, and the Adler-32
  trailer.
- **CRC-32 and Adler-32** (`checksum/`), table-driven and incremental.
- **A compressor** (`compress/`): greedy LZ77 over a 32 KiB window with hash
  chains, written with the fixed Huffman code, falling back to stored blocks
  when that would be smaller, wrapped as gzip, zlib or raw DEFLATE.

What it leaves out: dynamic-Huffman *encoding* and lazy matching (the
compressor is the "static Huffman with LZ77" the port asked for, so its output
is about 40% larger than `gzip -9`'s on text), preset dictionaries, streaming
(the whole input is read into memory, as puff also requires), and reading
from standard input, which kanso cannot do for bytes that are not text (see
FRICTION.md, F3).

## Running it

    KANSO=/tmp/claude-0/kanso-main/kanso
    $KANSO run . -- cat fixtures/data/hello.gz
    $KANSO run . -- info fixtures/data/prose.txt.gz
    $KANSO run . -- blocks fixtures/data/mixed.deflate
    $KANSO run . -- gzip fixtures/data/prose.txt -o /tmp/prose.gz
    $KANSO run . -- roundtrip fixtures/data/prose.txt

or build a binary with `$KANSO build .` (or `--release`) and run `./inflate`.
The commands:

    cat [--gzip|--zlib|--raw] FILE [-o OUT]   decompress; text to stdout, or
                                              any bytes to OUT
    hex FILE                                  decompressed bytes as hex
    info FILE                                 header fields, sizes, checksums
    blocks FILE                               one line per DEFLATE block
    bench FILE N                              decompress N times
    gzip | zlib | deflate FILE [-o OUT]       compress; hex to stdout or
                                              binary to OUT
    roundtrip FILE                            compress three ways, decompress
    crc32 FILE | adler32 FILE                 checksums of the file as it is

The format is detected from the first bytes (gzip's magic number, then a zlib
header that passes its check, else raw DEFLATE) unless a flag names it.
Compressed output goes to stdout as hex because kanso's standard output takes
only text; `-o` writes the bytes.

## Tests

    sh check.sh            unit tests, then every fixture on three engines
    sh check.sh --bless    rewrite expected outputs from the interpreter

There are 55 `test_*` constants across the five modules and 77 fixtures in
`fixtures/cases/`. Each fixture is a command line and its expected output;
`check.sh` runs it on the interpreter, a dev-tier binary and a release binary
and requires all three to match byte for byte. Several fixtures hand the
compressor's output to the real `gzip -d` and to Python's `zlib` to check it
against independent decoders.

The compressed inputs in `fixtures/data/` were written by the real `gzip`
binary and by zlib through Python, and `fixtures/make_data.py` regenerates
all of them deterministically. They cover each block type, each zlib
strategy (default, filtered by fixed codes, Huffman-only, RLE), level 0
stored blocks over 64 KiB, a stream that mixes all three block kinds, empty
inputs, a gzip header with every optional field, two gzip members, a 350 KB
text for throughput, and one fault per corrupt fixture. Faults no compressor
writes (an over-subscribed code-length code, a repeat with nothing before it,
a single one-bit distance code) are assembled bit by bit.

## Layout

    main.kso           the entry: hands the arguments to cli
    checksum/          CRC-32 and Adler-32
    deflate/           bit reader, Huffman codes, tables, the block loop
    container/         gzip, zlib and format detection
    compress/          bit writer, fixed code, LZ77, wrappers
    cli/               commands and reports
    fixtures/          data, cases, and the script that makes the data
    bugs/              minimal reproductions of compiler and runtime bugs
    scratch/           earlier versions kept for FRICTION.md's measurements

## Credit

puff.c is by Mark Adler, part of zlib (zlib license). DEFLATE, zlib and gzip
are specified in RFC 1951, 1950 and 1952 by L. Peter Deutsch and Jean-loup
Gailly.
