"""Put a chapter out of date in the way that exercises the write path.

Two things have to be true before `--write` does the work this gate exists to
watch. A panel must have drifted, or nothing is rewritten at all. And a panel's
recorded output must be missing, because that is the branch — `no_recorded` —
that reaches the escaping the runaway lived in. Staling a body alone leaves
that branch untaken and gates nothing.
"""

import os
import sys

chapter, samples = sys.argv[1], sys.argv[2]

source = open(chapter).read()
marker = "<pre><code>"
at = source.index(marker, source.index("knot.kso"))
cut = at + len(marker)
open(chapter, "w").write(source[:cut] + "STALE" + source[cut:])

os.remove(os.path.join(samples, "knot.out"))
