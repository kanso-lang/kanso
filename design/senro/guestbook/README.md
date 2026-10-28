# The guestbook, as a senro prototype

The first application senro is built for (#2178), running on the pieces that
are still on branches: `net/dial` and `net/read_bytes`, the Postgres client in
`../pg`, derivations (#2186), sha256's HMAC and PBKDF2 with base64, a server
that waits as one statement, and kata's `- import` line.

```
kanso run design/senro/guestbook      # Postgres on 127.0.0.1:54329, trust login
kanso derived design/senro/guestbook  # the schema and route table derived
```

- `main.kso` connects, creates each table the schema names if it is
  missing, and serves the route table that `derive routes/rest` declares.
- `migrations.kso` keeps the schema as the changes that built it, as a pub
  constant. `derive models/tables` folds them into `schema` while the app
  compiles, and refuses a record type whose table is missing or whose field
  has no column: `mesage` in `entries/signature` is a compile error at the
  type.
- `../routes` is the convention: `home/index` answers GET /, `entries/index`
  GET /entries, `entries/create` POST /entries. Any other pub function in the
  app's modules is refused where it is declared, so `entries/destory` is a
  compile error rather than a route.
- `../web` serves the table, one connection at a time, and decodes a posted
  form.
- `entries.kso` declares the `signature` model, stores one with a
  parameterized insert, so what a visitor types is stored as text and never
  read as SQL, and renders the list.
- `views/signed.slim` is the page, compiled by kata to `views/signed.kso`.
  It reads the `signature` records `entries.kso` builds by their fields,
  which needs no import.
