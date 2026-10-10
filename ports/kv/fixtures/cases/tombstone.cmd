# A delete appends a record with value size 0xffffffff and no value.
export KANSO_NOW=1700000000000
kv "$STORE" put k v
kv "$STORE" delete k
bytes "$STORE/2.bitcask.data"
kv "$STORE" get k
kv "$STORE" delete k
kv "$STORE" dump
kv "$STORE" list
