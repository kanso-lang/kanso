inflate gzip fixtures/data/prose.txt -o $OUT/p.gz && gzip -dc $OUT/p.gz | cmp - fixtures/data/prose.txt && echo identical
