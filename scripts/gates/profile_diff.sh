#!/bin/sh
# Where two callgrind profiles of the same binary disagree, function by function.
#
# The compile gates read one number out of a profile: `kanso::main` inclusive.
# When two readings of one binary on one machine disagree, that number says how
# much and nothing about where, and every hunt so far has had to guess. On
# 2026-09-16 three rows each read exactly 13 low against goldens taken from an
# earlier run, and the shape of that -- one constant, three unrelated compiles
# -- is a fact about a per-process term rather than about the compiler. Which
# term is a question the profiles already answer and nothing was asking them.
#
# So: total each function's SELF cost in both files, join on the name, and
# print every function that moved. A per-process term shows up as one or two
# frames near the process's entry; a real compiler difference spreads across
# the passes that changed. The two are not confusable once the list is printed.
#
# THE PROFILE IS PARSED HERE RATHER THAN THROUGH callgrind_annotate, and that
# is not a preference. `--threshold` is a percentage of the total and 100 is
# its maximum, so the tool stops as soon as the running percentage ROUNDS to
# 100 -- on a profile whose hot function is 99.999% of it, the whole tail is
# dropped, and the tail is this instrument's entire subject. A thirteen-
# instruction move in a hundred and thirty-two million lives there. The first
# draft of this script read the annotated table and reported two profiles that
# differ by exactly that as identical.
set -e
a=$1
b=$2
if [ ! -f "$a" ] || [ ! -f "$b" ]; then
  echo "profile_diff: want two callgrind output files" >&2
  exit 2
fi

# Callgrind's format, only as far as self cost needs it:
#
#   positions: line          how many leading fields on a cost line are position
#   fn=(7) some::function    a name, defining compression id 7
#   fn=(7)                   the same name again, by id
#   calls=N target           the NEXT cost line is a call, and its cost is the
#                            callee's -- self cost must not count it
#   12 3456                  a cost line: position(s), then one cost per event
self() {
  awk '
    /^positions:/ { np = NF - 1; next }
    /^calls=/     { skip = 1; next }
    # A name can be DEFINED on a call line -- cfn= for a callee, cfi=/cfl= for
    # its file -- and referenced later by id alone on an fn=/fl= line. The
    # cursor must not move for these, but the id must be recorded, or the row
    # comes out with half its name missing.
    /^(cfn|cfi|cfl|cob)=/ {
      key = substr($0, 1, index($0, "=") - 1)
      rest = substr($0, index($0, "=") + 1)
      if (match(rest, /^\([0-9]+\)/)) {
        id = substr(rest, 2, RLENGTH - 2)
        name = substr(rest, RLENGTH + 1)
        sub(/^ +/, "", name)
        if (name != "") {
          if (key == "cfn") fname[id] = name
          else if (key != "cob") flname[id] = name
        }
      }
      next
    }
    # fl= names the file a function was compiled from; fi= and fe= switch it
    # mid-function, which is how an inlined body is attributed to the header it
    # came from. They share one compression id space with fl=, and the rows this
    # instrument prints are the file:function pairs callgrind_annotate prints.
    /^(fl|fi|fe)=/ {
      rest = substr($0, 4)
      if (match(rest, /^\([0-9]+\)/)) {
        id = substr(rest, 2, RLENGTH - 2)
        name = substr(rest, RLENGTH + 1)
        sub(/^ +/, "", name)
        if (name != "") flname[id] = name
        file = flname[id]
      } else {
        file = rest
      }
      next
    }
    /^fn=/ {
      rest = substr($0, 4)
      if (match(rest, /^\([0-9]+\)/)) {
        id = substr(rest, 2, RLENGTH - 2)
        name = substr(rest, RLENGTH + 1)
        sub(/^ +/, "", name)
        if (name != "") fname[id] = name
        fn = fname[id]
      } else {
        fn = rest
      }
      next
    }
    /^[0-9*+-]/ {
      if (skip) { skip = 0; next }
      if (np < 1) np = 1
      cost = $(np + 1)
      if (cost ~ /^[0-9]+$/) total[file ":" fn] += cost
      next
    }
    { skip = 0 }
    END { for (f in total) printf "%s\t%d\n", f, total[f] }
  ' "$1"
}

# Named for this process. Two callers in one tree is the ordinary case -- the
# three compile gates each diff their own pair, and the spec runs two
# comparisons at once -- and a fixed path makes one of them read the other's
# answer.
pd=${TMPDIR:-/tmp}/pd.$$
trap 'rm -f "$pd.a" "$pd.b"' EXIT INT TERM
self "$a" > "$pd.a"
self "$b" > "$pd.b"

awk -F'\t' '
  NR == FNR { first[$1] = $2; seen[$1] = 1; next }
  {
    here[$1] = 1
    d = $2 - first[$1]
    if (d != 0) { delta[$1] = d; total += d }
  }
  END {
    for (f in seen) if (!here[f] && first[f] != 0) { delta[f] = -first[f]; total -= first[f] }
    n = 0
    for (f in delta) n++
    if (n == 0) {
      print "profile_diff: every function agrees; the two profiles are identical"
      exit 0
    }
    printf "profile_diff: %d function(s) moved, %d instructions net\n", n, total
    for (f in delta) printf "%12d  %s\n", delta[f], f
  }
' "$pd.a" "$pd.b" | { read -r head; echo "$head"; sort -k1,1n; }
