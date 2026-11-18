// Driving clang for the two build tiers, and the runtime objects they link,
// compiled once and cached by what built them. main.rs reads this file with
// include!, so its items stay in the binary's root module in the order they
// had there, and the compiler builds as it did (kanso#1848, kanso#2502).
// cargo fmt does not reach an included file, so CI's format job checks the
// files under src/'s subdirectories itself.

/// Release: whole-program LTO across the program and a freshly compiled
/// runtime — the slowest build and the fastest binary.
fn release_clang(stem: &str, ll_path: &str) -> std::io::Result<std::process::ExitStatus> {
    // THE RUNTIME IS LINKED AS MACHINE CODE, NOT AS BITCODE. It was `-O3
    // -flto`, which put the whole runtime through the LTO link beside the
    // program on every release build, and the runtime is three quarters of
    // what that link generates code for: on the codegen corpus 59,082 bytes
    // of runtime `.text` against 19,135 of program. Built native, the release
    // row reads 2,848,583,206 against 6,598,715,476, -56.8%, and the run
    // program pays +3.38%, almost all of it three helpers the link used to
    // inline -- `k_b_find2_raw`, `k_b_find2_below_raw` and `k_beat_iter`.
    // The objective weighs the two at about +1.6 and -0.26. ThinLTO was the
    // other way to shrink this link and was declined at -3.47% for +3.03%;
    // design/compiler-log.md has both.
    //
    // The runtime inlines at the program's threshold. The LTO link applied
    // `-inline-threshold=2000` to the runtime as well while the runtime was
    // bitcode, and compiled on its own it fell back to clang's 225: helpers
    // such as `k_b_push_grow` stopped being inlined into their callers inside
    // the runtime. On runbench the ladder read 1,811,839,413 at 225,
    // 1,809,106,361 at 1000, 1,806,069,074 at 2000 and 1,805,972,054 at 4000.
    // The object is compiled once and cached, so the build pays it once.
    let runtime_obj = linked_as(
        cached_runtime_object("release", &["-O3", "-mllvm", "-inline-threshold=2000"])?,
        "kanso_linked_runtime_release.o",
    );
    let hot_obj = linked_as(
        cached_object("release_hot", Some(HOT_RECIPE), &|| hot_source().into(), &["-O3", "-flto"])?,
        "kanso_linked_runtime_release_hot.o",
    );
    std::process::Command::new("clang")
        // THE PROGRAM IS OPTIMIZED ONCE, AT THE LINK. `-O3` here ran the full
        // pipeline on the program's IR as clang -cc1 turned it into bitcode,
        // and the LTO link then ran it again over program and hot helpers
        // together. `-O1` before the link and `plugin-opt=O3` at it, on this
        // container: the release codegen row 2,903,108,801 -> 1,751,444,900,
        // -39.7%, and the run program 1,806,069,074 -> 1,895,750,256, +4.97%.
        // `-O2` read 2,536,822,676 and 1,815,479,975; no pre-link passes at
        // all read 1,380,698,213 and 2,053,358,047, +13.7%, because the link's
        // pipeline expects its input already simplified. Scored by the
        // objective against main's goldens, -O1 is +0.19 and -O2 +0.11.
        //
        // On Linux only. `-plugin-opt` is the gold plugin's spelling and
        // Apple's ld64 refuses it ("ld: unknown options: -plugin-opt=O3"),
        // so elsewhere both steps stay at -O3 as they were.
        .args(if cfg!(target_os = "linux") {
            &["-O1", "-Wl,-plugin-opt=O3"][..]
        } else {
            &["-O3"][..]
        })
        .arg("-flto")
        // lld where it can take the LTO link: `lld_links_lto` says why.
        .args(lld_args())
        // and LLVM 19's own link pipeline without one pass: `lto_pipeline_args`.
        .args(lto_pipeline_args())
        // Eight times clang's default of 250. The run program spends one
        // instruction in ten on `push`, `pop` and `ret` -- 215,229,225 of
        // 2,185,625,151 in the binary's own code -- and the functions paying
        // most are small and hot rather than looping: k_map_sorted is 49.6%
        // frame, string_at and entry_onto 20.0% each. Inlining is what
        // removes a frame, so the threshold is the lever.
        //
        // THE LADDER IS NOT MONOTONE, which is why the value here was found
        // by measuring rather than by reasoning. On runbench.ll against 1000:
        // 1250 reads -0.9473%, 1500 reads -0.8295% -- WORSE than 1250 while
        // costing 10,336 more bytes -- 2000 reads -2.0948% and 3000 reads
        // -2.3746%. A wider threshold admits a different SET of inlinings,
        // and a superset of decisions is not a better program, so no value
        // between two measured ones may be assumed to lie between them.
        //
        // 2000 is taken because 3000 costs 40.5% of .text for a further
        // 0.28%, where 2000 costs 13.2%. Those are this container's ladder,
        // taken by linking runbench.ll and counting; CI's own work row read
        // -1.2281% for the same step, 59% of what the ladder projected, so
        // the ladder orders the rungs and does not size them.
        //
        // Machine-code size has no welfare term -- Clay ruled that on
        // 2026-09-05 -- but `.text` keeps its own exact vein, so the growth
        // is watched even though it is not scored.
        //
        // WHICH MEANS THE .TEXT REASON ABOVE IS NOT WHY 2000 STANDS, and on
        // 2026-09-19 the scored reason was measured. The ladder carries two
        // more rungs, taken the same way, on a runbench.ll byte-identical
        // across every arm:
        //
        //     225    1,938,999,983   343,128 bytes   22,317,482 calls
        //     2000   1,840,276,313   424,088         18,812,341
        //     4000   1,823,291,354   510,152
        //     8000   1,821,134,592   567,496
        //
        // 4000 is -0.923% of the run and 8000 a further -0.118%. What stops
        // 4000 is `codegen_instructions_release`, which is a PRODUCTION
        // welfare term at weight 0.15: the same box, two passes, each arm
        // byte-identical, reads 6,827,333,184 against 7,075,918,942, a rise
        // of 3.64%. The dev tier does not move at all (595,943,218 both
        // arms), because `dev_clang` passes -O0 and no -mllvm.
        //
        // Scored against the goldens by ratio, the run gain alone is 77.27 ->
        // 77.33 and the pair together is 77.26: one hundredth BELOW the
        // floor. The break-even, worked before the codegen arm finished, was
        // a release build 3.2% dearer, and it came in at 3.64%. And the
        // paragraph above records that CI's work row moved 59% of what this
        // container's ladder projected for 1000 -> 2000; at that travel the
        // run gain is nearer 0.54% and the trade is not close.
        //
        // So 2000 stands on the objective rather than on `.text`, and 4000 is
        // measured-and-declined. design/compiler-log.md carries the frame
        // measurement this came out of.
        //
        // THE 3.64% ABOVE WAS A CLANG 18 FIGURE, and the reason still holds
        // without it. Until kanso#1795 the codegen gate found clang 18, which
        // builds without this compiler's pipeline. Measured again under clang
        // 19 on 2026-10-07, 4000 takes 0.650% off runbench and leaves the
        // codegen corpus's `clang -cc1` and ld.lld byte-identical, because the
        // corpus has no function whose cost falls between the two thresholds.
        // Building runbench itself costs `clang -cc1` 38.7% more and ld.lld
        // 32.6% more. The log entry of that date has the figures.
        .arg("-mllvm")
        .arg("-inline-threshold=2000")
        .args(if cfg!(target_arch = "x86_64") { &["-mssse3"][..] } else { &[][..] })
        // HOW MANY THREADS THE LINKER'S LTO MAY USE, when a measurement asks.
        // Unset -- which is every build but a gate's -- the plugin picks, and a
        // release build keeps every core it can get.
        //
        // The gate asks for one, because a row cannot be pinned to a number the
        // scheduler helps choose. `ld` splits LTO codegen across threads,
        // callgrind counts every thread, and how the work lands is not a
        // property of the input: two links of byte-identical bitcode on this
        // container read 20,565,047,254 and 20,565,047,243 on one pair and
        // 20,565,047,241 and 20,565,049,584 on the next, while both `clang`
        // children came back byte for byte every time. With `jobs=1` the same
        // pair reads 20,574,502,681 twice.
        //
        // That is the 2026-09-15 rule applied where it belongs: the state is
        // put into a known one for the measurement rather than explained
        // afterwards. It is NOT made the default, because a user's release
        // build has no row to keep and every reason to use the cores.
        .args(match std::env::var("KANSO_LTO_JOBS") {
            Ok(n) if !n.is_empty() => vec![format!("-Wl,-plugin-opt=jobs={n}")],
            _ => vec![],
        })
        // AND THE LTO OBJECT GETS A NAME THE RUN CHOOSES, for the measurement
        // only. clang writes it to `/tmp/<stem>-XXXXXX.o` with fresh hex every
        // run; `ld`'s LLVM plugin puts that path into a `StringMap`, and the
        // probe length depends on the string. Twenty-two names were measured on
        // one binary and one corpus with every other input held: twenty read
        // 5,163,341,031 and two -- `4b8c1a` and `fedcba` -- read 5,163,341,042.
        // Eleven apart, deterministic per name, about one name in eleven.
        //
        // That is the eleven `codegen_instructions_release` has been
        // disagreeing with itself by since kanso#1507 pinned the thread count.
        // kanso#1502's own job drew both buckets: 6,820,866,344 and then
        // 6,820,866,355. About one job in eleven goes red on that row for
        // nothing a diff can explain, and three rounds were spent on it.
        //
        // `-save-temps=obj` writes the object beside the output under a name
        // derived from the input, so the string is the same every run. It is
        // the 2026-09-15 rule again: the state is put into a known one rather
        // than explained afterwards. NOT the default, for the same reason
        // `KANSO_LTO_JOBS` is not -- it leaves a file in a user's directory and
        // a user's build has no row to keep.
        .args(match std::env::var("KANSO_FIXED_TEMPS") {
            Ok(v) if !v.is_empty() => vec!["-save-temps=obj"],
            _ => vec![],
        })
        .arg("-Wno-override-module")
        .arg("-o")
        .arg(stem)
        .arg(ll_path)
        .arg(&hot_obj)
        .arg(&runtime_obj)
        .arg("-lm")
        .without_driver()
}

/// Dev (the default): the program compiles unoptimized and links against a
/// cached optimized runtime object, so the runtime's cost is paid once per
/// runtime version, not per build.
fn dev_clang(stem: &str, ll_path: &str) -> std::io::Result<std::process::ExitStatus> {
    let runtime_obj =
        linked_as(cached_runtime_object("dev", DEV_RUNTIME_OPT)?, "kanso_linked_runtime_dev.o");
    std::process::Command::new("clang")
        .arg("-O0")
        // Gold where it links, lld where it does not: `dev_link_args` has
        // the measurements.
        .args(dev_link_args())
        .arg("-Wno-override-module")
        .arg("-o")
        .arg(stem)
        .arg(ll_path)
        .arg(&runtime_obj)
        .arg("-lm")
        .without_driver()
}

/// A clang command that runs the jobs its driver would have run, without the
/// driver.
///
/// Every LLVM process spends most of its start-up relocating libLLVM: the
/// driver of a dev build is 31,702,963 instructions, 83% of them in the
/// dynamic loader, and it does nothing a build needs beyond deciding the two
/// commands it then spawns -- `clang -cc1` and the linker. Those commands
/// depend on the toolchain, the flags and the file names, and on nothing a
/// program's IR says. So the driver is asked once, with `-###`, for a build
/// whose files carry placeholder names in a directory of their own; the
/// answer is kept under a key naming the tools, the flags and the objects;
/// and every later build runs the two commands with its own names put back.
///
/// Anything unexpected -- a driver that prints other than two jobs, a key
/// that cannot be formed, a job whose program is missing -- runs the driver
/// as before. `KANSO_CLANG_DRIVER` forces the driver, which is how the spec
/// compares the two.
trait WithoutDriver {
    fn without_driver(&mut self) -> std::io::Result<std::process::ExitStatus>;
}

impl WithoutDriver for std::process::Command {
    fn without_driver(&mut self) -> std::io::Result<std::process::ExitStatus> {
        let args: Vec<String> = self.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        if cfg!(target_os = "linux") && std::env::var_os("KANSO_CLANG_DRIVER").is_none() {
            if let Some(status) = replayed(&args) {
                return status;
            }
        }
        self.status()
    }
}

const IN_MARK: &str = "kanso_replay_in";
const OUT_MARK: &str = "kanso_replay_out";

/// The two jobs for `args`, run directly. None when the replay cannot be
/// trusted, and the caller runs the driver.
fn replayed(args: &[String]) -> Option<std::io::Result<std::process::ExitStatus>> {
    // `-save-temps=obj` is only ever asked for to give the LTO object a name
    // that is the same every run, because ld's plugin hashes that path and a
    // random one moved the release row by eleven instructions one run in
    // eleven. The replay's names are fixed already, so it leaves the option to
    // the driver, which still needs it.
    let args: Vec<String> = args.iter().filter(|a| *a != "-save-temps=obj").cloned().collect();
    // Exactly one `-o`, and the input is the argument after it: that is the
    // only shape `dev_clang` and `release_clang` hand over, and anything else
    // goes to the driver.
    let o = args.iter().position(|a| a == "-o")?;
    if args.iter().filter(|a| *a == "-o").count() != 1
        || args.iter().any(|a| a.starts_with("-save-temps"))
    {
        return None;
    }
    let out = std::path::Path::new(args.get(o + 1)?);
    let ll = std::path::Path::new(args.get(o + 2)?);
    let ll_name = ll.file_stem()?.to_str()?;
    let out_name = out.file_name()?.to_str()?;
    let cwd = std::env::current_dir().ok()?;
    let ll_abs = cwd.join(ll);
    let out_abs = cwd.join(out);
    let mut shape: Vec<String> = args.to_vec();
    shape[o + 1] = OUT_MARK.to_string();
    shape[o + 2] = format!("{IN_MARK}.ll");

    let identity = format!(
        "{}|{}|{}|{}|{}|{}",
        tool_identity("clang")?,
        tool_identity("ld.lld").unwrap_or_default(),
        tool_identity("ld.gold").unwrap_or_default(),
        shape.join("\u{0}"),
        std::env::var("LIBRARY_PATH").unwrap_or_default(),
        std::env::var("COMPILER_PATH").unwrap_or_default(),
    );
    let key = kanso::hash::digest_of(identity.as_bytes());
    let cache = std::env::temp_dir().join(format!("kanso_jobs_{}{}", hex16(key.0), hex16(key.1)));

    // Named for the output rather than the process. The jobs run inside it,
    // and lld's LTO reads the directory it runs in into what it hashes: with a
    // stage named for the pid, three runs of one release build read
    // 1,644,357,816 and 1,644,922,716 apart. Two builds writing the same
    // output were already racing for it, so sharing a stage costs nothing new.
    let named = kanso::hash::digest_of(out_abs.to_string_lossy().as_bytes());
    let stage = std::env::temp_dir().join(format!("kanso_stage_{}", hex16(named.0)));
    let _ = std::fs::remove_dir_all(&stage);
    std::fs::create_dir_all(&stage).ok()?;
    let stage_str = stage.to_str()?.to_string();
    let jobs = match std::fs::read_to_string(&cache).ok().and_then(|t| jobs_of(&t)) {
        Some(jobs) => jobs,
        None => {
            let jobs = asked(&shape, &stage, &stage_str)?;
            let staged = std::env::temp_dir().join(format!(
                "kanso_jobs_{}{}_{}",
                hex16(key.0),
                hex16(key.1),
                pid_tag()
            ));
            let text: Vec<String> = jobs.iter().map(|j| j.join("\u{0}")).collect();
            if std::fs::write(&staged, text.join("\n")).is_ok() {
                let _ = std::fs::rename(&staged, &cache);
            }
            jobs
        }
    };
    let _ = std::fs::remove_file(stage.join(format!("{IN_MARK}.ll")));
    #[cfg(unix)]
    std::os::unix::fs::symlink(&ll_abs, stage.join(format!("{ll_name}.ll"))).ok()?;
    let put_back = |a: &str| a.replace(IN_MARK, ll_name).replace(OUT_MARK, out_name);
    let mut status = None;
    for (n, job) in jobs.iter().enumerate() {
        let mut argv: Vec<String> = job.iter().map(|a| put_back(a)).collect();
        // The link writes where the build asked, not into the stage.
        if n + 1 == jobs.len() {
            let at = argv.iter().position(|a| a == "-o")?;
            argv[at + 1] = out_abs.to_str()?.to_string();
            if let Some(dir) = std::env::var("KANSO_LINK_DIR").ok().filter(|d| !d.is_empty()) {
                argv = searching_only(argv, &dir);
            }
        }
        let ran =
            std::process::Command::new(&argv[0]).args(&argv[1..]).current_dir(&stage).status();
        let failed = !matches!(&ran, Ok(s) if s.success());
        status = Some(ran);
        if failed {
            break;
        }
    }
    let _ = std::fs::remove_dir_all(&stage);
    status
}

/// The link job with every `-L` replaced by one directory, in the place the
/// first one held.
///
/// `ld.gold` reads each search directory whole before it looks anything up, so
/// what the link costs depends on how many files the machine has installed
/// there and how long their names are. One file added to
/// `/usr/lib/x86_64-linux-gnu` moved the dev codegen row by 3,559 instructions,
/// and removing it put the row back to the instruction. On CI the row took two
/// values on one binary, 130,465,044 and 130,465,609, and they followed the
/// runner image (`ubuntu24/20260927.320` against `20261004.327`) rather than
/// anything in the tree. The 2026-09-15 rule asks for that state to be put in
/// a known one, so the codegen gate stages a directory holding only the
/// libraries the link resolves and names it here. Unset for every other build:
/// a user's link searches where the driver says.
///
/// `-nostdlib` goes with it, and is the linker's option rather than the
/// driver's: "search only the directories on the command line". Without it
/// gold still read its own four defaults, `//lib/x86_64-linux-gnu` and the
/// rest, and the added file still moved the row by 1,864.
fn searching_only(argv: Vec<String>, dir: &str) -> Vec<String> {
    let mut out = Vec::with_capacity(argv.len() + 1);
    let mut placed = false;
    for a in argv {
        if a.starts_with("-L") {
            if !placed {
                out.push("-nostdlib".to_string());
                out.push(format!("-L{dir}"));
                placed = true;
            }
            continue;
        }
        out.push(a);
    }
    out
}

/// The driver's jobs for `shape`, with the stage and the driver's own temporary
/// object replaced by marks. None unless there are exactly two and both name a
/// program that exists.
fn asked(shape: &[String], stage: &std::path::Path, stage_str: &str) -> Option<Vec<Vec<String>>> {
    // The driver refuses an input that is not there, even when only asked.
    std::fs::write(stage.join(format!("{IN_MARK}.ll")), "").ok()?;
    // The driver prints its jobs on stderr. They go to a file rather than a
    // pipe, so no read loop that scheduling can lengthen lands in the row.
    let listing = stage.join("jobs");
    let said = std::process::Command::new("clang")
        .arg("-###")
        .args(shape)
        .current_dir(stage)
        .stdout(std::process::Stdio::null())
        .stderr(std::fs::File::create(&listing).ok()?)
        .status()
        .ok()?;
    if !said.success() {
        return None;
    }
    let text = std::fs::read_to_string(&listing).ok()?;
    let mut jobs: Vec<Vec<String>> =
        text.lines().filter(|l| l.starts_with(" \"")).map(quoted_args).collect::<Option<_>>()?;
    if jobs.len() != 2
        || jobs.iter().any(|j| j.is_empty() || !std::path::Path::new(&j[0]).is_file())
    {
        return None;
    }
    // The object cc1 writes and the linker reads: the driver names it in its
    // temp directory with a random suffix, and the replay names it beside the
    // input, relative to the stage both jobs run in. NO PATH OF THE STAGE'S
    // REACHES A JOB. The stage is named for the process, and ld's LLVM plugin
    // hashes the object's path, so a path carrying the pid would put back the
    // eleven-instruction wobble `-save-temps=obj` exists to remove. The two
    // compilation directories are the stage's too, and are only written into
    // debug information, which these builds do not ask for; they are `.`.
    let at = jobs[0].iter().position(|a| a == "-o")?;
    let temp_obj = jobs[0].get(at + 1)?.clone();
    let ours = format!("{IN_MARK}.o");
    for job in &mut jobs {
        for a in job.iter_mut() {
            *a = a.replace(&temp_obj, &ours);
            for dir in ["-fdebug-compilation-dir=", "-fcoverage-compilation-dir="] {
                if a.starts_with(dir) {
                    *a = format!("{dir}.");
                }
            }
        }
    }
    if jobs.iter().flatten().any(|a| a.contains(stage_str)) {
        return None;
    }
    Some(jobs)
}

/// One `-###` line: arguments in double quotes, with `\` escaping the next
/// character.
fn quoted_args(line: &str) -> Option<Vec<String>> {
    let mut args = Vec::new();
    let mut chars = line.chars();
    loop {
        match chars.next() {
            None => return Some(args),
            Some(' ') => continue,
            Some('"') => {
                let mut arg = String::new();
                loop {
                    match chars.next()? {
                        '\\' => arg.push(chars.next()?),
                        '"' => break,
                        c => arg.push(c),
                    }
                }
                args.push(arg);
            }
            Some(_) => return None,
        }
    }
}

fn jobs_of(text: &str) -> Option<Vec<Vec<String>>> {
    let jobs: Vec<Vec<String>> =
        text.split('\n').map(|j| j.split('\u{0}').map(str::to_string).collect()).collect();
    match jobs.len() == 2 && jobs.iter().all(|j| std::path::Path::new(&j[0]).is_file()) {
        true => Some(jobs),
        false => None,
    }
}

/// What names a cached runtime object: everything that decides what clang
/// makes of the one source. Two builds that differ in any of these must not
/// share an object.
fn runtime_key(profile: &str, opt: &[&str], preserve: bool, counting: bool) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::hash::DefaultHasher::new();
    // THE SOURCE'S DIGEST, NOT THE SOURCE. The runtime is a constant of this
    // binary, so what it hashes to was settled by the compiler that built it;
    // walking it here cost a quarter of `kanso play`'s
    // start-up and told every process the same thing. See `hash::digest_of`.
    kanso::hash::RUNTIME_DIGEST.hash(&mut hasher);
    profile.hash(&mut hasher);
    // THE CONVENTION IS NOT IN THE SOURCE -- it is a `-D` the probe decides,
    // so a machine that gains or loses clang 19 would otherwise reuse an
    // object built under the other answer and link it against IR built under
    // the new one. The two halves disagreeing about registers is a
    // miscompile, so it goes in the key.
    preserve.hash(&mut hasher);
    // The same reason, for the same kind of reason: whether the runtime carries
    // its twenty-seven counter gates is a `-D` the caller decides, not a fact
    // in the source, so an object built one way must not be handed to a build
    // that wanted the other. A shipped binary linked against a counting runtime
    // pays for gates it can never reach; a counting binary linked against a
    // gate-free one reports zeros and the goldens all move at once.
    counting.hash(&mut hasher);
    // And the flags the object is compiled with. They were not in the key, so
    // changing a profile's flags left every machine linking the object the
    // old flags built: a bitcode runtime cached before the release build went
    // native would have been linked as bitcode for as long as it stayed in
    // the temp directory, and nothing would have said so.
    opt.hash(&mut hasher);
    hasher.finish()
}

/// The name a link is handed for a cached runtime object.
///
/// A cached object's name carries a digest: of the runtime's text, and for
/// the hot unit of this file's bytes as well (`HOT_RECIPE`). The linker keeps
/// its inputs' names in a string map, and how far a probe walks depends on
/// the string. An edit to this file that changed nothing a build emits
/// renamed the hot unit, and the release codegen row moved by 17
/// instructions, every one of them the linker's. So a measured build, which
/// sets `KANSO_FIXED_TEMPS`, links a copy under a plain name, written afresh
/// by each build. Any other build links the cached file itself: it has no
/// row to keep. `tests/the_measured_link_names_its_runtime.rs` reads the
/// link's line both ways.
fn linked_as(object: std::path::PathBuf, plain: &str) -> std::path::PathBuf {
    match std::env::var("KANSO_FIXED_TEMPS") {
        Ok(v) if !v.is_empty() => {
            let named = std::env::temp_dir().join(plain);
            match std::fs::copy(&object, &named) {
                Ok(_) => named,
                Err(_) => object,
            }
        }
        _ => object,
    }
}

fn cached_runtime_object(profile: &str, opt: &[&str]) -> std::io::Result<std::path::PathBuf> {
    cached_object(profile, None, &|| kanso::RUNTIME_C.into(), opt)
}

/// What the hot unit's text is made by, beside the runtime it is cut from:
/// `hot_source_of` is in this file, so a change to it names a new object. The
/// runtime's digest alone left a build reusing a unit the old recipe made.
const HOT_RECIPE: (u64, u64) = kanso::hash::digest_of(include_bytes!("clang.rs"));

/// The helpers a release build keeps in its LTO link, as a translation unit
/// of their own. The runtime itself is machine code there, and the link can
/// only inline what it holds as bitcode: `k_b_find2_raw`,
/// `k_b_find2_below_raw` and `k_beat_iter` were 161,703,385 instructions of
/// the run program's calls when the whole runtime went native. So this unit
/// holds a copy of each such helper as a C99 inline definition, which the
/// link inlines where the program calls it and never compiles on its own;
/// the runtime defines them too, as machine code, for its own callers and for
/// any call the link leaves. See `inline_only`.
///
/// The text is taken from `runtime.c` itself rather than kept in a second
/// file, because a dozen specs read these functions out of that file and a
/// copy would drift from what they check. The typedefs come the same way.
/// The declarations below are the globals and functions the helpers reach,
/// which `runtime.c` defines with external linkage for this unit's sake.
fn hot_source() -> String {
    hot_source_of(kanso::RUNTIME_C)
}

fn hot_source_of(runtime: &str) -> String {
    let mut out = String::from(concat!(
        "#include <stdint.h>\n",
        "#include <stddef.h>\n",
        "#include <string.h>\n",
        "#if defined(__aarch64__)\n#include <arm_neon.h>\n",
        "#elif defined(__x86_64__)\n#include <tmmintrin.h>\n#endif\n",
        "#ifdef KANSO_COUNTERS_BUILD\n#define K_COUNTING 1\n#else\n#define K_COUNTING 0\n#endif\n",
        // An inline definition reaches the unit's static helpers, which C
        // warns about and which is safe here: the definition is never emitted.
        "#pragma clang diagnostic ignored \"-Wstatic-in-inline\"\n",
    ));
    for (start, end) in [
        ("typedef struct { long long tag; long long payload; } KValue;", "\n"),
        ("enum { K_INT, K_FLOAT, ", "\n"),
        ("typedef struct { char* data; int len; int cap; } KStr;", "\n"),
        ("typedef struct KBlock {", "\n"),
        ("typedef struct { KBlock* block;", " KMark;\n"),
        ("#define K_BEAT_MAX ", "\n"),
        ("typedef struct { char* data; size_t cap; size_t used; } KCarryBuf;", "} KCarry;\n"),
        ("#define K_TOKEN_BITS ", "\n"),
        ("typedef struct { KStr s; char data[8]; } KToken;", "\n"),
    ] {
        out.push_str(hot_text(runtime, start, end));
    }
    out.push_str(concat!(
        "extern KBlock* k_blocks;\n",
        "extern KStr* k_seek_str;\n",
        "extern char* k_arena;\n",
        "extern size_t k_arena_left;\n",
        "extern long long k_stat_beat_iters;\n",
        "extern long long k_stat_find2_calls;\n",
        "extern KMark k_beat_stack[K_BEAT_MAX];\n",
        "extern int k_beat_depth;\n",
        "extern KMark* k_beat_top;\n",
        "extern KMark k_beat_none;\n",
        "extern KMark* k_seek_under;\n",
        "extern int k_buf_dirty;\n",
        "extern KCarry k_carries[K_BEAT_MAX];\n",
        "extern long long k_live_block_bytes;\n",
        "extern KToken k_token_store[1 << K_TOKEN_BITS];\n",
        "extern unsigned char k_token_clean[1 << K_TOKEN_BITS];\n",
        "void k_beat_push_deep(void);\n",
        "void k_beat_rewind_slow(KMark* m);\n",
        "__attribute__((noreturn, noinline)) void k_die(const char* msg);\n",
        "extern long long k_seek_char;\n",
        "extern long k_seek_byte;\n",
        "extern long long k_stat_seek_resumes;\n",
        "extern long long k_stat_seek_steps;\n",
        "extern KValue k_ascii_cache[128];\n",
        "extern char k_ascii_ready[128];\n",
        "extern KValue k_wide_cache[256];\n",
        "extern uint32_t k_wide_key[256];\n",
        "extern unsigned char k_wide_ready[256];\n",
        "KValue k_none(void);\n",
        "KValue k_b_at_str_seek(KStr* s, long long want);\n",
        "KValue k_at_ascii_miss(KStr* s, long at);\n",
        "__attribute__((preserve_most)) KValue k_b_at_wide_miss(uint32_t q, uint32_t key, unsigned slot, long w);\n",
        "extern long long k_stat_number_spans;\n",
        "long long k_number_span_scan(const unsigned char* d, long long len, long long from);\n",
        "__attribute__((noinline, cold, preserve_most)) KValue k_b_to_int_slow(const char* data, long long len, const char* origin);\n",
        "__attribute__((noinline)) KValue k_to_float_scan_out(const char* data, long long len, const char* origin);\n",
        "KValue k_b_to_int(KValue sv, const char* origin);\n",
        "KValue k_b_to_float(KValue v, const char* origin);\n",
        "KValue k_b_slice_raw(const unsigned char* data, long long len, long long from, long long to);\n",
        "extern long long k_stat_el_parses;\n",
        "extern const struct { unsigned long long hi, lo; int e2; } k_el_pow10[];\n",
        "extern int k_stats_on;\n",
        "__attribute__((noinline, cold)) KValue k_list_empty_slow(void);\n",
        "__attribute__((noinline, cold)) KValue k_map_empty_slow(void);\n",
    ));
    // Each definition is found by its name and taken from the start of the
    // line that names it, attribute and all. The two scanners are matched on
    // the signature rather than on `always_inline`, because the ratchet row
    // that puts them out of line removes that attribute, and a lift keyed on
    // it found nothing and the tree stopped building: the row read UNBUILT
    // on kanso#1585.
    out.push_str(hot_text(runtime, "static KStr* k_as_str(KValue v) {", "\n"));
    out.push_str(hot_text(runtime, "static const unsigned char k_number_bytes[256] = {", "};\n"));
    for start in [
        "static inline int k_tail_window(",
        "static inline void k_beat_rewind(KMark* m) {",
        "__attribute__((always_inline)) void k_beat_iter(void) {",
        "__attribute__((always_inline)) void k_beat_push(void) {",
        "long long k_b_find2_raw(const unsigned char* d,",
        "long long k_b_find2_below_raw(const unsigned char* d,",
        "static long k_cp_len(unsigned char b) {",
        "static inline __attribute__((always_inline)) KValue k_at_char(KStr* s, long at) {",
        "KValue k_b_at_str(KStr* s, long long want) {",
        "static inline uint64_t k_nondigits8(uint64_t x) {",
        "static inline __attribute__((always_inline)) long long k_digits_end(const unsigned char* d,",
        "long long k_b_number_span_raw(const unsigned char* d, long long len, long long from) {",
    ] {
        out.push_str(inline_only(start));
        out.push_str(hot_line_text(runtime, start, "\n}\n"));
    }
    // The decoder's two slice parsers, with what they inline. `k_int` and
    // `k_float` are plain functions in runtime.c, so the unit takes its own
    // static copy of each rather than calling them.
    for start in ["KValue k_int(long long i) {", "KValue k_float(double d) {"] {
        out.push_str("static inline ");
        out.push_str(hot_line_text(runtime, start, "}\n"));
    }
    out.push_str(hot_text(runtime, "#define K_EL_POW10_MIN ", "\n"));
    out.push_str(hot_text(runtime, "#define K_EL_POW10_MAX ", "\n"));
    out.push_str(hot_text(runtime, "static const unsigned long long k_pow10_small[8] = {", "};\n"));
    for start in [
        "static inline __attribute__((always_inline)) int k_span_in(long long from, long long to,",
        "static inline long long k_digits8_value(uint64_t w, long long n) {",
        "static inline int k_digits_back8(const char* end, long long n, long long* out) {",
        "static int k_el_parse(unsigned long long w, long long q, double* out) {",
        "static inline int k_float_shape8(const char* data, long long len, long long room,",
    ] {
        out.push_str(hot_line_text(runtime, start, "\n}\n"));
    }
    out.push_str(hot_text(
        runtime,
        "static KValue k_to_int_text(const char* data, long long len, long long room,\n                            const char* origin) {",
        "\n}\n",
    ));
    out.push_str(hot_text(
        runtime,
        "static inline __attribute__((always_inline)) KValue\nk_to_float_text(const char* data, long long len, long long room, const char* origin) {",
        "\n}\n",
    ));
    // The empty list and map, which the decoder makes at every array and
    // object it opens: the headers, the seed sizes and the two layouts.
    for (start, end) in [
        ("typedef struct { long long capw; long long used; } KBuf;", "\n"),
        ("typedef struct { long long len; KValue* items; } KList;", "\n"),
        ("typedef struct {\n    long long len;        /* raw appended", "} KMap;\n"),
        ("static long long k_ptr(void* p) {", "\n"),
        ("#define K_LIST_SEED 6", "~(size_t)15))\n"),
        ("#define K_MAP_SEED 10", "~(size_t)15))\n"),
    ] {
        out.push_str(hot_text(runtime, start, end));
    }
    for start in [
        "static inline void k_buf_set_cap(KBuf* b, long long cap, int malloced) {",
        "static inline __attribute__((always_inline)) KValue k_list_seed(unsigned char* whole) {",
        "static inline __attribute__((always_inline)) KValue k_map_seed(unsigned char* whole) {",
        "KValue k_list_empty(void) {",
        "KValue k_map_empty(void) {",
    ] {
        out.push_str(inline_only(start));
        out.push_str(hot_line_text(runtime, start, "\n}\n"));
    }
    for start in [
        "KValue k_b_to_int_slice_raw(const unsigned char* d, long long len, long long from, long long to, const char* origin) {",
        "KValue k_b_to_float_slice_raw(const unsigned char* d, long long len, long long from, long long to, const char* origin) {",
    ] {
        out.push_str(inline_only(start));
        out.push_str(hot_line_text(runtime, start, "\n}\n"));
    }
    // Clang emits an inline definition only where the unit refers to it, and
    // nothing here calls these, so a table refers to each. The table has
    // external linkage, so the unit's own compile keeps it, and nothing reads
    // it, so the link drops it and then every body the program does not call.
    // With `used` on it the link kept and optimized all four in every build.
    out.push_str("const void* const k_hot_inline_bodies[] = {");
    for name in INLINE_ONLY {
        out.push_str("(const void*)");
        out.push_str(name.trim_end_matches('('));
        out.push_str(", ");
    }
    out.push_str("};\n");
    out
}

const INLINE_ONLY: [&str; 10] = [
    "k_beat_iter(",
    "k_beat_push(",
    "k_b_find2_raw(",
    "k_b_find2_below_raw(",
    "k_b_at_str(",
    "k_b_number_span_raw(",
    "k_b_to_int_slice_raw(",
    "k_b_to_float_slice_raw(",
    "k_list_empty(",
    "k_map_empty(",
];

/// `inline ` for the definitions runtime.c also compiles as machine code, so
/// the unit's copy is a C99 inline definition: the link inlines it where the
/// program calls it and drops it everywhere else. Defined outright in this
/// unit, each was optimized and compiled again by every release link, called
/// or not. Linking the codegen corpus, `ld.lld` read 252,430,133 with four
/// helpers defined outright, 295,287,065 with eight, and 187,756,033 with the
/// eight inline.
fn inline_only(start: &str) -> &'static str {
    if INLINE_ONLY.iter().any(|n| start.contains(n)) {
        "inline "
    } else {
        ""
    }
}

/// `hot_text` from the start of the line holding `start`.
fn hot_line_text<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let at = source.find(start).unwrap_or_else(|| panic!("runtime.c no longer holds `{start}`"));
    let line = source[..at].rfind('\n').map_or(0, |n| n + 1);
    hot_text(source, &source[line..at + start.len()], end)
}

/// From `start` through the first `end` after it, inclusive. A function ends
/// at the first closing brace in column zero, which is how every definition in
/// `runtime.c` is written.
fn hot_text<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let from = source.find(start).unwrap_or_else(|| panic!("runtime.c no longer holds `{start}`"));
    let to = from
        + source[from..].find(end).unwrap_or_else(|| panic!("`{start}` has no end in runtime.c"))
        + end.len();
    &source[from..to]
}

/// The object clang makes of `source`, from the temp directory when a build
/// already made it. The source is asked for only when it did not: the hot
/// unit's text is cut out of the runtime by some forty searches. Building it
/// on every release build of the codegen corpus, kanso's own process read
/// 74,931,992 instructions, and asking for it only on a miss, 30,665,523.
fn cached_object(
    profile: &str,
    recipe: Option<(u64, u64)>,
    source: &dyn Fn() -> std::borrow::Cow<'static, str>,
    opt: &[&str],
) -> std::io::Result<std::path::PathBuf> {
    use std::hash::{Hash, Hasher};
    let preserve = closure_convention() == kanso::codegen::ClosureConvention::PreserveNone;
    let counting = kanso::codegen::counters_wanted();
    let mut named = runtime_key(profile, opt, preserve, counting);
    if let Some(recipe) = recipe {
        let mut hasher = std::hash::DefaultHasher::new();
        (named, recipe).hash(&mut hasher);
        named = hasher.finish();
    }
    let key = hex16(named);
    let object = std::env::temp_dir().join(format!("kanso_runtime_{profile}_{key}.o"));
    if object.exists() {
        return Ok(object);
    }
    if let Some(bytes) = prebuilt(profile, recipe, opt, preserve, counting) {
        let staging =
            std::env::temp_dir().join(format!("kanso_runtime_{profile}_{key}_{}.o", pid_tag()));
        std::fs::write(&staging, bytes)?;
        std::fs::rename(&staging, &object)?;
        return Ok(object);
    }
    // THE SOURCE GOES TO CLANG ON ITS STANDARD INPUT. It used to be written to
    // `kanso_runtime_<profile>_<key>.c` and compiled from there, and an
    // object records the name of the file it was compiled from: as its file
    // symbol, and in bitcode as the module's name, which the LTO link reads.
    // The key is a digest, so the hot unit's bitcode carried a digest of
    // this file's bytes, and an edit here that emitted nothing new moved the
    // linker's instruction count by 64 after the link was already handed a
    // plain name (`linked_as`). Read from standard input the object records
    // no name a digest decides, and the same text makes the same object.
    let staging =
        std::env::temp_dir().join(format!("kanso_runtime_{profile}_{key}_{}.o", pid_tag()));
    // `-Werror=unknown-attributes` is the belt to the probe's braces: clang 18
    // only WARNS about `preserve_none` and would silently compile the runtime
    // on the C convention while the emitted IR used the other one.
    let convention: &[&str] = match preserve {
        true => &["-DKANSO_PRESERVE_NONE", "-Werror=unknown-attributes"],
        false => &[],
    };
    let counters: &[&str] = match counting {
        true => &["-DKANSO_COUNTERS_BUILD"],
        false => &[],
    };
    let mut child = std::process::Command::new("clang")
        .args(opt)
        .args(convention)
        .args(counters)
        .args(if cfg!(target_arch = "x86_64") { &["-mssse3"][..] } else { &[][..] })
        .args(["-x", "c", "-c", "-", "-o"])
        .arg(&staging)
        .stdin(std::process::Stdio::piped())
        .spawn()?;
    {
        use std::io::Write;
        let mut text =
            child.stdin.take().ok_or_else(|| std::io::Error::other("clang took no input"))?;
        text.write_all(source().as_bytes())?;
    }
    let status = child.wait()?;
    if !status.success() {
        return Err(std::io::Error::other("clang failed on the runtime"));
    }
    std::fs::rename(&staging, &object)?;
    Ok(object)
}

include!(concat!(env!("OUT_DIR"), "/runtime_dev_opt.rs"));

/// The dev runtime object build.rs compiled, when it is the one asked for.
///
/// It is the object `cached_object` would compile for the dev tier with no
/// counters, on the convention build.rs's clang took, so it is handed out under
/// the same key; any other recipe, or a probe that answered differently here,
/// compiles as before. Empty when build.rs had no clang.
fn prebuilt(
    profile: &str,
    recipe: Option<(u64, u64)>,
    opt: &[&str],
    preserve: bool,
    counting: bool,
) -> Option<&'static [u8]> {
    const OBJECT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/runtime_dev.o"));
    const PRESERVE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/runtime_dev_preserve"));
    let wanted = profile == "dev" && recipe.is_none() && opt == DEV_RUNTIME_OPT && !counting;
    (wanted && !OBJECT.is_empty() && preserve == (PRESERVE == b"1")).then_some(OBJECT)
}

#[cfg(test)]
mod a_cached_runtime_is_named_by_what_built_it {
    use super::runtime_key;

    /// A runtime object built with one set of flags is not the object another
    /// set asks for. The release profile went from bitcode to machine code by
    /// changing its flags alone, and with the flags outside the key every
    /// machine that had built the bitcode object kept linking it.
    #[test]
    fn the_runtime_key_names_the_flags() {
        assert_ne!(
            runtime_key("release", &["-O3", "-flto"], false, false),
            runtime_key("release", &["-O3"], false, false),
            "two flag sets share a cached runtime object"
        );
        assert_eq!(
            runtime_key("release", &["-O3"], false, false),
            runtime_key("release", &["-O3"], false, false),
        );
    }
}

#[cfg(test)]
mod the_hot_unit_is_taken_from_the_runtime {
    use super::hot_source;

    /// Every piece the release build's bitcode unit is assembled from is
    /// still where the extraction looks for it. A signature edited in
    /// `runtime.c` without the list in `hot_source` panics here rather than
    /// in a user's release build.
    #[test]
    fn every_hot_definition_is_found() {
        let unit = hot_source();
        for name in [
            "k_tail_window",
            "k_beat_rewind",
            "k_beat_iter",
            "k_beat_push",
            "k_b_find2_raw",
            "k_b_find2_below_raw",
            "k_b_at_str",
            "k_b_number_span_raw",
            "k_b_to_int_slice_raw",
            "k_b_to_float_slice_raw",
            "k_list_empty",
            "k_map_empty",
        ] {
            assert!(unit.contains(&format!("{name}(")), "the hot unit lacks {name}");
        }
    }

    /// The ratchet row for the two scanners puts `noinline` where their
    /// `always_inline` was, and the release build must still find them: a
    /// lift keyed on the attribute stopped the mutated tree building, and the
    /// row read UNBUILT on kanso#1585. Watched red with the scanners' start
    /// strings keyed on the attribute again.
    #[test]
    fn the_scanners_are_found_without_their_attribute() {
        let bare = kanso::RUNTIME_C.replace(
            "__attribute__((always_inline)) long long k_b_find2",
            "__attribute__((noinline)) long long k_b_find2",
        );
        assert_ne!(bare, kanso::RUNTIME_C, "the attribute is no longer there to replace");
        let unit = super::hot_source_of(&bare);
        for name in ["k_b_find2_raw", "k_b_find2_below_raw"] {
            assert!(
                unit.contains(&format!("\ninline __attribute__((noinline)) long long {name}(")),
                "the mutated {name} was not lifted as an inline copy"
            );
        }
    }
}
