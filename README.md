# xChat

A decentralized, peer-to-peer messaging system delivering secure, end-to-end encrypted communication across heterogeneous physical transports without reliance on central servers.

Users connect via cryptographic addresses (`xchat_<hex>`) instead of phone numbers or email addresses to guarantee anonymity. This serverless, multi-transport architecture makes the network highly resistant to censorship, outages, and surveillance.

---

## Conceptual Architecture

The fundamental architectural principle is:

> **Cryptography sits strictly above the transport layer.**

Physical transports (Bluetooth Low Energy, Local Wi-Fi, Wi-Fi Direct, Internet TCP/IP) carry opaque encrypted protocol envelopes. Neither relays nor eavesdroppers have plaintext access or knowledge of long-term user identities.

```text
┌───────────────────────────────────────────────┐
│                    UI                         │
├───────────────────────────────────────────────┤
│             Messaging Protocol                │
├───────────────────────────────────────────────┤
│       E2EE Session / Group Crypto             │
├───────────────────────────────────────────────┤
│      Identity + Key Management                │
├───────────────────────────────────────────────┤
│        Message Queue / Store Forward          │
├───────────────────────────────────────────────┤
│              Routing Layer                    │
├───────────────────────────────────────────────┤
│        Transport Abstraction Layer            │
├──────────┬──────────┬──────────┬───────────────┤
│ Internet │ Wi-Fi    │ Wi-FiDir │ Bluetooth     │
└──────────┴──────────┴──────────┴───────────────┘
```

For complete technical specifications, review the formal 50-section specification in [docs/SECURITY_ARCHITECTURE.md](docs/SECURITY_ARCHITECTURE.md).

---

## Workspace Structure

The project is structured as a modular Rust Cargo workspace:

```text
xChat_AP_2026/
├── crates/
│   ├── xchat-crypto/     # Cryptographic primitives, Ed25519/X25519 identity, X3DH,
│   │                     # Double Ratchet, ChaCha20-Poly1305 AEAD, HKDF, padding, zeroize
│   ├── xchat-core/       # Wire envelopes, store-and-forward outbox, replay filters,
│   │                     # encrypted keystore, blind relay nodes, session coordinator
│   ├── xchat-transport/  # Transport trait, async TCP framing, BLE mesh simulator,
│   │                     # and multi-transport failover manager
│   └── xchat-cli/        # CLI application, node daemon, and interactive mesh simulation
├── docs/
│   └── SECURITY_ARCHITECTURE.md  # Formal 50-section security architecture specification
├── task_plan.md          # Implementation plan and tracking
├── findings.md           # Architectural decisions and threat model analysis
└── progress.md           # Progress and test execution log
```

---

## Security Guarantees

| Area | Security Guarantee |
| :--- | :--- |
| **Confidentiality & Integrity** | ChaCha20-Poly1305 AEAD with bound Associated Authenticated Data (AAD). |
| **Forward Secrecy (FS)** | Continuous per-message Double Ratchet; old keys are zeroized immediately after decryption. |
| **Break-in Recovery (PCS)** | Ephemeral Diffie-Hellman ratcheting guarantees security recovery after key compromise. |
| **Offline Messaging** | X3DH initial key agreement via published prekey bundles (Identity + Signed Prekey + One-Time Prekeys). |
| **Transport Independence** | Changing physical transports (BLE -> TCP) preserves ongoing cryptographic ratchet state without reset. |
| **Relay Blindness** | Intermediary nodes inspect only ephemeral destination routing tokens without access to plaintexts or long-term identities. |
| **Anti-Replay Protection** | Sliding window and persistent message ID tracking reject retransmitted packets. |
| **Metadata Minimization** | Bucketed message padding (64, 128, 256, 512, 1024 bytes) and rotating ephemeral discovery tokens prevent passive traffic analysis. |

---

## Getting Started

### Prerequisites

- [Rust & Cargo](https://rustup.rs/) (edition 2021, Rust 1.80+ recommended)

### Build the Workspace

```bash
cargo build --workspace
```

### Run All Unit and Integration Tests

```bash
cargo test --workspace
```

---

## Using the CLI

### 1. Generate Cryptographic Identity

Generate a fresh Ed25519 signing key, X25519 DH key, signed prekey, and one-time prekeys:

```bash
cargo run -- keygen
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
Prekey Bundle (JSON): ...
```

### 2. Run the Multi-Transport Mesh Simulation

Execute an end-to-end multi-node simulation demonstrating:
- **Offline store-and-forward:** Sender encrypts with X3DH + Double Ratchet while recipient is offline.
- **Relay blindness:** Blind relay node buffers opaque ciphertext without knowing user identities.
- **Transport handover:** Recipient comes online on TCP LAN; relay flushes envelopes from BLE to TCP.
- **Bi-directional ratcheting:** Recipient decrypts and replies, advancing the Diffie-Hellman ratchet.
- **Active adversarial defenses:** Automated tests confirming replay attacks, ciphertext bit-flips, and routing header tampering are blocked.

```bash
cargo run -- simulate-mesh
```

### 3. Start a Blind Relay Node

Run an autonomous blind relay server buffering and routing opaque packets for offline peers:

```bash
cargo run -- daemon --port 7000 --relay
```

### 4. Start a Standard Peer Daemon

Run a peer listener ready to receive framed encrypted envelopes:

```bash
cargo run -- daemon --port 7001
```

---

## Code Quality & Verification

The codebase strictly adheres to Rust quality and safety standards:

```bash
# Verify code formatting
cargo fmt --check

# Strict clippy linting (zero warnings permitted)
cargo clippy --workspace -- -D warnings

# Execute entire test suite
cargo test --workspace
```
