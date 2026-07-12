# editor support

`kanso/` is one TextMate-grammar bundle that serves both editor families — the grammar lives in `kanso/syntaxes/kanso.tmLanguage.json`.

## rubymine / any jetbrains ide

1. Settings → Editor → TextMate Bundles
2. `+` and select this repo's `editors/kanso` directory
3. Open any `.kso` file

The bundled TextMate plugin must be enabled (it is by default). If `.kso` doesn't pick up, check Settings → Editor → File Types for a stale association.

## vs code

Symlink the bundle into your extensions directory and reload:

```
ln -s ~/dev/kanso/editors/kanso ~/.vscode/extensions/kanso-lang.kanso-syntax-0.1.0
```

The bundle doubles as a VS Code extension (`package.json` declares the language and grammar), so this also brings `//` comment toggling and bracket/quote autoclosing from `language-configuration.json`.

## what highlights

Declarations (`fn`/`type` plus their names), fields, builtins (`print`, `map`, `err`, ...), primitive types, `true`/`false`/`none`, `_`, integers, strings with `{interpolation}` highlighted as embedded kanso, comments, and the operators — including ` . ` pipes and `>>` sequencing.
