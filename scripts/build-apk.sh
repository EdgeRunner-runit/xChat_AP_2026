#!/usr/bin/env bash
# ==============================================================================
# Build Standalone Android APK for xChat
# Produces signed dist/xchat-v0.1.0.apk
# ==============================================================================
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
ANDROID_DIR="${ROOT_DIR}/android"
BUILD_DIR="${ANDROID_DIR}/build"
DIST_DIR="${ROOT_DIR}/dist"

ANDROID_SDK="${ANDROID_HOME:-${HOME}/Library/Android/sdk}"
# Find the latest build-tools version available
BUILD_TOOLS_VERSION="$(ls -1 "${ANDROID_SDK}/build-tools" | sort -V | tail -n 1)"
BUILD_TOOLS="${ANDROID_SDK}/build-tools/${BUILD_TOOLS_VERSION}"
ANDROID_JAR="${ANDROID_SDK}/platforms/android-34/android.jar"

AAPT2="${BUILD_TOOLS}/aapt2"
D8="${BUILD_TOOLS}/d8"
ZIPALIGN="${BUILD_TOOLS}/zipalign"
APKSIGNER="${BUILD_TOOLS}/apksigner"

echo "=========================================================="
echo "          Building Standalone Android APK for xChat       "
echo " Using Android Build Tools: ${BUILD_TOOLS_VERSION}"
echo "=========================================================="

if [ ! -f "$ANDROID_JAR" ]; then
    echo "[!] Error: Android platform android-34 not found at ${ANDROID_JAR}"
    exit 1
fi

rm -rf "${BUILD_DIR}"
mkdir -p "${BUILD_DIR}/gen" "${BUILD_DIR}/classes" "${BUILD_DIR}/dex" "${DIST_DIR}"

# 1. Compile Resources
echo "[*] Step 1/7: Compiling resources with aapt2..."
"${AAPT2}" compile --dir "${ANDROID_DIR}/app/src/main/res" -o "${BUILD_DIR}/compiled_res.zip"

# 2. Link Resources & Manifest
echo "[*] Step 2/7: Linking manifest and assets..."
"${AAPT2}" link \
    -I "${ANDROID_JAR}" \
    "${BUILD_DIR}/compiled_res.zip" \
    --manifest "${ANDROID_DIR}/app/src/main/AndroidManifest.xml" \
    -A "${ANDROID_DIR}/app/src/main/assets" \
    --java "${BUILD_DIR}/gen" \
    -o "${BUILD_DIR}/app-unaligned.apk" \
    --auto-add-overlay

# 3. Compile Java Source Code
echo "[*] Step 3/7: Compiling Java sources..."
javac -source 8 -target 8 \
    -cp "${ANDROID_JAR}" \
    -d "${BUILD_DIR}/classes" \
    "${BUILD_DIR}/gen/network/xchat/app/R.java" \
    "${ANDROID_DIR}/app/src/main/java/network/xchat/app/AndroidBridge.java" \
    "${ANDROID_DIR}/app/src/main/java/network/xchat/app/MainActivity.java"

# 4. Convert Bytecode to Dalvik DEX
echo "[*] Step 4/7: Converting to DEX bytecode via d8..."
"${D8}" --lib "${ANDROID_JAR}" --min-api 21 \
    --output "${BUILD_DIR}/dex" \
    "${BUILD_DIR}"/classes/network/xchat/app/*.class

# 5. Pack DEX into APK
echo "[*] Step 5/7: Packing classes.dex into APK..."
(cd "${BUILD_DIR}/dex" && zip -u "${BUILD_DIR}/app-unaligned.apk" classes.dex)

# 6. Zipalign APK (4-byte alignment)
echo "[*] Step 6/7: Aligning APK with zipalign..."
"${ZIPALIGN}" -v -p 4 "${BUILD_DIR}/app-unaligned.apk" "${BUILD_DIR}/xchat-aligned.apk"

# 7. Sign APK with apksigner
echo "[*] Step 7/7: Signing APK with apksigner..."
KEYSTORE="${ANDROID_DIR}/debug.keystore"
if [ ! -f "$KEYSTORE" ]; then
    echo "[*] Creating production-compatible release signing keystore..."
    keytool -genkey -v -keystore "$KEYSTORE" \
        -alias xchat \
        -keyalg RSA \
        -keysize 2048 \
        -validity 10000 \
        -storepass xchatpass \
        -keypass xchatpass \
        -dname "CN=xChat Release, OU=Mobile Security, O=xChat Network, L=Global, S=Global, C=XX"
fi

APK_OUT="${DIST_DIR}/xchat-v0.1.0.apk"
"${APKSIGNER}" sign \
    --ks "$KEYSTORE" \
    --ks-pass pass:xchatpass \
    --key-pass pass:xchatpass \
    --out "${APK_OUT}" \
    "${BUILD_DIR}/xchat-aligned.apk"

# Verify signature
echo "[*] Verifying signed APK..."
"${APKSIGNER}" verify --verbose "${APK_OUT}"

# Generate Checksum
(cd "${DIST_DIR}" && shasum -a 256 "xchat-v0.1.0.apk" > "xchat-v0.1.0.apk.sha256")

echo "=========================================================="
echo " [✓] Android APK Built Successfully!"
echo " Output: ${APK_OUT}"
echo " Size:   $(ls -lh "${APK_OUT}" | awk '{print $5}')"
echo " SHA256: $(cat "${DIST_DIR}/xchat-v0.1.0.apk.sha256")"
echo "=========================================================="
