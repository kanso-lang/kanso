# Type syntax — the ratified design

Gaveled 2026-07-24. The design below is settled. Nothing here is built —
kanso today has no user-defined parameterized types, and field types are
bare names (`peers:list`). The rulings reserve the syntax so the constraint
work has a shape to land in.

## The three forms

A type is a name, a slice, or an application. There is nothing else.

```
string                  a name
[]string                a slice of string
map[string int]         an application: map applied to string and int
```

- **`[]T` is a slice**, Go's prefix form. A type reads left to right —
  "slice of string" — and composes without backtracking: `[]map[string int]`
  is a slice of maps. Postfix `T[]` would read inside-out on the same
  example, and it would keep the postfix bracket occupied.
- **`Name[args]` applies type arguments.** Arguments are separated by
  spaces, like every other kanso list.

## Containers are ordinary applications

`map` takes its key first and its value second, in the argument positions
any other type would use.

```
map[string int]         string keys, int values
set[string]
pair[string int]
```

Go spells this `map[K]V`, a form only `map` can use. Here `map` is a
builtin type that happens to be parameterized, and it reads the same way
`pair` does. The grammar has one rule instead of two.

`map` is also the enumerable adapter (`map coll fn`). Type position and
value position never overlap, so types and values are separate namespaces.

## Tight bracket applies, spaced bracket begins a type

```
map[k []k]              two arguments: k, and []k
```

A `[` pressed against the identifier before it opens an argument list. A
`[` with space before it, or at the start of a type, opens a slice. This
is the same tight-versus-spaced rule the lexer already uses to tell field
access from the pipe.

## The binder declares a constraint; the fields give its order

A type declares its constraints up front, and uses them in its fields:

```
type <k>foo
  name:k
  friend_names:k[]
```

`<k>` says that `k` is a constraint — a name standing for whatever type
arrives — and the fields then force every position mentioning it to agree.
Give `foo[string]` and `name` is a string and `friend_names` is an array of
strings. That agreement is the whole content of `k`; drop it for `any` in
both fields and the type still compiles while the relation is gone.

The binder does not carry order. Order is the order the constraints first
appear in the fields, so `type <k v>pair` with `first:k` and `second:v`
takes `pair[string int]` and nothing has to be repeated.

The binder is not superfluous, which an earlier ruling had it. Fields say
which positions share a name; they cannot say that the name is a variable
rather than a type. Without the binder, `first:k` means a constraint only
because no type named `k` happens to exist — so a declaration's meaning
would depend on the global set of type names, and adding a type called `k`
later would silently turn a constraint into a concrete annotation. The
binder states variable-ness outright, which is the one fact the fields
cannot carry.

## Functions carry no type parameters, and no needless annotations

A function is where inference is strongest, so a function declares
nothing about its types that the compiler can work out. An annotation is
legal only when it is load-bearing: when removing it would change or
break something.

```
fn foo u:string
  only_takes_a_string u      # error: `u:string` is already implied
```

Dispatch discriminators are the common load-bearing case — those choose
which arm is being defined, which is not something inference can derive.
Subtypes and typesets also mean there is not always a principal type to
land on, and where inference genuinely cannot decide, the annotation
becomes load-bearing and is allowed. The rule adjusts itself: superfluous
means removable with no effect.

The consequence for this document is that type syntax barely appears in
function bodies. Slices, applications, and typeset names live almost
entirely in type definitions.

## Typesets, not unions

`typeset` is the name. Members are space-separated and alphabetized:

```
type num float64 int
```

`|` is not in the language.

## Products are records at every arity

kanso has no positional product type. Access is by name, never by
position — the same rule that bans `_` in binding patterns and prefers
keyed reads. A two-field positional value is as wrong as a five-field
one, so there is no `tuple` and no positional `pair`.

The stdlib does ship a two-field constrained record named `pair`, with fields
`first` and `second`, because `zip` produces one and `to_h` consumes one.
Ordinal field names are honest there and nowhere else: `zip` is
domain-blind, so it cannot know what its two values mean. Application
code always knows, and writes the domain record.

## Every typeset has a name

A typeset is declared before it is used. There is no inline form.

```
type principal string user
map[principal string]
```

Naming forces the question "what concept is this?", and a typeset whose
best name is `string_or_user` is usually a typeset that should not exist.
The cost is a `type` line for a genuine one-off, paid mostly by typesets
worth a second look.

This is also what keeps the grammar at three forms. An inline typeset
would need grouping, because a bare space already separates type
arguments — and grouping brings parens into type position, a rule for
one-member groups, and an ordering rule inside the group. None of that
exists.

## Alternatives and why they lose

- **`[string]` for slices.** Mirrors the `[1 2 3]` literal, but reads
  inside-out once composed, and it occupies the postfix bracket that
  application needs.
- **Go's `map[K]V`.** A form only `map` can use. Two rules where
  `map[K V]` needs one.
- **`map{string int}`.** Borrows the map literal's braces without the
  colon that makes them read as a mapping, so the resemblance is visual
  rather than semantic. It also re-specializes `map` after `map[K V]`
  made it ordinary. The resonance argument, applied consistently, would
  drag slices back to `[string]`.
- **`{string:int}` as a bare map type.** The honestly resonant form, and
  the colon collides with the annotation colon: `m:{string:int}` uses one
  punctuation mark for two jobs.
- **Anonymous typesets in parens** — `map[(string user) string]`. Costs
  parens in type position to save a `type` line.
- **`tuple`, or a positional `pair`.** Positional access at any arity.

## Deferred

- **Constraints.** `<k v>` means any k and any v, which is what `pair`
  needs. Map keys will eventually want something like comparable. Bounded
  polymorphism is a separate design.
- **Error locality.** Full inference with no firewall annotations means a
  type error can surface away from its cause; ML and Haskell programmers
  add signatures they do not need for exactly this reason. kanso's answer
  is the language server showing inferred types and diagnostics that
  report the whole conflicting chain. The tradeoff is accepted
  deliberately.
