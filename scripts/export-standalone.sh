#!/usr/bin/env bash
# Build a standalone tree for westerngazoo/guion-video-creator:
#   - Rust workspace (crates, templates, CI, docs from guion/)
#   - ENCARGO fixtures/docs from the existing guion-video-creator repo
#
# Usage:
#   ./scripts/export-standalone.sh [output-dir]
# Default output: /tmp/guion-video-creator-export

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-/tmp/guion-video-creator-export}"
GVC="${GVC_REPO:-/tmp/gvc-test}"

rm -rf "$OUT"
mkdir -p "$OUT"

tar -C "$ROOT" \
  --exclude target \
  --exclude '.git' \
  -cf - . | tar -C "$OUT" -xf -

if [[ -d "$GVC/docs" ]]; then
  mkdir -p "$OUT/docs/encargo"
  cp -a "$GVC/docs/." "$OUT/docs/encargo/"
fi
if [[ -d "$GVC/fixtures" ]]; then
  cp -a "$GVC/fixtures" "$OUT/"
fi
if [[ -f "$GVC/README.md" ]]; then
  cp "$GVC/README.md" "$OUT/README-ENCARGO.md"
fi

cat > "$OUT/PUBLISH.md" <<'EOF'
# Publishing this tree

The Cursor Cloud Agent (`cursor[bot]`) cannot push to `guion-video-creator`.
Publish with your credentials:

```bash
export GH_TOKEN='ghp_...'   # or: gh auth login
./scripts/publish-to-github.sh
```

Or apply the bundle:

```bash
git clone https://github.com/westerngazoo/guion-video-creator.git
cd guion-video-creator
git pull
git bundle unbundle ../guion-video-creator.bundle
git push origin main
```
EOF

echo "export → $OUT"
