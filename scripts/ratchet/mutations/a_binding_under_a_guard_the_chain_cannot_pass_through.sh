#!/bin/sh
# The beat's chain test follows an accumulator through a body's bindings,
# read through the guards above them since 2026-09-16. This puts the top-level
# read back: a name bound under a guard is invisible to the chain, and the
# fixture's `opened` stops being a link the accumulator passes through.
set -e
grep -q '            Stmt::Expr(Expr::Guard { rest, .. }) => binds_into(rest, out),' src/beat.rs
sed -i 's|            Stmt::Expr(Expr::Guard { rest, .. }) => binds_into(rest, out),|            Stmt::Expr(Expr::Guard { .. }) => {}|' src/beat.rs
grep -q 'Stmt::Expr(Expr::Guard { .. }) => {}' src/beat.rs
