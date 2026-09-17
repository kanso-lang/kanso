#!/bin/sh
# Point this clone's git at the tracked hooks/ directory.
#
# Git will not let a repository set core.hooksPath for itself — a repo that
# could would run its own code on clone — so this is one command per clone,
# and there is no arrangement that removes it.

set -e

cd "$(git rev-parse --show-toplevel)"
git config core.hooksPath hooks

echo "core.hooksPath = hooks"
