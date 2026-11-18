// What the host's clang and linkers can do, probed once and remembered, and
// the padded names temp paths are built from. main.rs reads this file with
// include!, so its items stay in the binary's root module in the order they
// had there, and the compiler builds as it did (kanso#1848, kanso#2502).
// cargo fmt does not reach an included file, so CI's format job checks the
// files under src/'s subdirectories itself.

/// Whether this host's clang can take `preserve_none` on a closure body.
///
/// PROBED WITH THE .ll FORM, and deliberately not with the C attribute: the
/// two fail in opposite ways. clang 18 rejects `preserve_nonecc` at the
/// parser and merely WARNS about `__attribute__((preserve_none))`, so a probe
/// built on the C half answers yes on a toolchain that would then refuse the
/// module the emitter writes. The C side carries
/// `-Werror=unknown-attributes` for the same reason, so a wrong answer breaks
/// the build at the first define rather than producing a binary whose two
/// halves disagree about registers.
///
/// Asked once per process. The emitted IR and the runtime object must agree,
/// so `cached_runtime_object` keys on this too.
fn closure_convention() -> kanso::codegen::ClosureConvention {
    use kanso::codegen::ClosureConvention;
    static ANSWER: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    match *ANSWER.get_or_init(remembered_probe) {
        true => ClosureConvention::PreserveNone,
        false => ClosureConvention::Absent,
    }
}

/// The probe's answer, kept on disk beside the runtime objects.
///
/// The answer belongs to the clang binary, not to the process asking, and the
/// probe is a whole clang run: 32,201,483 instructions, 6.4% of a dev build
/// of the codegen corpus, paid by every build to learn what the last one
/// learned. It is keyed by the clang the PATH resolves to, followed through
/// its symlinks, with that file's size and modification time, so installing
/// another clang asks again. A key that cannot be formed, or a file that holds
/// anything but the one byte this writes, means asking.
fn remembered_probe() -> bool {
    let Some(identity) = clang_identity() else { return preserve_none_probe() };
    let key = identity
        .bytes()
        .fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3));
    let key = hex16(key);
    let path = std::env::temp_dir().join(format!("kanso_pn_answer_{key}"));
    match std::fs::read(&path).ok().as_deref() {
        Some(b"1") => return true,
        Some(b"0") => return false,
        _ => {}
    }
    let answer = preserve_none_probe();
    // Written under a name of its own and renamed into place, so a build
    // running beside this one reads the whole byte or nothing.
    let staged = std::env::temp_dir().join(format!("kanso_pn_answer_{key}_{}", pid_tag()));
    if std::fs::write(&staged, if answer { b"1" } else { b"0" }).is_ok() {
        let _ = std::fs::rename(&staged, &path);
    }
    answer
}

/// The clang this process would run, as a path followed through its links,
/// with its size and modification time.
fn clang_identity() -> Option<String> {
    tool_identity("clang")
}

/// Whether clang can hand an LTO link to lld, asked once per pair of tools.
///
/// lld links a release build for 4.26% fewer instructions than GNU ld with
/// LLVM's plugin, measured on the codegen corpus on this container:
/// 1,750,778,100 against 1,676,140,277. Nearly all of the difference is the
/// linker's own work -- the plugin process spent 128,765,719 in libc and
/// 52,810,020 in libbfd around LLVM's 926,724,119 -- and the program it
/// produces runs within 281 instructions of GNU ld's on runbench.
///
/// It is asked rather than assumed because an lld from another LLVM release
/// than clang's cannot read clang's bitcode, and a runner can carry one: the
/// probe links a one-line LTO program, and anything short of success keeps
/// GNU ld. The answer is remembered under clang's identity and that of the
/// `ld.lld` on PATH, if there is one.
fn lld_links_lto() -> bool {
    // clang finds lld in its own LLVM directory as well as on PATH, so what
    // is on PATH is part of the key and not a condition: a runner with lld
    // only beside clang answered the spec's probe yes while this said no
    // without asking.
    let Some(clang) = tool_identity("clang") else {
        return false;
    };
    let lld = tool_identity("ld.lld").unwrap_or_default();
    let key = format!("{clang}|{lld}")
        .bytes()
        .fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3));
    let key = hex16(key);
    let path = std::env::temp_dir().join(format!("kanso_lld_answer_{key}"));
    match std::fs::read(&path).ok().as_deref() {
        Some(b"1") => return true,
        Some(b"0") => return false,
        _ => {}
    }
    let answer = lld_probe();
    let staged = std::env::temp_dir().join(format!("kanso_lld_answer_{key}_{}", pid_tag()));
    if std::fs::write(&staged, if answer { b"1" } else { b"0" }).is_ok() {
        let _ = std::fs::rename(&staged, &path);
    }
    answer
}

/// LLVM 19's `lto<O3>` pipeline, as `opt -print-pipeline-passes` writes it,
/// with `deadargelim` taken out and four passes put in: non-trivial loop
/// unswitching, `correlated-propagation` twice and `early-cse<memssa>`.
///
/// Dead-argument elimination judges the words of a two-word return one at a
/// time, by what the callers read, and keeps a function's arguments whole
/// when its body makes a `musttail` call but not its return. A function whose
/// callers read only the tag loses the second word while its `musttail`
/// callee keeps both, the two disagree, and the verifier aborts the link. A
/// program declaring a subtype and calling std/regexp met it in 2026-09-28's
/// generated programs, in the library's predicates. Which functions it strikes
/// depends on what inlining did first, so the emitter cannot steer around it:
/// writing the calls between functions as plain `tail` calls cost the run
/// program 1.82%, and giving each such function a kept caller cost 5.1%.
///
/// Before unswitching joined it, this pipeline handed to lld unchanged linked
/// runbench byte for byte as the default does, and without `deadargelim` the
/// run program read 1,071,975,512 against 1,071,414,767, +0.052%.
///
/// `simple-loop-unswitch<nontrivial;trivial>` runs beside the first `licm`.
/// The default pipeline has no unswitch pass at the link, so a test of a value
/// a loop does not change stays inside the loop even after `licm` has hoisted
/// the value: the escape loop multiplied `k * 31` once and tested its overflow
/// flag on every element. Unswitching copies the loop once for each outcome of
/// the test, and the copy that runs has no test in it. The run program fell
/// 0.422% and the codegen corpus's link rose 188,548 instructions, 0.078% of
/// ld.lld (2026-10-07).
///
/// `correlated-propagation` runs after each `jump-threading`, and
/// `early-cse<memssa>` before the first. `lto<O3>` names neither, and the
/// `-O1` compile before the link runs `early-cse` but no correlated
/// propagation. Correlated propagation drops an overflow test that an
/// earlier test on the same value has already decided: in the escape loop,
/// `n * 7` not overflowing bounds `n`, and the test on `n - 1` goes. The run
/// program fell a further 0.699% and the corpus's link rose 8,142,538
/// instructions, 3.4% of ld.lld (2026-10-07).
const LTO_O3_WITHOUT_DEADARGELIM: &str = concat!(
    "cross-dso-cfi,openmp-opt,globaldce<vfe-linkage-unit-visibility>,inferatt",
    "rs,function<eager-inv>(callsite-splitting),pgo-icall-prom,ipsccp,called-",
    "value-propagation,cgscc(function-attrs),rpo-function-attrs,globalsplit,w",
    "holeprogramdevirt,globalopt,function(mem2reg),constmerge,function<eager-",
    "inv>(instcombine<max-iterations=1;no-use-loop-info;no-verify-fixpoint>,a",
    "ggressive-instcombine),cgscc(inline<only-mandatory>,inline),globalopt,op",
    "enmp-opt,globaldce<vfe-linkage-unit-visibility>,cgscc(argpromotion),func",
    "tion<eager-inv>(instcombine<max-iterations=1;no-use-loop-info;no-verify-",
    "fixpoint>,constraint-elimination,early-cse<memssa>,jump-threading,correlated-propagation,sroa<modify-cfg>,tailcal",
    "lelim),cgscc(function-attrs),require<globals-aa>,function(invalidate<aa>",
    "),cgscc(openmp-opt-cgscc),function<eager-inv>(loop-mssa(licm<allowspecul",
    "ation>,simple-loop-unswitch<nontrivial;trivial>),gvn<>,memcpyopt,dse,move-auto-init,mldst-motion<no-split-footer-",
    "bb>,loop(indvars,loop-deletion,loop-unroll-full),loop-distribute,loop-ve",
    "ctorize<no-interleave-forced-only;no-vectorize-forced-only;>,infer-align",
    "ment,loop-unroll<O3>,transform-warning,sroa<preserve-cfg>,instcombine<ma",
    "x-iterations=1;no-use-loop-info;no-verify-fixpoint>,simplifycfg<bonus-in",
    "st-threshold=1;forward-switch-cond;switch-range-to-icmp;switch-to-lookup",
    ";no-keep-loops;hoist-common-insts;sink-common-insts;speculate-blocks;sim",
    "plify-cond-branch;no-speculate-unpredictables>,sccp,instcombine<max-iter",
    "ations=1;no-use-loop-info;no-verify-fixpoint>,bdce,slp-vectorizer,vector",
    "-combine,infer-alignment,instcombine<max-iterations=1;no-use-loop-info;n",
    "o-verify-fixpoint>,loop-mssa(licm<allowspeculation>),alignment-from-assu",
    "mptions,jump-threading,correlated-propagation),lowertypetests,lowertypetests,function(loop-sink",
    ",div-rem-pairs,simplifycfg<bonus-inst-threshold=1;no-forward-switch-cond",
    ";switch-range-to-icmp;no-switch-to-lookup;keep-loops;hoist-common-insts;",
    "no-sink-common-insts;speculate-blocks;simplify-cond-branch;speculate-unp",
    "redictables>),elim-avail-extern,globaldce<vfe-linkage-unit-visibility>,c",
    "g-profile,function(annotation-remarks),verify",
);

/// The arguments that run the link's optimization as `LTO_O3_WITHOUT_DEADARGELIM`
/// says, or none.
///
/// The pipeline is LLVM 19's, and another release's default may differ, so
/// it is used only when clang says it is 19 or 18 and a one-line LTO program
/// links with it through lld. Elsewhere the link keeps its own default. The
/// answer is remembered under clang's identity, as `lld_links_lto`'s is.
fn lto_pipeline_args() -> Vec<String> {
    if lld_args().is_empty() {
        return Vec::new();
    }
    let Some(clang) = tool_identity("clang") else {
        return Vec::new();
    };
    let key = format!("{clang}|major|{}", LTO_O3_WITHOUT_DEADARGELIM)
        .bytes()
        .fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3));
    let path = std::env::temp_dir().join(format!("kanso_lto_pipeline_answer_{}", hex16(key)));
    let answer = match std::fs::read(&path).ok().as_deref() {
        Some(b"19") => Some(19),
        Some(b"18") => Some(18),
        Some(b"0") => None,
        _ => {
            let answer = lto_pipeline_probe();
            let staged = std::env::temp_dir().join(format!(
                "kanso_lto_pipeline_answer_{}_{}",
                hex16(key),
                pid_tag()
            ));
            let word = answer.map_or("0".to_string(), |m| m.to_string());
            if std::fs::write(&staged, word).is_ok() {
                let _ = std::fs::rename(&staged, &path);
            }
            answer
        }
    };
    match answer {
        Some(major) => {
            vec!["-Xlinker".to_string(), format!("--lto-newpm-passes={}", lto_pipeline_for(major))]
        }
        None => Vec::new(),
    }
}

/// `LTO_O3_WITHOUT_DEADARGELIM` as the given LLVM release spells it.
///
/// 18's `lto<O3>`, as its `opt -print-pipeline-passes` writes it, is 19's
/// pass for pass, less simplifycfg's `speculate-unpredictables` option, which
/// 19 added. 18 needs the pipeline as much as 19 does: kq's LTO link under
/// clang 18 aborted in `deadargelim` once tail calls through closures became
/// `musttail` in tailcc lambdas, the same disagreement of two-word returns the
/// constant's comment describes.
fn lto_pipeline_for(major: u32) -> String {
    match major {
        18 => LTO_O3_WITHOUT_DEADARGELIM
            .replace(";no-speculate-unpredictables", "")
            .replace(";speculate-unpredictables", ""),
        _ => LTO_O3_WITHOUT_DEADARGELIM.to_string(),
    }
}

/// The LLVM release whose spelling of the pipeline links here, if one does.
fn lto_pipeline_probe() -> Option<u32> {
    [19, 18].into_iter().find(|&major| lto_pipeline_links(major))
}

fn lto_pipeline_links(major: u32) -> bool {
    let dir = std::env::temp_dir();
    let c = dir.join(format!("kanso_lto_pipeline_probe_{}.c", pid_tag()));
    let ll = dir.join(format!("kanso_lto_pipeline_probe_{}.ll", pid_tag()));
    let out = dir.join(format!("kanso_lto_pipeline_probe_{}", pid_tag()));
    // The pipeline is spelled for one major version, which is asked with the
    // preprocessor: a status, where reading `--version` would drain a pipe.
    // Output thrown away rather than read, for the reason
    // `preserve_none_probe` gives.
    let quiet = |cmd: &mut std::process::Command| {
        cmd.stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    };
    let check = format!("#if __clang_major__ != {major}\n#error\n#endif\n");
    let ok = std::fs::write(&c, check).is_ok()
        && std::fs::write(&ll, "define i32 @main() {\n  ret i32 0\n}\n").is_ok()
        && quiet(std::process::Command::new("clang").arg("-fsyntax-only").arg(&c))
        && quiet(
            std::process::Command::new("clang")
                .args(["-O1", "-flto", "-Wno-override-module"])
                .args(lld_args())
                .arg("-Xlinker")
                .arg(format!("--lto-newpm-passes={}", lto_pipeline_for(major)))
                .arg(&ll)
                .arg("-o")
                .arg(&out),
        );
    let _ = std::fs::remove_file(&c);
    let _ = std::fs::remove_file(&ll);
    let _ = std::fs::remove_file(&out);
    ok
}

/// The arguments that put a link in lld, or none.
///
/// lld links on every hardware thread by default, which a user's build wants
/// and a measurement cannot have: under callgrind the thread pool's
/// scheduling lands in the count, and the codegen rows read 434,345,526 and
/// 434,337,763 on two CI runs of one tree, and 1,677,317,287 and 1,677,792,392
/// on release. Three dev links here read 434,957,073, 434,928,291 and
/// 434,926,291 on the default and 434,600,869 three times with `--threads=1`.
/// `KANSO_LTO_JOBS` is already how a measurement asks for one LTO job, so it
/// sets lld's thread count as well.
fn lld_args() -> Vec<String> {
    if !(cfg!(target_os = "linux") && lld_links_lto()) {
        return Vec::new();
    }
    let mut args = vec!["-fuse-ld=lld".to_string()];
    if let Ok(n) = std::env::var("KANSO_LTO_JOBS") {
        if !n.is_empty() {
            args.push(format!("-Wl,--threads={n}"));
        }
    }
    args
}

fn lld_probe() -> bool {
    let dir = std::env::temp_dir();
    let ll = dir.join(format!("kanso_lld_probe_{}.ll", pid_tag()));
    let out = dir.join(format!("kanso_lld_probe_{}", pid_tag()));
    if std::fs::write(&ll, "define i32 @main() {\n  ret i32 0\n}\n").is_err() {
        return false;
    }
    // Output thrown away rather than read, for the reason `preserve_none_probe`
    // gives.
    let ok = std::process::Command::new("clang")
        .args(["-O1", "-flto", "-fuse-ld=lld", "-Wl,-plugin-opt=O3", "-Wno-override-module"])
        .arg(&ll)
        .arg("-o")
        .arg(&out)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let _ = std::fs::remove_file(&ll);
    let _ = std::fs::remove_file(&out);
    ok
}

/// The arguments that link a dev build: gold where it links, and otherwise
/// what `lld_args` says.
///
/// A dev link has no LTO in it, so any linker can take it, and the three on a
/// Linux runner cost very different amounts to link the codegen corpus:
/// GNU ld 82,492,510 instructions, lld 43,557,070 and gold 29,108,334. Most
/// of lld's lead over gold goes before it reads an input. lld is linked
/// against libLLVM, and the dynamic loader spends about 17.8 million
/// instructions resolving libLLVM's symbols before lld reads anything.
///
/// Gold also drops the build ID. The driver asks for one on every link, gold
/// computes it as a SHA-1 of the whole output, and that is 4,346,261 of its
/// instructions. Nothing reads the ID of a dev binary, which is rebuilt
/// whenever its source changes. Without it, gold links the corpus in
/// 24,762,073.
///
/// Asked rather than assumed, like lld: a probe links a one-line program with
/// gold, and anything short of success leaves the link to `lld_args`.
fn dev_link_args() -> Vec<String> {
    if cfg!(target_os = "linux") && gold_links() {
        return vec!["-fuse-ld=gold".to_string(), "-Wl,--build-id=none".to_string()];
    }
    lld_args()
}

/// Whether clang can link a plain program with gold, asked once per pair of
/// tools and remembered the way `lld_links_lto` remembers its answer.
fn gold_links() -> bool {
    let Some(clang) = tool_identity("clang") else {
        return false;
    };
    let gold = tool_identity("ld.gold").unwrap_or_default();
    let key = format!("{clang}|{gold}")
        .bytes()
        .fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3));
    let key = hex16(key);
    let path = std::env::temp_dir().join(format!("kanso_gold_answer_{key}"));
    match std::fs::read(&path).ok().as_deref() {
        Some(b"1") => return true,
        Some(b"0") => return false,
        _ => {}
    }
    let answer = gold_probe();
    let staged = std::env::temp_dir().join(format!("kanso_gold_answer_{key}_{}", pid_tag()));
    if std::fs::write(&staged, if answer { b"1" } else { b"0" }).is_ok() {
        let _ = std::fs::rename(&staged, &path);
    }
    answer
}

fn gold_probe() -> bool {
    let dir = std::env::temp_dir();
    let ll = dir.join(format!("kanso_gold_probe_{}.ll", pid_tag()));
    let out = dir.join(format!("kanso_gold_probe_{}", pid_tag()));
    if std::fs::write(&ll, "define i32 @main() {\n  ret i32 0\n}\n").is_err() {
        return false;
    }
    let ok = std::process::Command::new("clang")
        .args(["-O0", "-fuse-ld=gold", "-Wl,--build-id=none", "-Wno-override-module"])
        .arg(&ll)
        .arg("-o")
        .arg(&out)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let _ = std::fs::remove_file(&ll);
    let _ = std::fs::remove_file(&out);
    ok
}

/// A tool on PATH, as a path followed through its links, with its size and
/// modification time.
fn tool_identity(name: &str) -> Option<String> {
    let dirs = std::env::var_os("PATH")?;
    let found = std::env::split_paths(&dirs).map(|d| d.join(name)).find(|p| p.is_file())?;
    let real = std::fs::canonicalize(&found).ok()?;
    let meta = std::fs::metadata(&real).ok()?;
    let modified = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(format!("{}:{}:{}", real.display(), meta.len(), modified.as_nanos()))
}

/// The process id, always the same number of characters.
///
/// EVERY TEMP PATH THIS FILE BUILDS CARRIES ONE, and a path's LENGTH is a term
/// in what the process costs: the bytes are copied, walked and handed to
/// `open`. A pid is anywhere from one digit to seven, so two runs of one binary
/// on one box wrote paths of different lengths and counted different
/// instructions for doing the same thing. On 2026-09-17 that was the whole of
/// what the codegen row's second reading still disagreed by once the warm-up
/// was fixed: 120 instructions out of 1,071,604,729 on the dev tier and 582 out
/// of 7,307,728,731 on release, on one binary in one job.
///
/// Seven digits covers every pid Linux hands out under the default
/// `pid_max` of 4,194,304. A larger `pid_max` widens the field rather than
/// truncating it, so uniqueness is never at risk; the length guarantee is,
/// and `tests/a_temp_path_is_the_same_length_every_run.rs` says so in the
/// same breath as it pins the width.
fn pid_tag() -> String {
    pid_tag_of(std::process::id())
}

/// Split out from `pid_tag` so the width can be asked about a pid this process
/// does not have.
fn pid_tag_of(pid: u32) -> String {
    let width = (pid.checked_ilog10().unwrap_or(0) as usize + 1).max(7);
    let mut digits = vec![b'0'; width];
    let mut rest = pid;
    for slot in digits.iter_mut().rev() {
        *slot = b'0' + (rest % 10) as u8;
        rest /= 10;
    }
    String::from_utf8(digits).expect("decimal digits are ascii")
}

/// Sixteen hex digits, with the same work for every value.
///
/// A key's COST was a function of its value. `{:016x}` writes the digits a
/// value has and pads the rest one `write_char` at a time, and most keys here
/// hash a tool's identity, which carries the file's modification time, so the
/// value changes from one runner image to the next. On 2026-09-25 the start-up
/// row read 52,427 on one runner and 52,375 on another for one binary, and the
/// 52 were `pad_integral` and `write_char` naming the gold answer's file.
/// `pid_tag_of` writes its seven digits the same way, for the same reason.
fn hex16(n: u64) -> String {
    let digits: Vec<u8> =
        (0..16).map(|i| b"0123456789abcdef"[(n >> (60 - 4 * i)) as usize & 15]).collect();
    String::from_utf8(digits).expect("hex digits are ascii")
}

/// Compile a two-define module: one carrying the convention, one calling
/// through it. Both halves are there because a toolchain that parsed the
/// define and refused the call site would still refuse what the emitter
/// writes, and a probe testing only the define would not know.
fn preserve_none_probe() -> bool {
    let dir = std::env::temp_dir();
    let ll = dir.join(format!("kanso_pn_probe_{}.ll", pid_tag()));
    let obj = dir.join(format!("kanso_pn_probe_{}.o", pid_tag()));
    let module = "define preserve_nonecc i64 @p(i64 %x) { ret i64 %x }\n\
                  define i64 @q(i64 %x) {\n\
                  \x20 %r = call preserve_nonecc i64 @p(i64 %x)\n\
                  \x20 ret i64 %r\n\
                  }\n";
    if std::fs::write(&ll, module).is_err() {
        return false;
    }
    // `status()` WITH THE CHILD'S OUTPUT THROWN AWAY, not `output()`, and the
    // difference is measurable. Only `status.success()` was ever read here, but
    // `output()` opens pipes and drains them, and how many `poll` and `read`
    // calls that takes depends on when the child's bytes arrive rather than on
    // anything the compiler did. Measured 2026-09-17: two runs of one binary on
    // one box, same corpus, same environment, counted 1,016,046,470 and
    // 1,016,048,745 for `kanso build`, and eight frames of ten thousand six
    // hundred and forty-two accounted for every instruction of the difference:
    // `FileDesc::read_to_end` +903, `small_probe_read` +591, `read_output`
    // +253, `read` +220, `__memcpy_avx` +176, `poll` +88, `__errno_location`
    // +44. All of it is the pipe-draining loop. With no pipe there is no loop.
    let ok = std::process::Command::new("clang")
        .args(["-Wno-override-module", "-c"])
        .arg(&ll)
        .arg("-o")
        .arg(&obj)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let _ = std::fs::remove_file(&ll);
    let _ = std::fs::remove_file(&obj);
    ok
}

#[cfg(test)]
mod a_temp_path_is_the_same_length_every_run {
    use super::pid_tag_of;

    /// Every pid Linux hands out under the default `pid_max` renders to the
    /// same number of characters, so two runs of one binary build paths of
    /// one length.
    #[test]
    fn every_pid_under_the_default_ceiling_is_seven_characters() {
        for pid in [1u32, 2, 9, 10, 99, 100, 999, 1000, 65_535, 999_999, 4_194_303, 4_194_304] {
            let tag = pid_tag_of(pid);
            assert_eq!(
                tag.len(),
                7,
                "pid {pid} rendered {tag:?}, {} characters. A path whose length \
                 moves with the pid makes two runs of one binary count different \
                 instructions for the same work.",
                tag.len()
            );
        }
    }

    /// The hex writer spells what `{:016x}` spells, leading zeros included.
    #[test]
    fn a_key_reads_as_the_padded_hex_it_replaced() {
        for n in [0u64, 1, 0xf, 0x0123_4567_89ab_cdef, 0x0fff_ffff_ffff_ffff, u64::MAX] {
            assert_eq!(super::hex16(n), format!("{n:016x}"));
        }
    }

    /// And it is still a pid: padding may not collide two of them.
    #[test]
    fn padding_keeps_every_pid_distinct() {
        let pids = [1u32, 10, 100, 1000, 10_000, 100_000, 1_000_000, 4_194_303];
        let tags: std::collections::BTreeSet<String> =
            pids.iter().map(|p| pid_tag_of(*p)).collect();
        assert_eq!(tags.len(), pids.len(), "padding collided two pids: {tags:?}");
    }

    /// Past the default ceiling the field widens rather than truncating. A
    /// host with a larger `pid_max` loses the length guarantee and keeps
    /// uniqueness, which is the right way round.
    #[test]
    fn a_wider_pid_widens_the_field() {
        assert_eq!(pid_tag_of(12_345_678), "12345678");
        assert_ne!(pid_tag_of(12_345_678), pid_tag_of(2_345_678));
    }
}
