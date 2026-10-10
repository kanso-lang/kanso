# The object store underneath: blobs, trees and commits by id.
ugit init
printf 'hello\n' > hello.txt
blob=$(ugit hash-object hello.txt)
echo "$blob"
prefix=$(echo "$blob" | cut -c1-4)
ugit cat-file -t "$prefix"
ugit cat-file -p "$prefix"
ugit cat-file blob "$prefix"
ugit cat-file tree "$prefix"
ugit cat-file -p
ugit hash-object
mkdir -p src/lib
printf 'fn main\n' > src/main.kso
printf 'fn lib\n' > src/lib/lib.kso
ugit add .
tree=$(ugit write-tree)
echo "$tree"
ugit cat-file -p "$tree"
ugit cat-file -t "$tree"
tick
ugit commit -m "plumbing"
ugit rev-parse HEAD
ugit rev-parse @
ugit rev-parse master
ugit rev-parse refs/heads/master
ugit cat-file -p HEAD
ugit rev-parse 0000
ugit rev-parse zz
ls .ugit/objects | wc -l
