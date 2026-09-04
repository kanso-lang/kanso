#!/bin/sh
# What the compile row reads on THIS host, and where inside it the number sits.
#
#   sh scripts/compile_row_probe.sh [callgrind-out-file]
#
# WHY THIS EXISTS SEPARATELY FROM THE GATE. scripts/gates/compile_instructions.sh
# refuses on a host whose toolchain is not the goldens' — a container stops
# before it measures, and the reasons are in scripts/gates/host_gate.sh. That
# refusal is about COMPARING, and it is right: a number counted under a
# different glibc cannot be read against a recorded row. Measuring is a
# different act, and the investigation of 2026-09-04 needed it — the row's
# binary-to-binary drift was named by building ten binaries here and reading
# each one.
#
# THIS SCRIPT MAY NEVER REGENERATE A GOLDEN. It prints; it writes nothing under
# bench/. A row is re-sat by CI and copied out of the job log, and nothing here
# changes that. Anyone reaching for this to fix a red gate is holding the wrong
# tool.
#
# WHAT THE THREE NUMBERS ARE. The row is the whole process, and the process is
# not only the compiler:
#
#   row      what the gate would count, the callgrind summary
#   maps     pthread_getattr_np inclusive. std::rt::lang_start_internal calls
#            it to place Rust's stack guard, and it parses /proc/self/maps
#            with getline and sscanf. About 0.27% of the row.
#   program  `kanso::main` inclusive, which is everything the compiler actually
#            does. It is this crate's own symbol rather than a standard-library
#            one, for the reason scripts/gates/compile_instructions.sh gives:
#            CI's toolchain does not emit the closure the gate first read.
#
# The three numbers below were read with `std::rt::lang_start::{{closure}}` as
# the program frame, which is what the gate anchored on for one round;
# `kanso::main` sits exactly 10 instructions below it on every profile retained,
# so the spans quoted here hold under either anchor.
#
# Measured on 2026-09-04: the row moves up to 3,963 between binaries built from
# sources that differ only in code or data no execution reaches, and `program`
# moves 1,028 of it. Where the difference is data the drop is total — a 64 KiB
# .bss addition moved the row 2,130 while `program` held at 41,878,959 to the
# instruction — and where it is CODE it is not: 7,632 bytes of .text nothing
# calls moved `program` 402.
#
# An earlier version of this comment said all the movement was in `maps`. It is
# not, and it was written before the .text case was probed with the frame read
# out. The split is here so the next reader can see which half a number moved
# in, which is worth having and is not the same as a number that cannot move.
#
# The environment is emptied and the tunables pinned for the reasons
# scripts/gates/compile_instructions.sh gives at length: the kernel copies the
# environment onto the new process's stack, and glibc sizes its malloc and
# string thresholds from the cache sizes it reads out of the CPU.
set -e
out=${1:-/tmp/cg.probe}
sh scripts/gates/library_box.sh
box=/tmp/kanso-compile-ir

tune=glibc.cpu.x86_data_cache_size=0x8000
tune=$tune:glibc.cpu.x86_shared_cache_size=0x2000000
tune=$tune:glibc.cpu.x86_non_temporal_threshold=0x1800000
tune=$tune:glibc.cpu.x86_rep_movsb_threshold=0x840
tune=$tune:glibc.cpu.x86_rep_stosb_threshold=0x800
tune=$tune:glibc.malloc.arena_max=1
tune=$tune:glibc.malloc.mmap_threshold=131072
tune=$tune:glibc.malloc.trim_threshold=131072
tune=$tune:glibc.malloc.top_pad=131072
tune=$tune:glibc.malloc.tcache_count=7

printf 'probe sha=%.12s ' "$(sha256sum "$box/kanso" | cut -d' ' -f1)"
size --format=sysv "$box/kanso" \
  | awk '/^\.(text|data|bss)[ \t]/ { printf "%s=%s ", $1, $2 }'
echo

(
  cd "$box"
  env -i PATH=/usr/bin:/bin GLIBC_TUNABLES="$tune" valgrind --tool=callgrind \
    --callgrind-out-file="$out" ./kanso check lib/json >/dev/null 2>/dev/null
)

printf 'probe row=%s\n' "$(grep -o '^summary: [0-9]*' "$out" | tr -dc 0-9)"

# callgrind_annotate prints these with thousands separators; strip them so the
# line can be diffed against another sitting without a second pass.
if command -v callgrind_annotate >/dev/null; then
  callgrind_annotate --inclusive=yes --threshold=100 "$out" 2>/dev/null \
    | awk '
        /pthread_getattr_np@@/ && !maps { gsub(/,/, "", $1); maps = $1 }
        /kanso::main/ && !prog { gsub(/,/, "", $1); prog = $1 }
        END { printf "probe maps=%s program=%s\n", maps, prog }'
fi
