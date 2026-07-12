# visibility: `pub` instead of `_` — proposal, draft 0.1

Status: **proposed** (Clay asked for "something more elegant than underscores";
not yet gaveled; nothing implemented).

## the two real defects of `_name`

1. **Visibility changes rename every call site.** Promoting `_parse` to public
   means touching every internal caller — a mechanical diff that buries the one
   line that matters (the decision to export). Visibility is a property of the
   declaration; encoding it in the name smears it across the module.
2. **`_` is already taken.** It means wildcard in patterns and deliberately
   unused in bindings. A third meaning (module-private) makes the most loaded
   character in the language carry unrelated concepts — braided, in Hickey
   terms.

## proposal

A `pub` modifier on the declaration, private by default:

```
pub fn decode text
  _value (bytes text)

fn value_at cs p        # private: the default needs no mark
  ...

pub tau = 6.28318

pub type parse_failure
  position:int
  reason:string
```

- **Private is the default.** The unmarked case is the common one, and the
  module's API surface is exactly the set of `pub` lines — greppable in one
  pass.
- Overload groups: `pub` appears on **every arm** of a public group, and mixed
  marking is a compile error. (One decision per name; the repetition keeps
  each arm honest when read in isolation.)
- Canonical order stays alphabetical by bare name; `pub` doesn't sort.
- "Unused private declaration" keeps working verbatim — the check moves from
  name-prefix to modifier-absence.
- `main` and `test_*` are implicitly entry points, as today; marking them
  `pub` is a formatting error (nothing may be redundant).

## why not the alternatives

- **Capitalization (Go):** collides with all-lowercase snake_case; kanso names
  would grow a second alphabet for one bit of information.
- **Export list at the top of the module:** separates the fact from the
  declaration it describes; every read of a fn requires a second lookup.
- **File-based (public = declared in the file named after the module):**
  braids file layout with API surface; moving a fn between files would change
  its visibility.

## cost

One keyword (the first modifier in the language) and a mechanical migration:
delete `_` prefixes, add `pub` to what lacked one. Both engines, spec §
modules, the book's ch06, kanso-json, and kq all change in one sweep. The
migration is a rename with no semantic ambiguity, so it can be scripted and
verified by the differential suite.

## open sub-question

Whether `pub` also marks re-exports once cross-module imports land (Go says
no; rust says `pub use`). Deferred until imports exist.
