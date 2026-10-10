inflate cat fixtures/data/noise.zlib -o $OUT/noise && gzip -dc fixtures/data/noise.gz | cmp - $OUT/noise && echo identical
