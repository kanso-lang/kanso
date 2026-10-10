# Merge output obeys the size limit too, and the store reopens from the
# hint files it left.
export KANSO_NOW=1700000000000
kv --max-file-size 50 "$STORE" <<'SCRIPT'
put one first
put two second
put three third
put one uno
put four fourth
put five fifth
delete four
merge
status
SCRIPT
ls "$STORE"
kv "$STORE" list
kv "$STORE" get one
