# The bytes of one record: crc32, stamp, key size, value size, key, value,
# all big-endian. 0x6553f100 is 1700000000, the pinned clock in seconds.
export KANSO_NOW=1700000000000
kv "$STORE" put a 1
bytes "$STORE/1.bitcask.data"
kv "$STORE" dump
