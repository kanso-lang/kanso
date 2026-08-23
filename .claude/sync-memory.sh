#!/bin/sh
# Put this machine's global Claude memory where a container can read it.
#
# A session on the web clones this repo and nothing else, so ~/.claude/CLAUDE.md
# — the memory that governs every project, and is deliberately not checked in
# anywhere — is simply absent there. This copies it into a PRIVATE git repo the
# session can attach on demand. kanso itself is public; the memory never lands
# in it.
#
#   sh .claude/sync-memory.sh                 # push ~/.claude/CLAUDE.md
#   sh .claude/sync-memory.sh --show          # say what it would do, do nothing
#
# The destination is $CLAUDE_MEMORY_REMOTE, defaulting to the private repo
# below. Create it once with:
#
#   gh repo create kanso-lang/memory --private
#
# Then, in a session: attach that repo and read its CLAUDE.md. The section
# "Clay's memory, in a container" in this repo's CLAUDE.md says the same thing
# to whoever is reading.
set -eu

memory=${CLAUDE_MEMORY_FILE:-$HOME/.claude/CLAUDE.md}
remote=${CLAUDE_MEMORY_REMOTE:-git@github.com:kanso-lang/memory.git}
clone=${CLAUDE_MEMORY_CLONE:-$HOME/.claude/.memory-sync}

if [ "${1:-}" = "--show" ]; then
  printf 'memory: %s\nremote: %s\nclone:  %s\n' "$memory" "$remote" "$clone"
  exit 0
fi

if [ ! -f "$memory" ]; then
  printf 'no memory file at %s — set CLAUDE_MEMORY_FILE to where it lives\n' \
    "$memory" >&2
  exit 1
fi

# Built rather than cloned, because a repo created and never pushed to has no
# branch to clone and `git clone` fails on it. Init and fetch reach the same
# place from either state, and the clone is kept between runs.
if [ ! -d "$clone/.git" ]; then
  mkdir -p "$clone"
  git -C "$clone" init --quiet
  git -C "$clone" remote add origin "$remote"
fi
git -C "$clone" fetch --quiet origin || true

# main is the branch this writes. Starting it from the remote's is what makes a
# second machine add to the history instead of forking it — and the fork is not
# theoretical: a repo whose default branch is `master` hands a fresh clone an
# empty checkout, and the push after it is rejected as non-fast-forward.
if git -C "$clone" rev-parse --verify --quiet origin/main >/dev/null; then
  git -C "$clone" checkout --quiet -B main origin/main
else
  git -C "$clone" symbolic-ref HEAD refs/heads/main
fi

cp "$memory" "$clone/CLAUDE.md"
git -C "$clone" add CLAUDE.md

# An unchanged memory makes no commit, so the history reads as the memory's
# own edits rather than as a record of how often this ran.
if git -C "$clone" diff --quiet --cached; then
  echo 'memory unchanged'
  exit 0
fi

git -C "$clone" commit --quiet \
  -m "memory from $(hostname -s 2>/dev/null || echo this machine), $(date -u +%Y-%m-%d)"
git -C "$clone" push --quiet -u origin HEAD:refs/heads/main
echo 'memory pushed'
