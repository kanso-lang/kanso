# show prints a commit's log entry and what it changed.
ugit init
printf 'first line\n' > f.txt
ugit add f.txt
tick
ugit commit -m "add f"
printf 'first line\nsecond line\n' > f.txt
printf 'g\n' > g.txt
ugit add .
tick
ugit commit -m "grow f, add g"
ugit tag grown
ugit show
ugit show grown
id=$(ugit rev-parse master | cut -c1-12)
ugit show "$id"
ugit show nothing
ugit show HEAD~1
ugit show HEAD^^
ugit show HEAD~5
