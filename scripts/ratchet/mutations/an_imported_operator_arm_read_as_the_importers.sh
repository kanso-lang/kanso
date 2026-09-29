#!/bin/sh
# An imported operator arm is checked as though the importer wrote it.
#
# The import prefixes every name a module declares except an operator's, so
# the checker reads the type an arm names to tell an imported `+` from the
# importer's own. With that test gone, a module's `+` that builds the module's
# own record is refused as a foreign construction once the module is
# imported: the micro corpus, which runs each fixture as a library, and
# sibling_types' arm fixtures both stop at the check.
set -e
f=src/check.rs
grep -q "fn imported_arm" "$f"
line='    qualified && !local'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the imported-arm pattern test moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    qualified \&\& !local$/    false \&\& qualified \&\& !local/' "$f"
grep -qxF '    false && qualified && !local' "$f"
