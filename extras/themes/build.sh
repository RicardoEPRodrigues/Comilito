#!/usr/bin/env bash
# Packs each theme folder here into a .comilito-theme file (a zip; see docs/themes.md) in dist/.
# Import the result from Preferences › Appearance › "Import theme file…".
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p dist
for dir in */; do
  dir=${dir%/}
  [[ -f "$dir/theme.json" ]] || continue
  # The package is named after the theme ("name" in theme.json).
  name=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["name"])' "$dir/theme.json")
  out="dist/$name.comilito-theme"
  rm -f "$out"
  # -X leaves out extra file attributes; -r takes the fonts/ folder too.
  (cd "$dir" && zip -q -r -X "../$out" .)
  echo "built $out"
done
