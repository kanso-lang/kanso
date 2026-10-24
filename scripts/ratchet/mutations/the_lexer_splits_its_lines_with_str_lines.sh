#!/bin/sh
# The lexer finds each line of a source a word at a time. This mutation puts
# `str::lines` back, which calls memchr once a line and strips a `\r` the same
# way. Same lines, and the front end reads heavier.
set -e
grep -qxF "    let raws = source_lines(source);" src/lexer.rs || {
  echo "the lexer's line split moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    let raws = source_lines(source);$/    let raws: Vec<\&str> = source.lines().collect();/; s/^fn source_lines(source: &str) -> Vec<&str> {$/#[allow(dead_code)]\nfn source_lines(source: \&str) -> Vec<\&str> {/' src/lexer.rs
grep -qxF "    let raws: Vec<&str> = source.lines().collect();" src/lexer.rs
