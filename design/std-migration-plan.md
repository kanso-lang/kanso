# the effects/text migration: the last of the ambient set

Gavel 1's remaining names leave ambience. Mechanism chosen: REAL WRAPPER
MODULES + gated internal names (over a virtual-module table) — the std
source stays readable kanso, matching the render_value precedent, at the
cost of one tailcc hop per call.

## The mechanism

1. **Internal names**: engines accept a `builtin_` prefix at their
   dispatch points (eval call_builtin, codegen BUILTIN_CALLS emission,
   infer builtin_set, wasm) by stripping it before lookup — one strip
   rule, four surfaces.
2. **The gate**: `builtin_*` names resolve ONLY in std-origin files.
   load_dependencies stamps every std dep's file as `std/<mod>/<f>.kso`
   (the embed table already does; disk-resolved std deps get the same
   stamp), and check rejects `builtin_*` calls from any other file.
3. **Wrapper modules** (embedded in the include_str table for the
   browser): std/time (sleep), std/random (random), std/io (read_file,
   write_file, stdin, args), std/text (slice, join, chars, char_code,
   from_code, concat, utf8, to_int, to_float), std/math (sqrt, round).
4. **Ambient set shrinks to final form**: syntax targets + at-brackets +
   length + entries + push + put + print + if. render_value moves behind
   the gate (it was never meant to be user-callable).
5. **Corpus sweep**: json (chars/slice/join/utf8/to_int/from_code ->
   text/*), examples (concurrency: random/sleep -> random/random +
   time/sleep), book samples (ch05 io/read_file etc.), goldens.

## Order

engines' strip rule -> the gate -> wrapper modules + embeds -> ambient
list shrink -> corpus sweep -> goldens -> PR. Sibling sweep after merge
(kq/vse/json use text ops).
