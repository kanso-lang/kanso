#!/bin/sh
# The module writer marks the strings the body names in the same pass that
# reads its calls, and scans only the type tables for them afterwards. This
# mutation scans the body for them again, as a pass of its own did. Same
# marks, and the browser compile row reads heavier.
set -e
t='            named.iter_mut().zip(named_strings(&[&self.globals], self.strings.len()))'
[ "$(grep -cxF "$t" src/codegen/module.rs)" -eq 1 ] || {
  echo "the string marks moved; this mutation needs rewriting" >&2
  exit 1
}
awk -v t="$t" '
  $0 == t {
    print "            named.iter_mut().zip(named_strings("
    print "                &pieces.iter().map(String::as_str).chain([self.globals.as_str()]).collect::<Vec<_>>(),"
    print "                self.strings.len(),"
    print "            ))"
    next
  }
  { print }' src/codegen/module.rs > src/codegen/module.rs.new
mv src/codegen/module.rs.new src/codegen/module.rs
if grep -qxF "$t" src/codegen/module.rs; then exit 1; fi
