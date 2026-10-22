# xChat

A decentralized, peer-to-peer messaging system delivering secure, end-to-end encrypted communication across heterogeneous physical transports without reliance on central servers.

Users connect via cryptographic addresses instead of phone numbers or email addresses to guarantee anonymity. This serverless, multi-transport architecture makes the network highly resistant to censorship, outages, and surveillance.

## Architecture

The system decouples cryptography from network transports:

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

For detailed specifications, see the [Security & Cryptography Architecture](docs/SECURITY_ARCHITECTURE.md).

## Core Security & Cryptographic Principles

- **Transport Independence:** Cryptography sits strictly above the transport layer. Messages survive seamless handovers between Bluetooth, Local Wi-Fi, Wi-Fi Direct, and Internet transports.
- **Relay Blindness:** Intermediaries only handle opaque encrypted envelopes bound to ephemeral routing tokens.
- **End-to-End Encryption:**
  - **1:1 Sessions:** Double Ratchet + X3DH / PQXDH (post-quantum hybrid forward secrecy).
  - **Group Chats:** MLS (Messaging Layer Security, RFC 9420).
  - **Symmetric AEAD:** ChaCha20-Poly1305 (RFC 8439) with HKDF key derivation.
  - **Hop Security:** Noise Protocol Framework handshakes between direct physical neighbors.
- **Metadata Minimization:** Rotating discovery tokens, fixed bucket padding, encrypted routing envelopes, and store-and-forward outbox queues.
