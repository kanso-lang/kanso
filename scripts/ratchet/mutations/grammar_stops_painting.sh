#!/bin/sh
# The editor grammar stops naming a type as a type, so every kanso file opens
# with its type names the colour of ordinary words.
set -e
grammar=editors/kanso/syntaxes/kanso.tmLanguage.json
grep -q 'entity\.name\.type' "$grammar"
sed -i.bak 's/entity\.name\.type/entity.name.other/g' "$grammar"
rm -f "$grammar.bak"
grep -qv 'entity\.name\.type' "$grammar"
