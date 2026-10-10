# Staging and committing: files, directories, deletions, and the cases
# where there is nothing to record.
ugit init
ugit add
ugit add missing.txt
ugit commit -m "empty"
printf 'one\n' > one.txt
mkdir -p docs/notes
printf 'readme\n' > docs/readme.md
printf 'a note\n' > docs/notes/a.md
ugit add one.txt
ugit status
ugit add ./docs/
ugit status
tick
ugit commit -m "first commit

with a body that spans
two lines"
ugit commit -m "again"
ugit log
rm docs/notes/a.md
printf 'two\n' > two.txt
ugit add docs two.txt
ugit status
tick
ugit commit
ugit commit -m "remove a note, add two"
ugit log --oneline
ugit cat-file -p HEAD
