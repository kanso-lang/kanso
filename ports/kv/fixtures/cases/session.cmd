# With no command, kv reads a script from standard input and keeps the
# store open between lines. Blank lines and # comments are skipped.
export KANSO_NOW=1700000000000
kv "$STORE" <<'SCRIPT'
# stock the shelf
put matcha 600
put hojicha 450

put genmaicha 380
get matcha
put matcha 650
get matcha
delete hojicha
get hojicha
list
status
SCRIPT
