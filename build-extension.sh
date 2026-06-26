#!/usr/bin/env bash
set -euo pipefail

# Сборка ZIP-архивов расширения для публикации в Chrome Web Store и Firefox Add-ons.

PROJECT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "==> Building Chrome extension..."
rm -rf /tmp/frkn-extension-chrome
cp -r "${PROJECT_DIR}/extension" /tmp/frkn-extension-chrome
rm -f /tmp/frkn-extension-chrome/manifest-firefox.json
cd /tmp/frkn-extension-chrome
zip -r "${PROJECT_DIR}/frkn-service-checker-store-chrome.zip" . >/dev/null

echo "==> Building Firefox extension..."
rm -rf /tmp/frkn-extension-firefox
cp -r "${PROJECT_DIR}/extension" /tmp/frkn-extension-firefox
cp /tmp/frkn-extension-firefox/manifest-firefox.json /tmp/frkn-extension-firefox/manifest.json
rm -f /tmp/frkn-extension-firefox/manifest-firefox.json
cd /tmp/frkn-extension-firefox
zip -r "${PROJECT_DIR}/frkn-service-checker-store-firefox.zip" . >/dev/null

echo "==> Done."
ls -lh "${PROJECT_DIR}/frkn-service-checker-store-chrome.zip" "${PROJECT_DIR}/frkn-service-checker-store-firefox.zip"
