# Two hundred keys, each written three times, then merged.
export KANSO_NOW=1700000000000
i=1
while [ $i -le 200 ]; do
  echo "put key$i first"
  echo "put key$i second"
  echo "put key$i value$i"
  i=$((i + 1))
done > script.txt
echo "delete key7" >> script.txt
echo "status" >> script.txt
echo "merge" >> script.txt
echo "status" >> script.txt
kv --max-file-size 4096 "$STORE" < script.txt | grep -v "^ok$"
kv "$STORE" get key200
kv "$STORE" get key7
kv "$STORE" list | wc -l
