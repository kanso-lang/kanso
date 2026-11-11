#!/bin/sh
# Before a job installs packages: make sure apt reads a mirror whose name
# resolves on this runner.
#
# A hosted runner's apt reads /etc/apt/apt-mirrors.txt, with Azure's mirror
# first. On 2026-11-11 that name stopped resolving on the runners for hours
# ("Temporary failure resolving 'azure.archive.ubuntu.com'") while it
# resolved everywhere else, and every job that installs a package died in
# the install, in three repositories, before any test ran. The other names
# in the list did not save it.
#
# So this asks first. When the first mirror resolves, nothing changes. When
# it does not, the list is rewritten to every other mirror below whose name
# does resolve, in order, and apt is told to try a fetch again before giving
# up. One other mirror was not enough: the first run that fell back to a
# single one installed clang from it and then failed to connect for the
# next package. apt checks what it downloads against the archive's
# signature, so which mirror served the bytes does not change what is
# installed.
#
# The KANSO_APT_* variables are for
# tests/apt_reads_a_mirror_that_resolves.rs, which runs this against a list
# and a resolver of its own.
set -u
list=${KANSO_APT_MIRRORS:-/etc/apt/apt-mirrors.txt}
again=${KANSO_APT_RETRIES:-/etc/apt/apt.conf.d/80-fetch-again}
lookup=${KANSO_APT_LOOKUP:-getent hosts}
as_root=${KANSO_APT_SUDO-sudo}
first=azure.archive.ubuntu.com
others="archive.ubuntu.com/ubuntu/
mirrors.kernel.org/ubuntu/
mirrors.mit.edu/ubuntu/
mirror.math.princeton.edu/pub/ubuntu/
mirrors.ocf.berkeley.edu/ubuntu/"

# Asked twice, a moment apart: one lost answer is not an outage.
resolves() {
  $lookup "$1" >/dev/null 2>&1 && return 0
  sleep "${KANSO_APT_PAUSE:-2}"
  $lookup "$1" >/dev/null 2>&1
}

[ -f "$list" ] || exit 0
grep -q "$first" "$list" || exit 0
if resolves "$first"; then
  exit 0
fi
echo "apt: $first does not resolve on this runner"
found=""
rank=0
for other in $others; do
  if resolves "${other%%/*}"; then
    rank=$((rank + 1))
    found="${found}http://$other	priority:$rank
"
    echo "apt: reading http://$other"
  fi
done
if [ "$rank" -eq 0 ]; then
  echo "apt: no other mirror resolves either; the install will say what it could not fetch"
  exit 0
fi
printf "$found" | $as_root tee "$list" >/dev/null
printf 'Acquire::Retries "4";\n' | $as_root tee "$again" >/dev/null
