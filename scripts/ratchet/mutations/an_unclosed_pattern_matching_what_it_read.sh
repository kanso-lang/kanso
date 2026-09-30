#!/bin/sh
# Tell the pattern compiler that every parse ended where the pattern did. An
# open group, class, count or escape and a `)` that closes nothing are then
# matched as whatever the parse happened to read, and the micro corpus
# fixture prints a match where every engine should print the refusal.
set -e
old='  over = parsed_body.pos - length chars - 1'
[ "$(grep -cxF "$old" lib/regexp/regexp.kso)" -eq 1 ]
sed -i.bak 's#^  over = parsed_body.pos - length chars - 1$#  over = 0#' lib/regexp/regexp.kso
rm -f lib/regexp/regexp.kso.bak
grep -qxF '  over = 0' lib/regexp/regexp.kso
