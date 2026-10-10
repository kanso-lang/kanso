# sat: a CDCL SAT solver in kanso

A port of the core of [MiniSat](http://minisat.se/) by Niklas Eén and Niklas
Sörensson, the small conflict-driven clause-learning solver described in
"An Extensible SAT-solver" (SAT 2003) and the basis of a great many solvers
since. This port was written from the published description of the
algorithm and my understanding of MiniSat 2.2's design, not from its source.

It reads a formula in DIMACS CNF, decides whether it is satisfiable, and
answers in the SAT competition format. A satisfying assignment is checked
against every input clause before it is printed.

## What it covers

- **DIMACS input**: comment lines anywhere, clauses spanning lines, tabs, the
  SATLIB `%` terminator, and a line-numbered refusal for every malformed
  input (missing or repeated header, a literal past the declared variables,
  a clause count that disagrees with the header, an unterminated clause).
- **Clause loading** as MiniSat does it: literals sorted and deduplicated,
  tautologies dropped, units propagated at level 0, an empty clause refuting
  the formula at once.
- **Two watched literals** for unit propagation.
- **First-UIP conflict analysis** with MiniSat's deep (recursive)
  minimization of the learnt clause, cut short by the abstraction of its
  decision levels.
- **Non-chronological backjumping** to the second-highest level of the
  learnt clause.
- **VSIDS**: variable activities bumped on every conflict, decayed by 0.95,
  rescaled past 1e100, kept in a binary heap; decisions take the most active
  unassigned variable with its saved phase (negative at first).
- **Restarts**: Luby (the default, 100 conflicts a unit) or geometric (100
  conflicts, growing by half each time).
- **Learnt clause deletion**: clause activities bumped and decayed by 0.999;
  when the learnt clauses outnumber a limit that grows at each restart, the
  least active half goes, keeping binary clauses and clauses that are the
  reason for a current assignment.
- **Output** in the competition format, with exit status 10 for
  satisfiable, 20 for unsatisfiable and 1 for an error, and `c` lines giving
  conflicts, decisions, propagations, restarts and learnt literals.
- **Benchmark generators** for the pigeonhole principle, n queens and
  uniform random 3-SAT, so every fixture can be rebuilt from a command line.

## What it leaves out

- MiniSat's level-0 simplification (removing satisfied clauses), its blocker
  literals in the watch lists, and random decisions.
- The learnt-clause limit is simpler than MiniSat's: a third of the original
  clauses plus 10, growing by a tenth at each restart, where MiniSat grows it
  on its own conflict schedule.
- Clause deletion is lazy: a deleted clause is emptied, and the watch lists
  drop it when propagation next visits it.
- The forced literal of a clause is found by value during analysis, rather
  than kept in the clause's first position, because rewriting a clause here
  is a copy (see FRICTION.md, F26).
- MiniSat's preprocessor (SatELite-style elimination), incremental solving,
  assumptions, proof output and resource limits.

The search is deterministic: there is no random seed, so the three kanso
engines print the same bytes for every formula.

## Running it

    KANSO=/tmp/claude-0/kanso-main/kanso

    $KANSO run main.kso -- solve tests/bench/queens8.cnf
    $KANSO run main.kso -- solve --geometric tests/bench/php5.cnf
    $KANSO run main.kso -- solve - < formula.cnf
    $KANSO run main.kso --interp -- solve tests/solve/tiny.cnf

    $KANSO run main.kso -- gen pigeonhole 6 > php6.cnf
    $KANSO run main.kso -- gen queens 10 > queens10.cnf
    $KANSO run main.kso -- gen random 100 426 21 > r100.cnf

    $KANSO build main.kso --release && ./main solve php6.cnf

A satisfiable answer looks like this:

    c kanso-sat: 3 variables, 3 clauses
    c restart policy: luby, 100 conflicts a unit
    c conflicts: 0
    c decisions: 1
    c learnt literals: 0 (0 removed by minimization)
    c propagations: 3
    c restarts: 0
    c model checked: all 3 clauses satisfied
    s SATISFIABLE
    v -1 2 3 0

## Layout

    main.kso          the entry: hands the arguments to cli/dispatch
    cli/              the command line and the competition-format output
    dimacs/           the DIMACS reader and writer
    solver/           the CDCL solver
      vec.kso           a persistent vector, since lists have no indexed update
      lits.kso          the literal encoding
      trail.kso         assignment, decision levels, backjumping
      watch.kso         two-watched-literal propagation
      analyze.kso       first-UIP analysis and deep minimization
      order.kso         VSIDS activities and the decision heap
      db.kso            the clause database and clause activities
      reduce.kso        learnt clause deletion
      search.kso        loading, the search loop and restarts
      model.kso         checking a model against the formula
    gen/              pigeonhole, queens and random 3-SAT generators
    tests/
      solve/          handwritten formulas and their answers
      errors/         malformed input and the refusals
      cli/            command lines, with stdin where one is read
      bench/          the generated benchmark set and MANIFEST
      oracle.py       an independent DPLL used to check verdicts and models
    check.sh          runs everything on all three engines
    bugs/             reductions of the compiler bugs found while porting
    FRICTION.md       the journal

## Tests

`./check.sh` runs the unit tests of every module (54 `test_` constants), then
59 fixture cases on the interpreter, a dev build and a release build, and
fails if any output, including stderr and the exit status, differs from the
expected file or between engines. Every benchmark file must also be exactly
what `gen` prints for its arguments on each engine. When python3 is
available, `tests/oracle.py`, a plain DPLL sharing no code with the solver,
checks the verdict and the model of every handwritten formula and the
smaller benchmarks, and of 60 random formulas near the satisfiability
threshold solved by the release build with each restart policy.

The slowest fixture, pigeonhole 6, takes about 7 seconds on the interpreter
and under half a second as a release build.
