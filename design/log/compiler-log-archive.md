

## 2026-10-12 — a name the author declared wins over a builtin and an import

#1891's first item, from the ports. Three reductions broke the 2026-08-29
ruling that an author's own name wins over an import's.

- **A type named like a builtin.** mustache declared `type run` and built one
  with `run 1 2 3`, importing nothing. The checker refused it twice over, with
  the count of the builtin std/os wraps and with that builtin's demand for a
  string. chip8's `type random` drew the same arity refusal. The checker let a
  declared function shadow a builtin and forgot a declared type; it now asks
  both. Native code had the same gap on the play route, where the name arrives
  bare: the build stopped on the builtin's count. Through an import the name
  arrives qualified and native already built the record.
- **A function named like an imported type.** mustache imported std/list and
  declared `fn counting a b`. std/list exports a record `counting`, which the
  import enrols under its bare name, and the interpreter built that record
  before looking for the file's group. It stopped on the field count where
  native called the function. A declared group now wins over an imported
  type of the same bare name.

`tests/play_verb.rs` pins both on both engines:
`a_type_named_like_a_builtin_is_the_files_own` and
`a_function_named_like_an_imported_type_is_the_files_own`, with fixtures in
`tests/golden/play`. The first also runs through an import as
`tests/golden/micro/a_type_named_like_a_builtin_is_the_programs`, which adds
`type random`. Main refuses both play fixtures. Four ratchet rows break one
guard each, and each turns its spec red.

The test-file half of the item (a test file's `import "std/list"` replacing a
private constructor) is #1883's, in kanso#1907.

CI's rows, on the tree merged with #1913: the shadow check costs each compile
a little. compile_instructions lands on 25,888,376 (+6,825),
entry_instructions on 85,392,486 (+11,173) and library_instructions on
85,946,778 (+11,196); browser_compile_instructions on 391,315,220 (+3,718) and
interp_instructions on 488,739,924 (+108). emit_instructions falls 1,084 to
7,012,393. The welfare floor drops by what this costs, under the rule that
building the language is priced at its cost.

On main after #1904 the tab's compile reads `browser_compile_instructions`
392,382,093, up 3,718 from main's 392,378,375 on the same tree.
