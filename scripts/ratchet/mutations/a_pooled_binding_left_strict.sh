#!/bin/sh
# A binding the pool makes is a cell whatever its uses, since a strict one
# runs the work before the sleep. This mutation leaves it to the demand
# analysis, which keeps it strict: the overflow is raised before "first".
set -e
line='            let pooled = name.as_str().starts_with(crate::ahead::POOLED);'
[ "$(grep -cxF "$line" src/demand.rs)" -eq 1 ] || {
  echo "the pooled binding's vote moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            let pooled = name\.as_str()\.starts_with(crate::ahead::POOLED);$/            let pooled = false;/' src/demand.rs
if grep -qxF "$line" src/demand.rs; then exit 1; fi
