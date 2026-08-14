'use strict';

(function () {
/* Where this script was loaded from, so the wasm beside it is found from a
   page at any depth — a chapter lives under /book/ and a relative fetch would
   look for the engine inside that directory. */
const HERE = document.currentScript ? document.currentScript.src : location.href;
/* The kanso engine in the tab: the real toolchain compiled to wasm, plus the
   tokenizer that paints it. Shared by the playground and the landing page's
   live sample so the wiring exists once. */
/* ---------- tokenizer (mirrors the site's .k .f .s .i .t .o .c classes) ---------- */

const KEYWORDS = new Set(['fn', 'type']);
const NULLARY = new Set(['true', 'false', 'none', 'err']);
const BUILTINS = new Set([
  'args', 'at', 'bytes', 'char_code', 'chars', 'concat', 'entries', 'filter',
  'from_code', 'if', 'join', 'length', 'map', 'print', 'push', 'put',
  'random', 'read_file', 'slice', 'sleep', 'sort', 'stdin', 'sum',
  'to_float', 'to_int', 'utf8', 'write_file',
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
      /* the site's hand-marked panels nest the interpolation inside the
         string and colour the braces with it, so the literal reads as one
         object rather than three; match that or the two renderings of kanso
         on one site disagree */
      html += esc(run) + span('i', line.slice(i, close + 1));
      run = '';
      i = close + 1;
      continue;
    }
    run += ch;
    i += 1;
  }
  return [`<span class="s">${html}${esc(run)}</span>`, i];
}

function highlightLine(line) {
  let html = '';
  let i = 0;
  let afterFn = false;
  let afterType = false;
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
        afterType = name === 'type';
      } else if (afterFn) {
        html += span('f', name);
        afterFn = false;
      } else if (afterType) {
        afterType = false;
        if (ascription) {
          const parent = line.slice(i + name.length + 1)
            .match(/^[a-z0-9_\[\]]*/)[0];
          html += span('t', name) + span('o', ':') + span('t', parent);
          i += name.length + 1 + parent.length;
          continue;
        }
        html += span('t', name);
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
  const response = await fetch(new URL('kanso.wasm', HERE));
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
async function runCompiled(src, compileFn) {
  const { ptr, len } = writeInput(src);
  const status = compileFn(ptr, len, tailCalls ? 1 : 0);
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


/* ---------- what a page needs ---------- */

async function ready() {
  if (!wasm) await loadWasm();
}

/* Run a program the way the playground does: compiled to wasm when the
   backend covers it, interpreted when it declines. */
async function runSource(src) {
  await ready();
  wasm.kanso_set_seed(Date.now() >>> 0);
  const compiled = await runCompiled(src, wasm.kanso_compile_wasm);
  if (compiled) return compiled;
  return Object.assign(callKanso('kanso_run', src), { engine: 'interp' });
}

/* Run a playground buffer: a play file — declarations and statements in
   one file, stdlib imports only. Same two engines, the play door. */
async function playSource(src) {
  await ready();
  wasm.kanso_set_seed(Date.now() >>> 0);
  const compiled = await runCompiled(src, wasm.kanso_play_wasm);
  if (compiled) return compiled;
  return Object.assign(callKanso('kanso_play', src), { engine: 'interp' });
}

/* Run a library: a file that exports `play`, which the language runs through
   an entry file that imports it. There is no filesystem here, so the engine
   is handed the library under the name the import will use and compiles the
   entry beside it — the same two files the command line is given. */
async function runLibrary(stem, src) {
  await ready();
  wasm.kanso_set_seed(Date.now() >>> 0);
  wasm.kanso_forget_sources();
  const path = writeInput(stem);
  const file = writeInput(stem + '.kso');
  const lib = writeInput(src);
  wasm.kanso_hand_source(path.ptr, path.len, file.ptr, file.len, lib.ptr, lib.len);
  const entry = `import "${stem}"\n\n${stem}/play\n`;
  const compiled = await runCompiled(entry, wasm.kanso_compile_wasm);
  if (compiled) return compiled;
  return Object.assign(callKanso('kanso_run', entry), { engine: 'interp' });
}

window.KansoEngine = {
  ready, runSource, playSource, runLibrary, highlight, callKanso,
  get wasm() { return wasm; },
};
})();
