# A record larger than the limit still has to go somewhere: a file of its
# own.
export KANSO_NOW=1700000000000
kv --max-file-size 20 "$STORE" <<'SCRIPT'
put small x
put large this value alone is longer than the twenty byte limit
put after y
status
SCRIPT
