# Each run starts a new data file; the newest record for a key wins, and the
# ones it shadows are dead bytes until a merge.
KANSO_NOW=1700000000000 kv "$STORE" put colour red
KANSO_NOW=1700000060000 kv "$STORE" put colour green
KANSO_NOW=1700000120000 kv "$STORE" put colour blue
kv "$STORE" get colour
kv "$STORE" dump
kv "$STORE" status
