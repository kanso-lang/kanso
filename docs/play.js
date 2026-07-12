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

async function loadWasm() {
  const response = await fetch('kanso.wasm');
  const { instance } = await WebAssembly.instantiate(await response.arrayBuffer(), {});
  wasm = instance.exports;
}

function callKanso(entry, text) {
  const bytes = new TextEncoder().encode(text);
  const ptr = wasm.kanso_alloc(bytes.length);
  new Uint8Array(wasm.memory.buffer, ptr, bytes.length).set(bytes);
  const code = wasm[entry](ptr, bytes.length);
  const out = new Uint8Array(wasm.memory.buffer, wasm.kanso_out_ptr(), wasm.kanso_out_len());
  return { code, text: new TextDecoder().decode(out) };
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

function run() {
  if (!wasm) return;
  const { code, text } = callKanso('kanso_run', editor.value);
  output.textContent = text || '(no output)';
  output.classList.toggle('play-error', code !== 0);
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
