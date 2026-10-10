# FRICTION: porting ripgrep to kanso

A journal kept while writing the port, in the order the problems came up.
Every entry was hit while compiling code in this directory, and the code
under "wrote" is what the tree holds now unless the entry says it changed.
Measurements were taken on this container on 2026-10-10 against the kanso
build at /tmp/claude-0/kanso-main/kanso.

## Entries

### F1: an entry file cannot hold a single definition
- kind: tooling
- severity: minor
- where: main.kso:1
- wanted: one file with a helper and the statement that uses it, the shape of the book's samples in chapters 04 and 05:
  ```
  import "std/os"

  fn show bs
    print "bytes {length bs}"

  os/read_bytes "d/b.bin" .> show
  ```
- wrote: every definition in a module directory, `search/`, and a `main.kso` holding imports and one statement:
  ```
  import "./search"
  import "std/os"

  os/args .> search/main
  ```
- why it matters: my first program, written to learn the effect API, was refused: "`t1.kso` is a library -- nothing to run ... run its definitions beside their statements with `kanso play`". The message names the fix, but the brief and chapter 01 list `run`, `check`, `test` and `build`, and `play` turns up only in `kanso help`. A newcomer meets the library/entry split before the first real line of code.

### F2: a regex counted repetition is read as string interpolation
- kind: confusing-semantics
- severity: major
- where: search/pattern.kso:7 (`specials`), and any regex literal with `{n,m}`
- wanted: `regexp/matches "o{2,3}" s`, as in every other language
- wrote: `regexp/matches "o\{2,3}" s`
- why it matters: `{2,3}` inside a string is an interpolation, so the literal is parsed as the expression `2,3`, and the error is "kanso has no commas; enumerations are space-separated", with a caret inside a string literal. Nothing in the message mentions braces or regexes. Worse, `x{3}` has no comma: it compiles, interpolates the number 3, and searches for `x3` (checked: `regexp/find "x{3}" "xxx x3"` answers `x3`). Any program that writes regexes (a search tool, a lexer, a validator) has to remember the backslash on every count.

### F3: a function that answers a boolean must be renamed with `?`, along with every call
- kind: refactoring-hazard
- severity: minor
- where: search/glob.kso (nine helpers)
- wanted: the matcher's recursive helpers named `matching`, `step`, `single`, `double`
- wrote: `matching?`, `step?`, `single?`, `double?`, `dirs?`, `anything?`, `bracketed?`, `literal?`, `starred?`, and every call to each
- why it matters: the first `kanso check` of a 120-line file gave nine `error[naming]` diagnostics. The rule is inferred from the body, so a helper that later stops answering a boolean must be renamed again, callers included. I did the rename with sed, which also rewrote two English words in comments ("A single? member", "literal?."). Partly aesthetic: `step? p i s j "*"` reads as a question about stepping, and it is not one.

### F4: a new top-level name in one file breaks a local binding in another
- kind: refactoring-hazard
- severity: major
- where: search/pattern.kso:24 (`quoted`), search/cli.kso (`shape_of`, `whole_number`), search/walk.kso (`by_kind`, `in_turn`)
- wanted: `fn escaped p` in pattern.kso
- wrote: `fn quoted p`. ignore.kso, a different file of the same module, already had `escaped = first == "\\#" or ...` inside a function body, and the new declaration turned that line into an error: "`escaped` is already a declaration; rename the binding".
- why it matters: it happened five times in one sitting (`escaped`, `shape`, `whole`, `kind`, `each`), each time in a file I was not editing. A module is one namespace with no shadowing, so every top-level name in every file competes with every parameter and local in every other file. The error lands on the innocent local, not on the declaration that caused it. On a team, adding a helper breaks a colleague's file. See F20 for the silent version of the same hazard.

### F5: list literals, map literals and strings must each fit on one line
- kind: missing-feature
- severity: major
- where: search/types.kso:50-79, search/cli.kso:174 (`valued?`), search/walk.kso:331 (`help_lines`)
- wanted: a table written one entry per line:
  ```
  valued_flags = [
    "--after-context"
    "--before-context"
    ...
  ]
  ```
- wrote: a different workaround for each table. The flag names became eleven dispatch arms (`fn valued? "--glob"` / `true`). The shell globs became five strings split on spaces (`text/split "{sh_shells} {sh_dots} {sh_more_dots} {sh_plain} {sh_more}" " "`). The help text became a pipe of pushes, one per line, forty lines of `. push "  -e, --regexp ..."`.
- why it matters: a program with data in it (a flag table, a file-type table, help text, test vectors) meets the 80-column limit with no legal way to wrap a literal. The errors do not point at the fix either: a multi-line list gives "an inline constant has no indented block", and a backslash at the end of a string gives "unterminated escape".

### F6: a map read is always maybe-none, and `_` does not catch none
- kind: confusing-semantics
- severity: minor
- where: search/cli.kso:564-581 (`yes?`, `listed`, `or_else`)
- wanted: `m["globs"]` after `fresh` has put `[]` there, and `m[key] == true` for a flag
- wrote: `listed m "globs"` (`or_else m[key] []`), and `yes? m[key]` with three arms, `none`, `true` and `_`, in place of the comparison
- why it matters: comparing a map value to `true` is refused ("comparing to `true` asks a question the value already answers"), but the value can be `none`, so the comparison is the honest question. And a `_` arm does not match `none`, which I learned from `error[exhaustive]` after writing `fn yes? _`; the book says a catch-all declines an err but does not say it declines a none.

### F7: the pipe skips a none, so `x . or_else d` never supplies the default
- kind: confusing-semantics
- severity: major
- where: search/cli.kso:506
- wanted: `mode = m["mode"] . or_else print_lines`
- wrote: `mode = or_else m["mode"] print_lines`
- why it matters: the pipe passes a none straight past the function, so the piped spelling compiles and leaves `mode` as none. The default never applies, and nothing warns. Appendix B mentions this once, in a paragraph about `describe`. The book teaches `.` as "x . f is f x", and for the one value a default exists to handle, it is not.

### F8: std/regexp has no compiled-pattern value, so a checked regex is re-checked at every call
- kind: stdlib-gap
- severity: major
- where: search/lines.kso:138-157 (`matching?`, `answered?`, `hits_in`, `nonempty`), search/pattern.kso:103 (`regex_trouble`)
- wanted:
  ```
  re = regexp/compile cfg.regex      # fails here, once, if it fails at all
  regexp/match? re line              # a boolean, always
  ```
- wrote: the pattern is matched against "" once at start-up only to see whether it errs, and every later call goes through a wrapper with an `(err _)` arm the program can never take:
  ```
  fn matching? regex line
    answered? (regexp/match? regex line)

  fn answered? (err _)
    false
  ```
- why it matters: the checker refused five call sites ("this can be an err and `!=` wants a value") for a pattern already proven to compile. And with no compiled value, every call re-parses the pattern: a 100,000-line search parses the same regex 100,000 times. I could not isolate how much of the run time that is (see F23), because there is no way to call the matcher without the parse.

### F9: an imported module's names collide with a type of my own, and the message does not say so
- kind: diagnostic
- severity: minor
- where: search/walk.kso:10 (`progress`, first written `tally`)
- wanted: `type tally` with three fields, built with `tally false false false`
- wrote: `type progress`
- why it matters: the error was "no 3-argument arm of `tally` (arms take 1)". The one-argument arm was `list/tally`, which joined my short-name space when I imported std/list. The type declaration itself was accepted, and the message never says where the other arm lives.

### F10: `if` inside a list literal is four elements, silently
- kind: engine-bug
- severity: major
- where: search/lines.kso (`noticed`, `counted`, as first written); bugs/if_in_list_literal/
- wanted: `[if dest.with_name "{dest.path}:{n}" "{n}"]`
- wrote: the `if` bound to a name, then `[line]`
- why it matters: the interpreter builds `[<fn> true "x" "y"]` and the native build fails with "native backend: `if` as a bare value is not yet supported", naming no file and no line. The checker already refuses the same shape for an ordinary call ("a list element is one atom"), so this is a hole in that check. I found it only because the native build refused; on the interpreter the program printed garbage without complaint, and I had to bisect a 2,000-line program by hand to find the `if`.

### F11: indexing a list needs a guard the checker can read, every time
- kind: confusing-semantics
- severity: minor
- where: search/glob.kso:29 (`string_at`), and its 29 callers
- wanted: `name = names[i]` inside a loop that already stops at `i > length names`
- wrote: first, a cast I found by accident, `text/join [names[i]] ""`, at 16 sites. Then, once I learned the checker reads `return x if i < 1 or i > length xs` as a bounds proof, one helper:
  ```
  pub fn string_at cs i
    if (i < 1 or i > length cs) "" cs[i]
  ```
- why it matters: the guard has to say `i < 1` even when the loop starts at 1, because the checker cannot see that; drop that half and the index is maybe-none again. The idiom is not in the book; I found it by reading `scripts/fingerprint` in the compiler repository. Until then the only way I knew to get a string out of a list was a join that would fail at run time on a none.

### F12: a record has no update syntax, so changing one field names all of them
- kind: missing-feature
- severity: major
- where: search/cli.kso:643 (`with_regex`), search/cli.kso:30-60 (`config`)
- wanted: `cfg with regex = r`, or Rust's `Config { regex: r, ..cfg }`
- wrote: a 28-line function that rebuilds the config field by field to change one:
  ```
  pub fn with_regex c regex
    config
      c.after
      c.before
      ...
      regex
      ...
      c.vimgrep
  ```
- why it matters: `config` has 26 fields, constructed positionally in alphabetical order. Adding `-f`, `--heading`, `--vimgrep`, `-0` and `--include-zero` meant adding six fields, and each one had to go in the right alphabetical slot in three places: the type, the constructor in `config_of`, and `with_regex`. A field in the wrong slot type-checks whenever its neighbour holds the same kind of value (most of these are booleans) and silently swaps two settings. This is also why the command line is read into a map first and only becomes a record at the end: a record cannot be built up one flag at a time.

### F13: running an effect for each element of a list has no obvious spelling
- kind: missing-feature
- severity: minor
- where: search/walk.kso:121 (`in_turn`)
- wanted: Haskell's `foldM`, or a `for` loop: visit each directory entry, carrying what the run has seen so far
- wrote: at first, five hand-written recursive loops (`visit`, `read_rules`, `gathered`, `each_root`, `reading_patterns`), each with an index, a bounds guard and a continuation lambda:
  ```
  fn visit cfg named here names i t
    return effect t if i > length names
    name = string_at names i
    on = (next -> visit cfg named here names (i + 1) next)
    entry cfg named here name t .> on
  ```
  Late in the port I found that an ordinary fold can build the chain, and the five loops became one helper and five one-line calls:
  ```
  fn in_turn xs start step
    list/fold xs (effect start) (chain x -> chain .> (acc -> step acc x))
  ```
- why it matters: the idiom works on all three engines, but neither the book nor std/list shows it, and chapter 09 teaches the hand-written recursion. A walker, a batch job or a test runner needs this on its first day. My first name for the helper, `each`, collided with a local in lines.kso (F4).

### F14: case folding in std/regexp is ASCII only, and std/text cannot change case at all
- kind: stdlib-gap
- severity: minor
- where: fixtures/divergences.txt (`unicode_fold`), search/pattern.kso:85 (`has_upper?`)
- wanted: `-i CAFÉ` finding `café`, and `text/lower` and `text/upper` for smart case
- wrote: smart case compares character codes against 65 to 90 by hand, and the `unicode_fold` fixture records that the port does not find `café`
- why it matters: ripgrep folds case across Unicode, and so does every regex engine a programmer brings. Accented text is ordinary data, and a search tool that cannot match it case-blind is wrong for most of the world's languages.

### F15: standard output takes strings, so bytes that are not UTF-8 cannot be written back
- kind: stdlib-gap
- severity: minor
- where: search/decode.kso:17 (`as_text`), fixtures/divergences.txt (`latin1`)
- wanted: print a matching line's bytes as they are in the file, as rg does
- wrote: the file is decoded, and a broken sequence becomes U+FFFD in the output
- why it matters: `io/write` and `print` take strings, and `os/read_bytes` is the only byte-level I/O. A tool that passes data through (grep, cat, a diff) cannot be faithful to a Latin-1 file.

### F16: no lossy UTF-8 decode, and no `\u` or `\0` string escapes
- kind: stdlib-gap
- severity: minor
- where: search/decode.kso:17-80, search/lines.kso:109, search/decode_test.kso
- wanted: `text/utf8_lossy bytes`, and `"\u{fffd}"` and `"\0"` in a literal
- wrote: a 60-line UTF-8 validator that rebuilds the byte list with `EF BF BD` in place of each bad sequence, and `text/from_code 65533` and `text/from_code 0` where the escapes would go
- why it matters: `text/utf8` answers an err for the whole file if one byte is wrong, so any program that reads files it did not write needs this function, and each will write its own. The missing escapes are a smaller thing, but the error (`unknown escape \u`) gives no hint that `from_code` exists.

### F17: a list of numbers and a byte string look the same and are not
- kind: confusing-semantics
- severity: minor
- where: search/decode_test.kso:12
- wanted: `decoded [104 105 0 33]` in a test, the same shape `os/read_bytes` yields
- wrote: `decoded (text/to_bytes [104 105 0 33])`
- why it matters: the test failed at run time with "error[runtime]: find2 takes bytes --> search:11:8". The location names the module but not the file. Appendix B describes `text/bytes` as returning `int[]` and `text/find2` as taking `int[]`, so nothing in the documentation says a list literal of the same numbers will be refused.

### F18: overload ordering forces arms out of the order a reader would want
- kind: aesthetics
- severity: nit
- where: search/cli.kso:449 (`set_value ... "--sort"`), search/lines.kso:41-51 (`binary_state`, `nul_state`)
- wanted: `fn set_value m "--sort" "path"` beside the other `set_value` arms, in alphabetical order; and `fn binary_state true _` / `fn binary_state _ none` / `fn binary_state _ nul`
- wrote: `--sort` handed to a separate `sorted_by` group, because an arm with two literals must come before every arm with one; and `binary_state` split into two groups, because the three arms "tie: each is the more specific one somewhere"
- why it matters: the specificity rule is clear and the diagnostics say exactly what is wrong, but in both cases the fix was to break one decision into two functions, not to reorder. A flag table that keeps one flag's arms elsewhere is harder to read.

### F19: a constant that does not fit on a line must grow a binding
- kind: aesthetics
- severity: nit
- where: search/pattern_test.kso, search/ignore_test.kso
- wanted:
  ```
  test_word_shape =
    regex_for ["fn"] "sensitive" false "word" == "\\b(?:(?:fn))\\b"
  ```
- wrote:
  ```
  test_word_shape =
    got = regex_for ["fn"] "sensitive" false "word"
    got == "\\b(?:(?:fn))\\b"
  ```
- why it matters: "a single-expression constant is written inline" and "a line holds at most 80 characters" together forbid the only layout for a long single-expression test. The error names the inline rule and not the width rule, so the first fix it suggests is the one that cannot work. It came up nine times in the test files.

### F20: a test helper silently joins an overload group in another file, and the verbs disagree about it
- kind: engine-bug
- severity: major
- where: search/lines_test.kso (helper `trial`, first named `run`); bugs/check_verbs_disagree/repro.sh
- wanted: a test helper `fn run args body` in lines_test.kso
- wrote: `fn trial args body`
- why it matters: walk.kso already had a two-arm `run` group at a different arity. Nothing said so: the two became one overload group. `kanso check search` and `kanso test search` accepted the tests, while `kanso check main.kso` and `kanso build` refused them with "this can be an err and `==` wants a value" pointing into the test file. So a passing test suite stopped the program it tests from building. I could not reduce the reproduction below the port's own module; the script copies it and shows the three verbs answering differently.

### F21: `\W`, `\D` and `\S` are read as the letters W, D and S
- kind: engine-bug
- severity: major
- where: search/pattern.kso:24-55 (`widened`, `negated_class`); bugs/regexp_negated_escapes/
- wanted: `rg -S 'todo\W' src`, which finds `TODO:`
- wrote: a pass over every pattern that spells the three out as `[^a-zA-Z0-9_]`, `[^0-9]` and `[^ \t\n\r]` outside classes. Inside a class there is no spelling for them, so `[\W]` still means the letter.
- why it matters: an escape std/regexp does not know falls through to the character itself, so these neither work nor fail, and a typo like `\q` quietly matches `q`. The fixture caught it only because rg was the oracle.

### F22: no way to ask whether output is a terminal, how big a file is, or what a symlink points at
- kind: stdlib-gap
- severity: minor
- where: README.md ("What it leaves out")
- wanted: `os/is_terminal`, a file size, `lstat`, the working directory
- wrote: nothing; the features are left out
- why it matters: rg picks its defaults by asking whether stdout is a terminal (headings, line numbers, colour) and whether stdin is (read it, or search the working directory). Without that, the port always behaves as if piped. `--max-filesize`, `-L` and ignore files above the working directory need the other three. Any command-line tool meant for both people and pipes hits the first.

### F23: the port is 25 to 100 times slower than rg, and the interpreter 100 times slower again
- kind: performance
- severity: minor
- where: whole program; measured on a 5.2 MB tree of 200 files and 100,000 lines
- wanted: within a small factor of rg on a literal search
- wrote: nothing different; these are the numbers, three runs each:
  ```
  rg -c TODO big                0.009 - 0.011 s
  release   -c TODO             0.24 - 0.27 s
  release   -n -w 'struct value' 1.05 - 1.19 s
  dev       -c TODO             0.67 - 0.72 s
  dev       -n -w 'struct value' 2.70 - 2.76 s
  interp    -c TODO             59 s
  build --release                8.9 s     build (dev) 0.56 s
  ```
- why it matters: the release binary is usable for a source tree. The interpreter is not, and `kanso test` runs on it, so any unit test that wants a realistic input (a few thousand lines) is slow enough to leave out. I kept the unit tests to inputs of a few lines and pushed size to the fixtures, which run native.

### F24: `kanso build` writes the binary into the current directory, named for the file
- kind: tooling
- severity: nit
- where: check.sh:24-27, fixtures/accept.sh
- wanted: `kanso build main.kso -o "$work/rg"`
- wrote: `(cd "$work/dev" && "$K" build "$here/main.kso")`, which leaves `main` and `main.ll` there
- why it matters: the output is called `main`, not `rg`, and the `.ll` file beside it is litter in a working tree. Every script that builds has to change directory first.

## What worked well

**Dispatch on literal strings made the flag table.** Each flag is an arm, and the parser has no switch statement in it:
```
fn set_flag m _ "--invert-match"
  put m "invert" true

fn set_flag _ spelled _
  bad_usage "unrecognized flag {spelled}"
```
Short flags are a second group, `fn canonical "-v"` / `"--invert-match"`, so the rest of the parser only ever sees long names. Adding `--null`, `--heading` or `--vimgrep` was one arm each.

**Marker types for output modes.** `count_lines`, `list_matching`, `quiet` and the rest are field-less types, and `rendered` has one arm per mode. Adding `--count-matches` was an arm, with nothing to keep in sync elsewhere.

**A missing file is data.** `os/read_file` answers `file_not_found` for a path that is not there, so reading three optional ignore files in every directory needed one extra arm and no error handling at all:
```
fn with_rules _ rules (file_not_found _)
  rules
```

**A foreign err can be dispatched on.** `text/utf8` answers an err for invalid UTF-8, and an `(err _)` arm in my module catches it and falls back to the lossy decoder. The err-is-a-value model made "try the fast path, repair on failure" a two-arm function.

**Effects as values made the output simple.** Each file's output is built as a string by pure code and written with one `io/write`, chained after the previous file's. Ordering is explicit, there is no shared buffer, and `os/exit` is an err that stops everything after it.

**The three engines agreed everywhere I could check.** 105 fixtures, each run on the interpreter, a dev binary and a release binary: no output differed between engines. The one engine problem (F10) was the native backend refusing a program, not computing a different answer. And because the expected files came from ripgrep itself, 100 of the 105 fixtures are byte-for-byte rg's own output.

**Several diagnostics were better than I expected.** "`override_rule` takes 1 argument(s), and a list element is one atom -- this reads as 2 separate elements. Write `(override_rule …)`" told me exactly what was wrong and how to fix it. The exhaustiveness check refused `rendered ... (shown_binary binary nul)` because the binary offset could be none, and the fix (a `text_file` marker instead of overloading none) made the code clearer.

**Tests as constants, built from the real parser.** Unit tests get their config the way the program does, `c = parse ["-A" "1" "match"]`, so no test builds a 26-field record by hand, and the parser is tested by every search test.

**`return x if cond` guards** kept the recursive matchers flat. The glob matcher (`*`, `**/`, `?`, classes, escapes) is 130 lines and passed its twelve tests the first time it compiled.

## Summary

The five I would fix first, in order:

1. **F4 and F20, the module-wide namespace.** One flat namespace with no shadowing means a declaration in one file breaks a local in another (five times in this port), and a helper in a test file can silently merge into an overload group defined elsewhere, after which `check`, `test` and `build` disagree about the same code. At least report the cross-file collision at the declaration, and never merge groups across arities silently.
2. **F8 and F21, std/regexp.** Give it a compiled-pattern value that cannot be an err, and make an unknown escape an error. A search tool, a lexer or a validator all hit both.
3. **F12, record update.** A 26-field record changed one field at a time costs a 28-line function, and alphabetical positional construction makes a misplaced boolean a silent bug.
4. **F5, multi-line literals.** Every table in this program (flags, file types, help text) needed a different workaround for the 80-column limit.
5. **F7, the pipe skipping a none.** `x . or_else d` compiles and silently never applies the default. It is the one place where the obvious spelling is wrong with no diagnostic. F10 (an `if` in a list literal producing garbage on the interpreter) belongs beside it as a bug to fix whatever else changes.

Writing ripgrep in kanso was mostly pleasant once the program was split along kanso's grain: a pure core that turns bytes into lines of output, and a thin layer of effects that walks directories and writes. Dispatch fits a command-line tool well. Flags, output modes, ignore verdicts (`admitted`, `excluded`, `unruled`) and binary states all became small arm groups that read like tables. The friction came from three places. The module namespace kept breaking files I was not touching. Data that does not fit on one line has nowhere to go. And std/regexp is a good backtracking engine with the interface of a one-shot function, which makes a program that runs one pattern a hundred thousand times pay for that on every line, in the checker's eyes and possibly at run time. The strictness found real mistakes (a none I had not planned for, a binary offset that could be missing), and it also cost real time on rules whose benefit in this program I could not see (F3, F18, F19). The port agrees with ripgrep byte for byte on 100 of its 105 fixtures, on all three engines, and the five differences are explained.
