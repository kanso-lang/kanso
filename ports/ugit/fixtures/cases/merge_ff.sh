# A merge whose base is HEAD only moves the branch forward.
ugit init
printf 'v1\n' > app.txt
ugit add app.txt
tick
ugit commit -m "v1"
ugit checkout -b feature
printf 'v2\n' > app.txt
printf 'added\n' > extra.txt
ugit add .
tick
ugit commit -m "v2"
ugit checkout master
ugit merge feature
ugit log --oneline
cat app.txt extra.txt
ugit merge feature
ugit merge-base master feature
ugit merge
ugit merge nowhere
