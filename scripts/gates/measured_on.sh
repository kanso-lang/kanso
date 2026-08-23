#!/bin/sh
# Two veins here hold numbers that belong to the host that measured them, and
# until this existed nothing said so except prose. The instructions golden has
# warned in its header since it was written that a row must never be read
# against a number measured somewhere else. The warning did not stop this
# branch: the gate was run in a container, it printed a diff, and the
# container's numbers went into the file over the runner's. A diff invites a
# paste.
#
# So a golden that belongs to a host says which one, in a line the gate reads:
#
#     # measured-on glibc=2.39-0ubuntu8.8
#     # measured-on clang=18.1.3
#
# The line names the facts and this reads exactly those. Anywhere they do not
# match it refuses, before the expensive part of the gate runs, and prints no
# numbers at all — there is nothing to copy.
#
# The granularities differ on purpose. glibc carries its Ubuntu revision
# because two revisions of the same upstream release demonstrably moved the
# rows: 2.39-0ubuntu8.7 against 2.39-0ubuntu8.8 is about four hundred retired
# instructions before main, and a few thousand where memcpy carries the work.
# clang carries only the upstream version, because what selects codegen is the
# release and nothing here shows a package revision moving a byte of .text.
# Pin what has been shown to matter; a fact pinned tighter than the evidence
# reds the gate on changes that are not changes.
set -e
golden=$1
if [ -z "$golden" ]; then
  echo "::error::this wants the golden to check as its argument"
  exit 2
fi
want=$(sed -n 's/^# measured-on //p' "$golden")
if [ -z "$want" ]; then
  echo "::error::$golden names no host, so nothing can say whether its rows"
  echo "::error::may be read here. Add a measured-on line naming the"
  echo "::error::facts its numbers depend on."
  exit 1
fi
have=""
for fact in $want; do
  case "${fact%%=*}" in
    glibc)
      v=$(ldd --version 2>/dev/null | head -1 | sed -n 's/.*GLIBC \([^)]*\)).*/\1/p')
      if [ -z "$v" ]; then
        v=$(ldd --version 2>/dev/null | head -1 | awk '{print $NF}')
      fi
      have="$have glibc=${v:-unknown}"
      ;;
    clang)
      v=$(clang --version 2>/dev/null | sed -n '1s/.*version \([0-9][0-9.]*\).*/\1/p')
      have="$have clang=${v:-unknown}"
      ;;
    *)
      echo "::error::$golden names a fact this gate cannot read: ${fact%%=*}"
      exit 2
      ;;
  esac
done
have=$(echo "$have" | sed 's/^ //')
echo "$golden: measured-on $want; here $have"
if [ "$want" != "$have" ]; then
  echo "::error::these rows were measured on $want and this host is $have,"
  echo "::error::so the two cannot be compared. Do not regenerate $golden"
  echo "::error::from here — let CI measure it and copy the rows out of the"
  echo "::error::job log. If the runner image itself moved, every row moves"
  echo "::error::with it and none has regressed: regenerate them in one go,"
  echo "::error::update the measured-on line, and say so in"
  echo "::error::design/compiler-log.md."
  exit 1
fi
