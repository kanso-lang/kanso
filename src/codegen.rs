use crate::ast::*;
use crate::diag::Span;
use crate::hash::Map as HashMap;
use crate::infer::{self, Set, BYTES, DESC, ERR, FAIL, FLOAT, INT, LIST, MAP, NONE, REC, STR, TOP};
use crate::name::Name;
use std::fmt::Write as _;

const K_TRUE: i64 = 2;

/// What an `if`'s condition left behind: the blocks that branched straight to
/// the merge because the condition was a failure, each with the value it
/// carried. A comparison on two ints leaves none, which is the point.
struct Cond {
    failed: Vec<(String, String)>,
}
const K_FALSE: i64 = 3;
const K_NONE: i64 = 4;
/// A succeeded effect's yield; the runtime's newest tag, after K_SUB.
const K_DONE: i64 = 16;
const K_ERR: i64 = 5;
const K_DESC: i64 = 8;

/// What an arm's discriminating pattern tests, when a switch on the value's tag
/// can express it. Records are `Rec` rather than a tag because they all carry
/// tag 7 and are told apart by the id and field count inside.
enum ArmCase {
    Tags(Vec<i64>),
    Rec(i64, usize),
}

/// Whether this build wants the allocation counters. BOTH halves of a binary
/// ask it -- the emitter for the eight inlined gates, and the runtime object
/// for its own twenty-seven -- because a counting runtime linked against
/// gate-free IR, or the reverse, is a binary whose counters are half there.
///
/// Either flag says yes. `--counters` is the explicit ask; a build running
/// under KANSO_COUNTERS is a process that is itself counting, and a binary it
/// produces is going to be counted too. Without the second, tests/golden.rs's
/// .mem vein -- which sets KANSO_COUNTERS around a build it drives through the
/// library and has no way to pass a flag -- silently got a gate-free binary,
/// and every allocation counter in the corpus moved at once.
///
/// Read once and kept. The emitter, the runtime object's key and the program
/// binary's key all ask, and each read walks the environment; the one place
/// that sets the flag, `--counters`, does it while parsing the arguments,
/// before anything asks.
pub fn counters_wanted() -> bool {
    static WANTED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *WANTED.get_or_init(|| {
        std::env::var_os("KANSO_COUNTERS_BUILD").is_some()
            || std::env::var_os("KANSO_COUNTERS").is_some()
    })
}

/// How many `k_stats_on` gates DECLARES carries. Pinned so that adding one
/// without teaching `index_declares` about it fails the build.
pub const STATS_GATE_SITES: usize = 10;

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
declare %KValue @k_unsub(%KValue)
declare %KValue @k_rec(i64, i64, ptr)
declare %KValue @k_pair_failure(%KValue, %KValue)
declare %KValue @k_rec_reuse(i64, i64, ptr, %KValue)
declare %KValue @k_parsed_box(i64, i64, i64)
declare %KValue @k_parsed_words(%KValue)
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
declare %KValue @k_set_field(%KValue, ptr, %KValue)
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
declare %KValue @k_b_utf8(%KValue, ptr)
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
declare void @k_beat_iter_carry()
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

pub(crate) const BUILTIN_CALLS: [&str; 58] = [
    "effect",
    "net_port",
    "start",
    "kill",
    "at",
    "is_desc",
    "append",
    "find2",
    "find2_below",
    "number_span",
    "bytes",
    "to_bytes",
    "bind",
    "rescue",
    "annotate",
    "read_file",
    "read_bytes",
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
        let Expr::Ident(callee, _, _) = head.as_ref() else { continue };
        let Some(target) = callee.strip_prefix("builtin_") else { continue };
        let all_forwarded = args.len() == params.len()
            && args.iter().zip(&params).all(|(a, p)| matches!(a, Expr::Ident(n, _, _) if n == p));
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
        if let Expr::Ident(n, _, _) | Expr::Partial(n, _) = expr {
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

/// Whether the host's clang can take `preserve_none` on a closure body.
///
/// The emitter cannot ask -- it never runs clang -- so the answer arrives
/// from whoever does. It is an explicit choice rather than a probe result on
/// purpose: `tests/compile_cost.rs` and `tests/perf_ratchet.rs` pin
/// compile-side goldens, and a golden that moved with the installed clang
/// would go red on a developer's machine for something the developer did not
/// do. Those callers pass `Absent` and their veins stay a property of the
/// compiler. Only the two build paths in main.rs ask the probe.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClosureConvention {
    /// `preserve_nonecc` on closure bodies and the arms that call them.
    PreserveNone,
    /// The C convention, which every clang can take.
    Absent,
}

impl ClosureConvention {
    /// The keyword to write before a closure define or call, with its
    /// trailing space, or nothing.
    fn keyword(self) -> &'static str {
        match self {
            ClosureConvention::PreserveNone => "preserve_nonecc ",
            ClosureConvention::Absent => "",
        }
    }
}

pub fn emit_ir(program: &Program, convention: ClosureConvention) -> Result<String, String> {
    emit_ir_for(program, convention, true)
}

/// The module a dev build compiles: the same as `emit_ir`'s, with the
/// runtime's helpers left to be called rather than marked `alwaysinline`.
///
/// A dev build compiles at `-O0`, where the always-inliner still copies every
/// helper into every call site and the instruction selector then walks each
/// copy. The codegen corpus's module carries thirty-six such definitions, and
/// without the attribute `clang -cc1` read 316,180,072 instructions against
/// 359,109,516, -11.95%. What a dev binary gives up is the inlining itself,
/// which the dev tier does not promise: it is the tier that compiles fast.
/// No helper needs to be inlined to be correct. None allocates on the stack,
/// reads a frame or return address, or makes a `musttail` call, and the
/// program's calls into them are all plain calls.
pub fn emit_ir_dev(program: &Program, convention: ClosureConvention) -> Result<String, String> {
    emit_ir_for(program, convention, false)
}

/// The work both tiers share, and the frame `emit_instructions` anchors on.
/// Kept out of line so the anchor exists whichever entry point reached it.
#[inline(never)]
/// Arms no value in the program can reach, dropped before anything is emitted.
///
/// An arm whose parameter pattern names a record type matches only a value of
/// that type, and the only way a value of a declared type comes to exist is an
/// expression that names the type: a construction, a partial of it, a bare
/// constructor handed on as a function, or an upcast to it. So a type that no
/// expression anywhere in the program names never has a value, and neither does
/// a subtype of it that is never built. `entries` is the one builder outside
/// the program's text, and it makes `entry`, id 0. An arm matching an unbuilt
/// type, at any depth of its pattern, can never be chosen, and what only it
/// calls is dead with it -- which the prune after emission then removes.
///
/// std/list's `next` dispatches over every lazy adapter the library declares,
/// so a program that maps once emitted every adapter's arm and everything each
/// arm calls: 1,560 of the codegen corpus's 4,628 lines of IR were adapters it
/// never builds and what they call. A group keeps all its arms if
/// every one of them would go, so a call that reaches it still fails the way
/// it did. The interpreter is untouched and the differential corpus is the
/// check that the two still agree.
fn without_unbuilt_arms(program: &Program) -> Option<Program> {
    use crate::ast::{Expr, Pattern, Stmt, TemplatePart};
    let mut ids: HashMap<&str, i64> = HashMap::default();
    ids.insert("entry", 0);
    for (i, ty) in program.types.iter().enumerate() {
        ids.insert(ty.name.as_str(), (i + 1) as i64);
    }
    for ty in &program.types {
        if let Some(id) = ty.origin.as_deref().and_then(|o| ids.get(o).copied()) {
            ids.insert(ty.name.as_str(), id);
        }
    }
    let mut built: crate::hash::Set<i64> = crate::hash::Set::default();
    built.insert(0);
    fn names_in_stmt<'a>(s: &'a Stmt, out: &mut Vec<&'a str>) {
        match s {
            Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => {
                names_in(expr, out)
            }
        }
    }
    fn names_in<'a>(e: &'a Expr, out: &mut Vec<&'a str>) {
        match e {
            Expr::Int(..) | Expr::Float(..) | Expr::Hole(_) => {}
            Expr::Ident(n, _, _) | Expr::Partial(n, _) => out.push(n.as_str()),
            Expr::MapLit(pairs, _) => {
                for (k, v) in pairs {
                    names_in(k, out);
                    names_in(v, out);
                }
            }
            Expr::Str(parts, _) => {
                for p in parts {
                    if let TemplatePart::Interp(inner) = p {
                        names_in(inner, out);
                    }
                }
            }
            Expr::List(items, _) => items.iter().for_each(|x| names_in(x, out)),
            Expr::App { head, args, .. } => {
                names_in(head, out);
                args.iter().for_each(|x| names_in(x, out));
            }
            Expr::Field { base, .. } => names_in(base, out),
            Expr::Index { base, index, .. } => {
                names_in(base, out);
                names_in(index, out);
            }
            Expr::BinOp { lhs: a, rhs: b, .. } | Expr::Join { lhs: a, rhs: b, .. } => {
                names_in(a, out);
                names_in(b, out);
            }
            Expr::Lambda { body, .. } => names_in(body, out),
            Expr::Block(stmts, _) | Expr::Build(stmts, _) => {
                stmts.iter().for_each(|s| names_in_stmt(s, out))
            }
            Expr::Upcast { expr, ty, .. } => {
                names_in(expr, out);
                out.push(ty.as_str());
            }
            Expr::Guard { cond, early, rest, .. } => {
                names_in(cond, out);
                names_in(early, out);
                rest.iter().for_each(|s| names_in_stmt(s, out));
            }
        }
    }
    fn unbuilt(p: &Pattern, ids: &HashMap<&str, i64>, built: &crate::hash::Set<i64>) -> bool {
        match p {
            Pattern::Ctor { ty, fields, .. } => {
                ids.get(ty.as_str()).is_some_and(|id| !built.contains(id))
                    || fields.iter().any(|f| unbuilt(f, ids, built))
            }
            _ => false,
        }
    }
    // Only a body that can run builds anything. std/list declares a builder
    // for every adapter, and counting names over every declaration found
    // every adapter built, whether or not the program ever called the
    // builder. So reachability and construction are one fixpoint: a group is
    // reached when a live arm names it, an arm is live when its group is
    // reached and no type in its pattern is unbuilt, and a live arm's body
    // builds what it names. The roots are the entry, every constant, and the
    // groups the emitter calls without the source naming them -- the
    // renderer and the user operators.
    let root = |d: &crate::ast::FnDecl| {
        d.name == crate::ast::ENTRY
            || d.params.is_empty()
            || d.name == RENDER_GROUP
            || !d.name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '/')
    };
    if !program.fns.iter().any(|d| d.name == crate::ast::ENTRY) {
        return None;
    }
    let mut reached: crate::hash::Set<&str> = crate::hash::Set::default();
    for d in &program.fns {
        if root(d) {
            reached.insert(d.name.as_str());
        }
    }
    let mut read = vec![false; program.fns.len()];
    loop {
        let mut moved = false;
        for (at, d) in program.fns.iter().enumerate() {
            if read[at] || !reached.contains(d.name.as_str()) {
                continue;
            }
            if d.params.iter().any(|p| unbuilt(p, &ids, &built)) {
                continue;
            }
            read[at] = true;
            moved = true;
            let mut named: Vec<&str> = Vec::new();
            d.body.iter().for_each(|s| names_in_stmt(s, &mut named));
            for name in named {
                if let Some(id) = ids.get(name) {
                    built.insert(*id);
                }
                reached.insert(name);
            }
            // A subtype's value matches its parent's patterns, so building one
            // builds every ancestor it flows as.
            loop {
                let before = built.len();
                for ty in &program.types {
                    let (Some(child), Some(parent)) =
                        (ids.get(ty.name.as_str()), ty.parent.as_deref())
                    else {
                        continue;
                    };
                    if built.contains(child) {
                        if let Some(p) = ids.get(parent) {
                            built.insert(*p);
                        }
                    }
                }
                if built.len() == before {
                    break;
                }
            }
        }
        if !moved {
            break;
        }
    }
    let dead: Vec<bool> =
        program.fns.iter().map(|d| d.params.iter().any(|p| unbuilt(p, &ids, &built))).collect();
    if !dead.iter().any(|d| *d) {
        return None;
    }
    // A group every one of whose arms would go keeps them all.
    let mut live_arms: HashMap<(&str, usize), usize> = HashMap::default();
    for (d, gone) in program.fns.iter().zip(&dead) {
        let n = live_arms.entry((d.name.as_str(), d.params.len())).or_insert(0);
        if !gone {
            *n += 1;
        }
    }
    let fns: Vec<crate::ast::FnDecl> = program
        .fns
        .iter()
        .zip(&dead)
        .filter(|(d, gone)| !**gone || live_arms[&(d.name.as_str(), d.params.len())] == 0)
        .map(|(d, _)| d.clone())
        .collect();
    Some(Program {
        fns,
        types: program.types.clone(),
        imports: program.imports.clone(),
        reexports: program.reexports.clone(),
        root: program.root.clone(),
    })
}

fn emit_ir_for(
    program: &Program,
    convention: ClosureConvention,
    inline_helpers: bool,
) -> Result<String, String> {
    let pruned = without_unbuilt_arms(program);
    let program = pruned.as_ref().unwrap_or(program);
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
    // One `Analysis` for all three, rather than one each: see
    // `linear::for_the_emitter`.
    let (in_place_pushes, reusable_records, (builder_joins, builder_params, builder_carried)) =
        crate::linear::for_the_emitter(program);
    // Beat loops rewind the arena between iterations. Groups returning the
    // by-value %parsed are excluded: k_beat_pop judges heap-ness from the
    // returned tag word, and the packed representation would mislead it.
    let mut beat = crate::beat::beat_loops(program, &inference, &in_place_pushes);
    beat.ids.retain(|(n, a), _| escape.returns_ty(n, *a).is_none());
    beat.demoted.retain(|(_, callee)| beat.ids.contains_key(callee));
    let forwarders = forwarder_map(program);
    let framed = framed_views(program, &forwarders);
    let mut backend = Backend {
        convention,
        inline_helpers,
        program,
        forwarders,
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
        group_by_name: group_indices_by_name(program),
        cycle_reached: cycle_reached(program),
        kept_out: kept_out(program),
        framed_views: framed,
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
        globals: String::new(),
        interned: HashMap::default(),
        body: String::new(),
        lift_counter: 0,
        fn_value_wrappers: Vec::new(),
        builtin_value_wrappers: Vec::new(),
        defers_self_reference: !knotted.is_empty(),
        knotted,
        print_value_wrapper: false,
        caf_cells: Vec::new(),
        closure_cells: Vec::new(),
        closure_consts: Vec::new(),
        demand: crate::demand::analyze(program),
        thunk_sites: Vec::new(),
    };
    backend.emit().map(|ir| through_doors(ir, convention))
}

/// The runtime functions the program calls that take `preserve_nonecc`
/// wherever closures do. src/runtime.c defines each with `K_DOORCC`, which is
/// the same attribute under the same probe, and says why these four;
/// `tests/the_doors_the_program_calls_agree.rs` holds the two lists equal,
/// since a door on one convention called on the other passes its arguments
/// in the wrong registers.
pub const PRESERVE_NONE_DOORS: [&str; 4] =
    ["k_b_append_rendered", "k_b_entries", "k_b_to_float_slice", "k_b_utf8"];

/// Every declare of and call to a door, written with the convention.
///
/// One pass over the module. Every door is spelled `k_b_...`, so the pass
/// stops only where `%KValue @k_b_` appears, and writes the convention in
/// front of it when a door's name and its `(` follow and a `declare ` or a
/// `call ` comes before. This was eight `str::replace` calls, each building
/// the whole module again: on a one-line `kanso play` under clang 19 they were
/// 402,596 of the 1,067,649 instructions under `kanso::main`.
fn through_doors(ir: String, convention: ClosureConvention) -> String {
    if convention != ClosureConvention::PreserveNone {
        return ir;
    }
    const MARK: &str = "%KValue @k_b_";
    let mut out = String::with_capacity(ir.len() + 4096);
    let mut rest = ir.as_str();
    while let Some(at) = rest.find(MARK) {
        let (head, tail) = rest.split_at(at);
        let name = &tail["%KValue @".len()..];
        let door = PRESERVE_NONE_DOORS
            .iter()
            .any(|d| name.strip_prefix(d).is_some_and(|after| after.starts_with('(')));
        out.push_str(head);
        if door && (head.ends_with("declare ") || head.ends_with("call ")) {
            out.push_str("preserve_nonecc ");
        }
        out.push_str(MARK);
        rest = &tail[MARK.len()..];
    }
    out.push_str(rest);
    out
}

struct Backend<'a> {
    /// see `ClosureConvention`; decided by the caller, never probed here
    convention: ClosureConvention,
    /// Whether the runtime's helpers go out as `alwaysinline`: yes for a
    /// release build, where inlining them is the point, and no for a dev
    /// build. See `emit_ir_dev`.
    inline_helpers: bool,
    program: &'a Program,
    /// Which declarations make up each group, by name, in program order.
    ///
    /// `group_indices` used to answer this by scanning `program.fns` end to
    /// end and collecting the matches into a fresh `Vec<usize>`, and
    /// `group_param_set` and `group_return_set` ask it once per parameter and
    /// once per call site. That is a question about the whole program asked
    /// once per name -- the shape kanso#1464, kanso#1468, kanso#1473 and
    /// kanso#1475 each removed from somewhere else -- and it cost 66,271,847
    /// instructions of `kanso build bench/runbench` over 7,556 calls.
    ///
    /// Keyed by NAME, with arity filtered off the result, for the reason
    /// kanso#1475's index is: the names come from the program's own
    /// declarations but also from call sites, and a `&str` key borrows as
    /// `str` where a tuple key would demand the program's lifetime.
    group_by_name: HashMap<&'a str, Vec<usize>>,
    /// Names a cycle can reach, which get no cohort bracket. See
    /// `cycle_reached`.
    cycle_reached: crate::hash::Set<&'a str>,
    /// See `kept_out`.
    kept_out: crate::hash::Set<&'a str>,
    /// See `framed_views`: (group, arity, span of the `bytes` call).
    framed_views: crate::hash::Set<(String, usize, Span)>,
    inference: infer::Inference,
    forwarders: HashMap<(String, usize), String>,
    /// subtype name -> parent name; non-empty programs get chain-aware
    /// dispatch checks, everyone else keeps the exact ones
    sub_parents: HashMap<String, String>,
    /// typeset name -> members; an annotated param matches any member
    typesets: HashMap<String, Vec<String>>,
    escape: crate::escape::EscapeInfo,
    byte_disc: crate::hash::Set<(String, usize, usize)>,
    in_place_pushes: crate::hash::Set<(std::sync::Arc<str>, usize, usize)>,
    reusable_records: crate::hash::Map<(std::sync::Arc<str>, usize, usize), String>,
    builder_joins: crate::hash::Set<(std::sync::Arc<str>, usize, usize)>,
    builder_params: crate::hash::Set<(String, usize, usize)>,
    /// Argument positions already carrying the builder, so no seed is needed.
    builder_carried: crate::hash::Set<(std::sync::Arc<str>, usize, usize)>,
    beat: crate::beat::Beats,
    type_ids: HashMap<&'a str, i64>,
    strings: Vec<(String, Vec<u8>)>,
    /// Constant tables the module defines, written beside the interned
    /// strings rather than into `body`, so the scans that read the body for
    /// calls do not walk them.
    globals: String,
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
    /// one permanent slot per zero-capture lambda site, built on first visit
    closure_cells: Vec<String>,
    /// A lambda that captures nothing is one value for the whole run, so it
    /// is a link-time constant rather than a slot filled on first visit.
    /// The fold's `%fnp = load ptr, ptr %c` then folds to the wrapper and
    /// the indirect call through the closure becomes a direct one LLVM can
    /// inline. (cell, wrapper, arity)
    closure_consts: Vec<(String, String, usize)>,
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
    file: std::sync::Arc<str>,
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
    /// Stack slots the body asked for, held back for the entry block.
    entry_allocas: Vec<String>,
    /// Whether a byte view's header lives in this frame, which turns the
    /// body's tail calls into plain ones: `framed_views` says why.
    frame_held: bool,
    /// A non-strict byte index builds a `%KValue` — the byte, or none — and a
    /// byte discriminator immediately collapses it back to one i64. The box
    /// is a phi over a struct, and LLVM will not sink the `extractvalue` into
    /// the diamond's two arms, so the collapse survives to machine code as
    /// four instructions on the hot path. This maps such a box to the raw
    /// i64 phi emitted beside it, so the crossing can take the raw form and
    /// leave the box for the dead-code pass.
    raw_byte: crate::hash::Map<String, String>,
    /// Whether the hot predicates are called in their two-word forms, which
    /// the dev tier's instruction selector can lower where it cannot lower a
    /// call passing a `%KValue`.
    words: bool,
    /// Values whose two words are already in hand, tag then payload. An
    /// unboxed parameter crosses as a raw i64 and is boxed on entry, and every
    /// read of its tag or payload used to take it back apart with an
    /// `extractvalue`; the tag is 0 and the payload is the argument itself.
    known_words: crate::hash::Map<String, (String, String)>,
    /// The blocks a checked integer op in this function branches to when it
    /// overflows, and the message they die with. The block is the same three
    /// lines wherever the op is, so every op shares the first, and `body`
    /// writes it once at the end rather than once an op.
    overflow_traps: Vec<String>,
    overflow_message: String,
}
/// Whether a line the emitters wrote is a stack slot, asked at the ONE place
/// the needle can be.
///
/// A slot reads `%name = alloca <type>`, and `%name` holds no space, so
/// ` = alloca ` begins at the line's FIRST space or it is not there at all.
/// Reading the whole line to learn that is what `FnEmit::write` used to do,
/// and it was the emitter's fourth-largest frame: 18.2 million instructions
/// over 144,261 lines on the build profile, about 126 a line to answer a
/// question the first seven bytes settle. Measured on runbench's 36,086
/// emitted lines, all 235 slots put the needle between offsets five and
/// seven, and each of those is that line's first space.
///
/// The check stays in `write` rather than at the seven sites that emit an
/// alloca, for the reason the comment there gives: the rule has to hold for a
/// site nobody has written yet.
/// `tests/a_stack_slot_is_found_where_the_first_space_is.rs` compiles the
/// corpora and asserts this reading and the whole-line one agree for every
/// line, so a line that ever carried the needle elsewhere is caught rather
/// than quietly written into the wrong block.
pub fn is_a_stack_slot(text: &str) -> bool {
    match text.find(' ') {
        Some(at) => text[at..].starts_with(" = alloca "),
        None => false,
    }
}

impl FnEmit {
    fn new(words: bool) -> Self {
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
            file: crate::ast::unstamped(),
            ret_ty: "%KValue".to_string(),
            group: String::new(),
            synthetic: false,
            arity: 0,
            lazy_cells: Vec::new(),
            entry_allocas: Vec::new(),
            frame_held: false,
            raw_byte: crate::hash::Map::default(),
            words,
            known_words: crate::hash::Map::default(),
            overflow_traps: Vec::new(),
            overflow_message: String::new(),
        }
    }

    /// Writes `{r} = {call}`, where `call` asks one of the hot predicates about
    /// a `%KValue` in the form a release module inlines. The dev tier asks the
    /// two-word form instead (`k_truthy_w` and `k_check_rec_fast_w` in
    /// DECLARES), with the value's words pulled out
    /// first and passed as scalars. At -O0 clang's fast instruction selector
    /// lowers a call only when every argument is a scalar, so each call passing
    /// a `%KValue` went to the slow selector on its own: 216 of them on the
    /// codegen corpus. A release module never calls the two-word forms, so it
    /// never carries them.
    fn predicate(&mut self, r: &str, call: String) {
        const FORMS: [(&str, &str, bool); 2] = [
            ("call i64 @k_truthy(%KValue ", "k_truthy_w", true),
            ("call i64 @k_check_rec_fast(%KValue ", "k_check_rec_fast_w", true),
        ];
        if !self.words {
            self.line(&format!("{r} = {call}"));
            return;
        }
        let split = FORMS.iter().find_map(|(prefix, to, pay)| {
            let rest = call.strip_prefix(prefix)?;
            // A constant operand spells its own commas; it keeps the old form.
            let end = rest.find([',', ')']).filter(|_| rest.starts_with('%'))?;
            Some((*to, *pay, rest[..end].to_string(), rest[end..].to_string()))
        });
        match split {
            Some((to, pay, value, rest)) => {
                let tag = self.tmp();
                self.line(&format!("{tag} = extractvalue %KValue {value}, 0"));
                let words = match pay {
                    true => {
                        let p = self.tmp();
                        self.line(&format!("{p} = extractvalue %KValue {value}, 1"));
                        format!("i64 {tag}, i64 {p}")
                    }
                    false => format!("i64 {tag}"),
                };
                self.line(&format!("{r} = call i64 @{to}({words}{rest}"));
            }
            None => self.line(&format!("{r} = {call}")),
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
        self.write(&text);
    }

    /// Every stack slot the body asks for is held back and written at the top
    /// of the entry block, whichever block asked for it. An alloca standing in
    /// a later block is a dynamic stack object to LLVM: the function keeps a
    /// frame pointer and restores `rsp` through it on every return, and the
    /// slot is claimed again on each pass. Sixty-eight of the decoder's
    /// seventy-three slots stood that way and the decode paid 8.78% for them.
    /// Diverting here rather than at each emitter is what makes the rule hold
    /// for a site nobody has written yet.
    ///
    /// A slot hoisted out of a loop body is shared between passes rather than
    /// claimed afresh. Every consumer of one copies out of it before it
    /// returns — `k_rec` and `k_closure` memcpy, `k_list_lit` and `k_map_lit`
    /// build their own storage — so nothing survives a pass to read the next
    /// pass's bytes.
    fn write(&mut self, text: &str) {
        if is_a_stack_slot(text) {
            self.entry_allocas.push(text.to_string());
            return;
        }
        let _ = writeln!(self.out, "  {text}");
    }

    /// The function's body: what the emitters wrote, with the stack slots at
    /// the head of the entry block so each one dominates its uses.
    fn body(&self) -> String {
        let unboxed = self.without_unread_reboxes();
        let mut out = unboxed.as_deref().unwrap_or(&self.out);
        let trapped;
        if !self.overflow_traps.is_empty() {
            let mut text = out.to_string();
            for label in &self.overflow_traps {
                let message = &self.overflow_message;
                let _ = write!(text, "{label}:\n  call void @k_die(ptr @{message})\n");
                text.push_str("  unreachable\n");
            }
            trapped = text;
            out = &trapped;
        }
        if self.entry_allocas.is_empty() {
            return out.to_string();
        }
        let mut head = String::new();
        let mut rest = out;
        if let Some(end) = out.find('\n') {
            let first = &out[..end];
            if first.ends_with(':') && !first.starts_with(' ') {
                head.push_str(first);
                head.push('\n');
                rest = &out[end + 1..];
            }
        }
        for slot in &self.entry_allocas {
            let _ = writeln!(head, "  {slot}");
        }
        head.push_str(rest);
        head
    }

    /// The body without the boxing of an unboxed parameter nothing reads.
    /// Every read of such a parameter's words is answered from
    /// `known_words`, so in a function that only compares and passes it on,
    /// the `insertvalue` on entry builds a value no line names.
    fn without_unread_reboxes(&self) -> Option<String> {
        let mut out: Option<String> = None;
        for (value, (_, payload)) in &self.known_words {
            let text = out.as_deref().unwrap_or(&self.out);
            let line = format!(
                "  {value} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 {payload}, 1\n"
            );
            let Some(at) = text.find(&line) else { continue };
            let named = text.match_indices(value.as_str()).any(|(i, _)| {
                i != at + 2
                    && !text[i + value.len()..]
                        .bytes()
                        .next()
                        .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.')
            });
            if !named {
                let mut kept = text[..at].to_string();
                kept.push_str(&text[at + line.len()..]);
                out = Some(kept);
            }
        }
        out
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
    /// A failure rides in the same two words and comes back as itself: the
    /// runtime asks before it builds, which the inline build here did not.
    fn box_parsed(&mut self, e: &str) -> String {
        let (_, id) = self.parsed[e];
        let w0 = self.tmp();
        self.raw(&format!("{w0} = extractvalue %parsed {e}, 0"));
        let w1 = self.tmp();
        self.raw(&format!("{w1} = extractvalue %parsed {e}, 1"));
        let t = self.tmp();
        self.raw(&format!("{t} = call %KValue @k_parsed_box(i64 {id}, i64 {w0}, i64 {w1})"));
        t
    }

    /// A line whose operands are already values: the boxing rewrite emits
    /// through here, so a fresh temp is never scanned against itself.
    fn raw(&mut self, text: &str) {
        self.write(text);
    }

    /// The label a checked integer op branches to on overflow.
    fn overflow_trap(&mut self, message: String) -> String {
        if let Some(trap) = self.overflow_traps.first() {
            return trap.clone();
        }
        let trap = self.label();
        self.overflow_traps.push(trap.clone());
        self.overflow_message = message;
        trap
    }

    fn start_block(&mut self, label: &str) {
        let _ = writeln!(self.out, "{label}:");
        self.cur_label = label.to_string();
    }

    /// Branches on `value` to the label its case names, or to `dflt`. A
    /// release module writes a `switch`, which its optimiser turns into a jump
    /// table. clang's fast instruction selector at -O0 does not lower one, so
    /// a dev module asks the cases in turn with a compare and a branch each,
    /// which it does.
    fn switch_on(&mut self, value: &str, dflt: &str, cases: &[String]) {
        if !self.words {
            self.line(&format!("switch i64 {value}, label %{dflt} [\n{}\n  ]", cases.join("\n")));
            return;
        }
        for case in cases {
            let (n, target) = case
                .trim()
                .strip_prefix("i64 ")
                .and_then(|c| c.split_once(", label %"))
                .expect("a case reads `i64 N, label %L`");
            let hit = self.tmp();
            self.line(&format!("{hit} = icmp eq i64 {value}, {n}"));
            let next = self.label();
            self.line(&format!("br i1 {hit}, label %{target}, label %{next}"));
            self.start_block(&next);
        }
        self.line(&format!("br label %{dflt}"));
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
        if operand == "{ i64 16, i64 0 }" {
            return infer::DONE;
        }
        self.sets.get(operand).copied().unwrap_or(TOP)
    }
}

/// Dispatchers, wrappers and lambda bodies nothing live names. A block is kept
/// when a chain of names reaches it from the entry or from a block that is not
/// a candidate; a dead caller's call to its callee does not count, and neither
/// does a cycle of dead blocks naming each other. Only `d_`, `w_` and `klam`
/// symbols are candidates: everything else is either the entry, a
/// builder the constant initialiser calls, or a switch the runtime calls by
/// name. A lambda body is named by its wrapper or by the closure built over
/// it, and one in a library function the program never reaches is named by
/// neither; at `-O0` clang compiled each of them anyway.
/// `cells` pairs a constant closure's cell with the wrapper it points at. The
/// pointer lives in a module global rather than in any function body, so the
/// wrapper is named by the cell rather than by a call, and a site that loads
/// the cell is what keeps it.
fn prune_unnamed(body: &str, entry: &str, cells: &[(String, String, usize)]) -> String {
    let blocks = ir_defines(body);
    let mut alive = vec![false; blocks.len()];
    {
        // Every name this prune will ever ask about: one per block, plus the
        // closure cells. The index is keyed on exactly these, so a name
        // nothing asks about costs nothing to walk past.
        let queries: crate::hash::Set<&str> = blocks
            .iter()
            .map(|(sym, _)| sym.as_str())
            .chain(cells.iter().map(|(cell, _, _)| cell.as_str()))
            .filter(|sym| !sym.is_empty())
            .collect();
        // Which of those names each block writes, read off once per block. The
        // search this replaced asked
        // the question once per (block, name) pair and built a fresh two-way
        // searcher for each: 72.10% of `kanso build bench/runbench`, with
        // `names_symbol` 71.71% of the process on its own.
        // A name the one-pass scan cannot tokenise is answered the old way,
        // per block, rather than assumed away. Emitted symbols are all one of
        // the two shapes `queries_named` reads, so this list is empty in
        // practice -- and the correctness of the index does not rest on that
        // being true, which is the point of having it.
        let odd: Vec<&str> = queries.iter().copied().filter(|sym| !one_pass_reads(sym)).collect();
        let mut names: Vec<crate::hash::Set<&str>> =
            blocks.iter().map(|(_, text)| queries_named(text, &queries)).collect();
        for (written, (_, text)) in names.iter_mut().zip(blocks.iter()) {
            for sym in &odd {
                if names_symbol(text, sym) {
                    written.insert(sym);
                }
            }
        }
        // A mark from the roots rather than a count of mentions. Striking a
        // block once nothing else named it was reference counting, and a
        // cycle names itself: std/list's merge sort is merge, merge_on, pick
        // and advance calling round, so a program that never sorts struck
        // `sort`, `msort` and `span` and kept the four, which nothing could
        // call.
        let at_sym: crate::hash::Map<&str, usize> = blocks
            .iter()
            .enumerate()
            .filter(|(_, (sym, _))| !sym.is_empty())
            .map(|(at, (sym, _))| (sym.as_str(), at))
            .collect();
        let wrapped: crate::hash::Map<&str, &str> =
            cells.iter().map(|(cell, w, _)| (cell.as_str(), w.as_str())).collect();
        let mut work: Vec<usize> = blocks
            .iter()
            .enumerate()
            .filter(|(_, (sym, _))| {
                // The runtime calls the thunk dispatcher itself, from
                // `k_force`, so no emitted line names it.
                sym == entry
                    || sym == "d_thunk_eval"
                    || !(sym.starts_with("d_")
                        || sym.starts_with("w_")
                        || sym.starts_with("klam")
                        || sym.starts_with("\"d_")
                        || sym.starts_with("\"w_"))
            })
            .map(|(at, _)| at)
            .collect();
        for &at in &work {
            alive[at] = true;
        }
        while let Some(at) = work.pop() {
            for name in &names[at] {
                // A cell is a module global, so the wrapper it points at is
                // named by whoever loads the cell rather than by a call.
                let target = wrapped.get(name).map_or(*name, |w| *w);
                for sym in [*name, target] {
                    if let Some(&next) = at_sym.get(sym) {
                        if !alive[next] {
                            alive[next] = true;
                            work.push(next);
                        }
                    }
                }
            }
        }
    }
    blocks.into_iter().zip(alive).filter(|(_, alive)| *alive).map(|((_, text), _)| text).collect()
}

/// Whether `queries_named` can tokenise this symbol, which is to say whether
/// it is one of the two shapes the emitter writes: an unquoted run of
/// `[alnum _]`, or a quoted name from `"` to its closing `"` with no other
/// quote inside. Anything else is answered by `names_symbol` per block, so
/// the index stays exact without assuming the emitter's spellings.
fn one_pass_reads(sym: &str) -> bool {
    match sym.strip_prefix('"') {
        Some(rest) => match rest.strip_suffix('"') {
            Some(inner) => !inner.contains('"'),
            None => false,
        },
        None => !sym.is_empty() && sym.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'),
    }
}

/// Which of `queries` this text names, in one pass, answering exactly what
/// `names_symbol` answers for each of them.
///
/// The delimiter rule is `names_symbol`'s: `@sym` counts only where the byte
/// after it is not alphanumeric, `_` or `"`. Two token shapes cover every
/// symbol the emitter writes, and `the_only_two_shapes_a_symbol_takes` pins
/// it: an unquoted name is a run of those same bytes, and a quoted one runs
/// from `"` to the next `"`, which is how `quoted()` spells a name holding a
/// slash or an operator character. Scanning the unquoted run alone would stop
/// at the slash inside `@"d_add/2"` and miss it.
fn queries_named<'a>(text: &str, queries: &crate::hash::Set<&'a str>) -> crate::hash::Set<&'a str> {
    let held = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'"';
    let bytes = text.as_bytes();
    let mut found = crate::hash::Set::default();
    let mut at = 0;
    // `find` on an ascii char is memchr; `position` walked a byte a step.
    while let Some(next) = text[at..].find('@') {
        let from = at + next + 1;
        at = from;
        let end = match bytes.get(from) {
            Some(b'"') => match bytes[from + 1..].iter().position(|b| *b == b'"') {
                Some(close) => from + close + 2,
                None => continue,
            },
            _ => {
                let mut end = from;
                while end < bytes.len() && held(bytes[end]) {
                    end += 1;
                }
                end
            }
        };
        if matches!(bytes.get(end), Some(b) if held(*b)) {
            continue;
        }
        if let Some(name) = queries.get(&text[from..end]) {
            found.insert(*name);
        }
    }
    found
}

/// Every declaration's index, by name, in program order.
///
/// `Backend::group_indices` scanned `program.fns` end to end and collected the
/// matches into a fresh `Vec<usize>` on every call, and `group_param_set` and
/// `group_return_set` ask it once per parameter and once per call site.
fn group_indices_by_name(program: &Program) -> HashMap<&str, Vec<usize>> {
    let mut by_name: HashMap<&str, Vec<usize>> = HashMap::default();
    for (at, decl) in program.fns.iter().enumerate() {
        by_name.entry(decl.name.as_str()).or_default().push(at);
    }
    by_name
}

/// Every declared name a cycle can reach: the members of every cycle in what
/// the bodies mention, and everything those members mention, onward.
///
/// A cohort bracket costs a push and a pop on every call it wraps, and pays
/// only when the call leaves garbage worth a rewind. A name a cycle reaches
/// can run once per element of something -- a parser's recursive descent, a
/// loop, a helper one of those calls per number -- and its calls are the
/// small ones. A name no cycle reaches runs a number of times fixed by
/// straight-line code, which is where a phase of a program starts and ends.
/// Names, not groups, and a mention counts whether it is a call or a value.
/// A cycle through a closure some library calls back is not seen; that miss
/// costs speed and nothing else, because the bracket is sound on any call
/// the license admits.
fn cycle_reached(program: &Program) -> crate::hash::Set<&str> {
    fn mentions(expr: &Expr, index: &HashMap<&str, usize>, out: &mut Vec<usize>) {
        if let Expr::Ident(n, _, _) | Expr::Partial(n, _) = expr {
            if let Some(&at) = index.get(&**n) {
                out.push(at);
            }
        }
        crate::for_each_child(expr, |child| mentions(child, index, out));
    }
    let mut index: HashMap<&str, usize> = HashMap::default();
    let mut names: Vec<&str> = Vec::new();
    for decl in &program.fns {
        index.entry(decl.name.as_str()).or_insert_with(|| {
            names.push(decl.name.as_str());
            names.len() - 1
        });
    }
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); names.len()];
    for decl in &program.fns {
        let from = index[decl.name.as_str()];
        for stmt in &decl.body {
            match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => {
                    mentions(expr, &index, &mut adj[from])
                }
                Stmt::Set { value, .. } => mentions(value, &index, &mut adj[from]),
            }
        }
    }
    let mut hot = vec![false; names.len()];
    let mut queue: Vec<usize> = Vec::new();
    for scc in crate::beat::sccs_of(&adj) {
        if scc.len() >= 2 || adj[scc[0]].contains(&scc[0]) {
            queue.extend(scc);
        }
    }
    while let Some(at) = queue.pop() {
        if !hot[at] {
            hot[at] = true;
            queue.extend(adj[at].iter().copied().filter(|&next| !hot[next]));
        }
    }
    names.iter().zip(hot).filter(|(_, h)| *h).map(|(n, _)| *n).collect()
}

/// Functions a dispatcher calls from its heavy arms, kept out of line.
///
/// A group of clauses compiles to one function with a switch at its head, and
/// LLVM gives that function one frame. When one arm inlines a callee that
/// loops, the loop's registers are callee-saved ones, so the prologue pushes
/// six of them and the epilogue pops them on every call -- including the calls
/// whose arm is a single append. The encoder's `encode_onto` is that shape:
/// 2,380,860 calls on runbench, 1,438,110 of them scalars that paid the frame
/// of the string arm's inlined escaper. A callee that can reach a cycle, named
/// in an arm of a group that also has an arm reaching none, stays a call, and
/// the dispatcher's cheap arms keep a frame of their own size.
fn kept_out(program: &Program) -> crate::hash::Set<&str> {
    fn mentions(expr: &Expr, index: &HashMap<&str, usize>, out: &mut Vec<usize>) {
        if let Expr::Ident(n, _, _) | Expr::Partial(n, _) = expr {
            if let Some(&at) = index.get(&**n) {
                out.push(at);
            }
        }
        crate::for_each_child(expr, |child| mentions(child, index, out));
    }
    fn decl_mentions(decl: &FnDecl, index: &HashMap<&str, usize>) -> Vec<usize> {
        let mut out = Vec::new();
        for stmt in &decl.body {
            match stmt {
                Stmt::Bind { expr, .. } | Stmt::Expr(expr) => mentions(expr, index, &mut out),
                Stmt::Set { value, .. } => mentions(value, index, &mut out),
            }
        }
        out
    }
    let mut index: HashMap<&str, usize> = HashMap::default();
    let mut names: Vec<&str> = Vec::new();
    for decl in &program.fns {
        index.entry(decl.name.as_str()).or_insert_with(|| {
            names.push(decl.name.as_str());
            names.len() - 1
        });
    }
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); names.len()];
    let per_decl: Vec<Vec<usize>> = program.fns.iter().map(|d| decl_mentions(d, &index)).collect();
    for (decl, m) in program.fns.iter().zip(&per_decl) {
        adj[index[decl.name.as_str()]].extend(m.iter().copied());
    }
    // loops: a name in a cycle, and every name that can reach one
    let mut loops = vec![false; names.len()];
    let mut callers: Vec<Vec<usize>> = vec![Vec::new(); names.len()];
    for (from, to) in adj.iter().enumerate() {
        for &t in to {
            callers[t].push(from);
        }
    }
    let mut queue: Vec<usize> = Vec::new();
    for scc in crate::beat::sccs_of(&adj) {
        if scc.len() >= 2 || adj[scc[0]].contains(&scc[0]) {
            queue.extend(scc);
        }
    }
    while let Some(at) = queue.pop() {
        if !loops[at] {
            loops[at] = true;
            queue.extend(callers[at].iter().copied().filter(|&c| !loops[c]));
        }
    }
    let mut out = crate::hash::Set::default();
    let mut groups: HashMap<(&str, usize), Vec<usize>> = HashMap::default();
    for (at, decl) in program.fns.iter().enumerate() {
        groups.entry((decl.name.as_str(), decl.params.len())).or_default().push(at);
    }
    for ((name, _), clauses) in &groups {
        if clauses.len() < 2 {
            continue;
        }
        // a switch on the argument's type, where the arms do unrelated work
        let typed = |c: &usize| {
            program.fns[*c]
                .params
                .iter()
                .any(|p| matches!(p, crate::ast::Pattern::Annotated { .. }))
        };
        if !clauses.iter().any(typed) {
            continue;
        }
        let own = index[name];
        let heavy = |c: &usize| per_decl[*c].iter().any(|&m| m != own && loops[m]);
        if clauses.iter().all(heavy) || !clauses.iter().any(heavy) {
            continue;
        }
        for c in clauses {
            for &m in &per_decl[*c] {
                if m != own && loops[m] {
                    out.insert(names[m]);
                }
            }
        }
    }
    out
}

/// The byte views that live in their function's frame. `bytes s` of a
/// string writes a three-word header -- length, data, capacity -- and the
/// header is all it allocates, since the view borrows the string's bytes.
/// JSON's `escape_onto` makes one per string it writes, 942,750 on the run
/// program, and reads it for a length, a scan and some slices before it
/// returns. A header in the arena costs the bump and every read through it;
/// a header in the frame is three stores LLVM takes apart into registers.
///
/// The frame must outlive every read. So a binding `x = bytes e` qualifies
/// when its function sits on no cycle of the call graph, which keeps the
/// frame from being claimed once per pass of a loop, and every mention of `x`
/// after it is one of these reads:
///   - the first argument of `length`, `find2`, `find2_below` or `slice`,
///     none of which keeps the header (a slice writes a header of its own
///     over the same bytes);
///   - the base of an index;
///   - an argument to a function whose parameter there is itself read only
///     this way, in every clause, found as the largest such set.
///
/// Anything else fails it: a return, a record or list, a closure that
/// could run after the frame is gone, a binding (which the demand pass
/// may make lazy), an argument to a name the body binds itself. A function
/// holding such a view makes its tail calls as plain calls, because a tail
/// call gives the frame back before the callee reads the header.
fn framed_views(
    program: &Program,
    forwarders: &HashMap<(String, usize), String>,
) -> crate::hash::Set<(String, usize, Span)> {
    struct Cx<'a> {
        groups: &'a HashMap<(&'a str, usize), Vec<&'a FnDecl>>,
        forwarders: &'a HashMap<(String, usize), String>,
        safe: &'a crate::hash::Set<(String, usize, usize)>,
        locals: crate::hash::Set<String>,
    }
    fn pattern_names(p: &Pattern, out: &mut crate::hash::Set<String>) {
        match p {
            Pattern::Var(n, _) => {
                out.insert(n.to_string());
            }
            Pattern::Annotated { name, .. } => {
                out.insert(name.to_string());
            }
            Pattern::Ctor { fields, whole, .. } => {
                for f in fields {
                    pattern_names(f, out);
                }
                if let Some(w) = whole {
                    out.insert(w.0.to_string());
                }
            }
            Pattern::Keyed { entries, .. } => {
                for e in entries {
                    out.insert(e.bind_name.clone());
                }
            }
            _ => {}
        }
    }
    fn stmt_expr(st: &Stmt) -> &Expr {
        match st {
            Stmt::Bind { expr, .. } | Stmt::Expr(expr) | Stmt::Set { value: expr, .. } => expr,
        }
    }
    fn locals_of(expr: &Expr, out: &mut crate::hash::Set<String>) {
        match expr {
            Expr::Lambda { params, .. } => {
                for (n, _) in params {
                    out.insert(n.clone());
                }
            }
            Expr::Block(stmts, _) | Expr::Build(stmts, _) | Expr::Guard { rest: stmts, .. } => {
                for st in stmts {
                    if let Stmt::Bind { pattern, .. } = st {
                        pattern_names(pattern, out);
                    }
                }
            }
            _ => {}
        }
        crate::for_each_child(expr, |c| locals_of(c, out));
    }
    // every name a body binds for itself, beside its parameters
    fn bound_in(stmts: &[Stmt]) -> crate::hash::Set<String> {
        let mut out = crate::hash::Set::default();
        for st in stmts {
            if let Stmt::Bind { pattern, .. } = st {
                pattern_names(pattern, &mut out);
            }
            locals_of(stmt_expr(st), &mut out);
        }
        out
    }
    fn mentions(expr: &Expr, x: &str) -> bool {
        if let Expr::Ident(n, _, _) = expr {
            if n.as_str() == x {
                return true;
            }
        }
        let mut found = false;
        crate::for_each_child(expr, |c| found = found || mentions(c, x));
        found
    }
    fn is_x(expr: &Expr, x: &str) -> bool {
        matches!(expr, Expr::Ident(n, _, _) if n.as_str() == x)
    }
    fn stmts_read_only(stmts: &[Stmt], x: &str, cx: &Cx) -> bool {
        stmts.iter().all(|st| match st {
            Stmt::Expr(e) => reads_only(e, x, cx),
            Stmt::Bind { expr, .. } | Stmt::Set { value: expr, .. } => !mentions(expr, x),
        })
    }
    fn reads_only(expr: &Expr, x: &str, cx: &Cx) -> bool {
        match expr {
            Expr::Ident(n, _, _) => n.as_str() != x,
            Expr::Lambda { .. } | Expr::Build(..) => !mentions(expr, x),
            Expr::Index { base, index, .. } if is_x(base, x) => reads_only(index, x, cx),
            Expr::Block(stmts, _) => stmts_read_only(stmts, x, cx),
            Expr::Guard { cond, early, rest, .. } => {
                reads_only(cond, x, cx) && reads_only(early, x, cx) && stmts_read_only(rest, x, cx)
            }
            Expr::App { head, args, piped: false, .. } => {
                let Expr::Ident(h, _, _) = head.as_ref() else {
                    return !mentions(expr, x);
                };
                let h = h.as_str();
                if h == x {
                    return false;
                }
                let n = args.len();
                let local = cx.locals.contains(h);
                let forwarded = cx.forwarders.get(&(h.to_string(), n)).map(String::as_str);
                let user = !local && forwarded.is_none() && cx.groups.contains_key(&(h, n));
                let builtin = match (local, forwarded) {
                    (true, _) => None,
                    (false, Some(b)) => Some(b),
                    (false, None) if user => None,
                    (false, None) => Some(h.strip_prefix("builtin_").unwrap_or(h)),
                };
                args.iter().enumerate().all(|(i, a)| {
                    if !is_x(a, x) {
                        return reads_only(a, x, cx);
                    }
                    match builtin {
                        Some(b) => {
                            i == 0 && matches!(b, "length" | "find2" | "find2_below" | "slice")
                        }
                        None => user && cx.safe.contains(&(h.to_string(), n, i)),
                    }
                })
            }
            _ => {
                let mut ok = true;
                crate::for_each_child(expr, |c| ok = ok && reads_only(c, x, cx));
                ok
            }
        }
    }
    let mut groups: HashMap<(&str, usize), Vec<&FnDecl>> = HashMap::default();
    for d in &program.fns {
        groups.entry((d.name.as_str(), d.params.len())).or_default().push(d);
    }
    let locals_for = |d: &FnDecl| {
        let mut out = bound_in(&d.body);
        for p in &d.params {
            pattern_names(p, &mut out);
        }
        out
    };
    // the bindings that could qualify, before anything is asked of their uses
    let mut candidates = Vec::new();
    for d in &program.fns {
        if d.is_getter() {
            continue;
        }
        for (i, st) in d.body.iter().enumerate() {
            let Stmt::Bind { pattern: Pattern::Var(x, _), expr } = st else { continue };
            let Expr::App { head, args, piped: false, span } = expr else { continue };
            let Expr::Ident(h, _, _) = head.as_ref() else { continue };
            if args.len() != 1 {
                continue;
            }
            let target = match forwarders.get(&(h.to_string(), 1)) {
                Some(b) => b.as_str(),
                None if groups.contains_key(&(h.as_str(), 1)) => continue,
                None => h.strip_prefix("builtin_").unwrap_or(h),
            };
            if target == "bytes" {
                candidates.push((d, i, x.as_str(), h.as_str(), *span));
            }
        }
    }
    if candidates.is_empty() {
        return crate::hash::Set::default();
    }
    // names on a cycle of the call graph, which may not hold a view
    let mut index: HashMap<&str, usize> = HashMap::default();
    let mut names: Vec<&str> = Vec::new();
    for d in &program.fns {
        index.entry(d.name.as_str()).or_insert_with(|| {
            names.push(d.name.as_str());
            names.len() - 1
        });
    }
    fn callees(expr: &Expr, index: &HashMap<&str, usize>, out: &mut Vec<usize>) {
        if let Expr::Ident(n, _, _) | Expr::Partial(n, _) = expr {
            if let Some(&at) = index.get(&**n) {
                out.push(at);
            }
        }
        crate::for_each_child(expr, |c| callees(c, index, out));
    }
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); names.len()];
    for d in &program.fns {
        let from = index[d.name.as_str()];
        for st in &d.body {
            callees(stmt_expr(st), &index, &mut adj[from]);
        }
    }
    let mut cyclic = vec![false; names.len()];
    for scc in crate::beat::sccs_of(&adj) {
        if scc.len() >= 2 || adj[scc[0]].contains(&scc[0]) {
            for at in scc {
                cyclic[at] = true;
            }
        }
    }
    candidates.retain(|(d, ..)| !cyclic[index[d.name.as_str()]]);
    if candidates.is_empty() {
        return crate::hash::Set::default();
    }
    // the parameters a view can reach, found by following it into each callee
    // it is handed to; nothing else is ever asked about
    fn handed(
        expr: &Expr,
        x: &str,
        locals: &crate::hash::Set<String>,
        user: &dyn Fn(&str, usize) -> bool,
        out: &mut Vec<(String, usize, usize)>,
    ) {
        if let Expr::App { head, args, piped: false, .. } = expr {
            if let Expr::Ident(h, _, _) = head.as_ref() {
                if !locals.contains(h.as_str()) && user(h.as_str(), args.len()) {
                    for (i, a) in args.iter().enumerate() {
                        if is_x(a, x) {
                            out.push((h.to_string(), args.len(), i));
                        }
                    }
                }
            }
        }
        crate::for_each_child(expr, |c| handed(c, x, locals, user, out));
    }
    let user = |h: &str, n: usize| {
        !forwarders.contains_key(&(h.to_string(), n)) && groups.contains_key(&(h, n))
    };
    let mut reached: crate::hash::Set<(String, usize, usize)> = crate::hash::Set::default();
    let mut work = Vec::new();
    for (d, i, x, _, _) in &candidates {
        let locals = locals_for(d);
        for st in &d.body[i + 1..] {
            handed(stmt_expr(st), x, &locals, &user, &mut work);
        }
    }
    while let Some(key) = work.pop() {
        if !reached.insert(key.clone()) {
            continue;
        }
        for c in &groups[&(key.0.as_str(), key.1)] {
            if let Pattern::Var(p, _) = &c.params[key.2] {
                let locals = locals_for(c);
                for st in &c.body {
                    handed(stmt_expr(st), p, &locals, &user, &mut work);
                }
            }
        }
    }
    // of those, the ones read only as a view, largest set first: every
    // position whose clauses all take a plain name or `_`, then drop the ones
    // a clause's body uses otherwise until nothing more drops
    let mut safe: crate::hash::Set<(String, usize, usize)> = reached
        .into_iter()
        .filter(|(name, n, i)| {
            groups[&(name.as_str(), *n)].iter().all(|c| {
                !c.is_getter() && matches!(c.params[*i], Pattern::Var(..) | Pattern::Wildcard(_))
            })
        })
        .collect();
    loop {
        let mut drop = Vec::new();
        for key in &safe {
            let clauses = &groups[&(key.0.as_str(), key.1)];
            let holds = clauses.iter().all(|c| {
                let Pattern::Var(p, _) = &c.params[key.2] else { return true };
                let mut cx = Cx { groups: &groups, forwarders, safe: &safe, locals: locals_for(c) };
                // the parameter itself is not a callee the body may bind
                cx.locals.remove(p.as_str());
                let twice = c
                    .params
                    .iter()
                    .enumerate()
                    .any(|(j, q)| j != key.2 && matches!(q, Pattern::Var(o, _) if o == p));
                let shadowed = twice || bound_in(&c.body).contains(p.as_str());
                !shadowed && stmts_read_only(&c.body, p, &cx)
            });
            if !holds {
                drop.push(key.clone());
            }
        }
        if drop.is_empty() {
            break;
        }
        for key in drop {
            safe.remove(&key);
        }
    }
    let mut out = crate::hash::Set::default();
    for (d, i, x, h, span) in candidates {
        let locals = locals_for(d);
        if locals.contains(h) {
            continue;
        }
        let rest = &d.body[i + 1..];
        let mut params = crate::hash::Set::default();
        for p in &d.params {
            pattern_names(p, &mut params);
        }
        let rebound = params.contains(x) || bound_in(rest).contains(x);
        let mut cx = Cx { groups: &groups, forwarders, safe: &safe, locals };
        cx.locals.remove(x);
        if !rebound && stmts_read_only(rest, x, &cx) {
            out.insert((d.name.clone(), d.params.len(), span));
        }
    }
    out
}

/// The lookup over that index. Keyed by NAME with arity filtered off the
/// result, because the names come from call sites as well as declarations and
/// a `&str` key borrows as `str` where a tuple key would demand the program's
/// lifetime.
fn group_indices_in<'s>(
    by_name: &'s HashMap<&str, Vec<usize>>,
    program: &'s Program,
    name: &str,
    arity: usize,
) -> impl Iterator<Item = usize> + 's {
    by_name
        .get(name)
        .map_or(&[][..], |v| v.as_slice())
        .iter()
        .copied()
        .filter(move |at| program.fns[*at].params.len() == arity)
}

/// Whether `text` writes a branch to `label`: `label %name` with the name
/// ending there, so `fail1` is not answered by `label %fail10`.
fn branches_to(text: &str, label: &str) -> bool {
    let probe = format!("label %{label}");
    text.match_indices(&probe).any(|(at, _)| {
        !text[at + probe.len()..]
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.')
    })
}

/// Which interned strings the texts name, by index: whether `@sN` appears,
/// and whether `@sN_lit` does. `intern` names string N `sN`, so a name is read
/// as a number and nothing is hashed. A name ends where LLVM's unquoted names
/// do, at the first byte that is not a letter, digit, `_`, `.` or `$`, so
/// `@s12` never answers for `@s12_lit` or `@s120`.
///
/// Collecting every `@name` into a set and asking it was 23,334 instructions
/// of `kanso play`'s start-up on a one-line program, most of it hashing names
/// nothing would ask about.
fn named_strings(texts: &[&str], count: usize) -> Vec<[bool; 2]> {
    let name_byte = |b: u8| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'$');
    let mut named = vec![[false; 2]; count];
    for text in texts {
        let bytes = text.as_bytes();
        let mut at = 0;
        while let Some(next) = text[at..].find("@s") {
            let from = at + next + 2;
            at = from;
            let mut to = from;
            let mut n = 0usize;
            while to < bytes.len() && bytes[to].is_ascii_digit() {
                n = n.saturating_mul(10).saturating_add(usize::from(bytes[to] - b'0'));
                to += 1;
            }
            if to == from || n >= count {
                continue;
            }
            let ends = |i: usize| !bytes.get(i).is_some_and(|b| name_byte(*b));
            if ends(to) {
                named[n][0] = true;
            } else if bytes[to..].starts_with(b"_lit") && ends(to + 4) {
                named[n][1] = true;
            }
        }
    }
    named
}

/// Every `sym` for which `text` writes `@sym(`, collected in one pass.
///
/// This answers `text.contains(&format!("@{sym}("))` exactly, for any `sym`
/// that holds no newline and no `(` -- which every LLVM symbol here does.
/// From each `@` the scan runs to the first `(`, stopping at a newline, so a
/// probe can never straddle two lines; that matters because one of the three
/// places this replaces searched DECLARES line by line rather than whole.
fn called_symbols(text: &str) -> crate::hash::Set<&str> {
    let bytes = text.as_bytes();
    let mut found = crate::hash::Set::default();
    let mut at = 0;
    // `find` on an ascii char is memchr; `position` walked a byte a step.
    while let Some(next) = text[at..].find('@') {
        let from = at + next + 1;
        let mut to = from;
        while to < bytes.len() && bytes[to] != b'(' && bytes[to] != b'\n' {
            to += 1;
        }
        if to < bytes.len() && bytes[to] == b'(' {
            found.insert(&text[from..to]);
        }
        at = from;
    }
    found
}

/// Every `sym` for which `text` writes `@sym\n`, collected in one pass.
///
/// A line can only end in `@sym`, so only the LAST `@` on each line can start
/// a match and the whole text is read once. That makes the answer exact for a
/// `sym` holding no `@` and no newline, which every symbol the emitter asks
/// about is: `@a @b\n` would have to be found under the name `a @b`, and
/// reading each line's last at alone cannot see it.
///
/// The forward scan this replaced ran from every `@` to the end of its line,
/// which on a big emitted body is the text times the number of ats: it cost
/// `kanso build bench/runbench` 209 million instructions, turning a fall into
/// a rise.
fn symbols_before_newline(text: &str) -> crate::hash::Set<&str> {
    let mut found = crate::hash::Set::default();
    let mut start = 0;
    for (newline, _) in text.match_indices('\n') {
        let line = &text[start..newline];
        if let Some(at) = line.rfind('@') {
            found.insert(&line[at + 1..]);
        }
        start = newline + 1;
    }
    found
}

/// The symbols DECLARES calls from its own inline definitions, which is every
/// line of it that is not itself a `declare`. DECLARES is a constant, so this
/// is read once for the life of the process rather than once per emit.
/// The symbols DECLARES calls from its own inline definitions, written down
/// rather than scanned for.
///
/// DECLARES is a `const`: the same 65 names in every process kanso runs.
/// Reading them off it once cost 62 answers for 1,024 lines of scanning, and
/// the scanning was 94% of the index this branch builds -- 292,701 of the
/// 278,812 `called_symbols` charges on the start-up corpus came through
/// `Once::call_once_force`, over 1,023 calls, against 18,159 from the emitter's
/// two. On a program that emits one `print` that is the whole of the index's
/// cost, and `kanso play` paid it to learn nothing it could not have been told.
///
/// Sorted, which is what the oracle's binary search needs. Emit never asks
/// the list: each `declare` line reads `DeclareLine::context`, computed from
/// it when the compiler is built.
///
/// `tests/the_declares_symbols_are_the_ones_declares_calls.rs` recomputes this
/// list from DECLARES with the same scan it replaces and asserts they are the
/// same set, so an edit to DECLARES that adds or drops a call turns that spec
/// red rather than silently leaving a symbol out of a program's declares.
const DECLARES_CONTEXT_CALLS: &[&str] = &[
    "k_b_append",
    "k_b_append_byte",
    "k_b_append_mut",
    "k_b_append_mut_byte",
    "k_b_append_mut_int",
    "k_b_append_mut_int2",
    "k_b_append_mut_word",
    "k_b_append_slice",
    "k_b_append_slice_fast",
    "k_b_append_word_slow",
    "k_b_at",
    "k_b_at_fast",
    "k_b_bit_and",
    "k_b_bit_and_fast",
    "k_b_bit_not",
    "k_b_bit_not_fast",
    "k_b_bit_or",
    "k_b_bit_or_fast",
    "k_b_bit_shl",
    "k_b_bit_shl_fast",
    "k_b_bit_shr",
    "k_b_bit_shr_fast",
    "k_b_bit_xor",
    "k_b_bit_xor_fast",
    "k_b_bytes",
    "k_b_bytes_fast",
    "k_b_bytes_frame",
    "k_b_find2",
    "k_b_find2_below",
    "k_b_find2_below_fast",
    "k_b_find2_below_raw",
    "k_b_find2_fast",
    "k_b_find2_raw",
    "k_b_length",
    "k_b_length_fast",
    "k_b_push_mut",
    "k_b_push_mut_fast",
    "k_b_put_mut",
    "k_b_put_mut_fast",
    "k_b_slice",
    "k_b_slice_fast",
    "k_b_slice_raw",
    "k_b_utf8_slice",
    "k_b_utf8_slice_fast",
    "k_b_utf8_slice_raw",
    "k_bool",
    "k_check_bool",
    "k_check_int",
    "k_check_rec",
    "k_check_rec_fast",
    "k_check_rec_fast_w",
    "k_check_tag",
    "k_field",
    "k_field_fast",
    "k_float",
    "k_force",
    "k_force_fast",
    "k_index",
    "k_index_fast",
    "k_int",
    "k_none",
    "k_str_lit",
    "k_str_lit_fast",
    "k_truthy",
    "k_truthy_bad",
    "k_truthy_w",
    "llvm.assume",
    "llvm.memcpy.p0.p0.i64",
];

/// The binary search each `declare` line used to ask at emit time, kept as
/// the oracle for `DeclareLine::context`.
#[cfg(test)]
fn declares_context_calls(sym: &str) -> bool {
    DECLARES_CONTEXT_CALLS.binary_search(&sym).is_ok()
}

#[cfg(test)]
mod every_declare_line_knows_whether_declares_calls_it {
    use super::{declares_context_calls, DECLARES, DECLARES_DEV, DECLARE_LINES, DECLARE_LINES_DEV};

    #[test]
    fn the_flag_is_the_search() {
        let mut asked = 0;
        for (text, lines) in [(DECLARES, &DECLARE_LINES), (DECLARES_DEV, &DECLARE_LINES_DEV)] {
            for line in lines.iter().filter(|l| l.sym_start < l.sym_end) {
                let sym = &text[line.sym_start..line.sym_end];
                assert_eq!(line.context, declares_context_calls(sym), "{sym}");
                asked += usize::from(line.context);
            }
        }
        assert!(asked > 0, "no declare line is one DECLARES calls, so this proves nothing");
    }
}

#[cfg(test)]
mod the_declares_symbols_are_the_ones_declares_calls {
    use super::{called_symbols, DECLARES, DECLARES_CONTEXT_CALLS};

    /// The scan the static replaces, kept as the oracle.
    fn scanned() -> std::collections::BTreeSet<&'static str> {
        let mut found = std::collections::BTreeSet::new();
        for line in DECLARES.lines().filter(|l| !l.starts_with("declare")) {
            found.extend(called_symbols(line));
        }
        found
    }

    /// Written down and scanned for are the same set. An edit to DECLARES that
    /// adds or drops a call turns this red rather than leaving a symbol out of
    /// a program's declares, where the only symptom is a link error in whatever
    /// program happens to reach it.
    #[test]
    fn the_written_list_is_what_the_scan_finds() {
        let written: std::collections::BTreeSet<&str> =
            DECLARES_CONTEXT_CALLS.iter().copied().collect();
        let found = scanned();
        let missing: Vec<&&str> = found.difference(&written).collect();
        let extra: Vec<&&str> = written.difference(&found).collect();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "DECLARES_CONTEXT_CALLS has drifted from DECLARES.\n               missing (DECLARES calls it, the list does not name it): {missing:?}\n               extra (the list names it, DECLARES does not call it): {extra:?}"
        );
    }

    /// And it is sorted, because it is asked with a binary search.
    #[test]
    fn the_list_is_sorted_and_holds_no_duplicate() {
        let mut sorted = DECLARES_CONTEXT_CALLS.to_vec();
        sorted.sort_unstable();
        assert_eq!(
            DECLARES_CONTEXT_CALLS,
            &sorted[..],
            "the list is asked with binary_search, which answers nonsense on an \
             unsorted slice and answers it quietly"
        );
        sorted.dedup();
        assert_eq!(sorted.len(), DECLARES_CONTEXT_CALLS.len(), "the list repeats a symbol");
    }

    /// The population is not empty, so the two tests above are not agreeing
    /// about nothing.
    #[test]
    fn the_list_is_not_empty() {
        assert!(
            DECLARES_CONTEXT_CALLS.len() > 40,
            "only {} symbols, where DECLARES's inline definitions called sixty-two \
             when this was written",
            DECLARES_CONTEXT_CALLS.len()
        );
    }
}

#[cfg(test)]
mod called_symbols_agrees {
    use super::*;

    /// What the emitter asked before the index existed, kept as the oracle.
    fn searched(text: &str, sym: &str) -> bool {
        text.contains(&format!("@{sym}("))
    }

    /// Every query the emitter makes, asked both ways, over a text.
    ///
    /// The symbols are DECLARES's own -- the real 163 -- plus every prefix and
    /// suffix of each, because a span-based index can agree on a whole name
    /// and still disagree on one letter of it, and the emitter's question is
    /// `@sym(` where a prefix would answer for a longer symbol if the index
    /// stored anything looser than the run up to the paren.
    fn agrees_over(text: &str) {
        let index = called_symbols(text);
        let mut asked = 0;
        for line in DECLARES.lines() {
            let Some(rest) = line.strip_prefix("declare ") else { continue };
            let Some(at) = rest.find('@') else { continue };
            let sym = &rest[at + 1..];
            let Some(paren) = sym.find('(') else { continue };
            let sym = &sym[..paren];
            for end in 1..=sym.len() {
                for start in 0..end {
                    let part = &sym[start..end];
                    assert_eq!(
                        index.contains(part),
                        searched(text, part),
                        "the index and the search disagree on `@{part}(`"
                    );
                    asked += 1;
                }
            }
        }
        assert!(asked > 10_000, "the corpus asked only {asked} questions");
    }

    #[test]
    fn over_the_declares_block() {
        agrees_over(DECLARES);
    }

    /// The `@cell` form, which matches only at the end of a line.
    ///
    /// The scan reads each line's LAST at, so this is the shape that goes
    /// wrong when a line holds more than one.
    #[test]
    fn a_cell_is_named_only_where_its_line_ends() {
        for text in [
            "@a\n",
            "@a",      // no newline, so nothing matches
            "@a @b\n", // only the last at can end the line
            "@a(\n",
            "  @a\n  @b\n",
            "@\n",
            "",
            "@a\n@a\n",
        ] {
            let index = symbols_before_newline(text);
            // Every symbol the emitter asks about is a generated identifier,
            // so the queries here hold no at and no newline -- see the note on
            // symbols_before_newline for the one shape outside that.
            for sym in ["a", "b", "", "a("] {
                assert_eq!(
                    index.contains(sym),
                    text.contains(&format!("@{sym}\n")),
                    "index and search disagree on `@{sym}\\n` in {text:?}"
                );
            }
        }
    }

    /// The edges a span scan can get wrong, each as a whole text.
    #[test]
    fn the_shapes_a_span_scan_can_lose() {
        for text in [
            "@a(",              // the plain case
            "@a",               // no paren at all
            "@a\n(",            // a newline between the name and the paren
            "@@a(",             // two ats, and `@a(` is still in there
            "@a@b(",            // an at inside the span
            "  call @a(i64 1)", // the shape a real line has
            "@a(@b(@c(",        // three in one line
            "",                 // nothing
            "@",                // an at and nothing after it
        ] {
            let index = called_symbols(text);
            for sym in ["a", "b", "c", "@a", "a@b", "", "ab"] {
                assert_eq!(
                    index.contains(sym),
                    searched(text, sym),
                    "index and search disagree on `@{sym}(` in {text:?}"
                );
            }
        }
    }
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

#[cfg(test)]
mod the_prune_agrees_with_the_search {
    use super::*;

    /// The mark written the slow way, kept as the oracle.
    ///
    /// It asks `names_symbol` once per (block, name) pair, which is what made
    /// the search this replaced 72.10% of `kanso build bench/runbench`. It is
    /// the definition of the right answer and nothing else, so it stays here
    /// rather than in the commit message.
    fn by_search(body: &str, entry: &str, cells: &[(String, String, usize)]) -> String {
        let blocks = ir_defines(body);
        let candidate = |sym: &str| {
            sym != entry
                && sym != "d_thunk_eval"
                && (sym.starts_with("d_")
                    || sym.starts_with("w_")
                    || sym.starts_with("klam")
                    || sym.starts_with("\"d_")
                    || sym.starts_with("\"w_"))
        };
        let mut alive: Vec<bool> = blocks.iter().map(|(sym, _)| !candidate(sym)).collect();
        loop {
            let named = |sym: &str| {
                blocks.iter().zip(&alive).any(|((_, text), live)| *live && names_symbol(text, sym))
            };
            let woken = blocks.iter().enumerate().position(|(at, (sym, _))| {
                !alive[at]
                    && (named(sym) || cells.iter().any(|(cell, w, _)| w == sym && named(cell)))
            });
            match woken {
                Some(at) => alive[at] = true,
                None => break,
            };
        }
        blocks.into_iter().zip(alive).filter(|(_, live)| *live).map(|((_, text), _)| text).collect()
    }

    fn define(sym: &str, body: &str) -> String {
        format!("define %KValue @{sym}() {{\nentry:\n{body}\n}}\n")
    }

    fn agree_on(body: &str, entry: &str, cells: &[(String, String, usize)]) {
        assert_eq!(
            prune_unnamed(body, entry, cells),
            by_search(body, entry, cells),
            "the index and the search kept different blocks"
        );
    }

    /// A chain the fixpoint has to walk down: the entry names nothing, so
    /// `d_a` goes, and only then is `d_b` unnamed, and only then `d_c`.
    #[test]
    fn a_chain_that_falls_one_round_at_a_time() {
        let body = [
            define("d_entry", "  ret %KValue zeroinitializer"),
            define("d_a", "  %x = call %KValue @d_b()\n  ret %KValue %x"),
            define("d_b", "  %x = call %KValue @d_c()\n  ret %KValue %x"),
            define("d_c", "  ret %KValue zeroinitializer"),
        ]
        .concat();
        assert!(!prune_unnamed(&body, "d_entry", &[]).contains("@d_c("));
        agree_on(&body, "d_entry", &[]);
    }

    /// Blocks that name each other and nothing live names. std/list's merge
    /// sort is four of them calling round, and counting mentions kept all
    /// four in every program that imported the library and never sorted.
    #[test]
    fn a_cycle_nothing_reaches_goes_whole() {
        let body = [
            define("d_entry", "  %x = call %KValue @d_kept()\n  ret %KValue %x"),
            define("d_kept", "  ret %KValue zeroinitializer"),
            define("d_merge", "  %x = call %KValue @d_pick()\n  ret %KValue %x"),
            define("d_pick", "  %x = call %KValue @d_merge()\n  ret %KValue %x"),
        ]
        .concat();
        let kept = prune_unnamed(&body, "d_entry", &[]);
        assert!(kept.contains("@d_kept("), "the named block was pruned");
        assert!(!kept.contains("@d_merge("), "a cycle nothing reaches was kept");
        assert!(!kept.contains("@d_pick("), "a cycle nothing reaches was kept");
        agree_on(&body, "d_entry", &[]);
    }

    /// The delimiter rule, which is the whole reason `names_symbol` exists:
    /// `@w_klam1` must not answer for `@w_klam17`.
    #[test]
    fn a_longer_name_does_not_answer_for_a_shorter_one() {
        let body = [
            define("d_entry", "  %x = call %KValue @w_klam17()\n  ret %KValue %x"),
            define("w_klam1", "  ret %KValue zeroinitializer"),
            define("w_klam17", "  ret %KValue zeroinitializer"),
        ]
        .concat();
        let kept = prune_unnamed(&body, "d_entry", &[]);
        assert!(kept.contains("@w_klam17("), "the named wrapper was pruned");
        assert!(!kept.contains("define %KValue @w_klam1()"), "the unnamed wrapper was kept");
        agree_on(&body, "d_entry", &[]);
    }

    /// A quoted name, which is how `quoted()` spells one holding a slash. The
    /// one-pass scan reads it from quote to quote; a run over the delimiter
    /// class alone would stop at the slash and lose it.
    #[test]
    fn a_quoted_name_is_read_to_its_closing_quote() {
        let body = [
            define("d_entry", "  %x = call %KValue @\"d_add/2\"()\n  ret %KValue %x"),
            define("\"d_add/2\"", "  ret %KValue zeroinitializer"),
            define("\"d_sub/2\"", "  ret %KValue zeroinitializer"),
        ]
        .concat();
        let kept = prune_unnamed(&body, "d_entry", &[]);
        assert!(kept.contains("@\"d_add/2\"("), "the named quoted block was pruned");
        assert!(!kept.contains("@\"d_sub/2\"()"), "the unnamed quoted block was kept");
        agree_on(&body, "d_entry", &[]);
    }

    /// A lambda body is a candidate too. Its wrapper names it, so it goes in
    /// the round after the wrapper does; one a closure is built over is named
    /// by that construction and stays.
    #[test]
    fn a_lambda_body_goes_with_the_wrapper_that_named_it() {
        let body = [
            define(
                "d_entry",
                "  %c = call %KValue @k_closure(ptr @klam5, i64 1)\n  ret %KValue %c",
            ),
            define("w_klam3", "  %x = call %KValue @klam3()\n  ret %KValue %x"),
            define("klam3", "  ret %KValue zeroinitializer"),
            define("klam5", "  ret %KValue zeroinitializer"),
        ]
        .concat();
        let kept = prune_unnamed(&body, "d_entry", &[]);
        assert!(kept.contains("define %KValue @klam5("), "the lambda a closure names was pruned");
        assert!(!kept.contains("define %KValue @klam3("), "the lambda nothing reaches was kept");
        agree_on(&body, "d_entry", &[]);
    }

    /// A wrapper is kept when a CELL that stands for it is named, which is the
    /// second half of the question and the one that reads `cells`.
    #[test]
    fn a_wrapper_lives_while_its_cell_is_named() {
        let body = [
            define("d_entry", "  %x = load %KValue, ptr @k_clo_7\n  ret %KValue %x"),
            define("w_klam3", "  ret %KValue zeroinitializer"),
            define("w_klam4", "  ret %KValue zeroinitializer"),
        ]
        .concat();
        let cells = [("k_clo_7".to_string(), "w_klam3".to_string(), 1usize)];
        let kept = prune_unnamed(&body, "d_entry", &cells);
        assert!(kept.contains("@w_klam3("), "the wrapper its cell names was pruned");
        assert!(!kept.contains("@w_klam4("), "the wrapper nothing names was kept");
        agree_on(&body, "d_entry", &cells);
    }

    /// A name the one-pass scan cannot tokenise falls back to the search, so
    /// the index is exact without assuming how a symbol is spelled.
    #[test]
    fn an_odd_name_is_answered_the_old_way() {
        assert!(one_pass_reads("d_foo"));
        assert!(one_pass_reads("\"d_add/2\""));
        assert!(!one_pass_reads("d_foo.bar"));
        assert!(!one_pass_reads("\"un\"closed"));
        let body = [
            define("d_entry", "  %x = call %KValue @d_foo.bar()\n  ret %KValue %x"),
            define("d_foo.bar", "  ret %KValue zeroinitializer"),
            define("d_foo.baz", "  ret %KValue zeroinitializer"),
        ]
        .concat();
        let kept = prune_unnamed(&body, "d_entry", &[]);
        assert!(kept.contains("@d_foo.bar("), "the named odd block was pruned");
        assert!(!kept.contains("@d_foo.baz("), "the unnamed odd block was kept");
        agree_on(&body, "d_entry", &[]);
    }

    /// Real emitted IR, with the symbols the emitter really writes.
    ///
    /// The text is what `emit_ir` produced, so the prune has already run over
    /// it once; feeding it back asks both implementations the same several
    /// hundred questions about a body neither was written against.
    #[test]
    fn real_emitted_ir_reads_the_same_both_ways() {
        let source = "fn count 0 acc\n  acc\n\nfn count n acc\n  count (n - 1) (acc + n)\n\nmain = print \"{count 10 0}\"\n";
        let program = crate::compile("sample.kso", source, false).expect("the sample compiles");
        let ir = emit_ir(&program, ClosureConvention::Absent).expect("the sample lowers");
        assert!(ir.lines().count() > 200, "the sample emitted only {} lines", ir.lines().count());
        for entry in ["d_main", "d_count"] {
            agree_on(&ir, entry, &[]);
        }
    }
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
fn call_twin(n: usize, convention: ClosureConvention, inline: bool) -> String {
    let args: String = (0..n).map(|i| format!(", %KValue %a{i}")).collect();
    let mut s = String::new();
    let attr = if inline { " alwaysinline" } else { "" };
    let _ = writeln!(s, "define internal %KValue @k_call{n}_fast(%KValue %f{args}){attr} {{");
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
    // The closure pointer this loads was written by the wrapper emitter, so
    // the two carry the same convention or the arguments land in the wrong
    // registers. `%fnp` is only ever a closure body: the arm tested tag == 11
    // above, so the fnref family (no env parameter) never reaches here.
    let cc = convention.keyword();
    let _ = writeln!(s, "  %r = call {cc}%KValue %fnp(ptr %env{args})");
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

/// The ambient group an interpolated value may dispatch to.
const RENDER_GROUP: &str = "render/to_string";

fn dsym(name: &str, arity: usize) -> String {
    quoted(&format!("d_{name}_{arity}"))
}

/// A lookup from a type id to one of `names`, read out of a constant array
/// rather than a switch, with `fallback` for an id past the end.
fn type_table(globals: &mut String, symbol: &str, names: &[String], fallback: &str) {
    let slots = names.len();
    let row: Vec<String> = names.iter().map(|n| format!("ptr @{n}")).collect();
    let _ = writeln!(
        globals,
        "@{symbol}_table = private unnamed_addr constant [{slots} x ptr] [{}]",
        row.join(", ")
    );
    let _ = writeln!(
        globals,
        "define ptr @{symbol}(i64 %id) {{\nentry:\n  %in = icmp ult i64 %id, {slots}\n  \
         br i1 %in, label %T, label %TD\nT:\n  \
         %at = getelementptr [{slots} x ptr], ptr @{symbol}_table, i64 0, i64 %id\n  \
         %name = load ptr, ptr %at\n  ret ptr %name\nTD:\n  ret ptr @{fallback}\n}}\n"
    );
}

/// Word `n` of `value` when it is a literal `{ i64 A, i64 B }`. Reading one
/// off a constant with `extractvalue` is an instruction clang's fast selector
/// at -O0 does not handle, and it sends the rest of the block to the slow one.
fn literal_word(value: &str, n: usize) -> Option<&str> {
    let inner = value.strip_prefix("{ i64 ")?.strip_suffix(" }")?;
    let (a, b) = inner.split_once(", i64 ")?;
    Some(if n == 0 { a } else { b })
}

/// An `i1` saying both tags are the int tag, 0. A tag already known to be 0,
/// a literal's or an unboxed parameter's, needs no compare, and two such
/// need no `and`: `n + 1` used to write `icmp eq i64 0, 0` and an `and` for
/// the literal on every addition.
fn both_ints(f: &mut FnEmit, ta: &str, tb: &str) -> String {
    let mut tests: Vec<String> = Vec::new();
    for tag in [ta, tb] {
        if tag != "0" {
            let t = f.tmp();
            f.line(&format!("{t} = icmp eq i64 {tag}, 0"));
            tests.push(t);
        }
    }
    match tests.as_slice() {
        [] => "true".to_string(),
        [one] => one.clone(),
        [a, b] => {
            let both = f.tmp();
            f.line(&format!("{both} = and i1 {a}, {b}"));
            both
        }
        _ => unreachable!("two tags make at most two tests"),
    }
}

/// Two in-place byte appends in a row, the second onto the first's result and
/// that result read nowhere else in the function, as one call to
/// `k_b_append_mut_int2`. Both calls were already the proven-bytes,
/// proven-int door, so the pair asks the same questions once; see the helper.
///
/// The search goes from one call to the next rather than line by line, and a
/// body with no pair comes back as it was, because this runs on every build
/// and most programs have none.
fn paired_appends(body: &str) -> std::borrow::Cow<'_, str> {
    const CALL: &str = " = call %KValue @k_b_append_mut_int(%KValue ";
    fn parse(line: &str) -> Option<(&str, &str, &str)> {
        let rest = line.strip_prefix("  ")?;
        let (name, rest) = rest.split_once(CALL)?;
        let args = rest.strip_suffix(')')?;
        let (acc, x) = args.split_once(", %KValue ")?;
        Some((name, acc, x))
    }
    let line_end = |from: usize| body[from..].find('\n').map_or(body.len(), |n| from + n);
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    let mut from = 0;
    while let Some(n) = body[from..].find(CALL) {
        let at = from + n;
        let start = body[..at].rfind('\n').map_or(0, |n| n + 1);
        let end = line_end(at);
        from = end;
        if end >= body.len() {
            break;
        }
        let next = line_end(end + 1);
        let (Some((a, acc, x)), Some((b, acc2, y))) =
            (parse(&body[start..end]), parse(&body[end + 1..next]))
        else {
            continue;
        };
        if acc2 != a {
            continue;
        }
        // a use count is the function's own
        let first = body[..start].rfind("\ndefine ").map_or(0, |n| n + 1);
        let last = body[next..].find("\n}\n").map_or(body.len(), |n| next + n);
        let used: usize = body[first..last].lines().map(|l| count_operand(l, a)).sum();
        if used == 2 {
            edits.push((
                start,
                next,
                format!("  {b} = call %KValue @k_b_append_mut_int2(%KValue {acc}, %KValue {x}, %KValue {y})"),
            ));
            from = next;
        }
    }
    if edits.is_empty() {
        return std::borrow::Cow::Borrowed(body);
    }
    let mut out = String::with_capacity(body.len());
    let mut kept = 0;
    for (start, end, line) in edits {
        out.push_str(&body[kept..start]);
        out.push_str(&line);
        kept = end;
    }
    out.push_str(&body[kept..]);
    std::borrow::Cow::Owned(out)
}

/// How many times `name` appears in `line` as a whole operand.
fn count_operand(line: &str, name: &str) -> usize {
    let bytes = line.as_bytes();
    line.match_indices(name)
        .filter(|(at, _)| {
            let after = bytes.get(at + name.len()).copied().unwrap_or(b' ');
            !(after.is_ascii_alphanumeric() || after == b'_' || after == b'.')
        })
        .count()
}

/// One read of a byte run: `x[p + k] == c` or `x[p] == c`, non-strict, with
/// `c` a byte literal and `k` a small literal. Answers `(x, p, k, c)`.
fn byte_read(e: &Expr) -> Option<(&str, &str, i64, i64)> {
    use num_traits::ToPrimitive;
    let Expr::BinOp { op: "==", lhs, rhs, .. } = e else { return None };
    let Expr::Index { base, index, strict: false, .. } = &**lhs else { return None };
    let Expr::Ident(x, _, _) = &**base else { return None };
    let Expr::Int(c, _) = &**rhs else { return None };
    let c = c.to_i64().filter(|c| (0..=255).contains(c))?;
    let (p, k) = match &**index {
        Expr::Ident(p, _, _) => (p, 0),
        Expr::BinOp { op: "+", lhs, rhs, .. } => {
            let Expr::Ident(p, _, _) = &**lhs else { return None };
            let Expr::Int(k, _) = &**rhs else { return None };
            (p, k.to_i64().filter(|k| k.abs() < 1 << 20)?)
        }
        _ => return None,
    };
    Some((x.as_str(), p.as_str(), k, c))
}

/// The bytes and the position a run reads, and each read's offset and byte.
type ByteRun<'e> = (&'e str, &'e str, Vec<(i64, i64)>);

/// A conjunction of `byte_read`s of the same two names. `a and b` desugars to
/// `if a b false`, and `and` groups to the left, so `r1 and r2 and r3` is
/// `if (if r1 r2 false) r3 false`; either side of an `if` may be another. Two
/// reads at least; one is an ordinary compare.
fn byte_run(args: &[Expr]) -> Option<ByteRun<'_>> {
    fn conj<'e>(e: &'e Expr, out: &mut Vec<(&'e str, &'e str, i64, i64)>) -> Option<()> {
        if let Some(read) = byte_read(e) {
            out.push(read);
            return Some(());
        }
        let Expr::App { head, args, piped: false, .. } = e else { return None };
        if !matches!(&**head, Expr::Ident(n, _, _) if n == "if") {
            return None;
        }
        both(args, out)
    }
    fn both<'e>(args: &'e [Expr], out: &mut Vec<(&'e str, &'e str, i64, i64)>) -> Option<()> {
        if args.len() != 3 || !matches!(&args[2], Expr::Ident(n, _, _) if n == "false") {
            return None;
        }
        conj(&args[0], out)?;
        conj(&args[1], out)
    }
    let mut reads = Vec::new();
    both(args, &mut reads)?;
    let (x, p, _, _) = *reads.first()?;
    if reads.len() < 2 || reads.iter().any(|r| (r.0, r.1) != (x, p)) {
        return None;
    }
    Some((x, p, reads.iter().map(|r| (r.2, r.3)).collect()))
}

fn inline_tag(f: &mut FnEmit, value: &str) -> String {
    if let Some(word) = literal_word(value, 0) {
        return word.to_string();
    }
    if let Some((tag, _)) = f.known_words.get(value) {
        return tag.clone();
    }
    let t = f.tmp();
    f.line(&format!("{t} = extractvalue %KValue {value}, 0"));
    t
}

fn inline_payload(f: &mut FnEmit, value: &str) -> String {
    if let Some(word) = literal_word(value, 1) {
        return word.to_string();
    }
    if let Some((_, payload)) = f.known_words.get(value) {
        return payload.clone();
    }
    let t = f.tmp();
    f.line(&format!("{t} = extractvalue %KValue {value}, 1"));
    t
}

/// Branch to `hit` when a 1-based index lands inside the container whose
/// length `len_ptr` points at, and to `miss` when it does not, one signed
/// compare at a time. The length is loaded between the two, which keeps LLVM
/// from folding the pair back into flags and an `and`. The signed spelling
/// is the one a guard such as `return acc if n < 1 or length bs < n` proves,
/// so LLVM can drop both where one ran upstream.
fn index_in_range(f: &mut FnEmit, idx: &str, len_ptr: &str, hit: &str, miss: &str) {
    let ge1 = f.tmp();
    f.line(&format!("{ge1} = icmp sge i64 {idx}, 1"));
    let above = f.label();
    f.line(&format!("br i1 {ge1}, label %{above}, label %{miss}"));
    f.start_block(&above);
    let len = f.tmp();
    f.line(&format!("{len} = load i64, ptr {len_ptr}"));
    assume_length(f, &len);
    let le_len = f.tmp();
    f.line(&format!("{le_len} = icmp sle i64 {idx}, {len}"));
    f.line(&format!("br i1 {le_len}, label %{hit}, label %{miss}"));
}

/// A length read out of a container is never negative, and saying so lets
/// LLVM merge a `1 <= i <= len` test into one unsigned compare where the
/// signed pair also stays visible to whatever proved it true upstream.
fn assume_length(f: &mut FnEmit, len: &str) {
    let ok = f.tmp();
    f.line(&format!("{ok} = icmp sge i64 {len}, 0"));
    f.line(&format!("call void @llvm.assume(i1 {ok})"));
}

/// Whether a value is not a failure, as one compare of its tag against the
/// err tag.
fn inline_not_failure(f: &mut FnEmit, value: &str) -> String {
    let set = f.set_of(value);
    // An EMPTY set is not a proof. `group_param_set` answers 0 for a parameter
    // the inference reached no shapes for, and 0 satisfies every `& mask == 0`
    // test written about it -- so the first draft of this fold read "nothing is
    // known" as "proved to be a boolean" and turned the hop below into `true`.
    // A group whose argument is a failure then dispatched instead of hopping
    // and died on `no overload matches these arguments`. The micro corpus
    // caught it: a_construction_merges_its_failures lost its third line.
    if set != 0 && set & !infer::BOOL == 0 {
        return "true".to_string();
    }
    not_failure_test(f, value)
}

/// The same test with no fold, for the one place a set cannot speak: a block
/// reached BECAUSE a value is outside the set recorded for it.
///
/// It was a call to an alwaysinline `k_not_failure`, then a compare of what
/// that returned. Every module paid for the inliner to open the call, and the
/// dev tier, which does no other optimising, kept the widening and the second
/// compare as well. The test is the one the runtime's `k_not_failure` makes,
/// and `tests/the_err_tag_is_the_runtime_s.rs` holds the number to the
/// runtime's.
fn not_failure_test(f: &mut FnEmit, value: &str) -> String {
    let tag = inline_tag(f, value);
    if let Ok(known) = tag.parse::<i64>() {
        return (known != K_ERR_TAG).to_string();
    }
    let ok = f.tmp();
    f.line(&format!("{ok} = icmp ne i64 {tag}, {K_ERR_TAG}"));
    ok
}

/// The runtime's `K_ERR`, the sixth tag in its enum.
pub const K_ERR_TAG: i64 = 5;

impl<'a> Backend<'a> {
    /// A group's declaration indices, read out of the index rather than
    /// scanned for. Program order, which is what the scan gave.
    fn group_indices<'s>(&'s self, name: &str, arity: usize) -> impl Iterator<Item = usize> + 's {
        group_indices_in(&self.group_by_name, self.program, name, arity)
    }

    fn group_param_set(&self, name: &str, arity: usize, param: usize) -> Set {
        self.group_indices(name, arity).fold(0, |acc, i| acc | self.inference.param(i, param))
    }

    fn group_return_set(&self, name: &str, arity: usize) -> Set {
        self.group_indices(name, arity).fold(0, |acc, i| acc | self.inference.returns[i])
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
            Some(Expr::Ident(_, span, _)) => self.builder_carried.contains(&(
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
                // A builder seeded with a subtype of string builds from the
                // string, which only a program declaring a subtype can hand it.
                let e = match self.sub_parents.is_empty() {
                    true => e.to_string(),
                    false => {
                        let u = f.tmp();
                        f.line(&format!("{u} = call %KValue @k_unsub(%KValue {e})"));
                        u
                    }
                };
                let t = f.tmp();
                f.line(&format!("{t} = call %KValue @k_b_str_builder(%KValue {e})"));
                f.record(&t, f.set_of(&e));
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
            if let Some(raw) = f.raw_byte.get(e) {
                // The index already merged the two arms as an i64; take that
                // and the box goes unread.
                return format!("i64 {raw}");
            }
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
            // passed outside tail position, or a carry take): unpack it into
            // the convention. A failure in the slot is its own two words, and
            // the runtime hands them over unread rather than reading fields
            // off a value that has none.
            let u = f.tmp();
            f.line(&format!("{u} = call %KValue @k_parsed_words(%KValue {e})"));
            let w0 = f.tmp();
            f.line(&format!("{w0} = extractvalue %KValue {u}, 0"));
            let w1 = f.tmp();
            f.line(&format!("{w1} = extractvalue %KValue {u}, 1"));
            let a = f.tmp();
            f.line(&format!("{a} = insertvalue %parsed undef, i64 {w0}, 0"));
            let p = f.tmp();
            f.line(&format!("{p} = insertvalue %parsed {a}, i64 {w1}, 1"));
            format!("%parsed {p}")
        } else if self.unboxed_param(callee, arity, i) {
            let p = inline_payload(f, e);
            format!("i64 {p}")
        } else {
            format!("%KValue {e}")
        }
    }

    /// Emit the entry-block reboxes that reconstruct each unboxed `%xi` param as
    /// the KValue the body expects.
    /// What inference proved about each slot, said out loud at the entry.
    ///
    /// The ladder dispatcher has always had this: a bare `Var` parameter
    /// records its group's set as it binds, so a body indexing that slot
    /// skips the tag test whose answer the front end already holds. The
    /// switch dispatcher binds the name straight to `%x{i}` — the switch has
    /// decided what the value is, so there is no pattern left to emit — and
    /// recorded nothing, so every group it took over went back to paying the
    /// test. Three of the decoder's four hot groups are switch-shaped, and
    /// jsonbench carried 16,017,450 instructions of tag test on that account.
    ///
    /// The set keeps its failure bits. A ladder's `Var` arm can drop them
    /// because the pattern it just emitted checked them off; a switch's
    /// default arm is reached BY a failing discriminator, so here they stay.
    ///
    /// Called from both dispatchers. On the ladder it moves no emitted line
    /// in today's corpus — every parameter a body indexes there is a bare
    /// `Var`, which already recorded — and it is called anyway so the two
    /// dispatchers tell a body the same things.
    fn record_param_sets(&self, f: &mut FnEmit, name: &str, arity: usize) {
        for i in 0..arity {
            let set = self.group_param_set(name, arity, i);
            f.record(&format!("%x{i}"), set);
            self.assume_param_tag(f, name, arity, i, set);
        }
    }

    /// A boxed parameter inference proves is one kind of heap value says so
    /// to LLVM, which then folds every test of that parameter's tag in the
    /// body. `record` tells this emitter the same thing, but the helpers it
    /// inlines -- `k_b_push_mut_fast` asking whether its list is a list --
    /// test the tag word themselves, and only LLVM sees into them.
    ///
    /// Not in a program that declares a subtype: a subtype's value carries tag
    /// 15 whatever it holds, which is why `tag_switch_shape` refuses the same
    /// programs.
    fn assume_param_tag(&self, f: &mut FnEmit, name: &str, arity: usize, i: usize, set: Set) {
        if !self.sub_parents.is_empty()
            || self.is_byte_disc(name, arity, i)
            || self.escape.carries_ty(name, arity, i).is_some()
            || self.unboxed_param(name, arity, i)
        {
            return;
        }
        self.assume_tag(f, &format!("%x{i}"), set);
    }

    /// `llvm.assume` that a boxed value's tag is the one kind its set allows,
    /// for the kinds whose set bit names exactly one runtime tag. The caller
    /// has already refused programs that declare a subtype.
    fn assume_tag(&self, f: &mut FnEmit, value: &str, set: Set) {
        let tag = match set {
            infer::FLOAT => 1,
            infer::STR => 6,
            infer::REC => 7,
            infer::LIST => 9,
            infer::MAP => 10,
            infer::BYTES => 13,
            _ => return,
        };
        let t = f.tmp();
        f.line(&format!("{t} = extractvalue %KValue {value}, 0"));
        let is = f.tmp();
        f.line(&format!("{is} = icmp eq i64 {t}, {tag}"));
        f.line(&format!("call void @llvm.assume(i1 {is})"));
    }

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
                f.known_words.insert(format!("%x{i}"), ("0".to_string(), format!("%x{i}r")));
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

    /// Whether a rendered value goes through the ambient `render/to_string`
    /// group: a set carrying REC may hit a user arm, so it is routed there.
    /// Primitive-only sets keep the direct call — coherence proves no arm can
    /// exist for them (design/render-plan.md).
    fn render_dispatchable(&self, f: &FnEmit, value: &str) -> bool {
        f.set_of(value) & (REC | NONE | infer::DONE | DESC) != 0
            && self.program.fns.iter().any(|d| d.name == RENDER_GROUP)
    }

    /// The string an interpolated `value` renders to, and the fail set it
    /// adds: only an err propagates out of an interpolation (a none renders
    /// `<none>`, so it is not a fail), and a dispatched arm may fail on its
    /// own.
    fn render_interp(&self, f: &mut FnEmit, value: &str) -> (String, Set) {
        let mut fails = f.set_of(value) & ERR;
        let dispatchable = self.render_dispatchable(f, value);
        let t = f.tmp();
        let value = self.as_value(f, value);
        match dispatchable {
            true => {
                f.line(&format!(
                    "{t} = call tailcc %KValue @{}(%KValue {value})",
                    dsym(RENDER_GROUP, 1)
                ));
                fails |= ERR;
            }
            false => f.line(&format!("{t} = call %KValue @k_render(%KValue {value}, i64 0)")),
        }
        (t, fails)
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
            if let Expr::Ident(n, _, _) | Expr::Partial(n, _) = expr {
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
        let mut f = FnEmit::new(!self.inline_helpers);
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
            f.body()
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
            true => prune_unnamed(&self.body, &dsym(crate::ast::ENTRY, 0), &self.closure_consts),
            false => self.body.clone(),
        };
        let body = paired_appends(&body);
        // One inline dispatcher per arity the program actually writes. An
        // unused `internal` definition costs nothing after optimization, but
        // it does cost a line, a define and a branch in the emitted golden —
        // so the twins are generated against the body rather than carried in
        // DECLARES the way the other inline helpers are.
        // Every `@sym(` the body writes, read off in one pass. The question
        // below is asked once per declare line -- 163 of them -- and used to
        // be answered by searching the whole emitted body for each, which is
        // 163 scans of a text that grows with the program. On a build of
        // bench/compile_corpus that search was 17.08% of the process.
        let body_calls = called_symbols(&body);
        let body_lines = symbols_before_newline(&body);
        let call_twins: String = (0..=4)
            .filter(|n| body_calls.contains(format!("k_call{n}_fast").as_str()))
            .map(|n| call_twin(n, self.convention, self.inline_helpers))
            .collect();
        let declares: String = {
            // BOTH SIDES OF THIS BUILT THE SAME INDEX. kanso#1461 landed one
            // inline, joining DECLARES’s non-declare lines and scanning the
            // three haystacks in place; this branch had already factored the
            // scan into `called_symbols` and cached the DECLARES half in a
            // `OnceLock`, so `body_calls` is computed once above and reused by
            // the call-twin filter rather than rebuilt here. The sets are the
            // same for every query the emitter makes: a symbol from a
            // `declare` line holds no whitespace and no `(`, so bounding the
            // span at the first `(` and bounding it at the first `(` or space
            // cannot disagree about one.
            //
            // `crate::hash::Set` and not std’s, which kanso#1461’s comment
            // gives the reason for and `tests/the_compile_path_hashes_with_a
            // _fixed_seed.rs` pins: std seeds per process, and a randomly
            // seeded table makes this count differ between two runs of one
            // binary, which the compile rows read as a reproduction failure.
            let twin_calls = called_symbols(&call_twins);
            // A symbol DECLARES's own definitions call is kept as well, and
            // each line knows whether it is one: `DeclareLine::context`.
            let called = |sym: &str| body_calls.contains(sym) || twin_calls.contains(sym);
            declares_for_program(called, called, counters_wanted(), self.inline_helpers, true)
        };
        let mut out = String::new();
        out.push_str(&call_twins);
        for cell in &self.caf_cells {
            let _ = writeln!(out, "@{cell} = internal global %KValue zeroinitializer");
            let _ = writeln!(out, "@{cell}_ready = internal global i8 0");
        }
        for cell in &self.closure_cells {
            let _ = writeln!(out, "@{cell} = internal global %KValue zeroinitializer");
        }
        for (cell, w, arity) in
            self.closure_consts.iter().filter(|(cell, _, _)| body_lines.contains(cell.as_str()))
        {
            // K_INT 0 is the env a zero-capture closure never reads; K_CLOSURE
            // is tag 11. The KClosure layout is the runtime's:
            // { fn, env, ncaps, arity }.
            let _ = writeln!(out, "@{cell}_env = internal constant %KValue zeroinitializer");
            let _ = writeln!(
                out,
                "@{cell}_clo = internal constant {{ ptr, ptr, i64, i64 }}                  {{ ptr @{w}, ptr @{cell}_env, i64 0, i64 {arity} }}"
            );
            let _ = writeln!(
                out,
                "@{cell} = internal constant %KValue                  {{ i64 11, i64 ptrtoint (ptr @{cell}_clo to i64) }}"
            );
        }
        // A string is interned when a function asks for it, and the function
        // may since have been pruned: on the codegen corpus 198 of 270
        // strings and 264 of their literal cells were named by nothing, 36% of
        // the module's bytes, each parsed and laid out by clang all the same.
        // Only the body and the type tables name a string.
        let named = named_strings(&[&body, &self.globals], self.strings.len());
        for ((name, bytes), [string, lit]) in self.strings.iter().zip(named) {
            if string {
                let _ = writeln!(
                    out,
                    "@{name} = private unnamed_addr constant [{} x i8] c\"{}\"",
                    bytes.len(),
                    ir_bytes(bytes)
                );
            }
            if lit {
                let _ = writeln!(out, "@{name}_lit = internal global %KValue zeroinitializer");
            }
        }
        out.push_str(&self.globals);
        out.push('\n');
        out.push_str(&body);
        let narrowed = narrow_tailcc(out);
        let mut module = String::with_capacity(declares.len() + 1 + narrowed.len());
        module.push_str(&declares);
        module.push('\n');
        module.push_str(&narrowed);
        Ok(module)
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
        f.line(&format!(
            "{t} = call %KValue @k_str_lit_fast(ptr @{name}, i64 {len}, ptr @{name}_lit)"
        ));
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
        // One slot per id: 0 is the entry, a declared type is its position
        // plus one, and an alias's position falls back, since the alias
        // constructs and matches under its origin's id.
        let (fallback, _) = self.intern("record\0");
        let (entry_name, _) = self.intern("entry\0");
        let slots = self.program.types.len() + 1;
        let mut names = vec![fallback.clone(); slots];
        names[0] = entry_name;
        // The spelling a rendered record prints, beside the identity the
        // runtime matches on. RULED 2026-08-29, "records print qualified,
        // everywhere": the root's own types take the root's name.
        let mut shown = names.clone();
        let mut differs = false;
        let root = self.program.root.clone();
        for ty in &self.program.types {
            if ty.origin.is_some() {
                // an alias shares its origin's id; the origin owns the slot
                continue;
            }
            let id = self.type_ids[ty.name.as_str()] as usize;
            let (name, _len) = self.intern(&format!("{}\0", ty.name));
            shown[id] = match root.is_empty() || crate::ast::has_slash(&ty.name) {
                true => name.clone(),
                false => {
                    differs = true;
                    self.intern(&format!("{root}/{}\0", ty.name)).0
                }
            };
            names[id] = name;
        }
        type_table(&mut self.globals, "k_type_name", &names, &fallback);
        // A program whose root declares no bare type prints every record
        // under the identity's spelling, and the second table is an alias
        // rather than a copy of the first.
        match differs {
            true => type_table(&mut self.globals, "k_type_shown", &shown, &fallback),
            false => self.globals.push_str("@k_type_shown = alias ptr (i64), ptr @k_type_name\n"),
        }
    }

    /// Field metadata for keyed reads: name-indexed lookup reads these
    /// per-type tables at runtime.
    fn emit_type_fields(&mut self) {
        let slots = self.program.types.len() + 1;
        let mut tables: Vec<Vec<String>> = vec![Vec::new(); slots];
        tables[0] = vec!["key".into(), "value".into()];
        for ty in &self.program.types {
            if ty.origin.is_some() {
                // an alias shares its origin's id; the origin owns the slot
                continue;
            }
            let id = self.type_ids[ty.name.as_str()] as usize;
            tables[id] = ty.fields.iter().map(|(name, _, _)| name.clone()).collect();
        }
        let (empty, _) = self.intern("\0");
        let mut globals = String::new();
        let counts: Vec<String> = tables.iter().map(|f| f.len().to_string()).collect();
        let _ = writeln!(
            globals,
            "@k_type_field_counts = private unnamed_addr constant [{slots} x i64] [{}]",
            counts.iter().map(|c| format!("i64 {c}")).collect::<Vec<_>>().join(", ")
        );
        let mut rows = Vec::with_capacity(slots);
        for (id, fields) in tables.iter().enumerate() {
            if fields.is_empty() {
                rows.push("ptr null".to_string());
                continue;
            }
            let names: Vec<String> = fields
                .iter()
                .map(|field| format!("ptr @{}", self.intern(&format!("{field}\0")).0))
                .collect();
            let _ = writeln!(
                globals,
                "@k_type_fields_{id} = private unnamed_addr constant [{} x ptr] [{}]",
                names.len(),
                names.join(", ")
            );
            rows.push(format!("ptr @k_type_fields_{id}"));
        }
        let _ = writeln!(
            globals,
            "@k_type_fields = private unnamed_addr constant [{slots} x ptr] [{}]",
            rows.join(", ")
        );
        let _ = writeln!(
            globals,
            "define i64 @k_type_field_count(i64 %id) {{\nentry:\n  %in = icmp ult i64 %id, {slots}\n  br i1 %in, label %C, label %CD\nC:\n  %at = getelementptr [{slots} x i64], ptr @k_type_field_counts, i64 0, i64 %id\n  %n = load i64, ptr %at\n  ret i64 %n\nCD:\n  ret i64 0\n}}\n"
        );
        let _ = writeln!(
            globals,
            "define ptr @k_type_field_name(i64 %id, i64 %i) {{\nentry:\n  %n = call i64 @k_type_field_count(i64 %id)\n  %in = icmp ult i64 %i, %n\n  br i1 %in, label %T, label %TD\nT:\n  %at = getelementptr [{slots} x ptr], ptr @k_type_fields, i64 0, i64 %id\n  %row = load ptr, ptr %at\n  %f = getelementptr ptr, ptr %row, i64 %i\n  %name = load ptr, ptr %f\n  ret ptr %name\nTD:\n  ret ptr @{empty}\n}}\n"
        );
        self.globals.push_str(&globals);
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
        let mut nullary_arms = 0;
        for decl in decls {
            for (i, pattern) in decl.params.iter().enumerate() {
                match pattern {
                    Pattern::Var(..) | Pattern::Wildcard(..) => {}
                    Pattern::IntLit(..) | Pattern::Nullary(..) => {
                        if disc.is_some_and(|d| d != i) {
                            return None;
                        }
                        disc = Some(i);
                        match matches!(pattern, Pattern::IntLit(..)) {
                            true => int_arms += 1,
                            false => nullary_arms += 1,
                        }
                    }
                    _ => return None,
                }
            }
        }
        // Two int arms, or one against a generic tail and nothing else — the
        // shape every counted recursion is written in, `fn f acc 0` beside
        // `fn f acc n`.
        let switchable = int_arms >= 2 || (int_arms == 1 && nullary_arms == 0);
        match (disc, switchable) {
            (Some(d), true) => Some(d),
            _ => None,
        }
    }

    /// What an arm's discriminating pattern tests, when a switch on the tag can
    /// express it. `None` means the arm cannot be a case: it wants a guard no
    /// tag names, it admits everything, or this backend cannot say what it
    /// matches.
    fn arm_case(&self, p: &Pattern) -> Option<ArmCase> {
        match p {
            Pattern::Nullary(nm, _) => Some(ArmCase::Tags(vec![match nm.as_str() {
                "true" => K_TRUE,
                "false" => K_FALSE,
                "done" => K_DONE,
                _ => K_NONE,
            }])),
            // A marker's bare mention is its value, so it binds nothing and
            // the whole of it is the record check. A ctor with fields wants
            // bindings the arm bodies below do not make.
            Pattern::Ctor { ty, fields, whole } => match fields.is_empty() && whole.is_none() {
                true => Some(ArmCase::Rec(*self.type_ids.get(ty.as_str())?, 0)),
                false => None,
            },
            Pattern::Annotated { ty, .. } => {
                // These answer before the guard and typeset logic below
                // them in the cascade, so they answer before it here too.
                if crate::ast::is_effect_type(ty) {
                    return Some(ArmCase::Tags(vec![K_DESC]));
                }
                if ty.ends_with("[]") {
                    return Some(ArmCase::Tags(vec![9]));
                }
                if ty.contains('[') {
                    return Some(ArmCase::Tags(vec![10]));
                }
                // A typeset matches when any member does, which is not a
                // tag, so not a case.
                if self.typesets.contains_key(ty.as_str()) {
                    return None;
                }
                match ty.as_str() {
                    "int" => Some(ArmCase::Tags(vec![0])),
                    "float64" => Some(ArmCase::Tags(vec![1])),
                    "string" => Some(ArmCase::Tags(vec![6])),
                    "bool" => Some(ArmCase::Tags(vec![K_TRUE, K_FALSE])),
                    "none" => Some(ArmCase::Tags(vec![K_NONE])),
                    "done" => Some(ArmCase::Tags(vec![K_DONE])),
                    // `some` is every tag but none and err, which a default
                    // expresses and a case does not.
                    "some" => None,
                    other => Some(ArmCase::Rec(
                        *self.type_ids.get(other)?,
                        self.field_count(other).ok()?,
                    )),
                }
            }
            _ => None,
        }
    }

    /// The set a discriminator carries inside one arm of a switch dispatcher:
    /// the tags the arm's case names, as inference spells them. `None` for an
    /// arm the default reaches, whose value the switch proved nothing about.
    fn arm_tags_set(&self, p: &Pattern, by_tag: bool) -> Option<Set> {
        let tag_set = |t: i64| match t {
            0 => INT,
            1 => FLOAT,
            2 => infer::TRUE,
            3 => infer::FALSE,
            4 => NONE,
            6 => STR,
            16 => infer::DONE,
            9 => LIST,
            10 => MAP,
            _ => TOP,
        };
        match by_tag {
            true => match self.arm_case(p)? {
                ArmCase::Tags(tags) => Some(tags.into_iter().fold(0, |acc, t| acc | tag_set(t))),
                ArmCase::Rec(..) => Some(REC),
            },
            false => match p {
                Pattern::IntLit(..) => Some(INT),
                Pattern::Nullary(nm, _) => Some(match nm.as_str() {
                    "true" => infer::TRUE,
                    "false" => infer::FALSE,
                    "done" => infer::DONE,
                    _ => NONE,
                }),
                _ => None,
            },
        }
    }

    /// A group whose arms discriminate on one parameter by the kind of value it
    /// is compiles to a switch on the tag instead of a cascade of checks. The
    /// cascade tests arms in order, so the switch is only the same program when
    /// no two arms can match the same value: every arm names a set of tags,
    /// those sets are pairwise disjoint, and at most one arm — the last — is
    /// generic and stands as the default.
    ///
    /// Records are the exception that needs no disjointness. They all carry tag
    /// 7, so they chain inside that one case in source order, which is the
    /// order the cascade would have tried them in.
    fn tag_switch_shape(&self, decls: &[&FnDecl]) -> Option<usize> {
        // A subtype is a K_SUB wrapper whose own tag is 15 whatever it holds,
        // and the cascade's checks see through it. A switch on the tag would
        // send every subtype value to the default.
        if !self.sub_parents.is_empty() {
            return None;
        }
        let arity = decls[0].params.len();
        if arity == 0 {
            return None;
        }
        let mut disc: Option<usize> = None;
        for decl in decls {
            for (i, pattern) in decl.params.iter().enumerate() {
                match pattern {
                    Pattern::Var(..) | Pattern::Wildcard(..) => {}
                    _ => {
                        if disc.is_some_and(|d| d != i) {
                            return None;
                        }
                        disc = Some(i);
                    }
                }
            }
        }
        let disc = disc?;
        let mut claimed: Vec<i64> = Vec::new();
        let mut cases = 0;
        for (k, decl) in decls.iter().enumerate() {
            match &decl.params[disc] {
                Pattern::Var(..) | Pattern::Wildcard(..) => {
                    if k + 1 != decls.len() {
                        return None;
                    }
                }
                p => {
                    match self.arm_case(p)? {
                        ArmCase::Tags(tags) => {
                            for t in tags {
                                if t == 7 || claimed.contains(&t) {
                                    return None;
                                }
                                claimed.push(t);
                            }
                        }
                        ArmCase::Rec(..) => {
                            if !claimed.contains(&7) {
                                claimed.push(7);
                            }
                        }
                    }
                    cases += 1;
                }
            }
        }
        match cases >= 2 {
            true => Some(disc),
            false => None,
        }
    }

    fn emit_switch_dispatcher(
        &mut self,
        name: &str,
        arity: usize,
        decls: &[&FnDecl],
        disc: usize,
        by_tag: bool,
    ) -> Result<(), String> {
        let params = self.abi_params(name, arity);
        let ret = self.ret_ty(name, arity);
        let mut f = FnEmit::new(!self.inline_helpers);
        f.ret_ty = ret.to_string();
        f.group = name.to_string();
        f.arity = arity;
        let sym_hdr = dsym(name, arity);
        let apart = if self.kept_out.contains(name) { " noinline" } else { "" };
        let header = format!("define tailcc {ret} @{sym_hdr}({}){apart} {{", params.join(", "));
        let (hop_name, _) = self.intern(&format!("{name}\0"));
        f.start_block("entry");
        self.rebox_params(&mut f, name, arity);
        self.emit_reader_hole(&mut f, name, arity)?;
        self.record_param_sets(&mut f, name, arity);
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
                // NOT the folding twin. This block runs only when one of the
                // arguments IS a failure, which is exactly the case the
                // parameter sets say cannot happen -- they describe what the
                // arms accept, and a caller is free to hand over something
                // else. Asking here is the whole point of the block.
                let good = not_failure_test(&mut f, &format!("%x{i}"));
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
        let mut arm_labels = Vec::new();
        for k in 0..decls.len() {
            arm_labels.push(format!("arm{k}"));
        }
        if by_tag {
            // One case per tag the arms name, and the arms that name none —
            // the generic one, a value the group does not answer for, and a
            // failure — share the default. The record arms live inside case 7
            // together, in the order the cascade would have tried them.
            let mut tag_cases: Vec<(i64, String)> = Vec::new();
            let mut rec_arms: Vec<(i64, usize, String)> = Vec::new();
            let mut generic_arm: Option<usize> = None;
            for (k, decl) in decls.iter().enumerate() {
                let label = arm_labels[k].clone();
                match self.arm_case(&decl.params[disc]) {
                    Some(ArmCase::Tags(tags)) => {
                        for t in tags {
                            tag_cases.push((t, label.clone()));
                        }
                    }
                    Some(ArmCase::Rec(id, nfields)) => rec_arms.push((id, nfields, label)),
                    None => generic_arm = Some(k),
                }
            }
            let generic_label = match generic_arm {
                Some(k) => format!("arm{k}"),
                None => "nomatch".to_string(),
            };
            let dflt = f.label();
            let rec7 = f.label();
            let mut cases: Vec<String> =
                tag_cases.iter().map(|(t, l)| format!("    i64 {t}, label %{l}")).collect();
            if !rec_arms.is_empty() {
                cases.push(format!("    i64 7, label %{rec7}"));
            }
            f.switch_on(&tag, &dflt, &cases);
            if !rec_arms.is_empty() {
                f.start_block(&rec7);
                for (id, nfields, label) in &rec_arms {
                    let c = f.tmp();
                    f.predicate(
                        &c,
                        format!(
                            "call i64 @k_check_rec_fast(%KValue {dv}, i64 {id}, i64 {nfields})"
                        ),
                    );
                    let b = f.tmp();
                    f.line(&format!("{b} = icmp ne i64 {c}, 0"));
                    let next = f.label();
                    f.line(&format!("br i1 {b}, label %{label}, label %{next}"));
                    f.start_block(&next);
                }
                f.line(&format!("br label %{dflt}"));
            }
            f.start_block(&dflt);
            // With no generic arm both sides of the test are `nomatch`, so
            // there is nothing to ask.
            match generic_arm {
                None => f.line("br label %nomatch"),
                Some(_) => {
                    let disc_ok = inline_not_failure(&mut f, &dv);
                    f.line(&format!("br i1 {disc_ok}, label %{generic_label}, label %nomatch"));
                }
            }
        } else {
            // classify arms
            let mut int_cases: Vec<(String, String)> = Vec::new();
            let mut nullary_cases: Vec<(i64, String)> = Vec::new();
            let mut generic_arm: Option<usize> = None;
            for (k, decl) in decls.iter().enumerate() {
                let label = arm_labels[k].clone();
                match &decl.params[disc] {
                    Pattern::IntLit(n, _) => int_cases.push((n.to_string(), label)),
                    Pattern::Nullary(nm, _) => {
                        let t = match nm.as_str() {
                            "true" => K_TRUE,
                            "false" => K_FALSE,
                            "done" => K_DONE,
                            _ => K_NONE,
                        };
                        nullary_cases.push((t, label));
                    }
                    _ => generic_arm = Some(k),
                }
            }
            let generic_label = match generic_arm {
                Some(k) => format!("arm{k}"),
                None => "nomatch".to_string(),
            };
            // A byte discriminator crossed as a raw i64 — the byte, or 256 for
            // none — and `rebox_params` rebuilt a KValue from it for the tree below
            // to take apart again. The comment there says the round trip folds back
            // into a raw switch. It does not: `str_char`'s loop spends seven
            // instructions a byte on `cmp $0x100 / sete / cmove / shl` before it
            // reaches its first arm, and every byte-dispatching function in the
            // json decoder pays the same. Switch on the raw value instead, with 256
            // standing for the `none` arm. The rebox stays emitted for the arms
            // whose bodies read the byte as a value; where none do it is dead and
            // the optimiser drops it, and where some do it sinks into those arms.
            //
            // Only when every nullary arm is `none`: a byte is never `true` or
            // `false`, so a group naming one is not the shape this describes.
            //
            // And only when every int arm is a byte. 256 is the sentinel this
            // switch reads as `none`, so a group that writes `fn kind 256` would
            // send a read past the end of a byte string to that arm — an arm no
            // byte can ever reach. The boxed tree below is immune, because it
            // tests the tag before it looks at the payload. A literal outside
            // 0..255 keeps its group on the boxed path, where it stays dead the
            // way the oracle says it is.
            //
            // The divergence this was found by is no longer reachable: it needed
            // a group with a 256 arm and no `none` arm, and the exhaustiveness
            // check refuses that call now. What remains is the group carrying
            // both, where dropping this clause writes `i64 256` twice into one
            // switch and clang refuses the module. Either way the guard is what
            // keeps the two apart; tests/a_byte_arm_no_byte_can_reach.rs pins it.
            let raw_switchable = self.is_byte_disc(name, arity, disc)
                && nullary_cases.iter().all(|(t, _)| *t == K_NONE)
                && int_cases
                    .iter()
                    .all(|(n, _)| n.parse::<i128>().is_ok_and(|v| (0..=255).contains(&v)));
            if raw_switchable {
                let mut cases: Vec<String> =
                    int_cases.iter().map(|(n, l)| format!("    i64 {n}, label %{l}")).collect();
                for (_, l) in &nullary_cases {
                    cases.push(format!("    i64 256, label %{l}"));
                }
                f.switch_on(&format!("%x{disc}r"), &generic_label, &cases);
            } else {
                let is_int = f.tmp();
                f.line(&format!("{is_int} = icmp eq i64 {tag}, 0"));
                let int_block = f.label();
                let not_int = f.label();
                f.line(&format!("br i1 {is_int}, label %{int_block}, label %{not_int}"));
                f.start_block(&int_block);
                let payload = inline_payload(&mut f, &dv);
                let cases: Vec<String> =
                    int_cases.iter().map(|(n, l)| format!("    i64 {n}, label %{l}")).collect();
                f.switch_on(&payload, &generic_label, &cases);
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
            }
        }

        // A switch whose cases cover every value the discriminator can hold
        // never falls to `nomatch`, and then the failure path is blocks
        // nothing reaches, the way a dispatcher's `fail` is.
        if branches_to(&f.out, "nomatch") {
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
            let msg =
                format!("no overload of `{}` matches these arguments\0", crate::ast::spoken(name));
            let (m, _len) = self.intern(&msg);
            f.line(&format!("call void @k_die(ptr @{m})"));
            f.line("unreachable");
        }
        // arm bodies: patterns are known matched, only bind generics
        for (k, decl) in decls.iter().enumerate() {
            f.start_block(&arm_labels[k]);
            f.versions.clear();
            f.origin_prefix = format!("{} at {}", crate::ast::frame_name(&decl.name), decl.file);
            f.hako = crate::provenance::package_of(&decl.file).to_string();
            f.file = decl.file.clone();
            f.synthetic = decl.synthetic;
            for (i, pattern) in decl.params.iter().enumerate() {
                // The switch has already decided what the value is, so an
                // annotation here is only a name for it.
                match pattern {
                    Pattern::Var(pname, _) | Pattern::Annotated { name: pname, .. } => {
                        f.bind(pname, &format!("%x{i}"))
                    }
                    _ => {}
                }
            }
            // And the body is told what the switch decided. Until 2026-09-09
            // the discriminator kept the whole group's set inside every arm,
            // so `n:int`'s body forced `n` again, asked whether a user
            // to_string arm could claim it, and took the generic door on
            // every builtin it handed `n` to. The case that reached this arm
            // is the proof, so the set inside it is the case's tags.
            let narrowed = self.arm_tags_set(&decl.params[disc], by_tag);
            let whole = f.set_of(&format!("%x{disc}"));
            if let Some(set) = narrowed {
                f.record(&format!("%x{disc}"), set);
            }
            self.emit_fn_body(&mut f, &decl.body)?;
            f.record(&format!("%x{disc}"), whole);
        }
        let _ = writeln!(
            self.body,
            "{header}
{}}}
",
            f.body()
        );
        Ok(())
    }

    /// The check call accepting one named type (chain-aware when the
    /// program declares subtypes). Shared by concrete annotations and
    /// typeset members.
    fn type_check_call(&self, value: &str, ty: &str) -> Result<String, String> {
        let subs = !self.sub_parents.is_empty();
        Ok(match ty {
            // a box is a box whatever it yields, and never a subtype
            t if crate::ast::is_effect_type(t) => {
                format!("call i64 @k_check_tag(%KValue {value}, i64 {K_DESC})")
            }
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
            "done" => format!("call i64 @k_check_tag(%KValue {value}, i64 {K_DONE})"),
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

    /// A zero-argument definition is a constant, and a constant is built once:
    /// the body emits unchanged under a build symbol and the real symbol
    /// becomes a cache in front of it, filled on first demand. Until
    /// 2026-09-07 only a body that was a literal froze, and every other
    /// constant -- a table concatenated from literals, a map built by a call
    /// -- was recomputed at every mention: sha256's sixty-four round
    /// constants were concatenated from eleven literal lists 16,000 times a
    /// run on runbench. The interpreter computes every constant once behind a
    /// knot cell, so recomputing was a divergence from the oracle in cost,
    /// and for a body with an effect in count as well. The literal rule dated
    /// from when the cells were filled before main, where a body that could
    /// fail would have failed early; a freeze on first demand fails exactly
    /// where the interpreter does.
    /// A knotted constant is the case that cannot be left unfrozen whatever
    /// the rule: unfrozen, the real symbol recomputes its body, so a mention
    /// inside that body re-enters the builder and the recursion has no floor.
    fn is_constant_body(&self, _decl: &FnDecl) -> bool {
        true
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

    /// The second hole in an err's infectiousness: a reader's getter answers
    /// the piece at the dispatcher's entry, before any arm is tried, which is
    /// where the other two engines answer it too. A getter's parameter is
    /// boxed and its body is a bare binder, so the read needs no convention
    /// of its own — and a reader group that grew a `%parsed` return would be
    /// one this hole cannot serve, so that is refused rather than skipped.
    fn emit_reader_hole(&mut self, f: &mut FnEmit, name: &str, arity: usize) -> Result<(), String> {
        let Some(field) = crate::ast::err_reader(name) else {
            return Ok(());
        };
        if arity != 1 {
            return Ok(());
        }
        if f.ret_ty != "%KValue" {
            return Err(format!("the reader `{name}` returns a parsed record"));
        }
        let (lit, _) = self.intern(&format!("{field}\0"));
        let is = f.tmp();
        f.line(&format!("{is} = call i64 @k_is_err(%KValue %x0)"));
        let err = f.tmp();
        f.line(&format!("{err} = icmp ne i64 {is}, 0"));
        let read = f.label();
        let arms = f.label();
        f.line(&format!("br i1 {err}, label %{read}, label %{arms}"));
        f.start_block(&read);
        let got = f.tmp();
        f.line(&format!("{got} = call %KValue @k_err_read(%KValue %x0, ptr @{lit})"));
        f.line(&format!("ret %KValue {got}"));
        f.start_block(&arms);
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
            return self.emit_switch_dispatcher(name, arity, decls, disc, false);
        }
        if let Some(disc) = self.tag_switch_shape(decls) {
            return self.emit_switch_dispatcher(name, arity, decls, disc, true);
        }
        let params = self.abi_params(name, arity);
        let ret = self.ret_ty(name, arity);
        let mut f = FnEmit::new(!self.inline_helpers);
        f.ret_ty = ret.to_string();
        f.group = name.to_string();
        f.arity = arity;
        let apart = if self.kept_out.contains(name) { " noinline" } else { "" };
        let header = format!("define tailcc {ret} @{sym_hdr}({}){apart} {{", params.join(", "));
        let (hop_name, _) = self.intern(&format!("{name}\0"));
        f.start_block("entry");
        self.rebox_params(&mut f, name, arity);
        self.emit_reader_hole(&mut f, name, arity)?;
        self.record_param_sets(&mut f, name, arity);
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
        // Whether an arm left a way into the next one. An arm whose every
        // parameter check was proved away never branches to its `fail`, and
        // then the arms after it and the failure path below are blocks nothing
        // reaches: 13% of the decoder's emitted lines were such blocks, each
        // parsed by clang only to be deleted by its first pass.
        let mut open = true;
        for (k, decl) in decls.iter().enumerate() {
            let fail = format!("fail{k}");
            let arm_at = f.out.len();
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
            if !branches_to(&f.out[arm_at..], &fail) {
                open = false;
                break;
            }
            f.start_block(&fail);
        }
        f.lazy_cells.truncate(cells_before);
        if !open {
            let _ = writeln!(self.body, "{header}\n{}}}\n", f.body());
            return Ok(());
        }
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
                let msg = format!(
                    "no overload of `{}` matches these arguments",
                    crate::ast::spoken(name)
                );
                let (m, _len) = self.intern(&format!("{msg}\0"));
                f.line(&format!("call void @k_die(ptr @{m})"));
            }
        }
        f.line("unreachable");
        let _ = writeln!(self.body, "{header}\n{}}}\n", f.body());
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
                // A failure in the status word is the error's own tag with
                // nothing above it, so its low byte, which is the value
                // field's tag, reads as a failure too. When that field's
                // pattern already refuses a failure and sends it to the same
                // label, asking the whole word first is a second test of one
                // fact, made once for every record a parser returns.
                let value_refuses = matches!(
                    fields.get(1),
                    Some(
                        Pattern::Var(..)
                            | Pattern::Wildcard(_)
                            | Pattern::IntLit(..)
                            | Pattern::Nullary(..)
                    )
                );
                if !value_refuses {
                    let ok = inline_not_failure(f, status);
                    let cont = f.label();
                    f.line(&format!("br i1 {ok}, label %{cont}, label %{fail}"));
                    f.start_block(&cont);
                }
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
                // The position is an int whatever the word held, so its
                // pattern has no failure to refuse.
                self.emit_pattern_known(f, &poskv, &fields[0], fail, TOP & !FAIL)?;
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
            if matches!(head.as_ref(), Expr::Ident(n, _, _) if n == ty)
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
            f.predicate(&c, call);
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
                    "done" => K_DONE,
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
                    f.predicate(&c, call);
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
                Stmt::Bind {
                    pattern: Pattern::Var(name, _),
                    expr: Expr::App { args, span, .. },
                } if self.framed_views.contains(&(f.group.clone(), f.arity, *span)) => {
                    // Only a proven string: a header that may be the frame's
                    // or the arena's is a pointer LLVM cannot take apart,
                    // and the frame then saved 11,110,137 of the 28,013,580
                    // instructions it saves on runbench when it can. So
                    // `k_b_bytes_frame` has no slow arm. It allocates nothing,
                    // and the counting build takes it too: the header is not an
                    // arena byte, and k_stat_sh_bytes has nothing to count.
                    let v = self.emit_expr(f, &args[0])?;
                    let arg_sets = [f.set_of(&v)];
                    let t = f.tmp();
                    if arg_sets[0] == STR {
                        let slot = f.tmp();
                        f.line(&format!("{slot} = alloca [3 x i64], align 8"));
                        f.line(&format!(
                            "{t} = call %KValue @k_b_bytes_frame(%KValue {v}, ptr {slot})"
                        ));
                        f.frame_held = true;
                    } else {
                        f.line(&format!("{t} = call %KValue @k_b_bytes_fast(%KValue {v})"));
                    }
                    f.record(&t, infer::builtin_set("bytes", &arg_sets));
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
                            f.predicate(
                                &c,
                                format!(
                                    "call i64 @k_check_rec_fast(%KValue {value}, i64 {id}, i64 {})",
                                    fields.len()
                                ),
                            );
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
        // A name no declaration answers to is a VALUE here — a parameter
        // holding a function, a local, a builtin, a record's constructor —
        // and the callers route that to `emit_partial_value` before asking
        // for a lambda, so what reaches this point is a declared group.
        debug_assert!(self.program.fns.iter().any(|d| d.name == name));
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
        args.extend(params.iter().map(|(n, s)| {
            Expr::Ident(Name::new(&n.clone()), *s, crate::ast::Resolution::default())
        }));
        let head = Expr::Ident(Name::new(name), span, crate::ast::Resolution::default());
        let body = Expr::App { head: Box::new(head), args, piped: false, span };
        Ok(Expr::Lambda { params, body: Box::new(body), span })
    }

    /// Whether a name is a declared group, which is what decides how `&` over
    /// it lowers: a group's arities are known here, a value's are not.
    fn declared(&self, name: &str) -> bool {
        self.program.fns.iter().any(|d| d.name == name)
    }

    /// `&f 2` where `f` is a VALUE — a parameter, a local, a builtin handed
    /// out, a constructor. Its arity is settled when the arguments arrive,
    /// which is the interpreter's rule and one a lambda cannot follow, since a
    /// lambda fixes its count where it is written. The runtime holds the
    /// callee and the supplied arguments in a body-less closure and settles
    /// the count at each call; `k_partial{n}` builds one.
    fn emit_partial_value(
        &mut self,
        f: &mut FnEmit,
        name: &Name,
        supplied: &[Expr],
        span: Span,
    ) -> Result<String, String> {
        let callee =
            self.emit_expr(f, &Expr::Ident(name.clone(), span, crate::ast::Resolution::default()))?;
        let mut held: Vec<String> = Vec::new();
        for a in supplied {
            held.push(self.emit_expr(f, a)?);
        }
        let n = held.len();
        if n > 4 {
            return Err(format!(
                "native backend: a partial over a value holds at most 4 arguments, got {n}"
            ));
        }
        let arg_ir: String = held.iter().map(|v| format!(", %KValue {v}")).collect();
        let t = f.tmp();
        f.line(&format!("{t} = call %KValue @k_partial{n}(%KValue {callee}{arg_ir})"));
        f.record(&t, TOP);
        Ok(t)
    }

    fn emit_expr(&mut self, f: &mut FnEmit, expr: &Expr) -> Result<String, String> {
        match expr {
            // the interpreter is the oracle for `&`; the backends reject it out
            // loud rather than lowering something that would diverge
            Expr::Partial(name, span) => {
                if !self.declared(name) {
                    return self.emit_partial_value(f, name, &[], *span);
                }
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
                            let ident = Expr::Ident(
                                Name::new(&target.clone()),
                                *span,
                                crate::ast::Resolution::default(),
                            );
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
            Expr::Hole(_) => Ok("{ i64 4, i64 0 }".to_string()),
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
                            let (t, failed) = self.render_interp(f, &value);
                            fails |= failed;
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
            Expr::Ident(name, _, _) => {
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
                    "done" => Ok(format!("{{ i64 {K_DONE}, i64 0 }}")),
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
                // A partial over a VALUE is not flattened: its arity is
                // the callee's at run time, so the held arguments build the
                // runtime partial and the rest reach it through `k_call`,
                // which is where the count is settled.
                if let Expr::App { head: inner, args: held, .. } = head.as_ref() {
                    if let Expr::Partial(name, nspan) = inner.as_ref() {
                        if self.declared(name) {
                            let mut all = held.clone();
                            all.extend(args.iter().cloned());
                            let callee = Expr::Ident(
                                name.clone(),
                                *nspan,
                                crate::ast::Resolution::default(),
                            );
                            return self.emit_call_full(f, &Box::new(callee), &all, *piped, *span);
                        }
                    }
                }
                if let Expr::Partial(name, _) = head.as_ref() {
                    if !self.declared(name) {
                        return self.emit_partial_value(f, name, args, *span);
                    }
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
                let read = self.emit_at(f, &container, &key, *strict, *span);
                if !*strict {
                    return Ok(read);
                }
                // the sigil is the choice of channel (ruled 2026-09-16): the
                // element, or the missing-index err, settles into a box
                let boxed = f.tmp();
                f.line(&format!("{boxed} = call %KValue @k_b_effect(%KValue {read})"));
                f.record(&boxed, DESC);
                Ok(boxed)
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
                // A lambda that captures nothing is the same value every
                // evaluation, so it builds once into a permanent slot -- the
                // arrangement `str_const` already uses for a string literal.
                // The alloca, the two allocations and the memcpy go away and
                // the site is a load after the first visit.
                if captures.is_empty() {
                    let cell = format!("{lifted}_cell");
                    // the ccc wrapper, never the tailcc fn: C calls this pointer
                    self.closure_consts.push((cell.clone(), format!("w_{lifted}"), params.len()));
                    let t = f.tmp();
                    f.line(&format!("{t} = load %KValue, ptr @{cell}"));
                    return Ok(t);
                }
                let n = captures.len();
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
            Expr::List(items, _) if items.is_empty() => {
                let t = f.tmp();
                f.line(&format!("{t} = call %KValue @k_list_empty()"));
                f.record(&t, LIST);
                Ok(t)
            }
            Expr::MapLit(pairs, _) if pairs.is_empty() => {
                let t = f.tmp();
                f.line(&format!("{t} = call %KValue @k_map_empty()"));
                f.record(&t, MAP);
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
            // A guard is a tail `if` whose else is the rest of the body, so
            // its condition is asked as a question the same way: a
            // comparison branches on its own icmp and an `or` branches arm by
            // arm. Read for a value, `length xs == 0` in json's encode_list
            // became a tagged boolean that `k_truthy` was then asked about.
            // A failing condition still returns itself: with no merge label
            // `emit_cond` emits the return.
            let early_label = f.label();
            let rest_label = f.label();
            self.emit_cond(f, cond, &early_label, &rest_label, None)?;
            f.start_block(&early_label);
            self.emit_tail(f, early)?;
            f.start_block(&rest_label);
            self.emit_fn_body(f, rest)?;
            return Ok(());
        }
        if let Expr::App { head, args, piped: false, .. } = expr {
            if let Expr::Ident(name, _, _) = head.as_ref() {
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
            if let Expr::Ident(name, _, _) = &**head {
                if name == "if" && f.lookup(name).is_none() {
                    if let Some(t) = self.emit_byte_run(f, args)? {
                        self.emit_ret(f, &t);
                        return Ok(());
                    }
                    // In tail position a failing condition returns rather than
                    // joining a phi, which is the whole difference from the
                    // value form; `emit_cond` takes no merge label and emits
                    // the return itself.
                    let then_label = f.label();
                    let else_label = f.label();
                    self.emit_cond(f, &args[0], &then_label, &else_label, None)?;
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
                let region = matches!(expr, Expr::App { span, .. }
                    if self.beat.regions.contains(&(f.file.clone(), span.line as usize, span.col as usize)));
                if outside_cluster
                    || region
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
                                let edge = ((f.group.clone(), f.arity), (name.to_string(), n));
                                if self.beat.rewind.contains(&edge) {
                                    f.line("call void @k_beat_iter()");
                                }
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
                        // a header in this frame must outlive the callee's
                        // reads of it, so the call keeps the frame
                        let kind = match f.frame_held {
                            true => "call",
                            false => "musttail call",
                        };
                        f.line(&format!(
                            "{t} = {kind} tailcc {callee_ret} @{}({})",
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

    /// `x[p + k] == c and x[p + k'] == c' ...` over one bytes value and one
    /// int, read as one range test and plain byte compares.
    ///
    /// The json decoder matches `true`, `false` and `null` this way, and each
    /// read in the chain paid for itself: an overflow check on `p + k`, a test
    /// of each end of the range, a none-or-byte merge and the compare, about
    /// fourteen instructions a byte and 612,500 literals a run on the run
    /// program. Every read here sits between `p + kmin` and `p + kmax`, so
    /// when that window lies inside the bytes no read can miss and no sum can
    /// overflow, and the answer is the bytes compared. Outside the window at
    /// least one read is none, which compares false, but the general path is
    /// kept for it rather than written as `false`: it is also the path a sum
    /// that would overflow takes, and that one traps.
    fn emit_byte_run(&mut self, f: &mut FnEmit, args: &[Expr]) -> Result<Option<String>, String> {
        let Some((x, p, reads)) = byte_run(args) else { return Ok(None) };
        if f.lookup("false").is_some() || f.lookup("if").is_some() {
            return Ok(None);
        }
        let (Some(xv), Some(pv)) = (f.lookup(x), f.lookup(p)) else { return Ok(None) };
        if f.set_of(&xv) != BYTES || f.set_of(&pv) != INT {
            return Ok(None);
        }
        let kmin = reads.iter().map(|r| r.0).min().unwrap_or(0);
        let kmax = reads.iter().map(|r| r.0).max().unwrap_or(0);
        let bp = inline_payload(f, &xv);
        let bptr = f.tmp();
        f.line(&format!("{bptr} = inttoptr i64 {bp} to ptr"));
        let len_ptr = f.tmp();
        f.line(&format!("{len_ptr} = getelementptr %KBytes, ptr {bptr}, i64 0, i32 0"));
        let len = f.tmp();
        f.line(&format!("{len} = load i64, ptr {len_ptr}"));
        let idx = inline_payload(f, &pv);
        let lo = f.tmp();
        f.line(&format!("{lo} = icmp sge i64 {idx}, {}", 1 - kmin));
        let top = f.tmp();
        f.line(&format!("{top} = sub i64 {len}, {kmax}"));
        let hi = f.tmp();
        f.line(&format!("{hi} = icmp sle i64 {idx}, {top}"));
        let inside = f.tmp();
        f.line(&format!("{inside} = and i1 {lo}, {hi}"));
        let fast = f.label();
        let slow = f.label();
        let merge = f.label();
        f.line(&format!("br i1 {inside}, label %{fast}, label %{slow}"));
        f.start_block(&fast);
        let data_ptr = f.tmp();
        f.line(&format!("{data_ptr} = getelementptr %KBytes, ptr {bptr}, i64 0, i32 1"));
        let data = f.tmp();
        f.line(&format!("{data} = load ptr, ptr {data_ptr}"));
        let mut all = "true".to_string();
        for (k, c) in &reads {
            let off = f.tmp();
            f.line(&format!("{off} = add i64 {idx}, {}", k - 1));
            let at = f.tmp();
            f.line(&format!("{at} = getelementptr i8, ptr {data}, i64 {off}"));
            let byte = f.tmp();
            f.line(&format!("{byte} = load i8, ptr {at}"));
            let same = f.tmp();
            f.line(&format!("{same} = icmp eq i8 {byte}, {}", *c as u8 as i8));
            let both = f.tmp();
            f.line(&format!("{both} = and i1 {all}, {same}"));
            all = both;
        }
        let tag = f.tmp();
        f.line(&format!("{tag} = select i1 {all}, i64 2, i64 3"));
        let hit = f.tmp();
        f.line(&format!("{hit} = insertvalue %KValue {{ i64 undef, i64 0 }}, i64 {tag}, 0"));
        f.line(&format!("br label %{merge}"));
        f.start_block(&slow);
        let general = self.emit_if_value(f, args)?;
        let general = self.as_value(f, &general);
        let slow_from = f.cur_label.clone();
        f.line(&format!("br label %{merge}"));
        f.start_block(&merge);
        let t = f.tmp();
        f.line(&format!("{t} = phi %KValue [ {hit}, %{fast} ], [ {general}, %{slow_from} ]"));
        f.record(&t, infer::BOOL | f.set_of(&general));
        Ok(Some(t))
    }

    /// An `if` read for its value: the condition asked as a question, each arm
    /// emitted as a value, and a phi over the arms and any failure the
    /// condition carried out.
    fn emit_if_value(&mut self, f: &mut FnEmit, args: &[Expr]) -> Result<String, String> {
        let then_label = f.label();
        let else_label = f.label();
        let merge = f.label();
        let cond = self.emit_cond(f, &args[0], &then_label, &else_label, Some(&merge))?;
        f.start_block(&then_label);
        let then_value = self.emit_expr(f, &args[1])?;
        let then_from = f.cur_label.clone();
        f.line(&format!("br label %{merge}"));
        f.start_block(&else_label);
        let else_value = self.emit_expr(f, &args[2])?;
        let else_from = f.cur_label.clone();
        f.line(&format!("br label %{merge}"));
        f.start_block(&merge);
        let mut arms = vec![
            format!("[ {then_value}, %{then_from} ]"),
            format!("[ {else_value}, %{else_from} ]"),
        ];
        let mut fail_set = 0;
        for (v, from) in &cond.failed {
            arms.push(format!("[ {v}, %{from} ]"));
            fail_set |= f.set_of(v) & FAIL;
        }
        let t = f.tmp();
        f.line(&format!("{t} = phi %KValue {}", arms.join(", ")));
        f.record(&t, f.set_of(&then_value) | f.set_of(&else_value) | fail_set);
        Ok(t)
    }

    /// A condition is asked a question, not read for a value, and a
    /// comparison already knows the answer as an i1 before it builds anything.
    ///
    /// The value form costs eleven instructions where two would do. `b < 32`
    /// in the json escape path emits `icmp slt` and then `select` on it to
    /// build a KValue tagged 2 or 3, which a phi merges with `k_cmp`'s answer
    /// from the guarded slow arm, and the `if` then calls `k_not_failure` and
    /// `k_truthy` on the phi and branches on THAT. LLVM folds the pair away
    /// where the select reaches the branch directly; through the phi it
    /// cannot, because `k_cmp` may answer with a failure. So `setl` becomes a
    /// tag and two instructions later the tag is compared back. Measured on
    /// that one site: 108,174,000 instructions where 19,668,000 would do,
    /// 1.60% of encodebench.
    ///
    /// So the fast arm branches on its own i1 and never builds a value. The
    /// slow arm is unchanged — it calls the runtime, tests for a failure, asks
    /// whether the answer is true, and the failure it may carry is one more
    /// arm of the `if`'s phi. A condition that is not a comparison this can
    /// fuse is emitted as a value and tested the way it always was.
    fn emit_cond(
        &mut self,
        f: &mut FnEmit,
        cond: &Expr,
        then_label: &str,
        else_label: &str,
        merge: Option<&str>,
    ) -> Result<Cond, String> {
        // `a and b` parses as `if a b false`, `a or b` as `if a true b` and
        // `not a` as `if a false true`, so a condition is very often another
        // `if`. Asked as a question the inner one costs nothing: each arm is
        // asked the same question the outer `if` asked, and an arm that is the
        // literal the desugaring wrote is an unconditional branch. Read for a
        // value instead it builds a tagged boolean through a phi and the outer
        // `if` takes it apart again — the same family as the comparison above,
        // and the reason `scan_at`'s digit test measured a sixth of what the
        // arithmetic predicted: the `if`'s own test fused and the `and` under
        // it did not. Nothing is duplicated, because both arms branch to the
        // labels the outer `if` already made.
        if let Expr::App { head, args, .. } = cond {
            if args.len() == 3 {
                if let Expr::Ident(name, _, _) = &**head {
                    if name == "if" && f.lookup(name).is_none() {
                        let inner_then = f.label();
                        let inner_else = f.label();
                        let mut failed =
                            self.emit_cond(f, &args[0], &inner_then, &inner_else, merge)?.failed;
                        f.start_block(&inner_then);
                        let yes = self.emit_cond(f, &args[1], then_label, else_label, merge)?;
                        failed.extend(yes.failed);
                        f.start_block(&inner_else);
                        let no = self.emit_cond(f, &args[2], then_label, else_label, merge)?;
                        failed.extend(no.failed);
                        return Ok(Cond { failed });
                    }
                }
            }
        }
        // The arms that desugaring writes. A constant answers the question
        // rather than being built and asked.
        if let Expr::Ident(name, _, _) = cond {
            if f.lookup(name).is_none() {
                if name == "true" {
                    f.line(&format!("br label %{then_label}"));
                    return Ok(Cond { failed: Vec::new() });
                }
                if name == "false" {
                    f.line(&format!("br label %{else_label}"));
                    return Ok(Cond { failed: Vec::new() });
                }
            }
        }
        if let Expr::BinOp { op, lhs, rhs, span } = cond {
            if matches!(*op, "==" | "!=" | "<" | "<=" | ">" | ">=") {
                let a = self.emit_expr(f, lhs)?;
                let a = self.maybe_force(f, a);
                let b = self.emit_expr(f, rhs)?;
                let b = self.maybe_force(f, b);
                let a = self.as_value(f, &a);
                let b = self.as_value(f, &b);
                // A record on either side dispatches to the operator's user
                // arms, which answer with whatever the arm returns rather than
                // a boolean, so that condition is a value like any other.
                let armable = self.program.fns.iter().any(|d| d.name == *op && d.params.len() == 2);
                let routed = armable && (f.set_of(&a) | f.set_of(&b)) & REC != 0;
                if !routed {
                    return Ok(
                        self.emit_cmp_branch(f, op, &a, &b, *span, then_label, else_label, merge)
                    );
                }
                let v = self.emit_binop(f, op, &a, &b, *span)?;
                return Ok(self.test_cond_value(f, v, then_label, else_label, merge));
            }
        }
        // A condition the demand analysis deferred arrives as a thunk, and
        // asking a thunk whether it is true reads the thunk rather than the
        // answer. Force before testing: `maybe_force` emits nothing where the
        // set proves there is no thunk, so a strict condition is unchanged.
        let v = self.emit_expr(f, cond)?;
        let v = self.maybe_force(f, v);
        Ok(self.test_cond_value(f, v, then_label, else_label, merge))
    }

    /// The value form: is it a failure, is it true, branch. The failure leaves
    /// by the `if`'s merge carrying the condition itself, which is what the
    /// language says an `if` over a failure answers.
    fn test_cond_value(
        &mut self,
        f: &mut FnEmit,
        v: String,
        then_label: &str,
        else_label: &str,
        merge: Option<&str>,
    ) -> Cond {
        let ok = inline_not_failure(f, &v);
        let check = f.label();
        let failed = match merge {
            Some(merge) => {
                let fail_from = f.cur_label.clone();
                f.line(&format!("br i1 {ok}, label %{check}, label %{merge}"));
                vec![(v.clone(), fail_from)]
            }
            None => {
                let bail = f.label();
                f.line(&format!("br i1 {ok}, label %{check}, label %{bail}"));
                f.start_block(&bail);
                self.emit_ret(f, &v);
                Vec::new()
            }
        };
        f.start_block(&check);
        let tv = f.tmp();
        f.predicate(&tv, format!("call i64 @k_truthy(%KValue {v})"));
        let tb = f.tmp();
        f.line(&format!("{tb} = icmp ne i64 {tv}, 0"));
        f.line(&format!("br i1 {tb}, label %{then_label}, label %{else_label}"));
        Cond { failed }
    }

    /// The guarded comparison, branching instead of answering. Two ints take
    /// the icmp straight to the branch; anything else goes to the runtime and
    /// is tested as a value.
    #[allow(clippy::too_many_arguments)]
    fn emit_cmp_branch(
        &mut self,
        f: &mut FnEmit,
        op: &str,
        a: &str,
        b: &str,
        span: Span,
        then_label: &str,
        else_label: &str,
        merge: Option<&str>,
    ) -> Cond {
        let cmp = match op {
            "==" => "eq",
            "!=" => "ne",
            "<" => "slt",
            "<=" => "sle",
            ">" => "sgt",
            _ => "sge",
        };
        if f.set_of(a) == INT && f.set_of(b) == INT {
            let pa = inline_payload(f, a);
            let pb = inline_payload(f, b);
            let c = f.tmp();
            f.line(&format!("{c} = icmp {cmp} i64 {pa}, {pb}"));
            f.line(&format!("br i1 {c}, label %{then_label}, label %{else_label}"));
            return Cond { failed: Vec::new() };
        }
        let code = match op {
            "==" => 0,
            "!=" => 1,
            "<" => 2,
            "<=" => 3,
            ">" => 4,
            _ => 5,
        };
        let _ = span;
        let ta = inline_tag(f, a);
        let tb = inline_tag(f, b);
        let both = both_ints(f, &ta, &tb);
        let fast = f.label();
        let slow = f.label();
        f.line(&format!("br i1 {both}, label %{fast}, label %{slow}"));
        f.start_block(&fast);
        let pa = inline_payload(f, a);
        let pb = inline_payload(f, b);
        let c = f.tmp();
        f.line(&format!("{c} = icmp {cmp} i64 {pa}, {pb}"));
        f.line(&format!("br i1 {c}, label %{then_label}, label %{else_label}"));
        f.start_block(&slow);
        let sv = f.tmp();
        f.line(&format!("{sv} = call %KValue @k_cmp(%KValue {a}, %KValue {b}, i64 {code})"));
        f.record(&sv, infer::BOOL | FAIL);
        self.test_cond_value(f, sv, then_label, else_label, merge)
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
        let pure_int = f.set_of(a) == INT && f.set_of(b) == INT;
        if (op == "%" || op == "/") && pure_int {
            // Two integers the inference has proved: the remainder or the
            // quotient is one instruction, and the two divisors it cannot
            // take — zero, whose failure the runtime words, and minus one,
            // whose quotient overflows at INT64_MIN and traps on x86 — go to
            // the call as before. runbench asked k_mod 2,565,677 times, 43
            // million instructions inside the call, every one of them an
            // integer pair.
            let pa = inline_payload(f, a);
            let pb = inline_payload(f, b);
            let zero = f.tmp();
            f.line(&format!("{zero} = icmp eq i64 {pb}, 0"));
            let minus = f.tmp();
            f.line(&format!("{minus} = icmp eq i64 {pb}, -1"));
            let edge = f.tmp();
            f.line(&format!("{edge} = or i1 {zero}, {minus}"));
            let fast = f.label();
            let slow = f.label();
            let merge = f.label();
            f.line(&format!("br i1 {edge}, label %{slow}, label %{fast}"));
            f.start_block(&fast);
            let r = f.tmp();
            let insn = if op == "%" { "srem" } else { "sdiv" };
            f.line(&format!("{r} = {insn} i64 {pa}, {pb}"));
            let fv = f.tmp();
            f.line(&format!("{fv} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 {r}, 1"));
            f.line(&format!("br label %{merge}"));
            f.start_block(&slow);
            let sv = f.tmp();
            f.line(&format!("{sv} = {slow_call}"));
            f.line(&format!("br label %{merge}"));
            f.start_block(&merge);
            let t = f.tmp();
            f.line(&format!("{t} = phi %KValue [ {fv}, %{fast} ], [ {sv}, %{slow} ]"));
            f.record(&t, INT | ERR);
            return Ok(t);
        }
        if op == "/" || op == "%" {
            let t = f.tmp();
            f.line(&format!("{t} = {slow_call}"));
            f.record(&t, (f.set_of(a) & FAIL) | (f.set_of(b) & FAIL) | INT | FLOAT | ERR);
            return Ok(t);
        }
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
                    let (m, _) = self.intern(
                        "integer overflow (int64 native build; spec int is arbitrary precision)\0",
                    );
                    let trap = f.overflow_trap(m);
                    f.line(&format!("br i1 {overflow}, label %{trap}, label %{ok}"));
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
        let both = both_ints(f, &ta, &tb);
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
        // This phi carried no set until 2026-09-07, so every reader of it took
        // the default, which is TOP, which contains THUNK -- and an `if` over a
        // comparison then emitted `k_force_fast` on a value that is a boolean
        // by construction. Both arms are known: the fast one is the select or
        // the insertvalue written just above, and the slow one is `k_cmp`,
        // which answers `k_bool` or the failure it was handed, or `k_add` and
        // its two siblings, which answer an int, a float or that same failure.
        // None of the six can answer a thunk.
        f.record(
            &t,
            match op {
                "+" | "-" | "*" => (f.set_of(a) & FAIL) | (f.set_of(b) & FAIL) | INT | FLOAT,
                _ => (f.set_of(a) & FAIL) | (f.set_of(b) & FAIL) | infer::BOOL,
            },
        );
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
            let idx = inline_payload(f, key);
            let load = f.label();
            let miss = f.label();
            let merge = f.label();
            // Two branches, the way the room tests ask: joined with `and`, the
            // pair became flags and an `or` wherever nothing upstream settled it.
            index_in_range(f, &idx, &len_ptr, &load, &miss);
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
            if !strict {
                // The same merge as one i64: the byte, or 256 for none. A byte
                // discriminator wants exactly this and would otherwise rebuild
                // it with an extractvalue pair, an icmp and a select. Nothing
                // else reads it, so it costs nothing where it is unused.
                let raw = f.tmp();
                f.line(&format!("{raw} = phi i64 [ {wide}, %{load} ], [ 256, %{miss_from} ]"));
                f.raw_byte.insert(t.clone(), raw);
            }
            return t;
        }
        // A plain index of a container the sets prove is a list, read the way
        // the twin's list arm reads it, with no tag test in front. What comes
        // out is whatever the list holds, so the result carries no set. The
        // strict form answers a box around the element, which is the runtime's
        // to build, so it keeps the general path.
        if !strict && f.set_of(container) == LIST && f.set_of(key) == INT {
            let lp = inline_payload(f, container);
            let lptr = f.tmp();
            f.line(&format!("{lptr} = inttoptr i64 {lp} to ptr"));
            let idx = inline_payload(f, key);
            let load = f.label();
            let miss = f.label();
            let merge = f.label();
            // Two branches, the way the room tests ask: joined with `and`, the
            // pair became flags and an `or` wherever nothing upstream settled it.
            index_in_range(f, &idx, &lptr, &load, &miss);
            f.start_block(&load);
            let items_ptr = f.tmp();
            f.line(&format!("{items_ptr} = getelementptr i8, ptr {lptr}, i64 8"));
            let items = f.tmp();
            f.line(&format!("{items} = load ptr, ptr {items_ptr}"));
            let off = f.tmp();
            f.line(&format!("{off} = add i64 {idx}, -1"));
            let slot = f.tmp();
            f.line(&format!("{slot} = getelementptr %KValue, ptr {items}, i64 {off}"));
            let hit = f.tmp();
            f.line(&format!("{hit} = load %KValue, ptr {slot}"));
            f.line(&format!("br label %{merge}"));
            f.start_block(&miss);
            f.line(&format!("br label %{merge}"));
            f.start_block(&merge);
            let t = f.tmp();
            f.line(&format!(
                "{t} = phi %KValue [ {hit}, %{load} ], [ {{ i64 4, i64 0 }}, %{miss} ]"
            ));
            return t;
        }
        let ct = inline_tag(f, container);
        let is_bytes = f.tmp();
        f.line(&format!("{is_bytes} = icmp eq i64 {ct}, 13"));
        let kt = inline_tag(f, key);
        // a literal index's tag is known, and then the bytes test is the test
        let both = match kt.as_str() {
            "0" => is_bytes,
            _ => {
                let is_int = f.tmp();
                f.line(&format!("{is_int} = icmp eq i64 {kt}, 0"));
                let both = f.tmp();
                f.line(&format!("{both} = and i1 {is_bytes}, {is_int}"));
                both
            }
        };
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
        let idx = inline_payload(f, key);
        let load = f.label();
        // Two branches, the way the room tests ask: joined with `and`, the
        // pair became flags and an `or` wherever nothing upstream settled it.
        index_in_range(f, &idx, &len_ptr, &load, &slow);
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
        // The same i64 merge the proven path above builds, with the collapse
        // the crossing would otherwise write AFTER the merge sunk into this
        // arm instead. The fast arm's byte is already an i64 and pays
        // nothing; this arm is the one that calls the runtime, so four more
        // instructions here are four the hot path does not run. They are an
        // extract pair, an icmp and a select -- all pure, so where no byte
        // discriminator reads the result the whole thing is dead and goes.
        let slow_raw = match strict {
            true => String::new(),
            false => {
                let stag = f.tmp();
                f.line(&format!("{stag} = extractvalue %KValue {slow_value}, 0"));
                let spay = f.tmp();
                f.line(&format!("{spay} = extractvalue %KValue {slow_value}, 1"));
                let sisn = f.tmp();
                f.line(&format!("{sisn} = icmp eq i64 {stag}, 4"));
                let sraw = f.tmp();
                f.line(&format!("{sraw} = select i1 {sisn}, i64 256, i64 {spay}"));
                sraw
            }
        };
        let slow_from = f.cur_label.clone();
        f.line(&format!("br label %{merge}"));
        f.start_block(&merge);
        let t = f.tmp();
        f.line(&format!(
            "{t} = phi %KValue [ {fast_value}, %{load} ], [ {slow_value}, %{slow_from} ]"
        ));
        if !strict {
            let raw = f.tmp();
            f.line(&format!("{raw} = phi i64 [ {wide}, %{load} ], [ {slow_raw}, %{slow_from} ]"));
            f.raw_byte.insert(t.clone(), raw);
        }
        // This merge carried no set, so every reader took the default, which
        // is TOP, which contains THUNK -- and `maybe_force` then emitted a
        // `k_force_fast` on a value that reaches it through two arms neither
        // of which can answer a thunk. The proven path above has always
        // recorded `INT | NONE`; this one is the same index with the tag test
        // still to run, and it was missed. It is the shape the comment at the
        // arithmetic phi records, found in a second place.
        //
        // The bound is the LIST bit and nothing weaker. The fast arm is the
        // `insertvalue` written just above, an int. The slow arm is `k_b_at`
        // or the shim in front of it, and their cases answer a byte, a
        // one-character string, `none`, or a failure they were handed --
        // except the LIST case, which answers `l->items[i - 1]`, whatever the
        // list holds, thunks included. So the set is safe to narrow exactly
        // when the container cannot be a list.
        if f.set_of(container) & LIST == 0 {
            f.record(&t, TOP & !infer::THUNK);
        }
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
            // A `.>` whose subject is a strict index is a bind over a box the
            // read settles on the spot (ruled 2026-09-16), so nothing about it
            // is deferred: the element goes straight to the callback, and a
            // miss answers the missing-index err boxed. The chain's answer is
            // a box either way -- what the callback answered when that is one,
            // else its answer settled -- which is what the oracle's bind node
            // yields when the executor reaches it. The bind node, its closure
            // and the box under it are never built.
            if let Expr::Index { base, index, strict: true, span: at } = &args[0] {
                let container = self.emit_expr(f, base)?;
                let container = self.maybe_force(f, container);
                let key = self.emit_expr(f, index)?;
                let key = self.maybe_force(f, key);
                let read = self.emit_at(f, &container, &key, true, *at);
                let ok = inline_not_failure(f, &read);
                let docall = f.label();
                let missed = f.label();
                let merge = f.label();
                f.line(&format!("br i1 {ok}, label %{docall}, label %{missed}"));
                f.start_block(&docall);
                let called = self.emit_call_rest(f, head, args, Some(read.clone()), span)?;
                let ctag = inline_tag(f, &called);
                let is_box = f.tmp();
                f.line(&format!("{is_box} = icmp eq i64 {ctag}, 8"));
                let wrap = f.label();
                let answered = f.label();
                let called_from = f.cur_label.clone();
                f.line(&format!("br i1 {is_box}, label %{answered}, label %{wrap}"));
                f.start_block(&wrap);
                let settled = f.tmp();
                f.line(&format!("{settled} = call %KValue @k_b_effect(%KValue {called})"));
                f.line(&format!("br label %{answered}"));
                f.start_block(&answered);
                let boxed = f.tmp();
                f.line(&format!(
                    "{boxed} = phi %KValue [ {called}, %{called_from} ], [ {settled}, %{wrap} ]"
                ));
                let boxed_from = f.cur_label.clone();
                f.line(&format!("br label %{merge}"));
                f.start_block(&missed);
                let failed = f.tmp();
                f.line(&format!("{failed} = call %KValue @k_b_effect(%KValue {read})"));
                f.line(&format!("br label %{merge}"));
                f.start_block(&merge);
                let t = f.tmp();
                f.line(&format!(
                    "{t} = phi %KValue [ {boxed}, %{boxed_from} ], [ {failed}, %{missed} ]"
                ));
                f.record(&t, DESC);
                return Ok(t);
            }
            let piped_value = self.emit_expr(f, &args[0])?;
            if f.set_of(&piped_value) & DESC != 0 {
                let mut body_args: Vec<Expr> = vec![Expr::Ident(
                    Name::new("__piped"),
                    span,
                    crate::ast::Resolution::default(),
                )];
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
            Expr::Ident(name, _, _) => {
                matches!(name.as_str(), "true" | "false" | "none" | "done")
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
        let Expr::Ident(name, _, _) = head else {
            unreachable!("non-ident heads take the computed path");
        };
        if name == "if" {
            if let Some(t) = self.emit_byte_run(f, args)? {
                return Ok(t);
            }
            return self.emit_if_value(f, args);
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
                if let Expr::Ident(inner, _, _) = &**inner_head {
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
                        // three KValues and the origin pointer want seven of
                        // the six integer registers the abi has, so the last
                        // spills and the callee reloads it; the twin tests the
                        // three tags here and hands the raw door five scalars.
                        f.line(&format!(
                            "{t} = call %KValue @k_b_utf8_slice_fast(%KValue {}, %KValue {}, %KValue {}, {origin})",
                            parts[0], parts[1], parts[2]
                        ));
                        f.record(&t, infer::builtin_set("utf8", &[sliced]));
                        return Ok(t);
                    }
                }
            }
        }
        // `append acc (slice cs a b)` builds a view header for the only
        // purpose of copying its bytes out and dropping it. The slice's whole
        // content is a pointer and a length the accumulator's copy needs
        // anyway, so the fused door reads the range out of `cs` and never
        // boxes it. Same wrapper-spelling rule as the pair above: only the
        // builtin spelling is fused, and `append` never gives birth to an err,
        // so no origin has to move with it.
        if first.is_none() && args.len() == 2 && self.builtin_named(name, 2) == "append" {
            if let Expr::App { head: inner_head, args: inner_args, piped: false, .. } = &args[1] {
                if let Expr::Ident(inner, _, _) = &**inner_head {
                    if self.builtin_named(inner, inner_args.len()) == "slice"
                        && inner_args.len() == 3
                    {
                        let acc = self.emit_expr(f, &args[0])?;
                        let acc = self.maybe_force(f, acc);
                        let mut parts = Vec::new();
                        for a in inner_args {
                            let v = self.emit_expr(f, a)?;
                            parts.push(self.maybe_force(f, v));
                        }
                        let sets: Vec<Set> = parts.iter().map(|e| f.set_of(e)).collect();
                        let sliced = infer::builtin_set("slice", &sets);
                        // the same uniqueness the unfused site would have got
                        let mutate = self.in_place_pushes.contains(&(
                            f.file.clone(),
                            span.line as usize,
                            span.col as usize,
                        ));
                        let t = f.tmp();
                        f.line(&format!(
                            "{t} = call %KValue @k_b_append_slice_fast(%KValue {acc}, %KValue {}, %KValue {}, %KValue {}, i64 {})",
                            parts[0], parts[1], parts[2], i64::from(mutate)
                        ));
                        f.record(&t, infer::builtin_set("append", &[f.set_of(&acc), sliced]));
                        return Ok(t);
                    }
                }
            }
        }
        // `to_int (slice cs a b)` and `to_float (slice cs a b)` build a view
        // for the one purpose of reading a number out of it, which is every
        // number the JSON decoder meets. The fused doors read the range out
        // of `cs` in place. Only the builtin spelling is fused, as above, and
        // the origin moves with the call because both conversions can give
        // birth to an err.
        if first.is_none() && args.len() == 1 {
            let door = match self.builtin_named(name, 1).as_str() {
                "to_int" => Some("to_int"),
                "to_float" => Some("to_float"),
                _ => None,
            };
            if let (
                Some(door),
                Expr::App { head: inner_head, args: inner_args, piped: false, .. },
            ) = (door, &args[0])
            {
                if let Expr::Ident(inner, _, _) = &**inner_head {
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
                        let origin = self.origin_arg(f, span);
                        let t = f.tmp();
                        f.line(&format!(
                            "{t} = call %KValue @k_b_{door}_slice(%KValue {}, %KValue {}, %KValue {}, {origin})",
                            parts[0], parts[1], parts[2]
                        ));
                        f.record(&t, infer::builtin_set(door, &[sliced]));
                        return Ok(t);
                    }
                }
            }
        }
        // `append acc "{x}"` where `x` is a number: the template rendered
        // `x` into a string for the one purpose of copying its bytes into
        // `acc` and dropping it, which is every scalar the JSON encoder
        // writes. The fused door renders the digits into a stack buffer and
        // copies them from there, so no string is built. A value the ambient
        // to_string group could claim keeps the dispatch the template would
        // have made, so a user arm is never skipped; every other tag the door
        // hands to k_render itself, the same call the template makes, so the
        // bytes cannot differ. Same wrapper-spelling rule as the pair above.
        // `bytes ""` starts a builder, and the next thing that happens to it
        // is an append. As a view of the empty string it owned no storage, so
        // that append grew it from nothing: 175,527 grows a run on runbench,
        // one for each escaped string the decoder unescapes, at about 88
        // instructions each. The literal takes a builder with 64 bytes of room
        // instead, which is the capacity that first grow chose, so every later
        // grow is the one it was. Only the literal: a view of any other string
        // is what `bytes` has always built, and testing for an empty string at
        // run time cost encodebench two instructions on every string it
        // escapes.
        if first.is_none() && args.len() == 1 && self.builtin_named(name, 1) == "bytes" {
            if let Expr::Str(parts, _) = &args[0] {
                if parts.iter().all(|p| matches!(p, TemplatePart::Lit(s) if s.is_empty())) {
                    let t = f.tmp();
                    f.line(&format!("{t} = call %KValue @k_b_bytes_seed()"));
                    f.record(&t, infer::builtin_set("bytes", &[STR]));
                    return Ok(t);
                }
            }
        }
        // `append acc "true"`: a literal of one to eight bytes into a builder
        // the linearity analysis proved this site owns. `k_b_append_mut_word`
        // says why the literal travels as a word.
        if first.is_none() && args.len() == 2 && self.builtin_named(name, 2) == "append" {
            if let Expr::Str(parts, _) = &args[1] {
                let text: Option<String> = parts
                    .iter()
                    .map(|p| match p {
                        TemplatePart::Lit(s) => Some(s.as_str()),
                        _ => None,
                    })
                    .collect();
                let mutate = self.in_place_pushes.contains(&(
                    f.file.clone(),
                    span.line as usize,
                    span.col as usize,
                ));
                if let Some(text) = text.filter(|t| mutate && (1..=8).contains(&t.len())) {
                    let acc = self.emit_expr(f, &args[0])?;
                    let acc = self.maybe_force(f, acc);
                    let mut bytes = [0u8; 8];
                    bytes[..text.len()].copy_from_slice(text.as_bytes());
                    let word = i64::from_le_bytes(bytes);
                    let (lit, n) = self.intern(&text);
                    let t = f.tmp();
                    f.line(&format!(
                        "{t} = call %KValue @k_b_append_mut_word(%KValue {acc}, i64 {word}, i64 {n}, ptr @{lit}, ptr @{lit}_lit)"
                    ));
                    f.record(&t, infer::builtin_set("append", &[f.set_of(&acc), STR]));
                    return Ok(t);
                }
            }
        }
        if first.is_none() && args.len() == 2 && self.builtin_named(name, 2) == "append" {
            if let Expr::Str(parts, _) = &args[1] {
                if let [TemplatePart::Interp(inner)] = parts.as_slice() {
                    let acc = self.emit_expr(f, &args[0])?;
                    let acc = self.maybe_force(f, acc);
                    let value = self.emit_expr(f, inner)?;
                    let value = self.maybe_force(f, value);
                    let mutate = self.in_place_pushes.contains(&(
                        f.file.clone(),
                        span.line as usize,
                        span.col as usize,
                    ));
                    let fusable = !self.render_dispatchable(f, &value);
                    let fuse_render = f.set_of(&value) & (INT | FLOAT) != 0 && fusable;
                    let t = f.tmp();
                    let rendered_set = match fuse_render {
                        true => {
                            let fails = f.set_of(&value) & ERR;
                            let value = self.as_value(f, &value);
                            f.line(&format!(
                                "{t} = call %KValue @k_b_append_rendered(%KValue {acc}, %KValue {value}, i64 {})",
                                i64::from(mutate)
                            ));
                            STR | fails
                        }
                        false => {
                            // both operands are already emitted, so the call
                            // the generic path would have made is made here:
                            // the byte twin, whose string arm copies inline.
                            // The first cut called k_b_append_mut instead and
                            // paid 22,789,710 in k_b_append_wide on runbench.
                            let (rendered, fails) = self.render_interp(f, &value);
                            let sym = match mutate {
                                true => "k_b_append_mut_byte",
                                false => "k_b_append_byte",
                            };
                            f.line(&format!(
                                "{t} = call %KValue @{sym}(%KValue {acc}, %KValue {rendered})"
                            ));
                            STR | fails
                        }
                    };
                    f.record(&t, infer::builtin_set("append", &[f.set_of(&acc), rendered_set]));
                    return Ok(t);
                }
            }
        }
        // a region: the mark goes down before the arguments, so what they
        // allocate is inside it (see beat::region_sites)
        let region = first.is_none()
            && self.beat.regions.contains(&(f.file.clone(), span.line as usize, span.col as usize))
            && !name.starts_with("builtin_")
            && !self.type_ids.contains_key(name.as_str())
            && self.program.fns.iter().any(|d| d.name == *name && d.params.len() == args.len());
        if region {
            f.line("call void @k_beat_push()");
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

        // A builtin sees a subtype's value as its parent's, which is what the
        // oracle's call_builtin does before anything else. Only a program that
        // declares a subtype can hand one over, so only that program pays.
        // A type's constructor comes through here too and must see the value
        // it wraps, so only a real builtin's arguments are unwrapped.
        if !shadows && !self.sub_parents.is_empty() && crate::check::builtin_arity(name).is_some() {
            for e in emitted.iter_mut() {
                let t = f.tmp();
                f.line(&format!("{t} = call %KValue @k_unsub(%KValue {e})"));
                *e = t;
            }
        }
        if name == "err" {
            let origin = self.origin_arg(f, span);
            let t = f.tmp();
            f.line(&format!("{t} = call %KValue @k_err(%KValue {}, {origin})", emitted[0]));
            f.record(&t, ERR);
            return Ok(t);
        }
        // `rescue` and `annotate` are handed the site they were written at,
        // like `err` and `wrap_err`: `annotate` because it raises an err of
        // its own, `rescue` because its licence is foreign-only and the
        // site's package is what the failure's raiser is compared against.
        // The runtime wraps the callback in a closure holding both and hands
        // the result to the one worded node.
        if name == "rescue" || name == "annotate" {
            let origin = self.origin_arg(f, span);
            let t = f.tmp();
            f.line(&format!(
                "{t} = call %KValue @k_b_{name}(%KValue {}, %KValue {}, {origin})",
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
            let crosses_down = self
                .group_by_name
                .get(name)
                .and_then(|at| at.first())
                .is_some_and(|&i| *self.program.fns[i].file != *f.file);
            let cohort_entry = !beat_entry
                && !region
                && !register_returned
                && crosses_down
                && !self.cycle_reached.contains(f.group.as_str())
                && !f.synthetic
                && !caller_loops
                && emitted.iter().all(|e| f.set_of(e) & arg_heapish == 0);
            if !region && (beat_entry || cohort_entry) {
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
            let result = if region {
                let p = f.tmp();
                f.line(&format!("{p} = call %KValue @k_region_pop(%KValue {t})"));
                p
            } else if beat_entry {
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
            let returned = self.group_return_set(name, n) | fails;
            f.record(&result, returned);
            if callee_ret == "%KValue" && self.sub_parents.is_empty() {
                self.assume_tag(f, &result, returned);
            }
            return Ok(result);
        }
        // Declared, but at no arity this call can reach. The interpreter
        // reports it when the call runs, so native reports the same words at
        // the same moment rather than refusing to build a program the oracle
        // executes.
        if !was_builtin && self.program.fns.iter().any(|d| d.name == *name) {
            let msg =
                format!("no overload of `{}` matches these arguments", crate::ast::spoken(name));
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
            // `length` of a value the sets already prove is bytes is the
            // header's first field. The twin tests the tag and keeps a call
            // arm for everything else, and that arm is a block merge: a
            // later read of the same field cannot be forwarded across it, so
            // a loop that asks `length coll < i` and then indexes `coll[i]`
            // loads the length twice and compares it twice.
            let held = f.set_of(&emitted[0]);
            if name == "length" && held != 0 && held & !(BYTES | LIST) == 0 {
                let bp = inline_payload(f, &emitted[0]);
                let bptr = f.tmp();
                f.line(&format!("{bptr} = inttoptr i64 {bp} to ptr"));
                let len_ptr = f.tmp();
                f.line(&format!("{len_ptr} = getelementptr %KBytes, ptr {bptr}, i64 0, i32 0"));
                let len = f.tmp();
                f.line(&format!("{len} = load i64, ptr {len_ptr}"));
                let t = f.tmp();
                f.line(&format!("{t} = insertvalue %KValue {{ i64 0, i64 undef }}, i64 {len}, 1"));
                f.record(&t, infer::builtin_set("length", &[held]));
                return Ok(t);
            }
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
                // fit falls through to the C path inside the twin. Where the
                // sets already prove both tags -- a bytes accumulator and an
                // int -- the twin's two opening tests are asked twice, so
                // there is a second door that starts after them.
                match f.set_of(&emitted[0]) == BYTES && f.set_of(&emitted[1]) == INT {
                    true => "append_mut_int",
                    false => "append_mut_byte",
                }
            } else if BIT_TWINS.contains(&name) {
                // one machine op each where the operand tags say int and a
                // shift is in range. `&` `|` `^` reach the twin through the
                // operator route; these are the same work written as a name,
                // which is how lib/bits spells what no operator says.
                bit_twin(name)
            } else if name == "slice" {
                // the bytes arm is a header read and four compares, and the
                // three failure guards plus two tag tests in front of it cost
                // more than the view it builds. The twin tests the tags and
                // hands the arithmetic a pointer, a length and two integers.
                "slice_fast"
            } else if name == "find2" {
                // four KValues do not fit the six registers the ABI has, so
                // the fourth arrives on the stack and the callee unpacks it
                // before it can splat the byte — fourteen instructions of the
                // fifty-four this cost, where the scan itself was ten. The
                // twin tests the tags, hands the scan five scalars, and folds
                // to nothing at a call site whose needles are literals.
                "find2_fast"
            } else if name == "find2_below" {
                // the same door with one more bound, and the largest spill of
                // the five the emitter declares over six registers: FIVE
                // KValues are ten register-sized arguments, so four arrive on
                // the caller's stack. The twin tests the five tags and hands
                // the scan a pointer, a length and four integers — six, and
                // nothing spills.
                "find2_below_fast"
            } else if name == "bytes" {
                // two field reads and a three-field header into the arena, and
                // the arena bump is already inline in the append twin. The
                // call and its tag ladder were most of the thirty this cost.
                "bytes_fast"
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
        let mut f = FnEmit::new(!self.inline_helpers);
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
            let slot = f.tmp();
            f.line(&format!("{slot} = getelementptr %KValue, ptr %env, i64 {i}"));
            let t = f.tmp();
            f.line(&format!("{t} = load %KValue, ptr {slot}"));
            f.bind(cap, &t);
        }
        for (i, p) in params.iter().enumerate() {
            f.bind(p, &format!("%a{i}"));
        }
        self.emit_tail(&mut f, body)?;
        let sig: String = (0..params.len()).map(|i| format!(", %KValue %a{i}")).collect();
        let _ = writeln!(
            self.body,
            "define tailcc %KValue @{lifted}(ptr %env{sig}) {{\n{}}}\n",
            f.body()
        );
        // The convention rides on the wrapper AND on every arm that calls
        // through a closure pointer. Split them and the arguments arrive in
        // the wrong registers: leaving the runtime's `k_call{n}` on the C
        // convention while this carries preserve_none made encodebench print
        // `error[runtime]: bytes takes a string` where its answer is
        // `done: 74072800`.
        let cc = self.convention.keyword();
        let _ = writeln!(
            self.body,
            "define {cc}%KValue @w_{lifted}(ptr %env{sig}) {{\nentry:\n  %r = call \
             tailcc %KValue @{lifted}(ptr %env{sig})\n  ret %KValue %r\n}}\n"
        );
        Ok(())
    }
}

fn collect_idents(expr: &Expr, out: &mut Vec<String>) {
    match expr {
        Expr::Int(..) | Expr::Float(..) | Expr::Partial(..) | Expr::Hole(..) => {}
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
        Expr::Ident(name, _, _) => out.push(name.to_string()),
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
///
/// Only code is read. A line opening with `@` defines a global, a string
/// constant among them, and a constant's bytes are the program's: a program
/// printing `call tailcc` once lost the words from its constant, kept the
/// declared length, and clang refused the module.
/// Whether `line` holds `needle`, found from the byte `at` places into it.
/// `str::contains` builds a substring searcher for every call, about a
/// hundred instructions before it reads a byte, and `narrow_tailcc` asks two
/// or three of these of every line it walks: 178 calls on a one-line
/// program's start-up. The anchor byte is found by memchr, and the needle is
/// compared where one lands. Pick `at` so the anchor is rare in the emitter's
/// lines.
fn holds(line: &str, needle: &str, at: usize) -> bool {
    let anchor = needle.as_bytes()[at] as char;
    line.match_indices(anchor)
        .any(|(i, _)| i >= at && line.as_bytes()[i - at..].starts_with(needle.as_bytes()))
}

fn narrow_tailcc(ir: String) -> String {
    let code = |line: &str| !line.starts_with('@');
    // Split once and walk the lines three times. Splitting is a search for
    // every newline, and three splits of a one-line program's body were
    // 75,721 of the start-up row's instructions.
    let lines: Vec<&str> = ir.lines().collect();
    let mut keep: crate::hash::Set<String> = crate::hash::Set::default();
    let mut current: Option<String> = None;
    for &line in lines.iter().filter(|line| code(line)) {
        if let Some(rest) = line.strip_prefix("define ") {
            current = symbol_of(rest);
        }
        if holds(line, "musttail call", 0) {
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
    let mut trampolines: Vec<(String, String)> = Vec::new();
    let mut spilling: crate::hash::Set<String> = crate::hash::Set::default();
    for &line in &lines {
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
        trampolines.push((
            name.clone(),
            format!(
                "define {ret} @{}({}) {{\nentry:\n  %r = call tailcc {ret} @{}({})\n  ret {ret} %r\n}}\n",
                trampoline_name(&name),
                taken.join(", "),
                quoted(&name),
                handed.join(", ")
            ),
        ));
        spilling.insert(name);
    }

    let mut rerouted: crate::hash::Set<String> = crate::hash::Set::default();
    let mut out = String::with_capacity(ir.len());
    for &line in &lines {
        // Every rewrite below needs the word, so a line without it is copied
        // as it stands and its callee is never looked up.
        if !code(line) || !holds(line, "tailcc ", 4) {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        let named = symbol_of(line);
        let reroute = !holds(line, "musttail call", 0)
            && line.contains("call tailcc ")
            && named.as_ref().is_some_and(|n| spilling.contains(n));
        if reroute {
            let name = named.expect("checked above");
            let call = format!("@{}(", quoted(&name));
            let through = format!("@{}(", trampoline_name(&name));
            out.push_str(&line.replace("call tailcc ", "call ").replace(&call, &through));
            rerouted.insert(name);
        } else if !named.as_ref().is_some_and(|n| keep.contains(n)) {
            out.push_str(&line.replace("tailcc ", ""));
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    // A trampoline no call was rerouted through is a function nothing names,
    // which clang compiles at `-O0` all the same.
    for (name, t) in trampolines {
        if rerouted.contains(&name) {
            out.push_str(&t);
        }
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

#[cfg(test)]
mod the_emitter_group_index_answers_what_the_scan_answered {
    use super::*;

    /// What `Backend::group_indices` did before the index existed, kept as the
    /// oracle.
    fn scanned(program: &Program, name: &str, arity: usize) -> Vec<usize> {
        program
            .fns
            .iter()
            .enumerate()
            .filter(|(_, d)| d.name == name && d.params.len() == arity)
            .map(|(i, _)| i)
            .collect()
    }

    /// Every question the emitter can ask, asked both ways. `group_param_set`
    /// and `group_return_set` FOLD over the answer, so a missing or extra
    /// index is a wrong type set and a silently wrong ABI -- and the fold is
    /// order-independent, which is why order is checked here anyway rather
    /// than left to a future caller to discover.
    fn agrees_over(program: &Program) {
        let by_name = group_indices_by_name(program);
        let mut arities: Vec<usize> = program.fns.iter().map(|d| d.params.len()).collect();
        arities.sort_unstable();
        arities.dedup();
        let mut names: Vec<&str> = program.fns.iter().map(|d| d.name.as_str()).collect();
        names.push("a name nothing declares");
        for name in names {
            for &arity in arities.iter().chain(std::iter::once(&99)) {
                let want = scanned(program, name, arity);
                let got: Vec<usize> = group_indices_in(&by_name, program, name, arity).collect();
                assert_eq!(
                    want, got,
                    "`{name}` at arity {arity}: the scan found {want:?} and the index found {got:?}"
                );
            }
        }
    }

    #[test]
    fn over_the_library_the_compiler_carries() {
        let program =
            crate::compile_module(std::path::Path::new("lib/json"), false).expect("lib/json");
        agrees_over(&program);
    }

    #[test]
    fn over_a_name_declared_at_two_arities() {
        // One name at two arities, and a second sharing neither: the case a
        // name-keyed index has to get right.
        let src = concat!(
            "fn f x\n  x\n\n",
            "fn f x y\n  x + y\n\n",
            "fn g x\n  x\n\n",
            "main = print \"{f 1} {f 1 2} {g 3}\"\n"
        );
        let program = crate::compile("test.kso", src, false).expect("the fixture compiles");
        agrees_over(&program);
    }
}

#[cfg(test)]
mod a_trampoline_is_emitted_for_a_call_that_uses_it {
    use super::narrow_tailcc;

    /// Five KValues are ten registers, past the eight AArch64 passes, and the
    /// musttail keeps the convention, so the function spills.
    const LOOP: &str = "define tailcc %KValue @f(%KValue %a, %KValue %b, %KValue %c, %KValue %d, %KValue %e) {\nentry:\n  %r = musttail call tailcc %KValue @f(%KValue %a, %KValue %b, %KValue %c, %KValue %d, %KValue %e)\n  ret %KValue %r\n}\n";

    #[test]
    fn none_when_nothing_is_rerouted() {
        let out = narrow_tailcc(LOOP.to_string());
        assert!(!out.contains("@f.c("), "a trampoline nothing calls was emitted:\n{out}");
    }

    #[test]
    fn one_when_a_call_goes_through_it() {
        let caller = "define %KValue @g(%KValue %a) {\nentry:\n  %r = call tailcc %KValue @f(%KValue %a, %KValue %a, %KValue %a, %KValue %a, %KValue %a)\n  ret %KValue %r\n}\n";
        let out = narrow_tailcc(format!("{LOOP}{caller}"));
        assert!(
            out.contains("define %KValue @f.c("),
            "the rerouted call has no trampoline:\n{out}"
        );
        assert!(out.contains("call %KValue @f.c("), "the call was not rerouted:\n{out}");
    }
}
