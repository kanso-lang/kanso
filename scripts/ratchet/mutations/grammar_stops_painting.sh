#!/bin/sh
# The editor grammar stops naming a string a string, so every quoted word in
# every kanso file opens the colour of ordinary code. The string rule is the
# one to break here: it spans a begin and an end rather than matching text, and
# a gate that gathers only matching rules cannot see it at all.
set -e
grammar=editors/kanso/syntaxes/kanso.tmLanguage.json
grep -q 'string\.quoted\.double' "$grammar"
sed -i.bak 's/string\.quoted\.double/string.quoted.other/g' "$grammar"
rm -f "$grammar.bak"
grep -qv 'string\.quoted\.double' "$grammar"
