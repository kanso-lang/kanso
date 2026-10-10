# A record that is whole but fails its checksum at the end of the newest
# file is treated like a torn write.
export KANSO_NOW=1700000000000
kv "$STORE" <<'SCRIPT'
put a 1
put b 2
SCRIPT
poke "$STORE/1.bitcask.data" 31 7a
kv "$STORE" list
kv "$STORE" dump
