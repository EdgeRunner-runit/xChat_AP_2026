#!/usr/bin/env bash
set -euo pipefail

VERSION="0.1.0"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
DIST_DIR="${ROOT_DIR}/dist"

echo "Building release package for xChat v${VERSION}..."
cd "${ROOT_DIR}"

cargo build --release

mkdir -p "${DIST_DIR}/xchat-macos-arm64"
cp target/release/xchat-cli "${DIST_DIR}/xchat-macos-arm64/xchat"
cp README.md "${DIST_DIR}/xchat-macos-arm64/"
cp docs/USER_MANUAL.md "${DIST_DIR}/xchat-macos-arm64/"

cd "${DIST_DIR}"
tar -czvf "xchat-v${VERSION}-macos-arm64.tar.gz" -C xchat-macos-arm64 .
shasum -a 256 "xchat-v${VERSION}-macos-arm64.tar.gz" > "xchat-v${VERSION}-macos-arm64.sha256"
rm -rf xchat-macos-arm64

echo "Release package created successfully at: ${DIST_DIR}/xchat-v${VERSION}-macos-arm64.tar.gz"
cat "${DIST_DIR}/xchat-v${VERSION}-macos-arm64.sha256"
