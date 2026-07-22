#!/usr/bin/env python3
"""Book panels render their sample files. `--check` diffs every panel whose
title names a sample against that sample's text; `--write` regenerates the
drifted ones from source through the shared highlighter."""
import re, sys, os, html

CHAPTER_DIR = "docs/book"
KEYWORDS = {"fn", "pub", "type", "import"}

def esc(text):
    return text.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")

def highlight_string(tok):
    # a string literal: interpolations get the i span, inside the s span
    body = tok[1:-1]
    parts, i = [], 0
    while i < len(body):
        if body[i] == "{":
            depth, j = 1, i + 1
            while j < len(body) and depth:
                depth += {"{": 1, "}": -1}.get(body[j], 0)
                j += 1
            parts.append(f'<span class="i">{esc(body[i:j])}</span>')
            i = j
        else:
            k = body.find("{", i)
            k = len(body) if k == -1 else k
            parts.append(esc(body[i:k]))
            i = k
    return f'<span class="s">"{"".join(parts)}"</span>'

TOKEN = re.compile(
    r'"(?:\\.|[^"\\])*"'      # string
    r"|#[^\n]*"                # comment
    r"|[a-z_][a-zA-Z0-9_/]*[?!]?"  # ident (incl. qualified, ? !)
    r"|\d+\.\d+|\d+"           # number
    r"|>>|->|==|!=|<=|>=|[=<>+\-*/.]"  # operators
    r"|.",                     # anything else verbatim
    re.S,
)

OPS = {">>", "->", "==", "!=", "<=", ">=", "=", "<", ">", "+", "-", "*", "/", "."}

def highlight_line(line):
    out = []
    toks = [(m.group(0), m.start()) for m in TOKEN.finditer(line)]
    # find, per identifier, whether it is in f position:
    #   decl name after fn/pub; head of an application (ident followed by an
    #   argument token on the same line); a bare statement reference
    stripped = line.strip()
    idents = [t for t, _ in toks if re.fullmatch(r"[a-z_][a-zA-Z0-9_/]*[?!]?", t)]
    for idx, (tok, pos) in enumerate(toks):
        if tok.startswith('"'):
            out.append(highlight_string(tok))
        elif tok.startswith("#"):
            out.append(f'<span class="c">{esc(tok)}</span>')
        elif tok in KEYWORDS:
            out.append(f'<span class="k">{tok}</span>')
        elif re.fullmatch(r"\d+\.\d+|\d+", tok):
            out.append(f'<span class="n">{tok}</span>')
        elif tok in OPS:
            out.append(f'<span class="o">{esc(tok)}</span>')
        elif re.fullmatch(r"[a-z_][a-zA-Z0-9_/]*[?!]?", tok):
            out.append(mark_ident(line, toks, idx))
        else:
            out.append(esc(tok))
    return "".join(out)

def mark_ident(line, toks, idx):
    tok, pos = toks[idx]
    prev = toks[idx - 1][0] if idx > 0 else None
    prev2 = toks[idx - 2][0] if idx > 1 else None
    nxt = toks[idx + 1][0] if idx + 1 < len(toks) else None
    # skip whitespace tokens in prev/next reasoning: TOKEN's "." fallback
    # emits spaces as single chars; walk to meaningful neighbours instead
    def near(j, step):
        while 0 <= j < len(toks):
            t = toks[j][0]
            if t.strip():
                return t
            j += step
        return None
    p = near(idx - 1, -1)
    n = near(idx + 1, 1)
    if p in {"fn", "pub"}:
        return f'<span class="f">{tok}</span>'
    if p == "type":
        return tok
    applied = n is not None and (
        re.fullmatch(r'[a-z_][a-zA-Z0-9_/]*[?!]?|\d+\.\d+|\d+', n)
        or n.startswith('"') or n == "(" or n == "["
    ) and not (n == "[" and line[toks[idx][1] + len(tok):].startswith("["))
    # ident immediately postfixed by [ is an indexed base, plain
    after = line[pos + len(tok):]
    if after.startswith("["):
        return tok
    head_position = p in {None, "=", ">>", ".", "(", "->"}
    bare_statement = p is None and n is None
    if head_position and (applied or n is None or n in {">>", "."}):
        return f'<span class="f">{tok}</span>'
    return tok

def render(source):
    return "\n".join(highlight_line(l) for l in source.rstrip("\n").split("\n"))

PANEL = re.compile(
    r'(<div class="code-panel-title">([a-z_0-9/]+\.kso)[^<]*</div>\s*<pre><code>)(.*?)(</code></pre>)',
    re.S,
)

def strip_tags(html_body):
    text = re.sub(r"<[^>]+>", "", html_body)
    return html.unescape(text)

def main():
    write = "--write" in sys.argv
    drift = 0
    for ch in sorted(os.listdir(CHAPTER_DIR)):
        if not re.fullmatch(r"(ch\d+|app[a-z])\.html", ch):
            continue
        path = os.path.join(CHAPTER_DIR, ch)
        content = open(path).read()
        stem = ch.split(".")[0]
        sample_dir = os.path.join(CHAPTER_DIR, "samples", stem)
        def fix(m):
            nonlocal drift
            name = m.group(2)
            sample = os.path.join(sample_dir, name)
            candidates = [sample] + [
                os.path.join(sample_dir, d, name)
                for d in (os.listdir(sample_dir) if os.path.isdir(sample_dir) else [])
                if os.path.isdir(os.path.join(sample_dir, d))
            ]
            found = next((c for c in candidates if os.path.exists(c)), None)
            if not found:
                return m.group(0)
            source = open(found).read()
            if strip_tags(m.group(3)).rstrip("\n") == source.rstrip("\n"):
                return m.group(0)
            drift += 1
            print(f"{'rewrote' if write else 'drifted'}: {ch} :: {name}")
            if write:
                return m.group(1) + render(source) + m.group(4)
            return m.group(0)
        updated = PANEL.sub(fix, content)
        if write and updated != content:
            open(path, "w").write(updated)
    print(f"{drift} panel(s) {'rewritten' if write else 'drifted'}")
    return 0 if (write or drift == 0) else 1

if __name__ == "__main__":
    sys.exit(main())
