# The day's customers, one request at a time.
at=http://127.0.0.1:$1
curl -s $at/menu
curl -s -d 'dango 2' $at/order
curl -s -d 'mochi 1' $at/order
curl -s -d 'taiyaki 1' $at/order
curl -s $at/sales
curl -s -X POST $at/close
