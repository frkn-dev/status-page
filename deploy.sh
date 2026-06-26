#!/usr/bin/env bash
set -euo pipefail

# Деплой status-page на darkmachine2.frkn.org
# 1. Собирает production-сборку через trunk.
# 2. Синхронизирует dist/ на сервер в /opt/mirror/status.frkn.org.

REMOTE_HOST="darkmachine2.frkn.org"
REMOTE_PATH="/opt/mirror/status.frkn.org"

echo "==> Building status-page..."
if ! command -v trunk >/dev/null 2>&1; then
    echo "Error: trunk not found in PATH" >&2
    exit 1
fi

rm -rf dist
trunk build --release

echo "==> Packing browser extensions..."
rm -f frkn-service-checker-chrome.zip frkn-service-checker-firefox.zip

zip -r frkn-service-checker-chrome.zip extension/ -x "extension/manifest-firefox.json" >/dev/null

rm -rf /tmp/extension-firefox
cp -r extension /tmp/extension-firefox
cp /tmp/extension-firefox/manifest-firefox.json /tmp/extension-firefox/manifest.json
cd /tmp
zip -r /Users/2pizza/c/f/status-page/frkn-service-checker-firefox.zip extension-firefox/ -x "extension-firefox/manifest-firefox.json" >/dev/null
cd /Users/2pizza/c/f/status-page

cp frkn-service-checker-chrome.zip dist/
cp frkn-service-checker-firefox.zip dist/

echo "==> Deploying to ${REMOTE_HOST}:${REMOTE_PATH}..."
if ! command -v rsync >/dev/null 2>&1; then
    echo "Error: rsync not found in PATH" >&2
    exit 1
fi

rsync -avz --delete \
    --exclude='.git' \
    --exclude='speedtest/' \
    dist/ "${REMOTE_HOST}:${REMOTE_PATH}/"

echo "==> Done."
echo "    Site: https://status.frkn.org"
