#!/bin/sh
set -e
printf 'pub fn lint_bait(v: &Vec<u8>) -> usize { v.len() }\n' >> src/lib.rs
