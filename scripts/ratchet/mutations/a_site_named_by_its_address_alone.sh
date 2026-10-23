#!/bin/sh
# Compiled code stamps a death's place as an id in k_site_id, and k_died
# reads it back through the program's k_site_name. This mutation has the
# runtime read only k_site, the address a stamp used to store, so a death
# behind an id stamp prints no `failed in` line. The runtime corpus goes red.
set -e
line='    return k_site_id ? k_site_name(k_site_id) : k_site;'
grep -qxF "$line" src/runtime/core.c || {
  echo "the site read-back moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    return k_site_id ? k_site_name(k_site_id) : k_site;$/    return k_site;/' src/runtime/core.c
if grep -qxF "$line" src/runtime/core.c; then exit 1; fi
