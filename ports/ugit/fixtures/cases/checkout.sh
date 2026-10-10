# Switching branches rewrites the working tree and the index, and refuses
# to throw away local changes.
ugit init
printf 'shared\n' > shared.txt
printf 'main only\n' > main.txt
ugit add .
tick
ugit commit -m "on master"
ugit checkout -b side
ugit checkout -b side
mkdir dir
printf 'side only\n' > dir/side.txt
rm main.txt
printf 'shared, changed on side\n' > shared.txt
ugit add .
tick
ugit commit -m "on side"
ls -R | grep -v '^$'
ugit checkout master
ls -R | grep -v '^$'
cat shared.txt
ugit checkout master
ugit checkout side
ls -R | grep -v '^$'
ugit checkout master
printf 'edited\n' > shared.txt
ugit checkout side
cat shared.txt
ugit checkout -- shared.txt
cat shared.txt
ugit checkout -- nothere.txt
printf 'untracked in the way\n' > main2.txt
mkdir -p dir
printf 'mine\n' > dir/side.txt
ugit checkout side
rm dir/side.txt
printf 'kept edit\n' > main.txt
ugit checkout side
ugit status
ugit checkout -- main.txt
first=$(ugit rev-parse master)
ugit checkout "$first"
ugit status
ugit branch
ugit checkout side
ugit status
ugit checkout nowhere
