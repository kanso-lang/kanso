# A merge writes its output before it removes anything, and removes the old
# files oldest first. Wherever a crash stops the removal, the files left
# over are the newest of the old ones, so every delete that remains still
# sits after the put it shadows, and the merged copies come last of all.
export KANSO_NOW=1700000000000
kv "$STORE" put a 1
kv "$STORE" put b 2
kv "$STORE" put a 3
kv "$STORE" delete b
mkdir saved
cp "$STORE"/*.data saved/
kv "$STORE" merge
cp saved/3.bitcask.data saved/4.bitcask.data "$STORE"/
ls "$STORE"
kv "$STORE" list
kv "$STORE" get a
kv "$STORE" get b
cp saved/*.data "$STORE"/
ls "$STORE"
kv "$STORE" status
kv "$STORE" get a
kv "$STORE" get b
