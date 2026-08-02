#!/bin/sh
# Clone a sibling repository, preferring a branch named after the branch under
# test. A language change and the sweep it forces in kq, vse and kanso-json are
# one change; naming the branch the same in each is how they are checked
# together.
set -e
repo="$1"
into="$2"
branch="${GITHUB_HEAD_REF:-}"
url="https://github.com/kanso-lang/$repo"

if [ -n "$branch" ] && git ls-remote --exit-code --heads "$url" "$branch" >/dev/null 2>&1; then
  echo "$repo: $branch"
  git clone --depth 1 --branch "$branch" "$url" "$into"
else
  echo "$repo: main"
  git clone --depth 1 "$url" "$into"
fi
