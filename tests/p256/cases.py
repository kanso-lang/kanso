"""Cases for p256's differential harness, judged by OpenSSL through the
`cryptography` package. Writes <out>.cases (x y message signature, in hex)
and <out>.want (1 or 0 a line). Run: python3 cases.py <count> <seed> <out>"""
import random
import sys

from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.hazmat.primitives.asymmetric.utils import (
    decode_dss_signature, encode_dss_signature)

P = 2**256 - 2**224 + 2**192 + 2**96 - 1
N = 0xffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551

count, seed, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
rng = random.Random(seed)
keys = [ec.derive_private_key(rng.randrange(1, N), ec.SECP256R1()) for _ in range(64)]


def judged(x, y, msg, sig):
    # SEC 1 (2.3.4) takes a coordinate as a field element, below p, and a
    # COSE key holds each in 32 bytes. OpenSSL reduces one at or above p
    # and accepts the key, so the reference refuses those itself.
    if not (0 <= x < P and 0 <= y < P):
        return 0
    try:
        pub = ec.EllipticCurvePublicNumbers(x, y, ec.SECP256R1()).public_key()
    except ValueError:
        return 0
    try:
        pub.verify(sig, msg, ec.ECDSA(hashes.SHA256()))
        return 1
    except InvalidSignature:
        return 0


def der_int(v, pad=False):
    b = v.to_bytes(max(1, (v.bit_length() + 7) // 8), "big")
    if b[0] & 0x80 or pad:
        b = b"\x00" + b
    return b"\x02" + bytes([len(b)]) + b


def der(r, s, pad_r=False, pad_s=False):
    body = der_int(r, pad_r) + der_int(s, pad_s)
    return b"\x30" + bytes([len(body)]) + body


def case():
    k = rng.choice(keys)
    nums = k.public_key().public_numbers()
    x, y = nums.x, nums.y
    msg = rng.randbytes(rng.choice([0, 1, 5, 32, 55, 56, 64, 100, 300]))
    sig = k.sign(msg, ec.ECDSA(hashes.SHA256()))
    r, s = decode_dss_signature(sig)
    kind = rng.randrange(20)
    if kind < 7:
        pass
    elif kind == 7:
        msg = bytearray(msg + b"!")
        msg[rng.randrange(len(msg))] ^= 1 << rng.randrange(8)
        msg = bytes(msg)
    elif kind == 8:
        b = bytearray(sig)
        b[rng.randrange(len(b))] ^= 1 << rng.randrange(8)
        sig = bytes(b)
    elif kind == 9:
        sig = der(r, N - s)
    elif kind == 10:
        other = rng.choice(keys).public_key().public_numbers()
        x, y = other.x, other.y
    elif kind == 11:
        y = (y + 1) % P
    elif kind == 12:
        x, y = x + P, y
    elif kind == 13:
        sig = der(rng.choice([0, N, N + 1, r]), rng.choice([0, N, N - 1, s]))
    elif kind == 14:
        sig = der(r, s, pad_r=True)
    elif kind == 15:
        sig = sig + b"\x00"
    elif kind == 16:
        sig = der(r + N, s)
    elif kind == 17:
        x, y = (P - x), y
    elif kind == 18:
        y = P - y
        sig = der(r, s)
    else:
        sig = der(rng.randrange(1, N), rng.randrange(1, N))
    xb = x.to_bytes(32, "big") if x < 2**256 else x.to_bytes(33, "big")
    yb = y.to_bytes(32, "big")
    return (xb.hex(), yb.hex(), msg.hex(), sig.hex()), judged(x, y, msg, sig)


with open(out + ".cases", "w") as c, open(out + ".want", "w") as w:
    for _ in range(count):
        fields, want = case()
        c.write(" ".join(f if f else "" for f in fields) + "\n")
        w.write(f"{want}\n")
