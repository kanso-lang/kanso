/* kanso playground: the real interpreter compiled to wasm, plus a live
   syntax-highlighted editor using the site's token palette. */
'use strict';

/* ---------- tokenizer (mirrors the site's .k .f .s .i .t .o .c classes) ---------- */

const KEYWORDS = new Set(['fn', 'type']);
const NULLARY = new Set(['true', 'false', 'none', 'err']);
const BUILTINS = new Set([
  'args', 'at', 'bytes', 'char_code', 'chars', 'concat', 'entries', 'filter',
  'from_code', 'if', 'join', 'length', 'map', 'print', 'push', 'put',
  'read_file', 'slice', 'sort', 'stdin', 'sum', 'to_float', 'to_int',
  'utf8', 'write_file',
]);

function esc(text) {
  return text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function span(cls, text) {
  return cls ? `<span class="${cls}">${esc(text)}</span>` : esc(text);
}

function highlightString(line, start) {
  /* returns [html, endIndex] for a string literal starting at `start` */
  let html = '';
  let run = '"';
  let i = start + 1;
  while (i < line.length) {
    const ch = line[i];
    if (ch === '\\' && i + 1 < line.length) {
      run += ch + line[i + 1];
      i += 2;
      continue;
    }
    if (ch === '"') {
      run += ch;
      i += 1;
      break;
    }
    if (ch === '{') {
      const close = line.indexOf('}', i);
      if (close === -1) {
        run += ch;
        i += 1;
        continue;
      }
      html += span('s', run);
      html += span('o', '{') + span('i', line.slice(i + 1, close)) + span('o', '}');
      run = '';
      i = close + 1;
      continue;
    }
    run += ch;
    i += 1;
  }
  return [html + span('s', run), i];
}

function highlightLine(line) {
  let html = '';
  let i = 0;
  let afterFn = false;
  while (i < line.length) {
    const rest = line.slice(i);
    const hash = rest.match(/^#.*/);
    if (hash) {
      html += span('c', hash[0]);
      break;
    }
    if (line[i] === '"') {
      const [strHtml, end] = highlightString(line, i);
      html += strHtml;
      i = end;
      continue;
    }
    const word = rest.match(/^[a-z_][a-z0-9_]*/);
    if (word) {
      const name = word[0];
      const ascription = line[i + name.length] === ':' && /[a-z]/.test(line[i + name.length + 1] || '');
      if (KEYWORDS.has(name)) {
        html += span('k', name);
        afterFn = name === 'fn';
      } else if (afterFn) {
        html += span('f', name);
        afterFn = false;
      } else if (ascription) {
        const type = line.slice(i + name.length + 1).match(/^[a-z0-9_\[\]]*/)[0];
        html += esc(name) + span('o', ':') + span('t', type);
        i += name.length + 1 + type.length;
        continue;
      } else if (NULLARY.has(name)) {
        html += span('k', name);
      } else if (BUILTINS.has(name)) {
        html += span('f', name);
      } else {
        html += esc(name);
      }
      i += name.length;
      continue;
    }
    const number = rest.match(/^-?\d[\d_]*(\.\d+)?/);
    if (number) {
      html += span('i', number[0]);
      i += number[0].length;
      continue;
    }
    const op = rest.match(/^(->|>>|==|!=|<=|>=|[=+\-*\/<>.\[\]():])/);
    if (op) {
      html += span('o', op[0]);
      i += op[0].length;
      continue;
    }
    html += esc(line[i]);
    i += 1;
  }
  return html;
}

function highlight(source) {
  return source.split('\n').map(highlightLine).join('\n');
}

/* ---------- wasm glue: raw extern "C", no bindgen ---------- */

let wasm = null;

/* the compiled program's function table; k_callback lets host-side closures
   (map, filter, bind) call back into it */
let programTable = null;

/* wasm tail calls: a tiny module using return_call, validated up front */
const TAILCALL_PROBE = new Uint8Array([
  0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
  0x01, 0x04, 0x01, 0x60, 0x00, 0x00,
  0x03, 0x02, 0x01, 0x00,
  0x0a, 0x06, 0x01, 0x04, 0x00, 0x12, 0x00, 0x0b,
]);
const tailCalls = WebAssembly.validate(TAILCALL_PROBE);

async function loadWasm() {
  const response = await fetch('kanso.wasm');
  const imports = { env: { k_callback: (t, e, a) => programTable.get(t)(e, a) } };
  const { instance } = await WebAssembly.instantiate(await response.arrayBuffer(), imports);
  wasm = instance.exports;
}

function writeInput(text) {
  const bytes = new TextEncoder().encode(text);
  const ptr = wasm.kanso_alloc(bytes.length);
  new Uint8Array(wasm.memory.buffer, ptr, bytes.length).set(bytes);
  return { ptr, len: bytes.length };
}

function readOut() {
  const out = new Uint8Array(wasm.memory.buffer, wasm.kanso_out_ptr(), wasm.kanso_out_len());
  return new TextDecoder().decode(out);
}

function callKanso(entry, text) {
  const { ptr, len } = writeInput(text);
  const code = wasm[entry](ptr, len);
  return { code, text: readOut() };
}

function rtImports() {
  const env = {};
  for (const key of Object.keys(wasm)) {
    if (key.startsWith('rt_')) env[key] = wasm[key];
  }
  return env;
}

/* compile the editor's program to a wasm module and run it natively in the
   tab; returns null when the browser backend doesn't cover the program yet
   (the interpreter picks it up) */
async function runCompiled(src) {
  const { ptr, len } = writeInput(src);
  const status = wasm.kanso_compile_wasm(ptr, len, tailCalls ? 1 : 0);
  if (status === 2) return { code: 1, text: readOut(), engine: 'error' };
  if (status === 1) return null;
  const bytes = new Uint8Array(wasm.memory.buffer, wasm.kanso_wasm_ptr(), wasm.kanso_wasm_len()).slice();
  let instance;
  try {
    ({ instance } = await WebAssembly.instantiate(bytes, { env: rtImports() }));
  } catch (e) {
    console.warn('kanso wasm backend emitted a module the engine rejected', e);
    return null;
  }
  programTable = instance.exports.table;
  let handle;
  try {
    handle = instance.exports.main();
  } catch (e) {
    wasm.kanso_take_rt_error();
    return { code: 1, text: readOut(), engine: 'wasm' };
  }
  let code;
  try {
    code = wasm.kanso_exec_main(handle);
  } catch (e) {
    wasm.kanso_take_rt_error();
    return { code: 1, text: readOut(), engine: 'wasm' };
  }
  return { code, text: readOut(), engine: 'wasm' };
}

/* ---------- editor: transparent textarea over a highlighted mirror ---------- */

const editor = document.getElementById('editor');
const mirror = document.getElementById('mirror');
const output = document.getElementById('output');
const runButton = document.getElementById('run');
const examples = document.getElementById('examples');
const replForm = document.getElementById('repl-form');
const replInput = document.getElementById('repl-input');
const replLog = document.getElementById('repl-log');

function syncMirror() {
  mirror.innerHTML = highlight(editor.value) + '\n';
  mirror.scrollTop = editor.scrollTop;
  mirror.scrollLeft = editor.scrollLeft;
}

async function run() {
  if (!wasm) return;
  let result = await runCompiled(editor.value);
  let engine = result ? result.engine : null;
  if (!result) {
    result = callKanso('kanso_run', editor.value);
    engine = 'interp';
  }
  const badge = { wasm: '⚡ compiled to wasm in your tab', interp: 'interpreted', error: '' }[engine];
  output.textContent = (result.text || '(no output)') + (badge ? `\n\n— ${badge}` : '');
  output.classList.toggle('play-error', result.code !== 0);
}

editor.addEventListener('input', syncMirror);
editor.addEventListener('scroll', () => {
  mirror.scrollTop = editor.scrollTop;
  mirror.scrollLeft = editor.scrollLeft;
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
  hello: `main = print "hello, kanso"
`,
  dispatch: `fn fact 0
  1

fn fact n
  n * (fact (n - 1))

main = print "20! = {fact 20}"
`,
  railway: `fn describe n
  "half is {n}"

fn half 0
  err "cannot halve zero"

fn half n
  n / 2

main = print (describe (half 42))
`,
  pipes: `main =
  total = [9 1 8 2 7] . sort . map (n -> n * n) . sum
  print "sum of squares: {total}"
`,
  ordering: `fn cheapest prices
  sort prices . at 1

main =
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
fn fetch_quote city
  length city * 130

main =
  cities = ["tokyo" "kyoto" "osaka" "sapporo"]
  quotes = map cities (c -> fetch_quote c)
  cheapest = sort quotes . at 1
  print "four lookups fanned out, one answer fanned in: {cheapest} yen"
`,
  join: `# two effects with no order between them, joined by &; the >> is the
# barrier: serving happens only after both. failures accumulate -- if
# both sides err you get both reasons, not just the first.
main =
  steep = print "steeping the sencha"
  warm = print "warming the cups"
  steep & warm >> print "serving"
`,
  redux: `type deposit
  amount: int

type logger

type withdraw
  amount: int

main =
  moves = [(deposit 100) (withdraw 30) (withdraw 60) (deposit 5)]
  play 0 moves 1 logger (print "the till opens at 0 yen")

fn notify logger (deposit n) balance
  print "[logger] +{n} yen in -> the till holds {balance}"

fn notify logger (withdraw n) balance
  print "[logger] -{n} yen out -> the till holds {balance}"

fn play store actions i sub out
  step store actions i sub out (at actions i)

fn step _ _ _ _ out none
  out >> print "the till closes"

fn step store actions i sub out action
  next = update store action
  play next actions (i + 1) sub (out >> notify sub action next)

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

loadWasm().then(() => {
  output.textContent = 'ready — ⌘⏎ runs';
  run();
});
syncMirror();
