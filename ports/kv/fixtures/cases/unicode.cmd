# Keys and values are text, stored as UTF-8: the size fields count bytes.
export KANSO_NOW=1700000000000
kv "$STORE" put "café" "抹茶ラテ"
bytes "$STORE/1.bitcask.data"
kv "$STORE" get "café"
kv "$STORE" put "" empty-key
kv "$STORE" put empty-value ""
kv "$STORE" dump
