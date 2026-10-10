Links come in several shapes: [inline](https://example.com/a?b=c&d=e "Title"),
[full reference][ref], [collapsed][], [shortcut], and autolinks such as
<https://commonmark.org> and <someone@example.com>.

Images too: ![a *small* logo](/logo.png 'logo') and ![ref image][logo].

Destinations can hold spaces in angle brackets: [x](<my file.md>), and
non-ASCII text is percent-encoded: [ü](/naïve path).

[ref]: https://example.com/ref  "Ref Title"
[collapsed]: /collapsed
[shortcut]: </short cut> (paren title)
[logo]: /img/logo.svg
[ünïcödé]: /unicode

Labels match without regard to case or spacing: [UNÏCÖDÉ] and [Full
Reference][REF].

Entities: &copy; &amp; &#169; &#xA9; &ClockwiseContourIntegral; &notanentity;
and escapes: \*not emphasis\* \[not a link\] \\ backslash.
