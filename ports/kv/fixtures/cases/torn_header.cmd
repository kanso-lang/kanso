# Torn inside the header: fewer than fourteen bytes of the last record.
export KANSO_NOW=1700000000000
kv "$STORE" <<'SCRIPT'
put first value-one
put second value-two
SCRIPT
tear "$STORE/1.bitcask.data" 30
kv "$STORE" get second
kv "$STORE" get first
