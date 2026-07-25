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

replForm.addEventListener('submit', (event) => {
  event.preventDefault();
  if (!wasm) return;
  const input = replInput.value;
  if (!input.trim()) return;
  replEcho('repl-in', '» ' + input);
  const { code, text } = callKanso('kanso_repl_eval', input);
  if (text) replEcho(code === 0 ? 'repl-out' : 'repl-out play-error', text.trimEnd());
  replInput.value = '';
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
  amount:int

type logger

type withdraw
  amount:int

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
