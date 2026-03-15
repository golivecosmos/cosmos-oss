#!/bin/bash

set -euo pipefail

echo "[build-and-notarize] Starting secure production build"

if [[ "$(uname)" != "Darwin" ]]; then
  echo "[build-and-notarize] ERROR: macOS is required for signing/notarization"
  exit 1
fi

if ! command -v pnpm >/dev/null 2>&1; then
  echo "[build-and-notarize] ERROR: pnpm is not installed"
  exit 1
fi

if ! command -v xcrun >/dev/null 2>&1; then
  echo "[build-and-notarize] ERROR: xcrun is not available (install Xcode command line tools)"
  exit 1
fi

NOTARY_PROFILE="${COSMOS_NOTARY_PROFILE:-notarytool-profile}"

echo "[build-and-notarize] Using notary profile: ${NOTARY_PROFILE}"
echo "[build-and-notarize] This command enforces signing + notarization + security audit"

node scripts/release.mjs --notary-profile "${NOTARY_PROFILE}"

echo "[build-and-notarize] Complete"
