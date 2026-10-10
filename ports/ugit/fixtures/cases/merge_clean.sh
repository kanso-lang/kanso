# Changes to different files, and to different parts of one file, merge
# without help and are committed with both parents.
ugit init
printf 'title\n\nalpha\nbeta\ngamma\ndelta\nepsilon\n\nend\n' > story.txt
printf 'notes\n' > notes.txt
printf 'old\n' > old.txt
ugit add .
tick
ugit commit -m "base"
ugit branch feature
printf 'TITLE\n\nalpha\nbeta\ngamma\ndelta\nepsilon\n\nend\n' > story.txt
printf 'master file\n' > master.txt
ugit add .
tick
ugit commit -m "master: title, new file"
ugit checkout feature
printf 'title\n\nalpha\nbeta\ngamma\ndelta\nepsilon\n\nTHE END\n' > story.txt
printf 'notes, and more\n' > notes.txt
rm old.txt
ugit add .
tick
ugit commit -m "feature: ending, notes, drop old"
ugit checkout master
tick
ugit merge feature
cat story.txt notes.txt master.txt
ls
ugit status
ugit log
ugit merge-base master feature
