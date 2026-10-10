# When every key is deleted, a merge leaves no data files at all.
export KANSO_NOW=1700000000000
kv "$STORE" <<'SCRIPT'
put a 1
put b 2
delete a
delete b
merge
list
SCRIPT
ls "$STORE"
kv "$STORE" merge
