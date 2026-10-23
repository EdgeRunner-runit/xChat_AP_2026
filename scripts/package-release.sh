#!/usr/bin/env bash
set -euo pipefail

VERSION="0.1.0"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
DIST_DIR="${ROOT_DIR}/dist"

mkdir -p "${DIST_DIR}"
cd "${ROOT_DIR}"

echo "Building release package for xChat v${VERSION}..."

# Build arm64 (Apple Silicon)
cargo build --release --target aarch64-apple-darwin

# Build x86_64 (Intel Mac)
cargo build --release --target x86_64-apple-darwin

# Create Universal macOS binary
mkdir -p target/release
lipo -create -output target/release/xchat-macos-universal \
    target/aarch64-apple-darwin/release/xchat-cli \
    target/x86_64-apple-darwin/release/xchat-cli

# Package Universal macOS
mkdir -p "${DIST_DIR}/pkg-universal"
cp target/release/xchat-macos-universal "${DIST_DIR}/pkg-universal/xchat"
cp README.md "${DIST_DIR}/pkg-universal/"
cp docs/USER_MANUAL.md "${DIST_DIR}/pkg-universal/"
tar -czvf "${DIST_DIR}/xchat-v${VERSION}-macos-universal.tar.gz" -C "${DIST_DIR}/pkg-universal" .
shasum -a 256 "${DIST_DIR}/xchat-v${VERSION}-macos-universal.tar.gz" > "${DIST_DIR}/xchat-v${VERSION}-macos-universal.sha256"
rm -rf "${DIST_DIR}/pkg-universal"

# Package ARM64
mkdir -p "${DIST_DIR}/pkg-arm64"
cp target/aarch64-apple-darwin/release/xchat-cli "${DIST_DIR}/pkg-arm64/xchat"
cp README.md "${DIST_DIR}/pkg-arm64/"
cp docs/USER_MANUAL.md "${DIST_DIR}/pkg-arm64/"
tar -czvf "${DIST_DIR}/xchat-v${VERSION}-macos-arm64.tar.gz" -C "${DIST_DIR}/pkg-arm64" .
shasum -a 256 "${DIST_DIR}/xchat-v${VERSION}-macos-arm64.tar.gz" > "${DIST_DIR}/xchat-v${VERSION}-macos-arm64.sha256"
rm -rf "${DIST_DIR}/pkg-arm64"

# Package Intel x86_64
mkdir -p "${DIST_DIR}/pkg-x86_64"
cp target/x86_64-apple-darwin/release/xchat-cli "${DIST_DIR}/pkg-x86_64/xchat"
cp README.md "${DIST_DIR}/pkg-x86_64/"
cp docs/USER_MANUAL.md "${DIST_DIR}/pkg-x86_64/"
tar -czvf "${DIST_DIR}/xchat-v${VERSION}-macos-x86_64.tar.gz" -C "${DIST_DIR}/pkg-x86_64" .
shasum -a 256 "${DIST_DIR}/xchat-v${VERSION}-macos-x86_64.tar.gz" > "${DIST_DIR}/xchat-v${VERSION}-macos-x86_64.sha256"
rm -rf "${DIST_DIR}/pkg-x86_64"

echo "=========================================================="
echo "Release packages created in ${DIST_DIR}:"
ls -lh "${DIST_DIR}"/*.tar.gz
echo "=========================================================="
