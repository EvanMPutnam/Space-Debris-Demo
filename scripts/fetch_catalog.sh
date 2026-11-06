#!/usr/bin/env bash
# Builds assets/catalog.tle from several CelesTrak queries, de-duplicated by
# NORAD catalog number. Run from anywhere; paths resolve relative to the repo.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
out="$root/assets/catalog.tle"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

base="https://celestrak.org/NORAD/elements/gp.php"
queries=(
  "GROUP=active"
  "NAME=DEB"
  "NAME=R/B"
  "GROUP=cosmos-1408-debris"
  "GROUP=fengyun-1c-debris"
  "GROUP=iridium-33-debris"
  "GROUP=cosmos-2251-debris"
)

i=0
failed=0
for q in "${queries[@]}"; do
  [ "$i" -gt 0 ] && sleep 2
  file="$tmp/$(printf '%02d' "$i").tle"
  # --retry covers CelesTrak's intermittent HTTP 500s.
  if ! curl -sS --fail --retry 4 --retry-delay 10 -A "Space-Debris-Demo fetch_catalog.sh" \
    "$base?$q&FORMAT=tle" | tr -d '\r' > "$file"; then
    : > "$file"
  fi
  # CelesTrak returns a plain-text message (e.g. rate limit, no match) instead of TLEs on error.
  n="$(grep -c '^1 ' "$file" || true)"
  if [ "$n" -eq 0 ]; then
    echo "$q: FAILED, no TLEs; response: $(head -c 200 "$file")" >&2
    failed=1
  else
    echo "$q: $n objects" >&2
  fi
  i=$((i + 1))
done

if [ "$failed" -ne 0 ]; then
  echo "one or more queries failed; leaving $out unchanged" >&2
  exit 1
fi

# Keep well-formed 3-line records and drop repeated NORAD numbers (line 1, cols 3-7).
cat "$tmp"/*.tle | awk '
  { sub(/[ \t]+$/, "") }
  { lines[++n] = $0 }
  END {
    for (i = 1; i + 2 <= n; ) {
      if (substr(lines[i+1], 1, 2) == "1 " && substr(lines[i+2], 1, 2) == "2 ") {
        id = substr(lines[i+1], 3, 5)
        if (!(id in seen)) {
          seen[id] = 1
          print lines[i]; print lines[i+1]; print lines[i+2]
          kept++
        }
        i += 3
      } else {
        i++
      }
    }
    print kept " unique objects" > "/dev/stderr"
  }' > "$out.tmp"

mv "$out.tmp" "$out"
echo "wrote $out" >&2
