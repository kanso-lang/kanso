# CLAUDE.md — kanso

Guidance specific to this repo. General coding standards live in the global CLAUDE.md; this file holds what's particular to kanso. The first thing it holds is the writing voice, because the website and the book are the product, and they have to read like a person wrote them.

## Writing voice

All user-facing prose — the website, the book, the READMEs — is written in the voice of a good O'Reilly author. Plain, concrete, unhurried: the sound of someone who knows the material explaining it to you across a table. Not marketing copy, not a conference keynote. The reader is smart. Your job is to explain the material to them, not to perform it.

The failure mode is AI slop, and it has a recognizable signature. Hunt these and cut them.

**The antithesis flip.** "it doesn't X, it Ys." / "not X, but Y." / "A isn't B; it's C." This is the single most recognizable machine tell. Make the point once, in the positive, and stop.
- slop: *it doesn't work around the constraints. it spends them.*
- plain: *it treats each constraint as something to spend.*

**The epigram ending.** A short, punchy sentence bolted to the end of a paragraph to make it land.
- slop: *the rest of this page is that trade, cashed in one piece at a time.*
- plain: delete it. The paragraph already made its point.

**The self-announcing sentence.** Telling the reader how to feel about what comes next instead of writing it.
- slop: *this is the trick under the trick, and it's worth slowing down for.* / *here's the thing.* / *take this slowly.*
- plain: just say the thing.

**The milked metaphor.** One analogy, introduced once, then dropped. Don't keep returning to the janitor, the tax, the guardrail three sections later. And no precious images kept for their own sake.
- slop: *a small garbage collection wearing a trench coat* · *folklore with a decimal point* · *one chef rinsing a single cutting board*
- plain: *a small garbage collection by another name* · *isn't evidence of much* · cut it

**Manufactured rhythm.** Triples assembled for cadence rather than because there are three real things to name. Three em-dash asides in one sentence. Parallelism whose only job is to sound good. Real lists of real items are fine; decoration is not.

**Throat-clearing.** *the interesting part is…* / *it's worth noting that…* / *let's be honest.* Delete the opener and start with the content.

What to do instead: one idea per sentence, and vary the length. Concrete nouns and verbs — name the actual thing. State evidence plainly; never insist on it. An analogy earns its place only by making something clearer, and it retires after one use. Read the draft aloud: if a line sounds like a landing page, rewrite it.

One reliable smell test: if you are proud of a sentence, stop and check whether it carries meaning or only rhythm. The slop is usually the line you liked.

Mechanical check before shipping prose:
```
grep -nE "isn't .{0,60}\. it's|is not .{0,60}\. it's|—not .{0,40}, but|\. it (spends|is just how)" <file>
```
A clean grep is necessary, not sufficient — the families above are wider than any regex. Read it.
