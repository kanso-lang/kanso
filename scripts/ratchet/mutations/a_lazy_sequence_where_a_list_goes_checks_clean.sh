#!/bin/sh
# Stop asking whether a std/list call's lazy sequence lands where a builtin
# takes a list, so `text/join (list/map …) ""` checks clean again.
set -e
sed -i.bak 's/^                lazy_where_a_list_goes(cur, aliases, returns, &shadow, none_diags);$//' src/check/after_infer.rs
rm -f src/check/after_infer.rs.bak
! grep -q 'lazy_where_a_list_goes(cur' src/check/after_infer.rs
