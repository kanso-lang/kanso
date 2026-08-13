"""Put a chapter out of date in the way that exercises the write path.

Both panel shapes are staled, because they reach different code and only one
of them touches the escaping. A source panel is regenerated from the .kso. A
recorded-output panel is compared against the .out file and rewritten from it
when the two disagree, which is where a recorded body gets escaped into html.

A chapter carrying both is what this needs. ch03 has no output panel at all,
so its title is only ever a `.kso` name and the comparison never runs.

Deleting the .out reaches neither: a recorded output that is missing leaves
the panel alone, on the reading that the chapter may be quoting something the
book does not own.
"""

import sys

chapter = sys.argv[1]

source = open(chapter).read()
marker = "<pre><code>"

for title in ('code-panel-title">railway.kso', 'code-panel-title">kanso play railway.kso'):
    at = source.index(marker, source.index(title))
    cut = at + len(marker)
    source = source[:cut] + "STALE" + source[cut:]

open(chapter, "w").write(source)
