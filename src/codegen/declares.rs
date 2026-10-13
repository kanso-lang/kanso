// The runtime's declarations and inline definitions that every emitted
// module carries, and the tables built from them when the compiler is
// built. codegen.rs reads this file with include!, so these items stay in
// the codegen module and the compiler builds to the same binary it did
// when they sat in codegen.rs (kanso#1848). cargo fmt does not reach an
// included file, so CI's format job checks the files under src/'s
// subdirectories itself.

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
  %capa = and i64 %cap, -2
  %owned = icmp ne i64 %cap, 0
  br i1 %owned, label %fr, label %slow
fr:
  %usedp = getelementptr i8, ptr %data, i64 -8
  %used = load i64, ptr %usedp
  %atfront = icmp eq i64 %used, %len
  br i1 %atfront, label %roomat, label %slow
roomat:
  %len1 = add i64 %len, 1
  %fits = icmp sle i64 %len1, %capa
  br i1 %fits, label %claim, label %slow
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
  %capp = getelementptr i8, ptr %b, i64 16
  %cap = load i64, ptr %capp
  %capa = and i64 %cap, -2
  %len1 = add i64 %len, 1
  %fits = icmp sle i64 %len1, %capa
  br i1 %fits, label %bfr, label %slow
bfr:
  %datap = getelementptr i8, ptr %b, i64 8
  %data = load ptr, ptr %datap
  %usedp = getelementptr i8, ptr %data, i64 -8
  %used = load i64, ptr %usedp
  %atfront = icmp eq i64 %used, %len
  br i1 %atfront, label %bwrite, label %slow
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
  %scapa = and i64 %scap, -2
  %sowned = icmp ne i64 %scap, 0
  br i1 %sowned, label %sfr, label %slow
sfr:
  %susedp = getelementptr i8, ptr %sadata, i64 -8
  %sused = load i64, ptr %susedp
  %satfront = icmp eq i64 %sused, %slen
  br i1 %satfront, label %sroomat, label %slow
sroomat:
  %slenn = add i64 %slen, %n
  %sfits = icmp sle i64 %slenn, %scapa
  br i1 %sfits, label %swrite, label %slow
; The copy. A key, a `true` or a `null` is a handful of bytes, and a call into
; glibc's memcpy spends most of its instructions deciding how wide a move to
; make before it makes one. Sixteen bytes or fewer are copied here as a pair of
; overlapping loads, which reads and writes each end once and needs no loop and
; no count: 7,670,800 of these in encodebench and 3,480,400 more from the escape
; path. Anything longer keeps the call, where the vector path earns its entry.
swrite:
  %sdst = getelementptr i8, ptr %sadata, i64 %slen
  %ssmall = icmp ult i64 %n, 17
  br i1 %ssmall, label %sw16, label %swbig
swbig:
  call void @llvm.memcpy.p0.p0.i64(ptr %sdst, ptr %sdata, i64 %n, i1 false)
  br label %swdone
sw16:
  %sge8 = icmp ugt i64 %n, 7
  br i1 %sge8, label %sw8, label %sw7
sw8:
  %sw8a = load i64, ptr %sdata, align 1
  %sw8se = getelementptr i8, ptr %sdata, i64 %n
  %sw8sp = getelementptr i8, ptr %sw8se, i64 -8
  %sw8b = load i64, ptr %sw8sp, align 1
  store i64 %sw8a, ptr %sdst, align 1
  %sw8de = getelementptr i8, ptr %sdst, i64 %n
  %sw8dp = getelementptr i8, ptr %sw8de, i64 -8
  store i64 %sw8b, ptr %sw8dp, align 1
  br label %swdone
sw7:
  %sge4 = icmp ugt i64 %n, 3
  br i1 %sge4, label %sw4, label %sw3
sw4:
  %sw4a = load i32, ptr %sdata, align 1
  %sw4se = getelementptr i8, ptr %sdata, i64 %n
  %sw4sp = getelementptr i8, ptr %sw4se, i64 -4
  %sw4b = load i32, ptr %sw4sp, align 1
  store i32 %sw4a, ptr %sdst, align 1
  %sw4de = getelementptr i8, ptr %sdst, i64 %n
  %sw4dp = getelementptr i8, ptr %sw4de, i64 -4
  store i32 %sw4b, ptr %sw4dp, align 1
  br label %swdone
sw3:
  %sge1 = icmp ugt i64 %n, 0
  br i1 %sge1, label %sw1, label %swdone
sw1:
  %sw1a = load i8, ptr %sdata, align 1
  store i8 %sw1a, ptr %sdst, align 1
  %swmid = lshr i64 %n, 1
  %sw1sm = getelementptr i8, ptr %sdata, i64 %swmid
  %sw1b = load i8, ptr %sw1sm, align 1
  %sw1dm = getelementptr i8, ptr %sdst, i64 %swmid
  store i8 %sw1b, ptr %sw1dm, align 1
  %swlast = add i64 %n, -1
  %sw1sl = getelementptr i8, ptr %sdata, i64 %swlast
  %sw1c = load i8, ptr %sw1sl, align 1
  %sw1dl = getelementptr i8, ptr %sdst, i64 %swlast
  store i8 %sw1c, ptr %sw1dl, align 1
  br label %swdone
swdone:
  store i64 %slenn, ptr %susedp
  store i64 %slenn, ptr %sb
  ret %KValue %acc

slow:
  %f = call %KValue @k_b_append_mut(%KValue %acc, %KValue %x)
  ret %KValue %f
}
; The same claim again where the SETS already say what those two tag tests ask.
; `esc_byte` dispatches on the byte before it appends it, so by the time the
; twin above opens with `atag == 13` and `xtag == 0` both answers are already
; decided on the path that got here -- and LLVM will not thread them away,
; because the block is a merge that other predecessors reach with other tags.
; Four instructions on 9,833,200 of encodebench's 11,658,800 escape-fold
; iterations. Everything past the tags is the byte arm above, unchanged: the
; stats flag, the ownership test and the frontier test still decide, and
; anything they refuse still falls to the C.
define internal %KValue @k_b_append_mut_int(%KValue %acc, %KValue %x) alwaysinline {
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
  %capa = and i64 %cap, -2
  %owned = icmp ne i64 %cap, 0
  br i1 %owned, label %bfr, label %slow
bfr:
  %usedp = getelementptr i8, ptr %data, i64 -8
  %used = load i64, ptr %usedp
  %atfront = icmp eq i64 %used, %len
  br i1 %atfront, label %roomat, label %slow
roomat:
  %len1 = add i64 %len, 1
  %fits = icmp sle i64 %len1, %capa
  br i1 %fits, label %bwrite, label %slow
bwrite:
  %dst = getelementptr i8, ptr %data, i64 %len
  %xv = extractvalue %KValue %x, 1
  %byte = trunc i64 %xv to i8
  store i8 %byte, ptr %dst
  store i64 %len1, ptr %usedp
  store i64 %len1, ptr %b
  ret %KValue %acc
slow:
  %f = call %KValue @k_b_append_mut(%KValue %acc, %KValue %x)
  ret %KValue %f
}
; Two bytes appended one after the other to a builder this function owns, the
; second onto the first's result and nothing else reading that result between
; them: `text/append (text/append acc 92) b`, which is every escape the json
; encoder writes. The pair asks for room once and writes both bytes. Anything
; the fast path refuses runs the two single appends in order, as written.
define internal %KValue @k_b_append_mut_int2(%KValue %acc, %KValue %x, %KValue %y) alwaysinline {
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
  %capa = and i64 %cap, -2
  %owned = icmp ne i64 %cap, 0
  br i1 %owned, label %fr, label %slow
fr:
  %usedp = getelementptr i8, ptr %data, i64 -8
  %used = load i64, ptr %usedp
  %atfront = icmp eq i64 %used, %len
  br i1 %atfront, label %roomat, label %slow
roomat:
  %len2 = add i64 %len, 2
  %fits = icmp sle i64 %len2, %capa
  br i1 %fits, label %write, label %slow
write:
  %dst = getelementptr i8, ptr %data, i64 %len
  %xv = extractvalue %KValue %x, 1
  %xb = trunc i64 %xv to i8
  store i8 %xb, ptr %dst
  %dst1 = getelementptr i8, ptr %dst, i64 1
  %yv = extractvalue %KValue %y, 1
  %yb = trunc i64 %yv to i8
  store i8 %yb, ptr %dst1
  store i64 %len2, ptr %usedp
  store i64 %len2, ptr %b
  ret %KValue %acc
slow:
  %t = call %KValue @k_b_append_mut_int(%KValue %acc, %KValue %x)
  %r = call %KValue @k_b_append_mut_int(%KValue %t, %KValue %y)
  ret %KValue %r
}
; A string literal of one to eight bytes appended to a builder this function
; owns: `text/append acc "true"`, which is every `true`, `false` and `null` the
; json encoder writes. The literal is one word known when the program was
; compiled, so it is stored whole, where the string arm above loads the
; literal's cell, reads its header and copies a variable length. The bytes the
; word carries past the literal land in room the builder already owns and past
; its length, where nothing reads them. Anything the fast path refuses builds
; the literal and takes the string arm, so the counting build sees the append
; it always saw.
define internal %KValue @k_b_append_mut_word(%KValue %acc, i64 %word, i64 %n, ptr %lit, ptr %cell) alwaysinline {
  %atag = extractvalue %KValue %acc, 0
  %isb = icmp eq i64 %atag, 13
  br i1 %isb, label %stat, label %slow
stat:
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
  %capa = and i64 %cap, -2
  %owned = icmp ne i64 %cap, 0
  br i1 %owned, label %fr, label %slow
fr:
  %usedp = getelementptr i8, ptr %data, i64 -8
  %used = load i64, ptr %usedp
  %atfront = icmp eq i64 %used, %len
  br i1 %atfront, label %roomat, label %slow
roomat:
  %len8 = add i64 %len, 8
  %fits = icmp sle i64 %len8, %capa
  br i1 %fits, label %write, label %slow
write:
  %dst = getelementptr i8, ptr %data, i64 %len
  store i64 %word, ptr %dst, align 1
  %lenn = add i64 %len, %n
  store i64 %lenn, ptr %usedp
  store i64 %lenn, ptr %b
  ret %KValue %acc
slow:
  %f = call %KValue @k_b_append_word_slow(%KValue %acc, ptr %lit, i64 %n, ptr %cell)
  ret %KValue %f
}
; `append acc (slice cs from to)` where the accumulator is unique, both sides
; are bytes and the range fits the spare capacity it already has. That is the
; whole of what the json decoder's escape walk does between two escapes, and
; the out-of-line pair it used to reach — a call into `append_slice` and a
; second into `append_range` — cost more than the byte-at-a-time walk it
; replaced. The range is a pointer and a length here, so the claim is the same
; five loads the byte arm above makes and the copy is the same small ladder.
; Anything else — a growth, a counted run, an empty or out-of-range slice, a
; non-unique accumulator — falls to the C, which is the only spelling of the
; rule.
define internal %KValue @k_b_append_slice_fast(%KValue %acc, %KValue %cs, %KValue %fv, %KValue %tv, i64 %mut) alwaysinline {
  %ismut = icmp eq i64 %mut, 1
  br i1 %ismut, label %qtags, label %qslow
qtags:
  %qat = extractvalue %KValue %acc, 0
  %qab = icmp eq i64 %qat, 13
  %qct = extractvalue %KValue %cs, 0
  %qcb = icmp eq i64 %qct, 13
  %qft = extractvalue %KValue %fv, 0
  %qfi = icmp eq i64 %qft, 0
  %qtt = extractvalue %KValue %tv, 0
  %qti = icmp eq i64 %qtt, 0
  %qt1 = and i1 %qab, %qcb
  %qt2 = and i1 %qfi, %qti
  %qt3 = and i1 %qt1, %qt2
  br i1 %qt3, label %qstat, label %qslow
qstat:
  %qso = load i32, ptr @k_stats_on
  %qcount = icmp ne i32 %qso, 0
  br i1 %qcount, label %qslow, label %qrange
; An out-of-range or inverted range is the empty slice, which appends nothing —
; k_b_slice_raw's rule, and the reason it is answered here rather than sent to
; the C: a fifth of the runs between two escapes are empty, because two escapes
; sitting next to each other leave no bytes between them. Such a run returns
; the accumulator as it stands, from between the two bounds tests, rather than
; carrying a zero length through the claim and the copy.
qrange:
  %qfrom = extractvalue %KValue %fv, 1
  %qto = extractvalue %KValue %tv, 1
  %qcp = extractvalue %KValue %cs, 1
  %qc = inttoptr i64 %qcp to ptr
  %qoff = add i64 %qfrom, -1
  %qorder = icmp ult i64 %qoff, %qto
  br i1 %qorder, label %qspanhi, label %qempty
qempty:
  ret %KValue %acc
qspanhi:
  %qclen = load i64, ptr %qc
  %qhi = icmp ule i64 %qto, %qclen
  br i1 %qhi, label %qspan, label %qempty
qspan:
  %qcdp = getelementptr i8, ptr %qc, i64 8
  %qcdata = load ptr, ptr %qcdp
  %qsrc = getelementptr i8, ptr %qcdata, i64 %qoff
  %qn = sub i64 %qto, %qoff
  %qbp = extractvalue %KValue %acc, 1
  %qb = inttoptr i64 %qbp to ptr
  %qlen = load i64, ptr %qb
  %qadp = getelementptr i8, ptr %qb, i64 8
  %qadata = load ptr, ptr %qadp
  %qcapp = getelementptr i8, ptr %qb, i64 16
  %qcap = load i64, ptr %qcapp
  %qcapa = and i64 %qcap, -2
  %qowned = icmp ne i64 %qcap, 0
  br i1 %qowned, label %qfr, label %qslow
qfr:
  %qusedp = getelementptr i8, ptr %qadata, i64 -8
  %qused = load i64, ptr %qusedp
  %qatfront = icmp eq i64 %qused, %qlen
  br i1 %qatfront, label %qroomat, label %qslow
qroomat:
  %qlenn = add i64 %qlen, %qn
  %qfits = icmp sle i64 %qlenn, %qcapa
  br i1 %qfits, label %qwrite, label %qslow
; The same ladder the string arm uses, for the same reason: a run between two
; escapes is a median of three bytes, and a call into glibc's memcpy spends
; most of its instructions deciding how wide a move to make.
qwrite:
  %qdst = getelementptr i8, ptr %qadata, i64 %qlen
  %qsmall = icmp ult i64 %qn, 17
  br i1 %qsmall, label %qw16, label %qwbig
qwbig:
  call void @llvm.memcpy.p0.p0.i64(ptr %qdst, ptr %qsrc, i64 %qn, i1 false)
  br label %qwdone
qw16:
  %qge8 = icmp ugt i64 %qn, 7
  br i1 %qge8, label %qw8, label %qw7
qw8:
  %qw8a = load i64, ptr %qsrc, align 1
  %qw8se = getelementptr i8, ptr %qsrc, i64 %qn
  %qw8sp = getelementptr i8, ptr %qw8se, i64 -8
  %qw8b = load i64, ptr %qw8sp, align 1
  store i64 %qw8a, ptr %qdst, align 1
  %qw8de = getelementptr i8, ptr %qdst, i64 %qn
  %qw8dp = getelementptr i8, ptr %qw8de, i64 -8
  store i64 %qw8b, ptr %qw8dp, align 1
  br label %qwdone
qw7:
  %qge4 = icmp ugt i64 %qn, 3
  br i1 %qge4, label %qw4, label %qw3
qw4:
  %qw4a = load i32, ptr %qsrc, align 1
  %qw4se = getelementptr i8, ptr %qsrc, i64 %qn
  %qw4sp = getelementptr i8, ptr %qw4se, i64 -4
  %qw4b = load i32, ptr %qw4sp, align 1
  store i32 %qw4a, ptr %qdst, align 1
  %qw4de = getelementptr i8, ptr %qdst, i64 %qn
  %qw4dp = getelementptr i8, ptr %qw4de, i64 -4
  store i32 %qw4b, ptr %qw4dp, align 1
  br label %qwdone
qw3:
  %qge1 = icmp ugt i64 %qn, 0
  br i1 %qge1, label %qw1, label %qwdone
qw1:
  %qw1a = load i8, ptr %qsrc, align 1
  store i8 %qw1a, ptr %qdst, align 1
  %qwmid = lshr i64 %qn, 1
  %qw1sm = getelementptr i8, ptr %qsrc, i64 %qwmid
  %qw1b = load i8, ptr %qw1sm, align 1
  %qw1dm = getelementptr i8, ptr %qdst, i64 %qwmid
  store i8 %qw1b, ptr %qw1dm, align 1
  %qwlast = add i64 %qn, -1
  %qw1sl = getelementptr i8, ptr %qsrc, i64 %qwlast
  %qw1c = load i8, ptr %qw1sl, align 1
  %qw1dl = getelementptr i8, ptr %qdst, i64 %qwlast
  store i8 %qw1c, ptr %qw1dl, align 1
  br label %qwdone
qwdone:
  store i64 %qlenn, ptr %qusedp
  store i64 %qlenn, ptr %qb
  ret %KValue %acc
qslow:
  %qf = call %KValue @k_b_append_slice(%KValue %acc, %KValue %cs, %KValue %fv, %KValue %tv, i64 %mut)
  ret %KValue %qf
}
define internal %KValue @k_b_utf8_slice_fast(%KValue %c, %KValue %f, %KValue %t, ptr %o) alwaysinline {
  %ct = extractvalue %KValue %c, 0
  %ft = extractvalue %KValue %f, 0
  %tt = extractvalue %KValue %t, 0
  %isb = icmp eq i64 %ct, 13
  %isf = icmp eq i64 %ft, 0
  %ist = icmp eq i64 %tt, 0
  %g1 = and i1 %isb, %isf
  %plain = and i1 %g1, %ist
  br i1 %plain, label %raw, label %slow
raw:
  %bp = extractvalue %KValue %c, 1
  %by = inttoptr i64 %bp to ptr
  %blen = load i64, ptr %by
  %dp = getelementptr i8, ptr %by, i64 8
  %d = load ptr, ptr %dp
  %fv = extractvalue %KValue %f, 1
  %tv = extractvalue %KValue %t, 1
  %r = call %KValue @k_b_utf8_slice_raw(ptr %d, i64 %blen, i64 %fv, i64 %tv, ptr %o)
  ret %KValue %r
slow:
  %s = call %KValue @k_b_utf8_slice(%KValue %c, %KValue %f, %KValue %t, ptr %o)
  ret %KValue %s
}
define internal %KValue @k_b_slice_fast(%KValue %c, %KValue %f, %KValue %t) alwaysinline {
  %ct = extractvalue %KValue %c, 0
  %ft = extractvalue %KValue %f, 0
  %tt = extractvalue %KValue %t, 0
  %isb = icmp eq i64 %ct, 13
  %isf = icmp eq i64 %ft, 0
  %ist = icmp eq i64 %tt, 0
  %g1 = and i1 %isb, %isf
  %plain = and i1 %g1, %ist
  br i1 %plain, label %view, label %slow
view:
  %bp = extractvalue %KValue %c, 1
  %by = inttoptr i64 %bp to ptr
  %len = load i64, ptr %by
  %dp = getelementptr i8, ptr %by, i64 8
  %d = load ptr, ptr %dp
  %fv = extractvalue %KValue %f, 1
  %tv = extractvalue %KValue %t, 1
  %r = call %KValue @k_b_slice_raw(ptr %d, i64 %len, i64 %fv, i64 %tv)
  ret %KValue %r
slow:
  %s = call %KValue @k_b_slice(%KValue %c, %KValue %f, %KValue %t)
  ret %KValue %s
}
define internal %KValue @k_b_find2_below_fast(%KValue %cs, %KValue %from, %KValue %a, %KValue %b, %KValue %lim) alwaysinline {
  %cst = extractvalue %KValue %cs, 0
  %ft = extractvalue %KValue %from, 0
  %at = extractvalue %KValue %a, 0
  %bt = extractvalue %KValue %b, 0
  %lt = extractvalue %KValue %lim, 0
  %isb = icmp eq i64 %cst, 13
  %isf = icmp eq i64 %ft, 0
  %isa = icmp eq i64 %at, 0
  %isbb = icmp eq i64 %bt, 0
  %isl = icmp eq i64 %lt, 0
  %g1 = and i1 %isb, %isf
  %g2 = and i1 %isa, %isbb
  %g3 = and i1 %g1, %g2
  %plain = and i1 %g3, %isl
  br i1 %plain, label %scan, label %slow
scan:
  %bp = extractvalue %KValue %cs, 1
  %by = inttoptr i64 %bp to ptr
  %len = load i64, ptr %by
  %dp = getelementptr i8, ptr %by, i64 8
  %d = load ptr, ptr %dp
  %fv = extractvalue %KValue %from, 1
  %av = extractvalue %KValue %a, 1
  %bv = extractvalue %KValue %b, 1
  %lv = extractvalue %KValue %lim, 1
  %at1 = call i64 @k_b_find2_below_raw(ptr %d, i64 %len, i64 %fv, i64 %av, i64 %bv, i64 %lv)
  %r = insertvalue %KValue { i64 0, i64 undef }, i64 %at1, 1
  ret %KValue %r
slow:
  %f = call %KValue @k_b_find2_below(%KValue %cs, %KValue %from, %KValue %a, %KValue %b, %KValue %lim)
  ret %KValue %f
}
define internal %KValue @k_b_find2_fast(%KValue %cs, %KValue %from, %KValue %a, %KValue %b) alwaysinline {
  %cst = extractvalue %KValue %cs, 0
  %ft = extractvalue %KValue %from, 0
  %at = extractvalue %KValue %a, 0
  %bt = extractvalue %KValue %b, 0
  %isb = icmp eq i64 %cst, 13
  %isf = icmp eq i64 %ft, 0
  %isa = icmp eq i64 %at, 0
  %isbb = icmp eq i64 %bt, 0
  %g1 = and i1 %isb, %isf
  %g2 = and i1 %isa, %isbb
  %plain = and i1 %g1, %g2
  br i1 %plain, label %scan, label %slow
scan:
  %bp = extractvalue %KValue %cs, 1
  %by = inttoptr i64 %bp to ptr
  %len = load i64, ptr %by
  %dp = getelementptr i8, ptr %by, i64 8
  %d = load ptr, ptr %dp
  %fv = extractvalue %KValue %from, 1
  %av = extractvalue %KValue %a, 1
  %bv = extractvalue %KValue %b, 1
  %at1 = call i64 @k_b_find2_raw(ptr %d, i64 %len, i64 %fv, i64 %av, i64 %bv)
  %r = insertvalue %KValue { i64 0, i64 undef }, i64 %at1, 1
  ret %KValue %r
slow:
  %f = call %KValue @k_b_find2(%KValue %cs, %KValue %from, %KValue %a, %KValue %b)
  ret %KValue %f
}
define internal %KValue @k_str_lit_fast(ptr %data, i64 %len, ptr %slot) alwaysinline {
  %tag = load i64, ptr %slot
  %built = icmp eq i64 %tag, 6
  br i1 %built, label %hit, label %miss
hit:
  %v = load %KValue, ptr %slot
  ret %KValue %v
miss:
  %f = call %KValue @k_str_lit(ptr %data, i64 %len, ptr %slot)
  ret %KValue %f
}
define internal %KValue @k_b_length_fast(%KValue %v) alwaysinline {
  %tag = extractvalue %KValue %v, 0
  ; list (9) or bytes (13) in one compare; three compares on %tag lower as a chain
  %t4 = or i64 %tag, 4
  %fastable = icmp eq i64 %t4, 13
  br i1 %fastable, label %list, label %str
list:
  %p = extractvalue %KValue %v, 1
  %lp = inttoptr i64 %p to ptr
  %len = load i64, ptr %lp
  %r = insertvalue %KValue { i64 0, i64 undef }, i64 %len, 1
  ret %KValue %r
str:
  %is_str = icmp eq i64 %tag, 6
  br i1 %is_str, label %memo, label %slow
memo:
  %sp = extractvalue %KValue %v, 1
  %sptr = inttoptr i64 %sp to ptr
  %cp = getelementptr i8, ptr %sptr, i64 12
  %cap = load i32, ptr %cp
  %known = icmp slt i32 %cap, 0
  br i1 %known, label %count, label %slow
count:
  %ncap = xor i32 %cap, -1
  %n = sext i32 %ncap to i64
  %rs = insertvalue %KValue { i64 0, i64 undef }, i64 %n, 1
  ret %KValue %rs
slow:
  %f = call %KValue @k_b_length(%KValue %v)
  ret %KValue %f
}
; The bytes view of a string. `k_b_bytes` reads two fields out of the KStr and
; writes a three-field header into the arena, and the arena bump is already
; inline in k_b_append_byte above -- so all thirty instructions it cost were a
; call, a tag ladder and a bump the emitter can write itself. One comparison
; does the whole guard: a failure carries a tag that is not K_STR and goes to
; %slow, where the C entry owns the message and the propagation exactly as
; before. The counting path goes to %slow too, so k_stat_sh_bytes can never be
; dropped -- the same arrangement k_b_append_byte uses, for the same reason.
; KStr keeps its length in an i32, so the load is sign-extended; a view's cap
; is zero, which is what marks the data as borrowed rather than owned.
define internal %KValue @k_b_bytes_fast(%KValue %sv) alwaysinline {
  %tag = extractvalue %KValue %sv, 0
  %isstr = icmp eq i64 %tag, 6
  br i1 %isstr, label %chks, label %slow
chks:
  %so = load i32, ptr @k_stats_on
  %counting = icmp ne i32 %so, 0
  br i1 %counting, label %slow, label %fast
fast:
  %left = load i64, ptr @k_arena_left
  %has = icmp uge i64 %left, 32
  br i1 %has, label %alloc, label %slow
alloc:
  %sp = extractvalue %KValue %sv, 1
  %s = inttoptr i64 %sp to ptr
  %data = load ptr, ptr %s
  %lenp = getelementptr i8, ptr %s, i64 8
  %len32 = load i32, ptr %lenp
  %len = sext i32 %len32 to i64
  %ar = load ptr, ptr @k_arena
  %ar2 = getelementptr i8, ptr %ar, i64 32
  store ptr %ar2, ptr @k_arena
  %left2 = sub i64 %left, 32
  store i64 %left2, ptr @k_arena_left
  store i64 %len, ptr %ar
  %hd = getelementptr i8, ptr %ar, i64 8
  store ptr %data, ptr %hd
  %hc = getelementptr i8, ptr %ar, i64 16
  store i64 0, ptr %hc
  %pi = ptrtoint ptr %ar to i64
  %r0 = insertvalue %KValue { i64 13, i64 undef }, i64 %pi, 1
  ret %KValue %r0
slow:
  %f = call %KValue @k_b_bytes(%KValue %sv)
  ret %KValue %f
}
; A proven string's view with its header at %hdr: `framed_views` says why.
define internal %KValue @k_b_bytes_frame(%KValue %sv, ptr %hdr) alwaysinline {
  %sp = extractvalue %KValue %sv, 1
  %s = inttoptr i64 %sp to ptr
  %data = load ptr, ptr %s
  %lenp = getelementptr i8, ptr %s, i64 8
  %len32 = load i32, ptr %lenp
  %len = sext i32 %len32 to i64
  store i64 %len, ptr %hdr
  %hd = getelementptr i8, ptr %hdr, i64 8
  store ptr %data, ptr %hd
  %hc = getelementptr i8, ptr %hdr, i64 16
  store i64 0, ptr %hc
  %pi = ptrtoint ptr %hdr to i64
  %r0 = insertvalue %KValue { i64 13, i64 undef }, i64 %pi, 1
  ret %KValue %r0
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
  %lenok = icmp sge i64 %len, 0
  call void @llvm.assume(i1 %lenok)
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
; here: on the frontier with room, claim the slot and bump both lengths. An
; err item goes to the C, which answers the err: pushing one hands it on, as
; every builtin does. A bignum goes to the C as well, which moves it out of
; the arena when the list outlives the loop's rewind. Where the emitter's sets
; prove the item is neither -- a field a pattern bound, a literal, arithmetic
; that cannot fail or widen -- it calls `k_b_push_mut_known` directly and the
; item test is not written at all.
define internal %KValue @k_b_push_mut_fast(%KValue %lv, %KValue %item) alwaysinline {
  %itag = extractvalue %KValue %item, 0
  %ierr = icmp eq i64 %itag, 5
  %ibig = icmp eq i64 %itag, 17
  %islow = or i1 %ierr, %ibig
  br i1 %islow, label %eslow, label %eknown
eknown:
  %kr = call %KValue @k_b_push_mut_known(%KValue %lv, %KValue %item)
  ret %KValue %kr
eslow:
  %er = call %KValue @k_b_push_mut(%KValue %lv, %KValue %item)
  ret %KValue %er
}
define internal %KValue @k_b_push_mut_known(%KValue %lv, %KValue %item) alwaysinline {
  %ltag = extractvalue %KValue %lv, 0
  %islist = icmp eq i64 %ltag, 9
  br i1 %islist, label %lstat, label %lslow
lstat:
  %lso = load i32, ptr @k_stats_on
  %lcounting = icmp ne i32 %lso, 0
  br i1 %lcounting, label %lslow, label %litem
litem:
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
  br i1 %lfront, label %lroom, label %lslow
lroom:
  %llen2 = shl i64 %llen, 1
  %lneed = add i64 %llen2, 2
  %lfits = icmp sle i64 %lneed, %lcap
  br i1 %lfits, label %lwrite, label %lslow
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
; in any of the three, a bignum value -- takes the call.
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
  %verr = icmp eq i64 %tv, 5
  %vbig = icmp eq i64 %tv, 17
  %vbad = or i1 %verr, %vbig
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
  br i1 %pfront, label %pfit, label %pslow
pfit:
  %pneed = add i64 %mlen2, 2
  %pneed2 = shl i64 %pneed, 1
  %pfits = icmp sle i64 %pneed2, %pcap
  br i1 %pfits, label %pwrite, label %pslow
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
  %lenok = icmp sge i64 %len, 0
  call void @llvm.assume(i1 %lenok)
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
define internal i64 @k_truthy_w(i64 %tag, i64 %pay) alwaysinline {
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
  %v0 = insertvalue %KValue undef, i64 %tag, 0
  %v = insertvalue %KValue %v0, i64 %pay, 1
  %r = call i64 @k_truthy_bad(%KValue %v)
  ret i64 %r
}
define internal i64 @k_check_rec_fast_w(i64 %tag, i64 %pay, i64 %t, i64 %n) alwaysinline {
  %issub = icmp eq i64 %tag, 15
  br i1 %issub, label %slow, label %plain
plain:
  %isrec = icmp eq i64 %tag, 7
  br i1 %isrec, label %rec, label %no
no:
  ret i64 0
rec:
  %r = inttoptr i64 %pay to ptr
  %tid = load i64, ptr %r
  %np = getelementptr i8, ptr %r, i64 8
  %nf = load i64, ptr %np
  %et = icmp eq i64 %tid, %t
  %en = icmp eq i64 %nf, %n
  %both = and i1 %et, %en
  %out = zext i1 %both to i64
  ret i64 %out
slow:
  %v0 = insertvalue %KValue undef, i64 %tag, 0
  %v = insertvalue %KValue %v0, i64 %pay, 1
  %s = call i64 @k_check_rec(%KValue %v, i64 %t, i64 %n)
  ret i64 %s
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
declare %KValue @k_b_effect(%KValue)
declare %KValue @k_err_hop(%KValue, ptr)
declare void @llvm.assume(i1 noundef)
attributes #90 = { noinline }
declare %KValue @k_unsub(%KValue)
declare %KValue @k_rec(i64, i64, ptr)
declare %KValue @k_pair_failure(%KValue, %KValue)
declare %KValue @k_rec_reuse(i64, i64, ptr, %KValue)
declare %KValue @k_parsed_box(i64, i64, i64)
declare %KValue @k_parsed_words(%KValue)
declare %KValue @k_parsed_spill(i64, i64, i64, i64, i64) cold
declare %KValue @k_parsed_wide_first(i64) cold
declare %KValue @k_parsed_wide_value(i64) cold
declare %KValue @k_concat_arr_mut(i64, ptr)
declare %KValue @k_b_str_builder(%KValue)
declare %KValue @k_field(%KValue, i64)
declare %KValue @k_keyed_check(%KValue, i64)
declare %KValue @k_keyed_field(%KValue, ptr)
declare %KValue @k_b_field(%KValue, ptr)
declare void @k_no_field(%KValue, ptr)
declare %KValue @k_field_forced(%KValue, ptr)
declare %KValue @k_err_read(%KValue, ptr)
declare i64 @k_is_err(%KValue)
declare i64 @k_check_some(%KValue)
declare %KValue @k_err_inner(%KValue)
declare i64 @k_check_rec(%KValue, i64, i64)
declare i64 @k_check_str(%KValue, ptr, i64)
declare %KValue @k_concat(%KValue, %KValue)
declare %KValue @k_concat_arr(i64, ptr)
declare %KValue @k_render(%KValue, i64)
declare i64 @k_render_dispatchable(%KValue)
declare i64 @k_routes_to_arms(%KValue)
declare %KValue @k_add(%KValue, %KValue)
declare %KValue @k_int_add(%KValue, %KValue)
declare %KValue @k_int_sub(%KValue, %KValue)
declare %KValue @k_int_mul(%KValue, %KValue)
declare %KValue @k_int_of_digits(ptr, i64)
declare i64 @k_check_int_type(%KValue)
declare i64 @k_check_big_lit(%KValue, ptr, i64)
declare void @k_big_lift_out(ptr, i64)
declare void @k_big_lift_in(ptr, i64)
declare %KValue @k_big_lift_out1(%KValue)
declare %KValue @k_big_lift_in1(%KValue)
declare %KValue @k_sub(%KValue, %KValue)
declare %KValue @k_mul(%KValue, %KValue)
declare %KValue @k_div(%KValue, %KValue, ptr)
declare %KValue @k_mod(%KValue, %KValue, ptr)
declare %KValue @k_cmp(%KValue, %KValue, i64)
declare %KValue @k_desc_print(%KValue)
declare void @k_die(ptr) noreturn
declare void @k_die_arity(i64, i64) noreturn
declare void @k_die_overload(ptr) noreturn
declare void @k_die_destructure(%KValue, ptr) noreturn
declare { i64, i1 } @llvm.sadd.with.overflow.i64(i64, i64)
declare { i64, i1 } @llvm.ssub.with.overflow.i64(i64, i64)
declare { i64, i1 } @llvm.smul.with.overflow.i64(i64, i64)
declare %KValue @k_list_lit(i64, ptr)
declare %KValue @k_map_lit(i64, ptr)
declare %KValue @k_list_empty()
declare %KValue @k_map_empty()
declare %KValue @k_closure(ptr, i64, i64, ptr)
declare %KValue @k_closure_lit(ptr, i64, ptr)
declare %KValue @k_fnref(ptr)
declare %KValue @k_env_get(ptr, i64)
declare %KValue @k_b_at(%KValue, %KValue)
declare %KValue @k_b_is_desc(%KValue)
declare %KValue @k_index(%KValue, %KValue, ptr)
declare %KValue @k_b_bytes(%KValue)
declare %KValue @k_b_bytes_seed()
declare %KValue @k_b_chars(%KValue)
declare %KValue @k_b_split(%KValue, %KValue)
declare %KValue @k_b_concat(%KValue, %KValue)
declare %KValue @k_b_concat_mut(%KValue, %KValue)
declare %KValue @k_b_utf8(%KValue, ptr)
declare %KValue @k_b_built_text(%KValue)
declare %KValue @k_desc_args()
declare %KValue @k_desc_stdin()
declare %KValue @k_b_read_file(%KValue)
declare %KValue @k_b_read_bytes(%KValue)
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
declare %KValue @k_b_rescue(%KValue, %KValue, ptr)
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
declare void @k_beat_lap_carry()
declare %KValue @k_beat_pop(%KValue)
declare %KValue @k_cohort_pop(%KValue)
declare %KValue @k_region_pop(%KValue)
declare %KValue @k_call0(%KValue)
declare %KValue @k_call1(%KValue, %KValue)
declare %KValue @k_call2(%KValue, %KValue, %KValue)
declare %KValue @k_call3(%KValue, %KValue, %KValue, %KValue)
declare %KValue @k_call4(%KValue, %KValue, %KValue, %KValue, %KValue)
declare %KValue @k_partial0(%KValue)
declare %KValue @k_partial1(%KValue, %KValue)
declare %KValue @k_partial2(%KValue, %KValue, %KValue)
declare %KValue @k_partial3(%KValue, %KValue, %KValue, %KValue)
declare %KValue @k_partial4(%KValue, %KValue, %KValue, %KValue, %KValue)
declare %KValue @k_b_char_code(%KValue)
declare %KValue @k_b_entries(%KValue)
declare %KValue @k_b_filter(%KValue, %KValue)
declare %KValue @k_b_from_code(%KValue, ptr)
declare %KValue @k_b_join(%KValue, %KValue)
declare %KValue @k_b_keys(%KValue)
declare %KValue @k_b_values(%KValue)
declare %KValue @k_b_columns(%KValue, ptr)
declare %KValue @k_b_length(%KValue)
declare %KValue @k_b_map(%KValue, %KValue)
declare %KValue @k_b_push(%KValue, %KValue)
declare %KValue @k_b_push_mut(%KValue, %KValue)
declare void @llvm.memcpy.p0.p0.i64(ptr, ptr, i64, i1)
declare %KValue @k_b_append_mut(%KValue, %KValue)
declare %KValue @k_b_append_word_slow(%KValue, ptr, i64, ptr)
declare %KValue @k_b_put(%KValue, %KValue, %KValue)
declare %KValue @k_b_put_mut(%KValue, %KValue, %KValue)
declare %KValue @k_b_slice(%KValue, %KValue, %KValue)
declare %KValue @k_b_slice_raw(ptr, i64, i64, i64)
declare %KValue @k_b_utf8_slice(%KValue, %KValue, %KValue, ptr)
declare %KValue @k_b_utf8_slice_raw(ptr, i64, i64, i64, ptr)
declare %KValue @k_b_find2(%KValue, %KValue, %KValue, %KValue)
declare i64 @k_b_find2_raw(ptr, i64, i64, i64, i64)
declare %KValue @k_b_find2_below(%KValue, %KValue, %KValue, %KValue, %KValue)
declare i64 @k_b_find2_below_raw(ptr, i64, i64, i64, i64, i64)
declare %KValue @k_b_number_span(%KValue, %KValue)
declare %KValue @k_b_append(%KValue, %KValue)
declare %KValue @k_b_append_slice(%KValue, %KValue, %KValue, %KValue, i64)
declare %KValue @k_b_to_int_slice(%KValue, %KValue, %KValue, ptr)
declare %KValue @k_b_to_float_slice(%KValue, %KValue, %KValue, ptr)
declare %KValue @k_b_append_rendered(%KValue, %KValue, i64)
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
declare %KValue @k_b_tie(%KValue, %KValue, %KValue)
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

/// One line of DECLARES, located when the compiler is compiled.
///
/// Every module the emitter writes begins with DECLARES, less the declares its
/// program never calls and, in a build nobody will count, less the stats gates.
/// Doing that at run time meant splitting 1,186 lines, finding each declare's
/// symbol, asking every line whether it opened a gate, and allocating a
/// `String` per kept line to join afterwards: 1.1 million instructions of the
/// 2.7 million `print "x"` cost to build. The text is a constant, so the scan
/// runs in const evaluation and a build reads the finished table.
#[derive(Clone, Copy)]
struct DeclareLine {
    start: usize,
    end: usize,
    /// The symbol a `declare` line names, `sym_start == sym_end` when the line
    /// is not one the program's calls decide.
    sym_start: usize,
    sym_end: usize,
    /// What the line becomes in a build without counters: `Keep`, `Fold` with
    /// the fast path's label, or `Folded` for the two lines a gate folds.
    shipped: Shipped,
    /// Whether DECLARES's own definitions call the symbol, which keeps the
    /// line whatever the program calls. Answered when the compiler is built:
    /// asked at emit time it was a binary search over the context calls for
    /// every `declare` line of every module.
    context: bool,
}

const INLINE_ATTR: &[u8] = b" alwaysinline";

/// The eight inlined fast paths in DECLARES each ask `k_stats_on` before they
/// take the shortcut, because the shortcut bypasses the runtime call that
/// would have counted. A binary nobody is going to count does not need the
/// question: folding the eight gates to a constant and relinking the shipped
/// recipe reads 2,141,315,030 -> 2,115,346,210 on the run program, a fall of
/// 25,968,820 (1.2128%), with `.text` 2,048 bytes smaller and stdout byte for
/// byte the same.
///
/// The gate is exactly three lines wherever it appears, and `index_declares`
/// REFUSES anything else rather than silently leaving one in: a ninth site
/// written a different way turns the compiler's own build red.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Shipped {
    Keep,
    Fold { label_start: usize, label_end: usize },
    Folded,
}

const fn declares_line_count(text: &[u8]) -> usize {
    let mut n = 0;
    let mut i = 0;
    while i < text.len() {
        if text[i] == b'\n' {
            n += 1;
        }
        i += 1;
    }
    match !text.is_empty() && text[text.len() - 1] != b'\n' {
        true => n + 1,
        false => n,
    }
}

/// Where `needle` first occurs in `text[from..to]`, or `to`.
const fn find_in(text: &[u8], from: usize, to: usize, needle: &[u8]) -> usize {
    let mut i = from;
    while i + needle.len() <= to {
        let mut j = 0;
        while j < needle.len() && text[i + j] == needle[j] {
            j += 1;
        }
        if j == needle.len() {
            return i;
        }
        i += 1;
    }
    to
}

const fn starts_at(text: &[u8], at: usize, to: usize, prefix: &[u8]) -> bool {
    at + prefix.len() <= to && find_in(text, at, at + prefix.len(), prefix) == at
}

const fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

const DECLARES_LINES: usize = declares_line_count(DECLARES.as_bytes());

/// Whether `text[from..to]` is one of the symbols DECLARES calls from its own
/// definitions.
const fn context_calls_name(text: &[u8], from: usize, to: usize) -> bool {
    let mut n = 0;
    while n < DECLARES_CONTEXT_CALLS.len() {
        let name = DECLARES_CONTEXT_CALLS[n].as_bytes();
        if name.len() == to - from && find_in(text, from, to, name) == from {
            return true;
        }
        n += 1;
    }
    false
}

/// The table, and the refusals the run-time fold used to make, raised while
/// the compiler compiles: a gate written any other way is a build error.
const fn index_declares(text: &str) -> [DeclareLine; DECLARES_LINES] {
    let text = text.as_bytes();
    let blank = DeclareLine {
        start: 0,
        end: 0,
        sym_start: 0,
        sym_end: 0,
        shipped: Shipped::Keep,
        context: false,
    };
    let mut out = [blank; DECLARES_LINES];
    let mut at = 0;
    let mut n = 0;
    while n < DECLARES_LINES {
        let end = find_in(text, at, text.len(), b"\n");
        let mut line = DeclareLine {
            start: at,
            end,
            sym_start: at,
            sym_end: at,
            shipped: Shipped::Keep,
            context: false,
        };
        if starts_at(text, at, end, b"declare ") {
            let rest = at + b"declare ".len();
            let sigil = find_in(text, rest, end, b"@");
            if sigil < end {
                let paren = find_in(text, sigil + 1, end, b"(");
                if paren < end {
                    line.sym_start = sigil + 1;
                    line.sym_end = paren;
                    line.context = context_calls_name(text, sigil + 1, paren);
                }
            }
        }
        out[n] = line;
        at = end + 1;
        n += 1;
    }
    let mut stripped = 0;
    let mut i = 0;
    while i < DECLARES_LINES {
        let l = out[i];
        if find_in(text, l.start, l.end, b"load i32, ptr @k_stats_on") < l.end {
            assert!(i + 2 < DECLARES_LINES, "a stats gate runs off the end of DECLARES");
            let icmp = out[i + 1];
            let mut icmp_end = icmp.end;
            while icmp_end > icmp.start && is_space(text[icmp_end - 1]) {
                icmp_end -= 1;
            }
            assert!(
                find_in(text, icmp.start, icmp.end, b"= icmp ne i32 ") < icmp.end
                    && icmp_end >= icmp.start + 3
                    && text[icmp_end - 3] == b','
                    && text[icmp_end - 2] == b' '
                    && text[icmp_end - 1] == b'0',
                "the stats gate's second line is not the icmp this expects"
            );
            let br = out[i + 2];
            let mut br_start = br.start;
            while br_start < br.end && is_space(text[br_start]) {
                br_start += 1;
            }
            assert!(
                starts_at(text, br_start, br.end, b"br i1 "),
                "the stats gate's third line is not the br this expects"
            );
            // The last `label %`, as rsplit_once finds it.
            let mut last = br.end;
            let mut k = br.start;
            while k < br.end {
                let hit = find_in(text, k, br.end, b"label %");
                if hit == br.end {
                    break;
                }
                last = hit;
                k = hit + 1;
            }
            assert!(last < br.end, "the stats gate's third line is not the br this expects");
            let mut label_start = last + b"label %".len();
            let mut label_end = br.end;
            while label_start < label_end && is_space(text[label_start]) {
                label_start += 1;
            }
            while label_end > label_start && is_space(text[label_end - 1]) {
                label_end -= 1;
            }
            out[i].shipped = Shipped::Fold { label_start, label_end };
            out[i + 1].shipped = Shipped::Folded;
            out[i + 2].shipped = Shipped::Folded;
            stripped += 1;
            i += 3;
            continue;
        }
        i += 1;
    }
    assert!(
        stripped == STATS_GATE_SITES,
        "the stats gate moved: DECLARES holds a different number of them"
    );
    // `emit` hands only what follows DECLARES to `narrow_tailcc`, which is
    // right while the preamble names no convention for that pass to narrow.
    assert!(
        find_in(text, 0, text.len(), b"tailcc") == text.len(),
        "DECLARES spells tailcc, so emit must narrow it with the rest"
    );
    out
}

static DECLARE_LINES: [DeclareLine; DECLARES_LINES] = index_declares(DECLARES);

/// How many `define` lines of `text` carry the attribute a dev build leaves
/// out.
const fn inline_defines(text: &[u8]) -> usize {
    let mut n = 0;
    let mut at = 0;
    while at < text.len() {
        let end = find_in(text, at, text.len(), b"\n");
        if starts_at(text, at, end, b"define ") && find_in(text, at, end, INLINE_ATTR) < end {
            n += 1;
        }
        at = end + 1;
    }
    n
}

const DECLARES_DEV_LEN: usize =
    DECLARES.len() - INLINE_ATTR.len() * inline_defines(DECLARES.as_bytes());

/// DECLARES with ` alwaysinline` taken off every `define` line, made when the
/// compiler is built. A dev build writes this text rather than DECLARES, so
/// leaving the attribute out costs it nothing per line: a check in the loop
/// below cost `kanso play`'s start-up 10,508 instructions on a one-line
/// program, measured before the text moved here.
static DECLARES_DEV_BYTES: [u8; DECLARES_DEV_LEN] = {
    let text = DECLARES.as_bytes();
    let mut out = [0u8; DECLARES_DEV_LEN];
    let mut at = 0;
    let mut o = 0;
    while at < text.len() {
        let end = find_in(text, at, text.len(), b"\n");
        let cut = match starts_at(text, at, end, b"define ") {
            true => find_in(text, at, end, INLINE_ATTR),
            false => end,
        };
        let mut i = at;
        while i < end {
            if i == cut {
                i += INLINE_ATTR.len();
                continue;
            }
            out[o] = text[i];
            o += 1;
            i += 1;
        }
        if end < text.len() {
            out[o] = b'\n';
            o += 1;
        }
        at = end + 1;
    }
    assert!(o == DECLARES_DEV_LEN, "the dev text is not the length it was sized for");
    out
};

static DECLARES_DEV: &str = match std::str::from_utf8(&DECLARES_DEV_BYTES) {
    Ok(text) => text,
    Err(_) => panic!("the dev text is not utf-8"),
};

static DECLARE_LINES_DEV: [DeclareLine; DECLARES_LINES] = index_declares(DECLARES_DEV);

/// One runtime helper DECLARES defines: where its name sits in the text, the
/// lines from its `define` to its closing brace, and every helper it reaches
/// by calls, itself included, as a bit per helper.
#[derive(Clone, Copy)]
struct Helper {
    sym_start: usize,
    sym_end: usize,
    first: usize,
    last: usize,
    reaches: u64,
}

const fn helper_count(text: &str) -> usize {
    let text = text.as_bytes();
    let mut n = 0;
    let mut at = 0;
    while at < text.len() {
        let end = find_in(text, at, text.len(), b"\n");
        if starts_at(text, at, end, b"define internal ") {
            n += 1;
        }
        at = end + 1;
    }
    n
}

const HELPERS: usize = helper_count(DECLARES);

/// The helpers of `text`, found when the compiler is built. A helper runs
/// from a `define internal` line to the next line that is a lone `}`, and it
/// reaches every helper whose `@name(` its body writes, and theirs in turn.
const fn index_helpers(text: &str) -> [Helper; HELPERS] {
    const { assert!(HELPERS <= 64, "the helpers no longer fit the bit set that tracks them") };
    let bytes = text.as_bytes();
    let blank = Helper { sym_start: 0, sym_end: 0, first: 0, last: 0, reaches: 0 };
    let mut out = [blank; HELPERS];
    let mut n = 0;
    let mut line = 0;
    let mut at = 0;
    let mut open = false;
    while at < bytes.len() {
        let end = find_in(bytes, at, bytes.len(), b"\n");
        if starts_at(bytes, at, end, b"define internal ") {
            let sigil = find_in(bytes, at, end, b"@");
            let paren = find_in(bytes, sigil, end, b"(");
            out[n] = Helper {
                sym_start: sigil + 1,
                sym_end: paren,
                first: line,
                last: line,
                reaches: 1 << n,
            };
            open = true;
        } else if open && end == at + 1 && bytes[at] == b'}' {
            out[n].last = line;
            open = false;
            n += 1;
        }
        at = end + 1;
        line += 1;
    }
    assert!(n == HELPERS && !open, "a helper in DECLARES has no closing brace of its own");
    // Direct calls: every `@name(` in a helper's body that names a helper,
    // read in one pass over the text.
    let mut h = 0;
    let mut at = 0;
    let mut line = 0;
    while at < bytes.len() && h < HELPERS {
        let end = find_in(bytes, at, bytes.len(), b"\n");
        if line >= out[h].last {
            h += 1;
        } else if line > out[h].first {
            let mut k = at;
            while k < end {
                if bytes[k] == b'@' {
                    let mut g = 0;
                    while g < HELPERS {
                        let len = out[g].sym_end - out[g].sym_start;
                        if k + 1 + len < end && bytes[k + 1 + len] == b'(' {
                            let mut same = true;
                            let mut i = 0;
                            while i < len {
                                if bytes[k + 1 + i] != bytes[out[g].sym_start + i] {
                                    same = false;
                                    break;
                                }
                                i += 1;
                            }
                            if same {
                                out[h].reaches |= 1 << g;
                            }
                        }
                        g += 1;
                    }
                }
                k += 1;
            }
        }
        at = end + 1;
        line += 1;
    }
    // And their calls, to a fixed point.
    let mut changed = true;
    while changed {
        changed = false;
        let mut h = 0;
        while h < HELPERS {
            let mut g = 0;
            while g < HELPERS {
                if out[h].reaches & (1 << g) != 0
                    && out[h].reaches | out[g].reaches != out[h].reaches
                {
                    out[h].reaches |= out[g].reaches;
                    changed = true;
                }
                g += 1;
            }
            h += 1;
        }
    }
    out
}

static HELPERS_DEV: [Helper; HELPERS] = index_helpers(DECLARES_DEV);
static HELPERS_RELEASE: [Helper; HELPERS] = index_helpers(DECLARES);

/// DECLARES as a module wants it: joined by newlines with no newline after the
/// last, keeping a `declare` only when `referenced` says the program calls its
/// symbol, and folding each stats gate to its fast branch unless `counting`.
#[cfg(test)]
fn declares_for(referenced: impl Fn(&str) -> bool, counting: bool, inline: bool) -> String {
    declares_for_program(referenced, |_| true, counting, inline, false)
}

/// `declares_for`, for a program that calls the helpers `called` names. A dev
/// module leaves out every helper neither the program nor a helper it calls
/// reaches: at `-O0` nothing removes an unused internal function, and the
/// codegen corpus compiled twenty-four of its thirty-three for nothing.
/// Unreached helpers are skipped a whole block at a time, so the lines that
/// are kept pay nothing for it.
fn declares_for_program(
    referenced: impl Fn(&str) -> bool,
    called: impl Fn(&str) -> bool,
    counting: bool,
    inline: bool,
    keep_context: bool,
) -> String {
    let (text, lines, helpers): (&str, &[DeclareLine], &[Helper]) = match inline {
        true => (DECLARES, &DECLARE_LINES, &HELPERS_RELEASE),
        false => (DECLARES_DEV, &DECLARE_LINES_DEV, &HELPERS_DEV),
    };
    let mut live = 0u64;
    for helper in helpers {
        if called(&text[helper.sym_start..helper.sym_end]) {
            live |= helper.reaches;
        }
    }
    let mut out = String::with_capacity(text.len());
    let mut first = true;
    let mut from = 0;
    for (n, helper) in helpers.iter().enumerate() {
        if live & (1 << n) == 0 {
            declare_lines(
                &mut out,
                &mut first,
                text,
                &lines[from..helper.first],
                &referenced,
                counting,
                keep_context,
            );
            from = helper.last + 1;
        }
    }
    declare_lines(&mut out, &mut first, text, &lines[from..], &referenced, counting, keep_context);
    out
}

/// How many `k_stats_on` gates DECLARES holds in the helpers `ir` defines. A
/// module carries only the helpers its program reaches, so a counting build's
/// gates are these and not all `STATS_GATE_SITES` of them.
pub fn stats_gates_carried(ir: &str) -> usize {
    HELPERS_RELEASE
        .iter()
        .filter(|h| {
            let call = format!("@{}(", &DECLARES[h.sym_start..h.sym_end]);
            ir.lines().any(|l| l.starts_with("define internal ") && l.contains(&call))
        })
        .map(|h| {
            DECLARE_LINES[h.first..=h.last]
                .iter()
                .filter(|l| DECLARES[l.start..l.end].contains("load i32, ptr @k_stats_on"))
                .count()
        })
        .sum()
}

fn declare_lines(
    out: &mut String,
    first: &mut bool,
    text: &str,
    lines: &[DeclareLine],
    referenced: &impl Fn(&str) -> bool,
    counting: bool,
    keep_context: bool,
) {
    for line in lines {
        if line.sym_start < line.sym_end
            && !(keep_context && line.context)
            && !referenced(&text[line.sym_start..line.sym_end])
        {
            continue;
        }
        let piece = match (counting, line.shipped) {
            (false, Shipped::Folded) => continue,
            (false, Shipped::Fold { label_start, label_end }) => {
                if !*first {
                    out.push('\n');
                }
                out.push_str("  br label %");
                out.push_str(&text[label_start..label_end]);
                *first = false;
                continue;
            }
            _ => &text[line.start..line.end],
        };
        if !*first {
            out.push('\n');
        }
        out.push_str(piece);
        *first = false;
    }
}

#[cfg(test)]
mod the_dev_declares_are_the_release_ones_without_the_attribute {
    use super::declares_for;

    /// The oracle: the release text, with ` alwaysinline` taken off each
    /// `define` line by a scan at run time.
    fn stripped(text: &str) -> String {
        text.split('\n')
            .map(|l| match l.starts_with("define ") {
                true => l.replacen(" alwaysinline", "", 1),
                false => l.to_string(),
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn every_selection_agrees() {
        let none = |_: &str| false;
        let all = |_: &str| true;
        for counting in [false, true] {
            let dev = declares_for(all, counting, false);
            assert_eq!(dev, stripped(&declares_for(all, counting, true)));
            assert!(!dev.lines().any(|l| l.starts_with("define ") && l.contains(" alwaysinline")));
            assert_eq!(
                declares_for(none, counting, false),
                stripped(&declares_for(none, counting, true))
            );
        }
    }
}

#[cfg(test)]
mod the_declares_table_is_the_scan_it_replaced {
    use super::{declares_for, DECLARES, STATS_GATE_SITES};

    /// The scan the table replaced, kept verbatim as the oracle: the filter
    /// that dropped uncalled declares, then the fold of the stats gates.
    fn scanned(referenced: impl Fn(&str) -> bool, counting: bool) -> String {
        let kept: Vec<&str> = DECLARES
            .lines()
            .filter(|line| {
                let Some(rest) = line.strip_prefix("declare ") else { return true };
                let Some(at) = rest.find('@') else { return true };
                let sym = &rest[at + 1..];
                let Some(paren) = sym.find('(') else { return true };
                referenced(&sym[..paren])
            })
            .collect();
        match counting {
            true => kept.join("\n"),
            false => without_stats_gate(kept).join("\n"),
        }
    }

    fn without_stats_gate(lines: Vec<&str>) -> Vec<String> {
        let mut out: Vec<String> = Vec::with_capacity(lines.len());
        let mut i = 0;
        let mut stripped = 0;
        while i < lines.len() {
            let line = lines[i];
            if !line.contains("load i32, ptr @k_stats_on") {
                out.push(line.to_string());
                i += 1;
                continue;
            }
            let icmp = lines.get(i + 1).copied().unwrap_or("");
            let br = lines.get(i + 2).copied().unwrap_or("");
            assert!(
                icmp.contains("= icmp ne i32 ") && icmp.trim_end().ends_with(", 0"),
                "the stats gate's second line is not the icmp this expects: {icmp}"
            );
            let fast = br
                .rsplit_once("label %")
                .map(|(_, name)| name.trim())
                .filter(|_| br.trim_start().starts_with("br i1 "))
                .unwrap_or_else(|| {
                    panic!("the stats gate's third line is not the br this expects: {br}")
                });
            out.push(format!("  br label %{fast}"));
            stripped += 1;
            i += 3;
        }
        assert_eq!(
            stripped, STATS_GATE_SITES,
            "the stats gate moved: DECLARES holds a different number of them"
        );
        out
    }

    /// Every choice of which declares survive that the emitter can make is a
    /// predicate on the symbol, so a handful that cut DECLARES different ways
    /// -- all, none, and three that split it -- over both kinds of build.
    #[test]
    fn every_build_reads_the_same_preamble() {
        let cuts: [fn(&str) -> bool; 5] = [
            |_| true,
            |_| false,
            |s| s.len() % 2 == 0,
            |s| s.contains("_b_"),
            |s| s.ends_with("_fast"),
        ];
        for counting in [true, false] {
            for cut in cuts {
                assert_eq!(declares_for(cut, counting, true), scanned(cut, counting));
            }
        }
    }

    /// A gate folds to one line where it was three, so a build without
    /// counters is shorter by exactly two lines a gate.
    #[test]
    fn a_shipped_build_folds_every_gate() {
        let counted = declares_for(|_| true, true, true).lines().count();
        let shipped = declares_for(|_| true, false, true).lines().count();
        assert_eq!(counted - shipped, 2 * STATS_GATE_SITES);
    }
}
