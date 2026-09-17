#!/usr/bin/env bash
# Publish this tree to github.com/westerngazoo/guion-video-creator using YOUR GitHub credentials.
# The Cursor Cloud Agent token cannot do this (scoped to fisicobuenfisico only).
#
# Usage (from the guion repo root, after you created the empty repo on GitHub):
#   export GH_TOKEN='ghp_...'   # PAT with repo + admin:org (to create) or repo (to push)
#   ./scripts/publish-to-github.sh
#
# Or if the repo already exists and you use gh auth login:
#   gh auth login
#   ./scripts/publish-to-github.sh

set -euo pipefail

ORG="westerngazoo"
REPO="guion-video-creator"
REMOTE="https://github.com/${ORG}/${REPO}.git"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EXPORT="${EXPORT_DIR:-/tmp/guion-video-creator-export}"

if [[ "${USE_EXPORT:-1}" == "1" ]]; then
  GVC_REPO="${GVC_REPO:-$(mktemp -d)}"
  if [[ ! -d "$GVC_REPO/docs" ]]; then
    git clone --depth 1 "https://github.com/${ORG}/${REPO}.git" "$GVC_REPO" 2>/dev/null || true
  fi
  "$ROOT/scripts/export-standalone.sh" "$EXPORT"
  cd "$EXPORT"
else
  cd "$ROOT"
fi

if ! command -v gh >/dev/null; then
  echo "error: install GitHub CLI (gh)" >&2
  exit 1
fi

if [[ -n "${GH_TOKEN:-}" ]]; then
  export GH_TOKEN
  echo "$GH_TOKEN" | gh auth login --with-token 2>/dev/null || true
fi

if ! gh auth status >/dev/null 2>&1; then
  echo "error: not authenticated. Run: gh auth login   OR   export GH_TOKEN=ghp_..." >&2
  exit 1
fi

if ! gh repo view "${ORG}/${REPO}" >/dev/null 2>&1; then
  echo "creating ${ORG}/${REPO} (private)..."
  gh repo create "${ORG}/${REPO}" --private \
    --description "Declarative framework for physics animation (screenplay → video)" \
    --source . --remote origin --push
  echo "done: ${REMOTE}"
  exit 0
fi

echo "repo exists; pushing main..."
if git rev-parse --git-dir >/dev/null 2>&1; then
  git remote remove origin 2>/dev/null || true
  git remote add origin "$REMOTE"
  git push -u origin main
else
  git init -b main
  git add -A
  git commit -m "guion: initial import (guion-core + docs + CI)" || true
  git remote add origin "$REMOTE"
  git push -u origin main
fi

echo "done: ${REMOTE}"
