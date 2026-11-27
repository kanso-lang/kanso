# Compiler log

> # ⚠️ THIS FILE IS APPEND ONLY ⚠️
>
> **Never edit or delete an existing entry. Only ADD new entries at the bottom.**
>
> Every performance/memory approach considered, decision made, thing
> tried-and-reverted, and thread left open goes here — so no thread is ever
> silently dropped again. (The dead-reuse thread in the first entry is *exactly*
> why this file exists: a prior session wired `linear.rs` to nothing and no one
> noticed for weeks.)
>
> Newest entries at the bottom. Date every entry. Tag each item:
> **OPEN / DONE / REVERTED / REFUTED / SPECULATIVE**. When you close an OPEN
> thread, do NOT edit it — append a new entry that references it.

---

> The last forty entries. Everything older is in `log/compiler-log-archive.md`,
> unedited — go there for a thread this file does not mention, and search it
> before concluding an idea is new.

## 2026-11-11 — senro gets its command, its migration files and a page's check (#2455)

The first four items of the 2026-11-11 directive are built in senro, and
the site's guides follow them. senro's design is cloud's (#2280), so each
choice below was made where it came up and is recorded here and on #2455.

**The command.** `bin/senro` is a shell launcher over `senro.kso`. `new`,
`generate` and `routes` are kanso; `server` and `migrate` are handed to
`kanso run .`, because a child kanso starts has no terminal (`os/run`
captures a child's output and `os/start` drops it). `senro routes` reads
the table from what `kanso derived` prints, so listing routes compiles the
app and opens no database. A generator from outside senro is a kanso
program at `generators/<name>.kso`, in the app or beside a library its
entry imports by path.

**Migrations are files.** `migrations/<stamp>_<name>.kso` holds one pub
constant, a change or a list of changes, and the stamp is fourteen digits
of UTC. The order is the files' names. That needed one thing of the
compiler, `sources` on a derivation's module (the entry above), since a
module's files share one namespace and nothing else says which file a
constant is in. The models derivation defines `migrations/all` from the
files, so an entry is written as before, and a migration is recorded under
its stamp. Two things were decided against. A stamp in the constant's name
would have ordered them with no compiler change and made every migration's
name ugly. A list the app keeps by hand was what senro had, and two
branches that each add a migration conflict on it every time.

Rolling a migration back is not built. A migration is a list of changes
with no inverse written down, and that design is open on #2455.

**A page's check.** A module's `admits action viewer db` says who may
reach its actions. A provider's request names its page, and the server
answers it as it would the page: the page's module must lean on a module
that asks for the provider (worked out from the app's calls), a page that
asks for a sign-in answers only a visitor who has one, and `admits` is
asked once for the batched request. Before this the provider route was
open to anyone, which the README did not say. A token signed when the
page renders was weighed and dropped: a permission taken away afterwards
would have kept working until the token aged out. The hook is `admits`
and not `admits?` because a `?` function answers a plain true or false,
and this one may read the database.

**Generators.** `controller`, `model`, `migration` and `mailer` join
`scaffold` and `authentication`. `new` now runs a mail worker beside the
server, so a mailer in an app without accounts has something to send its
letters.

**Still owed on #2455.** Confirmation, lockout, sign-in tracking and an
inactivity timeout are hand-written in the guestbook and belong in what
the `authentication` generator turns on. `job` and `resource` generators.
Server components' actions are not yet answered as their page is. Guides
for controllers, mailers, testing, configuration, deployment and security.

No counter moved in kanso: the work is in senro and on the site.

## 2026-11-11 — `kanso test` performs a test whose value is an effect (#2458)

senro's guides owe a page on testing an app (#2455), and an app's actions
take a database session and answer an effect. `kanso test` could not test
one. It evaluated each `test_*` constant and passed the ones whose value
was `true`, so a test whose value was an effect read
`FAILED (returned <io>)`.

**The search.** The log's entries on the test verb (2026-10-14, the module
a test is run inside; 2026-10-29, std/expect in the binary) say nothing of
effects. The archive's first design of the verb says "a test is a constant
named `test_*` whose value is `true`. no framework, no assertion dsl". The
book says a test's value is a boolean in chapter 01, chapter 07 and the
tour. No fixture under `tests/golden/test_verb` held an effect, and no
issue asked. What the runner printed for an effect was what it prints for
any value that is not `true`, and nobody had decided it.

**The decision**, by the committee, recorded on #2458. A test stays a
constant and `true` stays the only passing value. An effect is a value, a
description an executor performs, so a test whose value is a description
has not said true or false yet. The runner performs it, with the executor
`kanso run --interp` uses, and judges what it yields as it judges any
test's value. An err the effect ends in fails the test and prints its
reason and where it was born, as an entry's unhandled err does. Tests
still run one at a time in the order the file declares them.

Two alternatives were turned down. A second kind of test, under another
prefix or another verb, makes two ideas of one. Judging the plan in place
of performing it yields nothing to compare: a plan lists the effects that
would run, and a test of a query needs the rows.

**Built.** `run_tests_on_stack` executes a test whose value is a `Desc`
and matches on what comes back. `tests/golden/test_verb/performed` holds a
pure test, an effect that yields true, one that yields false, one that
yields a string, one that ends in an err, and one after the err to show
the runner went on; `performed.out` and `performed.stderr` are what it
prints. Watched red on the old runner: all five effectful tests read
`FAILED (returned <io>)`.

The book's effects chapter gains a paragraph after its two levels of
testing, and the tour's sentence on what a test is now allows an effect.

**Open.** Each effectful test gets an executor of its own, seeded as
`run --interp` seeds one: from `KANSO_SEED` when it is set, and from the
system when it is not. A suite has no setup or teardown; a test that needs
a clean table makes one.
senro's testing helpers are built on this next.

## 2026-11-11 — senro: jobs by convention, an API generator, an idle window, Secure cookies, four more guides (#2455)

More of the 2026-11-11 directive, built in senro and written up on the
site. senro's design is cloud's (#2280); each choice is recorded here and
on #2455.

**Jobs by convention** (senro#122). An app's own work had no convention:
it built a table of performers by hand, and a job's name was a string
nothing checked. A module named `<name>_job` is now the job `<name>` and
its `perform db args` is the work. `derive jobs/performers` gives the
entry the table, found from the modules it imports as `routes/rest` finds
actions. `jobs/perform_later db name args` queues one, and a name written
as a string that is no job's is refused at the call while the app
compiles, as `routes/path_to` refuses a route that is not there. `senro
new` writes a second worker for these beside the one that sends mail, and
`jobs/beside` starts no worker for a table with no performers, so an app
with no jobs pays nothing for the line. One queue, `default`; a job's own
queue is left until an app needs one.

**Lockout and tracking in what the generator writes** (senro#121). The
guestbook's hand-written lockout and sign-in tracking moved into an
`accounts` module, and `senro generate authentication` writes a sign-in
built on it: five wrong passwords in a row lock an account for an hour,
and each sign-in is counted and stamped. Five and an hour were chosen
because the generated app unlocks by time alone for now, and the address
limit beside it already allows ten tries in three minutes. The columns the
module writes are `accounts/columns`, read by the generator, so the list
has one home. Confirmation by mail and the unlock link are still the
guestbook's alone.

**A resource with no pages** (senro#124). `senro generate resource` writes
an API over a model, as `rails g scaffold --api` does: the model and its
migration as `scaffold` writes them, and five actions that answer JSON.
`web/input` reads a posted form or a JSON object by the request's content
type, and the generated module keeps only the fields it was told about.
Two faults turned up while driving it. An update answered the row as the
model guessed it would be, so an int sent as JSON came back as a string;
`pg/update_by_id` now returns the stored row. And an update that changed
no field built an SQL statement with nothing to set; it now answers the
row as it stands.

**An inactivity window** (senro#125). `web/idle_after held minutes` ends
a session after that long with no request, as Devise's timeoutable does.
The cookie carries when the session was last seen, and an answer rewrites
it once a second has passed, so a page that makes several requests at
once does not rewrite it for each. The alternative was to keep the last
moment in the database, which would cost every signed-in request a write.
The generator does not turn the window on.

**Secure cookies** (senro#123). Reading `web` to write the security guide
found that the session cookie was `HttpOnly` and `SameSite=Lax` and never
`Secure`. The server speaks HTTP behind a proxy, and cannot see the
scheme the visitor used. It now reads `X-Forwarded-Proto` and marks every
cookie in the answer `Secure` when the header says `https`. The
alternative was a setting, which every deploy would have to remember. A
client that sends the header falsely gets cookies its own browser will
not send back, so trusting it costs nothing.

**The guides.** Four are new: controllers and actions, mailers, security,
and configuration and deployment. The jobs guide gains the convention,
and the sessions guide's lockout and tracking sections now quote
`accounts`. Mail's transports are described in the mailers guide alone;
the sessions guide points at it. The generators and command-line guides
gain the resource generator and the sessions guide the inactivity window.
Every panel is checked against senro at 70dc382 by `senro_quotes`.

A standalone output panel had no style of its own: `.code-output` was
written for the lower half of a code panel, and the senro guides use it
by itself. It drew pale text on the page with no background, and a long
line widened the whole page: the jobs guide measured 2,344 pixels across
in a 1,280-pixel window. `docs/style.css` now makes one that stands
alone in an article a panel with its own horizontal scroll. Ten guides
were rendered through the layout and measured at 1,280 and 390 pixels,
and each now stays inside its window.

Clay, reading the site: "the web site has what looks like a not equal
sign. it should just be != i think. you don't need to render it in a
fancy way that's not what real code looks like." JetBrains Mono joins
`!=` and `->` into one glyph each. The stylesheet now turns the font's
ligatures off for code everywhere, and `site_smoke` asks the browser what
it settled on for a line of code under that stylesheet. Watched red
before the rule: `code is shown with ligatures (normal)`.

**Open.** A guide to testing waits on #2460 and on senro's testing
helpers. A component's server action answered as its page, rolling a
migration back, and confirmation and unlock mail in what the generator
writes are still owed.

## 2026-11-11 — a test file beside an entry reads what the entry reads (#2460)

A senro app could not be tested with `kanso test`. Two things stood in the
way, both found by trying it on `examples/notebook`. The verb took the
app's directory for one module, where it is an entry and the files it
imports one at a time, and the load then failed on `views/`, which is a
module only because the entry's `derive kata/views` writes its functions.
And a test file is a library, so it could not write `derive` lines, and had
no `note/create`, no `routes` and no `schema` to call.

**The search.** The log's entries on the test verb (2026-10-14,
2026-10-29) and the 2026-10-12 gavel (#1883: a test file is an outside
consumer of the module beside it) are all about a module that is a
directory. No fixture under `tests/golden/test_verb` had an entry or a
derivation in it, and no test file in kanso, kq or senro sat beside a
`main.kso`.

**The decision**, by the committee, recorded on #2460. A test file beside
a `main.kso` tests that program. It is compiled with the entry's imports
and the entry's derivations, and none of the entry's statements run. It
may add imports of its own, and only those can read as unused. The gavel
on #1883 carries over a word at a time: a test file reads the thing beside
it as a consumer would, and beside an entry that thing is the program.

The alternative was to let a test file write `derive` lines. The committee
first leaned that way and turned it down on two counts. `routes/rest`
describes every module the entry imports, so a test file would have to
repeat the entry's whole import list to derive the same table. And the
app's derivations would then be written once in `main.kso` and again in
every test file, which is one fact with several homes.

**Built.** `compile_test_file` looks for `main.kso` in the test file's
directory. When it is there, the entry is parsed, its imports are lent to
the test file, and its derivations run over the entry as they do when it
is the program, so a derivation sees none of the test file's helpers. What
they declare is declared for the test file, and an arm they define is
compiled into its module. The library is read as far as both texts reach.
An entry that does not parse, or whose derivation refuses, is reported at
the entry's own line.

Three fixtures under `tests/golden/test_verb`: `beside_an_entry` calls a
module the entry imports, reads the table the entry derives and calls an
arm the derivation wrote; `beside_a_broken_entry` and
`beside_a_refused_entry` pin the two reports. Watched red with the lookup
turned off: `unknown name` for each of the entry's names, and for the
broken entry the test file's own `unknown name` where the syntax error was
expected.

**The rows.** CI's first reading had `library_instructions` 2,304 above
its golden. A file's import list was copied on every library compile, to
know which imports the file wrote itself once an entry had lent it more.
The copy is now made only when an entry is beside the file, and the row is
63,634,563 -> 63,634,689: 126 instructions for the question "is there an
entry beside this file" on a path where the answer is no.
`entry_instructions` is 119,166,280 -> 119,166,322. Nothing on the entry's
path changed, so that is the binary's layout. The browser's compile row
went the other way by two, 412,081,307 -> 412,081,305.

The welfare floor is lowered by what this costs, under the exception for
building a ruled part of the language: #2460 is the committee's decision,
and the floor's history says so: 95.39230414088809 ->
95.3923041102071.

**Open.** Each test file beside an entry compiles the app again. One test
file beside senro's `examples/notebook` took 1.9 s in this container from
start to finish, and nothing here measures an app with many of them.

## 2026-11-11 — directive: specs that nest, with hooks, first class in senro

Clay, to cloud: "the framework needs to have something like what the rails
helpers do where your tests set up a transaction in a before block or
around block, so it gets reverted at the end, and nested specs work and so
forth. i don't know how far you've gotten on our kanso alternative to
rspec/ginkgo/gomega but it should be a first class citizen in senro."

Recorded as #2464. kanso's testing was Go-test shaped by his ruling on
#966, which ended "eventually we'll make something more like rspec". This
is that.

## 2026-11-11 — std/spec: a spec is a test whose value is an effect (#2464)

**The search.** #966 and its entry (the Go-test shape), `std/expect`'s
header (why a matcher answers a record), the two entries above on what
`kanso test` performs and reads, and the lexer's rule for a list that
spans lines.

**The decision**, by the committee, recorded on #2464. A spec is a value
and a test is still a constant. `describe name items` answers an effect
that runs the examples under it, prints them as an outline and yields true
when every one passed, and `kanso test` has performed a test whose value
is an effect since #2458. So the verb gains nothing, and the compiler
still carries no test vocabulary, which is what #966 took out of it.

Hooks are functions around the rest. `around` is handed the value so far
and the rest of the example as a function; `before` and `after` are
written with it. A group's hooks wrap those of the groups inside it,
every example gets its hooks run fresh, and an example is handed what
they built as its one argument, so nothing plays the part of RSpec's
`let`. A group inside a `describe` is `context`: a nested `describe`
would be an effect sitting in a list, which the library could not look
inside.

No syntax was needed. A list that spans lines takes one element to a
line, a line that is a call needs no parentheses, and a line that ends by
opening a bracket spans in turn, so a nested spec is written as RSpec
writes one with the grammar as it stood. The first prototype wrapped each
item in parentheses and was refused at `])`, and the lexer's own comment
on element lines is what showed the parentheses were the mistake.

**Built.** `lib/spec/spec.kso`, shipped as `std/spec`. `std/expect` gains
`contain`, `have_length`, `start_with`, `end_with`, `be_none`, `be_false`,
`satisfy` and `not_to`, which answer an `unmet` record of what was wanted,
in words, and what was got. `expect` and `to` take none, since `be_none`
is a matcher.

`tests/golden/test_verb/nested_spec` is a stack built by `before` hooks
three groups deep, with an `around` that prints a line on each side of
every example. Its golden holds the outline, the two examples made to
fail named by their full descriptions with what they answered, and the
count. One fault was found by it and is pinned by it: an example that
ended in a failure skipped the closing half of its `around`. A failure is
a box, and only an arm that asks for the box is handed one, so the arm
that was meant to catch it inside the hooks never ran and the catch
outside them did. Watched red: that example's `(closed)` line was missing.

The book's modules chapter gains "specs that nest", with a panel the book
check runs. Breaking the panel's expected output turned the check red.

**Rows.** The compiler's table of shipped modules holds one more name, and
the four rows that pass through it worsened: `compile_instructions`
22,652,542 -> 22,652,874, `entry_instructions` 119,166,322 ->
119,166,434, `library_instructions` 63,634,689 -> 63,634,765, and
`browser_compile_instructions` 412,081,305 -> 412,081,336. The welfare
floor is
lowered by what that costs, 95.3923041102071 -> 95.39230375032061: the
directive asked for the library, so the exception for a ruled part of the
language applies.

**Open.** One example cannot be run by name. The outline is printed by
the library and `kanso test` adds its own line for the constant, so a
file's count of tests is its count of specs, with the examples counted on
the line above. senro's hooks, generated specs and guide are next.

## 2026-11-11 — a test beside an entry reads the names the entry binds (#2463)

The first test written for a senro app needed the app as its entry wires
it: `signed`, the route table with the sign-in requirement around it, and
`connected`, the effect that opens a database session. Both are bindings
among the entry's statements, and #2460 lent a test file the entry's
imports and derivations and nothing else. The test would have had to wire
the app a second time, and would then be testing its own wiring.

**The search.** The entries on the test verb (2026-10-12, 2026-10-14,
2026-10-29, and the two above) and on the entry (`an entry names a group`,
2026-10-29; `a lazy binding of an effect, handed back unread, still runs`,
2026-11-05) say nothing on a test reading an entry's names. No fixture
under `tests/golden/test_verb` had a binding in an entry.

**The decision**, by the committee, recorded on #2463. A test file beside
an entry reads the entry's bindings as names. An entry's statements are of
two kinds: a binding gives a value a name, and a bare expression is
performed. An effect is a value, so a binding performs nothing even when
its value is one, and naming it in a test evaluates what the entry wrote.
#2460's "none of the entry's statements run" still holds.

Moving the wiring into a module a test could import was turned down:
every app would be laid out for the test verb's sake, and one that kept
its wiring in the entry, where `senro new` puts it, could not be tested
as wired.

**Built.** In `compile_library_beside`, each `name = value` among the
statements of the entry beside a test file becomes a constant of the test
file's program, stamped with the entry's file. A test file that declares
one of those names is refused at its own declaration and told where the
entry binds it. A pattern on the left lends no names.

Two fixtures. `reads_the_entrys_names` binds a string and an effect that
prints it, and performs the effect; its test file reads the string and
performs the effect itself, and the golden shows the line printed once,
between the two tests. `names_the_entry_bound` pins the refusal. Watched
red with the bindings left behind: `unknown name` for both names in the
first, and in the second the test file's own `front` stood alone and
passed.

Tried on senro's `examples/notebook`: a test file reads `signed`.

**`kanso check` agrees.** A test file was checked by its content, as any
other file is: one holding constants alone was an entry with unused
bindings, and one with a function was a library that knew none of the
names the test verb lends it. So an editor showed `unknown name` on every
line of a test that passed. `compile_source` now hands a `*_test.kso`
file under `check` to `compile_test_file`, which the check verb and the
language server both reach. Watched red on the two fixtures' test files,
which `kanso test` ran and `kanso check` refused.

**Open.** A binding that takes its value apart lends nothing, and a test
that names one of its parts is told only that the name is unknown.

## 2026-11-12 — a dependency's local in a list is not the importer's function (#2466)

senro's spec generator is a new module that declares `fn listed s` and
imports the scaffold generator, which binds a local `listed` below a guard
and holds it in a list. `kanso check` refused the scaffold's list, in a
file the change had not touched: "`listed` takes 1 argument(s), and a list
element is one atom".

**The fault.** `call_shaped_at` asks whether a bare name in a list is a
function that takes arguments, and reads a table built from every
declaration of the module being checked. That module's dependencies are
walked with the same table, so a dependency's local was looked up among
the importer's functions. The walk's bound names would have excused it,
but they hold parameters and top-level binds, and a bind below a guard is
neither. Inside one module the collision cannot arise, because binding a
name a declaration has is refused first.

**Built.** For a declaration that is not the module's own, a bare name is
not looked up in the table. The dependency's own functions are qualified
when it is loaded, and its own check reads its lists against them, so a
bare call in a dependency's list is still refused where it is written.

Two fixtures under `tests/golden/shadowing`. `held` is the postcard from
the issue, run on both engines; watched red with the refusal above naming
`held/pair.kso`. `called` pins that a dependency's own bare call in its
own list is refused at its line, which passed before and after.

**Rows.** Skipping the lookup for every declaration from a dependency makes
three compile rows cheaper, as CI measured them against the main before
std/spec: `compile_instructions` 22,652,874 -> 22,642,731,
`entry_instructions` 119,166,434 -> 119,108,712 and `library_instructions`
63,634,765 -> 63,609,813. `browser_compile_instructions` worsened, 412,081,336
-> 412,081,396. The welfare score rose, and the floor is set to it,
95.39230375032061 -> 95.39235548899775. The check verb's route for a test
file reaches no compile any row measures.

## 2026-11-12 — a spec of one example says "1 example" (#2469)

The home-page spec `senro new` writes has one example, and `std/spec`
ended its run with "1 examples, 0 failed". The summary now counts with the
noun's number, as RSpec prints it: `counted 1` is "1 example", and any
other count takes the plural. `tests/golden/test_verb/one_example` is a
spec of one example; watched red on main with the plural.

**Rows.** `lib/` is compiled into the binary, so the compile rows moved
with it: `compile_instructions` 22,642,731 -> 22,642,405, and two worsened,
`entry_instructions` 119,108,712 -> 119,108,774 and `library_instructions`
63,609,813 -> 63,609,875. The welfare score rose, and the floor is set to
it, 95.39235548899775 -> 95.39235569042921.

## 2026-11-12 — gavel: senro fitted to one way of doing each thing (#2468)

Clay asked the committee to go over senro as one would fit a bicycle:
everything an app needs, nothing it doesn't, and one convention for each
job. The committee's reading is on #2468. It is built from an inventory of
every public name in senro's seventeen modules, the three example apps,
and what the generators write. It rules in eleven sections and orders the
work as six builds.

**Rulings built so far.**

- One name for a notice. The views took `flash` while the server said
  `notice`, and the views now say `notice` too. A scaffold's views are named
  for the plural, `notes_show.kata` and `notes_form.kata`. `visit/sees?`,
  `visit/sent_to?` and `passwords/stand_in_at` leave the public surface.
  The other names the inventory found no caller for outside senro's own
  specs stay public: those specs are how senro is driven
  (kanso-lang/senro#127).
- One call starts an app. An entry is `derive app/conventions`, a
  connection from `pg/from_env`, and
  `app/run "<name>" routes connected derived`. The new `app` module sits
  above `web`, `models`, `jobs` and `mail`, since `web` is below all
  three. A derivation can declare only plain data, and only a record's own
  module may build one, so `derived` is a map, and each migration in it is
  its version and its SQL. The port is `PORT`, 3000 when unset. A mail
  worker starts when the app has a mailer or calls one
  (kanso-lang/senro#128).

**Read again while building, and settled.**

- `models/integer?` and `wire/whole?` were listed for removal. senro's
  specs call both. The search that missed them ended in a word boundary
  after `?`, which never matches. Both stay.
- The resource controller keeps reading a field with `sent[n]` instead of
  `web/value`. A field the request leaves out is left out of an update,
  and `web/value` would read it as "" and blank it.
- The scaffold's refusal arms stay. Dropping them needs a refused write to
  bubble past the arm that reads the record. kanso lets a rescue open only
  a failure another package raised, and `models` and `web` are one package.
  A wrapper in every writing action would trade the arms for a call in
  each action.

Builds 3 (one way to answer and to read a request) and 4 (tables that
supply their id and timestamps) are in review as kanso-lang/senro#129
and its successor. The site follows the senro pin.

## 2026-11-12 — a derived file puts its types first (#2472)

A module's derived file was written in the order its definitions arrived,
so a derivation that wrote a function into a module ahead of another that
wrote a record there made a file the formatter refused: "canonical order
places type declarations before functions; move `thing` up". The app had
done nothing wrong, and the fix it was left with was to reorder its
`derive` lines. senro's pantry carried a comment explaining why
`models/tables` had to run first.

`load_with_definitions` now keeps a module's record types apart from the
rest of its definitions and writes them first. A definition joins the
types when its first line past its imports, blanks and comments opens
`type` or `pub type`. Within each half the definitions keep the order they
arrived in, so a file that was accepted before is written the same way.

The spec is `tests/golden/derive/a_type_defined_after_a_function_comes_first`:
one derivation writes `shown`, which takes a `thing`, and a second writes
the `thing` record and its maker. Without the fix both engines refuse the
derived file with the formatter's message above; with it they print
`thing 3`.

## 2026-11-12 — a module's own arm answers its bare call (#2473)

senro's notebook declares `fn id_of` in a module that imports `tokens`,
which exports an `id_of` of its own. A bare `id_of` there calls the
module's own function, and both engines run it, but the check refused the
caller: "this can be a none and `wanted` has no arm for it". The import's
`id_of` can answer none, and the module's own never does.

**The fault.** A name a module declares and one of its imports exports is
one overload group inside the module, and dispatch tries its arms in
order. `qualify` puts the module's own arms ahead of the import's, so an
importer runs them in that order. The module's own check merges its
dependencies in first, so there the import's arm stood ahead. Inference
then read the call's answer as the union of every arm the arguments could
reach, the import's included, and the none question read that union.

**Built.** Two changes, each needed. Where a module and its dependencies
are merged for the module's check, an import's twin of a name the module
declares goes in behind the module's own arms, as `qualify` orders them.
Inference stops at the first arm whose parameters are all binders: a later
arm answers that call only for a failure it names. A call that passes over
arms is marked by the arms it reaches, through `none_free`, the per-call
mark `math/round` already uses.

`tests/golden/own_arms_first` is the postcard from the issue, run on both
engines by `tests/own_arms_first.rs`. It was watched red with each change
taken out alone, both times with the refusal above.

**What the first push cost.** CI's reading of the first push put the
front end's peak on `compile_corpus` up 12,643 bytes (546,568 to 559,211)
and its allocations up 28. Built locally with each half of the change
alone, the merge carried all of it: it split the dependency's functions
with `partition`, which moves every one into a new vector while the old
one is still held. `extract_if` takes out only the twins that go behind,
and CI's second reading put the peak 84 bytes above main's (546,652).

The instruction rows stayed up about 0.7%: `compile_instructions`
22,642,405 to 22,800,026 on that reading. A callgrind diff of the check
of `compile_corpus` against main split the 159,052 instructions between
the two halves. The merge spent about 64,000 building a set of every
function name the module declares, and inference about 84,000 testing,
on every visit to every call, whether each arm's parameters were all
binders. Both are facts about the declarations. Which arms a call passes
over is now worked out once per group when inference starts, and the
merge looks up the larger of the two name lists in a set of the smaller
and keeps only the names both hold, which most modules have none of.
Locally that took the check from 159,052 instructions over main to
138,754, for 174 bytes more peak, which `welfare --score` prefers by a
hair. What is left in `eval_expr` is the new branch in the call loop,
which the precomputed table does not remove.

**Rows, from CI on 5c30213f8.** Every row below rose, and none of them
counts a program running: `compile_instructions` 22,642,405 to
22,778,659, `entry_instructions` 119,108,774 to 119,972,835,
`library_instructions` 63,609,875 to 64,026,941, `compile_peak_bytes`
546,568 to 546,826, `compile_allocs` 13,503 to 13,508,
`emit_instructions` 5,632,125 to 5,636,195, `interp_instructions`
741,096,980 to 741,118,066 (the interpreter's run includes its check of
the corpus), `interp_allocs` 1,067,941 to 1,067,977 and
`browser_compile_instructions` 412,081,396 to 412,743,106.
`browser_run_instructions` fell from 100,428,884 to 100,428,858. The
runtime veins did not move. The check now reads the program as it runs,
so the floor is lowered by what that costs, to 95.391044, under the rule
for the language working as specified.

## 2026-11-12 — gavel: senro's fit, builds 5b to 6 (#2468)

The rulings the committee made on #2468 after the first entry, each built
and merged in senro.

- **Templates name routes as actions do** (senro#132). A template writes
  `a href=(path_to "notes/show" r.id)`, and kata writes the path out while
  it derives the view, from the table `routes` builds, so the browser's
  views module imports nothing from the server. A name that is no route,
  or the wrong number of ids, is refused at the template's line. A paging
  link names only its query, `?page=2`, and `img src=(blob_path e.photo)`
  is written from `web/blob_path`.
- **One form of `path_to`** (senro#133). The record form, `path_to note`,
  found an arm only for a record declared beside a `show` action, and a
  scaffold keeps its record in the model's module. It goes, with the arms
  `rest` wrote for it. A path is named by the route's name and its ids.
- **The secret is found, not passed** (senro#134). An action is handed a
  `web/request` that carries the app's secret beside the parsed request.
  The secret is `pub secret` in the session module, which `routes/rest`
  puts in each row of the route table, and `SENRO_SECRET` replaces it when
  set. Twelve functions lose the argument. An app that keeps sessions and
  declares no secret is refused at the call. A secret never rides in a
  header, where an app that echoes headers would print it.
- **Signing in is one call** (senro#134). `accounts/attempt` checks the
  password, keeps the lock, counts wrong and right passwords, and answers
  `signed`, `inactive`, `locked` or `wrong`. Whether an account may sign in
  is a field, `active`, on the record the app builds, as Devise asks the
  model. The pantry writes through `item/update`.

Two fixes in kanso came out of building these: a module's own arm answers
its bare call ahead of an import's twin (#2473), and a derived file puts
its record types first (#2472). senro pins kanso d582d16c, which carries
both (senro#136). PRINCIPLES.md gained the section the review found
missing, "One way to do each thing" (senro#135).

## 2026-11-12 — senro answers a component's action as its page (#2455)

The last two items owed on #2455 are built. senro's design is cloud's (#2280), so the shape below is recorded here rather than put to the committee.

Auth in the framework: `senro generate authentication` writes confirmation by mail and the unlock link (senro#137, #138, #139). `accounts` signs both links, an account signs in once its address is confirmed, and the wrong password that locks an account mails the link that unlocks it. A test reads the mail its requests queued with `visit/mailed`. The guides followed in #2480.

Authorization at the page: a component's server action is now answered as the page it was posted from, as a provider call already was (senro#140). The page's script sends its path with the action. The server answers only from a page whose module leans on the component's, behind that page's sign-in, and only for a viewer its `admits` lets in. `derive kata/views` writes a `component` record into each server component's module, with the modules that lean on it, and an app lists them: `web/components routes [staple/component]`. That replaces `web/component act view`, which named every component's action and view by hand. The record's type is declared in the component's module, with no import, so a browser build of the module carries nothing of the server's. spec/serve/admitted acts on a tally from each page; with the check removed, its four refusals answer 200.

Rows: senro's bench/pages moved in CI. server_records went from 344,151 to 344,207, server_data from 1,130,160 to 1,130,145, and server_html from 3,087,836 to 3,087,821; the browser rows held. The bench's component runs in the browser, so it gets no record. The likeliest cause is kata/views now importing `data`, which has not been isolated. No kanso row moved.

Open: browser components still go through `browser/component act view`; giving them the same derived record is a small follow-up.

## 2026-11-12 — a literal applied as a function is refused where it is written (#2479)

A list written across lines takes one element per line, so a line holding
two strings reads as the first string applied to the second. No literal can
be called, and `kanso check` passed the line; both engines failed with
"`confirmations` is not callable" when it ran. In senro the same shape, in a
generator's list of modules, came out natively as an error naming an
unrelated function.

The checker now refuses a string, number, list or map at the head of an
application, at the literal's line and column. Inside a list the message
says the list takes one element per line:

```
error[type]: this element of the list is a string applied to 2 arguments, and a string cannot be called. A list written across lines takes one element per line
```

Elsewhere it reads "an int cannot be called, and here it is applied to 1
argument". A value called through a name, as in the runtime corpus's
`a_plain_value_is_not_callable`, still checks clean and fails when it runs,
because its value is not on the page. Two error goldens pin the refusal: the
issue's list and `x = 3 4`. main's binary checks both as `ok`.

**Rows.** The first cut ran the check as a pass of its own at every node and
cost the entry corpus's check 184,858 instructions in CI. The list case now
rides `call_shaped_at`, which already walks a list's elements, and the call
case rides `literal_argument_at`, which already takes an application apart.
What is left is the price of the refusal: `compile_instructions`
22,778,659 -> 22,780,737, `entry_instructions` 119,972,835 -> 119,985,418,
`library_instructions` 64,026,941 -> 64,031,825 and
`browser_compile_instructions` 412,743,106 -> 412,766,768. The check
refuses a program the language says is wrong, so the floor is lowered by
what it costs, under the rule for the language working as specified, as for
#2473: 95.39104366 -> 95.39101636. No runtime row moved. The compiler page
has a section for it, §303.

## 2026-11-12 — a browser component is listed by its record (senro#141)

The follow-up the last entry left open. A server component was listed by
the record `derive kata/views` writes into its module, and a browser
component was named by hand, `browser/component planner/act planner/view`,
under a name typed again in a map. kata now writes the record for every
component, and a browser module lists them as an entry lists its server
components: `browser/answer_pages [planner/component] views/rendered`.
`browser/component` is gone. A browser component's record names no pages,
since its actions never reach the server.

The record names the one template that draws a component's state, so a
component with two templates that bind actions is now refused on both
sides, where only a server component was before.
`spec/errors/component_drawn_by_two` is a browser counter with two such
templates; senro's main checks it clean.

Rows: senro's bench/pages moved. The first cut folded the list into a map
before the module read its event and read about 7,000 instructions more on
every browser row. Looking the component up when its event arrives, and
recording no pages for it, brought them to browser_start 13,643 -> 16,873,
browser_page 3,667,266 -> 3,670,780 and browser_act 115,229 -> 119,357, with
server_data 1,130,145 -> 1,130,451 and server_html 3,087,821 -> 3,088,133.
The derived code differs from main's only by the record and its type, and
taking out `std/list` or `browser`'s own type left start-up where it was, so
what remains is building a record of five fields where the module built one
of two and a map. No kanso row moved. The site's views guide quotes the new
lists, and the pin is d247faa.

## 2026-11-12 — gavel: the analysis, and no lint, carries a handed-out function's types (#2433)

A function handed out as a value has its parameters sent to every type,
because a value can be called with anything. A decoder whose position is
then not known to be an int keeps its record on the heap for its direct
callers too. Today the author's remedy is `p:int`, and nothing says it is
missing. The issue offered two directions, and the committee took the first.

**The lint is declined.** check.rs says kanso has no linter layer and refuses
what a linter elsewhere would warn about. A program without the annotation is
correct and only slower, so a refusal would refuse a program that runs.
Knowing that an annotation would change the answer also needs inference to
keep the direct callers' join apart from the value's, which is most of the
fix.

**Decision.** Inference keeps a handed-out group's types as its direct
callers prove them, and the value's wrapper takes anything. Under the
corpus-first gavel, the baseline lands first:
`a_decoder_handed_out_without_an_annotation_boxes_every_read` is
`a_decoder_handed_out_still_answers_its_callers_by_value` with `:int` taken
off `at`. It pins `sh_rec=3200` and `allocs=57` against its twin's 512 and
15, which is the issue's table. The fix scores as that fixture falling to its
twin. No other golden moved, and welfare does not read the mem vein. The
issue stays p3 because no benchmark pays this today (the measurement on
#2433, 2026-11-11).

**Copying the group does not reach it (measured the same day).** The
cheapest form of the decision is a source pass beside `flat::flatten`. It
copies a group that is both handed out and called from elsewhere, and sends
every mention by value to the copy. Built that way, the pass chose `at` and
the fixture still read `sh_rec=3200`. Inference joins a record type's fields
over every place that builds one. The copy builds `read bs[p] (p + 1)` with a
position of any type, so `read.next` widens, and `summed` hands that position
back to the direct `at`. No other mem golden moved, so the copy would only add
compile work, and it was not kept. What the fixture needs is for a function
value's parameters to hold what is actually passed to it at the calls made
through values, in place of every type. That is a flow analysis of function
values.

## 2026-11-12 — the dev tier stays at -O0 (measured)

The dev tier compiles a program at `-O0` and links the cached optimized
runtime. Its run of runbench costs three times the release build's, and
`dev_run_instructions` has improved least of any term (+13% on its
baseline). So I measured what optimizing the program module would buy, on
runbench's dev `.ll` (main at a759d16b, this container, clang 19.1.1). The
codegen column is the tool's own instructions on that one module.

| pipeline | codegen | run |
|---|---|---|
| `clang -O0` (today) | 2,886,553,773 | 3,057,358,794 |
| `clang -O1` | 14,923,691,533 | 1,847,473,765 |
| `llc -O0` | 2,891,930,318 | 3,148,019,518 |
| `opt -O1`, then `llc -O0` | 4,862,823,427 + 4,302,492,751 | 2,782,998,016 |
| `opt` with sroa, early-cse, instcombine and simplifycfg, then `llc -O0` | 1,103,876,794 + 2,815,114,850 | 3,267,742,793 |
| `llc -O1` (the same at `-O2`) | 7,474,749,318 | 2,365,328,093 |
| `llc -O0 -fast-isel=false` | 3,511,909,510 | 3,350,254,256 |

Most of what `-O1` buys comes from the backend: selection DAG and the greedy
allocator in place of fast instruction selection. A middle-end-only pipeline
recovers a fraction, and the short pass list makes the run slower. The
cheapest pipeline that moves the run is the backend at `-O1`: 25% off the run
for 2.6 times the codegen. At today's ratios that is about +0.007 on the
development side from `dev run speed` against about -0.017 from `dev
codegen`, whose satiation is 0.5. Plain `-O1` is worse, at -0.025. Under the
weights as written the dev tier stays at `-O0`.

## 2026-11-12 — gavel: senro ships as releases

Asked whether every senro change was on the site, the audit found the guides
level with senro's main except in one place. senro's README told an app to
get senro with `kanso install`, and that command refused it:
`kanso-lang/senro publishes no vX.Y.Z release tag`. `kanso install` resolves
a hako's highest release tag, and senro had none. Pinning to the branch with
`--from kanso-lang/senro@main` worked, and the README's example app checked
clean against what it fetched. Clay, 2026-11-12: "it definitely needs
release tags."

So senro is released. A session's push of a tag is refused with HTTP 403,
so senro tags itself: `VERSION` names the release, `0.1.0` to start, and a
workflow on each push to main tags the commit that first carries a version
(senro#143). Its first run tagged `v0.1.0` at dba26a72. A plain `kanso
install .` in a fresh app now prints `kanso-lang/senro v0.1.0`, and the
README's example app checks clean against it. senro#142's README says
`kanso install` fetches the newest release.

CLAUDE.md gains the rule that a merge changing what an app writes or runs
raises `VERSION`, and that the site's `senro-version` names a tagged commit.
The site now pins dba26a72. The command-line guide gains a section on
naming senro by repository, and its summary of `authentication` now names
the confirmation and unlock links senro#139 added.

## 2026-11-12 — directive: legacy route aliases are code

Clay, reading the routing guide: "I would prefer a legacy route aliases to
use code rather than the strings so that they are analyzed that compile time
obviously. get, put etc should be methods." An alias was a string map,
`{ "GET /signbook":"entries/index" }`, so the compiler saw neither the
action nor the method.

senro#144 makes each alias a call, `(routes/moved "/signbook"
entries/index)`, in the module's `legacy_route_aliases` list. The action is
a name, so the compiler resolves it. `routes/rest` reads each call through
the module's `calls`, as it reads `path_to`, and refuses a mistake at the
call. The methods are not functions. `put` and `delete` are taken by the
built-ins (`error[name]: the name put is already taken`), so two of the five
cannot be written, and an alias's action already fixes its method: the alias
takes the method of its action's route. The old map form is refused at the
declaration. senro's `VERSION` goes to 0.2.0 with it, since an app on 0.1.0
must rewrite its aliases.

The same question asked how senro spells member and collection routes,
deeper nesting and the rest of a Rails routes file. The routing guide gains
two sections. "actions beyond the seven" shows a member action as a nested
singular resource (`publication`, with `create` and `destroy` taking
`post_id`) and a collection action as a resource of its own (`searches`),
quoted from senro's new `spec/runs/actions_as_resources`. "what senro's
routing does not have" lists namespaces, renamed paths, nesting past one
level, constraints and hand-declared routes. A Readwise article on `derive`
and the conventions built on it went to Clay's reader.

## 2026-11-12 — the runner image moved to glibc 2.39-0ubuntu8.12

The cost-goldens job on kanso#2493, a docs change, refused every glibc-keyed
gate: the rows were measured on glibc 2.39-0ubuntu8.9 and the runner now
carries 8.12, with clang 19.1.1 and rustc 1.98.1 unchanged. main's last run
was on 8.9 and green, so main would refuse the same way if it ran again, and a
branch opened to carry the re-measure lands on the same new image. It rides
here, as the 8.8 -> 8.9 re-measure rode with its PR in September.

CI measured every row on 8.12 and none rose. Of the fifteen work rows six are
byte-identical and nine fell by one instruction (jsonbench, encodebench,
oneshot, widebench, digestbench, readbench, livebench, runbench and
runbench_dev). `codegen_instructions_dev` fell 117,027,868 -> 117,027,866 and
`codegen_instructions_release` 372,208,803 -> 372,208,799. The compile,
entry, library, start-up, interpreter, interpreter-memory and emit rows read
exactly their goldens. The rows are copied from the job log and the ten
measured-on lines name 8.12. The welfare floor is re-based by the rise the
revision gives, as a measurement of outside state (#2017), and its history
says so.

## 2026-11-12 — the book gains a chapter on derivations, with senro as its case study

Clay asked whether derivations had solid documentation. The book had one
section in the modules chapter: a routing example and three dense paragraphs.
Chapter 16, "conventions at compile time", is now the home. It builds a
command registry from nothing (`docs/book/samples/derive`, run and refused
by book_check), lists everything a derivation is handed and the five
declarations it may answer, states the rules it runs under, and reads four of
senro's derivations (routes, links and aliases through calls, records from
the schema, templates, job names) from senro's own source. A last section sets
it beside Haskell's deriving and Template Haskell, Rust's derive and
procedural macros, F# type providers and Zig's comptime. The modules chapter
keeps its example and points to the new chapter, so the details have one
home.

The senro excerpts are titled `senro/<path>`, which no file in this
repository holds, so book_quotes leaves them alone. senro_quotes now reads
`docs/book/derive.html` as well as `docs/senro`, checking only its `senro/`
panels against the pinned senro commit. Watched red with one word changed in
the `fn refused` excerpt, which it named by page and title.

The idea's origin is the #2186 ruling, 2026-10-28: Clay asked for "some
language construct that can allow you to do compile time mapping" so that
senro could place routes and tables by name and the compiler could check the
result.

## 2026-11-13 — senro 0.3.0: association readers take the session and are named for the key

Clay asked on #2497 for `has_many`-style associations that read naturally,
with `has_one` apart from `has_many` and room for legacy names (`owner_id`
holding a user). The committee's first reading named the readers
`user/posts` and `post/owner`. That puts a reader in the module that does
not hold its rows, so each model module would import the other, and kanso
refuses an import cycle. The decision, recorded on the issue, keeps each
reader in the module whose rows it answers and names it for the key's
column without `_id`.

senro#145 builds it, and senro tagged v0.3.0 at its merge. A reader takes
the session alone, which already names the viewer: `page/of_note db note`.
`notes.author_id → writers` gives `note/of_author db writer` and
`writer/author_of_note db note`; the parent's reader names the column
whenever the column is not named for the parent, so two keys to one table
read apart with nothing declared. A key column with a unique index answers
one row or none, as Rails' `has_one`: `cover/of_note db note`. senro's
`spec/db/associated` pins all of these on both engines against Postgres.

The site pins senro 0.3.0. The models guide teaches the new names and quotes
the extended spec, and the migrations guide quotes its migrations.
Declarations for keys the schema cannot describe wait for an app that needs
one.

## 2026-11-13 — a lambda's body may span lines

Clay, on #2498: "taking an indented body shouldn't be a special function for
it/context/etc. it's just a standard thing ... and the body would have to be a
lambda." Until now a lambda's body was one expression, and lines indented
under `(x ->` were read as continuation lines of that expression, so a binding
among them was refused as a needless continuation. #2498 had made `it "…"` and
`context "…"` block headers to get round that. The committee's decision on
#2495 was to make the body general instead, and this entry builds it.

A lambda whose `->` ends its line takes the lines indented under it as its
body. The lexer folds them into one `Tok::Body` token on the opening line, and
the parser reads that token as an `Expr::Block`, the node an `if` block already
uses, so every pass and both engines run it the way they run a `fn` body:
bindings, then the answer. A `)` that starts a line at the opening line's
indent closes the lambda, and the rest of that line continues the statement,
so `))` and `) .> …` both work. Bodies nest.

A block lambda may also be an element of a list that spans lines, which is
how a spec file writes `it "…" (s ->` among the items of a `describe`. The
list gatherer reads raw lines before there are any `Line`s to fold, so it
lexes the body lines as a file of their own, moved left to the margin, and
moves their spans back. A body there holds anything a body at the top level
holds; `a_lambda_body_in_a_list` nests one block lambda inside another and
puts a comment in the outer one. A body line shallower than two spaces under
its element is refused with its own error golden.

Three refusals, each with an error golden. An arrow that ends its line with
nothing indented under it, and a body that is not closed by a `)` at the
opening indent, are syntax errors. A body of one line that would fit on one
line with its head and close is refused as formatting, for the same reason a
short list may not span lines; a comment or blank line inside the body keeps
the spanning form. The micro golden `a_lambda_body_spans_lines` runs a block
lambda and a nested one on every engine, and the values chapter teaches the
form with `long_lambda.kso`.

Building it turned up an older bug. The lexer dropped a comment line only on
the path for top-level lines, so a comment among the lines of an `if` block
reached the parser as an empty line and was refused as "expected an
expression". It now drops a comment wherever an indented line is read. The
micro golden `a_comment_among_block_lines` puts a comment in an `if` block and
in a lambda's body; on main it fails at the first.

The lexer and parser grew, and four rows rose with them, all read from CI:
compile_instructions 22,780,737 -> 22,843,804, entry_instructions
119,985,418 -> 120,154,980, library_instructions 64,031,825 -> 64,189,750,
and browser_compile_instructions 412,766,768 -> 412,895,722. The front end
now looks at the last token of every line and every list element for an
arrow, which is where the extra work goes; no corpus file holds a block
lambda yet. No runtime, emitted or codegen row moved. The welfare floor
drops by that cost, 95.39101637 -> 95.39075120, under the language
exception. Open: #2498 is rewritten on this, with `it`
and `context` as ordinary calls taking block lambdas, once Clay says whether a
context may restate a value that setup above it reads.

## 2026-11-18 — src/main.rs in four files

`src/main.rs` had reached 3,011 lines, holding three groups of functions that
call each other only at their edges (#2502). Each now has its own file under
`src/main/`, beside `tail_calls.rs`: `toolchain.rs` probes the host's clang
and linkers and builds padded temp-path names, `clang.rs` drives the two build
tiers and caches the runtime objects they link, and `execute.rs` runs a built
program for `kanso run` and `kanso play`. `main.rs` keeps argument parsing,
the REPL, `build`, `test` and the dispatch, in 1,184 lines.

The files are read with `include!`, as `tail_calls.rs` has been since
kanso#1848, so every item stays in the binary's root module in its old order.
Each test module moved with the code it covers. Two things changed besides
location. The hot unit's recipe digest, `HOT_RECIPE`, now digests
`clang.rs`, which holds `hot_source_of`, where it used to digest all of
`main.rs`. Six specs read the binary's source as text, so they now read
`main.rs` and every file under `src/main/` through `tests/support/driver.rs`.
Each scanning spec went red on a line planted in `execute.rs`. Ten ratchet
mutations point at the file that now holds their line.

## 2026-11-18 — the runner image is pinned to ubuntu-24.04

The cost-goldens job on kanso#2503, two sentences of page prose, refused every
row: `ubuntu-latest` now resolves to ubuntu-26.04 (image 20261111.196.1),
with glibc 2.43-2ubuntu2.7 and clang 19.1.7 where every measured-on line names
glibc 2.39-0ubuntu8.12 and clang 19.1.1. Every Linux job on that run and on
kanso#2504 landed on 26.04. A new release of the OS moves the dynamic loader,
libc's string routines and the C compiler together, so taking its rows would
credit the compiler with the image's work.

The image is outside state, and it is now fixed the way rustc was on
2026-10-04: every Linux job in every workflow names `ubuntu-24.04`, the
release matrix included, and `tests/the_runner_image_is_pinned.rs` refuses a
`runs-on:` or matrix `os:` that names another Ubuntu label. The spec fails on
`ubuntu-latest` in either place. No row is re-measured and the floor does not
move. The goldens were measured on 24.04, and the patch updates that image
still takes are re-sat as on 2026-11-12. Moving to 26.04 is a pull request of
its own that moves every `runs-on` and re-sits every row on the new image.

## 2026-11-18 — the work and codegen rows count from libc's entry

With the image pinned, the cost goldens still refused on two pull requests
that changed no compiler code, on work, dev codegen and release codegen. One
re-run drew the same chip and the same three rows. Main's last green run was
on Intel model 0xcf and image build 20261111.370.1; the red runs were on
Intel model 0xad and build 20261111.127.1. Every work row read exactly 130
lower, codegen dev 1,098 lower and release 3,092 lower. The work tables of
the two runs differ in two functions and no others: `_dl_cache_libcmp` (146
fewer) and `_dl_load_cache_lookup` (16 more). That is the dynamic loader
searching `/etc/ld.so.cache` before `main`. The compile rows, anchored at
`kanso::main`, read the same on both.

The silicon and the image build changed together, so this does not say which
of them moved the search. Either way the loader's search is the runner's, so
`instructions.sh` and `codegen_instructions.sh` now pass
`--toggle-collect='__libc_start_main*'` and count each process from libc's
entry. `scripts/gates/past_the_loader.sh` runs in the cost-goldens job. It
counts a three-line C program with and without `LD_LIBRARY_PATH` set: counted
whole, the program reads 114,160 and 114,600 in CI, and from libc's entry it
reads 1,680 both times. The gate also checks that both gates pass the flag on
every callgrind they run. It goes red when the flag is dropped from either
gate, and red when the flag is weakened so that the loader's work is counted
again.

Every row moved down, taken from CI on AMD family 0x19 model 0x1. All fifteen
work rows fell by 149,051, the loader's start-up for one process.
`codegen_instructions_dev` fell 117,027,866 -> 85,822,279 and
`codegen_instructions_release` 372,208,799 -> 319,811,536. Those also lose
the relocation of libLLVM and LLVM's static constructors in every clang
process, which run before `main` and do not depend on what kanso emits. No row
rose. The welfare floor is re-based 95.39 -> 95.46 as a measurement that
stopped reading outside state (#2017). kq's instructions vein had the same
-130 on kq#246 and gets the same flag in kq#248.

## 2026-11-21 — gavel: a spec binds as every program binds

#2496 asked whether `x = e` in a spec should name what an effect yields, so
that setup reading a database could be written without `.>`. Clay ruled no:
"there is certainly no reason to treat things differently in specs ... if
you think there's a better way to deal with our monadic properties certainly
suggest it but there's nothing magical about tests that means we should just
throw out our principles."

The 2026-08-29 gavel, "effects are types, and the words are the only doors",
holds in spec files as everywhere else. `x = e` binds what `e` is, and only
`.>` and its kin take a value out of an effect. If binding an effect's value
reads badly in a spec, it reads badly in every program, and an improvement
goes to the language through the committee.

## 2026-11-21 — gavel: the committee designs `let` for specs

#2495 builds specs as blocks, and a spec needs RSpec's `let`: a value named
once for an example, which a nested context may restate. Clay left the design
to the committee, with two conditions. It uses no spec-only syntax or
binding, which the ruling above already requires. And it should be as plain
as derivations made convention over configuration. His lead: a context
answers a map of its givens, merged over its parent's, since a nested example
is the parent's context with some names restated. The committee weighs that
lead first and records its design on #2495 before anything is built through
`std/spec`.

## 2026-11-21 — gavel: JavaScript is solved now, and senro manages its packages

#2284 had been parked until an app needed a JavaScript library. Clay
unparked it: senro will need charts, maps and the like, so it is solved now.
He named a fourth option beside the three on the issue: kanso renders data
attributes, and the app's own JavaScript initializes the library at that
boundary. He called it "certainly a defensible argument" and possibly the
more elegant one. Separately, he asked for JavaScript dependency management
modelled on his jspm-rails, which traces an app's modules with jspm's
generator and vendors them under an import map instead of hand-written pins.

The committee chose the fourth option and wrote its reading on #2284. A
senro component's browser code already never touches the DOM: it answers
HTML, and the page script morphs it in. Each action also runs in a fresh
instance of the module, so kanso holds nothing between events that could
name a chart. senro#146 builds the dependency management (`senro js install`
and `senro js check`) and a morph that leaves a library's element to the
library. The way back, a library's event becoming a component's action, is
designed next under the same issue.

## 2026-11-21 — std/spec gains givens, and a context overrides them by name

Under the ruling above, the committee designed `let` on #2495 and it is built
in `std/spec`, with nothing added to the compiler. `given name f` is a hook
that hands `f` a reader for the givens so far and adds `name` with what `f`
answered. An effect is opened first, so a later given reads its value.
`around name wrap` puts what its hook hands on among the givens. When a
group's hooks join the ones around it, a hook naming a given that an outer
one named takes that one's place in the list. So a given declared below it
reads the context's value, as a subclass's method reads an override. An
example under givens is handed the reader.

Two shapes were tried and refused by the language before the reader. A map
read by field, `g.user`, is refused at check ("no record type has a field
`user`") or at run time ("`.` reads a field of a record"). A map read by key,
`g["user"]`, answers a value or `none`, and a function with no `none` arm
refuses it. A lambda's answer carries no such mark, so `g "user"` reaches
ordinary functions as a value. The design comment on #2495 said `g.user` at
first and is corrected there.

`tests/golden/test_verb/givens_spec` pins the override, an effect given made
once per example, a named `around`, and a context that adds a given reading
the ones above it. `givens_run` performs the same spec as an entry, natively
and interpreted, to one golden. Watched red with a restated given appended
instead of put in its parent's place: the admin was greeted as "hello, ada".
The book's modules chapter gains `seating_test.kso`. Laziness, `super` and
injection by parameter name are declined on the issue, with reasons.

## 2026-11-21 — senro 0.4.0 and 0.5.0: JavaScript through the page

The committee's reading on #2284 chose the boundary Clay proposed: an app's
own JavaScript reads the data attributes its views render and drives the
library, and the language gains nothing. senro built it in three pull
requests, and the site gains a guide, `docs/senro/guides/javascript.html`.

- senro#146 (0.4.0) manages packages as Clay's jspm-rails does. `senro js
  install` traces `javascript/application.js` with jspm's generator, which
  the app pins in its own `package.json`, vendors what the trace reaches into
  `vendor/javascript/`, and writes `importmap.json`. `senro js check` traces
  and downloads again and fails on any drift. `/senro.js` carries the map,
  so a layout needs no tag. The morph keeps the children of an element
  marked `data-owned-by`. The pantry draws its counts with Chartist, and its
  browser spec went red at "counted" with the morph's rule taken out.
- senro#147 pinned senro's runner to ubuntu-24.04. `ubuntu-latest` had moved
  to 26.04, where Playwright 1.56.1 will not install browsers, so every job
  failed before a spec ran.
- senro#148 (0.5.0) is the way back. The page script listens for every event
  a `data-on-*` binding names, and an action that names `value` is handed the
  event's value as text. It is pinned on native, the interpreter and wasm,
  for a server action, and in the browser, where the spec went red at the
  CustomEvent step with the detail dropped. The browser module pays for the
  value line: senro's `bench/pages` reads `browser_act_instructions` 119,357
  -> 119,782 and `browser_page_instructions` 3,670,780 -> 3,670,822, from CI.

## 2026-11-25 — senro 0.5.1, and the guides catch up with it

Three senro pull requests followed the JavaScript work.

- senro#149 kept one reader for an event's value, `data/value_of`, where
  `web.kso` and `browser.kso` had each held the same three arms. Its first
  form decoded the JSON itself and gave `data`, which every app imports, an
  import of std/json, and CI's `bench/pages` moved on every row. With the
  callers decoding, only `browser_page` moved, 3,670,822 -> 3,670,780, its
  value before senro#148. The same pull request makes a grep in
  `spec/generate/drive.sh` read /dev/null when it is handed no file: with an
  open standard input the spec had waited on it for nine minutes.
- senro#150 moved senro's kanso pin to 41a98055, the givens of #2495. Every
  spec passed and no golden moved.
- senro#151 (0.5.1) adds `importmap.json`'s modification time to
  `senro-version`, so a deploy that changes only the map reloads open pages
  once, as a new browser module does. The pantry's browser spec timed out
  waiting for the reload with the map left out.

The site's senro pin moves to v0.5.1. The deployment guide now says to ship
`javascript/`, `vendor/javascript/` and `importmap.json` with the binary and
that a deploy reloads open pages once; the JavaScript guide says the same of
a new map; the testing guide gains givens, with an example that was run as a
spec in the generated notebook before it went on the page.

## 2026-11-26 — gavel: an edited migration warns, and the schema check decides

senro refused to start when a migration's SQL had changed since it ran.
Clay: "i don't know that you need to prevent the app from starting. it's
just that if you were to try to run the migrations again, you might end up
with a new schema. i mean, be smart about it but i think that's the better
path." A migration that has run is never run again, so an edit cannot change
the live database; the harm is a database built afresh from the edited files.
senro#152 (0.5.2) makes the edit a warning that names the migration, with
what ran and what the file says now, and leaves the comparison `app/run`
makes against the live tables as the gate. `spec/db/migrated` pins both
cases on both engines: a respelled type warns and serves, a changed type
warns and is refused at the comparison. The migrations, models and
deployment guides say so. Recorded on #2509.

## 2026-11-26 — senro 0.5.5: sound that seeks, likes held once, counts

Three gaps a music app would meet, shipped as senro 0.5.3, 0.5.4 and 0.5.5,
and the site's guides moved to 0.5.5.

- senro#153 (0.5.3). A stored file answers `Range` with 206 and the span, a
  span past the end with 416, and several spans with the whole file. Sound
  and video are served inline, where an MP3 went out as a download that an
  `<audio>` element would not play. The uploads guide quotes the new golden.
- senro#154 (0.5.4). `models/add_unique_index` takes a list of columns, so a
  like is held once per handle and track. A refused write puts the problem
  on the last column, as Rails' `scope` does, and a `schema.sql` dump's
  index over several columns now counts.
- senro#155 (0.5.5). `like/count_of db "track_id" id` and `like/counts_by db
  "track_id"` count what a read would show: Postgres counts when the model
  has no `visible?`, and otherwise senro counts the rows the policy shows.
  With the policy check broken in the spec, a visitor's count of likes rose
  from 2 to 3, eve's hidden one leaking. The models guide covers both.

## 2026-11-26 — std/net/http names 206 and 416

senro#153 answers Range requests for stored files, so a player can seek in a
sound, and its golden showed `HTTP/1.1 206 ` and `HTTP/1.1 416 ` with no
reason, because `reason_of` in `lib/net/http` had no arm for either code.
HTTP allows an empty reason, so browsers read those lines; they now say
"Partial Content" and "Range Not Satisfiable".
`tests/golden/micro/a_partial_answer_names_its_status` pins 200, 206 and 416
on both engines; before the change it read `"206 "` and `"416 "`.

The phrases were 24 arms of `reason_of`, and two more arms raised
compile_instructions by 314 and entry and library by 120 each, by layout,
since neither corpus imports `std/net/http` and `lib/` is compiled into the
binary. As one map, `reasons`, read by a single `reason_of`,
compile_instructions is back where main has it, and the other two rise by
layout: entry_instructions 120,154,854 -> 120,154,980 and
library_instructions 64,189,624 -> 64,189,750, as CI measured. The
floor is lowered by that cost under #2280, as it was for `os/random_bytes`
and `net/peer`, senro's earlier prerequisites.

## 2026-11-26 — senro 0.5.7: package stylesheets, and files in a bucket

Two more gaps a music app would meet, and the site's guides moved to 0.5.7.

- senro#156 (0.5.6). An app's stylesheets live in `stylesheets/` and may
  import a package's by name, `@import "bootstrap/dist/css/bootstrap.min.css";`.
  `senro js install` vendors the file with its source map, fonts and images,
  and the map's `stylesheets` holds where it came from; senro rewrites the
  import to the vendored copy as it serves the stylesheet. The pantry's
  chart takes chartist's stylesheet this way. The JavaScript guide covers it.
- senro#157 (0.5.7). `SENRO_STORAGE=s3://<bucket>` keeps uploads in an
  S3-compatible store such as Cloudflare R2: an upload is a presigned PUT,
  and a download a redirect to a presigned GET, so the store sends the bytes
  and answers a `Range`. senro's new `objects` module signs with AWS's
  Signature Version 4. It reproduces AWS's published example and agreed
  with botocore on 220,000 random cases; `spec/buckets/kept` runs against
  moto's S3 server, which senro's CI now installs. The uploads guide covers
  it.

## 2026-11-27 — a derived import into a hako names the hako

The first app built on a released senro, shirabe, failed to check:
`album.derived.kso:1` held an import of
`../../../root/.hako/kanso-lang/senro@<sha>/models`, 93 columns, and the
formatter refused it. A derivation writes its arm's imports
from its own module, and `reread` rewrote each relative one as a path from
the module the arm joins. For senro fetched into `~/.hako` that path climbs
out of the app and back down into the cache, and its length is however far
apart the two are. senro's own specs import senro by relative path, so none
of them reached the cache.

`reread` now writes a path that lands in the hako cache by the hako's name,
`kanso-lang/senro/models`, which is how the app itself imports it and which
the lock resolves. `tests/hako.rs` gains
`a_derived_import_into_a_hako_names_the_hako`: a locked hako in a cache far
from the app writes an arm into the app that imports back into the hako.
Before the change it was refused with "a line holds at most 80 characters —
this one has 128"; now it prints `hello there`.

The new branch moves two rows, as CI measured: entry_instructions
120,154,980 -> 120,155,020 by layout, and browser_compile_instructions
412,895,722 -> 412,896,104, since the compiler built for the tab carries
the new code; the compile and library rows stand. A senro app
installed as a hako could not compile without this, so the floor is lowered
by that cost under #2280, as it was for kanso#2511.

## 2026-11-27 — std/p256 verifies ES256 signatures

kanso#2515 rules passkeys in, with the password as the fallback, and a
server that accepts a passkey checks an ECDSA signature over P-256 with
SHA-256 (ES256). kanso had SHA-256 and nothing for elliptic curves.
`std/p256` adds `verified? x y message signature`: the key's two
32-byte coordinates, the message and a DER signature, all byte lists.

The module is kanso over `std/bits`: ten 26-bit limbs per number,
Montgomery multiplication modulo p and modulo n with constants computed
from the curve's published values, Jacobian points, Shamir's trick for
u1 G + u2 Q, and an x comparison against r Z^2 that needs no inversion.
It refuses a key off the curve, a coordinate of 32 bytes that is not
below p, non-strict DER, and r or s outside 1..n-1.

Harness first, as the verification rules ask. The first harness was a
Python script over Python's `cryptography`; CI's python-free gate refused
it, so scripts/p256_cases does the same in kanso, driving OpenSSL's command
line: sixteen keys from `openssl ecparam`, signatures from `openssl dgst
-sign`, tamperings made by moving bytes, and the verdict from `openssl
pkeyutl -verify`. 51,000 of its cases agreed with the library in every
verdict, 22,358 accepted and 28,642 refused; the Python
harness had agreed on 52,000 more before it went.

Both harnesses found disagreements, and each was the reference's. A
coordinate written as x + p: OpenSSL reduces it and accepts the key, SEC 1
2.3.4 makes a coordinate a field element below p, so the scripts refuse it
as the library does. And a signature with a byte after its DER: `openssl
dgst -verify` reads no more of the signature file than the key's longest
signature, 72 bytes, so it judged the signature without the byte and
accepted; `pkeyutl` reads the whole file and refuses it.

tests/p256_agrees_with_openssl.rs runs a committed 240-case corpus with
OpenSSL's verdicts natively and its first twelve on the interpreter.
Watched red by comparing the point's y with r: every case read 0.
lib/p256/p256_test.kso holds RFC 6979 A.2.5's two signatures, each checked
against OpenSSL before it was written, and watched red the same way
(three of ten failed).

CI moved four rows. Three are compile rows, by layout, for a module more
in the binary's table: compile_instructions 22,843,804 -> 22,843,810,
entry_instructions 120,155,020 -> 120,155,062, library_instructions
64,189,750 -> 64,189,756. The fourth is browser_compile_instructions
412,896,104 -> 412,896,154, since the compiler built for the tab carries
the module's text too. Passkeys are ruled in kanso#2515, so the floor is
lowered by what these cost, as it was for kanso#2514.

Measured in this container, not CI: about 13 ms a verification at the
release tier, 34 ms at the development tier, 1.4 s on the interpreter.

## 2026-11-27 — a job's install does not read Microsoft's archive

A hosted runner lists Microsoft's package archive among its apt sources,
for tools the image ships. On 2026-11-27 that archive answered 403 to the
runners, `apt-get update` exited 100 because a source it reads was no
longer signed, and the cost-goldens job never installed clang 19. Every
gate after it then refused to compare, measured on clang 18 against rows
taken on 19. Two runs of kanso#2517 failed the same way, and nothing in
that change touches the install.

No job here installs anything from Microsoft's archive: clang, lld,
wasi-libc, valgrind and jq all come from Ubuntu's. `scripts/apt_ready.sh`,
which every install already runs first, now removes any source file that
names packages.microsoft.com. `tests/apt_reads_a_mirror_that_resolves.rs`
gains `microsofts_archive_is_not_read`, which gives the script a sources
directory holding the runner's `microsoft-prod.list` line and an Ubuntu
source and checks that only the Ubuntu one is left. With the removal taken
out of the script it fails, listing both files.
