# A merge keeps the newest record of every live key, drops overwritten
# records and deletes, writes a hint file beside its output, and removes
# the files it read.
export KANSO_NOW=1700000000000
kv "$STORE" put a 1
kv "$STORE" put b 2
kv "$STORE" put a 3
kv "$STORE" delete b
kv "$STORE" put c 4
ls "$STORE"
kv "$STORE" merge
ls "$STORE"
kv "$STORE" dump
bytes "$STORE/7.bitcask.hint"
kv "$STORE" list
kv "$STORE" get a
kv "$STORE" get b
