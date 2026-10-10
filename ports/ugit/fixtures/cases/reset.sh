# reset moves the branch; --hard brings the files along.
ugit init
printf 'one\n' > n.txt
ugit add n.txt
tick
ugit commit -m "one"
printf 'two\n' > n.txt
printf 'extra\n' > extra.txt
ugit add .
tick
ugit commit -m "two"
printf 'local edit\n' > n.txt
printf 'untracked\n' > loose.txt
ugit reset HEAD~1
ugit log --oneline
ugit status
cat n.txt
ugit add n.txt
ugit reset --hard
ugit status
cat n.txt
ls
ugit reset --hard master
ugit reset a b
