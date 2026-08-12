#!/bin/sh
set -e
if grep -rEn 'src="/(kanso-engine|play|landing-play)\.js"' ./_site --include=*.html; then
  echo "::error::an undigested script reference survived"; exit 1
fi
if grep -rn "fetch('kanso.wasm')" ./_site --include=*.js; then
  echo "::error::the engine still fetches the undigested module"; exit 1
fi
