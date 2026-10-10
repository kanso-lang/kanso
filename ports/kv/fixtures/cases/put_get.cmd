# One command per run: each run opens the store, does its work, and exits.
export KANSO_NOW=1700000000000
kv "$STORE" put tea sencha
kv "$STORE" put cake "dango, three to a stick"
kv "$STORE" get tea
kv "$STORE" get cake
kv "$STORE" list
kv "$STORE" get coffee
