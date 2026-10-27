# kata: Slim-style templates compiled to kanso

A prototype for #2178. `kata` reads a template written in a subset of Slim and
writes a kanso file holding one function per template. The view is then
ordinary kanso: `kanso check` reads its field accesses and its parameters, and
both engines run it.

```
kanso run cli -- app/views/page.slim > app/views/page.kso
kanso run app
```

## What the subset covers

- Nesting by indentation, with tags closed for you. Void elements (`meta`,
  `img`, `br` and the rest) get no closing tag.
- `tag.class#id` shorthand, with a bare `.class` or `#id` meaning a `div`. A
  `class` attribute is joined onto the shorthand's classes, as Slim does.
- Attributes as `name="text"`, where the text may hold `#{expr}`, or
  `name=expr` for a value.
- `= expr` writes a value escaped, `== expr` writes it raw, and text after a
  tag or a `|` may hold `#{expr}`, also escaped.
- `- if expr`, any number of `- else if expr`, an optional `- else`, and
  `- for x in xs`.
- `/` comments, which drop the rows under them too, and `doctype html`.
- A first line `- locals a b c` names the template's parameters, in order.

## What it writes

The template becomes a `pub fn` named after the file. Each `- if` branch and
each `- for` body becomes a helper function of exactly the names in scope it
mentions, because kanso refuses an unused parameter; a body that mentions none
becomes a constant. Markup and holes are packed into string literals that fit
kanso's 80 columns, and a comment above each literal names the template lines
its holes came from.

Partials and layouts need nothing of their own, because every template in a
views directory is a function in one module. A layout is a template that
takes the page as a local and writes it with `== body`, and a page uses it
with `== layout "Profile" (card user)`.

The generated views lean on three functions in `app/views/kata.kso`:
`escape`, `each` and `joined`. They belong in the standard library once the
design settles.

## What it does not do yet

- Diagnostics point at the generated file. The comments make the trip back to
  the template short, but a compiler that read `.slim` files itself could
  report `page.slim:18` directly.
- No `- case` and no whitespace control.
