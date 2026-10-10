# Unified diffs between any two of a commit, the index and the work tree.
ugit init
printf '1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n12\n13\n14\n15\n' > numbers.txt
printf 'keep\n' > keep.txt
printf 'bye\n' > gone.txt
ugit add .
tick
ugit commit -m "base"
ugit diff
printf '1\n2\nthree\n4\n5\n6\n7\n8\n9\n10\n11\n12\n13\nfourteen\n15\n16\n' > numbers.txt
ugit diff
ugit add numbers.txt
ugit diff
ugit diff --cached
printf 'no newline at the end' > tail.txt
rm gone.txt
ugit add .
ugit diff --cached
tick
ugit commit -m "second"
ugit diff HEAD~1 HEAD
first=$(ugit log --oneline | tail -1 | cut -c1-7)
ugit diff "$first" HEAD
ugit diff "$first"
printf 'no newline at the end, still\n' > tail.txt
ugit diff
ugit diff --cached "$first"
ugit diff a b c
