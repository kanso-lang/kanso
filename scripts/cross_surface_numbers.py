#!/usr/bin/env python3
"""One number, claimed twice, must be claimed the same.

A sitting's figures land on several pages: the decode board on the compiler
page, the same four rows as a code panel on the landing page, the same
kanso-versus-serde pair again in the landing prose. Each is edited by hand,
and the log records what happens when the sweep runs on memory — figure sets
that disagree across pages for a day at a time.

This does not know what the numbers should be. It knows which claims are the
same claim, and refuses to let two of them differ.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def decode_board():
    """The compiler page's table: ms per decode and peak, by decoder."""
    page = (ROOT / "docs/compiler.html").read_text()
    start = page.index('<table class="bench-table">')
    table = page[start : page.index("</table>", start)]
    rows = re.findall(
        r"<td>(.*?)</td>.*?"
        r'<td class="num">([0-9.]+)</td><td class="num">([0-9.]+) mb</td>',
        table,
        re.S,
    )
    out = {}
    for label, ms, mb in rows:
        name = re.sub(r"<[^>]+>", "", label).strip()
        key = name.split()[0]
        out[key] = (ms, mb)
    return out


def landing_panel():
    """The landing page's code panel: the same four rows, hand-typed."""
    page = (ROOT / "docs/index.html").read_text()
    rows = re.findall(r"# ([a-z_]+)[^\d\n]*([0-9]+\.[0-9]+)", page)
    return {name: ms for name, ms in rows}


def landing_prose():
    """The landing page's own sentence quoting the same sitting."""
    page = (ROOT / "docs/index.html").read_text()
    ms = re.findall(r"<strong>~([0-9.]+)&nbsp;ms</strong>", page)
    serde = re.search(r"serde_json</code> at ~([0-9.]+)&nbsp;ms", page)
    peak = re.search(r"flat ~([0-9.]+)&nbsp;mb", page)
    return {
        "kanso_ms": ms[0] if ms else None,
        "serde_ms": serde.group(1) if serde else None,
        "kanso_mb": peak.group(1) if peak else None,
    }


def main():
    board = decode_board()
    panel = landing_panel()
    prose = landing_prose()
    problems = []

    pairs = [("kanso", "kanso"), ("serde_json", "serde_json"), ("go", "go")]
    for board_key, panel_key in pairs:
        if board_key in board and panel_key in panel:
            if board[board_key][0] != panel[panel_key]:
                problems.append(
                    f"{board_key}: compiler board says {board[board_key][0]} ms, "
                    f"landing panel says {panel[panel_key]} ms"
                )

    if prose["kanso_ms"] and "kanso" in board:
        if prose["kanso_ms"] != board["kanso"][0]:
            problems.append(
                f"kanso: compiler board says {board['kanso'][0]} ms, "
                f"landing prose says {prose['kanso_ms']} ms"
            )
    if prose["serde_ms"] and "serde_json" in board:
        if prose["serde_ms"] != board["serde_json"][0]:
            problems.append(
                f"serde_json: compiler board says {board['serde_json'][0]} ms, "
                f"landing prose says {prose['serde_ms']} ms"
            )
    if prose["kanso_mb"] and "kanso" in board:
        if prose["kanso_mb"] != board["kanso"][1]:
            problems.append(
                f"kanso peak: compiler board says {board['kanso'][1]} mb, "
                f"landing prose says {prose['kanso_mb']} mb"
            )

    if not board or not panel:
        print("no board or panel found — the pattern this reads has moved")
        return 1
    for line in problems:
        print(f"disagree: {line}")
    if problems:
        print()
        print("the same sitting is published in more than one place; move them")
        print("together or the pages contradict each other.")
        return 1
    print(f"{len(board)} board rows agree with the landing panel and prose")
    return 0


if __name__ == "__main__":
    sys.exit(main())
