#!/usr/bin/env python3
"""Differential harness for the browser wasm backend.

Runs every golden-corpus program through docs/kanso.wasm in headless Chrome
(compile with kanso_compile_wasm, instantiate the emitted module, execute via
kanso_exec_main) and requires byte-identical (status, output) against the
native engine (`kanso run`). Programs the backend declines compile-time
(status 1) are reported as SKIP(fallback) with the reason.

The page POSTs its results back to this process's HTTP server; Chrome is
killed as soon as the report lands, so nothing depends on headless Chrome
exiting on its own (its --dump-dom/--virtual-time-budget exit is flaky).
"""
import json
import shutil
import subprocess
import sys
import tempfile
import threading
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
import os


def find_chrome():
    if p := os.environ.get("KANSO_CHROME"):
        return p
    candidates = [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/usr/bin/google-chrome",
        "/usr/bin/chromium-browser",
        "/usr/bin/chromium",
    ]
    import shutil as _sh
    for c in candidates:
        if os.path.exists(c):
            return c
    for name in ("google-chrome", "chromium-browser", "chromium", "chrome"):
        if p := _sh.which(name):
            return p
    raise SystemExit("no chrome found: set KANSO_CHROME")


CHROME = find_chrome()
KANSO = ROOT / "target/release/kanso"

PAGE = """<!doctype html>
<meta charset="utf-8">
<title>kanso browser differential</title>
<pre id="results"></pre>
<script>
'use strict';
const CASES = __CASES__;

const TAILCALL_PROBE = new Uint8Array([
  0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
  0x01, 0x04, 0x01, 0x60, 0x00, 0x00,
  0x03, 0x02, 0x01, 0x00,
  0x0a, 0x06, 0x01, 0x04, 0x00, 0x12, 0x00, 0x0b,
]);
const tailCalls = WebAssembly.validate(TAILCALL_PROBE);

let wasm = null;
let programTable = null;

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

function rtImports() {
  const env = {};
  for (const key of Object.keys(wasm)) {
    if (key.startsWith('rt_')) env[key] = wasm[key];
  }
  return env;
}

async function runCase(c) {
  const name = writeInput(c.name.split('/').pop());
  wasm.kanso_set_file(name.ptr, name.len);
  const { ptr, len } = writeInput(c.src);
  const status = wasm.kanso_compile_wasm(ptr, len, tailCalls ? 1 : 0);
  if (status === 2) return { kind: 'compile-error', code: 2, text: readOut() };
  if (status === 1) return { kind: 'fallback', reason: readOut() };
  const bytes = new Uint8Array(wasm.memory.buffer, wasm.kanso_wasm_ptr(), wasm.kanso_wasm_len()).slice();
  let instance;
  try {
    ({ instance } = await WebAssembly.instantiate(bytes, { env: rtImports() }));
  } catch (e) {
    return { kind: 'reject', reason: String(e) };
  }
  programTable = instance.exports.table;
  let handle;
  try {
    handle = instance.exports.main();
  } catch (e) {
    wasm.kanso_take_rt_error();
    return { kind: 'wasm', code: 1, text: readOut() };
  }
  let code;
  try {
    code = wasm.kanso_exec_main(handle);
  } catch (e) {
    wasm.kanso_take_rt_error();
    return { kind: 'wasm', code: 1, text: readOut() };
  }
  return { kind: 'wasm', code, text: readOut() };
}

async function main() {
  const response = await fetch('kanso.wasm');
  const imports = { env: { k_callback: (t, e, a) => programTable.get(t)(e, a) } };
  const { instance } = await WebAssembly.instantiate(await response.arrayBuffer(), imports);
  wasm = instance.exports;
  const results = [];
  for (const c of CASES) {
    let r;
    try {
      r = await runCase(c);
    } catch (e) {
      r = { kind: 'crash', reason: String((e && e.stack) || e) };
    }
    r.name = c.name;
    results.push(r);
  }
  return { tailCalls, results };
}

main()
  .then((payload) => fetch('/report', { method: 'POST', body: JSON.stringify(payload) }))
  .catch((e) =>
    fetch('/report', { method: 'POST', body: JSON.stringify({ error: String((e && e.stack) || e) }) })
  );
</script>
"""


def corpus():
    dirs = [ROOT / "examples", ROOT / "tests/golden/runtime"]
    paths = [path for d in dirs for path in sorted(d.glob("*.kso"))]
    # the browser has no filesystem, so `import` cannot resolve there — those
    # programs are out of scope for the differential until the playground
    # bundles the shipped library. skip them loudly rather than fail.
    runnable, skipped = [], []
    for path in paths:
        (skipped if "import " in path.read_text() else runnable).append(path)
    for path in skipped:
        print(f"SKIP  {path.relative_to(ROOT)} (uses import — no filesystem in the browser)")
    return runnable


def native_outcome(path):
    run = subprocess.run(
        [str(KANSO), "run", path.name],
        capture_output=True,
        cwd=path.parent,
        text=True,
        timeout=120,
    )
    return run.returncode, run.stdout + run.stderr


class ReportHandler(SimpleHTTPRequestHandler):
    report = None
    reported = threading.Event()

    def do_POST(self):
        length = int(self.headers.get("Content-Length", 0))
        type(self).report = self.rfile.read(length)
        self.send_response(200)
        self.end_headers()
        type(self).reported.set()

    def log_message(self, format, *args):
        pass


def browser_results(entries):
    workdir = Path(tempfile.mkdtemp(prefix="kanso-diff-"))
    shutil.copy(ROOT / "docs/kanso.wasm", workdir / "kanso.wasm")
    cases_js = json.dumps(entries).replace("</", "<\\/")
    (workdir / "index.html").write_text(PAGE.replace("__CASES__", cases_js))
    handler = partial(ReportHandler, directory=str(workdir))
    server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
    port = server.server_address[1]
    threading.Thread(target=server.serve_forever, daemon=True).start()
    chrome = subprocess.Popen(
        [
            CHROME,
            "--headless=new",
            "--disable-gpu",
            f"--user-data-dir={workdir / 'chrome-profile'}",
            f"http://127.0.0.1:{port}/index.html",
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    arrived = ReportHandler.reported.wait(timeout=120)
    chrome.kill()
    chrome.wait()
    server.shutdown()
    if not arrived:
        sys.exit("harness failure: the page never reported (120s)")
    payload = json.loads(ReportHandler.report.decode("utf-8"))
    if "error" in payload:
        sys.exit(f"harness failure: page error\n{payload['error']}")
    return payload


def show(text):
    return json.dumps(text)


def main():
    paths = corpus()
    entries = [
        {"name": str(path.relative_to(ROOT)), "src": path.read_text()} for path in paths
    ]
    payload = browser_results(entries)
    results = payload["results"]
    print(f"tail calls: {'yes' if payload['tailCalls'] else 'no'}")
    if [r["name"] for r in results] != [e["name"] for e in entries]:
        sys.exit("harness failure: result names do not match the corpus")

    passed, fallbacks, failures = 0, 0, 0
    for path, result in zip(paths, results):
        name = result["name"]
        kind = result["kind"]
        if kind == "fallback":
            fallbacks += 1
            reason = result["reason"].strip()
            print(f"SKIP  {name} (fallback: {reason})")
            continue
        if kind != "wasm":
            failures += 1
            reason = result.get("reason", result.get("text", "")).strip()
            print(f"FAIL  {name} ({kind}: {reason})")
            continue
        native_code, native_text = native_outcome(path)
        if (result["code"], result["text"]) == (native_code, native_text):
            passed += 1
            print(f"PASS  {name}")
        else:
            failures += 1
            print(f"FAIL  {name}")
            print(f"      native: code={native_code} text={show(native_text)}")
            print(f"      wasm:   code={result['code']} text={show(result['text'])}")

    print(f"\n{passed} passed, {fallbacks} fallback, {failures} failed")
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
