use crate::ast::{Expr, FnDecl, Pattern, Program, Stmt};
use crate::hash::{Map as HashMap, Set as HashSet};

/// The door principle, advised: a pub fn that returns a foreign type owes
/// its callers an operation that accepts it — re-exported or wrapped. The
/// analysis under-approximates (only constructions and calls it can trace),
/// so every advisory is a real handle with no door.
pub fn door_advisories(program: &Program) -> Vec<String> {
    // bare-enrollment clones are dispatch conveniences, not surface facts —
    // the door analysis reasons about the real declarations only
    let type_names: HashSet<&str> =
        program.types.iter().filter(|t| !t.synthetic).map(|t| t.name.as_str()).collect();
    let mut groups: HashMap<&str, Vec<usize>> = HashMap::default();
    for (i, decl) in program.fns.iter().enumerate() {
        if decl.synthetic {
            continue;
        }
        groups.entry(decl.name.as_str()).or_default().push(i);
    }
    let returns = return_type_names(program, &type_names, &groups);
    let pub_names: HashSet<&str> =
        program.fns.iter().filter(|d| d.is_pub && !d.synthetic).map(|d| d.name.as_str()).collect();
    let accepted = accepted_types(program, &pub_names, &groups);
    let mut advisories = Vec::new();
    let mut seen = HashSet::default();
    for (i, decl) in program.fns.iter().enumerate() {
        if !decl.is_pub || decl.name.contains('/') || decl.synthetic {
            continue;
        }
        for ty in &returns[i] {
            if !ty.contains('/') || accepted.contains(ty) {
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
    groups: &HashMap<&'a str, Vec<usize>>,
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
    groups: &HashMap<&'a str, Vec<usize>>,
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
    groups: &HashMap<&'a str, Vec<usize>>,
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
    groups: &HashMap<&'a str, Vec<usize>>,
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
    for &i in groups.get(name).into_iter().flatten() {
        set.extend(returns[i].iter().copied());
    }
    set
}

/// Types some pub operation accepts: a param naming the type in any decl of
/// a pub group, either the module's own or a foreign fn it forwards to.
fn accepted_types<'a>(
    program: &'a Program,
    pub_names: &HashSet<&str>,
    groups: &HashMap<&'a str, Vec<usize>>,
) -> HashSet<&'a str> {
    let mut surface_groups: HashSet<&str> =
        pub_names.iter().copied().filter(|n| !n.contains('/')).collect();
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
            if let Some(name) = target.filter(|n| n.contains('/')) {
                surface_groups.insert(name);
            }
        }
    }
    let mut accepted = HashSet::default();
    for (name, indices) in groups {
        if !surface_groups.contains(name) {
            continue;
        }
        for &i in indices {
            for pattern in &program.fns[i].params {
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
