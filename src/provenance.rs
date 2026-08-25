//! Which package's failure is this?
//!
//! The two-universe rule turns on where an err was *raised*, not on where
//! the reason type was declared and not on where the function that catches
//! it lives. Those coincide often enough to fool a checker: a package can
//! raise an err whose reason belongs to somebody else, and then rescue its
//! own failure through the borrowed name.
//!
//! So provenance is computed rather than guessed, and one hop at a time is
//! enough. An err can only reach a function through a pattern that matches
//! err — a bare parameter refuses failures — so every step of a failure's
//! travel is a call whose callee names it. Give each group the set of
//! packages whose errs it may hand back, take a fixpoint over the call
//! graph the way inference already does for value sets, and the rule reads
//! off it directly: a group that may receive an err raised in its own
//! package must return an err.

use crate::ast::{Expr, FnDecl, Pattern, Program, Stmt, TemplatePart};
use crate::hash::{Map as HashMap, Set as HashSet};

/// A declaration's identity for this pass: its name and arity. The name is
/// borrowed from the program, which outlives every `Provenance` built from
/// it. Owning it cost a `String` per group per fixpoint round, and the loop
/// below runs the whole program through up to two hundred of them.
type Group<'a> = (&'a str, usize);
/// Packages as a bitmask over an interned table — a program sees a handful,
/// and the fixpoint runs often enough that hashing strings dominated it.
type Pkgs = u32;

pub struct Provenance<'a> {
    /// per group, per parameter: packages whose errs may arrive there. The
    /// per-group return sets the fixpoint computes on the way are what feed
    /// these, and are not wanted afterwards.
    params: HashMap<Group<'a>, Vec<Pkgs>>,
    table: Vec<String>,
}

/// The package a declaration belongs to. The shipped library is one package
/// however many modules it spans; a fetched hako is its own; everything a
/// program compiles from its own tree is the program's.
pub fn package_of(file: &str) -> &str {
    if file.starts_with("std/") {
        return "std";
    }
    match file.split_once(".hako/") {
        Some((_, rest)) => {
            let mut parts = rest.split('/');
            match (parts.next(), parts.next()) {
                (Some(owner), Some(_)) => owner,
                _ => "",
            }
        }
        None => "",
    }
}

fn group_of(decl: &FnDecl) -> Group<'_> {
    (decl.name.as_str(), decl.params.len())
}

/// Does this pattern admit an err at all? Only an err-shaped pattern does:
/// a plain binder refuses failures, which is what makes one hop enough.
fn receives_err(pattern: &Pattern) -> bool {
    match pattern {
        Pattern::Ctor { ty, .. } => ty == "err",
        Pattern::Annotated { ty, .. } => ty == "err",
        _ => false,
    }
}

struct Walk<'a> {
    program: &'a Program,
    groups: HashMap<Group<'a>, Vec<usize>>,
    returns: HashMap<Group<'a>, Pkgs>,
    params: HashMap<Group<'a>, Vec<Pkgs>>,
    changed: bool,
}

/// Every package the program draws from, in a fixed order, so a set of them
/// is a machine word.
fn intern(program: &Program) -> Vec<String> {
    let mut names: Vec<String> = program
        .fns
        .iter()
        .map(|d| package_of(&d.file).to_string())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    names.sort();
    names.truncate(32);
    names
}

fn bit(table: &[String], pkg: &str) -> Pkgs {
    match table.iter().position(|n| n == pkg) {
        Some(i) => 1 << i,
        None => 0,
    }
}

impl<'a> Walk<'a> {
    /// The packages whose errs this expression may evaluate to, in a
    /// declaration belonging to `pkg` with `binds` naming what locals hold.
    fn expr(&mut self, e: &'a Expr, pkg: Pkgs, binds: &HashMap<&'a str, Pkgs>) -> Pkgs {
        match e {
            Expr::App { head, args, .. } => {
                let Expr::Ident(name, _) = head.as_ref() else {
                    return 0;
                };
                let bare = name.rsplit_once('/').map(|(_, n)| n).unwrap_or(name);
                // a raise is this package's, however the reason was named:
                // provenance is where the err was made, not what it carries
                if bare == "err" || bare == "wrap_err" {
                    return pkg;
                }
                for (i, arg) in args.iter().enumerate() {
                    let carried = self.expr(arg, pkg, binds);
                    if carried != 0 {
                        self.feed((name.as_str(), args.len()), i, carried);
                    }
                }
                self.returns.get(&(name.as_str(), args.len())).copied().unwrap_or(0)
            }
            Expr::Ident(name, _) => binds.get(name.as_str()).copied().unwrap_or(0),
            Expr::Seq(_, b, _) => self.expr(b, pkg, binds),
            // a guard's two exits are both this expression's value
            Expr::Guard { early, rest, .. } => {
                let mut pkgs = self.expr(early, pkg, binds);
                if let Some(Stmt::Expr(last)) = rest.last() {
                    pkgs |= self.expr(last, pkg, binds);
                }
                pkgs
            }
            Expr::Block(body, _) | Expr::Build(body, _) => match body.last() {
                Some(Stmt::Expr(last)) => self.expr(last, pkg, binds),
                _ => 0,
            },
            Expr::Upcast { expr, .. } => self.expr(expr, pkg, binds),
            // an err travelling inside any other form still reaches the
            // calls written there, and a call is where the rule applies —
            // so every sub-expression is walked even where the form itself
            // cannot evaluate to a failure
            Expr::Str(parts, _) => {
                for part in parts {
                    if let TemplatePart::Interp(inner) = part {
                        self.expr(inner, pkg, binds);
                    }
                }
                0
            }
            Expr::List(items, _) => {
                for item in items {
                    self.expr(item, pkg, binds);
                }
                0
            }
            Expr::MapLit(pairs, _) => {
                for (k, v) in pairs {
                    self.expr(k, pkg, binds);
                    self.expr(v, pkg, binds);
                }
                0
            }
            Expr::BinOp { lhs, rhs, .. } | Expr::Join { lhs, rhs, .. } => {
                self.expr(lhs, pkg, binds);
                self.expr(rhs, pkg, binds);
                0
            }
            Expr::Field { base, .. } => {
                self.expr(base, pkg, binds);
                0
            }
            Expr::Index { base, index, .. } => {
                self.expr(base, pkg, binds);
                self.expr(index, pkg, binds);
                0
            }
            Expr::Lambda { body, .. } => {
                self.expr(body, pkg, binds);
                0
            }
            _ => 0,
        }
    }

    /// Record that a caller may hand this group an err from these packages.
    fn feed(&mut self, group: Group<'a>, index: usize, pkgs: Pkgs) {
        let Some(arity) = self.groups.get(&group).and_then(|d| self.program.fns.get(d[0])) else {
            return;
        };
        let width = arity.params.len();
        let slot = self.params.entry(group).or_insert_with(|| vec![0; width]);
        if let Some(existing) = slot.get_mut(index) {
            if *existing | pkgs != *existing {
                *existing |= pkgs;
                self.changed = true;
            }
        }
    }

    fn absorb(&mut self, group: Group<'a>, pkgs: Pkgs) {
        let slot = self.returns.entry(group).or_default();
        if *slot | pkgs != *slot {
            *slot |= pkgs;
            self.changed = true;
        }
    }
}

pub fn analyze(program: &Program) -> Provenance<'_> {
    let table = intern(program);
    let mut groups: HashMap<Group<'_>, Vec<usize>> = HashMap::default();
    for (i, decl) in program.fns.iter().enumerate() {
        groups.entry(group_of(decl)).or_default().push(i);
    }
    let mut walk = Walk {
        program,
        groups,
        returns: HashMap::default(),
        params: HashMap::default(),
        changed: true,
    };
    // a pub group's callers are not all in view: the package raises errs and
    // anyone may hand one back, so a published err parameter is assumed to
    // see its own package's failures. Private groups are fed only by the call
    // sites actually written. The candidates never change; only whether their
    // package is known to raise yet.
    let candidates: Vec<(Group<'_>, usize, Pkgs)> = program
        .fns
        .iter()
        .filter(|d| d.is_pub && !d.synthetic)
        .flat_map(|d| {
            let pkg = bit(&table, package_of(&d.file));
            let group = group_of(d);
            d.params
                .iter()
                .enumerate()
                .filter(|(_, p)| receives_err(p))
                .map(move |(i, _)| (group, i, pkg))
                .collect::<Vec<_>>()
        })
        .collect();
    let mut rounds = 0;
    while walk.changed && rounds < 200 {
        walk.changed = false;
        rounds += 1;
        let raising: Pkgs = walk.returns.values().fold(0, |a, b| a | b);
        for (group, i, pkg) in &candidates {
            if raising & pkg != 0 {
                walk.feed(*group, *i, *pkg);
            }
        }
        for decl in &program.fns {
            let pkg = bit(&table, package_of(&decl.file));
            let group = group_of(decl);
            // a parameter that matches err holds whatever callers fed it
            let mut binds: HashMap<&str, Pkgs> = HashMap::default();
            for (i, pattern) in decl.params.iter().enumerate() {
                let Pattern::Annotated { name, ty, .. } = pattern else { continue };
                if ty != "err" {
                    continue;
                }
                let arriving =
                    walk.params.get(&group).and_then(|v| v.get(i)).cloned().unwrap_or_default();
                binds.insert(name.as_str(), arriving);
            }
            let mut result = 0;
            for stmt in &decl.body {
                match stmt {
                    Stmt::Bind { pattern, expr } => {
                        let value = walk.expr(expr, pkg, &binds);
                        if let Pattern::Var(name, _) = pattern {
                            binds.insert(name.as_str(), value);
                        }
                    }
                    Stmt::Expr(expr) => result = walk.expr(expr, pkg, &binds),
                    Stmt::Set { value, .. } => {
                        walk.expr(value, pkg, &binds);
                    }
                }
            }
            walk.absorb(group, result);
        }
    }
    Provenance { params: walk.params, table }
}

/// The rule: a group that may receive an err raised in its own package must
/// return an err. `returns` is inference's value-set per declaration, which
/// says whether this group hands back anything that is not a failure.
pub fn violations(
    program: &Program,
    prov: &Provenance,
    returns: &[crate::infer::Set],
) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::default();
    for (i, decl) in program.fns.iter().enumerate() {
        if decl.synthetic {
            continue;
        }
        let Some(set) = returns.get(i) else { continue };
        if set & !crate::infer::FAIL == 0 {
            continue;
        }
        let pkg = package_of(&decl.file);
        let mask = bit(&prov.table, pkg);
        let group = group_of(decl);
        let Some(arriving) = prov.params.get(&group) else { continue };
        let own = decl
            .params
            .iter()
            .enumerate()
            .any(|(n, p)| receives_err(p) && arriving.get(n).is_some_and(|s| s & mask != 0));
        if own && seen.insert(decl.name.clone()) {
            let whose = match pkg {
                "" => "this program".to_string(),
                other => format!("`{other}`"),
            };
            out.push(format!(
                "advisory[license]: `{}` rescues an err raised in {whose} — a \
                 failure is handled by a package that did not raise it; return \
                 an err, or let a caller elsewhere name the reason",
                decl.name
            ));
        }
    }
    out.sort();
    out
}
