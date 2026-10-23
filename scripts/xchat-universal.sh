#!/usr/bin/env bash
# ==============================================================================
# xChat Universal Launcher & Auto-Setup
# Supports: macOS (Apple Silicon & Intel), Linux, Android (Termux), Windows (Git Bash/MSYS)
# ==============================================================================
set -e

PORT="${XCHAT_PORT:-4000}"
XCHAT_HOME="${HOME}/.xchat"
mkdir -p "${XCHAT_HOME}/bin"

echo "============================================================="
echo "             xChat Universal Auto-Launcher v0.1.0            "
echo "   Decentralized Multi-Transport End-to-End Private Messenger"
echo "============================================================="

OS_RAW="$(uname -s 2>/dev/null || echo "Unknown")"
ARCH_RAW="$(uname -m 2>/dev/null || echo "Unknown")"

IS_ANDROID=false
if [ -d "/data/data/com.termux" ] || [ -n "${TERMUX_VERSION:-}" ] || [[ "${PREFIX:-}" == *"com.termux"* ]]; then
    IS_ANDROID=true
    PLATFORM="Android (Termux)"
elif [ "$OS_RAW" = "Darwin" ]; then
    PLATFORM="macOS ($ARCH_RAW)"
elif [ "$OS_RAW" = "Linux" ]; then
    PLATFORM="Linux ($ARCH_RAW)"
elif [[ "$OS_RAW" =~ MINGW|MSYS|CYGWIN ]]; then
    PLATFORM="Windows ($OS_RAW)"
else
    PLATFORM="$OS_RAW ($ARCH_RAW)"
fi

echo "[*] Detected Platform: ${PLATFORM}"

# Locate existing xChat binary
XCHAT_BIN=""
if [ -x "./xchat" ]; then
    XCHAT_BIN="$(pwd)/xchat"
elif [ -x "./target/release/xchat" ]; then
    XCHAT_BIN="$(pwd)/target/release/xchat"
elif [ -x "./target/release/xchat-cli" ]; then
    XCHAT_BIN="$(pwd)/target/release/xchat-cli"
elif [ -x "${XCHAT_HOME}/bin/xchat" ]; then
    XCHAT_BIN="${XCHAT_HOME}/bin/xchat"
elif command -v xchat >/dev/null 2>&1; then
    XCHAT_BIN="$(command -v xchat)"
fi

# If binary not found, auto-setup based on detected OS
if [ -z "$XCHAT_BIN" ]; then
    echo "[*] xChat binary not found locally. Starting automatic setup..."
    
    if [ "$OS_RAW" = "Darwin" ]; then
        echo "[*] Fetching pre-compiled Universal macOS package from GitHub Releases..."
        TAR_URL="https://github.com/EdgeRunner-runit/xChat_AP_2026/releases/download/v0.1.0/xchat-v0.1.0-macos-universal.tar.gz"
        TMP_PKG="$(mktemp /tmp/xchat-pkg.XXXXXX.tar.gz)"
        curl -fsSL "$TAR_URL" -o "$TMP_PKG"
        tar -xzf "$TMP_PKG" -C "${XCHAT_HOME}/bin"
        rm -f "$TMP_PKG"
        chmod +x "${XCHAT_HOME}/bin/xchat"
        XCHAT_BIN="${XCHAT_HOME}/bin/xchat"
        echo "[✓] Download and installation complete: ${XCHAT_BIN}"
        
    elif [ "$IS_ANDROID" = "true" ]; then
        echo "[*] Android Termux detected. Checking dependencies..."
        if ! command -v cargo >/dev/null 2>&1; then
            echo "[*] Installing Rust toolchain via pkg..."
            pkg update -y && pkg install -y rust git build-essential
        fi
        echo "[*] Building xChat for Android..."
        SRC_DIR="${XCHAT_HOME}/src"
        mkdir -p "$SRC_DIR"
        if [ ! -d "${SRC_DIR}/xChat_AP_2026" ]; then
            git clone https://github.com/EdgeRunner-runit/xChat_AP_2026.git "${SRC_DIR}/xChat_AP_2026"
        fi
        (cd "${SRC_DIR}/xChat_AP_2026" && cargo build --release)
        cp "${SRC_DIR}/xChat_AP_2026/target/release/xchat" "${XCHAT_HOME}/bin/xchat"
        chmod +x "${XCHAT_HOME}/bin/xchat"
        XCHAT_BIN="${XCHAT_HOME}/bin/xchat"
        echo "[✓] Android build complete: ${XCHAT_BIN}"
        
    elif [ "$OS_RAW" = "Linux" ]; then
        echo "[*] Linux environment detected."
        if command -v cargo >/dev/null 2>&1; then
            echo "[*] Compiling xChat via cargo install..."
            cargo install --git https://github.com/EdgeRunner-runit/xChat_AP_2026.git xchat-cli --root "${XCHAT_HOME}"
            XCHAT_BIN="${XCHAT_HOME}/bin/xchat"
        else
            echo "[!] Cargo not found. Please install Rust from https://rustup.rs or install prebuilt binary."
            exit 1
        fi
        
    elif [[ "$OS_RAW" =~ MINGW|MSYS|CYGWIN ]]; then
        echo "[*] Windows Bash environment detected."
        if command -v cargo >/dev/null 2>&1; then
            cargo install --git https://github.com/EdgeRunner-runit/xChat_AP_2026.git xchat-cli --root "${XCHAT_HOME}"
            XCHAT_BIN="${XCHAT_HOME}/bin/xchat.exe"
        else
            echo "[!] Cargo not found. Please install Rust or use the Windows batch launcher."
            exit 1
        fi
    fi
fi

if [ ! -x "$XCHAT_BIN" ]; then
    echo "[!] Error: Could not locate or build xChat binary."
    exit 1
fi

# Ensure cryptographic identity exists
KEYSTORE="${XCHAT_HOME}/keystore.json"
if [ ! -f "$KEYSTORE" ]; then
    echo "[*] First run: Generating local cryptographic identity and prekey bundle..."
    "$XCHAT_BIN" keygen
else
    echo "[*] Cryptographic identity loaded from ${KEYSTORE}"
fi

APP_URL="http://127.0.0.1:${PORT}"
echo ""
echo "============================================================="
echo "   xChat is ready! Launching App GUI..."
echo "   URL: ${APP_URL}"
echo "============================================================="
echo ""

# Auto-open browser in background
(
    sleep 1
    if [ "$IS_ANDROID" = "true" ]; then
        termux-open-url "$APP_URL" 2>/dev/null || am start -a android.intent.action.VIEW -d "$APP_URL" 2>/dev/null || true
    elif [ "$OS_RAW" = "Darwin" ]; then
        open "$APP_URL" 2>/dev/null || true
    elif [ "$OS_RAW" = "Linux" ]; then
        xdg-open "$APP_URL" 2>/dev/null || sensible-browser "$APP_URL" 2>/dev/null || x-www-browser "$APP_URL" 2>/dev/null || true
    elif [[ "$OS_RAW" =~ MINGW|MSYS|CYGWIN ]]; then
        start "$APP_URL" 2>/dev/null || true
    fi
) &

# Execute GUI server
exec "$XCHAT_BIN" app --port "$PORT"
