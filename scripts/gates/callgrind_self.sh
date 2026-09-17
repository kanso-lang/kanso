# Self cost per file:function, read straight out of a callgrind profile.
#
# callgrind_annotate would do this, except that its --threshold is a percentage
# and stops once the running total ROUNDS to the figure asked for, so the tail
# it drops is exactly where a thirteen-instruction frame lives. This reads the
# file.
#
# Output is one `file:function<TAB>self` row per frame, unsorted.
#
#   sh scripts/gates/callgrind_self.sh /tmp/cg.compile
set -e
[ -f "$1" ] || { echo "no such profile: $1" >&2; exit 2; }
awk '
  # `positions:` says how many leading position fields a cost line carries, so
  # the event column is the one after them.
  /^positions:/ { np = NF - 1; next }
  # A `calls=` line means the NEXT cost line is the callee`s INCLUSIVE cost
  # charged to this call site. Counting it as self cost double-counts the
  # whole subtree.
  /^calls=/ { skip = 1; next }
  # A name can be DEFINED on a call line -- cfn= for the callee, cfi=/cfl= for
  # its file -- and referenced later by id alone. The cursor must not move for
  # these, but the id has to be recorded or the later row loses its name.
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
  # came from. All three share one compression id space.
  /^(fl|fi|fe)=/ {
    rest = substr($0, 4)
    if (match(rest, /^\([0-9]+\)/)) {
      id = substr(rest, 2, RLENGTH - 2)
      name = substr(rest, RLENGTH + 1)
      sub(/^ +/, "", name)
      if (name != "") flname[id] = name
      file = flname[id]
    } else { file = rest }
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
    } else { fn = rest }
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
