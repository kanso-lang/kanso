use crate::ast::{Expr, FnDecl, Pattern, Program, Stmt};
use crate::hash::{Map as HashMap, Set as HashSet};

/// The door principle, advised: a pub fn that returns a foreign type owes
/// its callers an operation that accepts it — re-exported or wrapped. The
/// analysis under-approximates (only constructions and calls it can trace),
/// so every advisory is a real handle with no door.
/// The declaration indices of each group, as one flat vector and a range
/// apiece. A `Vec<usize>` per name was an allocation for a list of two or
/// three numbers, and this table is only ever read.
struct Groups<'a> {
    ranges: HashMap<&'a str, (u32, u32)>,
    flat: Vec<u32>,
}

impl<'a> Groups<'a> {
    /// Count the arms per name, turn the counts into starts, then place each
    /// declaration at its group's cursor. Synthetic declarations are skipped
    /// here as they were before, so the counts are an upper bound and a group
    /// whose synthetic arms were dropped leaves cells nothing reads.
    fn of(program: &'a Program) -> Groups<'a> {
        let mut ranges: HashMap<&str, (u32, u32)> =
            HashMap::with_capacity_and_hasher(program.fns.len(), Default::default());
        for decl in &program.fns {
            if decl.synthetic {
                continue;
            }
            ranges.entry(decl.name.as_str()).or_insert((0, 0)).1 += 1;
        }
        let mut at = 0;
        for slot in ranges.values_mut() {
            let count = slot.1;
            *slot = (at, at);
            at += count;
        }
        let mut flat: Vec<u32> = vec![0; at as usize];
        for (i, decl) in program.fns.iter().enumerate() {
            if decl.synthetic {
                continue;
            }
            let slot = ranges.get_mut(decl.name.as_str()).expect("every arm was counted");
            flat[slot.1 as usize] = i as u32;
            slot.1 += 1;
        }
        Groups { ranges, flat }
    }

    fn get(&self, name: &str) -> &[u32] {
        match self.ranges.get(name) {
            Some(&(start, end)) => &self.flat[start as usize..end as usize],
            None => &[],
        }
    }

    fn names(&self) -> impl Iterator<Item = (&'a str, &[u32])> + '_ {
        self.ranges
            .iter()
            .map(|(name, &(start, end))| (*name, &self.flat[start as usize..end as usize]))
    }
}

pub fn door_advisories(program: &Program) -> Vec<String> {
    // bare-enrollment clones are dispatch conveniences, not surface facts —
    // the door analysis reasons about the real declarations only
    let type_names: HashSet<&str> =
        program.types.iter().filter(|t| !t.synthetic).map(|t| t.name.as_str()).collect();
    let groups = Groups::of(program);
    let returns = return_type_names(program, &type_names, &groups);
    let pub_names: HashSet<&str> =
        program.fns.iter().filter(|d| d.is_pub && !d.synthetic).map(|d| d.name.as_str()).collect();
    let accepted = accepted_types(program, &pub_names, &groups);
    let mut advisories = Vec::new();
    let mut seen = HashSet::default();
    for (i, decl) in program.fns.iter().enumerate() {
        if !decl.is_pub || crate::ast::has_slash(&decl.name) || decl.synthetic {
            continue;
        }
        for ty in &returns[i] {
            if !crate::ast::has_slash(ty) || accepted.contains(ty) {
                continue;
            }
            if seen.insert((decl.name.as_str(), *ty)) {
                advisories.push(format!(
                    "advisory[door]: `{}` returns `{ty}` and the surface offers \
                     nothing that accepts it — re-export what callers need, or \
                     wrap it",
                    decl.name
                ));
            }
        }
    }
    advisories.sort();
    advisories
}

/// Fixpoint: the record type names each fn's return value can carry, traced
/// through constructions, local bindings, calls, and `if` arms.
fn return_type_names<'a>(
    program: &'a Program,
    type_names: &HashSet<&str>,
    groups: &Groups<'a>,
) -> Vec<HashSet<&'a str>> {
    let mut returns: Vec<HashSet<&'a str>> = vec![HashSet::default(); program.fns.len()];
    let mut changed = true;
    while changed {
        changed = false;
        for (i, decl) in program.fns.iter().enumerate() {
            let inferred = body_types(decl, program, type_names, groups, &returns);
            if !inferred.is_subset(&returns[i]) {
                returns[i].extend(inferred);
                changed = true;
            }
        }
    }
    returns
}

fn body_types<'a>(
    decl: &'a FnDecl,
    program: &Program,
    type_names: &HashSet<&str>,
    groups: &Groups<'a>,
    returns: &[HashSet<&'a str>],
) -> HashSet<&'a str> {
    let mut env: HashMap<&str, HashSet<&'a str>> = HashMap::default();
    let mut tail = HashSet::default();
    for (i, stmt) in decl.body.iter().enumerate() {
        match stmt {
            Stmt::Bind { pattern, expr } => {
                let set = expr_types(expr, type_names, groups, returns, &env);
                if let Pattern::Var(name, _) = pattern {
                    env.insert(name, set);
                }
            }
            Stmt::Set { .. } => {}
            Stmt::Expr(e) if i == decl.body.len() - 1 => {
                tail = expr_types(e, type_names, groups, returns, &env);
            }
            Stmt::Expr(_) => {}
        }
    }
    let _ = program;
    tail
}

fn expr_types<'a>(
    e: &'a Expr,
    type_names: &HashSet<&str>,
    groups: &Groups<'a>,
    returns: &[HashSet<&'a str>],
    env: &HashMap<&str, HashSet<&'a str>>,
) -> HashSet<&'a str> {
    match e {
        Expr::Ident(name, _) => name_types(name, type_names, groups, returns, env),
        Expr::App { head, args, .. } => {
            if let Expr::Ident(name, _) = head.as_ref() {
                if name == "if" && args.len() == 3 {
                    let mut set = expr_types(&args[1], type_names, groups, returns, env);
                    set.extend(expr_types(&args[2], type_names, groups, returns, env));
                    return set;
                }
                // an err carries its payload; the payload's type is what leaks
                if name == "err" && args.len() == 1 {
                    return expr_types(&args[0], type_names, groups, returns, env);
                }
                return name_types(name, type_names, groups, returns, env);
            }
            HashSet::default()
        }
        Expr::Seq(_, b, _) => expr_types(b, type_names, groups, returns, env),
        _ => HashSet::default(),
    }
}

fn name_types<'a>(
    name: &'a str,
    type_names: &HashSet<&str>,
    groups: &Groups<'a>,
    returns: &[HashSet<&'a str>],
    env: &HashMap<&str, HashSet<&'a str>>,
) -> HashSet<&'a str> {
    // Every name this answers with is a type the program declares, so the set
    // holds borrows of the program's own strings. It used to hold copies:
    // 2,706 allocations on lib/json, for names that were already in memory.
    if type_names.contains(name) {
        return HashSet::from_iter([name]);
    }
    if let Some(local) = env.get(name) {
        return local.clone();
    }
    let mut set = HashSet::default();
    for &i in groups.get(name) {
        set.extend(returns[i as usize].iter().copied());
    }
    set
}

/// Types some pub operation accepts: a param naming the type in any decl of
/// a pub group, either the module's own or a foreign fn it forwards to.
fn accepted_types<'a>(
    program: &'a Program,
    pub_names: &HashSet<&str>,
    groups: &Groups<'a>,
) -> HashSet<&'a str> {
    let mut surface_groups: HashSet<&str> =
        pub_names.iter().copied().filter(|n| !crate::ast::has_slash(n)).collect();
    for decl in &program.fns {
        if !surface_groups.contains(decl.name.as_str()) {
            continue;
        }
        if let Some(Stmt::Expr(tail)) = decl.body.last() {
            let target = match tail {
                Expr::Ident(name, _) => Some(name.as_str()),
                Expr::App { head, .. } => match head.as_ref() {
                    Expr::Ident(name, _) => Some(name.as_str()),
                    _ => None,
                },
                _ => None,
            };
            if let Some(name) = target.filter(|n| crate::ast::has_slash(n)) {
                surface_groups.insert(name);
            }
        }
    }
    let mut accepted = HashSet::default();
    for (name, indices) in groups.names() {
        if !surface_groups.contains(name) {
            continue;
        }
        for &i in indices {
            for pattern in &program.fns[i as usize].params {
                pattern_type_names(pattern, &mut accepted);
            }
        }
    }
    accepted
}

fn pattern_type_names<'a>(pattern: &'a Pattern, out: &mut HashSet<&'a str>) {
    match pattern {
        Pattern::Ctor { ty, fields, .. } => {
            out.insert(ty.as_str());
            for f in fields {
                pattern_type_names(f, out);
            }
        }
        Pattern::Annotated { ty, .. } => {
            out.insert(ty.as_str());
        }
        _ => {}
    }
}
