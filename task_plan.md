# Task Plan: xChat Multi-Transport Private Messenger Implementation

## Goal
Build, test, verify, and document a production-grade, full-fledged implementation of `xChat` in Rust based on the Security & Cryptography Architecture (`docs/SECURITY_ARCHITECTURE.md`).

## Architecture & Work Breakdown

### Phase 1: Workspace Scaffolding & Dependencies
- [x] Initialize Cargo workspace in `xChat_AP_2026`
- [x] Define workspace crates:
  - `crates/xchat-crypto`: Cryptographic primitives, identity keys, Double Ratchet, AEAD, KDF, padding, zeroize
  - `crates/xchat-core`: Wire envelopes, session state machine, store-and-forward outbox, replay cache
  - `crates/xchat-transport`: Transport abstraction trait, TCP/LAN transport, Simulated Bluetooth/Mesh transport, TransportManager
  - `crates/xchat-cli`: Interactive CLI, node daemon, and multi-peer runner
- [x] Configure dependencies (`tokio`, `chacha20poly1305`, `x25519-dalek`, `ed25519-dalek`, `hkdf`, `sha2`, `serde`, `bincode`, `zeroize`, etc.)

### Phase 2: Cryptographic Engine (`xchat-crypto`)
- [x] Device & Account Identity management (Ed25519 identity, X25519 Diffie-Hellman)
- [x] Asynchronous session establishment (X3DH prekey bundles: SPK, OPK)
- [x] Double Ratchet state machine:
  - Symmetric ratchet (Send chain, Receive chain)
  - DH ratchet (continuous forward secrecy & post-compromise security)
  - Skipped message keys table with bounded cache & TTL
- [x] Authenticated Encryption (ChaCha20-Poly1305) with bound AAD
- [x] Bucketed message padding & metadata minimization
- [x] Rotating ephemeral discovery token generation
- [x] Memory safety & `zeroize` on secret drops
- [x] Comprehensive unit tests for all crypto primitives & test vectors (9/9 passed)

### Phase 3: Core Wire Protocol, Store-and-Forward & State Machine (`xchat-core`)
- [x] Binary wire protocol: `EncryptedEnvelope`, `RoutingHeader`, `RouteToken`, `MessageId`
- [x] Anti-replay filter with sliding window and persistent cache
- [x] Store-and-forward persistent outbox queue with TTL enforcement and disk persistence
- [x] Relay-blind forwarding logic (relays route opaque envelopes without plaintext access)
- [x] Crypto state vs transport state decoupling
- [x] Comprehensive unit tests (9/9 passed)

### Phase 4: Transport Abstraction Layer (`xchat-transport`)
- [x] Universal `Transport` trait (`send`, `broadcast_envelope`, `receive_envelope`, `is_available`)
- [x] Local TCP / LAN Transport (framed asynchronous streaming)
- [x] Simulated BLE / Wireless Mesh Transport (simulating ad-hoc wireless connectivity & broadcast)
- [x] `TransportManager`: Multi-transport routing, automatic handover, and fallback
- [x] Comprehensive unit tests (3/3 passed)

### Phase 5: Client Application & Daemon (`xchat-cli`)
- [x] Node identity initialization & persistent keystore export (`keygen`)
- [x] Node daemon with blind relay mode (`daemon --relay`)
- [x] Multi-node test orchestration command (`simulate-mesh`)

### Phase 6: Comprehensive Testing, Hardening & Verification
- [x] Integration tests:
  - Multi-hop blind relay delivery (Alice -> Relay 1 -> Relay 2 -> Bob)
  - Transport handover during active session (Bluetooth drops -> switches to TCP LAN)
  - Skipped message keys & out-of-order packet delivery (Packets 4, 2, 1, 3)
- [x] Adversarial tests:
  - Replay attack defense verification
  - Ciphertext tampering detection via AEAD authentication
  - Routing header tampering detection via bound AAD
- [x] Code formatting (`cargo fmt --check`) and linting (`cargo clippy --workspace -- -D warnings`) with 0 errors/warnings

### Phase 7: Final Documentation & Git Commit
- [x] Complete README and architecture guides
- [x] Clean git commit

### Phase 8: Advanced Features & Lightweight App GUI
- [x] Emergency SOS Mesh Beacon (`xchat-core/src/sos.rs`)
- [x] Ephemeral Burner Mesh Rooms (`xchat-core/src/burner.rs`)
- [x] Duress PIN & Decoy Vault (`xchat-core/src/duress.rs`)
- [x] Chunked File/Voice Carrier (`xchat-core/src/chunking.rs`)
- [x] Cover Traffic & Noise Injection (`xchat-core/src/cover_traffic.rs`)
- [x] Embedded Lightweight App GUI (`crates/xchat-cli/src/gui.rs`) with embedded HTML/CSS/JS frontend
- [x] CLI integration: `xchat app`, `xchat gui`, `xchat sos`, `xchat burner`
- [x] Verification, testing (all 30 tests passed), and release build repackaging
