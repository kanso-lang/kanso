# The last entry of a hint file carries a CRC-32 of the entries before it,
# so a flipped bit in a hint is noticed and the data file is read instead.
export KANSO_NOW=1700000000000
kv "$STORE" <<'SCRIPT'
put alpha 1
put beta 2
merge
SCRIPT
bytes "$STORE/2.bitcask.hint"
poke "$STORE/2.bitcask.hint" 3 01
kv "$STORE" get beta
kv "$STORE" dump
