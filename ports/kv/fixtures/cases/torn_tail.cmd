# A crash in the middle of an append leaves a short record at the end of
# the newest file. Opening cuts the file back to the last whole record.
export KANSO_NOW=1700000000000
kv "$STORE" <<'SCRIPT'
put a 1
put b 2
put c 3
SCRIPT
tear "$STORE/1.bitcask.data" 45
kv "$STORE" list
bytes "$STORE/1.bitcask.data"
kv "$STORE" list
