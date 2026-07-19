# The err-arm migration (gavel B enforced)

Status: **planned, Clay-ruled ASAP-before-imports (2026-07-19). One PR.**

The rule: `err` is unhandleable — it rises to the endpoint. Dispatching on it
(`(err reason)` / `(err _)` arms) becomes a compile error. Constructing err
stays legal; returning err to a caller stays legal; only *eliminating* it is
banned. Askable failures are `none` or your own typed values.

## Compiler

1. `check.rs`: reject `Pattern::Ctor { ty: "err", .. }` in fn params.
   Diagnostic: `error[failure]: an err is unhandleable—it rises to the
   endpoint; ask with none or return a typed value instead`. Golden test in
   tests/golden/errors/.
2. `infer.rs` `pattern_catches`: drop the ERR case (keep NONE).
3. New builtin `valid_utf8` (bytes/list → bool), all five sites — the
   test-validity-first predicate lib/json needs. (Additive; same recipe as
   sqrt/round.)

## Library migrations (each site's true design)

- **kq `_render_result (err reason)` → DELETE the arm.** The railway already
  carries err past `print` to the endpoint; the arm is a hand-rolled no-op.
  Verify kq's error-output goldens (endpoint report replaces any bespoke text).
- **`must` (lib/json; kq mirrors in kanso-lang/kq) → DELETE.** Tests call `decode` directly; an err
  result fails a test by itself (non-true). Update json_test/kq_test call
  sites; `defect` type stays for true invariants.
- **`_string_ok p (err _)` → validity-first:**
  `if (valid_utf8 acc) (utf8 acc) (_fail p "invalid utf-8 in string")`.
- **`_number_ok p (err _)` → tighten the scan, trust the conversion.** kanso
  ints are arbitrary precision, so `to_int` cannot overflow; make the number
  scan enforce JSON's number grammar (the `1e`/`1e+` tails it currently lets
  through to `to_float`), fail with position at scan time, and call the
  conversion bare. A residual conversion err then IS a defect and rises —
  which is the correct semantics for a should-never-happen. Drive with the
  existing 16-test suite + new invalid-number cases.

## Corpus + docs (panels re-executed; book_check now gates CI)

- **examples/errors.kso** (homepage star): redesign on a typed value —
  `type failed { reason: string }`, `safe_ratio _ 0 -> failed "division by
  zero"`, `describe (failed reason)` — same output, doctrine-pure. Update the
  homepage card copy ("err rises; askable failures are types you dispatch
  on"), the panel, the golden, and the playground copy of the example.
- **ch07 teahouse**: menu miss becomes `type off_menu { item: string }`;
  `describe (off_menu item)`; menu_test drops `is_err` and compares values
  (`(price "pocky") == off_menu "pocky"`) — simpler AND teaches typed
  failures. Re-run samples, update panels.
- **ch05 fallback.kso**: its point was "executor-born errs bypass err arms" —
  now the arm cannot be written at all. Convert to a `_check` sample showing
  the rejection diagnostic; the prose gets simpler ("you can't even ask").
- **ch04/appa**: add the new diagnostic to the catalog (appa) with an executed
  `*_check.kso`; ch04 already teaches unhandleable (no err arms present).
- **playground bank example**: constructs err (legal); verify it doesn't
  dispatch; adjust copy if needed.
- **index.html failure card**: "an err reaching main unhandled is a compile
  error" line — reconcile with actual semantics (endpoint report at runtime;
  the compile error is for *dispatching*).

## Order

checker+infer+builtin → lib/json (suite green) → kq (race harness green,
byte-identity vs jq re-verified) → examples+goldens → book samples+panels
(book_check green) → homepage/playground. One PR; CI (incl. book_check + cost
golden) is the gate; merge on green.

## Findings from enforcement (2026-07-19, branch err-unhandleable)

Checker + infer changes are IN on the branch (rejection fires with the
teaching diagnostic; pattern_catches lost its err case). Corpus breakage:

- **lib/json's failure-positions contract reads through err**:
  `_failure_position (err (parse_failure p _))` unwraps to extract the
  position. Migration (per consensus doctrine): **decode returns the bare
  typed `parse_failure` value** — askable by dispatch, no unwrapping, the
  positions feature survives as a first-class value. Callers who want
  propagate-on-failure convert (`err`-construct) at their boundary; the
  json test suite asserts on the value directly. `_fail` mints
  parse_failure, not err.
- **Four src unit tests embed err-arm json sources** (beat, dispatch,
  escape, linear) — update their fixtures to the migrated lib.
- Remaining per the plan: valid_utf8 builtin, _string_ok/_number_ok
  validity-first, must deletion, kq mirror (kanso-lang/kq), examples/
  errors.kso typed-value redesign + homepage, book ch05/ch07 samples,
  goldens, playground.

## The typed-railway recipe (worked out 2026-07-19; execute mechanically)

`_fail` returns bare `parse_failure p reason` (no err). Typed values do NOT
auto-propagate, so every consumer of a maybe-failed value dispatches:

- **Result dispatchers** gain one arm `fn X cs f:parse_failure ... -> f`:
  `_finish`, `_array_step`, `_obj_key`, `_obj_value` (value.kso, json.kso).
- **Position consumers** (the trap): `_expect`'s return feeds arithmetic —
  its callers must dispatch (`fn _after_colon cs p2:parse_failure ... -> p2`
  shape) before using the position. Inventory every `_fail` call site and
  trace its consumer: scan.kso `_expect`; value.kso literal/unexpected-char
  paths; text.kso unterminated-string; `_finish`'s trailing check.
- **text.kso**: delete `_string_ok`; `_str_char 34` and `_string_at 34`
  become `if (valid_utf8 acc) (_parsed (p+1) (utf8 acc)) (_fail p "invalid
  utf-8 in string")`.
- **number.kso**: delete `_number_ok`; `_number_value` gates on shape
  predicates then converts: `_int_shape` = `-`? digit+ exactly (strtoll's
  accept set given the scan alphabet); `_float_shape` = sign? digit* frac?
  exp? with >=1 digit before any exponent, exp = [eE] sign? digit+
  (strtod's accept set; "1." and "-.5" legal). Residual conversion err =
  defect rising loudly (correct for should-never-happen).
- **json.kso**: `_failure_position` dispatches on bare
  `(parse_failure p _)` (legal, typed); `must` + its test delete
  (`defect` stays only if still referenced); `decode` returns the bare
  parse_failure — callers ask by dispatch.
- **Mirrors after lib/json is green** (16 tests): the four embedded rust
  fixtures (src/beat,dispatch,escape,linear tests), then kanso-lang/kq
  (same five files + _render_result arm deletion + kq specs), then
  examples/errors.kso typed redesign + golden + homepage card + playground,
  then book ch05 fallback (becomes a rejection _check sample) + ch07
  teahouse (off_menu type) with panels re-executed; book_check + cost
  golden + full suite gate the PR.

## BLOCKED ON CLAY (2026-07-19): the typed railway needs union register-return

lib/json is functionally green on the typed railway (16/16), but the COST
GOLDEN caught a +115% allocation regression (14.8M -> 31.9M allocs, 6 -> 10
arena blocks): `parse_failure` records now share `_parsed`'s return slots, so
the escape analysis conservatively drops BOTH from register-return and every
scanned token heap-allocates again.

The fix is a real codegen extension: **union register-return** — allow a
closed set of 2-field record shapes ({_parsed, parse_failure}, both
16-byte-packable) to share the {i64,i64} return convention with a shape
discriminant. This is musttail/ABI territory (the x86-risk zone ruled
Clay-watching), so the branch stops here. Alternatives if the union is
unwanted: (a) accept the regression until the extension lands (kq/serde wins
evaporate — not acceptable for launch-adjacent code); (b) restructure so
failures never share the hot return path (sentinel smuggling — anti-kanso).
Branch err-unhandleable holds: checker rejection, valid_utf8, the full
lib/json migration, updated tests. Remaining after unblock: 4 rust fixture
tests (analysis-fact pins), kq mirror, examples/homepage/playground, book
ch05/ch07 samples, goldens.
