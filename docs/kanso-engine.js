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
  'from_code', 'if', 'join', 'keys', 'length', 'map', 'print', 'push', 'put',
  'random', 'read_file', 'slice', 'sleep', 'sort', 'stdin', 'sum',
  'to_float', 'to_int', 'utf8', 'values', 'write_file',
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
    const op = rest.match(/^(->|==|!=|<=|>=|[=+\-*\/<>.\[\]():])/);
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
  const ptr = wasm.kanso_alloc(bytes.length) >>> 0;
  new Uint8Array(wasm.memory.buffer, ptr, bytes.length).set(bytes);
  return { ptr, len: bytes.length };
}

function readOut() {
  const out = new Uint8Array(wasm.memory.buffer, wasm.kanso_out_ptr() >>> 0, wasm.kanso_out_len() >>> 0);
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
  const bytes = new Uint8Array(wasm.memory.buffer, wasm.kanso_wasm_ptr() >>> 0, wasm.kanso_wasm_len() >>> 0).slice();
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


/* ---------- the native route: the emitter's module on runtime.c ----------

   The toolchain compiles a program the way `kanso build` does and lowers the
   module the native emitter writes to a wasm side module (src/ir_wasm.rs).
   That module links against runtime.c built for wasm32 (kanso-runtime.wasm):
   it imports the runtime's memory, table and stack pointer, plus two bases
   chosen here, `__memory_base` for its data and `__table_base` for its
   functions. Values live in the runtime's memory in native's layout, and the
   program calls the same k_* functions a native binary calls. */

let runtimeModule = null;

async function loadRuntime() {
  try {
    const response = await fetch(new URL('kanso-runtime.wasm', HERE));
    if (!response.ok) return;
    runtimeModule = await WebAssembly.compile(await response.arrayBuffer());
  } catch (e) {
    console.warn('kanso: the native route is unavailable', e);
  }
}

class ProcExit {
  constructor(code) { this.code = code; }
}

/* The dozen WASI calls the runtime makes when a program runs, answered for a
   page. No stream is a terminal, so diagnostics carry no colour. A sleep
   returns at once and moves the program's clock by what it asked for, so the
   scheduler meets its deadlines in the order native does. There are no files
   and no processes, so everything else answers ENOSYS and the runtime refuses
   the call in its own words. */
function wasiHost(env) {
  const host = { memory: null, out: [], err: [], slept: 0n, start: BigInt(Date.now()) * 1000000n };
  const view = () => new DataView(host.memory.buffer);
  const bytes = () => new Uint8Array(host.memory.buffer);
  const entries = Object.entries(env).map(([k, v]) => new TextEncoder().encode(`${k}=${v}\0`));
  const calls = {
    fd_write(fd, iovs, n, written) {
      let total = 0;
      for (let k = 0; k < n; k++) {
        const p = view().getUint32(iovs + k * 8, true);
        const len = view().getUint32(iovs + k * 8 + 4, true);
        (fd === 2 ? host.err : host.out).push(bytes().slice(p, p + len));
        total += len;
      }
      view().setUint32(written, total, true);
      return 0;
    },
    args_sizes_get(count, size) {
      view().setUint32(count, 0, true);
      view().setUint32(size, 0, true);
      return 0;
    },
    args_get() { return 0; },
    environ_sizes_get(count, size) {
      view().setUint32(count, entries.length, true);
      view().setUint32(size, entries.reduce((n, e) => n + e.length, 0), true);
      return 0;
    },
    environ_get(at, buf) {
      for (const e of entries) {
        view().setUint32(at, buf, true);
        bytes().set(e, buf);
        at += 4;
        buf += e.length;
      }
      return 0;
    },
    random_get(buf, len) {
      crypto.getRandomValues(bytes().subarray(buf, buf + len));
      return 0;
    },
    fd_fdstat_get(fd, stat) {
      bytes().fill(0, stat, stat + 24);
      return 0;
    },
    fd_prestat_get() { return 8; },
    clock_time_get(id, precision, at) {
      view().setBigUint64(at, host.start + host.slept, true);
      return 0;
    },
    poll_oneoff(subs, events, n, count) {
      for (let k = 0; k < n; k++) {
        const sub = subs + k * 48;
        const tag = bytes()[sub + 8];
        if (tag === 0) host.slept += view().getBigUint64(sub + 24, true);
        bytes().fill(0, events + k * 32, events + k * 32 + 32);
        bytes().set(bytes().slice(sub, sub + 8), events + k * 32);
        bytes()[events + k * 32 + 10] = tag;
      }
      view().setUint32(count, n, true);
      return 0;
    },
    sched_yield() { return 0; },
    proc_exit(code) { throw new ProcExit(code); },
  };
  host.imports = new Proxy(calls, { get: (target, name) => target[name] || (() => 52) });
  return host;
}

function joined(chunks) {
  const all = new Uint8Array(chunks.reduce((n, c) => n + c.length, 0));
  let at = 0;
  for (const c of chunks) {
    all.set(c, at);
    at += c.length;
  }
  return new TextDecoder().decode(all);
}

/* compile the editor's program through the native emitter and run it on the
   runtime; returns null when the route is unavailable or declines the
   program, and the older route picks it up */
async function runNative(src, compileFn) {
  if (!runtimeModule) return null;
  const { ptr, len } = writeInput(src);
  const status = compileFn(ptr, len);
  if (status === 2) return { code: 1, text: readOut(), engine: 'error' };
  if (status === 1) return null;
  const side = new Uint8Array(wasm.memory.buffer, wasm.kanso_wasm_ptr() >>> 0, wasm.kanso_wasm_len() >>> 0).slice();
  const data = wasm.kanso_side_data() >>> 0;
  const slots = wasm.kanso_side_table() >>> 0;
  const host = wasiHost({ KANSO_SEED: String(Date.now() >>> 0) });
  const runtime = await WebAssembly.instantiate(runtimeModule, { wasi_snapshot_preview1: host.imports });
  const rt = runtime.exports;
  host.memory = rt.memory;
  const env = {
    memory: rt.memory,
    __indirect_function_table: rt.__indirect_function_table,
    __memory_base: new WebAssembly.Global({ value: 'i32', mutable: false }, rt.malloc(data)),
    __table_base: new WebAssembly.Global({ value: 'i32', mutable: false }, rt.__indirect_function_table.grow(slots)),
  };
  let module;
  try {
    module = await WebAssembly.compile(side);
  } catch (e) {
    console.warn('kanso: the translator wrote a module the engine rejected', e);
    return null;
  }
  for (const imp of WebAssembly.Module.imports(module)) {
    if (!(imp.name in env)) env[imp.name] = rt[imp.name];
  }
  let code = 0;
  try {
    await WebAssembly.instantiate(module, { env });
    rt._start();
  } catch (e) {
    if (e instanceof ProcExit) {
      code = e.code;
    } else if (e instanceof RangeError) {
      /* the wasm call stack ran out: the sentence every engine prints */
      wasm.kanso_stack_exhausted();
      host.err.push(new TextEncoder().encode(readOut()));
      code = 1;
    } else {
      host.err.push(new TextEncoder().encode(`${e}\n`));
      code = 1;
    }
  }
  return { code, text: joined(host.out) + joined(host.err), engine: 'native' };
}


/* ---------- what a page needs ---------- */

async function ready() {
  if (!wasm) await Promise.all([loadWasm(), loadRuntime()]);
}

/* Run a program the way the playground does: compiled to wasm when the
   backend covers it, interpreted when it declines. */
async function runSource(src) {
  await ready();
  wasm.kanso_set_seed(Date.now() >>> 0);
  const native = await runNative(src, wasm.kanso_compile_native);
  if (native) return native;
  const compiled = await runCompiled(src, wasm.kanso_compile_wasm);
  if (compiled) return compiled;
  return Object.assign(callKanso('kanso_run', src), { engine: 'interp' });
}

/* Run a playground buffer: a play file — declarations and statements in
   one file, stdlib imports only. Same two engines, the play door. */
async function playSource(src) {
  await ready();
  wasm.kanso_set_seed(Date.now() >>> 0);
  const native = await runNative(src, wasm.kanso_play_native);
  if (native) return native;
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
  const native = await runNative(entry, wasm.kanso_compile_native);
  if (native) return native;
  const compiled = await runCompiled(entry, wasm.kanso_compile_wasm);
  if (compiled) return compiled;
  return Object.assign(callKanso('kanso_run', entry), { engine: 'interp' });
}

window.KansoEngine = {
  ready, runSource, playSource, runLibrary, highlight, callKanso,
  get wasm() { return wasm; },
};
})();
