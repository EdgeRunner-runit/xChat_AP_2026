#!/usr/bin/env bash
# ==============================================================================
# xChat Universal 1-Click Launcher & Auto-Setup
# Double-click in macOS Finder, or execute in Linux / Android (Termux)
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

# Check for existing local xChat binary
XCHAT_BIN=""
SCRIPT_DIR="$(cd "$(dirname "$0")" 2>/dev/null && pwd || echo "")"
if [ -n "$SCRIPT_DIR" ] && [ -x "${SCRIPT_DIR}/xchat" ]; then
    XCHAT_BIN="${SCRIPT_DIR}/xchat"
elif [ -n "$SCRIPT_DIR" ] && [ -x "${SCRIPT_DIR}/target/release/xchat" ]; then
    XCHAT_BIN="${SCRIPT_DIR}/target/release/xchat"
elif [ -n "$SCRIPT_DIR" ] && [ -x "${SCRIPT_DIR}/target/release/xchat-cli" ]; then
    XCHAT_BIN="${SCRIPT_DIR}/target/release/xchat-cli"
elif [ -x "${XCHAT_HOME}/bin/xchat" ]; then
    XCHAT_BIN="${XCHAT_HOME}/bin/xchat"
elif command -v xchat >/dev/null 2>&1; then
    XCHAT_BIN="$(command -v xchat)"
fi

# Auto-download or build binary if missing
if [ -z "$XCHAT_BIN" ]; then
    echo "[*] xChat binary not found locally. Starting automatic setup..."
    
    if [ "$OS_RAW" = "Darwin" ]; then
        echo "[*] Downloading pre-compiled Universal macOS release (Apple Silicon + Intel)..."
        TAR_URL="https://github.com/EdgeRunner-runit/xChat_AP_2026/releases/download/v0.1.0/xchat-v0.1.0-macos-universal.tar.gz"
        TMP_PKG="$(mktemp /tmp/xchat-pkg.XXXXXX.tar.gz)"
        curl -fSL "$TAR_URL" -o "$TMP_PKG"
        tar -xzf "$TMP_PKG" -C "${XCHAT_HOME}/bin"
        rm -f "$TMP_PKG"
        chmod +x "${XCHAT_HOME}/bin/xchat"
        XCHAT_BIN="${XCHAT_HOME}/bin/xchat"
        echo "[✓] Installation successful: ${XCHAT_BIN}"
        
    elif [ "$IS_ANDROID" = "true" ]; then
        echo "[*] Android Termux detected. Checking build toolchain..."
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
        echo "[✓] Android installation complete: ${XCHAT_BIN}"
        
    elif [ "$OS_RAW" = "Linux" ]; then
        echo "[*] Linux environment detected."
        if command -v cargo >/dev/null 2>&1; then
            echo "[*] Building xChat via cargo..."
            cargo install --git https://github.com/EdgeRunner-runit/xChat_AP_2026.git xchat-cli --root "${XCHAT_HOME}"
            XCHAT_BIN="${XCHAT_HOME}/bin/xchat"
        else
            echo "[!] Cargo not found. Please install Rust (https://rustup.rs) to build xChat."
            exit 1
        fi
        
    elif [[ "$OS_RAW" =~ MINGW|MSYS|CYGWIN ]]; then
        echo "[*] Windows Bash environment detected."
        if command -v cargo >/dev/null 2>&1; then
            cargo install --git https://github.com/EdgeRunner-runit/xChat_AP_2026.git xchat-cli --root "${XCHAT_HOME}"
            XCHAT_BIN="${XCHAT_HOME}/bin/xchat.exe"
        else
            echo "[!] Cargo not found. Please install Rust or use xchat-launcher.bat."
            exit 1
        fi
    fi
fi

if [ ! -x "$XCHAT_BIN" ]; then
    echo "[!] Error: Failed to find or install xChat binary."
    exit 1
fi

# Ensure cryptographic identity is generated
KEYSTORE="${XCHAT_HOME}/keystore.json"
if [ ! -f "$KEYSTORE" ]; then
    echo "[*] First run: Initializing cryptographic identity and prekeys..."
    "$XCHAT_BIN" keygen
else
    echo "[*] Cryptographic identity active (${KEYSTORE})"
fi

APP_URL="http://127.0.0.1:${PORT}"
echo ""
echo "============================================================="
echo "   xChat is running! Opening browser..."
echo "   URL: ${APP_URL}"
echo "============================================================="
echo ""

# Auto-open browser
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

# Launch App GUI server
exec "$XCHAT_BIN" app --port "$PORT"
