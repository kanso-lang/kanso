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
- **`must` (lib/json + kq) → DELETE.** Tests call `decode` directly; an err
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
