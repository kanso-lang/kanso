// The IR rewrites that pick a calling convention for the emitted module's
// tail calls and closure calls, and their tests. main.rs reads this file
// with include!, so the items stay in the binary's root module and the
// compiler builds to the same instructions it did when they sat in main.rs
// (kanso#1848). cargo fmt does not reach an included file, so CI's format
// job checks the files under src/'s subdirectories itself.

/// The widest arm, in argument words, that keeps `tailcc` in a release build.
///
/// On arm64 it is the register file, x0 through x7, past which the
/// convention is miscompiled (see `narrow_tailcc`). x86-64 lowers a `tailcc`
/// call whose arguments spill onto the stack correctly, so there the limit is
/// a cost and was measured. At eight it narrowed the JSON decoder's
/// `obj_key_end`, nine words, and a release build decoding an object of
/// 300,000 keys spent a frame per key and overflowed its stack. At nine the
/// decoder keeps its jumps and runbench reads 0.25% less. At ten and above the
/// wider std/regexp arms keep theirs too, and scanbench reads 1.9% more: a
/// tail call copies its stack arguments into the caller's, and past three of
/// them that costs more than the frame it saves.
const TAILCC_WIDEST: usize = match cfg!(target_arch = "aarch64") {
    true => 8,
    false => 9,
};

/// How many argument registers a parameter list wants: a %KValue and a %parsed
/// are each two i64s, everything else is one.
fn arg_registers(params: &str) -> usize {
    params
        .split(", ")
        .filter(|p| !p.trim().is_empty())
        .map(|p| match p.starts_with("%KValue") || p.starts_with("%parsed") {
            true => 2,
            false => 1,
        })
        .sum()
}

fn defined_symbol(line: &str) -> Option<(String, usize)> {
    let open = line.find('(')?;
    let close = line.rfind(')')?;
    let name = line[..open].rsplit('@').next()?.trim().trim_matches('"').to_string();
    Some((name, arg_registers(&line[open + 1..close])))
}

fn called_symbol(line: &str) -> Option<String> {
    let at = line.find(" @")?;
    let rest = &line[at + 2..];
    let end = rest.find('(')?;
    Some(rest[..end].trim().trim_matches('"').to_string())
}

/// tailcc kept on every arm no wider than `widest` argument words:
/// `TAILCC_WIDEST`, or twelve where `preserve_none_tails` follows, since that
/// convention passes twelve words in registers and an arm that fits keeps its
/// jump. On scanbench the twelve reads 1.52% less than the nine.
///
/// `tailcc` is what the beat machinery's `musttail` needs, and at -O1 and above
/// on arm64 the convention is miscompiled for an arm whose arguments spill past
/// x7 — the binary jumps to an address that was a value. The boundary is exactly
/// the register file: the micro corpus is clean at eight and one sample
/// segfaults at nine, growing to ten samples by twelve. So a wide arm keeps the
/// C convention and every call into it becomes ordinary, and everything narrow
/// enough keeps the guarantee the optimized build used to give up.
///
/// What a wide arm gives up is the jump: it spends a frame per hop, and a deep
/// recursion through one overflows the stack, which is loud.
fn narrow_tailcc(ir: String, widest: usize) -> String {
    let wide: std::collections::HashSet<String> = ir
        .lines()
        .filter(|l| l.starts_with("define tailcc ") || l.starts_with("declare tailcc "))
        .filter_map(defined_symbol)
        .filter(|(_, regs)| *regs > widest)
        .map(|(name, _)| name)
        .collect();
    let mut here = String::new();
    let mut out = String::new();
    for line in ir.lines() {
        let mut line = line.to_string();
        if line.starts_with("define ") || line.starts_with("declare ") {
            here = defined_symbol(&line).map(|(n, _)| n).unwrap_or_default();
            if wide.contains(&here) {
                line = line.replacen("tailcc ", "", 1);
            }
        } else if line.contains(" call ") {
            let callee = called_symbol(&line).unwrap_or_default();
            if wide.contains(&here) {
                line = line.replace("musttail ", "");
            }
            if wide.contains(&callee) {
                line = line.replace("musttail ", "").replace("tailcc ", "");
            }
        }
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// How many argument registers `preserve_nonecc` has on x86-64: r12 to r15,
/// rdi, rsi, rdx, rcx, r8, r9, r11 and rax.
const PRESERVE_NONE_REGISTERS: usize = 12;

/// The words a parameter of this type takes in a flattened signature, or
/// `None` for a type the rewrite does not carry.
fn flat_words(ty: &str) -> Option<usize> {
    match ty {
        "%KValue" | "%parsed" => Some(2),
        "i64" | "ptr" | "double" => Some(1),
        _ => None,
    }
}

/// A parameter or argument split into its type and the rest: `%KValue %x0`,
/// `i64 7`, `%KValue { i64 2, i64 0 }`.
fn typed(item: &str) -> Option<(&str, &str)> {
    let item = item.trim();
    let space = item.find(' ')?;
    let (ty, value) = (&item[..space], item[space + 1..].trim());
    flat_words(ty).map(|_| (ty, value))
}

/// Splits an argument list at the commas outside any brace, bracket or
/// parenthesis, so a constant `%KValue { i64 2, i64 0 }` stays one item.
fn top_level_items(list: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let (mut depth, mut start) = (0i32, 0);
    for (i, c) in list.char_indices() {
        match c {
            '{' | '[' | '(' | '<' => depth += 1,
            '}' | ']' | ')' | '>' => depth -= 1,
            ',' if depth == 0 => {
                items.push(list[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    if !list[start..].trim().is_empty() {
        items.push(list[start..].trim());
    }
    items
}

/// The two words of a literal `{ i64 A, i64 B }`. An argument that was a raw
/// word before ints could be bignums is often a boxed literal now, `n 0` among
/// them, and reading its words with `extractvalue` is an instruction clang's
/// fast selector at -O0 does not lower.
fn literal_words(value: &str) -> Option<(&str, &str)> {
    value.strip_prefix("{ i64 ")?.strip_suffix(" }")?.split_once(", i64 ")
}

/// The index just past the parenthesis that closes the one before `open`.
fn closing(line: &str, open: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (i, c) in line[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i + 1);
                }
            }
            _ => {}
        }
    }
    None
}

/// The symbol an `@` at `at` names, quotes removed, and where it ends.
fn symbol_at(line: &str, at: usize) -> (String, usize) {
    let rest = &line[at + 1..];
    match rest.strip_prefix('"') {
        Some(quoted) => {
            let end = quoted.find('"').unwrap_or(quoted.len());
            (quoted[..end].to_string(), at + 1 + end + 2)
        }
        None => {
            let end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || "_.$/-".contains(c)))
                .unwrap_or(rest.len());
            (rest[..end].to_string(), at + 1 + end)
        }
    }
}

/// A `define tailcc` header: the return type, the symbol, the parameters and
/// whatever follows the closing parenthesis.
struct TailDefine {
    ret: String,
    params: Vec<(String, String)>,
    words: usize,
}

fn tail_define(line: &str) -> Option<(String, TailDefine)> {
    let rest = line.strip_prefix("define tailcc ")?;
    let at = line.len() - rest.len() + rest.find(" @")? + 1;
    let ret = line["define tailcc ".len()..at].trim().to_string();
    let (name, after) = symbol_at(line, at);
    let close = closing(line, after)?;
    let mut params = Vec::new();
    let mut words = 0;
    for item in top_level_items(&line[after + 1..close - 1]) {
        let (ty, value) = typed(item)?;
        if !value.starts_with('%') || value.contains(' ') {
            return None;
        }
        words += flat_words(ty)?;
        params.push((ty.to_string(), value.to_string()));
    }
    Some((name, TailDefine { ret, params, words }))
}

/// Where a line calls a `tailcc` function: the offset of the convention
/// keyword, of the callee's `@`, and the symbol.
fn tail_call(line: &str) -> Option<(usize, usize, String)> {
    let keyword = line.find("call tailcc ")? + "call ".len();
    let at = keyword + line[keyword..].find(" @")? + 1;
    Some((keyword, at, symbol_at(line, at).0))
}

/// Every `tailcc` function joined to the others it `musttail`s into or is
/// `musttail`ed from takes `preserve_nonecc` and one flat signature.
///
/// The JSON decoder is a cycle of tail calls, `parse_value` into
/// `string_scan` into `obj_key_start` into `obj_delim` and round again, and
/// under `tailcc` every arm saved the callee-saved registers it used on entry
/// and restored them before each jump out. shrink-wrapping could not move the
/// saves, because an arm has several exits and each `musttail` is one. On
/// runbench those pushes and pops were 55,000,000 instructions in the four
/// arms named. `preserve_nonecc` has no callee-saved registers, so an arm
/// that jumps to the next owes nothing to restore.
///
/// A `musttail` call may cross an arity or a type only under `tailcc`. Under
/// any other convention the caller and callee prototypes must match, so each
/// set of functions a `musttail` joins takes one signature: every parameter
/// flattened to `i64` words, padded with `poison` to the widest member. The
/// widest decoder arm is nine words and the convention passes twelve in
/// registers, so nothing goes on the stack. A set that is wider than that, or
/// holds a parameter of a type not flattened here, or a member whose address
/// is taken, keeps `tailcc`.
fn preserve_none_tails(ir: String) -> String {
    let defines: std::collections::HashMap<String, TailDefine> =
        ir.lines().filter_map(tail_define).collect();
    let names: Vec<&String> = defines.keys().collect();
    let index: std::collections::HashMap<&str, usize> =
        names.iter().enumerate().map(|(i, n)| (n.as_str(), i)).collect();
    let mut parent: Vec<usize> = (0..names.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    // A set is spoiled by any use of a member that is not its header or a
    // call, and by a `define tailcc` this does not parse.
    let mut spoiled = vec![false; names.len()];
    let mut header_spoils = false;
    let mut here: Option<usize> = None;
    for line in ir.lines() {
        if line.starts_with("define ") || line.starts_with("declare ") {
            here = line
                .find(" @")
                .map(|at| symbol_at(line, at + 1).0)
                .and_then(|n| index.get(n.as_str()).copied());
            if line.starts_with("define tailcc ") && here.is_none() {
                header_spoils = true;
            }
            if line.starts_with("declare tailcc ") {
                header_spoils = true;
            }
        }
        let call = tail_call(line);
        if let (Some((_, _, callee)), Some(from)) = (&call, here) {
            if line.contains("musttail call tailcc ") {
                match index.get(callee.as_str()) {
                    Some(&to) => {
                        let (a, b) = (root(&mut parent, from), root(&mut parent, to));
                        parent[a] = b;
                    }
                    None => spoiled[from] = true,
                }
            }
        }
        let header_at = match line.starts_with("define ") {
            true => line.find(" @").map(|at| at + 1),
            false => None,
        };
        let call_at = call.as_ref().map(|(_, at, _)| *at);
        for (at, _) in line.match_indices('@') {
            if Some(at) == header_at || Some(at) == call_at {
                continue;
            }
            if let Some(&i) = index.get(symbol_at(line, at).0.as_str()) {
                spoiled[i] = true;
            }
        }
    }
    if header_spoils || names.is_empty() {
        return ir;
    }
    // Each set's width, its return type, and whether it may be rewritten.
    let mut width = vec![0usize; names.len()];
    let mut ret: Vec<Option<&str>> = vec![None; names.len()];
    let mut ok = vec![true; names.len()];
    for (i, name) in names.iter().enumerate() {
        let r = root(&mut parent, i);
        let define = &defines[name.as_str()];
        width[r] = width[r].max(define.words);
        ok[r] &= !spoiled[i];
        match ret[r] {
            None => ret[r] = Some(&define.ret),
            Some(seen) => ok[r] &= seen == define.ret,
        }
    }
    let flat: std::collections::HashMap<&str, usize> = names
        .iter()
        .enumerate()
        .filter_map(|(i, n)| {
            let r = root(&mut parent, i);
            (ok[r] && width[r] <= PRESERVE_NONE_REGISTERS).then_some((n.as_str(), width[r]))
        })
        .collect();
    if flat.is_empty() {
        return ir;
    }
    let mut out = String::with_capacity(ir.len() + ir.len() / 8);
    let mut fresh = 0usize;
    let mut unpack: Option<String> = None;
    for line in ir.lines() {
        if let Some(lines) = unpack.take() {
            // after the entry label when there is one
            match line.trim_end().ends_with(':') {
                true => {
                    out.push_str(line);
                    out.push('\n');
                    out.push_str(&lines);
                    continue;
                }
                false => out.push_str(&lines),
            }
        }
        if let Some((name, define)) =
            tail_define(line).filter(|(n, _)| flat.contains_key(n.as_str()))
        {
            let words = flat[name.as_str()];
            let mut signature = Vec::with_capacity(words);
            let mut lines = String::new();
            for (ty, value) in &define.params {
                match ty.as_str() {
                    "i64" => signature.push(format!("i64 {value}")),
                    "ptr" => {
                        signature.push(format!("i64 {value}.w"));
                        lines.push_str(&format!("  {value} = inttoptr i64 {value}.w to ptr\n"));
                    }
                    "double" => {
                        signature.push(format!("i64 {value}.w"));
                        lines.push_str(&format!("  {value} = bitcast i64 {value}.w to double\n"));
                    }
                    _ => {
                        signature.push(format!("i64 {value}.w0"));
                        signature.push(format!("i64 {value}.w1"));
                        lines.push_str(&format!(
                            "  {value}.half = insertvalue {ty} poison, i64 {value}.w0, 0\n  \
                             {value} = insertvalue {ty} {value}.half, i64 {value}.w1, 1\n"
                        ));
                    }
                }
            }
            for pad in signature.len()..words {
                signature.push(format!("i64 %pad{pad}"));
            }
            let at = line.find(" @").unwrap() + 1;
            let (_, after) = symbol_at(line, at);
            let close = closing(line, after).unwrap();
            out.push_str("define preserve_nonecc ");
            out.push_str(&line["define tailcc ".len()..after]);
            out.push('(');
            out.push_str(&signature.join(", "));
            out.push(')');
            out.push_str(&line[close..]);
            out.push('\n');
            unpack = Some(lines);
            continue;
        }
        if let Some((keyword, at, callee)) = tail_call(line) {
            if let Some(&words) = flat.get(callee.as_str()) {
                let (_, after) = symbol_at(line, at);
                let close = closing(line, after).unwrap();
                let indent = &line[..line.len() - line.trim_start().len()];
                let mut args = Vec::with_capacity(words);
                for item in top_level_items(&line[after + 1..close - 1]) {
                    let (ty, value) = typed(item).expect("an argument of a flattened callee");
                    match ty {
                        "i64" => args.push(format!("i64 {value}")),
                        "ptr" | "double" => {
                            fresh += 1;
                            let cast = if ty == "ptr" { "ptrtoint" } else { "bitcast" };
                            out.push_str(&format!(
                                "{indent}%pn{fresh} = {cast} {ty} {value} to i64\n"
                            ));
                            args.push(format!("i64 %pn{fresh}"));
                        }
                        _ => match literal_words(value) {
                            // a literal's words are written, not read off it
                            Some((a, b)) => {
                                args.push(format!("i64 {a}"));
                                args.push(format!("i64 {b}"));
                            }
                            None => {
                                for half in 0..2 {
                                    fresh += 1;
                                    out.push_str(&format!(
                                        "{indent}%pn{fresh} = extractvalue {ty} {value}, {half}\n"
                                    ));
                                    args.push(format!("i64 %pn{fresh}"));
                                }
                            }
                        },
                    }
                }
                while args.len() < words {
                    args.push("i64 poison".to_string());
                }
                out.push_str(&line[..keyword]);
                out.push_str("preserve_nonecc ");
                out.push_str(&line[keyword + "tailcc ".len()..after]);
                out.push('(');
                out.push_str(&args.join(", "));
                out.push(')');
                out.push_str(&line[close..]);
                out.push('\n');
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// The functions the runtime calls by name, which keep the C convention its
/// own calls use: every `extern` it declares.
fn runtime_externs(runtime: &str) -> std::collections::HashSet<String> {
    runtime
        .lines()
        .filter(|l| l.starts_with("extern ") && l.contains('('))
        .filter_map(|l| {
            let head = &l[..l.find('(')?];
            let name = head.rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).next()?;
            (!name.is_empty()).then(|| name.to_string())
        })
        .collect()
}

/// Every other function the program defines and calls only directly takes
/// `preserve_nonecc` too.
///
/// A function the program calls in the ordinary way saved the callee-saved
/// registers it used, whether or not its caller had anything live in them.
/// Under this convention the caller saves what it keeps live across the call
/// and the callee saves nothing. Nothing outside the module can call these:
/// what the runtime calls by name it declares `extern`, and those keep the C
/// convention, as does `main`, anything whose address is taken, and the
/// module's own copies of runtime helpers, whose names begin `k_`.
fn preserve_none_calls(ir: String, runtime: &str) -> String {
    let externs = runtime_externs(runtime);
    let plain = |line: &str| -> Option<String> {
        let rest = line.strip_prefix("define ")?;
        if rest.contains("preserve_nonecc") || rest.contains("tailcc ") {
            return None;
        }
        let at = line.find(" @")? + 1;
        let name = symbol_at(line, at).0;
        let kept = name == "main" || name.starts_with("k_") || externs.contains(&name);
        (!kept).then_some(name)
    };
    let mut chosen: std::collections::HashSet<String> = ir.lines().filter_map(plain).collect();
    if chosen.is_empty() {
        return ir;
    }
    // A direct call reaches its callee through `call <type> @name(`; any other
    // mention of a chosen name is its address, and it keeps its convention.
    let callee_at = |line: &str| -> Option<usize> {
        let call = line.find("call ")? + "call ".len();
        let at = call + line[call..].find(" @")? + 1;
        // one word between: the return type, and no convention before it
        (line[call..at].trim().split(' ').count() == 1).then_some(at)
    };
    let mut taken = Vec::new();
    for line in ir.lines() {
        let header = match line.starts_with("define ") {
            true => line.find(" @").map(|at| at + 1),
            false => None,
        };
        let call = callee_at(line);
        for (at, _) in line.match_indices('@') {
            if Some(at) == header || Some(at) == call {
                continue;
            }
            let name = symbol_at(line, at).0;
            if chosen.contains(&name) {
                taken.push(name);
            }
        }
    }
    for name in taken {
        chosen.remove(&name);
    }
    let mut out = String::with_capacity(ir.len() + ir.len() / 64);
    for line in ir.lines() {
        if plain(line).is_some_and(|n| chosen.contains(&n)) {
            // the convention follows the linkage, when there is one
            let rest = &line["define ".len()..];
            let linkage = ["internal ", "private "].into_iter().find(|l| rest.starts_with(l));
            let (linkage, rest) = match linkage {
                Some(l) => (l, &rest[l.len()..]),
                None => ("", rest),
            };
            out.push_str("define ");
            out.push_str(linkage);
            out.push_str("preserve_nonecc ");
            out.push_str(rest);
        } else if let Some(at) =
            callee_at(line).filter(|at| chosen.contains(&symbol_at(line, *at).0))
        {
            let call = line[..at].rfind("call ").unwrap() + "call ".len();
            out.push_str(&line[..call]);
            out.push_str("preserve_nonecc ");
            out.push_str(&line[call..]);
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod a_tail_cycle_takes_one_flat_signature {
    use super::preserve_none_tails;

    /// Two arms of a cycle with different arities, the shape of the JSON
    /// decoder's, and an entry that calls into it the ordinary way.
    const CYCLE: &str = "define tailcc %parsed @\"d/a_2\"(%KValue %x0, i64 %x1r) {\nentry:\n  %t1 = musttail call tailcc %parsed @\"d/b_3\"(%KValue %x0, i64 %x1r, %KValue { i64 2, i64 0 })\n  ret %parsed %t1\n}\ndefine tailcc %parsed @\"d/b_3\"(%KValue %x0, i64 %x1r, %KValue %x2) {\nentry:\n  %t1 = musttail call tailcc %parsed @\"d/a_2\"(%KValue %x2, i64 %x1r)\n  ret %parsed %t1\n}\ndefine %KValue @main_entry(%KValue %v) {\nentry:\n  %p = call tailcc %parsed @\"d/a_2\"(%KValue %v, i64 1)\n  ret %KValue %v\n}\n";

    #[test]
    fn every_member_takes_the_widest_signature() {
        let out = preserve_none_tails(CYCLE.to_string());
        assert!(!out.contains("tailcc"), "{out}");
        let headers: Vec<&str> =
            out.lines().filter(|l| l.starts_with("define preserve_nonecc")).collect();
        assert_eq!(
            headers,
            [
                "define preserve_nonecc %parsed @\"d/a_2\"(i64 %x0.w0, i64 %x0.w1, i64 %x1r, i64 %pad3, i64 %pad4) {",
                "define preserve_nonecc %parsed @\"d/b_3\"(i64 %x0.w0, i64 %x0.w1, i64 %x1r, i64 %x2.w0, i64 %x2.w1) {",
            ]
        );
        // the narrow arm's call is padded to the wide one's five words
        assert!(out.contains("musttail call preserve_nonecc %parsed @\"d/a_2\"(i64 %pn"), "{out}");
        assert!(out.contains(", i64 poison, i64 poison)"), "{out}");
        // and the entry, which is not in the cycle, calls in the same way
        assert!(out.contains("%p = call preserve_nonecc %parsed @\"d/a_2\"("), "{out}");
        // the parameters are put back together before the body reads them
        assert!(out.contains("entry:\n  %x0.half = insertvalue %KValue poison, i64 %x0.w0, 0\n  %x0 = insertvalue %KValue %x0.half, i64 %x0.w1, 1\n"), "{out}");
    }

    /// A member whose address is taken may be reached by an indirect call
    /// that still passes the old signature, so its whole cycle is left alone.
    #[test]
    fn an_address_taken_member_keeps_its_cycle_as_it_was() {
        let taken = format!("{CYCLE}@table = constant ptr @\"d/b_3\"\n");
        assert_eq!(preserve_none_tails(taken.clone()), taken);
    }

    /// A parameter of a type the rewrite does not flatten leaves the cycle
    /// alone rather than guessing at its words.
    #[test]
    fn an_unflattened_parameter_keeps_its_cycle_as_it_was() {
        let odd = CYCLE.replace("(%KValue %x0, i64 %x1r) {", "(%KValue %x0, i32 %x1r) {");
        assert_eq!(preserve_none_tails(odd.clone()), odd);
    }

    /// Wider than the convention's twelve registers, the arms would pass on
    /// the stack, and they keep `tailcc`.
    #[test]
    fn a_cycle_wider_than_the_registers_keeps_tailcc() {
        let wide = "define tailcc %KValue @w(%KValue %a, %KValue %b, %KValue %c, %KValue %d, %KValue %e, %KValue %f, i64 %g) {\nentry:\n  %r = musttail call tailcc %KValue @w(%KValue %a, %KValue %b, %KValue %c, %KValue %d, %KValue %e, %KValue %f, i64 %g)\n  ret %KValue %r\n}\n";
        assert_eq!(preserve_none_tails(wide.to_string()), wide);
    }
}

#[cfg(test)]
mod a_direct_call_saves_what_its_caller_keeps {
    use super::{preserve_none_calls, runtime_externs};

    const RUNTIME: &str = "extern KValue d_thunk_eval(long long site, KValue* args);\n\
                           extern const char* k_type_field_name(long long type_id, long long i);\n";

    const MODULE: &str = "define %KValue @\"d_m/f_1\"(%KValue %x0) {\nentry:\n  %r = call %KValue @\"d_m/g_1\"(%KValue %x0)\n  ret %KValue %r\n}\ndefine internal %KValue @\"d_m/g_1\"(%KValue %x0) {\nentry:\n  ret %KValue %x0\n}\ndefine %KValue @d_thunk_eval(i64 %site, ptr %args) {\nentry:\n  %r = call %KValue @\"d_m/f_1\"(%KValue zeroinitializer)\n  ret %KValue %r\n}\ndefine %KValue @k_user_main() {\nentry:\n  %r = call %KValue @\"d_m/f_1\"(%KValue zeroinitializer)\n  ret %KValue %r\n}\n";

    #[test]
    fn the_runtime_s_externs_are_read_from_its_source() {
        let names = runtime_externs(RUNTIME);
        assert!(names.contains("d_thunk_eval") && names.contains("k_type_field_name"), "{names:?}");
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn a_function_called_only_directly_takes_the_convention() {
        let out = preserve_none_calls(MODULE.to_string(), RUNTIME);
        assert!(out.contains("define preserve_nonecc %KValue @\"d_m/f_1\"("), "{out}");
        // after the linkage, which is where the parser wants it
        assert!(out.contains("define internal preserve_nonecc %KValue @\"d_m/g_1\"("), "{out}");
        assert!(out.contains("%r = call preserve_nonecc %KValue @\"d_m/g_1\"("), "{out}");
        assert_eq!(out.matches("call preserve_nonecc %KValue @\"d_m/f_1\"(").count(), 2, "{out}");
        // what the runtime calls by name keeps the C convention
        assert!(out.contains("define %KValue @d_thunk_eval("), "{out}");
        assert!(out.contains("define %KValue @k_user_main("), "{out}");
    }

    #[test]
    fn a_function_whose_address_is_taken_keeps_its_convention() {
        let taken = format!("{MODULE}@table = constant ptr @\"d_m/g_1\"\n");
        let out = preserve_none_calls(taken, RUNTIME);
        assert!(out.contains("define internal %KValue @\"d_m/g_1\"("), "{out}");
        assert!(out.contains("%r = call %KValue @\"d_m/g_1\"("), "{out}");
        assert!(out.contains("define preserve_nonecc %KValue @\"d_m/f_1\"("), "{out}");
    }
}
