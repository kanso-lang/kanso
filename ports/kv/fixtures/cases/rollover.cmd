# Past --max-file-size, writes move to the next data file. Each record
# below is 14 header bytes plus key and value, 25 bytes, so two fit in 50.
export KANSO_NOW=1700000000000
kv --max-file-size 50 "$STORE" <<'SCRIPT'
put a 0123456789
put b 0123456789
put c 0123456789
put d 0123456789
put e 0123456789
status
SCRIPT
ls "$STORE"
kv "$STORE" get e
