#!/bin/sh
# Clone a sibling repository, preferring a branch named after the branch under
# test. A language change and the sweep it forces in kq, vse and kanso-json are
# one change; naming the branch the same in each is how they are checked
# together.
#
# That preference is right for source and wrong for numbers. A performance
# golden is a claim about the compiler, so letting a compiler change bring the
# values it will be judged against dissolves the gate: kanso #639 pointed this
# at a kq branch carrying a carry_dedup already moved from 1 to 2721, the diff
# compared 2721 against 2721, and a change that made kq's headline row ten
# times slower than jq shipped green.
#
# So a coordinated branch supplies the sibling's SOURCE and never its
# performance goldens. Those are restored from main, and a counter that moves
# has to be argued in front of the number it moved from.
#
# A change that genuinely moves a sibling's counters — a syntax change that
# alters what the sweep allocates — would otherwise deadlock, because main's
# golden can never match it. The escape is a file named sibling-goldens-move
# in THIS repository, on the branch under test, saying which counters move and
# why. That keeps the loosening inside the change being reviewed, where #639's
# was a side effect of naming a branch.
set -e
repo="$1"
into="$2"
branch="${GITHUB_HEAD_REF:-}"
url="https://github.com/kanso-lang/$repo"

if [ -n "$branch" ] && git ls-remote --exit-code --heads "$url" "$branch" >/dev/null 2>&1; then
  git clone --depth 1 --branch "$branch" "$url" "$into"
  if [ -f sibling-goldens-move ]; then
    echo "$repo: $branch (goldens from the branch — sibling-goldens-move says:)"
    sed 's/^/  | /' sibling-goldens-move
    exit 0
  fi
  echo "$repo: $branch (performance goldens from main)"
  git -C "$into" remote set-branches origin main
  git -C "$into" fetch --depth 1 origin main
  restored=""
  for path in $(git -C "$into" ls-tree -r --name-only origin/main | grep -E '(golden|stamp|exceptions)' || true); do
    git -C "$into" checkout origin/main -- "$path" 2>/dev/null && restored="$restored $path"
  done
  if [ -n "$restored" ]; then
    echo "$repo: restored from main:$restored"
  fi
else
  echo "$repo: main"
  git clone --depth 1 "$url" "$into"
fi
