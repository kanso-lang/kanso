#!/bin/sh
# The survey is no longer handed the import an entry with a derive line makes
# without writing it. It counts one import of std/derive where there are two,
# does not keep the module from its first compile, and the entry compiles it
# again.
set -e
grep -q 'source.contains("\\nderive ")' src/lib/modules.rs || {
  echo "from_entry's test for a derive line moved; this mutation needs rewriting" >&2
  exit 1
}
old='    let derives = source.starts_with("derive ") || source.contains("\nderive ");'
[ "$(grep -cxF "$old" src/lib/modules.rs)" -eq 1 ]
sed -i 's/^    let derives = source\.starts_with("derive ") || source\.contains("\\nderive ");$/    let derives = false \&\& source.starts_with("derive ");/' src/lib/modules.rs
grep -q 'let derives = false && source.starts_with("derive ");' src/lib/modules.rs
