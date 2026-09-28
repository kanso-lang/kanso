#!/bin/sh
# Type the render group's parameter from its explicit calls alone, as inference
# did before 2026-09-28. An explicit `render/to_string 11` then proves it an
# int, the compiled builds pass it as a raw word, and an interpolated record
# reaches the renderer as its tag: the micro fixture
# an_interpolated_record_reaches_render_whole prints `7`.
set -e
grep -q '^        if decl.name == "render/to_string" {$' src/infer.rs
sed -i.bak 's|^        if decl.name == "render/to_string" {$|        if decl.name == "render/to_string!" {|' src/infer.rs
rm -f src/infer.rs.bak
grep -q '^        if decl.name == "render/to_string!" {$' src/infer.rs
