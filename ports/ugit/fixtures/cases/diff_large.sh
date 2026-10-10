# A 3,000-line file with scattered edits. The diff is checked by size and
# by its first hunks; before delta/myers.kso stopped reading the map it was
# writing, this took seconds on the compiled engines (FRICTION F28).
ugit init
seq 1 3000 | sed 's/^/line /' > big.txt
ugit add big.txt
tick
ugit commit -m "big"
sed -e 's/^line 17$/line seventeen/' -e '/^line 500$/d' -e 's/^line 2999$/&\nline 2999.5/' big.txt > big.new
mv big.new big.txt
ugit diff | wc -l
ugit diff | head -20
ugit add big.txt
tick
ugit commit -m "edits"
ugit diff HEAD~1 HEAD | tail -9
