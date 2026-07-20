# Function values and nullary application

How a function is passed rather than called, and how a function that takes no
arguments is called at all. Ratified in design dialogue 2026-07-20 (gavel BB);
the dispatch-group half leans on gavel G (eta-reduction), still open.

## The problem

Application is juxtaposition: `f x` calls `f` with `x`, and the argument
sitting next to `f` is what signals the call. A bare `f` with nothing beside it
is the function itself — a value you can pass.

That rule has nothing to say when there are no arguments. A name for a
function that takes none — a clock reading, a fresh id, a unit of work you
want to defer — can't use an adjacent argument to mark the call, so a bare
mention of it can't distinguish "hand me the function" from "run it."

An impure language answers with parentheses as a call operator: `now` is the
function, `now()` runs it. The parentheses are the whole disambiguator, and
they are why `now` and `now()` mean different things.

## The decision

Keep the parentheses, but read them as a value, not a new operator. A function
of no arguments is a function of the unit value `()`, and calling it is
ordinary application to that value:

```
now          # the function — a value you can pass
now()        # apply it to unit — the call
```

`now()` and `now ()` are the same term; application does not care about the
space. So the familiar spelling costs no new grammar — `()` is a value like
`"kanso"`, and `now ()` is the same rule as `greet "kanso"` with `()` where the
string was. This is unit application, the ML convention (OCaml, SML, Scala),
and it is why those languages never needed a sequencing arrow like Haskell's
`<-` to answer the syntax question.

## Why it is never ambiguous

A name resolves to exactly one of two shapes, and its declaration says which:

```
tau = 6.28318      # a constant — a value; you mention it, never call it
fn now             # a function — you mention it to pass it, now() to call it
```

Constants and functions share one namespace, and a constant cannot be
shadowed; only `fn` declarations carry multiple arms. So a name is a constant
or it is a function, decided where it is defined, and no use site has to guess.
There is no third form where the same definition could be read either way.

## The dispatch-group half

kanso functions are not single closures; a name is a group of dispatch arms.
Making a bare name a value therefore commits the language to a dispatch group
being a first-class value — the thing that, applied to an argument, runs the
dispatch:

```
fn encode true       # encode names a group of six arms
fn encode false
fn encode n:int
...

xs . map encode      # `encode` here is that group, passed as one value
```

Gavel G records that bare names as function values already compile; the corpus
still writes the forwarding lambda (`map (v -> encode v)`) because eta-reduction
is not yet mandated as the one rendering. So the *reference* direction already
works — the nullary convention adds the *call* direction as the degenerate case
of application, and strengthens the case for G: once a bare name is uniformly
the function value, `map encode` and `now()` are the same rule seen at one
argument and at none.

What remains for G to settle is canon, not capability: whether the forwarding
lambda becomes an error, and how a multi-arm group's value composes when it is
stored and passed around. The nullary case is the simplest instance of that
question, not a separate one.

## Effects are orthogonal

Whether a function performs an effect is a separate axis from how many
arguments it takes. `write_file "path" "body"` and a nullary clock reading get
the identical treatment — both evaluate to a description the executor performs,
both stay pure until then. Arity is a grammar question; effect handling is the
executor's, and the two do not interact. A nullary effect is spelled `now()`
for the same reason `write_file "path" "body"` is spelled with its arguments:
application is how you ask for the work, whatever the work is.

## Status

- **Ruled (BB):** a bare `fn` name is the function value; `name()` calls a
  nullary function as application to unit `()`. No `<-`.
- **Open (G):** eta-reduction as canon — whether the forwarding lambda is
  banned, and the composition rules for a dispatch group held as a value.
