# Branches and tags are files under refs/ holding a commit id.
ugit init
ugit branch
ugit branch early
printf 'x\n' > x.txt
ugit add x.txt
tick
ugit commit -m "x"
ugit branch
ugit branch topic
ugit branch topic
ugit branch fix HEAD
ugit branch
cat .ugit/refs/heads/topic
ugit tag
ugit tag v1
ugit tag v1
ugit tag
ugit rev-parse v1
ugit branch a b c
ugit branch bad nosuchthing
