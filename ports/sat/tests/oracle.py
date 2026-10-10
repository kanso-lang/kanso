"""An independent satisfiability oracle for the fuzz check in check.sh.

A plain DPLL with unit propagation, written separately from the kanso solver
and sharing no code with it. It is slow and only meant for formulas of a few
dozen variables. Usage:

    python3 -I oracle.py formula.cnf solver_output.txt

Exits 0 when the solver's `s` line agrees with the oracle and, for a
satisfiable answer, its `v` lines satisfy every clause; prints the reason and
exits 1 otherwise.
"""
import sys


def parse(path):
    clauses, cur = [], []
    for line in open(path):
        line = line.strip()
        if line == "%":
            break
        if not line or line[0] in "cp":
            continue
        for word in line.split():
            n = int(word)
            if n == 0:
                clauses.append(cur)
                cur = []
            else:
                cur.append(n)
    return clauses


def dpll(clauses, assignment):
    while True:
        unit = None
        for c in clauses:
            if any(assignment.get(abs(l)) == (l > 0) for l in c):
                continue
            free = [l for l in c if abs(l) not in assignment]
            if not free:
                return False
            if len(free) == 1:
                unit = free[0]
                break
        if unit is None:
            break
        assignment[abs(unit)] = unit > 0
    for c in clauses:
        for l in c:
            if abs(l) not in assignment:
                for value in (l > 0, l < 0):
                    trial = dict(assignment)
                    trial[abs(l)] = value
                    if dpll(clauses, trial):
                        return True
                return False
    return True


def main():
    clauses = parse(sys.argv[1])
    out = open(sys.argv[2]).read().split("\n")
    status = [l for l in out if l.startswith("s ")]
    model = set()
    for l in out:
        if l.startswith("v "):
            model.update(int(w) for w in l[2:].split() if w != "0")
    expected = dpll(clauses, {})
    said = status[0] if status else "(no s line)"
    if expected and said != "s SATISFIABLE":
        print("oracle says SAT, solver said", said)
        sys.exit(1)
    if not expected and said != "s UNSATISFIABLE":
        print("oracle says UNSAT, solver said", said)
        sys.exit(1)
    if expected:
        for i, c in enumerate(clauses, 1):
            if not any(l in model for l in c):
                print("clause", i, "is false under the solver's model")
                sys.exit(1)
    sys.exit(0)


main()
