# editor support

`kanso/` is one TextMate-grammar bundle that serves both editor families — the grammar lives in `kanso/syntaxes/kanso.tmLanguage.json`.

Beyond colour, `kanso lsp` is a language server: an editor that speaks the Language Server Protocol starts it, and it reports the diagnostics `kanso check` prints as you type, shows the type the compiler inferred for the name under the pointer, and jumps to where a name was declared. Hovering a parameter shows what it accepts: the union of what every call it is handed to can take, following it through as many calls as it passes. With `fn bar s:string` and `fn bar n:int`, hovering `x` in `fn foo x` (whose body is `bar x`) shows `x: int | string`. When the program's own calls hand it something narrower, a second line says what. The playground runs the same server in the tab.

Each recipe below starts the binary as `kanso lsp`, so `kanso` has to be on the editor's `PATH`, or named in full.

## rubymine / any jetbrains ide

1. Settings → Editor → TextMate Bundles
2. `+` and select this repo's `editors/kanso` directory
3. Open any `.kso` file

The bundled TextMate plugin must be enabled (it is by default). If `.kso` doesn't pick up, check Settings → Editor → File Types for a stale association.

For the language server, install the LSP4IJ plugin (Settings → Plugins, search "LSP4IJ"), then Settings → Languages & Frameworks → Language Servers → `+`:

- Server tab: name `kanso`, command `kanso lsp`
- Mappings tab, File name patterns: `*.kso`, language id `kanso`

After the grammar changes, JetBrains keeps the old one cached. Removing and re-adding the bundle is not always enough — untick it, Apply, tick it again, Apply.

## vs code

Symlink the bundle into your extensions directory and reload:

```
ln -s ~/dev/kanso/editors/kanso ~/.vscode/extensions/kanso-lang.kanso-syntax-0.1.0
```

The bundle doubles as a VS Code extension (`package.json` declares the language and grammar), so this also brings `//` comment toggling and bracket/quote autoclosing from `language-configuration.json`.

The extension starts the language server too, once its one dependency is installed: run `npm install` in `editors/kanso`. The `kanso.server.path` setting names the binary when `kanso` is not on the `PATH`.

## neovim

Neovim 0.11 and later carry an LSP client. In `init.lua`:

```lua
vim.filetype.add({ extension = { kso = 'kanso' } })
vim.lsp.config('kanso', { cmd = { 'kanso', 'lsp' }, filetypes = { 'kanso' } })
vim.lsp.enable('kanso')
```

`K` shows the hover and `gd` (or `vim.lsp.buf.definition()`) jumps to a definition.

## what highlights

Declarations (`fn`/`type` plus their names), fields, builtins (`print`, `map`, `err`, ...), primitive types, `true`/`false`/`none`, `_`, integers, strings with `{interpolation}` highlighted as embedded kanso, comments, and the operators — including ` . ` pipes and the `.>`, `.!` and `.?` chain words.
