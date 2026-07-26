#!/usr/bin/env python3
"""The two pages that run kanso in the tab must actually run it.

The landing page's sample and the playground share one engine module, so a
change to either can break both, and neither failure shows up in a Rust test.
This loads each page in headless Chrome against a local copy of docs/, clicks
run, and requires the output the page promises.

Jekyll is not involved: the pages are fragments with front matter, so the
harness strips it and splices the header include, which is all these two need.
"""
import http.server
import json
import os
import re
import shutil
import socketserver
import subprocess
import sys
import tempfile
import threading
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs"
PORT = 8749


def find_chrome():
    if path := os.environ.get("KANSO_CHROME"):
        return path
    candidates = [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/usr/bin/google-chrome",
        "/usr/bin/chromium-browser",
        "/usr/bin/chromium",
    ]
    for path in candidates:
        if Path(path).exists():
            return path
    for name in ("google-chrome", "chromium-browser", "chromium", "chrome"):
        if found := shutil.which(name):
            return found
    raise SystemExit("no chrome found: set KANSO_CHROME")


PROBE = """
<script>
(async () => {
  const post = (r) => fetch('/report', {method: 'POST', body: JSON.stringify(r)});
  const settled = async (id, reject) => {
    for (let i = 0; i < 200; i++) {
      await new Promise(r => setTimeout(r, 250));
      const text = document.getElementById(id).textContent;
      if (text && !reject(text)) return text;
    }
    return document.getElementById(id).textContent;
  };
  try {
    await post(await PAGE_PROBE(settled));
  } catch (e) { await post({err: String(e)}); }
})();
</script>
"""

LANDING = """
window.PAGE_PROBE = async (settled) => {
  for (let i = 0; i < 60 && !window.KansoEngine; i++) await new Promise(r => setTimeout(r, 100));
  // the panel loads the engine lazily, so wait for it rather than racing the
  // click against a megabyte of wasm
  await window.KansoEngine.ready();
  document.getElementById('hero-run').click();
  return {out: await settled('hero-output', t => t === 'running…')};
};
"""

PLAYGROUND = """
window.PAGE_PROBE = async (settled) => {
  const first = await settled('output', t => t.startsWith('ready') || t === 'running…');
  const picker = document.getElementById('examples');
  picker.value = 'fanout';
  picker.dispatchEvent(new Event('change'));
  await new Promise(r => setTimeout(r, 300));
  document.getElementById('run').click();
  const second = await settled('output', t => t === first || t === 'running…');
  // the repl reaches into the wasm module directly rather than through
  // runSource, so it is the one surface a load-order slip can silently break
  const log = document.getElementById('repl-log');
  document.getElementById('repl-input').value = '2 + 3';
  document.getElementById('repl-form').dispatchEvent(
    new Event('submit', {cancelable: true, bubbles: true}));
  // the echo lands first and the answer follows, so wait for the answer's
  // own line rather than for the log to change at all
  for (let i = 0; i < 80 && !log.querySelector('.repl-out'); i++)
    await new Promise(r => setTimeout(r, 100));
  const answer = log.querySelector('.repl-out');
  // the button against edited source: the picker path can pass while a plain
  // edit-then-run is broken, because switching examples runs its own code
  const editor = document.getElementById('editor');
  editor.value = 'print \"edited then run {6 * 7}\"\\n';
  editor.dispatchEvent(new Event('input'));
  const beforeRun = document.getElementById('output').textContent;
  document.getElementById('run').click();
  const edited = await settled('output', t => t === beforeRun || t === 'running…');
  // and the keyboard path, which is a listener of its own
  editor.value = 'print \"keys then run {2 + 3}\"\\n';
  editor.dispatchEvent(new Event('input'));
  editor.dispatchEvent(new KeyboardEvent('keydown', {key: 'Enter', metaKey: true, bubbles: true}));
  const keyed = await settled('output', t => t === edited || t === 'running…');
  return {out: first, fanout: second, repl: answer ? answer.textContent : '',
          edited, keyed};
};
"""


def render(page, probe_body, work):
    header = (DOCS / "_includes/site-header.html").read_text()
    body = re.sub(r"^---.*?---\n", "", (DOCS / page).read_text(), flags=re.S)
    body = body.replace('src="/', 'src="')
    html = (
        "<!doctype html><html><head><meta charset=utf-8></head><body>"
        + header
        + body
        + f"<script>{probe_body}</script>"
        + PROBE
        + "</body></html>"
    )
    (work / page).write_text(html)


def visit(page, work):
    report, done = {}, threading.Event()

    class Handler(http.server.SimpleHTTPRequestHandler):
        def do_POST(self):
            report.update(json.loads(self.rfile.read(int(self.headers["Content-Length"]))))
            self.send_response(200)
            self.end_headers()
            done.set()

        def log_message(self, *args):
            pass

    socketserver.TCPServer.allow_reuse_address = True
    server = socketserver.TCPServer(("127.0.0.1", PORT), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    chrome = subprocess.Popen(
        [
            find_chrome(),
            "--headless=new",
            "--disable-gpu",
            "--no-sandbox",
            f"--user-data-dir={work / 'chrome-profile'}",
            f"http://127.0.0.1:{PORT}/{page}",
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    done.wait(180)
    chrome.kill()
    chrome.wait()
    server.shutdown()
    server.server_close()
    return report


def main():
    work = Path(tempfile.mkdtemp())
    for asset in ("kanso-engine.js", "play.js", "landing-play.js", "kanso.wasm"):
        shutil.copy(DOCS / asset, work / asset)
    render("index.html", LANDING, work)
    render("playground.html", PLAYGROUND, work)
    os.chdir(work)

    failures = []
    landing = visit("index.html", work)
    if "hello, kanso" not in (landing.get("out") or ""):
        failures.append(f"the landing sample did not run: {landing}")

    playground = visit("playground.html", work)
    if "hello, kanso" not in (playground.get("out") or ""):
        failures.append(f"the playground's first example did not run: {playground}")
    # fanout puts a closure in a record field, which the browser engine could
    # not do until Value::TableFn — the regression this page must never take
    if "650 yen" not in (playground.get("fanout") or ""):
        failures.append(f"fanout did not run in the playground: {playground}")
    # the repl calls the module directly, so it breaks independently of the
    # run button and needs its own probe
    if "5" not in (playground.get("repl") or ""):
        failures.append(f"the repl did not answer: {playground.get('repl')!r}")
    # the run button against source the visitor typed, and the ⌘⏎ listener
    if "edited then run 42" not in (playground.get("edited") or ""):
        failures.append(f"the run button did not run edited source: {playground.get('edited')!r}")
    if "keys then run 5" not in (playground.get("keyed") or ""):
        failures.append(f"⌘⏎ did not run edited source: {playground.get('keyed')!r}")

    for line in failures:
        print(f"FAIL  {line}")
    if not failures:
        print("PASS  landing sample and playground both run in the browser")
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
