"""Regenerate the binary fixtures in data/.

Every compressed fixture comes from the real zlib (through Python's zlib
module) or from the real gzip binary, so the decoder is tested against what
those programs write, not against the port's own encoder. The corrupt
fixtures start from a good stream and damage one field each, or are built
bit by bit where zlib would never write the fault.

    python3 fixtures/make_data.py      (from the project directory)

The output is deterministic: no timestamps, fixed random seeds.
"""
import os
import random
import struct
import subprocess
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "data")


def put(name, data):
    with open(os.path.join(DATA, name), "wb") as f:
        f.write(data)


def raw(data, level=9, strategy=zlib.Z_DEFAULT_STRATEGY):
    c = zlib.compressobj(level, zlib.DEFLATED, -15, 9, strategy)
    return c.compress(data) + c.flush()


def zwrap(data, level=9, strategy=zlib.Z_DEFAULT_STRATEGY):
    c = zlib.compressobj(level, zlib.DEFLATED, 15, 9, strategy)
    return c.compress(data) + c.flush()


def gzip_member(data, name=None, comment=None, extra=None, hcrc=False,
                mtime=0, os_byte=3):
    flags = 0
    tail = b""
    if extra is not None:
        flags |= 4
        tail += struct.pack("<H", len(extra)) + extra
    if name is not None:
        flags |= 8
        tail += name + b"\0"
    if comment is not None:
        flags |= 16
        tail += comment + b"\0"
    if hcrc:
        flags |= 2
    header = bytes([31, 139, 8, flags]) + struct.pack("<I", mtime)
    header += bytes([2, os_byte]) + tail
    if hcrc:
        header += struct.pack("<H", zlib.crc32(header) & 0xFFFF)
    trailer = struct.pack("<II", zlib.crc32(data), len(data) & 0xFFFFFFFF)
    return header + raw(data) + trailer


VOCAB = ("the of and to in a is that for it as was with be by on not he "
         "this are or his from at which but have an they you were her "
         "huffman code length distance block stream window literal "
         "symbol bits byte inflate deflate tree table match copy "
         "kanso puff adler zlib gzip header trailer check sum").split()


def words(rng, n):
    out = []
    for _ in range(n):
        w = rng.choice(VOCAB)
        if rng.random() < 0.08:
            w = w.capitalize()
        out.append(w)
        if rng.random() < 0.06:
            out.append(".\n" if rng.random() < 0.3 else ",")
    return " ".join(out).replace(" .", ".").replace(" ,", ",") + "\n"


def gzip_binary(data):
    return subprocess.run(["gzip", "-9", "-n", "-c"], input=data, check=True,
                          capture_output=True).stdout


class Bits:
    """A little-endian bit writer, for streams zlib would never write."""

    def __init__(self):
        self.acc = 0
        self.n = 0

    def put(self, value, width):
        self.acc |= value << self.n
        self.n += width

    def put_code(self, code, width):
        # Huffman codes go most significant bit first.
        self.put(int(format(code, "0%db" % width)[::-1], 2), width)

    def bytes(self):
        return self.acc.to_bytes((self.n + 7) // 8, "little")


def fixed_code(sym):
    if sym < 144:
        return 0x30 + sym, 8
    if sym < 256:
        return 0x190 + sym - 144, 9
    if sym < 280:
        return sym - 256, 7
    return 0xC0 + sym - 280, 8


def fixed_block(items):
    b = Bits()
    b.put(1, 1)  # final
    b.put(1, 2)  # fixed
    for kind, value in items:
        if kind == "sym":
            b.put_code(*fixed_code(value))
        elif kind == "dist":
            b.put_code(value, 5)
    return b.bytes()


ORDER = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15]


def dynamic_fault(kind):
    """A dynamic block header that goes wrong in one way."""
    b = Bits()
    b.put(1, 1)
    b.put(2, 2)
    b.put(0, 5)   # HLIT: 257 literal/length codes
    b.put(0, 5)   # HDIST: 1 distance code
    b.put(15, 4)  # HCLEN: all 19 code-length codes
    # Code lengths 1, 16, 17 and 18 get two bits each, a complete code
    # whose codes are 00, 01, 10 and 11 in that order.
    clen = {1: 2, 16: 2, 17: 2, 18: 2}
    if kind == "over":
        clen = {1: 1, 16: 1, 17: 1, 18: 1}
    for s in ORDER:
        b.put(clen.get(s, 0), 3)
    code = {1: 0, 16: 1, 17: 2, 18: 3}
    if kind == "repeat":
        b.put_code(code[16], 2)
        b.put(0, 2)
    elif kind == "noend":
        # 258 zeros: 138 then 120, so symbol 256 has no code.
        b.put_code(code[18], 2)
        b.put(127, 7)
        b.put_code(code[18], 2)
        b.put(109, 7)
    elif kind == "toomany":
        # 276 zeros where 258 lengths were promised.
        b.put_code(code[18], 2)
        b.put(127, 7)
        b.put_code(code[18], 2)
        b.put(127, 7)
    return b.bytes() + b"\x00"


def one_distance_block():
    """A dynamic block whose distance code has a single one-bit code, the
    incomplete code puff allows. zlib always sends two distance codes, so
    this one is assembled by hand. It decodes to "abbbb"."""
    b = Bits()
    b.put(1, 1)
    b.put(2, 2)
    b.put(258 - 257, 5)   # HLIT: literal/length symbols 0-257
    b.put(1 - 1, 5)       # HDIST: one distance code
    b.put(18 - 4, 4)      # HCLEN: the first 18 entries of ORDER
    clen = {18: 1, 1: 2, 2: 2}           # codes 0, 10, 11
    for s in ORDER[:18]:
        b.put(clen.get(s, 0), 3)
    code = {18: (0, 1), 1: (2, 2), 2: (3, 2)}

    def send(sym, extra=None, width=0):
        b.put_code(*code[sym])
        if extra is not None:
            b.put(extra, width)

    send(18, 97 - 11, 7)            # 97 zeros
    send(2)                         # 'a'
    send(2)                         # 'b'
    send(18, 138 - 11, 7)           # 99-236
    send(18, 19 - 11, 7)            # 237-255
    send(2)                         # 256
    send(2)                         # 257
    send(1)                         # distance code 0: one bit
    lit = {97: 0, 98: 1, 256: 2, 257: 3}
    for sym in (97, 98, 257):
        b.put_code(lit[sym], 2)
    b.put_code(0, 1)                # distance 1, so 257 copies "bbb"
    b.put_code(lit[256], 2)
    return b.bytes()


def main():
    os.makedirs(DATA, exist_ok=True)
    for f in os.listdir(DATA):
        os.remove(os.path.join(DATA, f))
    rng = random.Random(1951)

    hello = (b"Hello, inflate. This is a short line of text, repeated: "
             b"hello, hello, hello.\n")
    put("hello.txt", hello)
    put("hello.deflate", raw(hello))
    put("hello.zlib", zlib.compress(hello, 9))
    put("hello.gz", gzip_member(hello, name=b"hello.txt"))

    # The real gzip binary, header and all.
    prose = words(rng, 1200).encode()
    put("prose.txt", prose)
    put("prose.txt.gz", gzip_binary(prose))

    # One stream per block type and per zlib strategy.
    put("stored.zlib", zwrap(hello, level=0))
    put("fixed.zlib", zwrap(hello, strategy=zlib.Z_FIXED))
    put("dynamic.zlib", zwrap(words(rng, 400).encode()))
    put("huffman_only.zlib",
        zwrap(words(rng, 300).encode(), strategy=zlib.Z_HUFFMAN_ONLY))
    put("rle.zlib", zwrap(b"a" * 300 + b"b" * 300 + b"ab" * 200,
                          strategy=zlib.Z_RLE))
    put("one_distance.deflate", raw(b"abcabcabcabcabcabcabcabc" * 3))
    put("empty.gz", gzip_member(b""))
    put("empty.zlib", zlib.compress(b""))

    # Several stored blocks: level 0 splits at 64 KiB, and random bytes are
    # not text, so these also exercise the binary output path.
    noise = bytes(rng.getrandbits(8) for _ in range(70000))
    put("noise.zlib", zwrap(noise, level=0))
    put("noise.gz", gzip_member(noise))

    # Mixed blocks in one stream: a full flush ends the block between pieces
    # (and adds an empty stored block as a sync marker), and zlib picks the
    # block kind that suits each piece: dynamic for prose, stored for noise,
    # fixed for a few bytes.
    c = zlib.compressobj(6, zlib.DEFLATED, -15)
    pieces = [words(rng, 200).encode(), noise[:300], b"xyzzy",
              words(rng, 50).encode()]
    mixed = b""
    for piece in pieces:
        mixed += c.compress(piece) + c.flush(zlib.Z_FULL_FLUSH)
    mixed += c.flush()
    put("mixed.deflate", mixed)

    # A header with every optional field, and a header CRC.
    put("fields.gz", gzip_member(hello, name=b"hello.txt",
                                 comment=b"a comment", extra=b"AB\x05\x00kanso",
                                 hcrc=True, mtime=1700000000, os_byte=255))
    # Two members, as `cat a.gz b.gz` makes.
    put("two_members.gz", gzip_member(b"first member\n")
        + gzip_member(b"second member\n"))

    # The throughput file: a few hundred kilobytes of text.
    put("big.txt.gz", gzip_binary(words(random.Random(7), 70000).encode()))

    # Corrupt streams, one fault each.
    good = gzip_member(hello, name=b"hello.txt")
    put("bad_magic.gz", b"\x1f\x8c" + good[2:])
    put("bad_method.gz", good[:2] + b"\x07" + good[3:])
    put("bad_flags.gz", good[:3] + bytes([good[3] | 0x80]) + good[4:])
    put("bad_crc.gz", good[:-8] + bytes([good[-8] ^ 1]) + good[-7:])
    put("bad_size.gz", good[:-4] + struct.pack("<I", len(hello) + 1))
    put("truncated.gz", good[:40])
    put("no_trailer.gz", good[:-8])
    put("trailing_junk.gz", good + b"junk")
    hc = gzip_member(hello, hcrc=True)
    put("bad_header_crc.gz", hc[:10] + bytes([hc[10] ^ 0xFF]) + hc[11:])

    z = zlib.compress(hello, 9)
    put("bad_zlib_check.zlib", bytes([z[0], z[1] ^ 1]) + z[2:])
    put("bad_zlib_method.zlib", bytes([0x77, 0x01]) + z[2:])
    put("zlib_dict.zlib", bytes([0x78, 0xBB]) + b"\x00\x00\x00\x01" + z[2:])
    put("bad_adler.zlib", z[:-1] + bytes([z[-1] ^ 0x10]))

    # Raw DEFLATE faults that no compressor writes.
    put("block_type_3.deflate", bytes([0b111]))
    put("stored_complement.deflate", bytes([1, 5, 0, 0xFA, 0xFE]) + b"hello")
    put("stored_short.deflate", bytes([1, 5, 0, 0xFA, 0xFF]) + b"hel")
    # 'a', then a match of length 3 at distance 2 with one byte written.
    put("too_far.deflate", fixed_block([("sym", 97), ("sym", 257),
                                        ("dist", 1), ("sym", 256)]))
    put("bad_length_symbol.deflate", fixed_block([("sym", 97), ("sym", 286)]))
    put("bad_distance_symbol.deflate",
        fixed_block([("sym", 97), ("sym", 98), ("sym", 257), ("dist", 30)]))
    put("no_final_block.deflate", bytes([0, 0, 0, 0xFF, 0xFF]))
    put("dynamic_oversubscribed.deflate", dynamic_fault("over"))
    put("dynamic_no_end.deflate", dynamic_fault("noend"))
    put("dynamic_repeat_first.deflate", dynamic_fault("repeat"))
    put("dynamic_too_many.deflate", dynamic_fault("toomany"))
    put("single_distance.deflate", one_distance_block())


if __name__ == "__main__":
    main()
