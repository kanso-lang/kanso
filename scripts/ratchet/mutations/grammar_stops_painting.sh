#!/bin/sh
set -e
sed -i.bak 's/"name": "keyword/"name": "kEyWoRd/g' docs/kanso.tmLanguage.json
rm -f docs/kanso.tmLanguage.json.bak
grep -q 'kEyWoRd' docs/kanso.tmLanguage.json
