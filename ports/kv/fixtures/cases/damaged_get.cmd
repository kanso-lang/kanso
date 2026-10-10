# Opening from a hint file does not read the values, so damage to a value
# is found by the get that reads it, which checks the record's checksum.
export KANSO_NOW=1700000000000
kv "$STORE" <<'SCRIPT'
put a 1
put b 2
merge
SCRIPT
poke "$STORE/2.bitcask.data" 15 39
kv "$STORE" get b
kv "$STORE" get a
