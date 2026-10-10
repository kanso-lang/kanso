# A script is checked before it runs, so a bad line applies nothing.
export KANSO_NOW=1700000000000
kv "$STORE" <<'SCRIPT'
put a 1
put b 2
get
put c 3
SCRIPT
echo "[exit $?]"
kv "$STORE" list
ls "$STORE"
