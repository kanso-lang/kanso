#!/bin/sh
# Every gate that reads the published pages, at once.
#
# There are three and they run in three different CI jobs, which is how a
# session comes to run two of them and push. On kanso#1328 the page's
# front_end_visits span still carried the number its golden had moved past.
# page_drift and prose_check were both run on that tree and both pass, because
# neither reads a data-golden span; golden_prose is the only one that does, and
# it turned two jobs red. The second was welfare, which runs golden_prose as
# its last step, so it reported ALREADY RED and could prove nothing about the
# three rows sharing that gate. One span cost a round.
#
#   sh scripts/gates/all_pages.sh           read-only, one summary
#   sh scripts/gates/all_pages.sh --write   rewrite the numbers that drifted
#
# --write reaches golden_prose alone. page_drift asks for an edit to
# docs/compiler.html that a person has to write, and prose_check reports
# sentences to rewrite by hand.
#
# It does NOT stop at the first objection, for the reason all_counters.sh
# gives: each gate reads a different property of the same pages, and the first
# to fail hides the rest -- which is the shape of the round above.
#
# prose_check owns about 28 of the sweep's 30 seconds. It reads all 29 pages
# with a regexp engine written in kanso; the figure is here so a reader waiting
# on it knows it has not hung.
set -e
write=0
[ "$1" = "--write" ] && write=1

# A page gate is a program under scripts/ that reads the published HTML. These
# three do. Five other programs hold a literal docs path and are NOT page
# gates, and tests/every_page_gate_is_in_the_sweep.rs walks the tree for that
# literal and requires every one of them to appear in one list or the other --
# so a page gate added later cannot go unswept the way golden_prose did.
#
# gate:what it reads:takes --write
gates="golden_prose:the data-golden spans against the goldens they name:yes
page_drift:docs/compiler.html against the log's budget of unpublished entries:no
prose_check:docs and docs/book for the three mechanical slop families:no"

# elsewhere:swept by
#
# The book pair reads docs/book and is swept by scripts/book_check.sh, which
# runs both before it replays the sample outputs; the spec checks that claim
# against that file rather than trusting this line. The other three read a docs
# path for something that is not the prose.
elsewhere="book_panels:scripts/book_check.sh
book_quotes:scripts/book_check.sh
browser_differential_run:serves the wasm blob and the samples to a browser
diagnostic_coverage:reads docs/book/samples as kanso source, not as pages
site_smoke:loads a built site in headless chrome"

cargo build --release >/dev/null 2>&1

objected=""
while IFS= read -r row; do
  [ -n "$row" ] || continue
  gate=${row%%:*}
  takes_write=${row##*:}
  printf '=== %s\n' "$gate"
  args=""
  [ "$write" -eq 1 ] && [ "$takes_write" = yes ] && args="-- --write"
  # shellcheck disable=SC2086
  if out=$(./target/release/kanso run "scripts/$gate" $args 2>&1); then
    echo "$out" | sed 's/^/    /'
  else
    echo "$out" | sed 's/^/    /'
    objected="$objected $gate"
  fi
done <<ROWS
$gates
ROWS

if [ -z "$objected" ]; then
  echo "pages: the three page gates agree with what the tree says"
  exit 0
fi
echo "pages objected:$objected"
[ "$write" -eq 1 ] && exit 1
echo "golden_prose --write rewrites drifted numbers; the other two want a human edit"
exit 1
