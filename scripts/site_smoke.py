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


BOOK = """
window.PAGE_PROBE = async (settled) => {
  for (let i = 0; i < 60 && !window.KansoEngine; i++) await new Promise(r => setTimeout(r, 100));
  await window.KansoEngine.ready();
  // a chapter's panels: the ones that can run here grew an editor and a
  // button, and the ones that need a filesystem stayed as the book recorded
  const panels = [...document.querySelectorAll('.code-panel')];
  const live = panels.filter(p => p.querySelector('.book-run'));
  const first = live[0];
  const out = first.querySelector('.code-output pre code');
  const before = out.textContent;
  first.querySelector('.book-run').click();
  for (let i = 0; i < 120 && out.textContent === before; i++)
    await new Promise(r => setTimeout(r, 100));
  // and the same panel against source the reader typed
  const editor = first.querySelector('textarea');
  editor.value = 'print "the reader edited this: {6 * 7}"\\n';
  editor.dispatchEvent(new Event('input'));
  const ran = out.textContent;
  first.querySelector('.book-run').click();
  for (let i = 0; i < 120 && out.textContent === ran; i++)
    await new Promise(r => setTimeout(r, 100));
  return {panels: panels.length, live: live.length, ran, edited: out.textContent,
          name: first.querySelector('.code-panel-title').textContent.trim()};
};
"""


# The chart reads its rows from the history branch over the network. A test
# that let it do so would depend on a branch's contents and on the network
# being there, and could pin no number. So the fetch is stubbed with a history
# of known length, and the assertion is that every series draws exactly that
# many points — a presence check would pass on an empty canvas, which is the
# failure this exists to catch.
CHART_ROWS = 6
CHART_HISTORY = "\n".join(
    json.dumps(
        {
            "commit": f"c{i}",
            "allocs": 7_000_000 + i * 1000,
            "encode_allocs": 16_000_000 + i * 2000,
            "oneshot_arena_peak_bytes": 3_000_000 + i * 4096,
            "compile_rounds": 40 + i,
            "compile_visits": 9000 + i * 10,
            "emitted_lines": 1500 + i,
            "compile_peak_bytes": 80_000_000 + i * 8192,
            "welfare": f"{75.0 + i * 0.1:.2f}",
        }
    )
    for i in range(CHART_ROWS)
)

CHART_PRELUDE = (
    "window.__chart = {stub: 0, err: ''};\n"
    "window.addEventListener('error', (e) => "
    "{ window.__chart.err = window.__chart.err || String(e.message); });\n"
    "const REAL = window.fetch.bind(window);\n"
    "const HISTORY_TEXT = " + json.dumps(CHART_HISTORY) + ";\n"
    "window.fetch = (url, opts) => {\n"
    "  if (String(url).includes('history.jsonl')) {\n"
    "    window.__chart.stub += 1;\n"
    "    return Promise.resolve(new Response(HISTORY_TEXT, {status: 200}));\n"
    "  }\n"
    "  return REAL(url, opts);\n"
    "};\n"
)

CHART = """
async function PAGE_PROBE() {
  const counts = async () => {
    const host = document.getElementById('trend-chart');
    if (!host) return null;
    const lines = [...host.querySelectorAll('polyline')];
    if (!lines.length) return null;
    return lines
      .map((l) => (l.getAttribute('points') || '').trim().split(/\\s+/).filter(Boolean).length)
      .sort((a, b) => a - b);
  };
  for (let i = 0; i < 200; i++) {
    const got = await counts();
    if (got) return {series: got};
    await new Promise((r) => setTimeout(r, 250));
  }
  const host = document.getElementById('trend-chart');
  const panel = document.querySelector('.perf-loading');
  return {
    series: [],
    why: {
      stubbed: window.__chart.stub,
      error: window.__chart.err,
      host: host ? host.innerHTML.slice(0, 200) : 'no #trend-chart',
      panel: panel ? panel.textContent.slice(0, 160) : 'no .perf-loading',
    },
  };
}
"""


def render(page, probe_body, work, extra=(), prelude=""):
    header = (DOCS / "_includes/site-header.html").read_text()
    body = re.sub(r"^---.*?---\n", "", (DOCS / page).read_text(), flags=re.S)
    body = body.replace('src="/', 'src="')
    # a chapter is served by a layout, which is where its scripts live
    body += "".join(f'<script src="{name}"></script>' for name in extra)
    # a prelude runs before the page's own scripts, which is the only place a
    # stub can stand in for something the page fetches on load
    html = (
        "<!doctype html><html><head><meta charset=utf-8>"
        + (f"<script>{prelude}</script>" if prelude else "")
        + "</head><body>"
        + header
        + body
        + f"<script>{probe_body}</script>"
        + PROBE
        + "</body></html>"
    )
    landing = work / page
    landing.parent.mkdir(parents=True, exist_ok=True)
    landing.write_text(html)


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
    shutil.copy(DOCS / "book-play.js", work / "book-play.js")
    render("book/ch04.html", BOOK, work, extra=("/kanso-engine.js", "/book-play.js"))
    render("compiler.html", CHART, work, prelude=CHART_PRELUDE)
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

    chart = visit("compiler.html", work)
    drawn = chart.get("series") or []
    if drawn != [CHART_ROWS] * 5:
        failures.append(
            f"the chart drew {drawn!r}; five series of {CHART_ROWS} points were fed to it"
            f" — {chart.get('why')}"
        )

    book = visit("book/ch04.html", work)
    if not (book.get("live") or 0):
        failures.append(f"no panel in chapter 04 became runnable: {book}")
    if "division by zero" not in (book.get("ran") or ""):
        failures.append(f"the chapter's first panel did not run: {book}")
    if "the reader edited this: 42" not in (book.get("edited") or ""):
        failures.append(f"a book panel did not run edited source: {book}")

    for line in failures:
        print(f"FAIL  {line}")
    if not failures:
        print("PASS  landing sample and playground both run in the browser")
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
