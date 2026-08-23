#!/bin/sh
# A python3 call sneaks into a shell harness — the #854/#862 shape, which
# twice followed the 2026-08-09 port within days.
set -e
printf 'python3 -c "pass"\n' >> scripts/book_check.sh
grep -q 'python3 -c "pass"' scripts/book_check.sh
