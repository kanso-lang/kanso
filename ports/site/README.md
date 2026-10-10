# site: a Zola-style static site generator in kanso

This is a port of the core of [Zola](https://www.getzola.org), the static
site generator written in Rust by Vincent Prouillet and contributors, and of
[Tera](https://keats.github.io/tera/), the template engine Zola uses. The
code here was written from the documented behaviour of both projects; none of
their source is copied.

A site is a directory:

    config.toml      base_url, title, taxonomies, extra, generate_feeds
    content/         markdown with TOML (+++) or YAML (---) front matter
    templates/       Tera-style templates
    static/          files copied as they are

and `site build` turns it into a tree of HTML under `public/`.

## Running it

    KANSO=/tmp/claude-0/kanso-main/kanso
    $KANSO run main.kso -- build fixtures/blog -o /tmp/out
    $KANSO run main.kso --interp -- build fixtures/blog -o /tmp/out
    $KANSO build main.kso --release && ./main build fixtures/blog -o /tmp/out

The commands:

    site build [DIR] [-o OUT] [--drafts] [-u BASE_URL]
    site check [DIR] [--drafts]       build in memory and report, write nothing
    site markdown FILE                print one markdown file as HTML

`OUT` defaults to `DIR/public`. The generator cannot delete files (std/os has
no remove), so build into a directory that does not exist yet.

`sh check.sh` runs the unit tests and every fixture on the interpreter, a dev
build and a release build. `sh check.sh --bless` rewrites the expected files
from the interpreter.

## What it covers

Content
- sections (`_index.md`) and pages, nested to any depth; a page belongs to the
  nearest section above it; a root section is made up when there is none
- page bundles (`dir/index.md`), with the other files in the directory copied
  beside the page and listed in `page.assets`
- slugs from the file name, from `slug`, or a whole `path`; a
  `2024-01-15-name.md` file name gives the date
- drafts (left out unless `--drafts`), `aliases` (redirect pages),
  `render = false` sections, `template` and `page_template`
- `sort_by` date, weight, title or none; `page.lower` and `page.higher`
- pagination (`paginate_by`), with a `paginator` like Zola's and a redirect at
  `page/1/`
- taxonomies from `config.taxonomies`, with `NAME/list.html` and
  `NAME/single.html` or the `taxonomy_list.html` and `taxonomy_single.html`
  fallbacks
- `sitemap.xml`, `robots.txt`, `atom.xml` (when `generate_feeds` is set) and
  `404.html` (when there is a template for it); each of the first three can be
  overridden by a template of the same name
- internal links `@/blog/post.md#anchor` in markdown and in `get_url`, and a
  build error for one that names no file
- shortcodes, `{{ name(arg=1) }}` and `{% name(arg=1) %}body{% end %}`, from
  `templates/shortcodes/NAME.html`

Templates (a Tera subset)
- `{{ }}`, `{% %}`, `{# #}`, and `-` on either side to trim whitespace
- `extends`, `block` with `super()`, `include`, `macro` with defaults,
  `import "file" as name`, `if`/`elif`/`else`, `for x in xs`,
  `for k, v in map`, `loop.index`/`index0`/`first`/`last`/`length`, `else`
  on loops, `set`, `set_global`, `filter` blocks, `raw`
- expressions: literals, lists, attribute and index access, `+ - * / %`, `~`,
  comparisons, `and`, `or`, `not`, `in`, `not in`, and `is` tests (defined,
  undefined, odd, even, string, number, iterable, divisibleby, starting_with,
  ending_with, containing)
- filters: safe, escape, upper, lower, capitalize, title, trim, slugify,
  striptags, wordcount, length, reverse, first, last, nth, join, truncate,
  replace, split, date, sort, map, filter, group_by, slice, concat, unique,
  int, float, abs, round, pluralize, linebreaksbr, urlencode, json_encode,
  as_str, default, and Zola's markdown
- functions: range, throw, super, get_url, get_page, get_section,
  get_taxonomy_url
- output is HTML-escaped unless marked safe, as in Zola, so `page.content`
  is written `{{ page.content | safe }}`

Markdown (a CommonMark subset with some GitHub extensions)
- ATX and setext headings with Zola's ids (`intro`, `intro-1`) and
  `{#custom-id}`, a nested table of contents in `page.toc`
- paragraphs, hard breaks, block quotes, bullet and ordered lists (nested,
  tight and loose), fenced code with a language class, thematic breaks, HTML
  blocks, tables with alignment
- emphasis, strong, strikethrough, code spans, links, images, reference
  links, autolinks, inline HTML, backslash escapes, entities
- `<!-- more -->` for `page.summary`

## What it leaves out

Syntax highlighting, Sass, the search index, image processing, multilingual
sites, `zola serve` and live reload, `load_data`, external link checking,
`transparent` sections, footnotes, smart punctuation, and Tera's `break` and
`continue`. Tera escapes `/` as `&#x2F;`; this port leaves it alone, so
templates do not need `| safe` on every URL. Error messages follow Zola's
content but not its exact wording.

## Layout

    main.kso         reads the arguments and hands them to zola/main
    zola/            the generator: CLI, disk I/O, content model, outputs
    tera/            the template engine: lexer, parser, renderer, filters
    markdown/        block parser, inline parser, HTML renderer
    toml/            the TOML subset used by config and front matter
    textkit/         string helpers std/text does not have
    fixtures/        sample sites, markdown cases, error cases, expectations
    bugs/            reproductions of compiler and runtime problems
    FRICTION.md      the journal of where kanso got in the way
