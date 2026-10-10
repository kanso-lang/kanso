# When both sides change the same lines, the merge stops with conflict
# markers, and the next commit concludes it.
ugit init
printf 'a\nb\nc\nd\ne\n' > letters.txt
ugit add letters.txt
tick
ugit commit -m "letters"
ugit checkout -b upper
printf 'a\nB\nc\nd\nE\n' > letters.txt
ugit add letters.txt
tick
ugit commit -m "upper b and e"
ugit checkout master
printf 'a\nbee\nc\nd\ne\n' > letters.txt
ugit add letters.txt
tick
ugit commit -m "spell b"
ugit merge upper
cat letters.txt
cat .ugit/MERGE_MSG
ugit status
ugit diff
ugit merge upper
printf 'a\nBee\nc\nd\nE\n' > letters.txt
ugit add letters.txt
ugit status
tick
ugit commit
ugit log --oneline
ugit show
ugit status
ls .ugit
