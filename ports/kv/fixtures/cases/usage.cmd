# Mistakes on the command line.
kv
echo "[exit $?]"
kv "$STORE" frobnicate
echo "[exit $?]"
kv "$STORE" put lonely
echo "[exit $?]"
kv "$STORE" list extra
echo "[exit $?]"
kv --max-file-size lots "$STORE" list
echo "[exit $?]"
kv --expiry
