# With --expiry, a record older than that many seconds reads as absent, and
# a merge drops it.
KANSO_NOW=1700000000000 kv "$STORE" put old early
KANSO_NOW=1700000090000 kv "$STORE" put new late
KANSO_NOW=1700000100000 kv --expiry 50 "$STORE" list
KANSO_NOW=1700000100000 kv --expiry 50 "$STORE" get old
KANSO_NOW=1700000100000 kv "$STORE" get old
KANSO_NOW=1700000100000 kv --expiry 50 "$STORE" merge
KANSO_NOW=1700000100000 kv "$STORE" list
