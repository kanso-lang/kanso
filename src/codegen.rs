use crate::ast::*;
use crate::diag::Span;
use crate::hash::Map as HashMap;
use crate::infer::{self, Set, BYTES, DESC, ERR, FAIL, FLOAT, INT, LIST, MAP, NONE, REC, STR, TOP};
use crate::name::Name;
use std::fmt::Write as _;

const K_TRUE: i64 = 2;
const K_FALSE: i64 = 3;
const K_NONE: i64 = 4;
const K_ERR: i64 = 5;

const DECLARES: &str = r#"%KValue = type { i64, i64 }
%parsed = type { i64, i64 }
%KBytes = type { i64, ptr }

; Inline twins of the runtime's hot one-liners (tag tests and value
; constructors). LTO declines to inline these across the .ll/.o module
; boundary, leaving a real call on every `if` condition and constructor;
; internal linkage keeps them from colliding with the runtime's own
; definitions, and alwaysinline folds them into every call site.
define internal %KValue @k_force_fast(%KValue %v) alwaysinline {
  %tag = extractvalue %KValue %v, 0
  %is = icmp eq i64 %tag, 14
  br i1 %is, label %slow, label %done
slow:
  %f = call %KValue @k_force(%KValue %v)
  ret %KValue %f
done:
  ret %KValue %v
}
@k_arena = external global ptr
@k_arena_left = external global i64
@k_stats_on = external global i32

define internal %KValue @k_b_append_byte(%KValue %acc, %KValue %x) alwaysinline {
  %atag = extractvalue %KValue %acc, 0
  %isb = icmp eq i64 %atag, 13
  br i1 %isb, label %chkx, label %slow
chkx:
  %xtag = extractvalue %KValue %x, 0
  %isi = icmp eq i64 %xtag, 0
  br i1 %isi, label %chks, label %slow
chks:
  %so = load i32, ptr @k_stats_on
  %counting = icmp ne i32 %so, 0
  br i1 %counting, label %slow, label %fast
fast:
  %bp = extractvalue %KValue %acc, 1
  %b = inttoptr i64 %bp to ptr
  %len = load i64, ptr %b
  %datap = getelementptr i8, ptr %b, i64 8
  %data = load ptr, ptr %datap
  %capp = getelementptr i8, ptr %b, i64 16
  %cap = load i64, ptr %capp
  %capneg = sub i64 0, %cap
  %isneg = icmp slt i64 %cap, 0
  %capa = select i1 %isneg, i64 %capneg, i64 %cap
  %owned = icmp ne i64 %cap, 0
  br i1 %owned, label %fr, label %slow
fr:
  %usedp = getelementptr i8, ptr %data, i64 -8
  %used = load i64, ptr %usedp
  %atfront = icmp eq i64 %used, %len
  %len1 = add i64 %len, 1
  %fits = icmp sle i64 %len1, %capa
  %ok = and i1 %atfront, %fits
  br i1 %ok, label %claim, label %slow
claim:
  %left = load i64, ptr @k_arena_left
  %has = icmp uge i64 %left, 32
  br i1 %has, label %alloc, label %slow
alloc:
  %dst = getelementptr i8, ptr %data, i64 %len
  %xv = extractvalue %KValue %x, 1
  %byte = trunc i64 %xv to i8
  store i8 %byte, ptr %dst
  store i64 %len1, ptr %usedp
  %ar = load ptr, ptr @k_arena
  %ar2 = getelementptr i8, ptr %ar, i64 32
  store ptr %ar2, ptr @k_arena
  %left2 = sub i64 %left, 32
  store i64 %left2, ptr @k_arena_left
  store i64 %len1, ptr %ar
  %hd = getelementptr i8, ptr %ar, i64 8
  store ptr %data, ptr %hd
  %hc = getelementptr i8, ptr %ar, i64 16
  store i64 %cap, ptr %hc
  %pi = ptrtoint ptr %ar to i64
  %r0 = insertvalue %KValue { i64 13, i64 undef }, i64 %pi, 1
  ret %KValue %r0
slow:
  %f = call %KValue @k_b_append(%KValue %acc, %KValue %x)
  ret %KValue %f
}
; The same claim where the linearity analysis proved the accumulator unique.
; It is the one above with the header work removed: nothing is allocated, the
; length is written where it already sits, and the argument comes back. That
; header claim is all the twin above does after the store, so the mutating
; append had the smaller body and was the only one still paying a call.
define internal %KValue @k_b_append_mut_byte(%KValue %acc, %KValue %x) alwaysinline {
  %atag = extractvalue %KValue %acc, 0
  %isb = icmp eq i64 %atag, 13
  br i1 %isb, label %chkx, label %slow
chkx:
  %xtag = extractvalue %KValue %x, 0
  %isi = icmp eq i64 %xtag, 0
  br i1 %isi, label %bstat, label %chkstr

; The byte arm, kept whole and separate from the string arm below rather than
; sharing their guards through a phi. Sharing costs the byte path two
; instructions per append — a phi and a second branch — which is 15,357,900 of
; them inside jsonbench's `str_char`, and the decoder appends bytes and nothing
; else. Two arms that duplicate five loads are cheaper than one arm that asks
; every byte which kind of append it is.
bstat:
  %bso = load i32, ptr @k_stats_on
  %bcount = icmp ne i32 %bso, 0
  br i1 %bcount, label %slow, label %bfast
bfast:
  %bp = extractvalue %KValue %acc, 1
  %b = inttoptr i64 %bp to ptr
  %len = load i64, ptr %b
  %datap = getelementptr i8, ptr %b, i64 8
  %data = load ptr, ptr %datap
  %capp = getelementptr i8, ptr %b, i64 16
  %cap = load i64, ptr %capp
  %capneg = sub i64 0, %cap
  %isneg = icmp slt i64 %cap, 0
  %capa = select i1 %isneg, i64 %capneg, i64 %cap
  %owned = icmp ne i64 %cap, 0
  br i1 %owned, label %bfr, label %slow
bfr:
  %usedp = getelementptr i8, ptr %data, i64 -8
  %used = load i64, ptr %usedp
  %atfront = icmp eq i64 %used, %len
  %len1 = add i64 %len, 1
  %fits = icmp sle i64 %len1, %capa
  %ok = and i1 %atfront, %fits
  br i1 %ok, label %bwrite, label %slow
bwrite:
  %dst = getelementptr i8, ptr %data, i64 %len
  %xv = extractvalue %KValue %x, 1
  %byte = trunc i64 %xv to i8
  store i8 %byte, ptr %dst
  store i64 %len1, ptr %usedp
  store i64 %len1, ptr %b
  ret %KValue %acc

; The string arm. The same claim over n bytes, which is what the encoder's
; `"true"`, `"null"` and every object key ask for: 7,670,800 of them in
; encodebench, each paying a call into the runtime and a second call into the
; wide path behind it.
chkstr:
  %iss = icmp eq i64 %xtag, 6
  br i1 %iss, label %sstat, label %slow
sstat:
  %sso = load i32, ptr @k_stats_on
  %scount = icmp ne i32 %sso, 0
  br i1 %scount, label %slow, label %sfast
sfast:
  %xp = extractvalue %KValue %x, 1
  %sp = inttoptr i64 %xp to ptr
  %sdata = load ptr, ptr %sp
  %slenp = getelementptr i8, ptr %sp, i64 8
  %slen32 = load i32, ptr %slenp
  %n = sext i32 %slen32 to i64
  %sbp = extractvalue %KValue %acc, 1
  %sb = inttoptr i64 %sbp to ptr
  %slen = load i64, ptr %sb
  %sdatap = getelementptr i8, ptr %sb, i64 8
  %sadata = load ptr, ptr %sdatap
  %scapp = getelementptr i8, ptr %sb, i64 16
  %scap = load i64, ptr %scapp
  %scapneg = sub i64 0, %scap
  %sisneg = icmp slt i64 %scap, 0
  %scapa = select i1 %sisneg, i64 %scapneg, i64 %scap
  %sowned = icmp ne i64 %scap, 0
  br i1 %sowned, label %sfr, label %slow
sfr:
  %susedp = getelementptr i8, ptr %sadata, i64 -8
  %sused = load i64, ptr %susedp
  %satfront = icmp eq i64 %sused, %slen
  %slenn = add i64 %slen, %n
  %sfits = icmp sle i64 %slenn, %scapa
  %sok = and i1 %satfront, %sfits
  br i1 %sok, label %swrite, label %slow
swrite:
  %sdst = getelementptr i8, ptr %sadata, i64 %slen
  call void @llvm.memcpy.p0.p0.i64(ptr %sdst, ptr %sdata, i64 %n, i1 false)
  store i64 %slenn, ptr %susedp
  store i64 %slenn, ptr %sb
  ret %KValue %acc

slow:
  %f = call %KValue @k_b_append_mut(%KValue %acc, %KValue %x)
  ret %KValue %f
}
define internal %KValue @k_b_length_fast(%KValue %v) alwaysinline {
  %tag = extractvalue %KValue %v, 0
  %is_list = icmp eq i64 %tag, 9
  %is_bytes = icmp eq i64 %tag, 13
  %fastable = or i1 %is_list, %is_bytes
  br i1 %fastable, label %list, label %slow
list:
  %p = extractvalue %KValue %v, 1
  %lp = inttoptr i64 %p to ptr
  %len = load i64, ptr %lp
  %r = insertvalue %KValue { i64 0, i64 undef }, i64 %len, 1
  ret %KValue %r
slow:
  %f = call %KValue @k_b_length(%KValue %v)
  ret %KValue %f
}
define internal %KValue @k_int(i64 %n) alwaysinline {
  %v = insertvalue %KValue { i64 0, i64 undef }, i64 %n, 1
  ret %KValue %v
}
define internal %KValue @k_float(double %d) alwaysinline {
  %bits = bitcast double %d to i64
  %v = insertvalue %KValue { i64 1, i64 undef }, i64 %bits, 1
  ret %KValue %v
}
define internal %KValue @k_bool(i64 %b) alwaysinline {
  %c = icmp ne i64 %b, 0
  %tag = select i1 %c, i64 2, i64 3
  %v = insertvalue %KValue { i64 undef, i64 0 }, i64 %tag, 0
  ret %KValue %v
}
define internal %KValue @k_none() alwaysinline {
  ret %KValue { i64 4, i64 0 }
}
define internal i64 @k_not_failure(%KValue %v) alwaysinline {
  %tag = extractvalue %KValue %v, 0
  %ne = icmp ne i64 %tag, 5
  %r = zext i1 %ne to i64
  ret i64 %r
}
define internal i64 @k_truthy(%KValue %v) alwaysinline {
  %tag = extractvalue %KValue %v, 0
  %t = icmp eq i64 %tag, 2
  br i1 %t, label %yes, label %chkf
yes:
  ret i64 1
chkf:
  %f = icmp eq i64 %tag, 3
  br i1 %f, label %no, label %bad
no:
  ret i64 0
bad:
  %r = call i64 @k_truthy_bad(%KValue %v)
  ret i64 %r
}
define internal i64 @k_check_tag(%KValue %v, i64 %t) alwaysinline {
  %tag = extractvalue %KValue %v, 0
  %c = icmp eq i64 %tag, %t
  %r = zext i1 %c to i64
  ret i64 %r
}
define internal i64 @k_check_int(%KValue %v, i64 %n) alwaysinline {
  %tag = extractvalue %KValue %v, 0
  %pay = extractvalue %KValue %v, 1
  %ct = icmp eq i64 %tag, 0
  %cp = icmp eq i64 %pay, %n
  %c = and i1 %ct, %cp
  %r = zext i1 %c to i64
  ret i64 %r
}
; A record read and a record pattern test, inlined for the shape that
; happens: a value whose tag IS `K_REC`, so neither has a subtype wrapper to
; walk. Between them `k_check_rec` and `k_field` are 2.90% of encodebench's
; own instructions before the call frames at each of the 134 sites the
; compiler writes for them, and a fold that matches a record pays both once a
; lap. Everything with a `K_SUB` on it falls through to the runtime, which
; walks the chain and answers as it always did — which is the same condition
; the runtime itself branches on, so the twin answers every shape the runtime
; answers without a call, and only a wrapper is left to it.
define internal i64 @k_check_rec_fast(%KValue %v, i64 %t, i64 %n) alwaysinline {
  %tag = extractvalue %KValue %v, 0
  %issub = icmp eq i64 %tag, 15
  br i1 %issub, label %slow, label %plain
plain:
  %isrec = icmp eq i64 %tag, 7
  br i1 %isrec, label %rec, label %no
no:
  ret i64 0
rec:
  %p = extractvalue %KValue %v, 1
  %r = inttoptr i64 %p to ptr
  %tid = load i64, ptr %r
  %np = getelementptr i8, ptr %r, i64 8
  %nf = load i64, ptr %np
  %et = icmp eq i64 %tid, %t
  %en = icmp eq i64 %nf, %n
  %both = and i1 %et, %en
  %out = zext i1 %both to i64
  ret i64 %out
slow:
  %s = call i64 @k_check_rec(%KValue %v, i64 %t, i64 %n)
  ret i64 %s
}
define internal %KValue @k_field_fast(%KValue %v, i64 %i) alwaysinline {
  %tag = extractvalue %KValue %v, 0
  %issub = icmp eq i64 %tag, 15
  br i1 %issub, label %slow, label %rec
rec:
  %p = extractvalue %KValue %v, 1
  %r = inttoptr i64 %p to ptr
  %fp = getelementptr i8, ptr %r, i64 16
  %fields = load ptr, ptr %fp
  %at = getelementptr %KValue, ptr %fields, i64 %i
  %val = load %KValue, ptr %at
  ret %KValue %val
slow:
  %s = call %KValue @k_field(%KValue %v, i64 %i)
  ret %KValue %s
}
; Bitwise work is a machine op wearing a call. Each of these takes two ints,
; does one instruction to them and boxes the answer, and digestbench spends
; 7.9% of itself in the five of them at about twenty-two instructions a call.
; The twin does the whole thing where the operand tags say int, which is every
; call the digest makes; anything else — a failure, a float, a shift outside
; nought to sixty-three — falls to the C entry, which answers as it always did
; and owns every diagnostic.
define internal %KValue @k_b_bit_and_fast(%KValue %a, %KValue %b) alwaysinline {
  %ta = extractvalue %KValue %a, 0
  %tb = extractvalue %KValue %b, 0
  %oa = icmp eq i64 %ta, 0
  %ob = icmp eq i64 %tb, 0
  %ok = and i1 %oa, %ob
  br i1 %ok, label %ints, label %slow
ints:
  %pa = extractvalue %KValue %a, 1
  %pb = extractvalue %KValue %b, 1
  %r = and i64 %pa, %pb
  %v = insertvalue %KValue { i64 0, i64 undef }, i64 %r, 1
  ret %KValue %v
slow:
  %s = call %KValue @k_b_bit_and(%KValue %a, %KValue %b)
  ret %KValue %s
}
define internal %KValue @k_b_bit_or_fast(%KValue %a, %KValue %b) alwaysinline {
  %ta = extractvalue %KValue %a, 0
  %tb = extractvalue %KValue %b, 0
  %oa = icmp eq i64 %ta, 0
  %ob = icmp eq i64 %tb, 0
  %ok = and i1 %oa, %ob
  br i1 %ok, label %ints, label %slow
ints:
  %pa = extractvalue %KValue %a, 1
  %pb = extractvalue %KValue %b, 1
  %r = or i64 %pa, %pb
  %v = insertvalue %KValue { i64 0, i64 undef }, i64 %r, 1
  ret %KValue %v
slow:
  %s = call %KValue @k_b_bit_or(%KValue %a, %KValue %b)
  ret %KValue %s
}
define internal %KValue @k_b_bit_xor_fast(%KValue %a, %KValue %b) alwaysinline {
  %ta = extractvalue %KValue %a, 0
  %tb = extractvalue %KValue %b, 0
  %oa = icmp eq i64 %ta, 0
  %ob = icmp eq i64 %tb, 0
  %ok = and i1 %oa, %ob
  br i1 %ok, label %ints, label %slow
ints:
  %pa = extractvalue %KValue %a, 1
  %pb = extractvalue %KValue %b, 1
  %r = xor i64 %pa, %pb
  %v = insertvalue %KValue { i64 0, i64 undef }, i64 %r, 1
  ret %KValue %v
slow:
  %s = call %KValue @k_b_bit_xor(%KValue %a, %KValue %b)
  ret %KValue %s
}
define internal %KValue @k_b_bit_shl_fast(%KValue %a, %KValue %b) alwaysinline {
  %ta = extractvalue %KValue %a, 0
  %tb = extractvalue %KValue %b, 0
  %oa = icmp eq i64 %ta, 0
  %ob = icmp eq i64 %tb, 0
  %ok = and i1 %oa, %ob
  br i1 %ok, label %range, label %slow
range:
  %pb = extractvalue %KValue %b, 1
  %inr = icmp ult i64 %pb, 64
  br i1 %inr, label %go, label %slow
go:
  %pa = extractvalue %KValue %a, 1
  %r = shl i64 %pa, %pb
  %v = insertvalue %KValue { i64 0, i64 undef }, i64 %r, 1
  ret %KValue %v
slow:
  %s = call %KValue @k_b_bit_shl(%KValue %a, %KValue %b)
  ret %KValue %s
}
define internal %KValue @k_b_bit_shr_fast(%KValue %a, %KValue %b) alwaysinline {
  %ta = extractvalue %KValue %a, 0
  %tb = extractvalue %KValue %b, 0
  %oa = icmp eq i64 %ta, 0
  %ob = icmp eq i64 %tb, 0
  %ok = and i1 %oa, %ob
  br i1 %ok, label %range, label %slow
range:
  %pb = extractvalue %KValue %b, 1
  %inr = icmp ult i64 %pb, 64
  br i1 %inr, label %go, label %slow
go:
  %pa = extractvalue %KValue %a, 1
  %r = ashr i64 %pa, %pb
  %v = insertvalue %KValue { i64 0, i64 undef }, i64 %r, 1
  ret %KValue %v
slow:
  %s = call %KValue @k_b_bit_shr(%KValue %a, %KValue %b)
  ret %KValue %s
}
define internal %KValue @k_b_bit_not_fast(%KValue %a) alwaysinline {
  %ta = extractvalue %KValue %a, 0
  %oa = icmp eq i64 %ta, 0
  br i1 %oa, label %int, label %slow
int:
  %pa = extractvalue %KValue %a, 1
  %r = xor i64 %pa, -1
  %v = insertvalue %KValue { i64 0, i64 undef }, i64 %r, 1
  ret %KValue %v
slow:
  %s = call %KValue @k_b_bit_not(%KValue %a)
  ret %KValue %s
}
; A demanded index into a list is a bounds check and a load, and it costs a
; call to reach. digestbench spends 21.7% of itself in `k_index`, at forty-one
; instructions a call. The twin answers the list case where the element is a
; plain value; a `none` element and a thunk both go to the runtime, the first
; because `k_index` turns it into the missing-index err and the second because
; forcing it is the runtime's job.
define internal %KValue @k_index_fast(%KValue %c, %KValue %k, ptr %o) alwaysinline {
  %tc = extractvalue %KValue %c, 0
  %tk = extractvalue %KValue %k, 0
  %islist = icmp eq i64 %tc, 9
  %isint = icmp eq i64 %tk, 0
  %shape = and i1 %islist, %isint
  br i1 %shape, label %bounds, label %slow
bounds:
  %pc = extractvalue %KValue %c, 1
  %l = inttoptr i64 %pc to ptr
  %len = load i64, ptr %l
  %i = extractvalue %KValue %k, 1
  %lo = icmp sgt i64 %i, 0
  %hi = icmp sle i64 %i, %len
  %inr = and i1 %lo, %hi
  br i1 %inr, label %read, label %slow
read:
  %ip = getelementptr i8, ptr %l, i64 8
  %items = load ptr, ptr %ip
  %j = add i64 %i, -1
  %at = getelementptr %KValue, ptr %items, i64 %j
  %v = load %KValue, ptr %at
  %tv = extractvalue %KValue %v, 0
  %isthunk = icmp eq i64 %tv, 14
  %isnone = icmp eq i64 %tv, 4
  %defer = or i1 %isthunk, %isnone
  br i1 %defer, label %slow, label %done
done:
  ret %KValue %v
slow:
  %s = call %KValue @k_index(%KValue %c, %KValue %k, ptr %o)
  ret %KValue %s
}
; The in-place list push, the same shape as the map insert below it. The C
; opens with six callee-saved pushes and a 168-byte frame on every call --
; the growth arm and the buffer bookkeeping share the function -- and behind
; that its fast path is thirteen instructions. jsonbench makes 1,459,800 of
; these a run at 59.7 apiece. Since the born-this-beat test came out of the
; fast arm the guard is four loads and two compares, so the whole thing fits
; here: on the frontier with room, claim the slot and bump both lengths.
define internal %KValue @k_b_push_mut_fast(%KValue %lv, %KValue %item) alwaysinline {
  %ltag = extractvalue %KValue %lv, 0
  %islist = icmp eq i64 %ltag, 9
  br i1 %islist, label %lstat, label %lslow
lstat:
  %lso = load i32, ptr @k_stats_on
  %lcounting = icmp ne i32 %lso, 0
  br i1 %lcounting, label %lslow, label %lshape
lshape:
  %lpi = extractvalue %KValue %lv, 1
  %l = inttoptr i64 %lpi to ptr
  %llen = load i64, ptr %l
  %itemspp = getelementptr i8, ptr %l, i64 8
  %items = load ptr, ptr %itemspp
  %lbuf = getelementptr i8, ptr %items, i64 -16
  %lcap = load i64, ptr %lbuf
  %lusedp = getelementptr i8, ptr %lbuf, i64 8
  %lused = load i64, ptr %lusedp
  %lfront = icmp eq i64 %lused, %llen
  %lneg = icmp slt i64 %lcap, 0
  %lncap = sub i64 0, %lcap
  %lcapa = select i1 %lneg, i64 %lncap, i64 %lcap
  %lfits = icmp slt i64 %llen, %lcapa
  %lok = and i1 %lfront, %lfits
  br i1 %lok, label %lwrite, label %lslow
lwrite:
  %lslot = getelementptr %KValue, ptr %items, i64 %llen
  store %KValue %item, ptr %lslot
  %llen1 = add i64 %llen, 1
  store i64 %llen1, ptr %lusedp
  store i64 %llen1, ptr %l
  ret %KValue %lv
lslow:
  %lr = call %KValue @k_b_push_mut(%KValue %lv, %KValue %item)
  ret %KValue %lr
}

; The in-place map insert, where the linearity analysis proved the map unique.
; The C spends a 312-byte frame and six callee-saved registers on every call,
; grow or not, because the growth arm and the view insert live in the same
; function; jsonbench pays that 1,254,150 times a decode run at seventy-eight
; instructions apiece. A map with no sorted view built is the whole of what
; the fast arm needs: `k_map_replace` answers on one branch, the view insert
; is a no-op, and the write is two slots at the frontier. Anything else -- a
; built view, a full buffer, a key that is not an int or a string, a failure
; in any of the three -- takes the call.
define internal %KValue @k_b_put_mut_fast(%KValue %mv, %KValue %k, %KValue %v) alwaysinline {
  %tm = extractvalue %KValue %mv, 0
  %ismap = icmp eq i64 %tm, 10
  br i1 %ismap, label %pkey, label %pslow
pkey:
  %tk = extractvalue %KValue %k, 0
  %ki = icmp eq i64 %tk, 0
  %ks = icmp eq i64 %tk, 6
  %keyok = or i1 %ki, %ks
  br i1 %keyok, label %pval, label %pslow
pval:
  %tv = extractvalue %KValue %v, 0
  %vbad = icmp eq i64 %tv, 5
  br i1 %vbad, label %pslow, label %pstat
pstat:
  %pso = load i32, ptr @k_stats_on
  %pcounting = icmp ne i32 %pso, 0
  br i1 %pcounting, label %pslow, label %pshape
pshape:
  %pmi = extractvalue %KValue %mv, 1
  %m = inttoptr i64 %pmi to ptr
  %sortedp = getelementptr i8, ptr %m, i64 16
  %sorted = load ptr, ptr %sortedp
  %hasview = icmp ne ptr %sorted, null
  br i1 %hasview, label %pslow, label %proom
proom:
  %mlen = load i64, ptr %m
  %pairspp = getelementptr i8, ptr %m, i64 8
  %pairs = load ptr, ptr %pairspp
  %pbuf = getelementptr i8, ptr %pairs, i64 -16
  %pcap = load i64, ptr %pbuf
  %pusedp = getelementptr i8, ptr %pbuf, i64 8
  %pused = load i64, ptr %pusedp
  %mlen2 = shl i64 %mlen, 1
  %pfront = icmp eq i64 %pused, %mlen2
  %pneed = add i64 %mlen2, 2
  %pneg = icmp slt i64 %pcap, 0
  %pncap = sub i64 0, %pcap
  %pcapa = select i1 %pneg, i64 %pncap, i64 %pcap
  %pfits = icmp sle i64 %pneed, %pcapa
  %pok = and i1 %pfront, %pfits
  br i1 %pok, label %pwrite, label %pslow
pwrite:
  %kslot = getelementptr %KValue, ptr %pairs, i64 %mlen2
  store %KValue %k, ptr %kslot
  %vidx = add i64 %mlen2, 1
  %vslot = getelementptr %KValue, ptr %pairs, i64 %vidx
  store %KValue %v, ptr %vslot
  store i64 %pneed, ptr %pusedp
  %mlen1 = add i64 %mlen, 1
  store i64 %mlen1, ptr %m
  ret %KValue %mv
pslow:
  %pr = call %KValue @k_b_put_mut(%KValue %mv, %KValue %k, %KValue %v)
  ret %KValue %pr
}

; The non-strict index. `k_index_fast` above is the STRICT form's fallback and
; only knows lists; everything written without the `!` went to the runtime by
; call, which is 7,237,200 of them in encodebench at thirty-five instructions
; apiece. A list slot and a byte are one load each once the bounds are known,
; and the two containers keep their length at the same offset. Out of range,
; a map, a string, a failure: all of it falls through, so `none` and the
; utf-8 seek stay where they were written.
define internal %KValue @k_b_at_fast(%KValue %c, %KValue %k) alwaysinline {
  %tk = extractvalue %KValue %k, 0
  %isint = icmp eq i64 %tk, 0
  br i1 %isint, label %shape, label %slow
shape:
  %tc = extractvalue %KValue %c, 0
  %islist = icmp eq i64 %tc, 9
  %isbytes = icmp eq i64 %tc, 13
  %known = or i1 %islist, %isbytes
  br i1 %known, label %bounds, label %slow
bounds:
  %pc = extractvalue %KValue %c, 1
  %p = inttoptr i64 %pc to ptr
  %len = load i64, ptr %p
  %i = extractvalue %KValue %k, 1
  %lo = icmp sgt i64 %i, 0
  %hi = icmp sle i64 %i, %len
  %inr = and i1 %lo, %hi
  br i1 %inr, label %pick, label %slow
pick:
  %j = add i64 %i, -1
  %dp = getelementptr i8, ptr %p, i64 8
  %data = load ptr, ptr %dp
  br i1 %islist, label %slot, label %byte
slot:
  %at = getelementptr %KValue, ptr %data, i64 %j
  %v = load %KValue, ptr %at
  ret %KValue %v
byte:
  %bp = getelementptr i8, ptr %data, i64 %j
  %b = load i8, ptr %bp
  %bz = zext i8 %b to i64
  %r = insertvalue %KValue { i64 0, i64 undef }, i64 %bz, 1
  ret %KValue %r
slow:
  %s = call %KValue @k_b_at(%KValue %c, %KValue %k)
  ret %KValue %s
}
define internal i64 @k_check_bool(%KValue %v) alwaysinline {
  %tag = extractvalue %KValue %v, 0
  %t = icmp eq i64 %tag, 2
  %f = icmp eq i64 %tag, 3
  %c = or i1 %t, %f
  %r = zext i1 %c to i64
  ret i64 %r
}
declare i64 @k_truthy_bad(%KValue)

declare %KValue @k_caf_freeze(%KValue)
declare void @k_math_ids(i64, i64)
declare %KValue @k_caf_blackhole()
declare %KValue @k_caf_complete(%KValue, %KValue)
declare %KValue @k_str_n(ptr, i64)
declare %KValue @k_str_lit(ptr, i64, ptr)
declare %KValue @k_err(%KValue, ptr)
declare %KValue @k_b_wrap_err(%KValue, %KValue, ptr)
declare %KValue @k_err_hop(%KValue, ptr)
declare %KValue @k_rec(i64, i64, ptr)
declare %KValue @k_pair_failure(%KValue, %KValue)
declare %KValue @k_rec_reuse(i64, i64, ptr, %KValue)
declare %KValue @k_concat_arr_mut(i64, ptr)
declare %KValue @k_b_str_builder(%KValue)
declare %KValue @k_field(%KValue, i64)
declare %KValue @k_keyed_check(%KValue, i64)
declare %KValue @k_keyed_field(%KValue, ptr)
declare %KValue @k_b_field(%KValue, ptr)
declare void @k_no_field(%KValue, ptr)
declare %KValue @k_field_forced(%KValue, ptr)
declare %KValue @k_set_field(%KValue, ptr, %KValue)
declare i64 @k_check_some(%KValue)
declare i64 @k_not_own_err(%KValue, ptr)
declare %KValue @k_err_inner(%KValue)
declare i64 @k_check_rec(%KValue, i64, i64)
declare i64 @k_check_str(%KValue, ptr, i64)
declare %KValue @k_concat(%KValue, %KValue)
declare %KValue @k_concat_arr(i64, ptr)
declare %KValue @k_render(%KValue, i64)
declare i64 @k_render_dispatchable(%KValue)
declare i64 @k_routes_to_arms(%KValue)
declare %KValue @k_add(%KValue, %KValue)
declare %KValue @k_sub(%KValue, %KValue)
declare %KValue @k_mul(%KValue, %KValue)
declare %KValue @k_div(%KValue, %KValue, ptr)
declare %KValue @k_mod(%KValue, %KValue, ptr)
declare %KValue @k_cmp(%KValue, %KValue, i64)
declare %KValue @k_desc_print(%KValue)
declare %KValue @k_seq(%KValue, %KValue)
declare void @k_die(ptr) noreturn
declare void @k_die_arity(i64, i64) noreturn
declare void @k_die_overload(ptr) noreturn
declare void @k_die_destructure(%KValue, ptr) noreturn
declare { i64, i1 } @llvm.sadd.with.overflow.i64(i64, i64)
declare { i64, i1 } @llvm.ssub.with.overflow.i64(i64, i64)
declare { i64, i1 } @llvm.smul.with.overflow.i64(i64, i64)
declare %KValue @k_list_lit(i64, ptr)
declare %KValue @k_map_lit(i64, ptr)
declare %KValue @k_closure(ptr, i64, i64, ptr)
declare %KValue @k_fnref(ptr)
declare %KValue @k_env_get(ptr, i64)
declare %KValue @k_b_at(%KValue, %KValue)
declare %KValue @k_b_is_desc(%KValue)
declare %KValue @k_index(%KValue, %KValue, ptr)
declare %KValue @k_b_bytes(%KValue)
declare %KValue @k_b_chars(%KValue)
declare %KValue @k_b_split(%KValue, %KValue)
declare %KValue @k_b_concat(%KValue, %KValue)
declare %KValue @k_b_utf8(%KValue, ptr)
declare %KValue @k_desc_args()
declare %KValue @k_desc_stdin()
declare %KValue @k_b_read_file(%KValue)
declare %KValue @k_b_write(%KValue)
declare %KValue @k_b_write_err(%KValue)
declare %KValue @k_b_env(%KValue)
declare %KValue @k_b_failed(%KValue)
declare %KValue @k_b_exists(%KValue)
declare %KValue @k_b_is_dir(%KValue)
declare %KValue @k_b_list_dir(%KValue)
declare %KValue @k_desc_now()
declare %KValue @k_b_make_dir(%KValue)
declare %KValue @k_b_write_file(%KValue, %KValue)
declare %KValue @k_b_run(%KValue, %KValue)
declare %KValue @k_b_start(%KValue, %KValue)
declare %KValue @k_b_kill(%KValue)
declare %KValue @k_b_listen(%KValue)
declare %KValue @k_b_net_port(%KValue)
declare %KValue @k_b_accept(%KValue)
declare %KValue @k_b_net_read(%KValue)
declare %KValue @k_b_net_write(%KValue, %KValue)
declare %KValue @k_b_net_close(%KValue)
declare %KValue @k_maybe_bind(%KValue, %KValue)
declare %KValue @k_b_bind(%KValue, %KValue)
declare %KValue @k_b_rescue(%KValue, %KValue)
declare %KValue @k_b_annotate(%KValue, %KValue, ptr)
declare %KValue @k_desc_join(%KValue, %KValue)
declare %KValue @k_desc_sleep(%KValue)
declare %KValue @k_desc_random(%KValue)
declare void @k_beat_push()
declare void @k_beat_iter()
declare void @k_carry_reset()
declare void @k_carry_stage(%KValue)
declare void @k_carry_stage_kept(%KValue)
declare %KValue @k_carry_take(i64)
declare void @k_beat_iter_carry()
declare %KValue @k_beat_pop(%KValue)
declare %KValue @k_cohort_pop(%KValue)
declare %KValue @k_call0(%KValue)
declare %KValue @k_call1(%KValue, %KValue)
declare %KValue @k_call2(%KValue, %KValue, %KValue)
declare %KValue @k_call3(%KValue, %KValue, %KValue, %KValue)
declare %KValue @k_call4(%KValue, %KValue, %KValue, %KValue, %KValue)
declare %KValue @k_b_char_code(%KValue)
declare %KValue @k_b_entries(%KValue)
declare %KValue @k_b_filter(%KValue, %KValue)
declare %KValue @k_b_from_code(%KValue, ptr)
declare %KValue @k_b_join(%KValue, %KValue)
declare %KValue @k_b_length(%KValue)
declare %KValue @k_b_map(%KValue, %KValue)
declare %KValue @k_b_push(%KValue, %KValue)
declare %KValue @k_b_push_mut(%KValue, %KValue)
declare void @llvm.memcpy.p0.p0.i64(ptr, ptr, i64, i1)
declare %KValue @k_b_append_mut(%KValue, %KValue)
declare %KValue @k_b_put(%KValue, %KValue, %KValue)
declare %KValue @k_b_put_mut(%KValue, %KValue, %KValue)
declare %KValue @k_b_slice(%KValue, %KValue, %KValue)
declare %KValue @k_b_utf8_slice(%KValue, %KValue, %KValue, ptr)
declare %KValue @k_b_find2(%KValue, %KValue, %KValue, %KValue)
declare %KValue @k_b_find2_below(%KValue, %KValue, %KValue, %KValue, %KValue)
declare %KValue @k_b_append(%KValue, %KValue)
declare %KValue @k_b_sort(%KValue)
declare %KValue @k_b_sum(%KValue)
declare %KValue @k_b_to_float(%KValue, ptr)
declare %KValue @k_b_bit_and(%KValue, %KValue)
declare %KValue @k_b_bit_or(%KValue, %KValue)
declare %KValue @k_b_bit_xor(%KValue, %KValue)
declare %KValue @k_b_bit_not(%KValue)
declare %KValue @k_b_bit_shl(%KValue, %KValue)
declare %KValue @k_b_bit_shr(%KValue, %KValue)
declare %KValue @k_b_sqrt(%KValue)
declare %KValue @k_b_round(%KValue)
declare %KValue @k_b_to_int(%KValue, ptr)
declare %KValue @k_b_to_bytes(%KValue, ptr)
declare %KValue @k_b_render_value(%KValue)
declare i64 @k_check_sub_tag(%KValue, i64)
declare i64 @k_check_sub_bool(%KValue)
declare i64 @k_check_sub_id(%KValue, i64)
declare i64 @k_check_sub_rec(%KValue, i64, i64)
declare %KValue @k_sub_ctor(i64, i64, %KValue, ptr, ptr)
declare %KValue @k_upcast(%KValue, i64, ptr)
declare %KValue @k_thunk_new(i64, i32, ...)
declare %KValue @k_thunk_release_unless(%KValue, %KValue)
declare void @k_thunk_note_escape(%KValue)
declare %KValue @k_force(%KValue)
declare %KValue @k_force_unless_black(%KValue)

"#;

pub(crate) const BUILTIN_CALLS: [&str; 55] = [
    "net_port",
    "start",
    "kill",
    "at",
    "is_desc",
    "append",
    "find2",
    "find2_below",
    "bytes",
    "to_bytes",
    "bind",
    "rescue",
    "annotate",
    "read_file",
    "write",
    "write_err",
    "env",
    "exists",
    "is_dir",
    "list_dir",
    "make_dir",
    "write_file",
    "run",
    "listen",
    "accept",
    "net_read",
    "net_write",
    "net_close",
    "concat",
    "utf8",
    "char_code",
    "chars",
    "split",
    "entries",
    "filter",
    "from_code",
    "join",
    "length",
    "map",
    "push",
    "put",
    "slice",
    "sort",
    "render_value",
    "sqrt",
    "bit_and",
    "bit_or",
    "bit_xor",
    "bit_not",
    "bit_shl",
    "bit_shr",
    "round",
    "sum",
    "to_float",
    "to_int",
];

/// The bit builtins that have an inline twin. Each is one machine op on two
/// ints, and a call to reach it; the twin does the int case where the tags
/// allow and leaves every other shape to the C entry that owns the message.
pub(crate) const BIT_TWINS: [&str; 6] =
    ["bit_and", "bit_or", "bit_xor", "bit_not", "bit_shl", "bit_shr"];

/// The twin's name for one of them. Separate from the table so a name added
/// to one without the other does not compile.
fn bit_twin(name: &str) -> &'static str {
    match name {
        "bit_and" => "bit_and_fast",
        "bit_or" => "bit_or_fast",
        "bit_xor" => "bit_xor_fast",
        "bit_not" => "bit_not_fast",
        "bit_shl" => "bit_shl_fast",
        "bit_shr" => "bit_shr_fast",
        other => unreachable!("bit_twin asked for `{other}`, which BIT_TWINS does not hold"),
    }
}

/// The count a builtin the native backend emits a direct call for takes.
/// Membership is this file's business — which builtins get a C entry rather
/// than an inline expansion — and the count is `check`'s, so this asks each
/// question where it is answered instead of keeping a second copy of the
/// counts here. That second copy is what let the backend refuse a call the
/// front door had waved through.
fn arity_of_emitted(name: &str) -> Option<usize> {
    match BUILTIN_CALLS.contains(&name) {
        true => crate::check::builtin_arity(name),
        false => None,
    }
}

/// Groups that are pure builtin forwarders: one arm, plain-var params,
/// body exactly `builtin_X p1 p2 ...` in order. Call sites bypass the
/// dispatch hop and reach the builtin (and its inline twins) directly.
fn forwarder_map(program: &Program) -> HashMap<(String, usize), String> {
    let mut counts: HashMap<(String, usize), usize> = HashMap::default();
    for d in &program.fns {
        *counts.entry((d.name.clone(), d.params.len())).or_default() += 1;
    }
    let mut out = HashMap::default();
    for d in &program.fns {
        if counts[&(d.name.clone(), d.params.len())] != 1 || d.body.len() != 1 {
            continue;
        }
        let params: Vec<&str> = d
            .params
            .iter()
            .filter_map(|p| match p {
                Pattern::Var(n, _) => Some(n.as_str()),
                _ => None,
            })
            .collect();
        if params.len() != d.params.len() {
            continue;
        }
        let Stmt::Expr(Expr::App { head, args, piped: false, .. }) = &d.body[0] else {
            continue;
        };
        let Expr::Ident(callee, _) = head.as_ref() else { continue };
        let Some(target) = callee.strip_prefix("builtin_") else { continue };
        let all_forwarded = args.len() == params.len()
            && args.iter().zip(&params).all(|(a, p)| matches!(a, Expr::Ident(n, _) if n == p));
        if all_forwarded {
            out.insert((d.name.clone(), d.params.len()), target.to_string());
        }
    }
    out
}

/// The constants a constant can reach from its own body, following mentions
/// through other constants. A name that reaches itself must be frozen: an
/// unfrozen mention re-enters the builder, and the recursion has no floor.
/// One name reaching itself and two names reaching each other are the same
/// shape, so the question is cycle membership.
pub(crate) fn knotted_constants(program: &Program) -> crate::hash::Set<String> {
    fn names(expr: &Expr, out: &mut Vec<String>) {
        if let Expr::Ident(n, _) | Expr::Partial(n, _) = expr {
            out.push(n.to_string());
        }
        crate::for_each_child(expr, |child| names(child, out));
    }
    let mut mentions: HashMap<&str, Vec<String>> = HashMap::default();
    for decl in program.fns.iter().filter(|d| d.params.is_empty()) {
        let found = mentions.entry(decl.name.as_str()).or_default();
        for stmt in &decl.body {
            match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => names(expr, found),
                Stmt::Set { value, .. } => names(value, found),
            }
        }
    }
    mentions
        .keys()
        .copied()
        .filter(|start| {
            let mut seen: crate::hash::Set<&str> = crate::hash::Set::default();
            let mut queue: Vec<&str> = vec![start];
            while let Some(here) = queue.pop() {
                for next in mentions.get(here).into_iter().flatten() {
                    if next == start {
                        return true;
                    }
                    if let Some((key, _)) = mentions.get_key_value(next.as_str()) {
                        if seen.insert(key) {
                            queue.push(key);
                        }
                    }
                }
            }
            false
        })
        .map(str::to_string)
        .collect()
}

pub fn emit_ir(program: &Program) -> Result<String, String> {
    let knotted = knotted_constants(program);
    let inference = infer::infer(program);
    let mut type_ids = HashMap::default();
    type_ids.insert("entry", 0i64);
    for (i, ty) in program.types.iter().enumerate() {
        type_ids.insert(ty.name.as_str(), (i + 1) as i64);
    }
    // an enrollment clone is an alias: it constructs and matches as its
    // origin, one identity per type no matter the spelling
    let clone_ids: Vec<(&str, i64)> = program
        .types
        .iter()
        .filter_map(|t| {
            t.origin.as_deref().and_then(|o| type_ids.get(o).map(|id| (t.name.as_str(), *id)))
        })
        .collect();
    for (name, id) in clone_ids {
        type_ids.insert(name, id);
    }
    let mut escape = crate::escape::analyze(program, &inference);
    // The by-value `%parsed` is two i64s, so it only fits a record shaped like
    // the scanner's `_parsed`: exactly two fields, a small int position packed
    // into the tag word and a non-failure value in the payload word. Any other
    // register-returnable record keeps the heap representation.
    let type_index: HashMap<&str, usize> =
        program.types.iter().enumerate().map(|(i, t)| (t.name.as_str(), i)).collect();
    escape.field_count.retain(|ty, n| {
        *n == 2
            && type_index.get(ty.as_str()).is_some_and(|&i| {
                inference.type_fields.get(i).is_some_and(|fields| {
                    fields.len() == 2 && fields[0] == INT && fields[1] & FAIL == 0
                })
            })
    });
    let packable: crate::hash::Set<String> = escape.field_count.keys().cloned().collect();
    escape.returns.retain(|_, ty| packable.contains(ty));
    escape.carries.retain(|_, ty| packable.contains(ty));
    let byte_disc = crate::dispatch::byte_dispatched(program, &inference);
    let in_place_pushes = crate::linear::in_place_pushes(program);
    let reusable_records = crate::linear::reusable_records(program);
    let (builder_joins, builder_params, builder_carried) = crate::linear::string_builders(program);
    // Beat loops rewind the arena between iterations. Groups returning the
    // by-value %parsed are excluded: k_beat_pop judges heap-ness from the
    // returned tag word, and the packed representation would mislead it.
    let mut beat = crate::beat::beat_loops(program, &inference, &in_place_pushes);
    beat.ids.retain(|(n, a), _| escape.returns_ty(n, *a).is_none());
    beat.demoted.retain(|(_, callee)| beat.ids.contains_key(callee));
    let mut backend = Backend {
        program,
        forwarders: forwarder_map(program),
        sub_parents: program
            .types
            .iter()
            .filter_map(|t| t.parent.clone().map(|p| (t.name.clone(), p)))
            .collect(),
        typesets: program
            .types
            .iter()
            .filter(|t| !t.members.is_empty())
            .map(|t| (t.name.clone(), t.members.clone()))
            .collect(),
        inference,
        escape,
        byte_disc,
        in_place_pushes,
        reusable_records,
        builder_joins,
        builder_params,
        builder_carried,
        beat,
        type_ids,
        strings: Vec::new(),
        interned: HashMap::default(),
        body: String::new(),
        lift_counter: 0,
        fn_value_wrappers: Vec::new(),
        builtin_value_wrappers: Vec::new(),
        defers_self_reference: !knotted.is_empty(),
        knotted,
        print_value_wrapper: false,
        caf_cells: Vec::new(),
        demand: crate::demand::analyze(program),
        thunk_sites: Vec::new(),
    };
    backend.emit()
}

struct Backend<'a> {
    program: &'a Program,
    inference: infer::Inference,
    forwarders: HashMap<(String, usize), String>,
    /// subtype name -> parent name; non-empty programs get chain-aware
    /// dispatch checks, everyone else keeps the exact ones
    sub_parents: HashMap<String, String>,
    /// typeset name -> members; an annotated param matches any member
    typesets: HashMap<String, Vec<String>>,
    escape: crate::escape::EscapeInfo,
    byte_disc: crate::hash::Set<(String, usize, usize)>,
    in_place_pushes: crate::hash::Set<(String, usize, usize)>,
    reusable_records: crate::hash::Map<(String, usize, usize), String>,
    builder_joins: crate::hash::Set<(String, usize, usize)>,
    builder_params: crate::hash::Set<(String, usize, usize)>,
    /// Argument positions already carrying the builder, so no seed is needed.
    builder_carried: crate::hash::Set<(String, usize, usize)>,
    beat: crate::beat::Beats,
    type_ids: HashMap<&'a str, i64>,
    strings: Vec<(String, Vec<u8>)>,
    interned: HashMap<Vec<u8>, String>,
    body: String,
    lift_counter: usize,
    fn_value_wrappers: Vec<(String, usize)>,
    /// (builtin, arity) pairs a program hands out as values, each needing a
    /// wrapper a dynamic call can reach.
    builtin_value_wrappers: Vec<(String, usize)>,
    defers_self_reference: bool,
    /// Zero-arity names that reach themselves through other constants.
    knotted: crate::hash::Set<String>,
    print_value_wrapper: bool,
    /// One cache cell per frozen constant, emitted as globals at the end,
    /// each beside an `i8` that says whether its builder has run. Nothing
    /// fills these before main any more: a constant builds itself on the
    /// first read, so one nobody reads is never built.
    caf_cells: Vec<String>,
    demand: crate::demand::DemandInfo<'a>,
    /// (site evaluator symbol, captured-arg count), indexed by site id.
    thunk_sites: Vec<(String, usize)>,
}

/// The frame epilogue: release each releasable cell unless the outgoing
/// value IS that cell (the returned-thunk case escapes upward, counted by
/// the runtime). Returns the value register to hand to `ret` — threading
/// through the helper keeps the IR linear.
fn release_cells(f: &mut FnEmit, value: &str) -> String {
    if f.lazy_cells.is_empty() || f.parsed.contains_key(value) {
        return value.to_string();
    }
    let cells = f.lazy_cells.clone();
    let mut v = value.to_string();
    for cell in cells {
        let t = f.tmp();
        f.line(&format!("{t} = call %KValue @k_thunk_release_unless(%KValue {cell}, %KValue {v})"));
        v = t;
    }
    v
}

struct FnEmit {
    out: String,
    tmp: usize,
    label: usize,
    cur_label: String,
    versions: HashMap<String, String>,
    sets: HashMap<String, Set>,
    /// Temps carrying the by-value %parsed type rather than a boxed KValue.
    /// Operands living in the by-value convention, and the record type
    /// each one holds — boxing one back needs to name its type and its id.
    parsed: crate::hash::Map<String, (String, i64)>,
    /// Err-origin prefix "{fn lazy_cells: Vec::new(), } at {file}" for the declaration being emitted.
    origin_prefix: String,
    /// Source file of the declaration being emitted, for keying push sites.
    file: String,
    /// LLVM return type of the function being emitted: `%parsed` or `%KValue`.
    ret_ty: String,
    /// Dispatcher group being emitted, for recognizing self-tail-calls.
    group: String,
    arity: usize,
    /// Whether the current arm is a bare enrollment clone: library plumbing
    /// wearing an unqualified name, which the cohort license must not read
    /// as user code.
    synthetic: bool,
    /// The package this arm belongs to. An arm cannot see an err its own
    /// package raised, and this is the side of that comparison the compiler
    /// knows; the err carries the other.
    hako: String,
    /// Registers of releasable lazy cells born in this body; every return
    /// path releases each unless the result aliases it.
    lazy_cells: Vec<String>,
}

impl FnEmit {
    fn new() -> Self {
        FnEmit {
            out: String::new(),
            tmp: 0,
            label: 0,
            cur_label: "entry".to_string(),
            versions: HashMap::default(),
            sets: HashMap::default(),
            parsed: crate::hash::Map::default(),
            origin_prefix: String::new(),
            hako: String::new(),
            file: String::new(),
            ret_ty: "%KValue".to_string(),
            group: String::new(),
            synthetic: false,
            arity: 0,
            lazy_cells: Vec::new(),
        }
    }

    fn tmp(&mut self) -> String {
        self.tmp += 1;
        format!("%t{}", self.tmp)
    }

    fn label(&mut self) -> String {
        self.label += 1;
        format!("L{}", self.label)
    }

    /// Only a carried argument slot reads the two-word convention. Every other
    /// consumer names its operand as a `%KValue`, and a `%parsed` reaching one
    /// is invalid IR the host's clang refuses. Repairing here rather than at
    /// each consumer is what makes the rule hold for consumers nobody has
    /// written yet: five separate sites were fixed one at a time before this,
    /// and the sixth would have shipped the same way.
    fn line(&mut self, text: &str) {
        let text = self.boxing_any_parsed_operand(text);
        let _ = writeln!(self.out, "  {text}");
    }

    fn boxing_any_parsed_operand(&mut self, text: &str) -> String {
        if self.parsed.is_empty() {
            return text.to_string();
        }
        // Sorted because the map's order is randomized per process, and a line
        // naming two carried operands would otherwise box them in an order
        // that differs between builds of the same program.
        let mut carried: Vec<String> =
            self.parsed.keys().filter(|t| named_as_a_value(text, t)).cloned().collect();
        carried.sort();
        carried.iter().fold(text.to_string(), |acc, t| {
            let boxed = self.box_parsed(t);
            let named = format!("%KValue {t}");
            let boxed_as = format!("%KValue {boxed}");
            let inner = acc
                .replace(&format!("{named},"), &format!("{boxed_as},"))
                .replace(&format!("{named})"), &format!("{boxed_as})"));
            match inner.strip_suffix(&named) {
                Some(head) => format!("{head}{boxed_as}"),
                None => inner,
            }
        })
    }

    /// Undo the by-value convention: rebuild the record the two words hold.
    /// The type is whatever produced the value, which the escape analysis
    /// already knows, because only a returnable type is ever in this shape.
    fn box_parsed(&mut self, e: &str) -> String {
        let (_, id) = self.parsed[e];
        let w0 = self.tmp();
        self.raw(&format!("{w0} = extractvalue %parsed {e}, 0"));
        let w1 = self.tmp();
        self.raw(&format!("{w1} = extractvalue %parsed {e}, 1"));
        let pos = self.tmp();
        self.raw(&format!("{pos} = lshr i64 {w0}, 8"));
        let vtag = self.tmp();
        self.raw(&format!("{vtag} = and i64 {w0}, 255"));
        let f0a = self.tmp();
        self.raw(&format!("{f0a} = insertvalue %KValue undef, i64 0, 0"));
        let f0 = self.tmp();
        self.raw(&format!("{f0} = insertvalue %KValue {f0a}, i64 {pos}, 1"));
        let f1a = self.tmp();
        self.raw(&format!("{f1a} = insertvalue %KValue undef, i64 {vtag}, 0"));
        let f1 = self.tmp();
        self.raw(&format!("{f1} = insertvalue %KValue {f1a}, i64 {w1}, 1"));
        let arr = self.tmp();
        self.raw(&format!("{arr} = alloca [2 x %KValue]"));
        let p0 = self.tmp();
        self.raw(&format!("{p0} = getelementptr [2 x %KValue], ptr {arr}, i64 0, i64 0"));
        self.raw(&format!("store %KValue {f0}, ptr {p0}"));
        let p1 = self.tmp();
        self.raw(&format!("{p1} = getelementptr [2 x %KValue], ptr {arr}, i64 0, i64 1"));
        self.raw(&format!("store %KValue {f1}, ptr {p1}"));
        let t = self.tmp();
        self.raw(&format!("{t} = call %KValue @k_rec(i64 {id}, i64 2, ptr {arr})"));
        t
    }

    /// A line whose operands are already values: the boxing rewrite emits
    /// through here, so a fresh temp is never scanned against itself.
    fn raw(&mut self, text: &str) {
        let _ = writeln!(self.out, "  {text}");
    }

    fn start_block(&mut self, label: &str) {
        let _ = writeln!(self.out, "{label}:");
        self.cur_label = label.to_string();
    }

    fn bind(&mut self, name: &str, temp: &str) {
        self.versions.insert(name.to_string(), temp.to_string());
    }

    fn lookup(&self, name: &str) -> Option<String> {
        self.versions.get(name).cloned()
    }

    fn record_parsed(&mut self, operand: &str, ty: &str, id: i64) {
        self.parsed.insert(operand.to_string(), (ty.to_string(), id));
    }

    fn is_parsed(&self, operand: &str) -> bool {
        self.parsed.contains_key(operand)
    }

    fn record(&mut self, operand: &str, set: Set) {
        self.sets.insert(operand.to_string(), set);
    }

    fn set_of(&self, operand: &str) -> Set {
        if operand.starts_with("{ i64 0,") {
            return INT;
        }
        if operand == "{ i64 2, i64 0 }" || operand == "{ i64 3, i64 0 }" {
            return infer::BOOL;
        }
        if operand == "{ i64 4, i64 0 }" {
            return NONE;
        }
        self.sets.get(operand).copied().unwrap_or(TOP)
    }
}

/// Dispatchers and wrappers nobody names. A dead caller still writes a call to
/// its dead callee, so one sweep leaves the callee named by a caller that is
/// itself about to go — hence the fixpoint. Only `d_` and `w_` symbols are
/// candidates: everything else is either the entry, a builder the constant
/// initialiser calls, or a switch the runtime calls by name.
fn prune_unnamed(body: &str, entry: &str) -> String {
    let mut blocks = ir_defines(body);
    loop {
        let named = |at: usize, sym: &str| {
            blocks.iter().enumerate().any(|(k, (_, text))| k != at && names_symbol(text, sym))
        };
        let doomed = blocks.iter().enumerate().position(|(at, (sym, _))| {
            // The runtime calls the thunk dispatcher itself, from `k_force`,
            // so no emitted line names it and it would go on the first sweep.
            sym != entry
                && sym != "d_thunk_eval"
                && (sym.starts_with("d_")
                    || sym.starts_with("w_")
                    || sym.starts_with("\"d_")
                    || sym.starts_with("\"w_"))
                && !named(at, sym)
        });
        match doomed {
            Some(at) => blocks.remove(at),
            None => break,
        };
    }
    blocks.into_iter().map(|(_, text)| text).collect()
}

/// Whether this text names `@sym`. A call writes `@sym(`, but a closure hands
/// its wrapper over as `ptr @sym,` — so the delimiter decides, and it also
/// keeps `@w_klam1` from answering for `@w_klam17`.
fn names_symbol(text: &str, sym: &str) -> bool {
    let needle = format!("@{sym}");
    text.match_indices(&needle).any(|(at, _)| {
        !matches!(
            text.as_bytes().get(at + needle.len()),
            Some(b) if b.is_ascii_alphanumeric() || *b == b'_' || *b == b'"'
        )
    })
}

/// Split emitted IR into segments, keeping every byte. A `define` segment
/// runs from its header to the closing brace on its own line and carries its
/// symbol; everything between definitions — globals the fnref statics live in
/// among them — is a segment with no symbol, which the prune never touches.
fn ir_defines(body: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut sym = String::new();
    let mut text = String::new();
    let mut inside = false;
    for line in body.split_inclusive('\n') {
        if !inside && line.starts_with("define ") {
            if !text.is_empty() {
                out.push((String::new(), std::mem::take(&mut text)));
            }
            inside = true;
            sym = line
                .find('@')
                .and_then(|at| {
                    line[at + 1..].find('(').map(|p| line[at + 1..at + 1 + p].to_string())
                })
                .unwrap_or_default();
        }
        text.push_str(line);
        if inside && line.trim_end() == "}" {
            out.push((std::mem::take(&mut sym), std::mem::take(&mut text)));
            inside = false;
        }
    }
    if !text.is_empty() {
        out.push((String::new(), text));
    }
    out
}

/// Whether an emitted line names this temp in a `%KValue` operand position.
/// The delimiter matters: `%t2` is a prefix of `%t20`, and boxing the wrong
/// register writes a program that type-checks and computes the wrong record.
fn named_as_a_value(text: &str, temp: &str) -> bool {
    let needle = format!("%KValue {temp}");
    text.match_indices(&needle).any(|(at, _)| {
        matches!(text.as_bytes().get(at + needle.len()), Some(b',') | Some(b')') | None)
    })
}

/// The dispatcher, inlined at the call site for the shape that actually
/// happens: a closure of the arity written, with no failure in an argument.
///
/// `k_call2` is 26 instructions and a fold applies it once a lap. Ten of the
/// 26 ask about the callable — is it a failure, is it a closure, does its
/// arity match — and a fold passes the same callable through its self-call
/// unchanged, so those ten are loop-invariant and LICM can hoist them out of
/// the loop TailCallElim makes of the recursion. It cannot hoist across a
/// call, which is what the runtime dispatcher is: LTO sees the body and
/// declines to inline it on cost, and `always_inline` on the C definition is
/// ignored because the emitted `.ll` calls the symbol by name. So the test
/// is emitted here instead, in the module the optimizer is already in.
///
/// Every shape the fast arm does not cover falls through to `k_call{n}`,
/// which re-asks everything and answers exactly as before — a failing
/// callable, a fnref, a wrong arity, a failing argument. That is why the
/// order here may differ from the runtime's: the arm only fires where all
/// the orders agree.
fn call_twin(n: usize) -> String {
    let args: String = (0..n).map(|i| format!(", %KValue %a{i}")).collect();
    let mut s = String::new();
    let _ =
        writeln!(s, "define internal %KValue @k_call{n}_fast(%KValue %f{args}) alwaysinline {{");
    let _ = writeln!(s, "  %ftag = extractvalue %KValue %f, 0");
    let _ = writeln!(s, "  %isclo = icmp eq i64 %ftag, 11");
    let _ = writeln!(s, "  br i1 %isclo, label %arity, label %slow");
    let _ = writeln!(s, "arity:");
    let _ = writeln!(s, "  %fp = extractvalue %KValue %f, 1");
    let _ = writeln!(s, "  %c = inttoptr i64 %fp to ptr");
    let _ = writeln!(s, "  %arp = getelementptr i8, ptr %c, i64 24");
    let _ = writeln!(s, "  %ar = load i64, ptr %arp");
    let _ = writeln!(s, "  %okar = icmp eq i64 %ar, {n}");
    let _ = writeln!(s, "  br i1 %okar, label %args, label %slow");
    let _ = writeln!(s, "args:");
    for i in 0..n {
        let _ = writeln!(s, "  %t{i} = extractvalue %KValue %a{i}, 0");
        let _ = writeln!(s, "  %e{i} = icmp eq i64 %t{i}, 5");
    }
    for i in 1..n {
        let prev = match i {
            1 => "%e0".to_string(),
            _ => format!("%or{}", i - 1),
        };
        let _ = writeln!(s, "  %or{i} = or i1 {prev}, %e{i}");
    }
    match n {
        0 => {
            let _ = writeln!(s, "  br label %go");
        }
        1 => {
            let _ = writeln!(s, "  br i1 %e0, label %slow, label %go");
        }
        _ => {
            let _ = writeln!(s, "  br i1 %or{}, label %slow, label %go", n - 1);
        }
    }
    let _ = writeln!(s, "go:");
    let _ = writeln!(s, "  %envp = getelementptr i8, ptr %c, i64 8");
    let _ = writeln!(s, "  %env = load ptr, ptr %envp");
    let _ = writeln!(s, "  %fnp = load ptr, ptr %c");
    let _ = writeln!(s, "  %r = call %KValue %fnp(ptr %env{args})");
    let _ = writeln!(s, "  ret %KValue %r");
    let _ = writeln!(s, "slow:");
    let _ = writeln!(s, "  %s = call %KValue @k_call{n}(%KValue %f{args})");
    let _ = writeln!(s, "  ret %KValue %s");
    let _ = writeln!(s, "}}");
    s
}

/// LLVM symbol for a dispatcher: quoted when the kanso name carries a
/// module qualifier's slash.
fn wsym(name: &str, arity: usize) -> String {
    // fn-value wrapper symbols share dsym's quoted-identifier rule
    match name.contains(['/', '!', '?', '+', '-', '*', '%', '<', '>', '=']) {
        true => format!("\"w_{name}_{arity}\""),
        false => format!("w_{name}_{arity}"),
    }
}

/// The static a `k_fnref` value points at: the wrapper, its arity, and the
/// name the diagnostic says when a call brings the wrong number of arguments.
fn rsym(name: &str, arity: usize) -> String {
    quoted(&format!("r_{name}_{arity}"))
}

fn dsym(name: &str, arity: usize) -> String {
    quoted(&format!("d_{name}_{arity}"))
}

fn inline_tag(f: &mut FnEmit, value: &str) -> String {
    let t = f.tmp();
    f.line(&format!("{t} = extractvalue %KValue {value}, 0"));
    t
}

fn inline_payload(f: &mut FnEmit, value: &str) -> String {
    let t = f.tmp();
    f.line(&format!("{t} = extractvalue %KValue {value}, 1"));
    t
}

/// Calls the alwaysinline twin rather than restating its tag test, so the
/// emitter cannot drift from the definition it inlines.
fn inline_not_failure(f: &mut FnEmit, value: &str) -> String {
    let r = f.tmp();
    f.line(&format!("{r} = call i64 @k_not_failure(%KValue {value})"));
    let ok = f.tmp();
    f.line(&format!("{ok} = icmp ne i64 {r}, 0"));
    ok
}

impl<'a> Backend<'a> {
    fn group_indices(&self, name: &str, arity: usize) -> Vec<usize> {
        self.program
            .fns
            .iter()
            .enumerate()
            .filter(|(_, d)| d.name == name && d.params.len() == arity)
            .map(|(i, _)| i)
            .collect()
    }

    fn group_param_set(&self, name: &str, arity: usize, param: usize) -> Set {
        self.group_indices(name, arity)
            .iter()
            .fold(0, |acc, i| acc | self.inference.param(*i, param))
    }

    fn group_return_set(&self, name: &str, arity: usize) -> Set {
        self.group_indices(name, arity).iter().fold(0, |acc, i| acc | self.inference.returns[*i])
    }

    /// A parameter proven to be exactly `int` crosses the tailcc boundary as a
    /// raw i64 instead of a boxed KValue. The dispatcher re-boxes it at entry so
    /// the body is untouched; LLVM's SROA folds that rebox against the body's
    /// payload reads (same function), and folds each caller's box against the
    /// extract we emit here — so only a raw i64 travels the musttail edge LLVM
    /// cannot otherwise see through. Sound because inference forces every param
    /// of a function used as a first-class value to TOP, never a bare `int`.
    fn unboxed_param(&self, name: &str, arity: usize, param: usize) -> bool {
        self.group_param_set(name, arity, param) == INT
    }

    /// Render one call argument in the callee's ABI: raw i64 for an unboxed
    /// slot (extract the payload), boxed KValue otherwise.
    /// Any arity-matching arm inspecting this position (anything but a bare
    /// Var/Wildcard) means a thunk must force before dispatch can select.
    fn scrutinizes(&self, callee: &str, arity: usize, i: usize) -> bool {
        self.program.fns.iter().any(|d| {
            d.name == callee
                && d.params.len() == arity
                && !matches!(d.params.get(i), Some(Pattern::Var(..)) | Some(Pattern::Wildcard(_)))
        })
    }

    /// An operand as an ordinary value. Only a carried argument slot reads the
    /// two-word convention; every other consumer — a render, a list, an
    /// ordinary parameter — needs the record itself. Converting here rather
    /// than at the call is what keeps the hot path free: a chain of carried
    /// slots never builds a record at all, which is the whole point of the
    /// convention and worth 254 MB on a json decode.
    fn as_value(&self, f: &mut FnEmit, e: &str) -> String {
        match f.is_parsed(e) {
            true => f.box_parsed(e),
            false => e.to_string(),
        }
    }

    fn call_arg(
        &self,
        f: &mut FnEmit,
        callee: &str,
        arity: usize,
        i: usize,
        e: &str,
        arg: Option<&Expr>,
    ) -> String {
        // A string this group builds by joining onto itself needs its seed
        // converted where it enters from outside: a builder writes into the
        // header it was given, and an interned literal cannot be written
        // through. The recursive call is not converted — it is already
        // carrying the builder made here.
        // A parameter forwarded round the same cycle is carrying the builder
        // already, and seeding it again copies the whole string once per hop.
        // A beat loop rewinds the arena between iterations and the shelf carries
        // the accumulator's header across the rewind, so the seed has to happen
        // inside the bracket: converted outside it, the header sits below the
        // mark and the join finds a string that is not a builder.
        let carried = match arg {
            Some(Expr::Ident(_, span)) => self.builder_carried.contains(&(
                f.file.clone(),
                span.line as usize,
                span.col as usize,
            )),
            _ => false,
        };
        let entering = self.builder_params.contains(&(callee.to_string(), arity, i))
            && !(f.group == callee && f.arity == arity)
            && !carried;
        let seeded;
        let e = match entering {
            true => {
                let t = f.tmp();
                f.line(&format!("{t} = call %KValue @k_b_str_builder(%KValue {e})"));
                f.record(&t, f.set_of(e));
                seeded = t;
                seeded.as_str()
            }
            false => e,
        };
        let forced;
        let e = match f.set_of(e) & crate::infer::THUNK != 0 && self.scrutinizes(callee, arity, i) {
            true => {
                forced = self.maybe_force(f, e.to_string());
                forced.as_str()
            }
            false => e,
        };
        if self.is_byte_disc(callee, arity, i) {
            // `e` is an `at`-on-bytes KValue (byte or none); hand it over as a
            // raw i64 — the byte value, or 256 for none. The box `at` built and
            // this unbox fold away in the caller, so a raw byte crosses the edge.
            let tag = f.tmp();
            f.line(&format!("{tag} = extractvalue %KValue {e}, 0"));
            let payload = f.tmp();
            f.line(&format!("{payload} = extractvalue %KValue {e}, 1"));
            let is_none = f.tmp();
            f.line(&format!("{is_none} = icmp eq i64 {tag}, {K_NONE}"));
            let raw = f.tmp();
            f.line(&format!("{raw} = select i1 {is_none}, i64 256, i64 {payload}"));
            return format!("i64 {raw}");
        }
        // A register-returned record reaching a slot that wants an ordinary
        // value has to be built back into one. The two words carry the whole
        // record — `(field0.payload << 8 | field1.tag, field1.payload)` — so
        // nothing is lost, but only a real record can be dispatched on, and a
        // getter is exactly the caller that dispatches.
        let boxed;
        let e = match f.is_parsed(e) && self.escape.carries_ty(callee, arity, i).is_none() {
            true => {
                boxed = f.box_parsed(e);
                boxed.as_str()
            }
            false => e,
        };
        if self.escape.carries_ty(callee, arity, i).is_some() {
            if f.is_parsed(e) {
                return format!("%parsed {e}");
            }
            // a boxed record reached a by-value slot (a construction bound or
            // passed outside tail position): unpack it into the convention
            let f0 = f.tmp();
            f.line(&format!("{f0} = call %KValue @k_field_fast(%KValue {e}, i64 0)"));
            let f1 = f.tmp();
            f.line(&format!("{f1} = call %KValue @k_field_fast(%KValue {e}, i64 1)"));
            let posp = f.tmp();
            f.line(&format!("{posp} = extractvalue %KValue {f0}, 1"));
            let sh = f.tmp();
            f.line(&format!("{sh} = shl i64 {posp}, 8"));
            let vt = f.tmp();
            f.line(&format!("{vt} = extractvalue %KValue {f1}, 0"));
            let w0 = f.tmp();
            f.line(&format!("{w0} = or i64 {sh}, {vt}"));
            let w1 = f.tmp();
            f.line(&format!("{w1} = extractvalue %KValue {f1}, 1"));
            let a = f.tmp();
            f.line(&format!("{a} = insertvalue %parsed undef, i64 {w0}, 0"));
            let p = f.tmp();
            f.line(&format!("{p} = insertvalue %parsed {a}, i64 {w1}, 1"));
            format!("%parsed {p}")
        } else if self.unboxed_param(callee, arity, i) {
            let p = f.tmp();
            f.line(&format!("{p} = extractvalue %KValue {e}, 1"));
            format!("i64 {p}")
        } else {
            format!("%KValue {e}")
        }
    }

    /// Emit the entry-block reboxes that reconstruct each unboxed `%xi` param as
    /// the KValue the body expects.
    fn rebox_params(&self, f: &mut FnEmit, name: &str, arity: usize) {
        for i in 0..arity {
            if self.is_byte_disc(name, arity, i) {
                // Reconstruct the KValue the boxed dispatch expects: 256 is none,
                // anything else is that byte. The reconstruction folds back into
                // a raw switch, so only the raw i64 actually crossed the edge.
                let is_none = f.tmp();
                f.line(&format!("{is_none} = icmp eq i64 %x{i}r, 256"));
                f.line(&format!(
                    "%x{i}b = insertvalue %KValue {{ i64 0, i64 undef }}, i64 %x{i}r, 1"
                ));
                f.line(&format!(
                    "%x{i} = select i1 {is_none}, %KValue {{ i64 4, i64 0 }}, %KValue %x{i}b"
                ));
            } else if self.unboxed_param(name, arity, i) {
                f.line(&format!(
                    "%x{i} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 %x{i}r, 1"
                ));
                // the unboxing condition is the proof: this slot is an int, and
                // saying so lets arithmetic on it skip the tag test and the
                // boxed fallback it guards
                f.record(&format!("%x{i}"), INT);
            }
        }
    }

    /// A switch discriminator inference proves is `at`-on-bytes, so it crosses
    /// as a raw i64 (byte value, or 256 for none) and is switched on directly.
    fn is_byte_disc(&self, name: &str, arity: usize, param: usize) -> bool {
        self.byte_disc.contains(&(name.to_string(), arity, param))
    }

    /// The dispatcher's parameter list: a raw i64 for a byte discriminator or a
    /// proven-int slot, a `%parsed` struct for a register-returnable record,
    /// else a boxed KValue.
    fn abi_params(&self, name: &str, arity: usize) -> Vec<String> {
        (0..arity)
            .map(|i| {
                if self.is_byte_disc(name, arity, i) {
                    format!("i64 %x{i}r")
                } else if self.escape.carries_ty(name, arity, i).is_some() {
                    format!("%parsed %x{i}")
                } else if self.unboxed_param(name, arity, i) {
                    format!("i64 %x{i}r")
                } else {
                    format!("%KValue %x{i}")
                }
            })
            .collect()
    }

    /// The LLVM return type of a function group: `%parsed` when it hands back a
    /// register-returnable record by value, else `%KValue`.
    fn ret_ty(&self, name: &str, arity: usize) -> &'static str {
        // A knotted constant's cell holds a blackhole while the cycle is still
        // building, and a register-returned record has nowhere to put a thunk:
        // the caller would read the thunk's payload as a record pointer.
        if arity == 0 && self.knotted.contains(name) {
            return "%KValue";
        }
        if self.escape.returns_ty(name, arity).is_some() {
            "%parsed"
        } else {
            "%KValue"
        }
    }

    /// A group we can hand out as a first-class value through a `w_` wrapper: a
    /// `%KValue` return and no by-value or byte-discriminated parameters, which
    /// the generic wrapper does not know how to convert.
    fn simple_fn_value(&self, name: &str, arity: usize) -> bool {
        self.ret_ty(name, arity) == "%KValue"
            && (0..arity).all(|i| {
                !self.is_byte_disc(name, arity, i)
                    && self.escape.carries_ty(name, arity, i).is_none()
            })
    }

    /// A bind is representable as a native thunk when its captures fit the
    /// cell (args[8]).
    fn thunkable(&self, f: &FnEmit, expr: &Expr) -> bool {
        let mut idents = Vec::new();
        collect_idents(expr, &mut idents);
        let mut captures: Vec<&String> = Vec::new();
        for id in &idents {
            if f.lookup(id).is_some() && !captures.contains(&id) {
                captures.push(id);
            }
        }
        captures.len() <= 8
    }

    /// Compile an expression into a cell: a site function over the names it
    /// reads, plus a `k_thunk_new` that captures their current values. The
    /// computation runs at first force.
    fn emit_cell(&mut self, f: &mut FnEmit, expr: &Expr) -> Result<String, String> {
        let mut idents = Vec::new();
        collect_idents(expr, &mut idents);
        let mut captures: Vec<String> = Vec::new();
        for id in idents {
            if f.lookup(&id).is_some() && !captures.contains(&id) {
                captures.push(id);
            }
        }
        let site = self.thunk_sites.len();
        let sym = format!("tsite{site}");
        self.thunk_sites.push((sym.clone(), captures.len()));
        self.emit_thunk_site(&sym, &captures, expr, f)?;
        let mut args = String::new();
        for cap in &captures {
            let temp = f.lookup(cap).expect("capture is bound");
            // A capture is stored as a %KValue, and a group that returns its
            // record in registers holds a %parsed.
            let temp = self.as_value(f, &temp);
            args.push_str(&format!(", %KValue {temp}"));
        }
        let t = f.tmp();
        f.line(&format!(
            "{t} = call %KValue (i64, i32, ...) @k_thunk_new(i64 {site}, i32 {}{args})",
            captures.len()
        ));
        f.record(&t, crate::infer::TOP);
        Ok(t)
    }

    /// Force a value that may be a thunk; no-op (no IR) when the set proves
    /// it can't be one, so strict code pays nothing. A program the demand
    /// analysis deferred nothing in can hold no thunk anywhere — every site
    /// vanishes, not just the set-proven ones (conservative TOP widenings
    /// carry the THUNK bit into code no thunk can reach).
    /// Force unless the value is a cell still being computed. Only a
    /// constructor's fields take this path; everywhere else a blackhole
    /// reached is the error it exists to report.
    fn force_unless_knot(&self, f: &mut FnEmit, value: String) -> String {
        if f.set_of(&value) & crate::infer::THUNK == 0 {
            return value;
        }
        let post = f.set_of(&value) & !crate::infer::THUNK;
        let t = f.tmp();
        f.line(&format!("{t} = call %KValue @k_force_unless_black(%KValue {value})"));
        f.record(&t, if post == 0 { crate::infer::TOP } else { post | crate::infer::THUNK });
        t
    }

    fn maybe_force(&self, f: &mut FnEmit, value: String) -> String {
        // A deferred self-reference is a thunk that no lazy-bind count knows
        // about, so the cheap exit has to admit it too.
        if self.demand.lazy_bind_count() == 0 && !self.defers_self_reference {
            return value;
        }
        if f.set_of(&value) & crate::infer::THUNK == 0 {
            return value;
        }
        let post = f.set_of(&value) & !crate::infer::THUNK;
        let t = f.tmp();
        f.line(&format!("{t} = call %KValue @k_force_fast(%KValue {value})"));
        // A forced thunk can yield anything its expr could; the bind site
        // recorded TOP, so widen conservatively past the removed bit.
        f.record(&t, if post == 0 { crate::infer::TOP & !crate::infer::THUNK } else { post });
        t
    }

    /// A storing position inside a constant that names itself holds a thunk
    /// rather than a value. The constant's cell is empty while its own body
    /// runs, so reading it there would read nothing; deferred, the read
    /// happens after `k_caf_init` has filled the cell, and the cycle closes.
    ///
    /// No captures: the only free name in such an expression is the global
    /// itself, which the site loads for itself.
    fn deferred_or_emitted(&mut self, f: &mut FnEmit, expr: &Expr) -> Result<String, String> {
        if !self.defers_self(f, expr) {
            return self.emit_expr(f, expr);
        }
        let site = self.thunk_sites.len();
        let sym = format!("tsite{site}");
        self.thunk_sites.push((sym.clone(), 0));
        self.emit_thunk_site(&sym, &[], expr, f)?;
        let t = f.tmp();
        f.line(&format!("{t} = call %KValue (i64, i32, ...) @k_thunk_new(i64 {site}, i32 0)"));
        f.record(&t, crate::infer::TOP);
        Ok(t)
    }

    /// Whether this expression, in this frame, mentions the constant being
    /// built. `arity == 0` keeps it to constants: a function that mentions
    /// itself is ordinary recursion and has a base case.
    fn defers_self(&self, f: &FnEmit, expr: &Expr) -> bool {
        fn mentions(expr: &Expr, name: &str) -> bool {
            if let Expr::Ident(n, _) | Expr::Partial(n, _) = expr {
                if n == name {
                    return true;
                }
            }
            crate::any_child(expr, |c| mentions(c, name))
        }
        f.arity == 0 && !f.group.is_empty() && mentions(expr, &f.group)
    }

    fn emit_thunk_site(
        &mut self,
        sym: &str,
        captures: &[String],
        expr: &Expr,
        outer: &FnEmit,
    ) -> Result<(), String> {
        let mut f = FnEmit::new();
        f.origin_prefix = outer.origin_prefix.clone();
        f.hako = outer.hako.clone();
        f.hako = outer.hako.clone();
        f.file = outer.file.clone();
        f.start_block("entry");
        for (i, cap) in captures.iter().enumerate() {
            f.bind(cap, &format!("%a{i}"));
        }
        self.emit_tail(&mut f, expr)?;
        let sig: Vec<String> = (0..captures.len()).map(|i| format!("%KValue %a{i}")).collect();
        let _ = writeln!(
            self.body,
            "define tailcc %KValue @{sym}({}) {{\n{}}}\n",
            sig.join(", "),
            f.out
        );
        Ok(())
    }

    fn emit_thunk_dispatcher(&mut self) {
        let mut arms = String::new();
        let mut cases = String::new();
        for (site, (sym, argc)) in self.thunk_sites.iter().enumerate() {
            let _ = writeln!(cases, "    i64 {site}, label %s{site}");
            let mut loads = String::new();
            let mut args: Vec<String> = Vec::new();
            for i in 0..*argc {
                let _ =
                    writeln!(loads, "  %s{site}a{i}p = getelementptr %KValue, ptr %args, i64 {i}");
                let _ = writeln!(loads, "  %s{site}a{i} = load %KValue, ptr %s{site}a{i}p");
                args.push(format!("%KValue %s{site}a{i}"));
            }
            let _ = writeln!(
                arms,
                "s{site}:\n{loads}  %s{site}r = call tailcc %KValue @{sym}({})\n  ret %KValue %s{site}r",
                args.join(", ")
            );
        }
        let _ = writeln!(
            self.body,
            "define %KValue @d_thunk_eval(i64 %site, ptr %args) {{\nentry:\n  switch i64 %site, label %bad [\n{cases}  ]\n{arms}bad:\n  unreachable\n}}\n"
        );
    }

    fn emit(&mut self) -> Result<String, String> {
        self.emit_type_names();
        self.emit_type_fields();
        // group by name across the whole program: the bare overload space
        // interleaves same-named decls from different modules
        let mut groups: Vec<(&str, Vec<&FnDecl>)> = Vec::new();
        for decl in &self.program.fns {
            match groups.iter_mut().find(|(name, _)| *name == decl.name) {
                Some((_, decls)) => decls.push(decl),
                None => groups.push((&decl.name, vec![decl])),
            }
        }
        // proximity breaks specificity ties: local arms precede clones —
        // and a subtype annotation outranks its ancestors, so arms sort
        // by total chain depth, deepest first (the interp's scores, as an
        // ordering; tie-rejection outlaws the incomparable cases)
        let depth_of = |ty: &str| -> i64 {
            let mut d = 0i64;
            let mut cur = ty;
            while let Some(p) = self.sub_parents.get(cur) {
                d += 1;
                cur = p.as_str();
            }
            d
        };
        // the ladder as a sort: literals, then concrete annotations (nearer
        // subtype declarations first), then typesets, then the generics
        for (_, decls) in &mut groups {
            decls.sort_by_key(|d| {
                let total: i64 = d
                    .params
                    .iter()
                    .map(|p| match p {
                        Pattern::IntLit(..) | Pattern::StrLit(..) | Pattern::Nullary(..) => 3000,
                        Pattern::Annotated { ty, .. } => {
                            match self.typesets.contains_key(ty.as_str()) {
                                true => 1000,
                                false => 2000 + depth_of(ty),
                            }
                        }
                        // an err arm ranks as its reason pattern does: a
                        // named leaf with the concretes, a typeset with the
                        // typesets, a bare binder just above the generics
                        Pattern::Ctor { ty, fields, .. } if ty == "err" && fields.len() == 1 => {
                            match &fields[0] {
                                Pattern::Annotated { ty: rty, .. } => {
                                    match self.typesets.contains_key(rty.as_str()) {
                                        true => 1000,
                                        false => 2000 + depth_of(rty),
                                    }
                                }
                                Pattern::Var(..) | Pattern::Wildcard(..) => 1,
                                _ => 2000,
                            }
                        }
                        // a constructor pattern ranks by the same chain the
                        // annotations use: naming the subtype is nearer than
                        // naming what it wraps
                        Pattern::Ctor { ty, .. } => 2000 + depth_of(ty),
                        Pattern::Keyed { .. } => 2000,
                        Pattern::Var(..) | Pattern::Wildcard(..) => 0,
                    })
                    .sum();
                (std::cmp::Reverse(total), d.synthetic)
            });
        }
        for (name, decls) in &groups {
            let mut by_arity: HashMap<usize, Vec<&FnDecl>> = HashMap::default();
            for d in decls {
                by_arity.entry(d.params.len()).or_default().push(d);
            }
            let mut arity_keys: Vec<usize> = by_arity.keys().copied().collect();
            arity_keys.sort_unstable();
            for arity in arity_keys {
                self.emit_dispatcher(name, arity, &by_arity[&arity])?;
            }
        }
        self.fn_value_wrappers.sort();
        self.fn_value_wrappers.dedup();
        let wrappers = self.fn_value_wrappers.clone();
        for (name, arity) in &wrappers {
            let arity = *arity;
            let params: Vec<String> = (0..arity).map(|i| format!("%KValue %a{i}")).collect();
            let mut conv = String::new();
            let call_args: Vec<String> = (0..arity)
                .map(|i| {
                    if self.unboxed_param(name, arity, i) {
                        let _ = writeln!(conv, "  %p{i} = extractvalue %KValue %a{i}, 1");
                        format!("i64 %p{i}")
                    } else {
                        format!("%KValue %a{i}")
                    }
                })
                .collect();
            let sym = dsym(name, arity);
            let _ = writeln!(conv, "  %r = call tailcc %KValue @{sym}({})", call_args.join(", "));
            let _ = writeln!(
                self.body,
                "define %KValue @{}({}) {{\nentry:\n{conv}  ret %KValue %r\n}}\n",
                wsym(name, arity),
                params.join(", ")
            );
            let (label, _) = self.intern(&format!("{name}\0"));
            let _ = writeln!(
                self.body,
                "@{} = private constant {{ ptr, i64, ptr, i64 }} \
                 {{ ptr @{}, i64 {arity}, ptr @{label}, i64 0 }}",
                rsym(name, arity),
                wsym(name, arity)
            );
        }
        self.builtin_value_wrappers.sort();
        self.builtin_value_wrappers.dedup();
        let builtins = self.builtin_value_wrappers.clone();
        for (name, arity) in &builtins {
            let arity = *arity;
            let params: Vec<String> = (0..arity).map(|i| format!("%KValue %a{i}")).collect();
            let call_args: Vec<String> = (0..arity).map(|i| format!("%KValue %a{i}")).collect();
            let held = format!("builtin.{name}");
            let _ = writeln!(
                self.body,
                "define %KValue @{}({}) {{\nentry:\n  %r = call %KValue @k_b_{name}({})\n  \
                 ret %KValue %r\n}}\n",
                wsym(&held, arity),
                params.join(", "),
                call_args.join(", ")
            );
            let (label, _) = self.intern(&format!("{name}\0"));
            let _ = writeln!(
                self.body,
                "@{} = private constant {{ ptr, i64, ptr, i64 }} \
                 {{ ptr @{}, i64 {arity}, ptr @{label}, i64 1 }}",
                rsym(&held, arity),
                wsym(&held, arity)
            );
        }
        if self.print_value_wrapper {
            let group = "render/to_string";
            let render = match self.program.fns.iter().any(|d| d.name == group) {
                true => format!(
                    "  %s1 = call tailcc %KValue @{}(%KValue %v)\n  br label %join",
                    dsym(group, 1)
                ),
                false => "  %s1 = call %KValue @k_render(%KValue %v, i64 0)\n  br label %join"
                    .to_string(),
            };
            let held = "builtin.print";
            let _ = writeln!(
                self.body,
                "define %KValue @{}(%KValue %a0) {{\nentry:\n  \
                 %v = call %KValue @k_force(%KValue %a0)\n  \
                 %d = call i64 @k_render_dispatchable(%KValue %v)\n  \
                 %c = icmp ne i64 %d, 0\n  \
                 br i1 %c, label %arm, label %plain\n\
                 arm:\n{render}\n\
                 plain:\n  \
                 %s2 = call %KValue @k_render(%KValue %v, i64 0)\n  \
                 br label %join\n\
                 join:\n  \
                 %s = phi %KValue [ %s1, %arm ], [ %s2, %plain ]\n  \
                 %r = call %KValue @k_desc_print(%KValue %s)\n  \
                 ret %KValue %r\n}}\n",
                wsym(held, 1)
            );
            let (label, _) = self.intern("print\0");
            let _ = writeln!(
                self.body,
                "@{} = private constant {{ ptr, i64, ptr, i64 }} \
                 {{ ptr @{}, i64 1, ptr @{label}, i64 1 }}",
                rsym(held, 1),
                wsym(held, 1)
            );
        }
        // Lazy v1: the thunk-site dispatcher the runtime's k_force calls.
        // Sites are emitted as cases as lazy binds are compiled; a program
        // with no lazy sites still defines the symbol so every binary links.
        self.emit_thunk_dispatcher();
        // No fills. Each constant seeds and builds its own cell on the first
        // read, so a program that never demands one never builds it. What is
        // left here is the math-id handshake below.
        let fills = String::new();
        // Division answers a declared type, so the runtime has to be told which
        // id the compiler gave it. Before the constants, because a constant may
        // divide. A program that cannot reach a math failure never declares the
        // pair and never builds one, so it has nothing to tell.
        let ids = match (
            self.type_ids.get(crate::DIVIDE_BY_ZERO),
            self.type_ids.get(crate::MATH_FAILURE),
        ) {
            (Some(dz), Some(mf)) => format!("  call void @k_math_ids(i64 {dz}, i64 {mf})\n"),
            _ => String::new(),
        };
        let _ = writeln!(
            self.body,
            "define void @k_caf_init() {{\nentry:\n{ids}{fills}  ret void\n}}\n"
        );
        // A library has no entry to call, and a stub calling one that is not
        // there is a symbol the linker would ask about.
        if self.program.fns.iter().any(|d| d.name == crate::ast::ENTRY) {
            let entry = dsym(crate::ast::ENTRY, 0);
            let _ = writeln!(
                self.body,
                "define %KValue @k_user_main() {{\nentry:\n  %r = call tailcc %KValue \
             @{entry}()\n  ret %KValue %r\n}}\n"
            );
        }
        // A declaration the program never calls is a line the compile golden
        // counts and the reader scrolls past. Keep a declare only when its
        // symbol appears somewhere outside the declare itself — in the body,
        // or inside one of the preamble's own inline definitions.
        // Only a program with an entry has a place for the walk to start. A
        // library's surface is its callers' business, and every definition in
        // it is reachable from outside the module the emitter can see.
        let body = match self.program.fns.iter().any(|d| d.name == crate::ast::ENTRY) {
            true => prune_unnamed(&self.body, &dsym(crate::ast::ENTRY, 0)),
            false => self.body.clone(),
        };
        // One inline dispatcher per arity the program actually writes. An
        // unused `internal` definition costs nothing after optimization, but
        // it does cost a line, a define and a branch in the emitted golden —
        // so the twins are generated against the body rather than carried in
        // DECLARES the way the other inline helpers are.
        let call_twins: String = (0..=4)
            .filter(|n| body.contains(&format!("@k_call{n}_fast(")))
            .map(call_twin)
            .collect();
        let declares: String = {
            let referenced = |sym: &str| {
                let probe = format!("@{sym}(");
                body.contains(&probe)
                    || call_twins.contains(&probe)
                    || DECLARES
                        .lines()
                        .filter(|l| !l.starts_with("declare"))
                        .any(|l| l.contains(&probe))
            };
            DECLARES
                .lines()
                .filter(|line| {
                    let Some(rest) = line.strip_prefix("declare ") else { return true };
                    let Some(at) = rest.find('@') else { return true };
                    let sym = &rest[at + 1..];
                    let Some(paren) = sym.find('(') else { return true };
                    referenced(&sym[..paren])
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        let mut out = declares;
        out.push('\n');
        out.push_str(&call_twins);
        for cell in &self.caf_cells {
            let _ = writeln!(out, "@{cell} = internal global %KValue zeroinitializer");
            let _ = writeln!(out, "@{cell}_ready = internal global i8 0");
        }
        for (name, bytes) in &self.strings {
            let _ = writeln!(
                out,
                "@{name} = private unnamed_addr constant [{} x i8] c\"{}\"",
                bytes.len(),
                ir_bytes(bytes)
            );
            let _ = writeln!(out, "@{name}_lit = internal global %KValue zeroinitializer");
        }
        out.push('\n');
        out.push_str(&body);
        Ok(narrow_tailcc(out))
    }

    /// Can a value matching this annotation be an err? Only then is the
    /// own-origin guard worth emitting — every other pattern cannot see a
    /// failure in the first place, so the check would be a call per match on
    /// a hot path to learn nothing.
    fn admits_err(&self, ty: &str) -> bool {
        if ty == "err" {
            return true;
        }
        match self.typesets.get(ty) {
            Some(members) => members.iter().any(|m| m != ty && self.admits_err(m)),
            None => false,
        }
    }

    /// The arm's package as an interned literal, for `k_not_own_err`.
    fn arm_hako(&mut self, f: &FnEmit) -> String {
        let (name, _) = self.intern(&format!("{}\0", f.hako));
        name
    }

    fn intern(&mut self, text: &str) -> (String, usize) {
        let bytes = text.as_bytes().to_vec();
        let len = bytes.len();
        if let Some(name) = self.interned.get(&bytes) {
            return (name.clone(), len);
        }
        let name = format!("s{}", self.strings.len());
        self.interned.insert(bytes.clone(), name.clone());
        self.strings.push((name.clone(), bytes));
        (name, len)
    }

    fn str_const(&mut self, f: &mut FnEmit, text: &str) -> String {
        // a literal is the same value every evaluation, so it builds once
        // into a permanent slot instead of allocating per visit
        let (name, len) = self.intern(text);
        let t = f.tmp();
        f.line(&format!("{t} = call %KValue @k_str_lit(ptr @{name}, i64 {len}, ptr @{name}_lit)"));
        t
    }

    /// The interned literal for an err construction site: the package that
    /// raises here, then the trace line, each NUL-terminated. The runtime
    /// reads the first for the match rule and the second for the report —
    /// one argument instead of a second threaded through nineteen runtime
    /// signatures to carry a package name.
    fn origin_arg(&mut self, f: &FnEmit, span: Span) -> String {
        let (name, _) = self.intern(&format!("{}\0{}:{}\0", f.hako, f.origin_prefix, span.line));
        format!("ptr @{name}")
    }

    fn emit_type_names(&mut self) {
        let mut body = String::new();
        body.push_str("define ptr @k_type_name(i64 %id) {\nentry:\n");
        let mut arms = String::new();
        let mut cases = String::new();
        for ty in &self.program.types {
            if ty.origin.is_some() {
                // an alias shares its origin's id; the origin owns the case
                continue;
            }
            let id = self.type_ids[ty.name.as_str()];
            let (name, _len) = self.intern(&format!("{}\0", ty.name));
            let _ = writeln!(cases, "    i64 {id}, label %T{id}");
            let _ = writeln!(arms, "T{id}:\n  ret ptr @{name}");
        }
        let (entry_name, _) = self.intern("entry\0");
        let _ = writeln!(cases, "    i64 0, label %T0");
        let _ = writeln!(arms, "T0:\n  ret ptr @{entry_name}");
        let (fallback, _) = self.intern("record\0");
        let _ = writeln!(body, "  switch i64 %id, label %TD [\n{cases}  ]");
        body.push_str(&arms);
        let _ = writeln!(body, "TD:\n  ret ptr @{fallback}");
        body.push_str("}\n\n");
        self.body.push_str(&body);
    }

    /// Field metadata for keyed reads: name-indexed lookup resolves against
    /// these per-type switch tables at runtime.
    fn emit_type_fields(&mut self) {
        let mut tables: Vec<(i64, Vec<String>)> = vec![(0, vec!["key".into(), "value".into()])];
        for ty in &self.program.types {
            if ty.origin.is_some() {
                // an alias shares its origin's id; the origin owns the case
                continue;
            }
            let id = self.type_ids[ty.name.as_str()];
            let fields = ty.fields.iter().map(|(name, _, _)| name.clone()).collect();
            tables.push((id, fields));
        }
        let mut body = String::new();
        body.push_str("define i64 @k_type_field_count(i64 %id) {\nentry:\n");
        let mut cases = String::new();
        let mut arms = String::new();
        for (id, fields) in &tables {
            let _ = writeln!(cases, "    i64 {id}, label %C{id}");
            let _ = writeln!(arms, "C{id}:\n  ret i64 {}", fields.len());
        }
        let _ = writeln!(body, "  switch i64 %id, label %CD [\n{cases}  ]");
        body.push_str(&arms);
        body.push_str("CD:\n  ret i64 0\n}\n\n");
        body.push_str("define ptr @k_type_field_name(i64 %id, i64 %i) {\nentry:\n");
        let (empty, _) = self.intern("\0");
        let mut cases = String::new();
        let mut arms = String::new();
        for (id, fields) in &tables {
            let _ = writeln!(cases, "    i64 {id}, label %T{id}");
            let mut inner = String::new();
            for (i, field) in fields.iter().enumerate() {
                let (name, _) = self.intern(&format!("{field}\0"));
                let _ = writeln!(inner, "    i64 {i}, label %T{id}F{i}");
                let _ = writeln!(arms, "T{id}F{i}:\n  ret ptr @{name}");
            }
            let _ = writeln!(arms, "T{id}:\n  switch i64 %i, label %TD [\n{inner}  ]");
        }
        let _ = writeln!(body, "  switch i64 %id, label %TD [\n{cases}  ]");
        body.push_str(&arms);
        let _ = writeln!(body, "TD:\n  ret ptr @{empty}");
        body.push_str("}\n\n");
        self.body.push_str(&body);
    }

    /// A group whose arms discriminate on one parameter with int/none literals
    /// (other params generic) compiles to a switch instead of an arm cascade.
    fn switch_shape(decls: &[&FnDecl]) -> Option<usize> {
        let arity = decls[0].params.len();
        if arity == 0 {
            return None;
        }
        let mut disc: Option<usize> = None;
        let mut int_arms = 0;
        for decl in decls {
            for (i, pattern) in decl.params.iter().enumerate() {
                match pattern {
                    Pattern::Var(..) | Pattern::Wildcard(..) => {}
                    Pattern::IntLit(..) | Pattern::Nullary(..) => {
                        if disc.is_some_and(|d| d != i) {
                            return None;
                        }
                        disc = Some(i);
                        if matches!(pattern, Pattern::IntLit(..)) {
                            int_arms += 1;
                        }
                    }
                    _ => return None,
                }
            }
        }
        match (disc, int_arms >= 2) {
            (Some(d), true) => Some(d),
            _ => None,
        }
    }

    fn emit_switch_dispatcher(
        &mut self,
        name: &str,
        arity: usize,
        decls: &[&FnDecl],
        disc: usize,
    ) -> Result<(), String> {
        let params = self.abi_params(name, arity);
        let ret = self.ret_ty(name, arity);
        let mut f = FnEmit::new();
        f.ret_ty = ret.to_string();
        f.group = name.to_string();
        f.arity = arity;
        let sym_hdr = dsym(name, arity);
        let header = format!("define tailcc {ret} @{sym_hdr}({}) {{", params.join(", "));
        let (hop_name, _) = self.intern(&format!("{name}\0"));
        f.start_block("entry");
        self.rebox_params(&mut f, name, arity);
        // any non-discriminator failure means no arm can match: propagate leftmost
        let mut all_ok: Option<String> = None;
        for i in 0..arity {
            if i == disc {
                continue;
            }
            if self.group_param_set(name, arity, i) & FAIL == 0 {
                continue;
            }
            let ok = inline_not_failure(&mut f, &format!("%x{i}"));
            all_ok = Some(match all_ok {
                None => ok,
                Some(prev) => {
                    let t = f.tmp();
                    f.line(&format!("{t} = and i1 {prev}, {ok}"));
                    t
                }
            });
        }
        let dispatch = f.label();
        if let Some(ok) = all_ok {
            let propagate = f.label();
            f.line(&format!("br i1 {ok}, label %{dispatch}, label %{propagate}"));
            f.start_block(&propagate);
            for i in 0..arity {
                let good = inline_not_failure(&mut f, &format!("%x{i}"));
                let next = f.label();
                let ret_it = f.label();
                f.line(&format!("br i1 {good}, label %{next}, label %{ret_it}"));
                f.start_block(&ret_it);
                let hopped = f.tmp();
                f.line(&format!(
                    "{hopped} = call %KValue @k_err_hop(%KValue %x{i}, ptr @{hop_name})"
                ));
                self.emit_ret_failure(&mut f, name, arity, &hopped);
                f.start_block(&next);
            }
            f.line("unreachable");
        } else {
            f.line(&format!("br label %{dispatch}"));
        }
        f.start_block(&dispatch);
        let dv = format!("%x{disc}");
        let tag = inline_tag(&mut f, &dv);
        // classify arms
        let mut int_cases: Vec<(String, String)> = Vec::new();
        let mut nullary_cases: Vec<(i64, String)> = Vec::new();
        let mut generic_arm: Option<usize> = None;
        let mut arm_labels = Vec::new();
        for (k, decl) in decls.iter().enumerate() {
            let label = format!("arm{k}");
            arm_labels.push(label.clone());
            match &decl.params[disc] {
                Pattern::IntLit(n, _) => int_cases.push((n.to_string(), label)),
                Pattern::Nullary(nm, _) => {
                    let t = match nm.as_str() {
                        "true" => K_TRUE,
                        "false" => K_FALSE,
                        _ => K_NONE,
                    };
                    nullary_cases.push((t, label));
                }
                _ => generic_arm = Some(k),
            }
        }
        let is_int = f.tmp();
        f.line(&format!("{is_int} = icmp eq i64 {tag}, 0"));
        let int_block = f.label();
        let not_int = f.label();
        f.line(&format!("br i1 {is_int}, label %{int_block}, label %{not_int}"));
        f.start_block(&int_block);
        let payload = inline_payload(&mut f, &dv);
        let generic_label = match generic_arm {
            Some(k) => format!("arm{k}"),
            None => "nomatch".to_string(),
        };
        let cases: Vec<String> =
            int_cases.iter().map(|(n, l)| format!("    i64 {n}, label %{l}")).collect();
        f.line(&format!(
            "switch i64 {payload}, label %{generic_label} [
{}
  ]",
            cases.join(
                "
"
            )
        ));
        f.start_block(&not_int);
        // nullary tags, then generic (non-failure) or propagation
        for (t, l) in &nullary_cases {
            let hit = f.tmp();
            f.line(&format!("{hit} = icmp eq i64 {tag}, {t}"));
            let next = f.label();
            f.line(&format!("br i1 {hit}, label %{l}, label %{next}"));
            f.start_block(&next);
        }
        let disc_ok = inline_not_failure(&mut f, &dv);
        let nomatch = "nomatch".to_string();
        f.line(&format!("br i1 {disc_ok}, label %{generic_label}, label %{nomatch}"));
        f.start_block("nomatch");
        // no arm matched: the discriminator is the only possible failure here
        let disc_fail = f.tmp();
        f.line(&format!("{disc_fail} = extractvalue %KValue {dv}, 0"));
        let is_err = f.tmp();
        f.line(&format!("{is_err} = icmp eq i64 {disc_fail}, 5"));
        let is_none = f.tmp();
        f.line(&format!("{is_none} = icmp eq i64 {disc_fail}, 4"));
        let failing = f.tmp();
        f.line(&format!("{failing} = or i1 {is_err}, {is_none}"));
        let ret_disc = f.label();
        let die = f.label();
        f.line(&format!("br i1 {failing}, label %{ret_disc}, label %{die}"));
        f.start_block(&ret_disc);
        let hopped = f.tmp();
        f.line(&format!("{hopped} = call %KValue @k_err_hop(%KValue {dv}, ptr @{hop_name})"));
        self.emit_ret_failure(&mut f, name, arity, &hopped);
        f.start_block(&die);
        let msg = format!("no overload of `{name}` matches these arguments\0");
        let (m, _len) = self.intern(&msg);
        f.line(&format!("call void @k_die(ptr @{m})"));
        f.line("unreachable");
        // arm bodies: patterns are known matched, only bind generics
        for (k, decl) in decls.iter().enumerate() {
            f.start_block(&arm_labels[k]);
            f.versions.clear();
            f.origin_prefix = format!("{} at {}", crate::ast::frame_name(&decl.name), decl.file);
            f.hako = crate::provenance::package_of(&decl.file).to_string();
            f.file = decl.file.clone();
            f.synthetic = decl.synthetic;
            for (i, pattern) in decl.params.iter().enumerate() {
                if let Pattern::Var(pname, _) = pattern {
                    f.bind(pname, &format!("%x{i}"));
                }
            }
            self.emit_fn_body(&mut f, &decl.body)?;
        }
        let _ = writeln!(
            self.body,
            "{header}
{}}}
",
            f.out
        );
        Ok(())
    }

    /// The check call accepting one named type (chain-aware when the
    /// program declares subtypes). Shared by concrete annotations and
    /// typeset members.
    fn type_check_call(&self, value: &str, ty: &str) -> Result<String, String> {
        let subs = !self.sub_parents.is_empty();
        Ok(match ty {
            "int" if subs => format!("call i64 @k_check_sub_tag(%KValue {value}, i64 0)"),
            "int" => format!("call i64 @k_check_tag(%KValue {value}, i64 0)"),
            "float64" if subs => format!("call i64 @k_check_sub_tag(%KValue {value}, i64 1)"),
            "float64" => format!("call i64 @k_check_tag(%KValue {value}, i64 1)"),
            "string" if subs => format!("call i64 @k_check_sub_tag(%KValue {value}, i64 6)"),
            "string" => format!("call i64 @k_check_tag(%KValue {value}, i64 6)"),
            "bool" if subs => format!("call i64 @k_check_sub_bool(%KValue {value})"),
            "bool" => format!("call i64 @k_check_bool(%KValue {value})"),
            "err" => format!("call i64 @k_check_tag(%KValue {value}, i64 {K_ERR})"),
            "none" => format!("call i64 @k_check_tag(%KValue {value}, i64 {K_NONE})"),
            // `some` is any value that is not none, and a failure is not a
            // value: without this arm the backend refused the annotation
            // outright, where the interpreter took it and the checker had
            // already passed the program.
            "some" => format!("call i64 @k_check_some(%KValue {value})"),
            other => match self.type_ids.get(other) {
                Some(id) if self.sub_parents.contains_key(other) => {
                    format!("call i64 @k_check_sub_id(%KValue {value}, i64 {id})")
                }
                Some(id) => {
                    let nfields = self.field_count(other)?;
                    match subs {
                        true => format!(
                            "call i64 @k_check_sub_rec(%KValue {value}, i64 {id}, i64 {nfields})"
                        ),
                        false => format!(
                            "call i64 @k_check_rec_fast(%KValue {value}, i64 {id}, i64 {nfields})"
                        ),
                    }
                }
                None => return Err(format!("native backend: unknown type `{other}`")),
            },
        })
    }

    /// The runtime's `want` encoding for a chain target: a declared
    /// type's id, or -(tag + 1) for a primitive.
    fn sub_want(&self, ty: &str) -> Result<i64, String> {
        Ok(match ty {
            "int" => -1,
            "float64" => -2,
            "string" => -7,
            other => match self.type_ids.get(other) {
                Some(id) => *id,
                None => return Err(format!("native backend: unknown type `{other}`")),
            },
        })
    }

    fn emit_dispatcher(
        &mut self,
        name: &str,
        arity: usize,
        decls: &[&FnDecl],
    ) -> Result<(), String> {
        if arity == 0 && decls.len() == 1 && self.is_constant_body(decls[0]) {
            return self.emit_frozen_constant(name, decls);
        }
        self.emit_dispatcher_as(&dsym(name, arity), name, arity, decls)
    }

    /// A zero-argument definition whose body is a literal is a constant, so it
    /// is worth building once. The body emits unchanged under a build symbol
    /// and the real symbol becomes a cache in front of it.
    /// A knotted constant must be frozen whether or not its body is a literal:
    /// unfrozen, the real symbol recomputes its body, so a mention inside that
    /// body re-enters the builder and the recursion has no floor. Frozen, the
    /// mention is a load from a cell that `k_caf_init` fills once before main,
    /// which is what makes the cycle finite.
    fn is_constant_body(&self, decl: &FnDecl) -> bool {
        if self.knotted.contains(&decl.name) {
            return true;
        }
        fn literal(expr: &Expr) -> bool {
            match expr {
                Expr::Int(..) | Expr::Float(..) => true,
                Expr::Str(parts, _) => parts.iter().all(|p| matches!(p, TemplatePart::Lit(_))),
                Expr::List(items, _) => items.iter().all(literal),
                Expr::MapLit(pairs, _) => pairs.iter().all(|(k, v)| literal(k) && literal(v)),
                _ => false,
            }
        }
        match decl.body.as_slice() {
            [Stmt::Expr(expr)] => literal(expr),
            _ => false,
        }
    }

    fn emit_frozen_constant(&mut self, name: &str, decls: &[&FnDecl]) -> Result<(), String> {
        let sym = dsym(name, 0);
        // a module-qualified name is quoted, so the suffix goes inside the quotes
        let build = match sym.strip_suffix('"') {
            Some(head) => format!("{head}_build\""),
            None => format!("{sym}_build"),
        };
        self.emit_dispatcher_as(&build, name, 0, decls)?;
        let cell = format!("caf_{}", self.caf_cells.len());
        self.caf_cells.push(cell.clone());
        // Built on first demand, not before main. `k_caf_init` used to run
        // every builder at startup, which made an undemanded knot do its work
        // anyway — ruled 2026-08-23 to be wrong, because work defers until it
        // is presented to IO and eager evaluation is a resource heuristic
        // inside that contract rather than a semantic an engine may expose.
        //
        // The ready flag is set BEFORE the builder runs, for the same reason
        // `k_caf_init` seeded every cell before running any builder: a
        // constant that mentions itself re-enters here, and it has to find the
        // blackhole rather than the zeroed global, which is an integer zero
        // and reads as one. That seeding is what makes the cycle finite.
        //
        // One branch, taken once. The alternative Clay named is update in
        // place — rewriting the indirection at first evaluation so later reads
        // check nothing — and it is the better shape if this costs anything
        // measurable. The number goes to the ledger before any freeze returns.
        let _ = writeln!(
            self.body,
            "define tailcc %KValue @{sym}() {{\n\
             entry:\n  \
               %r = load i8, ptr @{cell}_ready\n  \
               %is = icmp eq i8 %r, 0\n  \
               br i1 %is, label %build, label %ready\n\
             build:\n  \
               %b = call %KValue @k_caf_blackhole()\n  \
               store %KValue %b, ptr @{cell}\n  \
               store i8 1, ptr @{cell}_ready\n  \
               %v = call tailcc %KValue @{build}()\n  \
               %f = call %KValue @k_caf_complete(%KValue %v, %KValue %b)\n  \
               store %KValue %f, ptr @{cell}\n  \
               ret %KValue %f\n\
             ready:\n  \
               %c = load %KValue, ptr @{cell}\n  \
               ret %KValue %c\n\
             }}\n"
        );
        Ok(())
    }

    fn emit_dispatcher_as(
        &mut self,
        sym_hdr: &str,
        name: &str,
        arity: usize,
        decls: &[&FnDecl],
    ) -> Result<(), String> {
        if let Some(disc) = Self::switch_shape(decls) {
            return self.emit_switch_dispatcher(name, arity, decls, disc);
        }
        let params = self.abi_params(name, arity);
        let ret = self.ret_ty(name, arity);
        let mut f = FnEmit::new();
        f.ret_ty = ret.to_string();
        f.group = name.to_string();
        f.arity = arity;
        let header = format!("define tailcc {ret} @{sym_hdr}({}) {{", params.join(", "));
        let (hop_name, _) = self.intern(&format!("{name}\0"));
        f.start_block("entry");
        self.rebox_params(&mut f, name, arity);
        // A `%parsed` and a `%KValue` share a `{i64,i64}` layout: reinterpret the
        // parameter's two words as the discriminator KValue once, so the arms can
        // match failures and the propagation loop can hop. On the failure path it
        // *is* the failure; on success its low word (value.tag | pos<<8) is never
        // a failure tag, so `k_not_failure` still separates the two.
        for i in 0..arity {
            if self.escape.carries_ty(name, arity, i).is_some() {
                f.line(&format!("%x{i}w0 = extractvalue %parsed %x{i}, 0"));
                f.line(&format!("%x{i}w1 = extractvalue %parsed %x{i}, 1"));
                f.line(&format!("%x{i}sa = insertvalue %KValue undef, i64 %x{i}w0, 0"));
                f.line(&format!("%x{i}s = insertvalue %KValue %x{i}sa, i64 %x{i}w1, 1"));
            }
        }
        // A releasable cell is created inside an arm's body, so it exists only
        // in blocks that body dominates. The next arm's blocks and the
        // parameter-failure blocks below are reached without running it, and
        // releasing it there emits a use LLVM's verifier refuses. The cells
        // an arm registers are its own: each arm starts from this watermark.
        let cells_before = f.lazy_cells.len();
        for (k, decl) in decls.iter().enumerate() {
            let fail = format!("fail{k}");
            f.lazy_cells.truncate(cells_before);
            f.versions.clear();
            f.origin_prefix = format!("{} at {}", crate::ast::frame_name(&decl.name), decl.file);
            f.hako = crate::provenance::package_of(&decl.file).to_string();
            f.file = decl.file.clone();
            f.synthetic = decl.synthetic;
            for (i, pattern) in decl.params.iter().enumerate() {
                match self.escape.carries_ty(name, arity, i) {
                    Some(ty) => {
                        let ty = ty.to_string();
                        self.emit_parsed_pattern(&mut f, &format!("%x{i}s"), pattern, &fail, &ty)?;
                    }
                    None => {
                        let known = self.group_param_set(name, arity, i);
                        self.emit_pattern_known(&mut f, &format!("%x{i}"), pattern, &fail, known)?;
                    }
                }
            }
            self.emit_fn_body(&mut f, &decl.body)?;
            f.start_block(&fail);
        }
        f.lazy_cells.truncate(cells_before);
        for i in 0..arity {
            let val = match self.escape.carries_ty(name, arity, i).is_some() {
                true => format!("%x{i}s"),
                false => format!("%x{i}"),
            };
            let ok = inline_not_failure(&mut f, &val);
            let ret_label = f.label();
            let next = f.label();
            f.line(&format!("br i1 {ok}, label %{next}, label %{ret_label}"));
            f.start_block(&ret_label);
            let hopped = f.tmp();
            f.line(&format!("{hopped} = call %KValue @k_err_hop(%KValue {val}, ptr @{hop_name})"));
            self.emit_ret_failure(&mut f, name, arity, &hopped);
            f.start_block(&next);
        }
        // a getter that matched nothing is a field error to the reader, and
        // only the runtime can name the value it was handed
        match crate::ast::getter_field(name) {
            Some(field) if arity == 1 => {
                let (lit, _len) = self.intern(&format!("{field}\0"));
                // a getter never takes the by-value convention, so its
                // parameter is already an ordinary value here
                if self.ret_ty(name, arity) == "%parsed" {
                    f.line(&format!("call void @k_no_field(%KValue %x0, ptr @{lit})"));
                } else {
                    let got = f.tmp();
                    f.line(&format!(
                        "{got} = call %KValue @k_field_forced(%KValue %x0, ptr @{lit})"
                    ));
                    self.emit_ret(&mut f, &got);
                }
            }
            _ => {
                let msg = format!("no overload of `{name}` matches these arguments");
                let (m, _len) = self.intern(&format!("{msg}\0"));
                f.line(&format!("call void @k_die(ptr @{m})"));
            }
        }
        f.line("unreachable");
        let _ = writeln!(self.body, "{header}\n{}}}\n", f.out);
        Ok(())
    }

    fn emit_pattern(
        &mut self,
        f: &mut FnEmit,
        value: &str,
        pattern: &Pattern,
        fail: &str,
    ) -> Result<(), String> {
        self.emit_pattern_known(f, value, pattern, fail, TOP)
    }

    /// Match a pattern against a `%parsed` parameter. The `(ty ...)` arm succeeds
    /// when the discriminator is not a failure and binds the fields straight from
    /// the struct (no heap read); every other pattern (`none`, `(err ...)`,
    /// wildcard) matches the discriminator KValue exactly as the old boxed value
    /// would have.
    fn emit_parsed_pattern(
        &mut self,
        f: &mut FnEmit,
        status: &str,
        pattern: &Pattern,
        fail: &str,
        ty: &str,
    ) -> Result<(), String> {
        if let Pattern::Ctor { ty: pty, fields, whole } = pattern {
            if pty == ty {
                let ok = inline_not_failure(f, status);
                let cont = f.label();
                f.line(&format!("br i1 {ok}, label %{cont}, label %{fail}"));
                f.start_block(&cont);
                let w0 = f.tmp();
                f.line(&format!("{w0} = extractvalue %KValue {status}, 0"));
                let w1 = f.tmp();
                f.line(&format!("{w1} = extractvalue %KValue {status}, 1"));
                // field 0: the position, unshifted out of the tag word.
                let posp = f.tmp();
                f.line(&format!("{posp} = lshr i64 {w0}, 8"));
                let posa = f.tmp();
                f.line(&format!("{posa} = insertvalue %KValue undef, i64 0, 0"));
                let poskv = f.tmp();
                f.line(&format!("{poskv} = insertvalue %KValue {posa}, i64 {posp}, 1"));
                self.emit_pattern(f, &poskv, &fields[0], fail)?;
                // field 1: the value, its tag masked back out of the low byte.
                let vtag = f.tmp();
                f.line(&format!("{vtag} = and i64 {w0}, 255"));
                let va = f.tmp();
                f.line(&format!("{va} = insertvalue %KValue undef, i64 {vtag}, 0"));
                let vkv = f.tmp();
                f.line(&format!("{vkv} = insertvalue %KValue {va}, i64 {w1}, 1"));
                self.emit_pattern(f, &vkv, &fields[1], fail)?;
                if let Some(named) = whole {
                    f.bind(&named.0, status);
                }
                return Ok(());
            }
        }
        self.emit_pattern_known(f, status, pattern, fail, TOP)
    }

    /// Return a failure in the group's ABI shape: wrapped in a `%parsed` when the
    /// group returns records by value, a bare KValue otherwise.
    fn emit_ret_failure(&self, f: &mut FnEmit, name: &str, arity: usize, failure: &str) {
        let failure = release_cells(f, failure);
        if self.ret_ty(name, arity) == "%parsed" {
            self.emit_parsed_from_failure(f, &failure);
        } else {
            f.line(&format!("ret %KValue {failure}"));
        }
    }

    /// Return a KValue in the current function's ABI shape. A `%parsed`-returning
    /// function only reaches here with a failure (its record tails are built
    /// directly), so the failure's two words become the `%parsed`.
    fn emit_ret(&self, f: &mut FnEmit, value: &str) {
        let value = release_cells(f, value);
        if f.ret_ty == "%parsed" {
            self.emit_parsed_from_failure(f, &value);
        } else {
            f.line(&format!("ret %KValue {value}"));
        }
    }

    /// A `%parsed` and a `%KValue` share a `{i64,i64}` layout. On the failure
    /// path the two are the same value: the failure's tag/payload become the
    /// `%parsed` words, so the discriminator (low word ∈ {4,5}) stays intact.
    fn emit_parsed_from_failure(&self, f: &mut FnEmit, failure: &str) {
        let w0 = f.tmp();
        f.line(&format!("{w0} = extractvalue %KValue {failure}, 0"));
        let w1 = f.tmp();
        f.line(&format!("{w1} = extractvalue %KValue {failure}, 1"));
        let a = f.tmp();
        f.line(&format!("{a} = insertvalue %parsed undef, i64 {w0}, 0"));
        let p = f.tmp();
        f.line(&format!("{p} = insertvalue %parsed {a}, i64 {w1}, 1"));
        f.line(&format!("ret %parsed {p}"));
    }

    /// A direct construction of the register-returnable type a callee slot
    /// carries may cross the boundary packed; anything else in such a slot is
    /// already %parsed (a returnable call's result) by the analysis.
    fn packed_arg_fields<'e>(
        &self,
        callee: &str,
        arity: usize,
        i: usize,
        arg: &'e Expr,
    ) -> Option<&'e [Expr]> {
        let ty = self.escape.carries_ty(callee, arity, i)?;
        if let Expr::App { head, args, piped: false, .. } = arg {
            if matches!(head.as_ref(), Expr::Ident(n, _) if n == ty)
                && Some(&args.len()) == self.escape.field_count.get(ty).as_ref().map(|v| *v)
            {
                return Some(args);
            }
        }
        None
    }

    /// Pack a register-returnable construction into its by-value form for an
    /// argument position: same two words the tail form uses, yielded as a
    /// temp instead of returned.
    fn emit_packed_arg(
        &mut self,
        f: &mut FnEmit,
        args: &[Expr],
        ty: &str,
    ) -> Result<String, String> {
        let pos = self.emit_expr(f, &args[0])?;
        self.bail_on_failure(f, &pos);
        let value = self.emit_expr(f, &args[1])?;
        self.bail_on_failure(f, &value);
        let pos_payload = f.tmp();
        f.line(&format!("{pos_payload} = extractvalue %KValue {pos}, 1"));
        let shifted = f.tmp();
        f.line(&format!("{shifted} = shl i64 {pos_payload}, 8"));
        let vtag = f.tmp();
        f.line(&format!("{vtag} = extractvalue %KValue {value}, 0"));
        let w0 = f.tmp();
        f.line(&format!("{w0} = or i64 {shifted}, {vtag}"));
        let w1 = f.tmp();
        f.line(&format!("{w1} = extractvalue %KValue {value}, 1"));
        let a = f.tmp();
        f.line(&format!("{a} = insertvalue %parsed undef, i64 {w0}, 0"));
        let p = f.tmp();
        f.line(&format!("{p} = insertvalue %parsed {a}, i64 {w1}, 1"));
        let pid = self.type_ids[ty];
        f.record_parsed(&p, ty, pid);
        Ok(p)
    }

    /// Build a register-returnable record in tail position as a by-value
    /// `%parsed`. The two words hold `(value.tag | pos << 8, value.payload)` — a
    /// non-failure value's tag never collides with the failure tags 4/5, so the
    /// low byte of word 0 still tells success from failure. A failing field
    /// propagates exactly as `k_rec` would have — which means BOTH fields are
    /// evaluated and two failures merge, the way two failing operands of an
    /// operator do. Bailing on the first one skipped the second field
    /// entirely and handed back one reason where the oracle carried two.
    fn emit_parsed_construction(&mut self, f: &mut FnEmit, args: &[Expr]) -> Result<(), String> {
        let pos = self.emit_expr(f, &args[0])?;
        let value = self.emit_expr(f, &args[1])?;
        self.bail_on_pair_failure(f, &pos, &value);
        let pos_payload = f.tmp();
        f.line(&format!("{pos_payload} = extractvalue %KValue {pos}, 1"));
        let shifted = f.tmp();
        f.line(&format!("{shifted} = shl i64 {pos_payload}, 8"));
        let vtag = f.tmp();
        f.line(&format!("{vtag} = extractvalue %KValue {value}, 0"));
        let w0 = f.tmp();
        f.line(&format!("{w0} = or i64 {shifted}, {vtag}"));
        let w1 = f.tmp();
        f.line(&format!("{w1} = extractvalue %KValue {value}, 1"));
        let a = f.tmp();
        f.line(&format!("{a} = insertvalue %parsed undef, i64 {w0}, 0"));
        let p = f.tmp();
        f.line(&format!("{p} = insertvalue %parsed {a}, i64 {w1}, 1"));
        f.line(&format!("ret %parsed {p}"));
        Ok(())
    }

    /// If either field failed, return the merge of them in the current ABI
    /// shape; otherwise fall through with both known good.
    fn bail_on_pair_failure(&self, f: &mut FnEmit, left: &str, right: &str) {
        let ok_left = inline_not_failure(f, left);
        let ok_right = inline_not_failure(f, right);
        let both = f.tmp();
        f.line(&format!("{both} = and i1 {ok_left}, {ok_right}"));
        let cont = f.label();
        let bail = f.label();
        f.line(&format!("br i1 {both}, label %{cont}, label %{bail}"));
        f.start_block(&bail);
        let merged = f.tmp();
        f.line(&format!(
            "{merged} = call %KValue @k_pair_failure(%KValue {left}, %KValue {right})"
        ));
        self.emit_ret(f, &merged);
        f.start_block(&cont);
    }

    /// If `value` is a failure, return it in the current ABI shape; otherwise
    /// fall through with `value` known good.
    fn bail_on_failure(&self, f: &mut FnEmit, value: &str) {
        let ok = inline_not_failure(f, value);
        let cont = f.label();
        let bail = f.label();
        f.line(&format!("br i1 {ok}, label %{cont}, label %{bail}"));
        f.start_block(&bail);
        self.emit_ret(f, value);
        f.start_block(&cont);
    }

    fn emit_pattern_known(
        &mut self,
        f: &mut FnEmit,
        value: &str,
        pattern: &Pattern,
        fail: &str,
        known: Set,
    ) -> Result<(), String> {
        let check = |backend: &mut Backend, f: &mut FnEmit, call: String| {
            let c = f.tmp();
            f.line(&format!("{c} = {call}"));
            let b = f.tmp();
            f.line(&format!("{b} = icmp ne i64 {c}, 0"));
            let ok = f.label();
            f.line(&format!("br i1 {b}, label %{ok}, label %{fail}"));
            f.start_block(&ok);
            let _ = backend;
        };
        let branch_i1 = |f: &mut FnEmit, cond: String| {
            let ok = f.label();
            f.line(&format!("br i1 {cond}, label %{ok}, label %{fail}"));
            f.start_block(&ok);
        };
        let tag_is = |f: &mut FnEmit, value: &str, tag: i64| {
            let t = inline_tag(f, value);
            let b = f.tmp();
            f.line(&format!("{b} = icmp eq i64 {t}, {tag}"));
            b
        };
        match pattern {
            Pattern::IntLit(n, _) => {
                let is_int = tag_is(f, value, 0);
                let payload = inline_payload(f, value);
                let eq = f.tmp();
                f.line(&format!("{eq} = icmp eq i64 {payload}, {n}"));
                let both = f.tmp();
                f.line(&format!("{both} = and i1 {is_int}, {eq}"));
                branch_i1(f, both);
            }
            Pattern::StrLit(s, _) => {
                let (name, len) = self.intern(s);
                check(
                    self,
                    f,
                    format!("call i64 @k_check_str(%KValue {value}, ptr @{name}, i64 {len})"),
                );
            }
            Pattern::Nullary(name, _) => {
                let tag = match name.as_str() {
                    "true" => K_TRUE,
                    "false" => K_FALSE,
                    _ => K_NONE,
                };
                let b = tag_is(f, value, tag);
                branch_i1(f, b);
            }
            Pattern::Wildcard(_) => {
                if known & FAIL != 0 {
                    let ok = inline_not_failure(f, value);
                    branch_i1(f, ok);
                }
            }
            Pattern::Var(name, _) => {
                if known & FAIL != 0 {
                    let ok = inline_not_failure(f, value);
                    branch_i1(f, ok);
                }
                f.bind(name, value);
                f.record(value, known & !FAIL);
            }
            Pattern::Annotated { name, ty, .. } => {
                if ty.ends_with("[]") {
                    check(self, f, format!("call i64 @k_check_tag(%KValue {value}, i64 9)"));
                    f.bind(name, value);
                    return Ok(());
                }
                if ty.contains('[') {
                    check(self, f, format!("call i64 @k_check_tag(%KValue {value}, i64 10)"));
                    f.bind(name, value);
                    return Ok(());
                }
                // One arm for every annotation, typeset or not, because two
                // arms for one pattern kind is how the guard below went
                // missing: the typeset arm returned before reaching it, so a
                // typeset naming err let a package rescue its own failure on
                // native where the oracle passed it through. `wasm_backend`
                // has always had the one arm and has always been right.
                if self.admits_err(ty) {
                    let arm = self.arm_hako(f);
                    check(self, f, format!("call i64 @k_not_own_err(%KValue {value}, ptr @{arm})"));
                }
                // a typeset matches when any member does: OR the members'
                // checks and branch once. A plain annotation is the same
                // shape with one member.
                let members = match self.typesets.get(ty.as_str()) {
                    Some(members) => members.clone(),
                    None => vec![ty.to_string()],
                };
                let mut acc: Option<String> = None;
                for member in &members {
                    let call = self.type_check_call(value, member)?;
                    let c = f.tmp();
                    f.line(&format!("{c} = {call}"));
                    acc = Some(match acc {
                        None => c,
                        Some(prev) => {
                            let o = f.tmp();
                            f.line(&format!("{o} = or i64 {prev}, {c}"));
                            o
                        }
                    });
                }
                let combined = acc.expect("an annotation names at least one type");
                let b = f.tmp();
                f.line(&format!("{b} = icmp ne i64 {combined}, 0"));
                branch_i1(f, b);
                f.bind(name, value);
            }
            Pattern::Ctor { ty, fields, whole } => {
                if ty == "err" {
                    let arm = self.arm_hako(f);
                    check(self, f, format!("call i64 @k_not_own_err(%KValue {value}, ptr @{arm})"));
                    check(self, f, format!("call i64 @k_check_tag(%KValue {value}, i64 {K_ERR})"));
                    let inner = f.tmp();
                    f.line(&format!("{inner} = call %KValue @k_err_inner(%KValue {value})"));
                    self.emit_pattern(f, &inner, &fields[0], fail)?;
                    if let Some(named) = whole {
                        f.bind(&named.0, value);
                    }
                    return Ok(());
                }
                let id = *self
                    .type_ids
                    .get(ty.as_str())
                    .ok_or_else(|| format!("native backend: unknown type `{ty}`"))?;
                check(
                    self,
                    f,
                    format!(
                        "call i64 @k_check_rec_fast(%KValue {value}, i64 {id}, i64 {})",
                        fields.len()
                    ),
                );
                for (i, field) in fields.iter().enumerate() {
                    let fv = f.tmp();
                    f.line(&format!("{fv} = call %KValue @k_field_fast(%KValue {value}, i64 {i})"));
                    self.emit_pattern(f, &fv, field, fail)?;
                }
                // the as-pattern's name takes the value that matched, so an
                // arm answering it hands back what it was given
                if let Some(named) = whole {
                    f.bind(&named.0, value);
                }
            }
            Pattern::Keyed { .. } => {
                return Err("native backend: keyed patterns are slice 2".to_string())
            }
        }
        Ok(())
    }

    /// Constructor enforcement for multi-member field typesets: a field value
    /// matching no member is a defect (failures skip the check and propagate
    /// through `k_rec`).
    fn emit_typeset_checks(
        &mut self,
        f: &mut FnEmit,
        name: &str,
        emitted: &[String],
    ) -> Result<(), String> {
        let Some(decl) = self.program.types.iter().find(|t| t.name == name) else {
            return Ok(());
        };
        let fields = decl.fields.clone();
        for ((field, tys, _), value) in fields.iter().zip(emitted) {
            if tys.len() < 2 {
                continue;
            }
            let mut matched: Option<String> = None;
            for member in tys {
                let call = self.member_check_call(value, member)?;
                let c = f.tmp();
                f.line(&format!("{c} = {call}"));
                let b = f.tmp();
                f.line(&format!("{b} = icmp ne i64 {c}, 0"));
                matched = Some(match matched {
                    None => b,
                    Some(prev) => {
                        let t = f.tmp();
                        f.line(&format!("{t} = or i1 {prev}, {b}"));
                        t
                    }
                });
            }
            let matched = matched.expect("a typeset has members");
            let not_fail = inline_not_failure(f, value);
            let not_matched = f.tmp();
            f.line(&format!("{not_matched} = xor i1 {matched}, true"));
            let bad = f.tmp();
            f.line(&format!("{bad} = and i1 {not_matched}, {not_fail}"));
            let die = f.label();
            let ok = f.label();
            f.line(&format!("br i1 {bad}, label %{die}, label %{ok}"));
            f.start_block(&die);
            let msg = format!("field `{field}` of `{name}` takes {}\0", tys.join(" "));
            let (m, _) = self.intern(&msg);
            f.line(&format!("call void @k_die(ptr @{m})"));
            f.line("unreachable");
            f.start_block(&ok);
        }
        Ok(())
    }

    /// A field typeset's member resolves exactly as a parameter annotation
    /// does; keeping one resolver is what stops the two drifting apart.
    fn member_check_call(&self, value: &str, member: &str) -> Result<String, String> {
        self.type_check_call(value, member)
    }

    fn field_count(&self, ty: &str) -> Result<usize, String> {
        self.program
            .types
            .iter()
            .find(|t| t.name == ty)
            .map(|t| t.fields.len())
            .ok_or_else(|| format!("native backend: unknown type `{ty}`"))
    }

    fn emit_fn_body(&mut self, f: &mut FnEmit, body: &[Stmt]) -> Result<(), String> {
        let last = body.len() - 1;
        for (i, stmt) in body.iter().enumerate() {
            match stmt {
                Stmt::Set { .. } => unreachable!("`set` parses only inside `build`"),
                Stmt::Bind { pattern: Pattern::Var(name, _), expr }
                    if self.demand.is_lazy_bind(&f.group.clone(), f.arity, i)
                        && self.thunkable(f, expr) =>
                {
                    let t = self.emit_cell(f, expr)?;
                    let in_beat = self.beat.ids.contains_key(&(f.group.clone(), f.arity));
                    if !in_beat && self.demand.is_releasable(&f.group, f.arity, i) {
                        f.lazy_cells.push(t.clone());
                    }
                    f.bind(name, &t);
                }
                Stmt::Bind { pattern, expr } => {
                    self.emit_bind(f, pattern, expr)?;
                }
                Stmt::Expr(expr) => {
                    if i == last {
                        self.emit_tail(f, expr)?;
                    } else {
                        let _ = self.emit_expr(f, expr)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// One strict binding: evaluate, then bind the pattern's names.
    fn emit_bind(&mut self, f: &mut FnEmit, pattern: &Pattern, expr: &Expr) -> Result<(), String> {
        {
            {
                {
                    let value = self.emit_expr(f, expr)?;
                    let value = match pattern {
                        Pattern::Var(..) => value,
                        _ => self.maybe_force(f, value),
                    };
                    match pattern {
                        Pattern::Var(name, _) => f.bind(name, &value),
                        Pattern::Ctor { ty, fields, .. } => {
                            let id = *self
                                .type_ids
                                .get(ty.as_str())
                                .ok_or_else(|| format!("native backend: unknown type `{ty}`"))?;
                            let c = f.tmp();
                            f.line(&format!(
                                "{c} = call i64 @k_check_rec_fast(%KValue {value}, i64 {id}, i64 {})",
                                fields.len()
                            ));
                            let b = f.tmp();
                            f.line(&format!("{b} = icmp ne i64 {c}, 0"));
                            let ok = f.label();
                            let bad = f.label();
                            f.line(&format!("br i1 {b}, label %{ok}, label %{bad}"));
                            f.start_block(&bad);
                            // The value goes to the runtime rather than a baked
                            // sentence: the reader wants to see what they bound,
                            // and only the runtime knows it. Its keyed sibling
                            // `k_keyed_check` has always worked this way.
                            let (m, _) = self.intern(&format!("{ty}\0"));
                            f.line(&format!(
                                "call void @k_die_destructure(%KValue {value}, ptr @{m})"
                            ));
                            f.line("unreachable");
                            f.start_block(&ok);
                            for (i, field) in fields.iter().enumerate() {
                                if let Pattern::Var(name, _) = field {
                                    let fv = f.tmp();
                                    f.line(&format!(
                                        "{fv} = call %KValue @k_field_fast(%KValue {value}, i64 {i})"
                                    ));
                                    f.bind(name, &fv);
                                }
                            }
                        }
                        Pattern::Keyed { entries, .. } => {
                            let checked = f.tmp();
                            f.line(&format!(
                                "{checked} = call %KValue @k_keyed_check(%KValue {value}, i64 {})",
                                entries.len()
                            ));
                            for entry in entries {
                                let (name, _) = self.intern(&format!("{}\0", entry.field));
                                let fv = f.tmp();
                                f.line(&format!(
                                    "{fv} = call %KValue @k_keyed_field(%KValue {checked}, ptr @{name})"
                                ));
                                f.bind(&entry.bind_name, &fv);
                            }
                        }
                        _ => {
                            return Err(
                                "native backend: this binding pattern is not supported".to_string()
                            )
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// A partial application, as the lambda it is equivalent to: `&add 2`
    /// becomes `(x -> add 2 x)`. The remaining arity has to be unambiguous —
    /// with both `add a b` and `add a b c` declared, `&add 2` could be waiting
    /// for one argument or two, and the interpreter defers that choice until
    /// the arguments arrive. Nothing here can defer, so an ambiguous partial
    /// is refused out loud rather than guessed at, which is the escape the
    /// differential law allows an engine that covers less.
    fn partial_lambda(&self, name: &str, supplied: &[Expr], span: Span) -> Result<Expr, String> {
        let arities: Vec<usize> = {
            let mut seen: Vec<usize> = self
                .program
                .fns
                .iter()
                .filter(|d| d.name == name && d.params.len() >= supplied.len())
                .map(|d| d.params.len())
                .collect();
            seen.sort_unstable();
            seen.dedup();
            seen
        };
        // Two different things emptied that list, and one message spoke for
        // both. A name no declaration answers to is not a mistake here — the
        // front door refuses an unknown name before this runs — so what is
        // left is a name bound to a VALUE: a parameter holding a function, a
        // local, a builtin, a record's constructor. The interpreter takes a
        // partial over a value and settles its arity when the arguments
        // arrive; `tests/partial.rs` specifies that. This backend writes a
        // closure whose parameter count is fixed where the closure is, so it
        // cannot. Saying "no `f` takes more" put that on the program.
        if !self.program.fns.iter().any(|d| d.name == name) {
            return Err(format!(
                "native backend: `{name}` is a value here, and a partial over a value settles \
                 its arity when its arguments arrive — this backend fixes it where the closure \
                 is written"
            ));
        }
        // Currying past every arm is the one real error: `&` supplies without
        // running, so supplying an arm's last argument is a partial like any
        // other — the value waits to be called rather than being a call. What
        // nothing can finish is more arguments than any arm accepts.
        if arities.is_empty() {
            return Err(format!(
                "native backend: `&{name}` holds {} argument(s), and no `{name}` takes more",
                supplied.len()
            ));
        }
        // A partial that escapes as a value has to become a closure, and a
        // closure fixes its parameter count when it is written. That is fine
        // when one arm can still finish it; with several, the count is decided
        // by the arguments that arrive, which a closure cannot wait for. The
        // shape that needs it is a partial bound to a name and later applied
        // with more arguments than the shortest arm wants.
        let Some(&arity) = arities.first() else { unreachable!("checked non-empty") };
        if arities.len() > 1 {
            return Err(format!(
                "native backend: `&{name}` escapes as a value while {} arms could still finish \
                 it, and lowering it needs a partial the runtime does not have yet",
                arities.len()
            ));
        }
        let waiting = arity - supplied.len();
        let params: Vec<(String, Span)> =
            (0..waiting).map(|i| (format!("k#partial{i}"), span)).collect();
        let mut args = supplied.to_vec();
        args.extend(params.iter().map(|(n, s)| Expr::Ident(Name::new(&n.clone()), *s)));
        let head = Expr::Ident(Name::new(name), span);
        let body = Expr::App { head: Box::new(head), args, piped: false, span };
        Ok(Expr::Lambda { params, body: Box::new(body), span })
    }

    fn emit_expr(&mut self, f: &mut FnEmit, expr: &Expr) -> Result<String, String> {
        match expr {
            // the interpreter is the oracle for `&`; the backends reject it out
            // loud rather than lowering something that would diverge
            Expr::Partial(name, span) => {
                let lambda = self.partial_lambda(name, &[], *span)?;
                self.emit_expr(f, &lambda)
            }
            Expr::Upcast { expr: inner, ty, .. } => {
                let v = self.emit_expr(f, inner)?;
                let v = self.maybe_force(f, v);
                let want = self.sub_want(ty)?;
                let (tyn, _) = self.intern(&format!("{ty}\0"));
                let t = f.tmp();
                f.line(&format!(
                    "{t} = call %KValue @k_upcast(%KValue {v}, i64 {want}, ptr @{tyn})"
                ));
                f.record(&t, crate::infer::TOP);
                Ok(t)
            }
            Expr::Block(stmts, _) | Expr::Build(stmts, _) => {
                let mut value = "{ i64 4, i64 0 }".to_string();
                let last = stmts.len().saturating_sub(1);
                for (i, stmt) in stmts.iter().enumerate() {
                    match stmt {
                        Stmt::Bind { pattern, expr } => self.emit_bind(f, pattern, expr)?,
                        Stmt::Set { target, field, value, span } => {
                            let new = self.emit_expr(f, value)?;
                            let new = self.maybe_force(f, new);
                            let ident = Expr::Ident(Name::new(&target.clone()), *span);
                            let tv = self.emit_expr(f, &ident)?;
                            let tv = self.maybe_force(f, tv);
                            let (label, _) = self.intern(&format!("{field}\0"));
                            f.line(&format!(
                                "call %KValue @k_set_field(%KValue {tv}, ptr @{label}, %KValue {new})"
                            ));
                        }
                        Stmt::Expr(e) => {
                            let v = self.emit_expr(f, e)?;
                            if i == last {
                                value = v;
                            }
                        }
                    }
                }
                Ok(value)
            }
            Expr::Guard { .. } => {
                Err("native backend: a return guard sits only in tail position".to_string())
            }
            // A literal wider than the payload used to be truncated into it,
            // so `1 * 18446744073709551616` answered 0 — a wrong answer that
            // looked right, which is the one thing this build's ceiling is
            // supposed to refuse rather than produce.
            Expr::Int(n, _) => match i64::try_from(n) {
                Ok(fits) => Ok(format!("{{ i64 0, i64 {fits} }}")),
                Err(_) => Err(format!(
                    "native backend: the literal {n} does not fit this build's 64-bit \
                     int (spec int is arbitrary precision)"
                )),
            },
            Expr::Float(x, _) => {
                let t = f.tmp();
                f.line(&format!("{t} = call %KValue @k_float(double 0x{:016X})", x.to_bits()));
                Ok(t)
            }
            Expr::Str(parts, span) => {
                let joins_builder = self.builder_joins.contains(&(
                    f.file.clone(),
                    span.line as usize,
                    span.col as usize,
                ));
                let mut acc: Option<Vec<String>> = None;
                let mut fails: Set = 0;
                for part in parts {
                    let piece = match part {
                        TemplatePart::Lit(s) => self.str_const(f, s),
                        TemplatePart::Interp(inner) => {
                            let value = self.emit_expr(f, inner)?;
                            let value = self.maybe_force(f, value);
                            // only an err propagates out of interpolation; a none
                            // renders `<none>` via k_render, so it is not a fail
                            fails |= f.set_of(&value) & ERR;
                            // a set carrying REC may hit a user to_string arm:
                            // route through the ambient group. Primitive-only
                            // sets keep the direct call — coherence proves no
                            // arm can exist for them (design/render-plan.md).
                            let group = "render/to_string";
                            let dispatchable = f.set_of(&value) & (REC | NONE | DESC) != 0
                                && self.program.fns.iter().any(|d| d.name == group);
                            let t = f.tmp();
                            let value = self.as_value(f, &value);
                            match dispatchable {
                                true => {
                                    f.line(&format!(
                                        "{t} = call tailcc %KValue @{}(%KValue {value})",
                                        dsym(group, 1)
                                    ));
                                    fails |= ERR;
                                }
                                false => f.line(&format!(
                                    "{t} = call %KValue @k_render(%KValue {value}, i64 0)"
                                )),
                            }
                            t
                        }
                    };
                    match acc {
                        None => acc = Some(vec![piece]),
                        Some(ref mut pieces) => pieces.push(piece),
                    }
                }
                let out = match acc {
                    Some(pieces) if pieces.len() == 1 => {
                        pieces.into_iter().next().expect("one piece")
                    }
                    Some(pieces) if pieces.len() <= 16 => {
                        let arr = f.tmp();
                        f.line(&format!("{arr} = alloca [{} x %KValue]", pieces.len()));
                        for (i, p) in pieces.iter().enumerate() {
                            let slot = f.tmp();
                            f.line(&format!(
                                "{slot} = getelementptr [{} x %KValue], ptr {arr}, i64 0, i64 {i}",
                                pieces.len()
                            ));
                            f.line(&format!("store %KValue {p}, ptr {slot}"));
                        }
                        let t = f.tmp();
                        let sym = match joins_builder {
                            true => "k_concat_arr_mut",
                            false => "k_concat_arr",
                        };
                        f.line(&format!(
                            "{t} = call %KValue @{sym}(i64 {}, ptr {arr})",
                            pieces.len()
                        ));
                        t
                    }
                    Some(pieces) => {
                        let mut it = pieces.into_iter();
                        let mut prev = it.next().expect("non-empty");
                        for piece in it {
                            let t = f.tmp();
                            f.line(&format!(
                                "{t} = call %KValue @k_concat(%KValue {prev}, %KValue {piece})"
                            ));
                            prev = t;
                        }
                        prev
                    }
                    None => self.str_const(f, ""),
                };
                f.record(&out, STR | fails);
                Ok(out)
            }
            Expr::Ident(name, _) => {
                if let Some(temp) = f.lookup(name) {
                    return Ok(temp);
                }
                // A record type with no fields IS a value: `type unit` names one
                // thing and naming it builds it. A subtype and a typeset also
                // carry no fields, and neither is that — a subtype's name takes
                // one argument, and a typeset never constructs at all. Both
                // reached this arm and were emitted as nullary records, so
                // `print "{age}"` for `type age int` printed `<mod>/age` where
                // the oracle prints `<fn>` and the page refuses the name. It
                // falls through to the bare-value refusal below now, which is
                // what native already said for a record type that HAS fields.
                let nullary_record = |t: &crate::ast::TypeDecl| {
                    t.name == *name
                        && t.fields.is_empty()
                        && t.parent.is_none()
                        && t.members.is_empty()
                };
                if self.program.types.iter().any(nullary_record) {
                    let id = self.type_ids[name.as_str()];
                    let arr = f.tmp();
                    f.line(&format!("{arr} = alloca [1 x %KValue]"));
                    let t = f.tmp();
                    f.line(&format!("{t} = call %KValue @k_rec(i64 {id}, i64 0, ptr {arr})"));
                    f.record(&t, REC);
                    return Ok(t);
                }
                if self.program.fns.iter().any(|d| d.name == *name && d.params.is_empty()) {
                    let callee_ret = self.ret_ty(name, 0);
                    let t = f.tmp();
                    f.line(&format!("{t} = call tailcc {callee_ret} @{}()", dsym(name, 0)));
                    // A group that returns its record in registers hands back a
                    // %parsed, and everything downstream of a constant reads a
                    // %KValue: the failure guard, the return, the field read.
                    // Box it here, once, rather than teaching each consumer the
                    // other shape — the saving the register return buys is on
                    // the callee's side, and a constant is evaluated once.
                    let t = match self.escape.returns_ty(name, 0) {
                        Some(ty) if callee_ret == "%parsed" => {
                            f.record_parsed(&t, ty, self.type_ids[ty]);
                            self.as_value(f, &t)
                        }
                        _ => t,
                    };
                    f.record(&t, self.group_return_set(name, 0));
                    return Ok(t);
                }
                let arities: Vec<usize> = {
                    let mut seen = Vec::new();
                    for d in self.program.fns.iter().filter(|d| d.name == *name) {
                        if !seen.contains(&d.params.len()) {
                            seen.push(d.params.len());
                        }
                    }
                    seen
                };
                if arities.len() == 1
                    && (1..=4).contains(&arities[0])
                    && self.simple_fn_value(name, arities[0])
                {
                    let arity = arities[0];
                    self.fn_value_wrappers.push((name.to_string(), arity));
                    let t = f.tmp();
                    f.line(&format!("{t} = call %KValue @k_fnref(ptr @{})", rsym(name, arity)));
                    return Ok(t);
                }
                if !arities.is_empty() {
                    return Err(format!(
                        "native backend: `{name}` cannot be used as a function value \
                         (only 1-4 argument functions over plain values are supported)"
                    ));
                }
                let bare = name.strip_prefix("builtin_").unwrap_or(name.as_str());
                // A builtin is a function too, and `apply length "ab"` hands it
                // over the same way a declared group is handed over. The
                // interpreter calls it; the wrapper below is what lets a
                // compiled `k_callN` reach the same C entry.
                if let Some(arity) = arity_of_emitted(bare) {
                    if (1..=4).contains(&arity) {
                        self.builtin_value_wrappers.push((bare.to_string(), arity));
                        let t = f.tmp();
                        let sym = rsym(&format!("builtin.{bare}"), arity);
                        f.line(&format!("{t} = call %KValue @k_fnref(ptr @{sym})"));
                        return Ok(t);
                    }
                }
                // `print` is not one of them: its argument reaches a user's
                // `render/to_string` arm, and a call site picks that path from
                // the argument's set. Handed over, there is no set to read, so
                // its wrapper carries the choice into the run.
                if bare == "print" {
                    self.print_value_wrapper = true;
                    let t = f.tmp();
                    let sym = rsym("builtin.print", 1);
                    f.line(&format!("{t} = call %KValue @k_fnref(ptr @{sym})"));
                    return Ok(t);
                }
                match bare {
                    "true" => Ok("{ i64 2, i64 0 }".to_string()),
                    "false" => Ok("{ i64 3, i64 0 }".to_string()),
                    "none" => Ok("{ i64 4, i64 0 }".to_string()),
                    "args" => {
                        let t = f.tmp();
                        f.line(&format!("{t} = call %KValue @k_desc_args()"));
                        f.record(&t, DESC);
                        Ok(t)
                    }
                    "stdin" => {
                        let t = f.tmp();
                        f.line(&format!("{t} = call %KValue @k_desc_stdin()"));
                        f.record(&t, DESC);
                        Ok(t)
                    }
                    "now" => {
                        let t = f.tmp();
                        f.line(&format!("{t} = call %KValue @k_desc_now()"));
                        f.record(&t, DESC);
                        Ok(t)
                    }
                    _ => Err(format!(
                        "native backend: `{name}` as a bare value is not yet supported"
                    )),
                }
            }
            Expr::App { head, args, piped, span } => {
                // `&f a` supplies a and waits; the arity it waits for is the
                // one the gavel names — supplied plus holes picks the group
                // `(&roll 4) 5` is one application seen whole: the partial and
                // the arguments finishing it are both here, so it lowers to the
                // call it means. Dispatch then happens on the total count, which
                // is what the oracle does and what makes `(&roll 4) 5 6` reach
                // the three-argument arm rather than any arm chosen at the `&`.
                if let Expr::App { head: inner, args: held, .. } = head.as_ref() {
                    if let Expr::Partial(name, nspan) = inner.as_ref() {
                        let mut all = held.clone();
                        all.extend(args.iter().cloned());
                        let callee = Expr::Ident(name.clone(), *nspan);
                        return self.emit_call_full(f, &Box::new(callee), &all, *piped, *span);
                    }
                }
                if let Expr::Partial(name, _) = head.as_ref() {
                    let lambda = self.partial_lambda(name, args, *span)?;
                    return self.emit_expr(f, &lambda);
                }
                self.emit_call_full(f, head, args, *piped, *span)
            }
            Expr::Field { base, name, .. } => {
                let b = self.emit_expr(f, base)?;
                let (label, _) = self.intern(&format!("{name}\0"));
                let t = f.tmp();
                f.line(&format!("{t} = call %KValue @k_b_field(%KValue {b}, ptr @{label})"));
                f.record(&t, TOP);
                Ok(t)
            }
            Expr::Index { base, index, strict, span } => {
                let container = self.emit_expr(f, base)?;
                let container = self.maybe_force(f, container);
                let key = self.emit_expr(f, index)?;
                let key = self.maybe_force(f, key);
                Ok(self.emit_at(f, &container, &key, *strict, *span))
            }
            Expr::Seq(lhs, rhs, span) => {
                let a = self.emit_expr(f, lhs)?;
                let a = self.maybe_force(f, a);
                // GAVEL 15: the wall defers its right side, so the right
                // operand is a cell the executor forces once the left has
                // run. A name mentioned there is stored rather than demanded,
                // which is what lets a description name itself.
                if !self.thunkable(f, rhs) {
                    let _ = span;
                    return Err("native backend: `>>` defers its right side, and this one \
                                reads more than eight names — bind some of them before the \
                                wall"
                        .to_string());
                }
                let b = self.emit_cell(f, rhs)?;
                let t = f.tmp();
                f.line(&format!("{t} = call %KValue @k_seq(%KValue {a}, %KValue {b})"));
                f.record(&t, DESC | (f.set_of(&a) & FAIL));
                Ok(t)
            }
            Expr::Join { lhs, rhs, .. } => {
                let a = self.emit_expr(f, lhs)?;
                let a = self.maybe_force(f, a);
                let b = self.emit_expr(f, rhs)?;
                let b = self.maybe_force(f, b);
                let t = f.tmp();
                f.line(&format!("{t} = call %KValue @k_desc_join(%KValue {a}, %KValue {b})"));
                f.record(&t, (f.set_of(&a) & FAIL) | (f.set_of(&b) & FAIL) | DESC | ERR);
                Ok(t)
            }
            Expr::BinOp { op, lhs, rhs, span } => {
                let a = self.emit_expr(f, lhs)?;
                let a = self.maybe_force(f, a);
                let b = self.emit_expr(f, rhs)?;
                let b = self.maybe_force(f, b);
                self.emit_binop(f, op, &a, &b, *span)
            }
            Expr::Lambda { params, body, .. } => {
                // No lower bound: `&add 1 2` supplies an arm's last argument
                // and the value it leaves waits to be called, which is a
                // closure of no parameters.
                if params.len() > 4 {
                    return Err("native backend: a lambda takes at most 4 parameters".to_string());
                }
                let param_names: Vec<String> = params.iter().map(|(n, _)| n.clone()).collect();
                let mut idents = Vec::new();
                collect_idents(body, &mut idents);
                let mut captures: Vec<String> = Vec::new();
                for name in idents {
                    if f.lookup(&name).is_some()
                        && !captures.contains(&name)
                        && !param_names.contains(&name)
                    {
                        captures.push(name);
                    }
                }
                let lifted = format!("klam{}", self.lift_counter);
                self.lift_counter += 1;
                self.emit_lifted(&lifted, &param_names, &captures, body, f)?;
                let n = captures.len().max(1);
                let arr = f.tmp();
                f.line(&format!("{arr} = alloca [{n} x %KValue]"));
                for (i, cap) in captures.iter().enumerate() {
                    let temp = f.lookup(cap).expect("capture is bound");
                    let temp = self.as_value(f, &temp);
                    let slot = f.tmp();
                    f.line(&format!(
                        "{slot} = getelementptr [{n} x %KValue], ptr {arr}, i64 0, i64 {i}"
                    ));
                    f.line(&format!("store %KValue {temp}, ptr {slot}"));
                }
                let t = f.tmp();
                // the ccc wrapper, never the tailcc fn: C calls this pointer
                f.line(&format!(
                    "{t} = call %KValue @k_closure(ptr @w_{lifted}, i64 {}, i64 {}, ptr {arr})",
                    params.len(),
                    captures.len()
                ));
                Ok(t)
            }
            Expr::List(items, _) => {
                let mut emitted = Vec::new();
                for item in items {
                    let e = self.deferred_or_emitted(f, item)?;
                    emitted.push(self.as_value(f, &e));
                }
                let n = emitted.len().max(1);
                let arr = f.tmp();
                f.line(&format!("{arr} = alloca [{n} x %KValue]"));
                for (i, value) in emitted.iter().enumerate() {
                    let slot = f.tmp();
                    f.line(&format!(
                        "{slot} = getelementptr [{n} x %KValue], ptr {arr}, i64 0, i64 {i}"
                    ));
                    f.line(&format!("store %KValue {value}, ptr {slot}"));
                }
                let t = f.tmp();
                f.line(&format!(
                    "{t} = call %KValue @k_list_lit(i64 {}, ptr {arr})",
                    emitted.len()
                ));
                f.record(&t, LIST);
                Ok(t)
            }
            Expr::MapLit(pairs, _) => {
                let mut emitted = Vec::new();
                for (key, value) in pairs {
                    let k = self.emit_expr(f, key)?;
                    emitted.push(self.as_value(f, &k));
                    let v = self.emit_expr(f, value)?;
                    emitted.push(self.as_value(f, &v));
                }
                let n = emitted.len().max(1);
                let arr = f.tmp();
                f.line(&format!("{arr} = alloca [{n} x %KValue]"));
                for (i, value) in emitted.iter().enumerate() {
                    let slot = f.tmp();
                    f.line(&format!(
                        "{slot} = getelementptr [{n} x %KValue], ptr {arr}, i64 0, i64 {i}"
                    ));
                    f.line(&format!("store %KValue {value}, ptr {slot}"));
                }
                let t = f.tmp();
                f.line(&format!("{t} = call %KValue @k_map_lit(i64 {}, ptr {arr})", pairs.len()));
                f.record(&t, MAP);
                Ok(t)
            }
        }
    }

    /// Emit an expression in tail position: direct calls to kanso functions
    /// become guaranteed tail calls, and an if's branches stay tails.
    fn emit_tail(&mut self, f: &mut FnEmit, expr: &Expr) -> Result<(), String> {
        if let Expr::Guard { cond, early, rest, .. } = expr {
            let c = self.emit_expr(f, cond)?;
            let c = self.maybe_force(f, c);
            let ok = inline_not_failure(f, &c);
            let check = f.label();
            let bail = f.label();
            f.line(&format!("br i1 {ok}, label %{check}, label %{bail}"));
            f.start_block(&bail);
            self.emit_ret(f, &c);
            f.start_block(&check);
            let tv = f.tmp();
            f.line(&format!("{tv} = call i64 @k_truthy(%KValue {c})"));
            let tb = f.tmp();
            f.line(&format!("{tb} = icmp ne i64 {tv}, 0"));
            let early_label = f.label();
            let rest_label = f.label();
            f.line(&format!("br i1 {tb}, label %{early_label}, label %{rest_label}"));
            f.start_block(&early_label);
            self.emit_tail(f, early)?;
            f.start_block(&rest_label);
            self.emit_fn_body(f, rest)?;
            return Ok(());
        }
        if let Expr::App { head, args, piped: false, .. } = expr {
            if let Expr::Ident(name, _) = head.as_ref() {
                let bare = name.strip_prefix("builtin_").unwrap_or(name);
                if self.forwarders.contains_key(&(bare.to_string(), args.len()))
                    || self.forwarders.contains_key(&(name.to_string(), args.len()))
                {
                    let value = self.emit_expr(f, expr)?;
                    self.emit_ret(f, &value);
                    return Ok(());
                }
            }
        }
        if let Expr::App { head, args, piped, .. } = expr {
            if *piped && !args.is_empty() {
                // a tail pipe into a literal lambda is the bind, inlined:
                // guard the failure exactly as k_maybe_bind would, bind the
                // parameter, and the lambda body becomes this function's own
                // tail — a self-call there is a real musttail, so beats and
                // the carry apply through the ordinary machinery
                if let Expr::Lambda { params, body, .. } = head.as_ref() {
                    if params.len() == 1 && args.len() == 1 {
                        let value = self.emit_expr(f, &args[0])?;
                        // a description takes the executor's bind at runtime;
                        // anything else binds the parameter here and the
                        // lambda body becomes this function's own tail — the
                        // branch keeps both semantics exact with no reliance
                        // on inference
                        let tag = inline_tag(f, &value);
                        let is_desc = f.tmp();
                        f.line(&format!("{is_desc} = icmp eq i64 {tag}, 8"));
                        let desc_path = f.label();
                        let check = f.label();
                        f.line(&format!("br i1 {is_desc}, label %{desc_path}, label %{check}"));
                        f.start_block(&desc_path);
                        let t = f.tmp();
                        let closure = self.emit_expr(f, head)?;
                        f.line(&format!(
                            "{t} = call %KValue @k_maybe_bind(%KValue {value}, %KValue {closure})"
                        ));
                        f.record(&t, TOP);
                        self.emit_ret(f, &t);
                        f.start_block(&check);
                        let ok = inline_not_failure(f, &value);
                        let bail = f.label();
                        let cont = f.label();
                        f.line(&format!("br i1 {ok}, label %{cont}, label %{bail}"));
                        f.start_block(&bail);
                        self.emit_ret(f, &value);
                        f.start_block(&cont);
                        f.bind(&params[0].0, &value);
                        return self.emit_tail(f, body);
                    }
                }
                let value = self.emit_expr(f, expr)?;
                self.emit_ret(f, &value);
                return Ok(());
            }
            if let Expr::Ident(name, _) = &**head {
                if name == "if" && f.lookup(name).is_none() {
                    // A condition the demand analysis deferred arrives as a
                    // thunk here just as it does off the tail path; force
                    // before testing, and `maybe_force` still emits nothing
                    // where the set proves there is no thunk.
                    let cond = self.emit_expr(f, &args[0])?;
                    let cond = self.maybe_force(f, cond);
                    let ok = inline_not_failure(f, &cond);
                    let check = f.label();
                    let bail = f.label();
                    f.line(&format!("br i1 {ok}, label %{check}, label %{bail}"));
                    f.start_block(&bail);
                    self.emit_ret(f, &cond);
                    f.start_block(&check);
                    let tv = f.tmp();
                    f.line(&format!("{tv} = call i64 @k_truthy(%KValue {cond})"));
                    let tb = f.tmp();
                    f.line(&format!("{tb} = icmp ne i64 {tv}, 0"));
                    let then_label = f.label();
                    let else_label = f.label();
                    f.line(&format!("br i1 {tb}, label %{then_label}, label %{else_label}"));
                    f.start_block(&then_label);
                    self.emit_tail(f, &args[1])?;
                    f.start_block(&else_label);
                    self.emit_tail(f, &args[2])?;
                    return Ok(());
                }
                // A register-returnable record built in tail position becomes the
                // by-value %parsed result directly — no heap allocation.
                if let Some(&nfields) = self.escape.field_count.get(name.as_str()) {
                    if f.ret_ty == "%parsed" && args.len() == nfields {
                        return self.emit_parsed_construction(f, args);
                    }
                }
                // A demoted tail entry: emitted as a plain call so the
                // beat loop it enters gets its push/pop bracket. The caller
                // is acyclic, so the one retained frame is bounded. Lifted
                // lambdas never appear in the analysis's caller set, so ANY
                // tail entry into a beat-headed loop from outside its
                // cluster demotes — an unbracketed entry would let the
                // loop's rewinds unwind to an enclosing mark and free the
                // caller's own live data.
                let target = (name.to_string(), args.len());
                let outside_cluster = self.beat.ids.contains_key(&target)
                    && !self.beat.same_cluster(&target, &(f.group.clone(), f.arity));
                if outside_cluster
                    || self.beat.demoted.contains(&((f.group.clone(), f.arity), target))
                {
                    let value = self.emit_expr(f, expr)?;
                    self.emit_ret(f, &value);
                    return Ok(());
                }
                // the arity has to match a declaration: `d_{name}_{n}` for an
                // n nothing declares is a symbol the module never defines
                let is_program_fn = f.lookup(name).is_none()
                    && !self.type_ids.contains_key(name.as_str())
                    && name != "err"
                    && name != "print"
                    && self
                        .program
                        .fns
                        .iter()
                        .any(|d| d.name == *name && d.params.len() == args.len());
                if is_program_fn {
                    let n = args.len();
                    let mut emitted = Vec::new();
                    let mut packed: Vec<Option<String>> = Vec::new();
                    for (i, arg) in args.iter().enumerate() {
                        match self.packed_arg_fields(name, n, i, arg) {
                            Some(fields) => {
                                let fields = fields.to_vec();
                                let ty = self
                                    .escape
                                    .carries_ty(name, n, i)
                                    .expect("a packed argument fills a carried slot")
                                    .to_string();
                                let p = self.emit_packed_arg(f, &fields, &ty)?;
                                emitted.push(String::new());
                                packed.push(Some(p));
                            }
                            None => {
                                emitted.push(self.emit_expr(f, arg)?);
                                packed.push(None);
                            }
                        }
                    }
                    let callee_ret = self.ret_ty(name, n);
                    let same_ret = callee_ret == f.ret_ty;
                    if same_ret
                        && self
                            .beat
                            .same_cluster(&(name.to_string(), n), &(f.group.clone(), f.arity))
                    {
                        match self.beat.carried.get(&(name.to_string(), n)) {
                            Some(positions) => {
                                // evacuate the loop-varying arguments through
                                // the carry buffers, then rewind — before the
                                // ABI conversion below, so the call passes
                                // the evacuated values
                                f.line("call void @k_carry_reset()");
                                for &j in positions {
                                    let a = &emitted[j];
                                    // The accumulator crosses by identity: the
                                    // copy strips the room it was seeded with,
                                    // and the next join would re-seed. Only a
                                    // slot builder_params names is kept, so
                                    // nothing that merely has capacity is
                                    // aliased.
                                    let kept =
                                        self.builder_params.contains(&(name.to_string(), n, j));
                                    let stage = match kept {
                                        true => "k_carry_stage_kept",
                                        false => "k_carry_stage",
                                    };
                                    f.line(&format!("call void @{stage}(%KValue {a})"));
                                }
                                f.line("call void @k_beat_iter_carry()");
                                for (slot, &j) in positions.iter().enumerate() {
                                    let t = f.tmp();
                                    f.line(&format!(
                                        "{t} = call %KValue @k_carry_take(i64 {slot})"
                                    ));
                                    emitted[j] = t;
                                }
                            }
                            None => {
                                // everything this iteration allocated is
                                // dead; rewind to the entry mark
                                f.line("call void @k_beat_iter()");
                            }
                        }
                    }
                    let args_ir: Vec<String> = emitted
                        .iter()
                        .enumerate()
                        .map(|(i, e)| match &packed[i] {
                            Some(p) => format!("%parsed {p}"),
                            None => self.call_arg(f, name, n, i, e, args.get(i)),
                        })
                        .collect();
                    let t = f.tmp();
                    if same_ret {
                        // a frame with releasable cells settles them before the
                        // musttail: a cell riding out in the arguments escapes
                        // (counted); every other cell dies here
                        let cells = f.lazy_cells.clone();
                        for cell in cells {
                            if args_ir.iter().any(|a| a.ends_with(cell.as_str())) {
                                f.line(&format!("call void @k_thunk_note_escape(%KValue {cell})"));
                            } else {
                                let d = f.tmp();
                                f.line(&format!(
                                    "{d} = call %KValue @k_thunk_release_unless(%KValue {cell}, %KValue {{ i64 0, i64 0 }})"
                                ));
                            }
                        }
                        f.line(&format!(
                            "{t} = musttail call tailcc {callee_ret} @{}({})",
                            dsym(name, n),
                            args_ir.join(", ")
                        ));
                        f.line(&format!("ret {callee_ret} {t}"));
                    } else {
                        // A %parsed function tail-calling a KValue failure helper:
                        // can't musttail across the type change, so call and wrap.
                        f.line(&format!(
                            "{t} = call tailcc {callee_ret} @{}({})",
                            dsym(name, n),
                            args_ir.join(", ")
                        ));
                        self.emit_ret(f, &t);
                    }
                    return Ok(());
                }
            }
        }
        let value = self.emit_expr(f, expr)?;
        self.emit_ret(f, &value);
        Ok(())
    }

    fn emit_binop(
        &mut self,
        f: &mut FnEmit,
        op: &str,
        a: &str,
        b: &str,
        span: Span,
    ) -> Result<String, String> {
        // an operator reads ordinary values on both sides
        let a_owned = self.as_value(f, a);
        let b_owned = self.as_value(f, b);
        let (a, b) = (a_owned.as_str(), b_owned.as_str());
        // a record on either side dispatches to the operator's user arms; the
        // numeric fast paths below stay untouched for everything else
        let armable = matches!(op, "+" | "-" | "*" | "/" | "%" | "<" | ">" | "<=" | ">=" | "==")
            && self.program.fns.iter().any(|d| d.name == op && d.params.len() == 2);
        if armable && (f.set_of(a) | f.set_of(b)) & REC != 0 {
            let a_routes = f.tmp();
            f.line(&format!("{a_routes} = call i64 @k_routes_to_arms(%KValue {a})"));
            let b_routes = f.tmp();
            f.line(&format!("{b_routes} = call i64 @k_routes_to_arms(%KValue {b})"));
            let either = f.tmp();
            f.line(&format!("{either} = or i64 {a_routes}, {b_routes}"));
            let isrec = f.tmp();
            f.line(&format!("{isrec} = icmp ne i64 {either}, 0"));
            let user = f.label();
            let builtin = f.label();
            let merge = f.label();
            f.line(&format!("br i1 {isrec}, label %{user}, label %{builtin}"));
            f.start_block(&user);
            let uv = f.tmp();
            f.line(&format!(
                "{uv} = call tailcc %KValue @{}(%KValue {a}, %KValue {b})",
                dsym(op, 2)
            ));
            f.line(&format!("br label %{merge}"));
            let user_from = user.clone();
            f.start_block(&builtin);
            let bv = self.emit_binop_builtin(f, op, a, b, span)?;
            let builtin_from = f.cur_label.clone();
            f.line(&format!("br label %{merge}"));
            f.start_block(&merge);
            let t = f.tmp();
            f.line(&format!("{t} = phi %KValue [ {uv}, %{user_from} ], [ {bv}, %{builtin_from} ]"));
            f.record(
                &t,
                f.set_of(&bv) | self.group_return_set(op, 2) | ((f.set_of(a) | f.set_of(b)) & FAIL),
            );
            return Ok(t);
        }
        self.emit_binop_builtin(f, op, a, b, span)
    }

    fn emit_binop_builtin(
        &mut self,
        f: &mut FnEmit,
        op: &str,
        a: &str,
        b: &str,
        span: Span,
    ) -> Result<String, String> {
        let slow_call = match op {
            "+" => format!("call %KValue @k_add(%KValue {a}, %KValue {b})"),
            "-" => format!("call %KValue @k_sub(%KValue {a}, %KValue {b})"),
            "*" => format!("call %KValue @k_mul(%KValue {a}, %KValue {b})"),
            "/" => {
                let origin = self.origin_arg(f, span);
                format!("call %KValue @k_div(%KValue {a}, %KValue {b}, {origin})")
            }
            "%" => {
                let origin = self.origin_arg(f, span);
                format!("call %KValue @k_mod(%KValue {a}, %KValue {b}, {origin})")
            }
            "==" => format!("call %KValue @k_cmp(%KValue {a}, %KValue {b}, i64 0)"),
            "!=" => format!("call %KValue @k_cmp(%KValue {a}, %KValue {b}, i64 1)"),
            "<" => format!("call %KValue @k_cmp(%KValue {a}, %KValue {b}, i64 2)"),
            "<=" => format!("call %KValue @k_cmp(%KValue {a}, %KValue {b}, i64 3)"),
            ">" => format!("call %KValue @k_cmp(%KValue {a}, %KValue {b}, i64 4)"),
            ">=" => format!("call %KValue @k_cmp(%KValue {a}, %KValue {b}, i64 5)"),
            "&" => format!("call %KValue @k_b_bit_and_fast(%KValue {a}, %KValue {b})"),
            "|" => format!("call %KValue @k_b_bit_or_fast(%KValue {a}, %KValue {b})"),
            "^" => format!("call %KValue @k_b_bit_xor_fast(%KValue {a}, %KValue {b})"),
            // An operator the parser accepts and this does not know used to
            // land on the last arm and compare, which is a wrong answer with
            // nothing said. Naming every operator means a new one refuses to
            // build instead.
            other => return Err(format!("native backend: no lowering for `{other}`")),
        };
        if matches!(op, "&" | "|" | "^") {
            let t = f.tmp();
            f.line(&format!("{t} = {slow_call}"));
            f.record(&t, (f.set_of(a) & FAIL) | (f.set_of(b) & FAIL) | INT);
            return Ok(t);
        }
        if op == "/" || op == "%" {
            let t = f.tmp();
            f.line(&format!("{t} = {slow_call}"));
            f.record(&t, (f.set_of(a) & FAIL) | (f.set_of(b) & FAIL) | INT | FLOAT | ERR);
            return Ok(t);
        }
        let pure_int = f.set_of(a) == INT && f.set_of(b) == INT;
        if pure_int {
            let pa = inline_payload(f, a);
            let pb = inline_payload(f, b);
            let t = match op {
                "+" | "-" | "*" => {
                    let intrinsic = match op {
                        "+" => "llvm.sadd.with.overflow.i64",
                        "-" => "llvm.ssub.with.overflow.i64",
                        _ => "llvm.smul.with.overflow.i64",
                    };
                    let pair = f.tmp();
                    f.line(&format!(
                        "{pair} = call {{ i64, i1 }} @{intrinsic}(i64 {pa}, i64 {pb})"
                    ));
                    let sum = f.tmp();
                    f.line(&format!("{sum} = extractvalue {{ i64, i1 }} {pair}, 0"));
                    let overflow = f.tmp();
                    f.line(&format!("{overflow} = extractvalue {{ i64, i1 }} {pair}, 1"));
                    let ok = f.label();
                    let trap = f.label();
                    f.line(&format!("br i1 {overflow}, label %{trap}, label %{ok}"));
                    f.start_block(&trap);
                    let (m, _) = self.intern(
                        "integer overflow (int64 native build; spec int is arbitrary precision)\0",
                    );
                    f.line(&format!("call void @k_die(ptr @{m})"));
                    f.line("unreachable");
                    f.start_block(&ok);
                    let v = f.tmp();
                    f.line(&format!(
                        "{v} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 {sum}, 1"
                    ));
                    f.record(&v, INT);
                    v
                }
                _ => {
                    let cmp = match op {
                        "==" => "eq",
                        "!=" => "ne",
                        "<" => "slt",
                        "<=" => "sle",
                        ">" => "sgt",
                        _ => "sge",
                    };
                    let c = f.tmp();
                    f.line(&format!("{c} = icmp {cmp} i64 {pa}, {pb}"));
                    let v = f.tmp();
                    f.line(&format!(
                        "{v} = select i1 {c}, %KValue {{ i64 2, i64 0 }}, %KValue {{ i64 3, i64 0 }}"
                    ));
                    f.record(&v, infer::BOOL);
                    v
                }
            };
            return Ok(t);
        }
        let ta = inline_tag(f, a);
        let tb = inline_tag(f, b);
        let ia = f.tmp();
        f.line(&format!("{ia} = icmp eq i64 {ta}, 0"));
        let ib = f.tmp();
        f.line(&format!("{ib} = icmp eq i64 {tb}, 0"));
        let both = f.tmp();
        f.line(&format!("{both} = and i1 {ia}, {ib}"));
        let fast = f.label();
        let slow = f.label();
        let merge = f.label();
        f.line(&format!("br i1 {both}, label %{fast}, label %{slow}"));
        f.start_block(&fast);
        let pa = inline_payload(f, a);
        let pb = inline_payload(f, b);
        let (fast_value, fast_from) = match op {
            "+" | "-" | "*" => {
                let intrinsic = match op {
                    "+" => "llvm.sadd.with.overflow.i64",
                    "-" => "llvm.ssub.with.overflow.i64",
                    _ => "llvm.smul.with.overflow.i64",
                };
                let pair = f.tmp();
                f.line(&format!("{pair} = call {{ i64, i1 }} @{intrinsic}(i64 {pa}, i64 {pb})"));
                let sum = f.tmp();
                f.line(&format!("{sum} = extractvalue {{ i64, i1 }} {pair}, 0"));
                let overflow = f.tmp();
                f.line(&format!("{overflow} = extractvalue {{ i64, i1 }} {pair}, 1"));
                let fast_ok = f.label();
                f.line(&format!("br i1 {overflow}, label %{slow}, label %{fast_ok}"));
                f.start_block(&fast_ok);
                let v = f.tmp();
                f.line(&format!("{v} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 {sum}, 1"));
                (v, fast_ok)
            }
            _ => {
                let cmp = match op {
                    "==" => "eq",
                    "!=" => "ne",
                    "<" => "slt",
                    "<=" => "sle",
                    ">" => "sgt",
                    _ => "sge",
                };
                let c = f.tmp();
                f.line(&format!("{c} = icmp {cmp} i64 {pa}, {pb}"));
                let v = f.tmp();
                f.line(&format!(
                    "{v} = select i1 {c}, %KValue {{ i64 2, i64 0 }}, %KValue {{ i64 3, i64 0 }}"
                ));
                (v, fast.clone())
            }
        };
        f.line(&format!("br label %{merge}"));
        f.start_block(&slow);
        let sv = f.tmp();
        f.line(&format!("{sv} = {slow_call}"));
        let slow_from = f.cur_label.clone();
        f.line(&format!("br label %{merge}"));
        f.start_block(&merge);
        let t = f.tmp();
        f.line(&format!(
            "{t} = phi %KValue [ {fast_value}, %{fast_from} ], [ {sv}, %{slow_from} ]"
        ));
        Ok(t)
    }

    /// bytes-view indexing inlines to a bounds check and a byte load; every
    /// other container falls back to the runtime call.
    fn emit_at(
        &mut self,
        f: &mut FnEmit,
        container: &str,
        key: &str,
        strict: bool,
        span: Span,
    ) -> String {
        // The strict form's fallback is the twin, which answers a list index
        // without a call and hands the runtime everything else.
        // A container the inference knows is a string can only take the
        // twin's slow arm — the utf-8 seek does not inline — so those sites
        // keep the direct call and pay no tag test for a decision already
        // made at compile time.
        let slow_fn = match (strict, f.set_of(container) == STR) {
            (true, _) => "k_index_fast",
            (false, true) => "k_b_at",
            (false, false) => "k_b_at_fast",
        };
        let slow_extra = match strict {
            true => format!(", {}", self.origin_arg(f, span)),
            false => String::new(),
        };
        let proven = f.set_of(container) == BYTES && f.set_of(key) == INT;
        if proven {
            let bp = inline_payload(f, container);
            let bptr = f.tmp();
            f.line(&format!("{bptr} = inttoptr i64 {bp} to ptr"));
            let len_ptr = f.tmp();
            f.line(&format!("{len_ptr} = getelementptr %KBytes, ptr {bptr}, i64 0, i32 0"));
            let len = f.tmp();
            f.line(&format!("{len} = load i64, ptr {len_ptr}"));
            let idx = inline_payload(f, key);
            let ge1 = f.tmp();
            f.line(&format!("{ge1} = icmp sge i64 {idx}, 1"));
            let le_len = f.tmp();
            f.line(&format!("{le_len} = icmp sle i64 {idx}, {len}"));
            let in_range = f.tmp();
            f.line(&format!("{in_range} = and i1 {ge1}, {le_len}"));
            let load = f.label();
            let miss = f.label();
            let merge = f.label();
            f.line(&format!("br i1 {in_range}, label %{load}, label %{miss}"));
            f.start_block(&load);
            let data_ptr = f.tmp();
            f.line(&format!("{data_ptr} = getelementptr %KBytes, ptr {bptr}, i64 0, i32 1"));
            let data = f.tmp();
            f.line(&format!("{data} = load ptr, ptr {data_ptr}"));
            let off = f.tmp();
            f.line(&format!("{off} = add i64 {idx}, -1"));
            let byte_ptr = f.tmp();
            f.line(&format!("{byte_ptr} = getelementptr i8, ptr {data}, i64 {off}"));
            let byte = f.tmp();
            f.line(&format!("{byte} = load i8, ptr {byte_ptr}"));
            let wide = f.tmp();
            f.line(&format!("{wide} = zext i8 {byte} to i64"));
            let hit = f.tmp();
            f.line(&format!("{hit} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 {wide}, 1"));
            f.line(&format!("br label %{merge}"));
            f.start_block(&miss);
            let miss_value = if strict {
                let mv = f.tmp();
                f.line(&format!(
                    "{mv} = call %KValue @{slow_fn}(%KValue {container}, %KValue {key}{slow_extra})"
                ));
                mv
            } else {
                "{ i64 4, i64 0 }".to_string()
            };
            let miss_from = f.cur_label.clone();
            f.line(&format!("br label %{merge}"));
            f.start_block(&merge);
            let t = f.tmp();
            f.line(&format!(
                "{t} = phi %KValue [ {hit}, %{load} ], [ {miss_value}, %{miss_from} ]"
            ));
            f.record(&t, if strict { INT | ERR } else { INT | NONE });
            return t;
        }
        let ct = inline_tag(f, container);
        let is_bytes = f.tmp();
        f.line(&format!("{is_bytes} = icmp eq i64 {ct}, 13"));
        let kt = inline_tag(f, key);
        let is_int = f.tmp();
        f.line(&format!("{is_int} = icmp eq i64 {kt}, 0"));
        let both = f.tmp();
        f.line(&format!("{both} = and i1 {is_bytes}, {is_int}"));
        let fast = f.label();
        let slow = f.label();
        let merge = f.label();
        f.line(&format!("br i1 {both}, label %{fast}, label %{slow}"));
        f.start_block(&fast);
        let bp = inline_payload(f, container);
        let bptr = f.tmp();
        f.line(&format!("{bptr} = inttoptr i64 {bp} to ptr"));
        let len_ptr = f.tmp();
        f.line(&format!("{len_ptr} = getelementptr %KBytes, ptr {bptr}, i64 0, i32 0"));
        let len = f.tmp();
        f.line(&format!("{len} = load i64, ptr {len_ptr}"));
        let idx = inline_payload(f, key);
        let ge1 = f.tmp();
        f.line(&format!("{ge1} = icmp sge i64 {idx}, 1"));
        let le_len = f.tmp();
        f.line(&format!("{le_len} = icmp sle i64 {idx}, {len}"));
        let in_range = f.tmp();
        f.line(&format!("{in_range} = and i1 {ge1}, {le_len}"));
        let load = f.label();
        f.line(&format!("br i1 {in_range}, label %{load}, label %{slow}"));
        f.start_block(&load);
        let data_ptr = f.tmp();
        f.line(&format!("{data_ptr} = getelementptr %KBytes, ptr {bptr}, i64 0, i32 1"));
        let data = f.tmp();
        f.line(&format!("{data} = load ptr, ptr {data_ptr}"));
        let off = f.tmp();
        f.line(&format!("{off} = add i64 {idx}, -1"));
        let byte_ptr = f.tmp();
        f.line(&format!("{byte_ptr} = getelementptr i8, ptr {data}, i64 {off}"));
        let byte = f.tmp();
        f.line(&format!("{byte} = load i8, ptr {byte_ptr}"));
        let wide = f.tmp();
        f.line(&format!("{wide} = zext i8 {byte} to i64"));
        let fast_value = f.tmp();
        f.line(&format!(
            "{fast_value} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 {wide}, 1"
        ));
        f.line(&format!("br label %{merge}"));
        f.start_block(&slow);
        let slow_value = f.tmp();
        f.line(&format!(
            "{slow_value} = call %KValue @{slow_fn}(%KValue {container}, %KValue {key}{slow_extra})"
        ));
        let slow_from = f.cur_label.clone();
        f.line(&format!("br label %{merge}"));
        f.start_block(&merge);
        let t = f.tmp();
        f.line(&format!(
            "{t} = phi %KValue [ {fast_value}, %{load} ], [ {slow_value}, %{slow_from} ]"
        ));
        t
    }

    fn emit_call_full(
        &mut self,
        f: &mut FnEmit,
        head: &Expr,
        args: &[Expr],
        piped: bool,
        span: Span,
    ) -> Result<String, String> {
        if piped && !args.is_empty() {
            let piped_value = self.emit_expr(f, &args[0])?;
            if f.set_of(&piped_value) & DESC != 0 {
                let mut body_args: Vec<Expr> = vec![Expr::Ident(Name::new("__piped"), span)];
                body_args.extend(args[1..].iter().cloned());
                let lambda = Expr::Lambda {
                    params: vec![("__piped".to_string(), span)],
                    body: Box::new(Expr::App {
                        head: Box::new(head.clone()),
                        args: body_args,
                        span,
                        piped: false,
                    }),
                    span,
                };
                let closure = self.emit_expr(f, &lambda)?;
                let t = f.tmp();
                f.line(&format!(
                    "{t} = call %KValue @k_maybe_bind(%KValue {piped_value}, %KValue {closure})"
                ));
                f.record(&t, TOP);
                return Ok(t);
            }
            // a pipe hands its value on; a failure short-circuits before the
            // call (no dispatch, no hop) on every engine
            if f.set_of(&piped_value) & FAIL != 0 {
                let ok = inline_not_failure(f, &piped_value);
                let docall = f.label();
                let merge = f.label();
                let fail_from = f.cur_label.clone();
                f.line(&format!("br i1 {ok}, label %{docall}, label %{merge}"));
                f.start_block(&docall);
                let called = self.emit_call_rest(f, head, args, Some(piped_value.clone()), span)?;
                let call_from = f.cur_label.clone();
                f.line(&format!("br label %{merge}"));
                f.start_block(&merge);
                let t = f.tmp();
                f.line(&format!(
                    "{t} = phi %KValue [ {piped_value}, %{fail_from} ], [ {called}, %{call_from} ]"
                ));
                f.record(&t, f.set_of(&called) | (f.set_of(&piped_value) & FAIL));
                return Ok(t);
            }
            // no description or failure can flow here: an ordinary call
            return self.emit_call_rest(f, head, args, Some(piped_value), span);
        }
        self.emit_call_rest(f, head, args, None, span)
    }

    /// The origin an err would carry had it been born inside the wrapper
    /// rather than at this call site. Fusing past a wrapper skips the frame
    /// whose file and line name the birthplace, and the oracle still calls it.
    fn forwarder_origin(&mut self, name: &str, arity: usize) -> Option<String> {
        let decl = self.program.fns.iter().find(|d| d.name == name && d.params.len() == arity)?;
        let line = match decl.body.first()? {
            Stmt::Expr(Expr::App { span, .. }) => span.line,
            _ => return None,
        };
        // The same two-halves literal `origin_arg` builds, and for the same
        // reason: this stamps a wrapper's own frame on an err the builtin
        // raised, so the package it names is the WRAPPER's, not the caller's.
        let hako = crate::provenance::package_of(&decl.file);
        let prefix = format!("{} at {}", crate::ast::frame_name(&decl.name), decl.file);
        let (interned, _) = self.intern(&format!("{hako}\0{prefix}:{line}\0"));
        Some(format!("ptr @{interned}"))
    }

    /// The builtin a name stands for at a call site: itself with the
    /// `builtin_` prefix off, or whatever the forwarder map says a plain
    /// wrapper of this arity forwards to.
    fn builtin_named(&self, name: &str, arity: usize) -> String {
        match self.forwarders.get(&(name.to_string(), arity)) {
            Some(target) => target.clone(),
            None => name.strip_prefix("builtin_").unwrap_or(name).to_string(),
        }
    }

    fn emit_call_rest(
        &mut self,
        f: &mut FnEmit,
        head: &Expr,
        args: &[Expr],
        first: Option<String>,
        span: Span,
    ) -> Result<String, String> {
        // `first` is args[0], already emitted — a pipe hands its value in as
        // the head's first argument, it does not add one. Counting it twice
        // made a piped call to a lambda pass the value in both positions,
        // which the old unchecked cast to a two-argument signature dropped.
        let call_arity = args.len();
        // A literal lambda applied on the spot is a binding, not a value:
        // bind the arguments and emit the body here, instead of building a
        // closure and dispatching through k_callN. The fusion pass composes
        // adapter chains out of exactly these redexes, so without this step
        // a fused reducer pays two closures and two dynamic calls per
        // element. Failing arguments short-circuit first, as k_callN would.
        if let Expr::Lambda { params, body, .. } = head {
            if params.len() == call_arity {
                let mut vals: Vec<String> = Vec::new();
                let mut rest = args.iter();
                if let Some(v) = first.clone() {
                    vals.push(v);
                    rest.next();
                }
                for a in rest {
                    vals.push(self.emit_expr(f, a)?);
                }
                let mut bails: Vec<(String, String)> = Vec::new();
                let merge = f.label();
                for v in &vals {
                    if f.set_of(v) & FAIL == 0 {
                        continue;
                    }
                    let ok = inline_not_failure(f, v);
                    let bail_from = f.cur_label.clone();
                    let cont = f.label();
                    f.line(&format!("br i1 {ok}, label %{cont}, label %{merge}"));
                    bails.push((v.clone(), bail_from));
                    f.start_block(&cont);
                }
                let saved: Vec<(String, Option<String>)> =
                    params.iter().map(|(p, _)| (p.clone(), f.lookup(p))).collect();
                for ((p, _), v) in params.iter().zip(&vals) {
                    f.bind(p, v);
                }
                let out = self.emit_expr(f, body)?;
                for (p, old) in saved {
                    match old {
                        Some(v) => f.bind(&p, &v),
                        None => {
                            f.versions.remove(&p);
                        }
                    }
                }
                if bails.is_empty() {
                    return Ok(out);
                }
                let body_from = f.cur_label.clone();
                let out_set = f.set_of(&out);
                let fail_bits: Set = bails.iter().fold(0, |acc, (v, _)| acc | (f.set_of(v) & FAIL));
                f.line(&format!("br label %{merge}"));
                f.start_block(&merge);
                let t = f.tmp();
                let mut sources: Vec<String> =
                    bails.iter().map(|(v, from)| format!("[ {v}, %{from} ]")).collect();
                sources.push(format!("[ {out}, %{body_from} ]"));
                f.line(&format!("{t} = phi %KValue {}", sources.join(", ")));
                f.record(&t, out_set | fail_bits);
                return Ok(t);
            }
        }
        let computed_head = match head {
            // A local binding is a value. So is a top-level constant (a nullary
            // group) invoked with arguments and no arm at that arity: it holds a
            // function value, and `f x` calls that value, not a group named `f`.
            //
            // So is a value keyword, which is spelled like a name and reaches
            // this position when a callback is inlined: `list/map [1 2] none`
            // puts `none` where the callee goes. A number there takes the
            // computed path already and dies naming itself, and these must say
            // the same words rather than the emitter's.
            Expr::Ident(name, _) => {
                matches!(name.as_str(), "true" | "false" | "none")
                    || f.lookup(name).is_some()
                    || (call_arity >= 1
                        && self.program.fns.iter().any(|d| d.name == *name && d.params.is_empty())
                        && !self
                            .program
                            .fns
                            .iter()
                            .any(|d| d.name == *name && d.params.len() == call_arity))
            }
            _ => true,
        };
        if computed_head {
            // The callee is a value (a lambda, a parameter, a bound function),
            // not a declared group: emit the head and all arguments as values
            // and dispatch at runtime via the arity-matched k_callN.
            let callee = self.emit_expr(f, head)?;
            let mut arg_vals: Vec<String> = Vec::new();
            let mut rest = args.iter();
            if let Some(v) = first {
                arg_vals.push(v);
                rest.next();
            }
            for a in rest {
                arg_vals.push(self.emit_expr(f, a)?);
            }
            let n = arg_vals.len();
            if n > 4 {
                return Err(format!(
                    "native backend: a function value takes at most 4 arguments, got {n}"
                ));
            }
            let arg_ir: String = arg_vals.iter().map(|v| format!(", %KValue {v}")).collect();
            let t = f.tmp();
            f.line(&format!("{t} = call %KValue @k_call{n}_fast(%KValue {callee}{arg_ir})"));
            f.record(&t, TOP);
            return Ok(t);
        }
        let Expr::Ident(name, _) = head else {
            unreachable!("non-ident heads take the computed path");
        };
        if name == "if" {
            // A condition the demand analysis deferred arrives as a thunk, and
            // asking a thunk whether it is true reads the thunk rather than the
            // answer. Force before testing: `maybe_force` emits nothing where
            // the set proves there is no thunk, so a strict condition is
            // unchanged.
            let cond = self.emit_expr(f, &args[0])?;
            let cond = self.maybe_force(f, cond);
            let nf = f.tmp();
            f.line(&format!("{nf} = call i64 @k_not_failure(%KValue {cond})"));
            let ok = f.tmp();
            f.line(&format!("{ok} = icmp ne i64 {nf}, 0"));
            let check = f.label();
            let merge = f.label();
            let fail_from = f.cur_label.clone();
            f.line(&format!("br i1 {ok}, label %{check}, label %{merge}"));
            f.start_block(&check);
            let tv = f.tmp();
            f.line(&format!("{tv} = call i64 @k_truthy(%KValue {cond})"));
            let tb = f.tmp();
            f.line(&format!("{tb} = icmp ne i64 {tv}, 0"));
            let then_label = f.label();
            let else_label = f.label();
            f.line(&format!("br i1 {tb}, label %{then_label}, label %{else_label}"));
            f.start_block(&then_label);
            let then_value = self.emit_expr(f, &args[1])?;
            let then_from = f.cur_label.clone();
            f.line(&format!("br label %{merge}"));
            f.start_block(&else_label);
            let else_value = self.emit_expr(f, &args[2])?;
            let else_from = f.cur_label.clone();
            f.line(&format!("br label %{merge}"));
            f.start_block(&merge);
            let t = f.tmp();
            f.line(&format!(
                "{t} = phi %KValue [ {cond}, %{fail_from} ], [ {then_value}, %{then_from} ], \
                 [ {else_value}, %{else_from} ]"
            ));
            f.record(&t, f.set_of(&then_value) | f.set_of(&else_value) | (f.set_of(&cond) & FAIL));
            return Ok(t);
        }
        // utf8 of a slice reads a byte view for a pointer and a length and
        // drops it, three million times in a decode. The wrapper inlining
        // above has already turned `text/utf8` and `text/slice` into their
        // builtins, so the pair is visible here as written, before either
        // argument is emitted.
        // An err's birthplace is the function it is emitted in, so fusing a
        // call that still names a wrapper would move it out of `text/utf8` and
        // into the caller — the oracle, which really does call the wrapper,
        // would then disagree about where an invalid byte was found. The
        // wrapper inlining above rewrites the call to the builtin wherever it
        // can, and only that spelling is fused.
        if first.is_none() && args.len() == 1 && self.builtin_named(name, 1) == "utf8" {
            if let Expr::App { head: inner_head, args: inner_args, piped: false, .. } = &args[0] {
                if let Expr::Ident(inner, _) = &**inner_head {
                    if self.builtin_named(inner, inner_args.len()) == "slice"
                        && inner_args.len() == 3
                    {
                        let mut parts = Vec::new();
                        for a in inner_args {
                            let v = self.emit_expr(f, a)?;
                            parts.push(self.maybe_force(f, v));
                        }
                        let sets: Vec<Set> = parts.iter().map(|e| f.set_of(e)).collect();
                        let sliced = infer::builtin_set("slice", &sets);
                        // the wrapper's own line, where the unfused call
                        // would have been emitted, or this site when the
                        // spelling is already the builtin
                        let origin = match name.as_str() {
                            "builtin_utf8" => self.origin_arg(f, span),
                            _ => match self.forwarder_origin(name, 1) {
                                Some(o) => o,
                                None => self.origin_arg(f, span),
                            },
                        };
                        let t = f.tmp();
                        f.line(&format!(
                            "{t} = call %KValue @k_b_utf8_slice(%KValue {}, %KValue {}, %KValue {}, {origin})",
                            parts[0], parts[1], parts[2]
                        ));
                        f.record(&t, infer::builtin_set("utf8", &[sliced]));
                        return Ok(t);
                    }
                }
            }
        }
        let mut emitted = Vec::new();
        let mut iter = args.iter();
        if let Some(first_value) = first {
            emitted.push(first_value);
            iter.next();
        }
        for arg in iter {
            emitted.push(self.emit_expr(f, arg)?);
        }
        // std wrappers reach natives through the builtin_ prefix — and the
        // prefix BYPASSES group dispatch entirely, or a bare clone named
        // like the builtin would capture its own wrapper's body (the
        // d_join_2 self-recursion)
        let was_builtin = name.starts_with("builtin_");
        let name: &str = name.strip_prefix("builtin_").unwrap_or(name);

        // Every builtin's count, before anything reads an argument by index.
        // The block below emits `wrap_err` inline and takes `emitted[1]`, and
        // the guard further down covers only the names this file emits a
        // direct C call for — `wrap_err` is not one of them. So
        // `print (wrap_err 1)` walked off the end of a one-element vector and
        // aborted the process: exit 101 with a Rust backtrace, on a two-word
        // program. The front door refuses that program now, and this stays
        // because a backend that indexes an argument it never counted is one
        // front-end regression from doing it again.
        // A declaration of the same name is that declaration — the bail
        // further down says so, and this has to say it too, because it runs
        // first. lib/sha256 declares `bytes`, and a guard that skipped this
        // condition refused its three-argument call as a wrong-count builtin.
        let shadows = !was_builtin && self.program.fns.iter().any(|d| d.name == name);
        if !shadows {
            if let Some(takes) = crate::check::builtin_arity(name) {
                if emitted.len() != takes {
                    return Err(format!("native backend: `{name}` takes {takes} argument(s)"));
                }
            }
        }

        if name == "err" {
            let origin = self.origin_arg(f, span);
            let t = f.tmp();
            f.line(&format!("{t} = call %KValue @k_err(%KValue {}, {origin})", emitted[0]));
            f.record(&t, ERR);
            return Ok(t);
        }
        // `annotate` raises an err of its own, so like `err` and `wrap_err` it
        // is handed the site it was written at. The runtime wraps the callback
        // in a closure holding both and hands the result to rescue's node.
        if name == "annotate" {
            let origin = self.origin_arg(f, span);
            let t = f.tmp();
            f.line(&format!(
                "{t} = call %KValue @k_b_annotate(%KValue {}, %KValue {}, {origin})",
                emitted[0], emitted[1]
            ));
            f.record(&t, TOP);
            return Ok(t);
        }
        if name == "wrap_err" {
            let origin = self.origin_arg(f, span);
            let t = f.tmp();
            f.line(&format!(
                "{t} = call %KValue @k_b_wrap_err(%KValue {}, %KValue {}, {origin})",
                emitted[0], emitted[1]
            ));
            f.record(&t, ERR);
            return Ok(t);
        }
        if name == "print" {
            // a non-string argument renders through the same ambient
            // to_string dispatch interpolation uses, so user arms win
            let arg = match f.set_of(&emitted[0]) & !FAIL & !STR {
                0 => emitted[0].clone(),
                _ => {
                    let forced = self.maybe_force(f, emitted[0].clone());
                    let r = f.tmp();
                    f.line(&format!(
                        "{r} = call tailcc %KValue @{}(%KValue {forced})",
                        dsym("render/to_string", 1)
                    ));
                    f.record(&r, STR | (f.set_of(&forced) & FAIL) | ERR);
                    r
                }
            };
            let t = f.tmp();
            f.line(&format!("{t} = call %KValue @k_desc_print(%KValue {arg})"));
            f.record(&t, DESC | (f.set_of(&arg) & FAIL));
            return Ok(t);
        }
        if name == "sleep" || name == "random" {
            let t = f.tmp();
            f.line(&format!("{t} = call %KValue @k_desc_{name}(%KValue {})", emitted[0]));
            f.record(&t, DESC | (f.set_of(&emitted[0]) & FAIL));
            return Ok(t);
        }
        if let Some(id) = self.type_ids.get(name).copied() {
            if let Some(parent) = self.sub_parents.get(name).cloned() {
                if emitted.len() != 1 {
                    return Err(format!("native backend: `{name}` wraps one value"));
                }
                let inner = self.maybe_force(f, emitted[0].clone());
                let want = self.sub_want(&parent)?;
                let (tyn, _) = self.intern(&format!("{name}\0"));
                let (par, _) = self.intern(&format!("{parent}\0"));
                let t = f.tmp();
                f.line(&format!(
                    "{t} = call %KValue @k_sub_ctor(i64 {id}, i64 {want}, %KValue {inner}, ptr @{tyn}, ptr @{par})"
                ));
                f.record(&t, crate::infer::TOP);
                return Ok(t);
            }
            // A constructor slot is where a knot ties: a field still being
            // computed is stored, so the cell completes here and the field
            // resolves against it afterwards. Whether a cell is mid-flight is
            // a runtime fact, so the emitter cannot decide it — the helper
            // asks, and only programs that defer a self-reference pay for it.
            let emitted: Vec<String> = emitted
                .into_iter()
                .map(|e| match self.defers_self_reference {
                    true => self.force_unless_knot(f, e),
                    false => self.maybe_force(f, e),
                })
                .collect();
            self.emit_typeset_checks(f, name, &emitted)?;
            let n = emitted.len();
            let arr = f.tmp();
            f.line(&format!("{arr} = alloca [{n} x %KValue]"));
            for (i, value) in emitted.iter().enumerate() {
                let slot = f.tmp();
                f.line(&format!(
                    "{slot} = getelementptr [{n} x %KValue], ptr {arr}, i64 0, i64 {i}"
                ));
                f.line(&format!("store %KValue {value}, ptr {slot}"));
            }
            let t = f.tmp();
            // a record this call is the last reader of can be built into
            let victim = self
                .reusable_records
                .get(&(f.file.clone(), span.line as usize, span.col as usize))
                .and_then(|name| f.lookup(name));
            match victim {
                Some(v) => f.line(&format!(
                    "{t} = call %KValue @k_rec_reuse(i64 {id}, i64 {n}, ptr {arr}, %KValue {v})"
                )),
                None => f.line(&format!("{t} = call %KValue @k_rec(i64 {id}, i64 {n}, ptr {arr})")),
            }
            let fails: Set = emitted.iter().fold(0, |acc, e| acc | (f.set_of(e) & FAIL));
            f.record(&t, REC | fails);
            return Ok(t);
        }
        // The arity has to match a real declaration. Matching on the name
        // alone emits a call to `d_{name}_{n}` for any n the caller wrote,
        // and a dispatcher that was never defined is invalid IR the user
        // meets as a clang error.
        let declared = |d: &FnDecl| d.name == *name && d.params.len() == emitted.len();
        if !was_builtin && self.program.fns.iter().any(declared) {
            let n = emitted.len();
            let args_ir: Vec<String> = emitted
                .iter()
                .enumerate()
                .map(|(i, e)| self.call_arg(f, name, n, i, e, args.get(i)))
                .collect();
            let callee_ret = self.ret_ty(name, n);
            // A register-returned record comes back as two raw field words,
            // not a tagged value, and both pops read a KValue. Reinterpreting
            // one would hand them a pair of fields to treat as a tag and a
            // payload, so the frontier goes unmarked instead: these calls give
            // up the rewind rather than get it wrong.
            let register_returned = callee_ret == "%parsed";
            let beat_entry =
                self.beat.ids.contains_key(&(name.to_string(), n)) && !register_returned;
            // a construction cohort: a qualified call from user code whose
            // arguments are all immutable shapes (scalars, strings) cannot
            // have its caller's storage grown by the callee, so the call's
            // garbage dies with the pop. loops keep their own tier, and a
            // caller already inside a beat cluster lets its rewind do the
            // reclaiming instead.
            let heapish: Set = BYTES | LIST | MAP | REC | DESC | infer::FN | crate::infer::THUNK;
            // the call must cross down into a nested module — the caller's
            // own code reaching a dependency, at whatever depth the import
            // graph put the caller. a caller that is itself a rewinding loop
            // member keeps its own tier. a group that appears in the beat
            // ids only as a demoted entry is not a loop: its bracket never
            // rewinds mid-body, so the cohort wrap still applies inside it.
            let caller = (f.group.clone(), f.arity);
            let caller_loops = self.beat.ids.contains_key(&caller)
                && !self.beat.demoted.iter().any(|(_, callee)| *callee == caller);
            // bytes join scalars and strings in the license: raw bytes hold
            // no pointers and no thunks, so nothing a rewind frees can be
            // reached through them, and a mut-grown unique arg is
            // unreachable after its last use. containers stay excluded —
            // they can carry thunks whose forced values would die under a
            // cell the caller still holds.
            let arg_heapish = heapish & !BYTES;
            let caller_mod = crate::ast::split_qual(&f.group).map(|(m, _)| m).unwrap_or("");
            let callee_mod = crate::ast::split_qual(name).map(|(m, _)| m).unwrap_or("");
            let crosses_down = callee_mod.len() > caller_mod.len()
                && callee_mod.starts_with(caller_mod)
                && (caller_mod.is_empty() || callee_mod.as_bytes()[caller_mod.len()] == b'/');
            let cohort_entry = !beat_entry
                && !register_returned
                && crosses_down
                && !f.synthetic
                && !caller_loops
                && emitted.iter().all(|e| f.set_of(e) & arg_heapish == 0);
            if beat_entry || cohort_entry {
                // entering a beat loop or a cohort: mark the frontier; args
                // are already evaluated, so they live below the mark
                f.line("call void @k_beat_push()");
            }
            let t = f.tmp();
            f.line(&format!(
                "{t} = call tailcc {callee_ret} @{}({})",
                dsym(name, n),
                args_ir.join(", ")
            ));
            let fails: Set = emitted.iter().fold(0, |acc, e| acc | (f.set_of(e) & FAIL));
            let result = if beat_entry {
                let p = f.tmp();
                f.line(&format!("{p} = call %KValue @k_beat_pop(%KValue {t})"));
                p
            } else if cohort_entry {
                let p = f.tmp();
                f.line(&format!("{p} = call %KValue @k_cohort_pop(%KValue {t})"));
                p
            } else {
                t
            };
            if let Some(ty) = self.escape.returns_ty(name, n) {
                if callee_ret == "%parsed" {
                    f.record_parsed(&result, ty, self.type_ids[ty]);
                }
            }
            f.record(&result, self.group_return_set(name, n) | fails);
            return Ok(result);
        }
        // Declared, but at no arity this call can reach. The interpreter
        // reports it when the call runs, so native reports the same words at
        // the same moment rather than refusing to build a program the oracle
        // executes.
        if !was_builtin && self.program.fns.iter().any(|d| d.name == *name) {
            let msg = format!("no overload of `{name}` matches these arguments");
            let (m, _) = self.intern(&format!("{msg}\0"));
            f.line(&format!("call void @k_die(ptr @{m})"));
            f.line("unreachable");
            let after = f.label();
            f.start_block(&after);
            let t = f.tmp();
            f.line(&format!(
                "{t} = select i1 true, %KValue {{ i64 4, i64 0 }}, %KValue {{ i64 4, i64 0 }}"
            ));
            f.record(&t, NONE);
            return Ok(t);
        }
        if name == "at" && emitted.len() == 2 {
            return Ok(self.emit_at(f, &emitted[0].clone(), &emitted[1].clone(), false, span));
        }
        // a std wrapper that only forwards to a builtin costs a dispatched
        // call per use; the call site goes straight to the builtin (and its
        // inline twins). The rename lives INSIDE this branch only — it must
        // never leak into user-group dispatch, whose per-site specialized
        // signatures the renamed identity would not match.
        let forwarded = self.forwarders.get(&(name.to_string(), emitted.len())).cloned();
        let name: &str = match &forwarded {
            Some(target) => target.as_str(),
            None => name,
        };
        if let Some(arity) = arity_of_emitted(name) {
            if emitted.len() != arity {
                return Err(format!("native backend: `{name}` takes {arity} argument(s)"));
            }
            // builtins scrutinize every argument; a thunk forces here (the
            // gated force emits nothing when the set proves it can't be one)
            let emitted: Vec<String> =
                emitted.into_iter().map(|e| self.maybe_force(f, e)).collect();
            let mut args_ir: Vec<String> = emitted.iter().map(|e| format!("%KValue {e}")).collect();
            // builtins that can give birth to an err take the site's origin
            if matches!(name, "to_int" | "to_float" | "utf8" | "from_code" | "to_bytes") {
                args_ir.push(self.origin_arg(f, span));
            }
            // A push the linearity analysis proved unique extends its list in
            // place instead of allocating a fresh header.
            let in_place = self.in_place_pushes.contains(&(
                f.file.clone(),
                span.line as usize,
                span.col as usize,
            ));
            let sym = if name == "push" && in_place {
                // the twin claims the frontier slot itself; a grow, a full
                // buffer or anything that is not a list falls to the C
                "push_mut_fast"
            } else if name == "put" && in_place {
                // the twin writes the frontier pair itself where the map has
                // no sorted view; everything else falls to the C by call
                "put_mut_fast"
            } else if name == "append" && in_place {
                // the in-place byte claim inlines whole; a byte that does not
                // fit falls through to the C path inside the twin
                "append_mut_byte"
            } else if BIT_TWINS.contains(&name) {
                // one machine op each where the operand tags say int and a
                // shift is in range. `&` `|` `^` reach the twin through the
                // operator route; these are the same work written as a name,
                // which is how lib/bits spells what no operator says.
                bit_twin(name)
            } else if name == "length" {
                // the list case is a header load; the twin inlines it
                "length_fast"
            } else if name == "append" {
                // the single-byte frontier claim inlines whole; everything
                // else falls through to the C path inside the twin
                "append_byte"
            } else {
                name
            };
            let t = f.tmp();
            f.line(&format!("{t} = call %KValue @k_b_{sym}({})", args_ir.join(", ")));
            let arg_sets: Vec<Set> = emitted.iter().map(|e| f.set_of(e)).collect();
            f.record(&t, infer::builtin_set(name, &arg_sets));
            return Ok(t);
        }
        Err(format!("native backend: `{name}` is not yet supported"))
    }
}

impl<'a> Backend<'a> {
    fn emit_lifted(
        &mut self,
        lifted: &str,
        params: &[String],
        captures: &[String],
        body: &Expr,
        outer: &FnEmit,
    ) -> Result<(), String> {
        let mut f = FnEmit::new();
        f.origin_prefix = outer.origin_prefix.clone();
        f.hako = outer.hako.clone();
        // A lifted lambda is still code from the file it was written in, and
        // in-place sites are keyed by source position — without this the key
        // is ("", line, col) and every mark inside a lambda body is missed.
        // That miss was load-bearing until the analysis stopped marking writes
        // in lambdas whose run time it cannot account for; it now marks only a
        // fold's reducer, which is applied at once and per element to the
        // accumulator the fold owns.
        f.file = outer.file.clone();
        f.start_block("entry");
        for (i, cap) in captures.iter().enumerate() {
            let t = f.tmp();
            f.line(&format!("{t} = call %KValue @k_env_get(ptr %env, i64 {i})"));
            f.bind(cap, &t);
        }
        for (i, p) in params.iter().enumerate() {
            f.bind(p, &format!("%a{i}"));
        }
        self.emit_tail(&mut f, body)?;
        let sig: String = (0..params.len()).map(|i| format!(", %KValue %a{i}")).collect();
        let _ =
            writeln!(self.body, "define tailcc %KValue @{lifted}(ptr %env{sig}) {{\n{}}}\n", f.out);
        let _ = writeln!(
            self.body,
            "define %KValue @w_{lifted}(ptr %env{sig}) {{\nentry:\n  %r = call \
             tailcc %KValue @{lifted}(ptr %env{sig})\n  ret %KValue %r\n}}\n"
        );
        Ok(())
    }
}

fn collect_idents(expr: &Expr, out: &mut Vec<String>) {
    match expr {
        Expr::Int(..) | Expr::Float(..) | Expr::Partial(..) => {}
        Expr::Block(stmts, _) | Expr::Build(stmts, _) => {
            for stmt in stmts {
                match stmt {
                    Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => {
                        collect_idents(expr, out)
                    }
                }
            }
        }
        Expr::Field { base, .. } => collect_idents(base, out),
        Expr::Upcast { expr, .. } => collect_idents(expr, out),
        Expr::Str(parts, _) => {
            for part in parts {
                if let TemplatePart::Interp(inner) = part {
                    collect_idents(inner, out);
                }
            }
        }
        Expr::Ident(name, _) => out.push(name.to_string()),
        Expr::List(items, _) => {
            for item in items {
                collect_idents(item, out);
            }
        }
        Expr::MapLit(pairs, _) => {
            for (key, value) in pairs {
                collect_idents(key, out);
                collect_idents(value, out);
            }
        }
        Expr::App { head, args, .. } => {
            collect_idents(head, out);
            for arg in args {
                collect_idents(arg, out);
            }
        }
        Expr::Index { base, index, .. } => {
            collect_idents(base, out);
            collect_idents(index, out);
        }
        Expr::Seq(lhs, rhs, _) => {
            collect_idents(lhs, out);
            collect_idents(rhs, out);
        }
        Expr::Lambda { body, .. } => collect_idents(body, out),
        Expr::BinOp { lhs, rhs, .. } | Expr::Join { lhs, rhs, .. } => {
            collect_idents(lhs, out);
            collect_idents(rhs, out);
        }
        Expr::Guard { cond, early, rest, .. } => {
            collect_idents(cond, out);
            collect_idents(early, out);
            for stmt in rest {
                match stmt {
                    Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => {
                        collect_idents(expr, out)
                    }
                }
            }
        }
    }
}

fn ir_bytes(bytes: &[u8]) -> String {
    let mut out = String::new();
    for byte in bytes {
        match byte {
            0x20..=0x7e if *byte != b'"' && *byte != b'\\' => out.push(*byte as char),
            _ => {
                let _ = write!(out, "\\{byte:02X}");
            }
        }
    }
    out
}

/// `tailcc` where it is needed, the C convention everywhere else.
///
/// A `musttail` call may cross an arity or a type only under `tailcc`, so the
/// beat machinery's guaranteed tail calls need it. Every other call does not,
/// and paying for it is not free: a non-tail `call tailcc` whose arguments do
/// not all fit in registers is miscompiled on arm64 — five KValues want ten
/// argument registers and there are eight, and the two that spill come back
/// holding each other's values (task #70). So the convention is kept for the
/// functions a musttail reaches, on both ends of every such edge, and dropped
/// from the rest.
///
/// A function that needs the convention AND spills is reached through a
/// trampoline: one `tailcc` call per frame is lowered correctly, and it is
/// only the second one in a frame that comes back wrong.
///
/// The set is read out of the emitted text rather than recomputed, because a
/// second copy of "when do we musttail" would drift from the first and the
/// symptom of drift is silent corruption.
fn narrow_tailcc(ir: String) -> String {
    let mut keep: crate::hash::Set<String> = crate::hash::Set::default();
    let mut current: Option<String> = None;
    for line in ir.lines() {
        if let Some(rest) = line.strip_prefix("define ") {
            current = symbol_of(rest);
        }
        if line.contains("musttail call") {
            // both ends of a musttail edge must agree on the convention
            if let Some(callee) = symbol_of(line) {
                keep.insert(callee);
            }
            if let Some(name) = current.clone() {
                keep.insert(name);
            }
        }
    }
    // Kept functions whose arguments do not all fit in the eight registers
    // AArch64 passes them in. The count is the same on every host so the ir is
    // too; x86 passes fewer and does not exhibit the defect anyway.
    let mut trampolines: Vec<String> = Vec::new();
    let mut spilling: crate::hash::Set<String> = crate::hash::Set::default();
    for line in ir.lines() {
        let Some(rest) = line.strip_prefix("define tailcc ") else { continue };
        let Some(name) = symbol_of(rest) else { continue };
        if !keep.contains(&name) {
            continue;
        }
        let Some(open) = rest.find('(') else { continue };
        let Some(close) = rest.rfind(')') else { continue };
        let params: Vec<&str> = match rest[open + 1..close].trim().is_empty() {
            true => Vec::new(),
            false => rest[open + 1..close].split(", ").collect(),
        };
        let types: Vec<&str> = params.iter().filter_map(|p| p.split_whitespace().next()).collect();
        let registers: usize = types.iter().map(|t| register_width(t)).sum();
        if registers <= 8 {
            continue;
        }
        let ret = rest[..open].split('@').next().unwrap_or("%KValue").trim().to_string();
        let taken: Vec<String> =
            types.iter().enumerate().map(|(i, ty)| format!("{ty} %a{i}")).collect();
        let handed: Vec<String> =
            types.iter().enumerate().map(|(i, ty)| format!("{ty} %a{i}")).collect();
        trampolines.push(format!(
            "define {ret} @{}({}) {{\nentry:\n  %r = call tailcc {ret} @{}({})\n  ret {ret} %r\n}}\n",
            trampoline_name(&name),
            taken.join(", "),
            quoted(&name),
            handed.join(", ")
        ));
        spilling.insert(name);
    }

    let mut out = String::with_capacity(ir.len());
    for line in ir.lines() {
        let named = symbol_of(line);
        let reroute = !line.contains("musttail call")
            && line.contains("call tailcc ")
            && named.as_ref().is_some_and(|n| spilling.contains(n));
        if reroute {
            let name = named.expect("checked above");
            let call = format!("@{}(", quoted(&name));
            let through = format!("@{}(", trampoline_name(&name));
            out.push_str(&line.replace("call tailcc ", "call ").replace(&call, &through));
        } else if line.contains("tailcc ") && !named.as_ref().is_some_and(|n| keep.contains(n)) {
            out.push_str(&line.replace("tailcc ", ""));
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    for t in trampolines {
        out.push_str(&t);
    }
    out
}

/// The first symbol a line names, without its quotes.
fn symbol_of(line: &str) -> Option<String> {
    let at = line.find('@')?;
    let rest = &line[at + 1..];
    match rest.starts_with('"') {
        true => rest[1..].split('"').next().map(str::to_string),
        false => rest.split('(').next().map(|s| s.trim().to_string()),
    }
}

/// A qualified name, or one carrying a naming sigil, needs LLVM's quoted-
/// identifier form. One list, because a second copy drifts and what it costs
/// is a module clang refuses to read.
fn quoted(name: &str) -> String {
    match name.contains(['/', '!', '?', '+', '-', '*', '%', '<', '>', '=']) {
        true => format!("\"{name}\""),
        false => name.to_string(),
    }
}

fn trampoline_name(name: &str) -> String {
    quoted(&format!("{name}.c"))
}

/// How many argument registers a parameter of this type occupies.
fn register_width(ty: &str) -> usize {
    match ty {
        "%KValue" | "%parsed" => 2,
        _ => 1,
    }
}
