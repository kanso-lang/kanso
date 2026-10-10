# A hint file that does not parse is ignored in favour of its data file.
export KANSO_NOW=1700000000000
kv "$STORE" <<'SCRIPT'
put a 1
put bb 22
merge
SCRIPT
tear "$STORE/2.bitcask.hint" 25
kv "$STORE" list
kv "$STORE" get bb
