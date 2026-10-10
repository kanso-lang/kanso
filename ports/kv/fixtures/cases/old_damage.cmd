# Damage in a file that is no longer being written is not a torn write: the
# bytes stay as they are, and records after the damage are not read.
export KANSO_NOW=1700000000000
kv "$STORE" <<'SCRIPT'
put a 1
put b 2
put c 3
SCRIPT
kv "$STORE" put d 4
poke "$STORE/1.bitcask.data" 20 00
kv "$STORE" list
bytes "$STORE/1.bitcask.data"
kv "$STORE" list
