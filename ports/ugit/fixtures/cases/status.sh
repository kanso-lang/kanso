# Status sorts every path into staged, unstaged and untracked.
ugit init
printf 'a\n' > a.txt
printf 'b\n' > b.txt
printf 'c\n' > c.txt
ugit add a.txt b.txt c.txt
tick
ugit commit -m "three files"
ugit status
printf 'a2\n' > a.txt
ugit add a.txt
printf 'a3\n' > a.txt
rm b.txt
printf 'new\n' > d.txt
mkdir sub
printf 'deep\n' > sub/e.txt
ugit status
ugit add b.txt
ugit status
ugit add .
ugit status
tick
ugit commit -m "everything"
ugit status
