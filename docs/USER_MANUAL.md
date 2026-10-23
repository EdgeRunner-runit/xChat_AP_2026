# xChat User Manual & Deployment Guide

**Version:** 1.0  
**Target Audience:** Users, field operators, and engineers deploying xChat across Android, iOS, macOS, Linux, and Windows devices.

---

## 1. Introduction

**xChat** is a decentralized, peer-to-peer private messenger designed to operate completely without internet access, cellular networks, or central servers.

- **No Central Servers:** Communication is direct between devices or relayed through friendly mesh nodes.
- **No Phone Numbers or Emails:** Your identity is an Ed25519/X25519 cryptographic keypair generated entirely on your device.
- **Transport Independent:** Operates over Bluetooth Low Energy (BLE), Local Wi-Fi, Wi-Fi Direct, or Internet TCP/IP.
- **Relay Blindness:** Intermediate relay devices store and forward messages without being able to read message contents or determine who is talking to whom.

---

## 2. Platform Installation & Quick Start

### macOS
1. Download `xchat-macos-arm64.tar.gz` (Apple Silicon M1/M2/M3/M4) or Intel version.
2. Extract and move to your path:
   ```bash
   tar -xzvf xchat-v0.1.0-macos-arm64.tar.gz
   sudo mv xchat /usr/local/bin/
   ```
3. Test installation:
   ```bash
   xchat --help
   ```

### Linux (Ubuntu / Debian / Fedora / Arch)
1. Download `xchat-linux-x86_64.tar.gz`.
2. Extract and install:
   ```bash
   tar -xzvf xchat-linux-x86_64.tar.gz
   sudo mv xchat /usr/local/bin/
   chmod +x /usr/local/bin/xchat
   ```

### Windows 10 / 11
1. Download `xchat-windows-x86_64.zip`.
2. Extract `xchat.exe` to a folder (e.g. `C:\Tools\xchat\`).
3. Open PowerShell or Command Prompt:
   ```powershell
   .\xchat.exe --help
   ```

### Android
**Method A — Termux (Standalone Off-Grid Terminal):**
1. Install [Termux](https://f-droid.org/en/packages/com.termux/) from F-Droid.
2. Inside Termux, install Rust or download the pre-compiled ARM64 binary:
   ```bash
   pkg update && pkg install git rust
   git clone https://github.com/EdgeRunner-runit/xChat_AP_2026.git
   cd xChat_AP_2026
   cargo build --release
   cp target/release/xchat-cli $PREFIX/bin/xchat
   ```
3. Run:
   ```bash
   xchat keygen
   ```

**Method B — Native App Integration:**
- The core cryptographic and routing engine builds into an Android NDK shared library (`libxchat_core.so`) via UniFFI, embeddable into any Kotlin / Android Studio app.

### iOS
- Run directly inside local terminal environments like **iSH** or **a-Shell** on iPadOS/iOS.
- For native Swift apps, xChat's Rust core compiles to an XCFramework (`xchat.xcframework`) linked into iOS apps.

---

## 3. Step-by-Step Usage

### Step 0: Launch the Ready-to-Use App GUI
Run:
```bash
xchat app
```
*(or `xchat gui`)*

This boots your local encrypted node and automatically opens a responsive, dark-mode graphical user interface in your default browser at `http://127.0.0.1:8080`.
- **Zero Internet Required:** Served directly by the local lightweight binary (zero external web requests).
- **Features in GUI:**
  - Active E2EE messaging pane with Double Ratchet status
  - One-click **🚨 Emergency SOS** distress beacon trigger
  - **🔥 Burner Room** RAM-only ephemeral chat
  - **📱 Air-Gap QR** camera/modal key exchange
  - Contact list & live peer reachability pings

---

### Step 1: Generate Your Cryptographic Identity
Run the key generation tool:
```bash
xchat keygen
```

Output:
```text
=======================================================
   xChat Cryptographic Identity Key Generation
=======================================================
Device Address:       xchat_b770179928483d97
Identity Fingerprint: b770179928483d97fff0871acb785264f93a7a0730cadd99d6d239777a7a9f52
Ed25519 Sign Pubkey:  b770179928483d97fff0871acb785264f93a7a0730cadd99d6d239777a7a9f52
X25519 DH Pubkey:     fb7e38b246bbbe345c94ea7fe3671d5bd2dc79d4a5d49af10be13f5003c87d1a
Signed Prekey:        5fea0c4c6c2c18b8d4bfef70597ca0602b370ae4f57232f6649808e7815cc71d
Active One-Time Keys: 10
```

- **Device Address (`xchat_<hex>`):** Share this address with your contacts (like an anonymous username).
- **Identity Fingerprint:** Compare this fingerprint (via QR code or verbal confirmation) to verify authenticity and prevent man-in-the-middle attacks.

---

### Step 2: Off-Grid Direct Communication (Without Internet)

When no internet or cellular network exists, xChat devices connect directly over:
1. **Bluetooth Low Energy (BLE):** Devices automatically pair and discover nearby peers using rotating discovery tokens.
2. **Local Wi-Fi Direct / Portable Hotspot:** One device creates an offline mobile hotspot (no mobile data required). Both devices connect to the local IP subnet.

Start the peer daemon:
```bash
xchat daemon --port 7001
```

---

### Step 3: Run an Autonomous Blind Relay Node

To extend network reach across an office, house, or neighborhood, any device (an old laptop, Raspberry Pi, or phone plugged into a charger) can run as a **Blind Relay**:

```bash
xchat daemon --port 7000 --relay
```

**What the Blind Relay does:**
1. Collects encrypted envelopes from passing devices.
2. Stores them safely in a store-and-forward queue on disk.
3. Forwards them to recipients as soon as they appear in range.
4. **Relay Blindness:** The relay operator **CANNOT read message text** or see user identities.

---

### Step 4: Test Everything Locally with the Multi-Node Simulator

You can test the entire offline mesh, transport handover, and attack defense on any computer:
```bash
xchat simulate-mesh
```

---

### Step 5: Broadcast an Emergency SOS Mesh Distress Beacon
In disaster or off-grid emergency scenarios, broadcast a high-priority distress beacon to all nearby devices and relays:
```bash
xchat sos --message "Medical supplies required at coordinate site"
```
- Transmitted with maximum mesh forwarding priority.
- Decodable by all community search-and-rescue nodes.

---

### Step 6: Join an Ephemeral RAM-Only Burner Room
For temporary discussions that must leave zero trace on physical flash storage:
```bash
xchat burner --room tactical-alpha --pass secretpass123
```
- Messages exist in RAM only and automatically auto-expire.

---

### Step 7: Manage Saved Contacts & Ping Reachability
Save friends' addresses and check if they are currently within direct radio range:
```bash
# Add contact
xchat contact add Alice '<PREKEY_BUNDLE_JSON>'

# List contacts
xchat contact list

# Ping peer over physical links
xchat ping Alice
```

---

## 4. Communication Distances & Expected Speeds

### A. Direct Distance (Point-to-Point Without Intermediaries)

How far can two people talk directly without needing any other users or relays?

| Transport Mode | Indoor Range | Outdoor / Line-of-Sight | Notes |
| :--- | :--- | :--- | :--- |
| **Standard Bluetooth (BLE)** | 10 – 30 meters (through walls) | 50 – 100 meters | Standard on every phone and laptop. Low battery usage. |
| **BLE Long Range (Coded PHY)** | 50 – 100 meters | 300 – 800 meters | Supported by Bluetooth 5.0+ hardware. Excellent outdoor range. |
| **Local Wi-Fi / Hotspot** | 30 – 70 meters | 100 – 200+ meters | Uses phone hotspot or offline router. Very fast data rate. |
| **Directional Wi-Fi Antennas** | N/A | 1 – 5 kilometers | Uses directional outdoor antennas pointed between rooftops. |
| **LoRa Radio Extension (Optional)** | 1 – 3 kilometers (dense urban) | 5 – 15+ kilometers | Low-power sub-GHz radio module for extreme distance. |

---

### B. Distances Requiring Other Users (Multi-Hop Mesh & Store-and-Forward)

When communicating across longer distances (e.g. across a neighborhood, university campus, or entire city), xChat relies on two mesh relay mechanisms:

```text
[Alice] ──(BLE 50m)──> [User B Relay] ──(BLE 50m)──> [User C Relay] ──(BLE 50m)──> [Bob]
```

1. **Active Real-Time Mesh Hopping:**
   - Messages hop instantly from device to device across the area.
   - 3 to 10 intermediary devices can bridge distances of **500 meters to 2+ kilometers** across a neighborhood.
2. **Store-and-Forward / Sneakernet:**
   - If no continuous physical path of devices exists, a relay user carries the encrypted envelope in their phone's local queue as they walk or drive.
   - When they pass within range of Bob, the envelope is automatically and silently delivered.
   - **Range:** Virtually unlimited (kilometers to across town), with delivery time dependent on physical movement.

---

### C. Expected Speeds & Latency

| Physical Link | Data Speed (Throughput) | Delivery Latency | Best Used For |
| :--- | :--- | :--- | :--- |
| **Bluetooth Low Energy (BLE)** | 100 kbps – 1 Mbps | 20 – 100 ms (instant) | Text messages, cryptographic handshakes, emojis, location pins. Small photos take 1–3s. |
| **Local Wi-Fi / Wi-Fi Direct** | 20 Mbps – 100+ Mbps | 1 – 5 ms (instant) | High-definition photos, voice messages, video clips, large documents. |
| **LoRa Radio (Sub-GHz)** | 0.3 kbps – 5 kbps | 500 ms – 2 seconds | Short text messages and emergency coordinates only. |

---

## 5. Security & Operational Privacy Checklist (OPSEC)

To maintain maximum privacy in high-threat environments:

1. **Verify Identity Fingerprints in Person:** When adding a new trusted contact, compare their 64-character hex fingerprint or QR code to eliminate man-in-the-middle risks.
2. **Turn Off Unneeded Broadcasts:** Use xChat's rotating discovery tokens so your device cannot be tracked by third-party Bluetooth scanners.
3. **Set Message Expiration (TTL):** Standard envelopes expire after 24 hours (86,400s) and are automatically pruned from relays and local outboxes.
4. **Crypto-Shredding:** When deleting local messages, xChat deletes the ratcheted message key, rendering stored ciphertext unrecoverable even if the raw storage is examined.
