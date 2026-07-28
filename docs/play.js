/* kanso playground: the editor, the examples, and the repl strip. The engine
   and the tokenizer live in kanso-engine.js, shared with the landing page. */
'use strict';

const { ready, runSource, highlight, callKanso } = window.KansoEngine;
/* ---------- editor: transparent textarea over a highlighted mirror ---------- */

const editor = document.getElementById('editor');
const mirror = document.getElementById('mirror');
/* the <pre> is the scroll container (overflow:auto); the <code> mirror
   inside it is inline and can't scroll, so the highlight layer must be
   scrolled via its parent to track the textarea */
const mirrorScroll = mirror.parentElement;
const output = document.getElementById('output');
const runButton = document.getElementById('run');
const examples = document.getElementById('examples');
const replForm = document.getElementById('repl-form');
const replInput = document.getElementById('repl-input');
const replLog = document.getElementById('repl-log');

function syncMirror() {
  mirror.innerHTML = highlight(editor.value) + '\n';
  mirrorScroll.scrollTop = editor.scrollTop;
  mirrorScroll.scrollLeft = editor.scrollLeft;
}

async function run() {
  const result = await runSource(editor.value);
  const badge = { wasm: '⚡ compiled to wasm in your tab', interp: 'interpreted', error: '' }[result.engine];
  output.textContent = (result.text || '(no output)') + (badge ? `\n\n— ${badge}` : '');
  output.classList.toggle('play-error', result.code !== 0);
}

editor.addEventListener('input', syncMirror);
editor.addEventListener('scroll', () => {
  mirrorScroll.scrollTop = editor.scrollTop;
  mirrorScroll.scrollLeft = editor.scrollLeft;
});
editor.addEventListener('keydown', (event) => {
  if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
    event.preventDefault();
    run();
  }
  if (event.key === 'Tab') {
    event.preventDefault();
    const at = editor.selectionStart;
    editor.setRangeText('  ', at, editor.selectionEnd, 'end');
    syncMirror();
  }
});
runButton.addEventListener('click', run);

/* ---------- repl strip ---------- */

function replEcho(cls, text) {
  const line = document.createElement('div');
  line.className = cls;
  line.textContent = text;
  replLog.appendChild(line);
  replLog.scrollTop = replLog.scrollHeight;
}

replForm.addEventListener('submit', async (event) => {
  event.preventDefault();
  const input = replInput.value;
  if (!input.trim()) return;
  replInput.value = '';
  replEcho('repl-in', '» ' + input);
  /* callKanso reaches into the module directly, so unlike runSource it has
     no load of its own to wait on */
  await ready();
  const { code, text } = callKanso('kanso_repl_eval', input);
  if (text) replEcho(code === 0 ? 'repl-out' : 'repl-out play-error', text.trimEnd());
});

/* ---------- examples ---------- */

const EXAMPLES = {
  hello: `print "hello, kanso"
`,
  dispatch: `fn fact 0
  1

fn fact n
  n * (fact (n - 1))

pub play = print "20! = {fact 20}"
`,
  json: `import "std/json"

fn report (err reason)
  "did not parse: {reason}"

fn report doc
  "title {doc["title"]}, {length doc} fields in all"

pub play =
  good = "\\{\\"title\\": \\"kanso\\", \\"stars\\": 3}"
  torn = "\\{\\"title\\": \\"kanso\\", \\"stars\\": }"
  print (report (json/decode good))
  >> print (report (json/decode torn))
`,
  railway: `fn describe n
  "half is {n}"

fn half 0
  err "cannot halve zero"

fn half n
  n / 2

pub play = print (describe (half 42))
`,
  pipes: `import "std/list"

pub play =
  total = [9 1 8 2 7] . sort . map (n -> n * n) . sum
  print "sum of squares: {total}"
`,
  ordering: `import "std/list"

fn cheapest prices
  first (sort prices)

pub play =
  prices = [520 380 450 610 290]
  # these two share nothing: the compiler is free to run them in parallel
  low = cheapest prices
  total = sum prices
  # report consumes both, so it waits for both -- the barrier is the data
  report low total

fn report low total
  print "cheapest: {low} yen / total: {total} yen"
`,
  fanout: `# in go this is four goroutines, a channel, a WaitGroup, and a select.
# in kanso the channel is the data flow itself: fan-out is a map whose
# calls share nothing (the compiler is free to run them in parallel),
# and fan-in is whatever consumes the results -- the join is the data.
# go's select-over-message-types is kanso's dispatch-over-message-types:
# one arm per message, no select statement -- the redux example's update
# and notify arms are exactly that receive loop.
import "std/list"

fn fetch_quote city
  length city * 130

pub play =
  cities = ["tokyo" "kyoto" "osaka" "sapporo"]
  quotes = map cities (c -> fetch_quote c)
  cheapest = first (sort quotes)
  print "four lookups fanned out, one answer fanned in: {cheapest} yen"
`,
  build: `# two objects that point at each other. in most languages this needs a
# nullable field you check forever after, or a second pass that patches the
# link once both halves exist. a build block lets the knot be tied, then
# freezes the whole cohort -- once the block ends nothing can be rewritten, so
# the cycle is ordinary immutable data.
type person
  name
  partner

pub play =
  couple = build
    ada = person "ada" none
    bob = person "bob" ada
    ada.partner = bob
    [ada bob]
  a = couple[1]!
  print "{a.name} <-> {(a.partner).name} <-> {((a.partner).partner).name}"
`,
  contained: `# the same knot, but crossing call boundaries and then thrown away in
# bulk. tie hands the cycle out as an ordinary return value, round_trip
# walks two hops of it as an ordinary argument, and the loop builds two
# thousand of them and keeps none. a build block's cohort is born and dies
# inside one iteration, so the arena rewinds it whole -- no counting, no
# collector, and peak memory does not move with the count.
type node
  name
  peer

fn tie label
  build
    here = node label none
    there = node "pong" none
    here.peer = there
    there.peer = here
    here

fn round_trip n
  n.peer.peer.name

fn spin 0 acc
  acc

fn spin n acc
  spin (n - 1) (acc + length (round_trip (tie "ping")))

pub play =
  knot = tie "ping"
  print "one hop: {knot.peer.name}"
  >> print "back home: {round_trip knot}"
  >> print "two thousand more, all discarded: {spin 2000 0}"
`,
  currying: `# & holds a function's first arguments and waits for the rest.
# tax is a two-argument function; &tax 8 fixes the rate and hands back
# something that still wants a price. that is an ordinary value -- bind it,
# name it, pass it to another function -- with no lambda and no wrapper.
fn tax rate price
  price + (price * rate / 100)

fn quote pricer amount
  "{amount} becomes {pricer amount}"

pub play =
  local = &tax 8
  luxury = &tax 20
  print "one rate:  {quote local 250}"
  >> print "the other: {quote luxury 250}"
  >> print "and the same partial again: {local 100}"
`,
  join: `# two effects with no order between them -- parallel is the default, so
# plain lines say it. the >> is the wall: serving happens only after both.
# failures accumulate: if both sides err you get both reasons.
print "steeping the sencha"
print "warming the cups"
>> print "serving"
`,
  concurrency: `# in go, two things at once + waiting for both is a goroutine, a
# channel or WaitGroup, and a select. in kanso bare lines already run as
# cooperative green threads: the scheduler overlaps them, >> chains
# within a thread, and a lone >> line is a wall the whole group settles
# behind. brew blocks on a slow steep while rolls chains the dice
# beside it, so every roll lands during the steep. (in the browser
# sleep is instant, but the interleaved ORDER matches a live run.)
import "std/math"
import "std/time"

brew = print "brew: steeping" >> time/sleep 60 >> print "brew: poured"

pub play =
  brew
  rolls

fn roll i
  math/random 6 . (n -> print "roll {i}: a {n + 1}")

rolls = roll 1 >> roll 2 >> roll 3 >> roll 4 >> roll 5
`,
  redux: `import "std/time"

type deposit
  amount

type logger

type withdraw
  amount

fn drive store actions i sub out
  step store actions i sub out (actions[i])

fn notify logger (deposit n) balance
  print "[logger] +{n} yen in -> the till holds {balance}"

fn notify logger (withdraw n) balance
  print "[logger] -{n} yen out -> the till holds {balance}"

pub play =
  moves = [(deposit 100) (withdraw 30) (withdraw 60) (deposit 5)]
  drive 0 moves 1 logger (print "the till opens at 0 yen")

fn step _ _ _ _ out none
  out >> print "the till closes"

fn step store actions i sub out action
  next = update store action
  told = out >> notify sub action next >> time/sleep 350
  drive next actions (i + 1) sub told

fn update balance (deposit n)
  balance + n

fn update balance (withdraw n)
  if (balance < n) (err "overdrawn: tried {n} against {balance}") (balance - n)
`,
};

examples.addEventListener('change', () => {
  editor.value = EXAMPLES[examples.value];
  syncMirror();
  run();
});

// a reload restores the browser's remembered dropdown choice while the
// editor resets to the first example — pin the two together at startup
examples.value = 'hello';

ready().then(() => {
  output.textContent = 'ready — ⌘⏎ runs';
  run();
});
syncMirror();
