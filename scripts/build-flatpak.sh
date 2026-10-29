#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

if [ "${1:-}" = "--bundle" ]; then
  echo "==> Building Flatpak repository and single-file bundle..."
  flatpak-builder --user --repo=repo --force-clean build-dir flatpak/com.kumar.checkd.yml
  flatpak build-bundle repo checkd.flatpak com.kumar.checkd
  echo ""
  echo "==> Done! Created checkd.flatpak"
else
  echo "==> Building and installing Flatpak..."
  flatpak-builder --user --install --force-clean build-dir flatpak/com.kumar.checkd.yml
  echo ""
  echo "==> Done! You can run the app with:"
  echo "    flatpak run com.kumar.checkd"
fi
