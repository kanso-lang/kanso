#!/bin/sh
# Every canonical name the loader mints is `qual`, a slash and `name` joined,
# and `ast::qualified` writes those three pieces into one exactly-sized
# allocation. `format!` does the same job through the formatting machinery: a
# `{}` on a `&str` goes out through `Display::fmt`, `Formatter::pad` and
# `write_str`, into a string that starts empty and grows. Qualifying the
# compile corpus makes 712 of these joins.
#
# This mutation writes the helper back to the `format!` it replaced. The
# compile rows are the witness, measured on this branch against the shipped
# binary: the module corpus rises 690,734 instructions (+1.5138%), the entry
# corpus 2,306,556 (+1.5222%) and the library corpus 2,273,585 (+1.4889%).
# Nothing else about the compile changes -- the strings the two shapes produce
# are byte-identical, so every golden but the instruction veins is untouched,
# which is exactly the shape this gate exists to see.
set -e
grep -q '^    let mut joined = String::with_capacity(qual.len() + 1 + name.len());$' src/ast.rs || {
  echo "ast::qualified's sizing line changed shape; this needs rewriting" >&2
  exit 1
}
grep -q '^    joined.push_str(name);$' src/ast.rs || {
  echo "ast::qualified's final push changed shape; this needs rewriting" >&2
  exit 1
}
perl -0pi -e 's/    let mut joined = String::with_capacity\(qual\.len\(\) \+ 1 \+ name\.len\(\)\);\n    joined\.push_str\(qual\);\n    joined\.push\(.\/.\);\n    joined\.push_str\(name\);\n    joined\n/    format!("{qual}\/{name}")\n/' src/ast.rs
grep -q '^    format!("{qual}/{name}")$' src/ast.rs
