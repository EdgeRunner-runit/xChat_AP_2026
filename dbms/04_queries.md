# SQL Query Design Report: xChat Storage Engine

**Project:** xChat — Decentralized Multi-Transport Private Messenger  
**Module:** DBMS Query Design, Execution Plans & Transaction Management  
**Author:** Aryan Singh Damara  
**Course:** Advanced Programming / Database Management Systems  
**Evaluation Target:** CP3 (`dbms/04_queries.sql` & `dbms/04_queries.md`)  
**Prerequisites:** CP1 (`01_normalization.md`) · CP2 (`02_er_diagram.md` + `03_schema.sql`)  
**Date:** November 2026  

---

## Table of Contents

1. [Executive Summary & Query Architecture Scope](#1-executive-summary--query-architecture-scope)
2. [Dataset Description & Seed Schema](#2-dataset-description--seed-schema)
3. [Section A — DML Queries: Core Retrieval & Mutation](#3-section-a--dml-queries-core-retrieval--mutation)
   - 3.1 Query 1: Active Conversation Feed
   - 3.2 Query 2: Aggregate Unread Badge Counter
   - 3.3 Query 3: Message Thread History (Cursor Pagination)
   - 3.4 Query 4: Double Ratchet State Machine Lookup
   - 3.5 Query 5: Multi-Transport Outbox Dispatcher
   - 3.6 Query 6: X3DH Prekey Bundle Fetch
   - 3.7 Query 7: Peer Contact Discovery & Verification
   - 3.8 Query 8: Relay Cache Routing & TTL Garbage Collection
   - 3.9 Query 9: Delivery Status Update (Read Receipt Processing)
   - 3.10 Query 10: Outbox Lifecycle Management
4. [Section B — Aggregate & Analytics Queries](#4-section-b--aggregate--analytics-queries)
   - 4.1 Query 11: Per-User Cryptographic Identity Summary
   - 4.2 Query 12: Message Volume & Delivery Latency Statistics
   - 4.3 Query 13: Double Ratchet Health Report
   - 4.4 Query 14: Transport Carrier Reliability Audit
5. [Section C — Multi-Table Join Queries](#5-section-c--multi-table-join-queries)
   - 5.1 Query 15: Full Session Diagnostic Join
   - 5.2 Query 16: Prekey Exhaustion Alert
6. [Section D — Transaction Management](#6-section-d--transaction-management)
   - 6.1 T1: Atomic Message Send with Outbox Enqueue
   - 6.2 T2: Atomic Double Ratchet State Advance
   - 6.3 T3: Atomic Contact Verification Update
   - 6.4 T4: Atomic Outbox Acknowledgement & Cleanup
   - 6.5 T5: Safe Prekey Consumption (X3DH OTK Burn)
7. [Section E — Views](#7-section-e--views)
8. [Section F — Execution Plan Analysis](#8-section-f--execution-plan-analysis)
9. [Section G — Integrity Verification](#9-section-g--integrity-verification)
10. [CP3 Deliverable Summary & Verification Matrix](#10-cp3-deliverable-summary--verification-matrix)

---

## 1. Executive Summary & Query Architecture Scope

Checkpoint 3 (CP3) constructs the **production-grade SQL query suite** for the xChat decentralized messaging system's local storage engine. Building directly upon the BCNF relational schema established in CP2 (`dbms/03_schema.sql`), this checkpoint specifies:

1. **16 DML / analytical SQL queries** covering every critical runtime operation of the xChat node daemon — from message thread retrieval to cryptographic session state machine lookups.
2. **5 ACID-compliant transaction blocks** guarding against anomalies unique to cryptographic peer-to-peer systems, including the _ghost message anomaly_, the _ratchet desynchronization anomaly_, and the _prekey double-spend anomaly_.
3. **4 reusable SQL views** that encapsulate complex multi-table joins into stable virtual relations consumed by the application layer.
4. **5 `EXPLAIN QUERY PLAN` execution plans** formally demonstrating that every performance-critical query hits a covering index (zero full table scans on hot paths).
5. **6 integrity assertions** that programmatically verify correctness invariants after all data mutations.

### Why Query Design Requires Cryptographic Context

In a conventional centralized chat system, a suboptimal query plan causes UI latency. In xChat, a poorly designed query causes **catastrophic cryptographic failures**:

| Failure Scenario | Root Cause | Consequence |
| :--- | :--- | :--- |
| **Ratchet State Race Condition** | Non-atomic read-modify-write of `ratchet_sessions` without a transaction | Two outbound messages share the same symmetric ratchet key — **breaks forward secrecy** |
| **Ghost Message Anomaly** | Message inserted without its `outbox_queue` record (partial commit) | Message appears sent in UI but is never actually dispatched over any transport |
| **Prekey Double-Spend** | Two simultaneous X3DH initiators consuming the same One-Time Prekey | Two peers derive the same session key material — **destroys X3DH security** |
| **Skipped Key Memory Leak** | No TTL pruning of `skipped_keys` table | Unbounded accumulation of decryption keys for messages that will never arrive — **memory exhaustion on mobile** |

---

## 2. Dataset Description & Seed Schema

The CP3 seed dataset constructs a realistic 3-node xChat network for query demonstration:

```
Network Topology:
  Alice_Node  (usr_alice) ──── Verified P2P ────  Bob_Laptop  (usr_bob)
       │                                                │
       └──── Verified P2P ────  Carol_Phone (usr_carol) ┘
```

### Seed Data Inventory

| Table | Rows | Description |
| :--- | :---: | :--- |
| `users` | 3 | Alice, Bob, Carol — 3 independent xChat nodes |
| `identity_keys` | 6 | 2 keys each (Ed25519 signing + X25519 DH) |
| `prekey_bundles` | 7 | Signed prekeys + one-time prekeys (1 consumed for Alice) |
| `contacts` | 4 | Alice knows Bob & Carol; Bob knows Alice + 1 unverified unknown |
| `conversations` | 4 | 2 direct (Alice↔Bob, Alice↔Carol), 1 Bob↔Alice mirror, 1 Burner room |
| `ratchet_sessions` | 3 | Active DH ratchet states at step 7 (AB), step 2 (AC), step 7 (BA) |
| `skipped_keys` | 3 | Out-of-order key cache for session AB (ratchet steps r5+r6) |
| `messages` | 8 | Mix of READ / DELIVERED / SENT / PENDING states across threads |
| `outbox_queue` | 4 | QUEUED, IN_FLIGHT, and DEAD_LETTER envelopes across 4 carriers |
| `relay_cache` | 3 | 2 active + 1 intentionally expired relay packet for GC testing |

---

## 3. Section A — DML Queries: Core Retrieval & Mutation

### Query 1: Active Conversation Feed (Sidebar Chat List)

**Purpose:** Renders Alice's conversation sidebar — the primary UI screen showing all active threads sorted by most recent activity with unread message counts.

**Key Design Decisions:**

- Uses a **LEFT JOIN with conditional COUNT** (`CASE WHEN`) to count only incoming messages (`sender_address = ct.peer_address`) with unread status, without a subquery.
- The **`ORDER BY c.last_active_at DESC`** aligns with the `idx_conversations_user_active` index on `(user_id, last_active_at DESC)`, yielding an $O(\log N)$ index scan.

**Query 1A — Full conversation feed with unread counts:**

```sql
SELECT
    c.conversation_id,
    c.title,
    c.conv_type,
    c.last_active_at,
    ct.peer_name,
    ct.reachable_socket,
    ct.trust_status,
    COUNT(CASE WHEN m.delivery_status IN ('PENDING', 'SENT', 'DELIVERED')
               AND m.sender_address = ct.peer_address THEN 1 END) AS unread_count
FROM conversations c
JOIN contacts ct    ON c.contact_id = ct.contact_id
LEFT JOIN messages m ON c.conversation_id = m.conversation_id
WHERE c.user_id = 'usr_alice'
GROUP BY c.conversation_id, ...
ORDER BY c.last_active_at DESC;
```

**Execution Plan Verified:**
```
|--SEARCH c USING INDEX idx_conversations_user_active (user_id=?)
`--SEARCH ct USING INDEX sqlite_autoindex_contacts_1 (contact_id=?)
```

**Sample Output:**

| conversation_id | title | conv_type | last_active_at | peer_name | trust_status | unread_count |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `conv_ab` | Secure Chat: Alice ↔ Bob | DIRECT | 2026-11-10 22:01:00 | Bob_Laptop | VERIFIED | 1 |
| `conv_ac` | Secure Chat: Alice ↔ Carol | DIRECT | 2026-11-10 20:30:00 | Carol_Phone | VERIFIED | 1 |
| `conv_ab_burner` | Ephemeral Burner Room #A7F3 | BURNER | 2026-11-05 16:00:00 | Bob_Laptop | VERIFIED | 0 |

---

### Query 2: Aggregate Unread Badge Counter

**Purpose:** Counts the total unread message count for the app icon notification badge and per-conversation badge overlays.

**Key Design Decisions:**

- Filters by `delivery_status IN ('PENDING', 'SENT', 'DELIVERED')` and `sender_address = ct.peer_address` to count only *incoming unread* messages.
- Uses `idx_messages_delivery_status` for a targeted B-tree range scan.
- The `HAVING COUNT > 0` clause eliminates conversations with zero unread before returning results.

**Sample Output (per-conversation breakdown):**

| conversation_id | title | unread_count |
| :--- | :--- | :--- |
| `conv_ab` | Secure Chat: Alice ↔ Bob | 1 |
| `conv_ac` | Secure Chat: Alice ↔ Carol | 1 |

---

### Query 3: Message Thread History with Cursor Pagination

**Purpose:** Retrieves the message history for a conversation thread. Uses **cursor-based pagination** (instead of `OFFSET`) for $O(\log N)$ performance on large histories.

**Why cursor pagination over OFFSET:**

$\text{OFFSET}(k)$ requires the database to scan and discard $k$ rows, so cost = $O(k)$. Cursor pagination filters by `sent_at < cursor_timestamp`, seeking directly to the relevant B-tree leaf node, so cost = $O(\log N)$.

**Initial page load (most recent 20 messages):**

```sql
SELECT message_id, sender_address, ciphertext, nonce,
       ratchet_counter, delivery_status, sent_at, delivered_at
FROM messages
WHERE conversation_id = 'conv_ab'
ORDER BY sent_at DESC
LIMIT 20;
```

**Subsequent page (cursor-based older messages):**

```sql
SELECT ...
FROM messages
WHERE conversation_id = 'conv_ab'
  AND sent_at < '2026-11-10 10:02:00'   -- cursor: last sent_at from previous page
ORDER BY sent_at DESC
LIMIT 20;
```

**Execution Plan Verified:**
```
`--SEARCH messages USING INDEX idx_messages_conversation_sent_at (conversation_id=?)
```

---

### Query 4: Double Ratchet State Machine Lookup

**Purpose:** The most latency-sensitive query in the entire system — called on **every** packet encrypt/decrypt cycle to load the active ratchet session state from disk.

**Key Design Decisions:**

- Point lookup by `conversation_id` with the `idx_ratchet_sessions_conversation` unique index — guaranteed $O(1)$ seek.
- Includes a subquery counting cached `skipped_keys` for the diagnostics panel.
- The **skipped keys lookup subquery** (by `dh_ratchet_pub` + `message_counter`) enables decrypting out-of-order messages that arrived before their ratchet step was processed.

```sql
SELECT rs.session_id, rs.peer_dh_ratchet_pub, rs.send_counter, rs.recv_counter, ...
FROM ratchet_sessions rs
WHERE rs.conversation_id = 'conv_ab';
```

**Execution Plan Verified:**
```
`--SEARCH ratchet_sessions USING INDEX sqlite_autoindex_ratchet_sessions_2 (conversation_id=?)
```

**Sample Output:**

| session_id | send_counter | recv_counter | last_ratchet_at | cached_skipped_keys |
| :--- | :--- | :--- | :--- | :--- |
| `ses_ab` | 7 | 5 | 2026-11-10 22:00:00 | 3 |

---

### Query 5: Multi-Transport Outbox Dispatcher

**Purpose:** The background transport daemon calls this query to fetch the next batch of envelopes ready for dispatch. Prioritizes by retry count (ascending) and creation time.

**Key Design Decisions:**

- Double-condition filter `status = 'QUEUED' AND next_retry_at <= CURRENT_TIMESTAMP` maps directly to the `idx_outbox_queue_priority` compound index on `(status, next_retry_at)`.
- Joins to `contacts` to retrieve the peer's current socket address (which may change as the peer roams between IP addresses).
- `LIMIT 50` bounds the batch size to prevent the dispatcher from holding excessive in-memory state.

**Execution Plan Verified:**
```
|--SEARCH outbox_queue USING INDEX idx_outbox_queue_priority (status=? AND next_retry_at<?)
```

**Transport breakdown query output:**

| transport_type | status | envelope_count |
| :--- | :--- | :--- |
| TCP_LAN | QUEUED | 1 |
| BLE_MESH | QUEUED | 1 |
| SMS_MODEM | DEAD_LETTER | 1 |
| TCP_LAN | IN_FLIGHT | 1 |

---

### Query 6: X3DH Prekey Bundle Fetch

**Purpose:** When Alice initiates an X3DH session with an offline Bob, she fetches Bob's prekey bundle (identity key + signed prekey + one fresh one-time prekey). This query retrieves the bundle.

**Key Design Decisions:**

- Joins `identity_keys`, `prekey_bundles` (signed), and `prekey_bundles` (OTK) in a single query.
- `ORDER BY otk.key_id_num ASC LIMIT 1` selects the **lowest-numbered (oldest) unconsumed OTK** — matching Signal Protocol conventions for key rotation.
- The OTK is consumed atomically in **Transaction T5** after the bundle is fetched.

---

### Query 7: Peer Contact Discovery & Verification

**Purpose:** When a raw encrypted packet arrives over the mesh, the daemon immediately looks up the sender's address to verify they are a known, non-blocked contact before spending CPU on decryption.

**Key Design Decisions:**

- Uses the `idx_contacts_user_peer` compound index on `(user_id, peer_address)` for $O(\log N)$ identity resolution.
- **Short-circuit trust check:** A second query returns `NULL` if the contact is `BLOCKED`, allowing the daemon to drop the packet with zero further processing.

```sql
-- If NULL is returned, drop the packet
SELECT ct.contact_id FROM contacts ct
WHERE ct.user_id = 'usr_alice'
  AND ct.peer_address = 'xchat_bob_3c4d5e'
  AND ct.trust_status != 'BLOCKED';
```

---

### Query 8: Relay Cache Routing & TTL Garbage Collection

**Purpose:** Three-part relay node query suite:
- **(A) Routing lookup:** Retrieve opaque envelope by routing token for forwarding.
- **(B) Garbage collection:** Delete expired envelopes via indexed TTL scan.
- **(C) Diagnostics:** Count active, expired, and total relay buffer statistics.

**Key Design Decisions:**

- The GC `DELETE` uses `idx_relay_cache_ttl` on `expires_at`, enabling a targeted forward scan from the beginning of the index to `CURRENT_TIMESTAMP` — only expired rows are touched.
- The routing lookup adds `AND expires_at > CURRENT_TIMESTAMP` to silently discard expired packets without a separate TTL check.

**Execution Plan Verified:**
```
`--SEARCH relay_cache USING INDEX idx_relay_cache_ttl (expires_at<?)
```

---

### Query 9: Delivery Status Update (Read Receipt Processing)

**Purpose:** Updates `delivery_status` through the lifecycle `PENDING → SENT → DELIVERED → READ` as acknowledgements arrive from the peer node.

**Key Design Decisions:**

- Conditional `AND delivery_status = 'SENT'` guard on the single-message update prevents retrograde status transitions (e.g., accidentally re-marking a READ message as DELIVERED).
- The bulk READ-marking query filters `sender_address != 'xchat_alice_1a2b3c'` to only mark **incoming** messages (not Alice's own sent messages) as read when Alice opens the thread.

---

### Query 10: Outbox Lifecycle Management

**Purpose:** Full lifecycle management of envelopes in the store-and-forward outbox: `QUEUED → IN_FLIGHT → ACKNOWLEDGED` (success) or `QUEUED → DEAD_LETTER` (failure).

**Exponential Backoff Formula:**

$$\text{next\_retry\_at} = \text{now} + 60 \times 2^{\text{retry\_count}} \text{ seconds}$$

This yields delays of 60s, 120s, 240s, 480s, and 960s before the envelope is marked `DEAD_LETTER` at `max_retries = 5`.

```sql
UPDATE outbox_queue
SET next_retry_at = datetime(CURRENT_TIMESTAMP, '+' || (60 * (1 << retry_count)) || ' seconds'),
    retry_count   = retry_count + 1,
    status        = 'QUEUED'
WHERE envelope_id = 'env_ab_01'
  AND status = 'IN_FLIGHT'
  AND retry_count < max_retries;
```

---

## 4. Section B — Aggregate & Analytics Queries

### Query 11: Per-User Cryptographic Identity Summary

**Purpose:** Full diagnostic summary of a node's cryptographic posture — counts of identity keys, available OTKs, contacts, trust levels, and conversations.

Uses multi-way `LEFT JOIN` aggregation across all user-linked tables (`identity_keys`, `prekey_bundles`, `contacts`, `conversations`) collapsed into a single row per user with conditional counts.

**Sample Output (Alice):**

| user_id | username | total_identity_keys | available_otks | total_contacts | verified_contacts | total_conversations |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `usr_alice` | Alice_Node | 2 | 0 | 2 | 2 | 3 |

> **Note:** Alice's available OTK count is 0 because her last unconsumed OTK (`pre_alice_otk1`) is consumed in Transaction T5. `v_prekey_inventory` flags this as `CRITICAL: NO OTK`.

---

### Query 12: Message Volume & Delivery Latency Statistics

**Purpose:** Computes message count per delivery status and **average delivery latency** (seconds from `sent_at` to `delivered_at`) for each status tier. Used for the connection quality dashboard.

**Latency Computation:**

$$\text{latency\_sec} = \left(\text{julianday}(\text{delivered\_at}) - \text{julianday}(\text{sent\_at})\right) \times 86400$$

SQLite's `julianday()` function converts timestamps to fractional Julian Day Numbers (resolution: ~0.1 ms), and multiplying by 86,400 converts to seconds.

**Sample Output:**

| delivery_status | message_count | avg_delivery_latency_sec | min_latency_sec | max_latency_sec |
| :--- | :--- | :--- | :--- | :--- |
| READ | 4 | 5.5 | 3.0 | 8.0 |
| DELIVERED | 2 | 3.5 | 3.0 | 4.0 |
| SENT | 1 | NULL | NULL | NULL |
| PENDING | 2 | NULL | NULL | NULL |

---

### Query 13: Double Ratchet Health Report

**Purpose:** Surfaces potentially desynchronized or stale ratchet sessions to the security diagnostics panel. Computes a per-session health score based on:

- **Skipped key count:** > 100 → `CRITICAL`, > 20 → `WARNING`
- **Send/receive counter delta:** > 50 → `WARNING` (indicates the peer may be holding many unreceived messages)

The 3 cached skipped keys for session `ses_ab` correctly yield status `HEALTHY` (3 < 20).

**Sample Output:**

| session_id | peer_name | send_counter | recv_counter | delta | skipped_key_count | health |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `ses_ab` | Bob_Laptop | 7 | 5 | 2 | 3 | HEALTHY |
| `ses_ac` | Carol_Phone | 2 | 2 | 0 | 0 | HEALTHY |

---

### Query 14: Transport Carrier Reliability Audit

**Purpose:** Aggregates success rate, dead-letter rate, and average retry count per transport carrier (`TCP_LAN`, `BLE_MESH`, `SMS_MODEM`, `SPHINX_MIXNET`). Drives the "Carrier Hub" reliability panel in the UI.

**Success Rate Formula:**

$$\text{success\_rate} = \frac{\text{ACKNOWLEDGED}}{\text{TOTAL}} \times 100\%$$

`NULLIF(COUNT(*), 0)` prevents division-by-zero when a carrier has no dispatches.

---

## 5. Section C — Multi-Table Join Queries

### Query 15: Full Session Diagnostic Join

**Purpose:** A 7-way join producing a comprehensive single-row-per-conversation health view spanning `users → contacts → conversations → ratchet_sessions → skipped_keys → messages`.

This query is used by the diagnostics screen to show the complete state of every conversation from identity through to message delivery in a single, human-readable table.

**Join Graph:**
```
users ──JOIN── contacts ──JOIN── conversations ──LEFT JOIN── ratchet_sessions
                                      │                              │
                              LEFT JOIN messages            LEFT JOIN skipped_keys
```

---

### Query 16: Prekey Exhaustion Alert

**Purpose:** Identifies users with fewer than 3 available One-Time Prekeys and triggers background generation of a fresh prekey batch. This is the cryptographic equivalent of a "low fuel" warning — running out of OTKs forces X3DH to operate without one-time prekeys, **reducing forward secrecy guarantees**.

```sql
SELECT u.user_id, u.username, COUNT(pb.prekey_id) AS available_otk_count
FROM users u
LEFT JOIN prekey_bundles pb ON u.user_id = pb.user_id
                           AND pb.key_type = 'ONE_TIME_PREKEY'
                           AND pb.is_consumed = 0
GROUP BY u.user_id, u.username
HAVING COUNT(pb.prekey_id) < 3;
```

**Sample Output:**

| user_id | username | available_otk_count |
| :--- | :--- | :--- |
| `usr_alice` | Alice_Node | 0 |
| `usr_bob` | Bob_Laptop | 1 |
| `usr_carol` | Carol_Phone | 1 |

---

## 6. Section D — Transaction Management

All five transactions are designed to satisfy the **ACID properties** (Atomicity, Consistency, Isolation, Durability) with specific regard to the cryptographic failure modes unique to xChat.

### Transaction T1: Atomic Message Send with Outbox Enqueue

**Problem (Ghost Message Anomaly):** Without a transaction, if `INSERT INTO messages` succeeds but the application crashes before `INSERT INTO outbox_queue`, the message appears in the UI conversation history as "sent" but is never dispatched over any transport — a silent data loss failure.

**Solution:** Wraps both INSERTs in a single `BEGIN TRANSACTION / COMMIT`. The SQLite WAL journal guarantees that either both rows persist or neither does.

```sql
BEGIN TRANSACTION;
  INSERT INTO messages  (...) VALUES ('msg_ab_07', 'conv_ab', ...);
  INSERT INTO outbox_queue (...) VALUES ('env_ab_07', 'conv_ab', ...);
COMMIT;
```

**Anomaly Prevented:** Ghost Message Anomaly.

---

### Transaction T2: Atomic Double Ratchet State Advance

**Problem (Ratchet Desynchronization Anomaly):** The ratchet state advance involves three operations: (1) UPDATE the ratchet counters and DH key, (2) INSERT any new skipped keys, (3) DELETE expired skipped keys. If any sub-operation fails partway through, the local ratchet state diverges from the peer's expected state — causing **permanent decryption failure** for all subsequent messages.

**Solution:** All three operations execute within a single transaction. SQLite's `INSERT OR IGNORE` prevents duplicate-key errors if the skipped key already exists.

```sql
BEGIN TRANSACTION;
  UPDATE ratchet_sessions SET peer_dh_ratchet_pub='...', send_counter=8, ...
    WHERE conversation_id = 'conv_ab';
  INSERT OR IGNORE INTO skipped_keys (session_id, dh_ratchet_pub, message_counter, ...)
    VALUES ('ses_ab', 'bob_eph_dh_pub_ratchet_r7', 6, ...);
  DELETE FROM skipped_keys WHERE session_id='ses_ab' AND expires_at <= CURRENT_TIMESTAMP;
COMMIT;
```

**Anomaly Prevented:** Ratchet Desynchronization Anomaly.

---

### Transaction T3: Atomic Contact Verification Update

**Problem (Trust Level Race Condition):** In a concurrent scenario, if two threads both read a contact's trust status as `UNVERIFIED` and then both write `VERIFIED`, there is a window where a malicious packet from a blocked peer could be accepted between the read and write.

**Solution:** The `UPDATE` includes a **conditional guard** `AND trust_status = 'UNVERIFIED'`. The subsequent `SELECT changes()` returns the number of rows actually modified — if `0`, the contact was already in another state and no update occurred, preventing silent race conditions.

```sql
BEGIN TRANSACTION;
  UPDATE contacts SET trust_status = 'VERIFIED', updated_at = CURRENT_TIMESTAMP
  WHERE user_id = 'usr_bob' AND peer_address = 'xchat_unkn_000000'
    AND trust_status = 'UNVERIFIED';
  SELECT changes() AS rows_updated;  -- Must be 1
COMMIT;
```

**Anomaly Prevented:** Trust Level Race Condition.

---

### Transaction T4: Atomic Outbox Acknowledgement & Cleanup

**Problem (Partial ACK Anomaly):** When a P2P delivery ACK arrives, three state changes must happen together: (1) mark the message `DELIVERED`, (2) mark the outbox envelope `ACKNOWLEDGED`, (3) update `conversations.last_active_at`. If only (1) or (2) succeeds, the outbox retries dispatching an already-delivered message — causing the peer to receive **duplicate encrypted frames**.

**Solution:** All three `UPDATE` statements execute atomically in a single transaction.

**Anomaly Prevented:** Partial ACK / Duplicate Delivery Anomaly.

---

### Transaction T5: Safe Prekey Consumption (X3DH OTK Burn)

**Problem (Prekey Double-Spend Anomaly):** If two remote peers simultaneously initiate X3DH sessions with Alice and both are assigned the same One-Time Prekey, they derive **identical session key material** — catastrophically breaking the forward secrecy and authentication guarantees of the X3DH protocol.

**Solution:** The `UPDATE` atomically marks the OTK as consumed *only if it is currently unconsumed* (`AND is_consumed = 0`). `SELECT changes()` verifies exactly 1 row was updated. If `0` rows updated, the OTK was already consumed by a concurrent initiator — the caller must fall back to using no OTK.

```sql
BEGIN TRANSACTION;
  UPDATE prekey_bundles
  SET is_consumed = 1
  WHERE prekey_id = 'pre_alice_otk1' AND is_consumed = 0;
  SELECT changes() AS otk_consumed;  -- Must be 1; if 0, OTK already consumed
COMMIT;
```

**Anomaly Prevented:** X3DH Prekey Double-Spend Anomaly.

---

## 7. Section E — Views

Four reusable SQL views are defined to encapsulate complex multi-table joins into stable virtual relations. This follows the **separation of concerns** principle — the application layer queries simple named views rather than repeating complex JOIN logic.

| View Name | Tables Joined | Purpose |
| :--- | :--- | :--- |
| `v_active_conversations` | `conversations`, `contacts`, `messages` | Sidebar UI conversation feed with peer info + unread count |
| `v_ratchet_session_health` | `ratchet_sessions`, `conversations`, `users`, `contacts`, `skipped_keys` | Per-session health diagnostics (HEALTHY / WARNING / CRITICAL) |
| `v_outbox_pending_dispatch` | `outbox_queue`, `conversations`, `contacts` | Ready-to-dispatch envelopes with peer socket info, excluding blocked peers |
| `v_prekey_inventory` | `users`, `prekey_bundles` | OTK supply status per user (OK / WARNING: LOW OTK / CRITICAL: NO OTK) |

**Example — View `v_prekey_inventory` output:**

| user_id | username | signed_prekeys_available | otk_available | otk_consumed | prekey_status |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `usr_alice` | Alice_Node | 1 | 0 | 2 | CRITICAL: NO OTK |
| `usr_bob` | Bob_Laptop | 1 | 1 | 0 | WARNING: LOW OTK |
| `usr_carol` | Carol_Phone | 1 | 1 | 0 | WARNING: LOW OTK |

---

## 8. Section F — Execution Plan Analysis

Five `EXPLAIN QUERY PLAN` outputs were captured to verify that every high-frequency hot-path query uses a covering B-tree index — confirming that **zero full table scans** occur on operational queries.

| Query | Expected Index | Confirmed Plan |
| :--- | :--- | :--- |
| Q1 — Conversation feed | `idx_conversations_user_active` | `SEARCH c USING INDEX idx_conversations_user_active (user_id=?)` |
| Q3 — Message pagination | `idx_messages_conversation_sent_at` | `SEARCH messages USING INDEX idx_messages_conversation_sent_at (conversation_id=?)` |
| Q5 — Outbox dispatcher | `idx_outbox_queue_priority` | `SEARCH outbox_queue USING INDEX idx_outbox_queue_priority (status=? AND next_retry_at<?)` |
| Q4 — Ratchet lookup | `idx_ratchet_sessions_conversation` (UNIQUE) | `SEARCH ratchet_sessions USING INDEX sqlite_autoindex_ratchet_sessions_2 (conversation_id=?)` |
| Q8B — Relay GC | `idx_relay_cache_ttl` | `SEARCH relay_cache USING INDEX idx_relay_cache_ttl (expires_at<?)` |

> [!TIP]
> `sqlite_autoindex_ratchet_sessions_2` is the system-generated index name for the `UNIQUE` constraint on `ratchet_sessions.conversation_id` defined in the DDL. Its behavior is identical to a user-defined index.

---

## 9. Section G — Integrity Verification

Six programmatic assertion queries verify correctness invariants after all DML and transaction operations:

| Assertion ID | Invariant Checked | Expected Result |
| :--- | :--- | :--- |
| **H1** | Every `messages.conversation_id` FK resolves to a valid `conversations` row | `PASS` |
| **H2** | No active session with both `send_counter = 0` and `recv_counter = 0` has associated messages | `PASS` |
| **H3** | No `outbox_queue` row has `retry_count > max_retries` | `PASS` |
| **H4** | `PRAGMA foreign_key_check` returns zero violations across all 10 tables | `PASS` (empty result) |
| **H5** | No `conversation_id` has more than one `ratchet_sessions` row (1:1 constraint) | `PASS` |
| **H6** | All `SIGNED_PREKEY` entries have a non-null `signature` | `PASS` |

**Verified live execution (exit code `0`):**
```
PASS: All messages have valid conversation
PASS: No idle zero-counter sessions with messages
PASS: No outbox envelope exceeds max_retries
PASS: Strict 1:1 ratchet session per conversation
PASS: All signed prekeys have non-null signatures
PRAGMA foreign_key_check: (empty — 0 violations)
```

---

## 10. CP3 Deliverable Summary & Verification Matrix

| Deliverable | File | Status | Live Verification |
| :--- | :--- | :---: | :--- |
| SQL Query Suite | `dbms/04_queries.sql` | ✅ Complete | Executed against `03_schema.sql` — `EXIT:0`, zero errors |
| Query Documentation Report | `dbms/04_queries.md` | ✅ Complete | This document |
| Seed Dataset (Section A) | `04_queries.sql §A` | ✅ Complete | 10 tables × multi-row seed data committed successfully |
| DML Queries (Q1–Q10) | `04_queries.sql §B` | ✅ Complete | All 16 SELECT / UPDATE / DELETE queries produce correct output |
| Aggregate Queries (Q11–Q14) | `04_queries.sql §B` | ✅ Complete | All analytic queries return correct multi-table aggregations |
| Multi-Table Joins (Q15–Q16) | `04_queries.sql §C` | ✅ Complete | 7-way join and exhaustion alert query verified correct |
| Transaction T1 (Ghost Msg) | `04_queries.sql §D` | ✅ Complete | Atomic message + outbox INSERT committed and verified |
| Transaction T2 (Ratchet) | `04_queries.sql §D` | ✅ Complete | Atomic ratchet state advance + skipped key management verified |
| Transaction T3 (Trust Level) | `04_queries.sql §D` | ✅ Complete | Conditional trust UPDATE with `changes()` guard verified |
| Transaction T4 (Delivery ACK) | `04_queries.sql §D` | ✅ Complete | 3-way atomic delivery ACK update verified |
| Transaction T5 (OTK Burn) | `04_queries.sql §D` | ✅ Complete | Atomic OTK consumption with double-spend guard verified |
| Views (V1–V4) | `04_queries.sql §E` | ✅ Complete | All 4 views created and queried successfully |
| Execution Plan Analysis | `04_queries.sql §F` | ✅ Complete | 5/5 plans confirm index usage; zero full table scans on hot paths |
| Integrity Assertions | `04_queries.sql §G` | ✅ Complete | 6/6 assertions return `PASS`; `PRAGMA foreign_key_check` clean |

### Traceability: CP1 → CP2 → CP3

| CP1 Proof | CP2 Physical DDL | CP3 Query Coverage |
| :--- | :--- | :--- |
| $\text{user\_id} \to \text{Attributes}$ (users) | `users` table | Q1, Q2, Q11, Q16, V1, V4 |
| $\text{key\_id} \to \text{Attributes}$ (identity_keys) | `identity_keys` table | Q6, Q11 |
| $\text{prekey\_id} \to \text{Attributes}$ (prekey_bundles) | `prekey_bundles` table | Q6, Q11, Q16, T5, V4 |
| $\text{contact\_id} \to \text{Attributes}$ (contacts) | `contacts` table | Q1, Q7, Q15, T3, V1, V3 |
| $\text{conv\_id} \to \text{Attributes}$ (conversations) | `conversations` table | Q1, Q2, Q3, Q15, T1, T4, V1 |
| $\text{msg\_id} \to \text{Attributes}$ (messages) | `messages` table | Q2, Q3, Q9, Q12, T1, T4 |
| $\text{session\_id} \to \text{Attributes}$ (ratchet_sessions) | `ratchet_sessions` table | Q4, Q13, T2, V2 |
| $\text{Composite} \to \text{Key}$ (skipped_keys) | `skipped_keys` table | Q4, Q13, T2 |
| $\text{envelope\_id} \to \text{Attributes}$ (outbox_queue) | `outbox_queue` table | Q5, Q10, Q14, T1, T4, V3 |
| $\text{route\_token} \to \text{Attributes}$ (relay_cache) | `relay_cache` table | Q8, Q13 |

> [!IMPORTANT]
> Every BCNF relation proven in CP1 has at least one dedicated query, transaction, or view in CP3 that exercises its primary key lookup, mutation, and referential integrity constraints.
