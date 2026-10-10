# History, newest first, with the refs that point into it.
ugit init
ugit log
for n in 1 2 3; do
  printf '%s\n' "$n" > "file$n.txt"
  ugit add "file$n.txt"
  tick
  ugit commit -m "commit number $n"
done
ugit log
ugit tag v1.0 HEAD
ugit branch older master
ugit log --oneline
first=$(ugit log --oneline | tail -1 | cut -c1-7)
ugit tag start "$first"
ugit log --oneline
ugit log --oneline "$first"
ugit checkout "$first"
ugit log --oneline master
ugit log --oneline
ugit log nowhere
ugit log a b
