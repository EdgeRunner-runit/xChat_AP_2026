# Multi-Transport Private Messenger --- Security & Cryptography Architecture

**Status:** Draft v1.0  
**Security goal:** Strong end-to-end confidentiality, authentication, forward secrecy, post-compromise recovery, and metadata minimization across unreliable and heterogeneous transports.

------------------------------------------------------------------------

# 1. Executive Summary

The most important architectural decision is:

> **Cryptography must sit above the transport layer.**

Bluetooth, Wi-Fi, Internet, Wi-Fi Direct, RPC, relay nodes, and store-and-forward infrastructure should transport opaque encrypted protocol messages.

The relay/network layer should not need plaintext access.

Recommended conceptual stack:

``` text
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

The cryptographic layer should not care which bottom transport carried a packet.

------------------------------------------------------------------------

# 2. Threat Model

Assume attackers may:

- passively sniff wireless traffic
- actively inject packets
- replay packets
- impersonate network peers
- operate malicious relay nodes
- compromise some relay nodes
- compromise a user's device temporarily
- obtain old encrypted traffic
- observe packet timing and size
- correlate discovery events
- control the Internet connection
- selectively drop or delay packets
- attempt denial of service
- obtain metadata from infrastructure

Do **not** assume that encryption alone hides:

- who is communicating
- when communication occurs
- packet size
- number of messages
- approximate location
- relay participation

Metadata protection is a separate engineering problem.

------------------------------------------------------------------------

# 3. Security Goals

## 3.1 Confidentiality
Only authorized endpoints should decrypt message contents.

## 3.2 Authentication
A recipient should be able to determine whether messages originate from the expected cryptographic identity.

## 3.3 Forward secrecy
Compromise of current keys should not automatically reveal old messages.

## 3.4 Post-compromise security
After a temporary compromise, future communication should be able to recover security after fresh key material is exchanged.

## 3.5 Replay resistance
Previously captured packets should not be accepted as new messages.

## 3.6 Transport independence
Security must survive transport changes.

## 3.7 Offline operation
A sender should be able to initiate secure communication while the recipient is offline.

## 3.8 Relay blindness
Relays should normally transport ciphertext without possessing message decryption keys.

------------------------------------------------------------------------

# 4. Recommended Cryptographic Architecture

For a mature implementation, the preferred direction is:

``` text
Identity
   │
   ▼
Asynchronous authenticated key establishment
   │
   ▼
Continuous ratcheting
   │
   ▼
Per-message keys
   │
   ▼
AEAD encryption
   │
   ▼
Encrypted message envelope
   │
   ▼
Transport-independent routing
```

A practical protocol family is:

``` text
Initial session:
    PQXDH / modern asynchronous authenticated KEX

Ongoing 1:1 messaging:
    Double Ratchet

Group messaging:
    MLS (RFC 9420)

AEAD:
    ChaCha20-Poly1305 or AES-256-GCM

KDF:
    HKDF-SHA-256 / HKDF-SHA-512

Classical DH:
    X25519 (RFC 7748)

Signatures / identity authentication:
    Ed25519 (RFC 8032) where appropriate

Post-quantum:
    ML-KEM-based hybrid design
```

Signal's published specifications document X3DH/PQXDH, Double Ratchet, Sesame, and ML-KEM-based ratcheting concepts.

MLS (RFC 9420) is standardized for asynchronous group key establishment and provides forward secrecy and post-compromise security for groups ranging from two to thousands.

------------------------------------------------------------------------

# 5. Key Hierarchy

Do not use one master encryption key for everything.

Use a hierarchy similar to:

``` text
                 Identity Key
                      │
             ┌────────┴────────┐
             │                 │
       Authentication      Prekey system
                               │
                               ▼
                         Session secret
                               │
                               ▼
                           Root key
                               │
                    ┌──────────┴──────────┐
                    ▼                     ▼
               Send chain             Receive chain
                    │                     │
             ┌──────┴──────┐       ┌────┴───────┐
             ▼             ▼       ▼            ▼
          MsgKey1       MsgKey2  MsgKey1      MsgKey2
             │             │       │            │
             ▼             ▼       ▼            ▼
           AEAD          AEAD    AEAD         AEAD
```

Old message keys should be deleted as soon as practical.

------------------------------------------------------------------------

# 6. Identity Keys

Each installation/device should have a cryptographic identity.

Possible primitives:
- Ed25519 for signatures (RFC 8032)
- X25519 for Diffie-Hellman (RFC 7748)

The identity should not be casually regenerated.

Device identity and account identity should also be conceptually separated.

------------------------------------------------------------------------

# 7. Initial Key Agreement

## Option A --- X3DH

X3DH was designed for asynchronous messaging where the recipient may be offline. It uses identity keys, signed prekeys, and optional one-time prekeys to establish an initial shared secret.

### Advantages
- mature design
- designed for asynchronous messaging
- supports offline recipients
- provides forward secrecy properties
- widely studied

### Disadvantages
- classical public-key assumptions
- prekey infrastructure is required for the traditional deployment
- more complicated than a simple Noise handshake
- not inherently a multi-hop routing protocol

### Use
Good compatibility/reference design for an asynchronous messenger.

------------------------------------------------------------------------

# 8. PQXDH

PQXDH extends the asynchronous session-establishment model with post-quantum key material.

Signal's published documentation describes PQXDH as an asynchronous authenticated key agreement protocol providing post-quantum forward secrecy properties, while noting that the documented revision still relies on classical discrete-log assumptions for mutual authentication.

### Why use it
The system may have long-lived encrypted traffic stored by adversaries ("harvest now, decrypt later"):

``` text
capture traffic today
       ↓
store ciphertext
       ↓
obtain future cryptanalytic capability
       ↓
attempt decryption later
```

A hybrid post-quantum design reduces this risk.

### Why not blindly implement it
Post-quantum protocols increase:
- message size
- implementation complexity
- CPU/memory cost
- testing burden
- interoperability complexity

For an MVP, use an established implementation rather than designing a custom PQ handshake.

------------------------------------------------------------------------

# 9. Double Ratchet

The Double Ratchet should be the main conceptual choice for 1:1 messaging.

It derives fresh message keys continuously and combines symmetric-key and Diffie-Hellman ratchets. The published specification explicitly describes forward security and break-in recovery properties.

### Advantages
- per-message keys
- forward secrecy
- post-compromise recovery properties
- handles out-of-order messages
- established secure messaging architecture

### Disadvantages
- stateful
- complicated recovery logic
- difficult multi-device session management
- difficult to implement correctly from scratch

### Recommendation
**Use an audited implementation or a well-established protocol library.**  
Do not write your own Double Ratchet implementation from scratch for production.

------------------------------------------------------------------------

# 10. MLS for Groups

For group conversations, use **Messaging Layer Security (MLS, RFC 9420)** rather than inventing a group ratchet.

MLS is an IETF standards-track protocol designed for asynchronous group key establishment and provides forward secrecy and post-compromise security.

### Advantages
- designed for groups
- asynchronous
- efficient membership updates
- forward secrecy
- post-compromise security
- standardized

### Disadvantages
- more complicated than 1:1 encryption
- group state management is non-trivial
- membership changes create cryptographic operations
- implementation ecosystem is less mature than basic TLS libraries

### Recommendation
Recommended for serious group-chat architecture.

------------------------------------------------------------------------

# 11. Noise Protocol Framework

Noise is useful for **transport/session handshakes**, especially between peers and network nodes.

Noise is a framework for authenticated Diffie-Hellman handshakes and encrypted transport sessions. It supports different authentication patterns and forward secrecy depending on the selected pattern.

Use cases:
``` text
Phone ↔ Phone
Phone ↔ Relay
Relay ↔ Relay
Phone ↔ Local Gateway
```

### Why use it
- compact
- modular
- designed for protocol handshakes
- useful for peer authentication
- good fit for constrained links

### Why not use Noise as the entire messaging protocol?
Because a transport/session handshake is not the same thing as a persistent asynchronous messaging protocol.

Noise solves:
``` text
"How do these two peers establish a secure channel?"
```

A messaging ratchet solves:
``` text
"How do these users keep securely exchanging messages over time,
including when messages are delayed, reordered, or devices reconnect?"
```

Use them at different layers.

------------------------------------------------------------------------

# 12. AEAD Choice

Every encrypted message should use an authenticated encryption mode.

Recommended candidates:

## ChaCha20-Poly1305 (RFC 8439)
Provides confidentiality and authentication as an AEAD construction.

### Advantages
- fast on CPUs without AES acceleration
- efficient on mobile/constrained systems
- straightforward implementation
- widely deployed

### Disadvantages
- hardware-accelerated AES may outperform it on some platforms

### Recommendation
**Excellent default for a cross-platform messenger.**

------------------------------------------------------------------------

# 13. AES-256-GCM

### Advantages
- extremely widely deployed
- hardware acceleration on many devices
- mature cryptographic engineering ecosystem

### Disadvantages
- implementation mistakes around nonces can be catastrophic
- performance depends more strongly on hardware

### Recommendation
Support it where platform libraries strongly favor it, but standardize one primary AEAD for the protocol rather than allowing arbitrary algorithm negotiation.

------------------------------------------------------------------------

# 14. Why Not AES-CBC + HMAC?

Do not design a new protocol around:
``` text
AES-CBC + HMAC
```

Modern AEAD constructions are easier to compose correctly. Legacy encrypt-then-MAC constructions can be secure when implemented correctly, but they increase protocol complexity and provide little benefit for this application.

------------------------------------------------------------------------

# 15. HKDF

Use HKDF (RFC 5869) as the key derivation primitive where appropriate.

Conceptually:
``` text
Root Secret
     │
     ├── HKDF ──> Send Chain
     │
     └── HKDF ──> Receive Chain

Send Chain
     │
     ├── HKDF ──> Message Key 1
     ├── HKDF ──> Message Key 2
     ├── HKDF ──> Message Key 3
     └── ...
```

Never directly derive dozens of unrelated keys by ad-hoc hashing.

Use explicit domain separation:
``` text
app-v1/message-key
app-v1/route-key
app-v1/header-key
app-v1/attachment-key
```

------------------------------------------------------------------------

# 16. HPKE

HPKE is standardized in RFC 9180 and combines a KEM, KDF, and AEAD into a hybrid public-key encryption construction.

### Good use cases
- encrypting a small object to a recipient public key
- encrypted capability exchange
- bootstrapping some application-specific mechanisms
- infrastructure/control-plane encryption

### Why not use it as the entire chat protocol?
HPKE is primarily a public-key encryption construction. It does not replace:
- continuous message ratcheting
- asynchronous session management
- group membership management
- message ordering/replay state

Therefore:
``` text
HPKE ≠ Double Ratchet ≠ MLS
```
They solve different problems.

------------------------------------------------------------------------

# 17. Transport Encryption

There should be two distinct security layers:

## Layer 1 --- End-to-end message encryption
``` text
Alice
  │
  │ encrypted for Bob
  ▼
Relay A
  │
  ▼
Relay B
  │
  ▼
Bob
```
Relays cannot decrypt the message.

## Layer 2 --- Hop/session encryption
``` text
Alice ↔ Relay A
Relay A ↔ Relay B
Relay B ↔ Bob
```
Each link can additionally use a secure channel (e.g. Noise protocol). This prevents local attackers from easily inspecting routing/session traffic on individual links.

------------------------------------------------------------------------

# 18. Why Both Layers Matter

Suppose:
``` text
Alice
 ↓ Bluetooth
Peer A
 ↓ Wi-Fi
Peer B
 ↓ Internet
Bob
```

- A malicious Peer A should not be able to decrypt the message.
- A malicious Wi-Fi observer should not be able to read the message.
- A compromised Peer B should not be able to read messages merely because it is a relay.

Therefore:
``` text
E2EE protects content
+
Hop encryption protects individual links
+
Metadata minimization protects communication patterns
```

------------------------------------------------------------------------

# 19. Message Envelope

The wire format must strictly separate routing from content.

Conceptual example:
``` text
EncryptedMessage {
    protocol_version
    message_id
    destination_token
    route_metadata
    expiry
    ciphertext
    authentication_tag
}
```

The core principle:
``` text
Routing metadata ≠ plaintext message metadata
```

Do not put `sender_username`, `recipient_username`, `message_text`, `phone_number`, or `email` into relay-visible packets.

------------------------------------------------------------------------

# 20. Message IDs

Message IDs should be cryptographically random (128-bit or 256-bit) or derived from secure protocol state.

Avoid `message_id = timestamp + username`. Prefer cryptographically secure random identifiers with sufficient entropy. Message IDs must also be bound into authenticated associated data (AAD) where appropriate.

------------------------------------------------------------------------

# 21. Replay Protection

Every receiver should maintain replay state:
``` text
Message ID + Session epoch + Ratchet counter + Authenticated context
```

If an already-processed packet arrives again:
``` text
REPLAY → reject
```
Do not rely solely on timestamps because remote and offline devices may have drifted clocks.

------------------------------------------------------------------------

# 22. Store-and-Forward Security

Packets may sit on relay devices or queues for hours or days.

Therefore:
``` text
plaintext ──> encrypt ──> encrypted blob ──> disk
```
Never store plaintext on intermediary nodes with the intent to "encrypt later". Intermediaries store ciphertext only.

------------------------------------------------------------------------

# 23. TTL (Time-To-Live)

Every queued packet should have an authenticated expiry:
``` text
created_at
expires_at
```
The timestamp is not a trust anchor for cryptographic authenticity; receivers enforce TTL policies independently to clean up state and prevent stale replays.

------------------------------------------------------------------------

# 24. Metadata Privacy

Strong E2EE does **not** automatically hide metadata.

Potentially exposed information:
- Who is nearby
- When a device is transmitting
- Packet size
- Packet frequency
- Number of hops
- Approximate physical location
- Connection duration

Mitigations:
- Rotating discovery identifiers
- Randomized transmission jitter/timing where practical
- Packet padding
- Batching
- Encrypted routing tokens
- Minimal relay-visible metadata
- Optional cover traffic for high-security modes

------------------------------------------------------------------------

# 25. Padding

Messages of different lengths leak information (e.g. "yes" vs. a 12 MB document).

Apply bucketed padding before encryption:
``` text
1–64 bytes      → 64
65–256 bytes    → 256
257–1024 bytes  → 1024
...
```
For attachments, chunk into standardized sizes (e.g. 64 KB blocks).

------------------------------------------------------------------------

# 26. Discovery Privacy

Bluetooth/Wi-Fi discovery is one of the largest metadata risks.

Never continuously advertise:
- Human-readable device names ("Kushal's Phone")
- Public identity keys
- Permanent device UUIDs / MACs

Instead, derive rotating ephemeral discovery tokens:
``` text
Identity ──> KDF ──> Ephemeral discovery token ──(rotate)──> New token
```

------------------------------------------------------------------------

# 27. Relay Privacy

A relay ideally knows:
- Packet exists
- Next-hop routing token
- Packet expiry / TTL
- Approximate size

Relays must **not** know:
- Message plaintext
- Long-term sender identity
- Long-term recipient identity
- Conversation history or graphs

------------------------------------------------------------------------

# 28. Onion Routing --- Optional Advanced Layer

For high-threat metadata protection, multi-hop onion routing can be layered:
``` text
Alice
 ↓ Layer 1 encrypted for Relay A
Relay A
 ↓ Layer 2 encrypted for Relay B
Relay B
 ↓ Layer 3 encrypted for Relay C
Relay C
 ↓
Bob
```
Each relay peels only its own layer. Reserved for later iterations (V2) due to latency and implementation overhead.

------------------------------------------------------------------------

# 29. Why Not Just Use TLS?

TLS provides hop security (`Alice ↔ Server` and `Server ↔ Bob`), but the server can access application plaintext.

In this architecture:
``` text
Alice ─────────────── E2EE ─────────────── Bob
       ↕ hop security (TLS / Noise)
       relays
```
TLS or Noise is used under transports, but end-to-end encryption remains independent and sovereign.

------------------------------------------------------------------------

# 30. Why Not PGP?

PGP has awkward asynchronous ratcheting, cumbersome key management, weak forward secrecy in common deployments, and poor UX for real-time mobile/mesh messaging.

------------------------------------------------------------------------

# 31. Why Not a Single Static AES Key?

If a static key leaks, all historical and future messages are compromised. Continuous ratcheting produces ephemeral keys destroyed immediately after use.

------------------------------------------------------------------------

# 32. Why Not Invent a Custom Cipher?

Do not create custom ciphers, curves, MACs, or KDFs. The innovation belongs in **multi-transport routing, discovery, store-and-forward, relay architecture, metadata minimization, and user experience**.

------------------------------------------------------------------------

# 33. Proposed Protocol Stack

## MVP
``` text
Identity
  └── Ed25519 / X25519

Session establishment
  └── Established asynchronous secure-messaging protocol (X3DH / Signal Protocol)

1:1 messaging
  └── Double Ratchet

Group messaging
  └── MLS (RFC 9420)

KDF
  └── HKDF-SHA-256 (RFC 5869)

AEAD
  └── ChaCha20-Poly1305 (RFC 8439)

Peer transport
  └── Noise Protocol Framework

Internet transport
  └── TLS 1.3 / WebSocket / WebRTC where applicable

Storage
  └── Encrypted local database (SQLCipher / platform-encrypted key-value store)
```

------------------------------------------------------------------------

# 34. Advanced / V2

``` text
Initial KEX
  └── PQXDH / hybrid PQ design (ML-KEM)

1:1
  └── Double Ratchet + PQ ratchet (Triple Ratchet)

Groups
  └── MLS

Routing
  ├── multi-hop mesh routing
  ├── opportunistic store-and-forward
  ├── onion-style packet format (Sphinx / HORNET inspired)
  └── route privacy

Metadata
  ├── bucketed padding
  ├── batching
  ├── timing obfuscation / jitter
  └── rotating ephemeral discovery identifiers
```

------------------------------------------------------------------------

# 35. Protocol Comparison

| Protocol | Primary Purpose | Use? | Why | Why Not as Everything? |
| :--- | :--- | :--- | :--- | :--- |
| **X3DH** | Async session establishment | **Yes** (reference) | Offline recipient support | Classical assumptions |
| **PQXDH** | PQ async session establishment | **Yes** (mature stage) | Hybrid/PQ protection | Larger footprint & complexity |
| **Double Ratchet** | 1:1 messaging | **Yes** | Forward secrecy & post-compromise recovery | Stateful, 1:1 only |
| **MLS** | Group messaging | **Yes** | Standardized asynchronous group security | Complex group state logic |
| **Noise** | Peer / hop handshakes | **Yes** | Compact, modular channel security | Transport level, not asynchronous messenger |
| **HPKE** | Public-key encryption | Selectively | Clean KEM/KDF/AEAD composition | Not a continuous ratchet |
| **TLS** | Transport security | **Yes** | Industry standard link security | Point-to-point, not E2EE |
| **PGP** | General encryption | **No** | Legacy design | Poor messaging UX & ratcheting |
| **Static AES** | Symmetric encryption | **No** | Simple | Catastrophic compromise scope |
| **Custom crypto** | Anything | **No** | Zero benefit | Extremely high security risk |

------------------------------------------------------------------------

# 36. Key Storage

Store private keys in hardware-backed platform security modules:
- **Android:** Android Keystore (StrongBox / TEE)
- **iOS:** Keychain / Secure Enclave
- **Desktop:** OS Keyring / SecretService / DPAPI

Minimize persisted unencrypted key material in application flash storage.

------------------------------------------------------------------------

# 37. Device Compromise

Full endpoint compromise (malware with root/screen/keystroke logging access) cannot be solved cryptographically at rest. Differentiate threat boundaries explicitly:
1. Network attacker (Passive/Active)
2. Relay attacker (Untrusted / Compromised Node)
3. Server compromise
4. Endpoint compromise (Out-of-scope for cryptographic guarantees)

------------------------------------------------------------------------

# 38. Secure Deletion

On modern NAND flash storage, overwriting a file does not guarantee erasure due to wear leveling.
- Use crypto-shredding: destroy the encryption key to render stored ciphertext unrecoverable.
- Enforce short-lived in-memory plaintext buffers with explicit memory zeroization (`zeroize`).
- Delete ephemeral message keys immediately after decrypting or ratcheting.

------------------------------------------------------------------------

# 39. Cryptographic Versioning

Include an explicit protocol version byte and suite identifier in message headers. Never negotiate cryptographic parameters dynamically with untrusted parties.

------------------------------------------------------------------------

# 40. Algorithm Agility

Prevent downgrade attacks by specifying immutable cipher suites rather than open negotiation:
- **Suite 1:** X25519 + ChaCha20-Poly1305 + HKDF-SHA-256 + Ed25519
- **Suite 2 (Hybrid PQ):** X25519 + ML-KEM-768 + ChaCha20-Poly1305 + HKDF-SHA-512 + Ed25519

------------------------------------------------------------------------

# 41. Randomness

All cryptographic entropy must be sourced exclusively from OS-level CSPRNGs (e.g., `getrandom(2)`, `SecRandomCopyBytes`, `/dev/urandom`). Never use pseudo-random generators (`random.random()`, `Math.random()`) for any security-sensitive value.

------------------------------------------------------------------------

# 42. Nonces

AEAD nonces must never repeat under the same key. Derive unique 96-bit nonces deterministically from ratchet counter state or protocol HKDF expansion.

------------------------------------------------------------------------

# 43. Authentication of Routing Metadata

All routing headers visible to the outer layer must be bound into the AEAD Associated Authenticated Data (AAD) to detect unauthorized tampering or header swapping in transit.

------------------------------------------------------------------------

# 44. Cryptographic State Machine

``` text
NEW
 │
 ▼
KEY_ESTABLISHED
 │
 ▼
ACTIVE
 │
 ├──── send ────> MESSAGE_KEY_ADVANCED
 │
 ├──── receive ─> MESSAGE_KEY_ADVANCED
 │
 ├──── key change ─> RATCHET
 │
 └──── error ─────> RECOVERY ──> ACTIVE
```

Transport state (`NO_ROUTE`, `CONNECTING`, `CONNECTED`, `DISCONNECTED`) remains completely decoupled from crypto session state.

------------------------------------------------------------------------

# 45. Critical Architectural Rule

- A broken Bluetooth connection must **not** destroy the crypto session.
- Loss of Internet must **not** make conversations unavailable.
- Messages enter the encrypted outbox queue and traverse any available physical transport seamlessly.

------------------------------------------------------------------------

# 46. Security Testing Strategy

- **Unit tests:** Key derivation vectors, AEAD test vectors (RFC 8439), ratchet state transitions, replay rejection.
- **Property tests:** Out-of-order delivery, packet duplication, dropped packets, connection handover.
- **Adversarial tests:** Corrupted ciphertext, manipulated headers, replay attacks, key replay, downgrade attempts.
- **Fuzzing:** Protocol buffers / envelope decoders, routing parser, handshake state machine.

------------------------------------------------------------------------

# 47. Security Audit Strategy

1. Use audited, established protocol libraries (e.g., `libsignal`, `openmls`, `snow` for Noise).
2. Avoid custom primitives.
3. Keep the cryptographic core small, auditable, and decoupled.
4. Separate cryptographic state from networking.
5. Provide comprehensive test vectors.
6. Undergo third-party security audit and penetration testing before production release.

------------------------------------------------------------------------

# 48. Target System Architecture Diagram

``` text
                         APPLICATION (UI)
                              │
                 ┌────────────▼────────────┐
                 │     Messaging Layer     │
                 └────────────┬────────────┘
                              │
             ┌────────────────▼────────────────┐
             │       End-to-End Crypto         │
             │                                 │
             │  1:1 → Double Ratchet           │
             │  Group → MLS (RFC 9420)         │
             │  Async KEX → PQXDH / X3DH       │
             │  AEAD → ChaCha20-Poly1305       │
             │  KDF → HKDF-SHA-256             │
             └────────────────┬────────────────┘
                              │
                 ┌────────────▼────────────┐
                 │ Encrypted Message Queue │
                 │    (Store & Forward)    │
                 └────────────┬────────────┘
                              │
                 ┌────────────▼────────────┐
                 │     Route Manager       │
                 │   (Relay Blindness)     │
                 └────────────┬────────────┘
                              │
              ┌───────────────┼────────────────┐
              │               │                │
              ▼               ▼                ▼
          Internet           Wi-Fi        Bluetooth/BLE
          (TLS/TCP)       (mDNS/LAN)      (Mesh/GATT)
              │               │                │
              └───────────────┼────────────────┘
                              │
                       Peer / Relay Mesh
```

------------------------------------------------------------------------

# 49. Core Security Guarantees

| Area | Guarantee |
| :--- | :--- |
| **Content** | Relays and network observers cannot read message plaintexts. |
| **Session** | Ephemeral per-message keys: compromise of one key does not reveal past or future messages. |
| **Transport** | Messages survive transport handovers (e.g. Wi-Fi to Bluetooth to Cellular). |
| **Availability** | Offline / serverless operation: peer-to-peer and store-and-forward mesh delivery. |
| **Privacy** | Ephemeral rotating discovery identifiers prevent passive physical tracking. |
| **Architecture** | Adding or removing a transport layer requires zero modifications to the cryptographic engine. |

------------------------------------------------------------------------

# 50. Guiding Principle

The system is built as:

``` text
                    ONE
             secure messaging
                 protocol
                    │
             ┌──────┴──────┐
             │             │
          routing       cryptography
             │             │
       ┌─────┼─────┐       │
       │     │     │       │
      WiFi  BT   Internet  │
       │     │     │       │
       └─────┴─────┴───────┘
```

**Transport is replaceable. Cryptographic identity and messaging semantics are sovereign.**
