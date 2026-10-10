# Deletions merge too; a file deleted on one side and changed on the other
# is a conflict, and so is the same new file added two different ways.
ugit init
printf 'stays\n' > stays.txt
printf 'doomed\n' > doomed.txt
printf 'contested\n' > contested.txt
ugit add .
tick
ugit commit -m "base"
ugit checkout -b other
rm doomed.txt contested.txt
printf 'from other\n' > both.txt
ugit add .
tick
ugit commit -m "other: delete two, add both.txt"
ugit checkout master
printf 'contested, edited on master\n' > contested.txt
printf 'from master\n' > both.txt
ugit add .
tick
ugit commit -m "master: edit contested, add both.txt"
ugit merge other
ls
cat both.txt contested.txt
ugit status
