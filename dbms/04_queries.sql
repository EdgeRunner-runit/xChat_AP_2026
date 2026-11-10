-- ============================================================================
-- SQL Query Suite: xChat Relational Storage Engine
-- Project: xChat — Decentralized Multi-Transport Private Messenger
-- Module: DBMS Query Design, Execution Plans & Transaction Management
-- Author: Aryan Singh Damara
-- Course: Advanced Programming / Database Management Systems
-- Target: CP3 (dbms/04_queries.sql)
-- Standard: SQLite 3.35+ and PostgreSQL 14+ Compatible
-- Prerequisite: dbms/03_schema.sql (BCNF DDL + Triggers + Indexes)
-- ============================================================================

PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA busy_timeout = 5000;

-- ============================================================================
-- SECTION A: SEED DATA FOR QUERY DEMONSTRATION
-- Full multi-user, multi-conversation, multi-transport dataset
-- ============================================================================

BEGIN TRANSACTION;

-- Users (local nodes)
INSERT INTO users (user_id, username, master_fingerprint, created_at) VALUES
    ('usr_alice', 'Alice_Node',   'A1B2C3D4E5F6A7B8C9D0E1F2A3B4C5D6E7F8A9B0C1D2E3F4A5B6C7D8E9F0A1B2', '2026-10-01 09:00:00'),
    ('usr_bob',   'Bob_Laptop',   'B2C3D4E5F6A7B8C9D0E1F2A3B4C5D6E7F8A9B0C1D2E3F4A5B6C7D8E9F0A1B2C3', '2026-10-02 10:00:00'),
    ('usr_carol', 'Carol_Phone',  'C3D4E5F6A7B8C9D0E1F2A3B4C5D6E7F8A9B0C1D2E3F4A5B6C7D8E9F0A1B2C3D4', '2026-10-03 11:00:00');

-- Identity Keys
INSERT INTO identity_keys (key_id, user_id, key_type, public_key, encrypted_privkey, created_at) VALUES
    ('key_alice_ed',  'usr_alice', 'ED25519_SIGNING', 'ed25519_alice_pub_1a2b3c', X'AABBCCDDEEFF0011', '2026-10-01 09:00:00'),
    ('key_alice_dh',  'usr_alice', 'X25519_DH',       'x25519_alice_pub_2b3c4d',  X'BBCCDDEEFF001122', '2026-10-01 09:00:00'),
    ('key_bob_ed',    'usr_bob',   'ED25519_SIGNING', 'ed25519_bob_pub_3c4d5e',   X'CCDDEEFF00112233', '2026-10-02 10:00:00'),
    ('key_bob_dh',    'usr_bob',   'X25519_DH',       'x25519_bob_pub_4d5e6f',    X'DDEEFF0011223344', '2026-10-02 10:00:00'),
    ('key_carol_ed',  'usr_carol', 'ED25519_SIGNING', 'ed25519_carol_pub_5e6f7a', X'EEFF001122334455', '2026-10-03 11:00:00'),
    ('key_carol_dh',  'usr_carol', 'X25519_DH',       'x25519_carol_pub_6f7a8b',  X'FF00112233445566', '2026-10-03 11:00:00');

-- Prekey Bundles (Alice: 3 keys; Bob: 2 keys; Carol: 2 keys)
INSERT INTO prekey_bundles (prekey_id, user_id, key_id_num, key_type, public_key, signature, is_consumed, created_at) VALUES
    ('pre_alice_spk',  'usr_alice', 1, 'SIGNED_PREKEY',   'alice_spk_pub_x25519_01', 'alice_sig_spk_01', 0, '2026-10-01 09:01:00'),
    ('pre_alice_otk1', 'usr_alice', 2, 'ONE_TIME_PREKEY', 'alice_otk_pub_x25519_02', NULL,               0, '2026-10-01 09:01:00'),
    ('pre_alice_otk2', 'usr_alice', 3, 'ONE_TIME_PREKEY', 'alice_otk_pub_x25519_03', NULL,               1, '2026-10-01 09:01:00'),
    ('pre_bob_spk',    'usr_bob',   1, 'SIGNED_PREKEY',   'bob_spk_pub_x25519_01',   'bob_sig_spk_01',   0, '2026-10-02 10:01:00'),
    ('pre_bob_otk1',   'usr_bob',   2, 'ONE_TIME_PREKEY', 'bob_otk_pub_x25519_02',   NULL,               0, '2026-10-02 10:01:00'),
    ('pre_carol_spk',  'usr_carol', 1, 'SIGNED_PREKEY',   'carol_spk_pub_x25519_01', 'carol_sig_spk_01', 0, '2026-10-03 11:01:00'),
    ('pre_carol_otk1', 'usr_carol', 2, 'ONE_TIME_PREKEY', 'carol_otk_pub_x25519_02', NULL,               0, '2026-10-03 11:01:00');

-- Contacts (Alice knows Bob & Carol; Bob knows Alice)
INSERT INTO contacts (contact_id, user_id, peer_address, peer_name, reachable_socket, public_key, fingerprint, trust_status, created_at, updated_at) VALUES
    ('cnt_alice_bob',   'usr_alice', 'xchat_bob_3c4d5e',   'Bob_Laptop',  '192.168.1.15:9400', 'ed25519_bob_pub_3c4d5e',   'FP:3C:4D:5E:6F:7A:8B:9C:0D', 'VERIFIED',   '2026-10-04 08:00:00', '2026-11-01 14:00:00'),
    ('cnt_alice_carol', 'usr_alice', 'xchat_carol_5e6f7a', 'Carol_Phone', '192.168.1.33:9400', 'ed25519_carol_pub_5e6f7a', 'FP:5E:6F:7A:8B:9C:0D:1E:2F', 'VERIFIED',   '2026-10-05 09:00:00', '2026-11-02 10:00:00'),
    ('cnt_bob_alice',   'usr_bob',   'xchat_alice_1a2b3c', 'Alice_Node',  '192.168.1.10:9400', 'ed25519_alice_pub_1a2b3c', 'FP:1A:2B:3C:4D:5E:6F:7A:8B', 'VERIFIED',   '2026-10-04 08:05:00', '2026-11-01 14:05:00'),
    ('cnt_bob_unknown', 'usr_bob',   'xchat_unkn_000000',  'Unknown_Peer','10.0.0.99:9400',    'ed25519_unkn_pub_000000',  'FP:00:00:00:00:00:00:00:00', 'UNVERIFIED', '2026-10-10 12:00:00', '2026-10-10 12:00:00');

-- Conversations
INSERT INTO conversations (conversation_id, user_id, contact_id, conv_type, title, is_muted, created_at, last_active_at) VALUES
    ('conv_ab', 'usr_alice', 'cnt_alice_bob',   'DIRECT',    'Secure Chat: Alice <-> Bob',   0, '2026-10-04 08:10:00', '2026-11-10 22:00:00'),
    ('conv_ac', 'usr_alice', 'cnt_alice_carol', 'DIRECT',    'Secure Chat: Alice <-> Carol', 0, '2026-10-05 09:10:00', '2026-11-10 20:30:00'),
    ('conv_ba', 'usr_bob',   'cnt_bob_alice',   'DIRECT',    'Secure Chat: Bob <-> Alice',   0, '2026-10-04 08:10:00', '2026-11-10 22:00:00'),
    ('conv_ab_burner', 'usr_alice', 'cnt_alice_bob', 'BURNER', 'Ephemeral Burner Room #A7F3', 0, '2026-11-05 15:00:00', '2026-11-05 16:00:00');

-- Ratchet Sessions
INSERT INTO ratchet_sessions (session_id, conversation_id, peer_dh_ratchet_pub, local_dh_ratchet_pub, root_key_hash, send_chain_key_hash, recv_chain_key_hash, send_counter, recv_counter, last_ratchet_at) VALUES
    ('ses_ab', 'conv_ab', 'bob_eph_dh_pub_ratchet_r7',   'alice_eph_dh_pub_ratchet_r7',  'rk_hash_ab_step_7',  'cks_hash_ab_step_7',  'ckr_hash_ab_step_7',  7, 5, '2026-11-10 22:00:00'),
    ('ses_ac', 'conv_ac', 'carol_eph_dh_pub_ratchet_r2', 'alice_eph_dh_pub_ratchet_r2',  'rk_hash_ac_step_2',  'cks_hash_ac_step_2',  'ckr_hash_ac_step_2',  2, 2, '2026-11-10 20:30:00'),
    ('ses_ba', 'conv_ba', 'alice_eph_dh_pub_ratchet_r7', 'bob_eph_dh_pub_ratchet_r7',    'rk_hash_ba_step_7',  'cks_hash_ba_step_7',  'ckr_hash_ba_step_7',  5, 7, '2026-11-10 22:00:00');

-- Skipped Keys (out-of-order message keys cached for session ab)
INSERT INTO skipped_keys (session_id, dh_ratchet_pub, message_counter, message_key, expires_at) VALUES
    ('ses_ab', 'bob_eph_dh_pub_ratchet_r5', 3, 'mk_ab_r5_n3_encrypted', '2026-11-24 22:00:00'),
    ('ses_ab', 'bob_eph_dh_pub_ratchet_r5', 4, 'mk_ab_r5_n4_encrypted', '2026-11-24 22:00:00'),
    ('ses_ab', 'bob_eph_dh_pub_ratchet_r6', 1, 'mk_ab_r6_n1_encrypted', '2026-11-24 22:00:00');

-- Messages (Alice <-> Bob conversation)
INSERT INTO messages (message_id, conversation_id, sender_address, recipient_address, ciphertext, nonce, ratchet_counter, delivery_status, sent_at, delivered_at) VALUES
    ('msg_ab_01', 'conv_ab', 'xchat_alice_1a2b3c', 'xchat_bob_3c4d5e',   X'43495048455254455854', 'nonce_ab_01', 0, 'READ',      '2026-11-10 10:00:00', '2026-11-10 10:00:05'),
    ('msg_ab_02', 'conv_ab', 'xchat_bob_3c4d5e',   'xchat_alice_1a2b3c', X'43495048455254455855', 'nonce_ab_02', 0, 'READ',      '2026-11-10 10:01:00', '2026-11-10 10:01:03'),
    ('msg_ab_03', 'conv_ab', 'xchat_alice_1a2b3c', 'xchat_bob_3c4d5e',   X'43495048455254455856', 'nonce_ab_03', 1, 'READ',      '2026-11-10 10:02:00', '2026-11-10 10:02:08'),
    ('msg_ab_04', 'conv_ab', 'xchat_bob_3c4d5e',   'xchat_alice_1a2b3c', X'43495048455254455857', 'nonce_ab_04', 1, 'DELIVERED', '2026-11-10 20:00:00', '2026-11-10 20:00:04'),
    ('msg_ab_05', 'conv_ab', 'xchat_alice_1a2b3c', 'xchat_bob_3c4d5e',   X'43495048455254455858', 'nonce_ab_05', 2, 'SENT',      '2026-11-10 22:00:00', NULL),
    ('msg_ab_06', 'conv_ab', 'xchat_alice_1a2b3c', 'xchat_bob_3c4d5e',   X'43495048455254455859', 'nonce_ab_06', 3, 'PENDING',   '2026-11-10 22:01:00', NULL);

-- Messages (Alice <-> Carol conversation)
INSERT INTO messages (message_id, conversation_id, sender_address, recipient_address, ciphertext, nonce, ratchet_counter, delivery_status, sent_at, delivered_at) VALUES
    ('msg_ac_01', 'conv_ac', 'xchat_alice_1a2b3c', 'xchat_carol_5e6f7a', X'63495048455254455874', 'nonce_ac_01', 0, 'READ',      '2026-11-10 09:00:00', '2026-11-10 09:00:06'),
    ('msg_ac_02', 'conv_ac', 'xchat_carol_5e6f7a', 'xchat_alice_1a2b3c', X'63495048455254455875', 'nonce_ac_02', 0, 'DELIVERED', '2026-11-10 20:30:00', '2026-11-10 20:30:03');

-- Outbox Queue (pending envelopes for Bob, offline node)
INSERT INTO outbox_queue (envelope_id, conversation_id, destination_address, transport_type, raw_payload, retry_count, max_retries, next_retry_at, status, created_at) VALUES
    ('env_ab_01', 'conv_ab', 'xchat_bob_3c4d5e', 'TCP_LAN',      X'454E56454C4F504501', 0, 5, '2026-11-10 22:02:00', 'QUEUED',      '2026-11-10 22:01:00'),
    ('env_ab_02', 'conv_ab', 'xchat_bob_3c4d5e', 'BLE_MESH',     X'454E56454C4F504502', 2, 5, '2026-11-10 22:05:00', 'QUEUED',      '2026-11-10 22:01:30'),
    ('env_ab_03', 'conv_ab', 'xchat_bob_3c4d5e', 'SMS_MODEM',    X'454E56454C4F504503', 5, 5, '2026-11-10 22:30:00', 'DEAD_LETTER', '2026-11-10 20:00:00'),
    ('env_ac_01', 'conv_ac', 'xchat_carol_5e6f7a','TCP_LAN',     X'454E56454C4F504601', 0, 5, '2026-11-10 20:31:00', 'IN_FLIGHT',   '2026-11-10 20:30:00');

-- Relay Cache (blind mesh routing buffers)
INSERT INTO relay_cache (route_token, opaque_envelope, hop_count, ttl_seconds, received_at, expires_at) VALUES
    ('tok_relay_hop1_ab', X'52454C41595F424C494E445F454E56454C4F504501', 1, 3600, '2026-11-10 22:00:30', '2026-11-11 22:00:30'),
    ('tok_relay_hop2_ab', X'52454C41595F424C494E445F454E56454C4F504502', 2, 1800, '2026-11-10 22:00:35', '2026-11-10 23:00:35'),
    ('tok_relay_expired',  X'52454C41595F424C494E445F45585049524544',    3,    0, '2026-11-09 10:00:00', '2026-11-09 11:00:00');

COMMIT;

-- ============================================================================
-- SECTION B: DML QUERIES
-- Core data retrieval and manipulation operations
-- ============================================================================

-- ============================================================================
-- QUERY 1: Active Conversation Feed (Sidebar Chat List)
-- Retrieves all active conversations for a given user, ordered by most recent
-- activity. Covers the main chat sidebar showing last-active threads first.
-- Indexes used: idx_conversations_user_active, idx_contacts_user_peer
-- ============================================================================
-- Query 1A: Full conversation feed with peer info and unread message count
SELECT
    c.conversation_id,
    c.title,
    c.conv_type,
    c.last_active_at,
    c.is_muted,
    ct.peer_name,
    ct.peer_address,
    ct.reachable_socket,
    ct.trust_status,
    COUNT(CASE WHEN m.delivery_status IN ('PENDING', 'SENT', 'DELIVERED')
               AND m.sender_address = ct.peer_address THEN 1 END) AS unread_count
FROM conversations c
JOIN contacts ct ON c.contact_id = ct.contact_id
LEFT JOIN messages m ON c.conversation_id = m.conversation_id
WHERE c.user_id = 'usr_alice'
GROUP BY c.conversation_id, c.title, c.conv_type, c.last_active_at,
         c.is_muted, ct.peer_name, ct.peer_address, ct.reachable_socket, ct.trust_status
ORDER BY c.last_active_at DESC;

-- Query 1B: Latest message preview per conversation (last message snippet)
SELECT
    c.conversation_id,
    c.title,
    c.last_active_at,
    m.sender_address,
    m.delivery_status,
    m.sent_at AS last_message_time
FROM conversations c
JOIN messages m ON c.conversation_id = m.conversation_id
WHERE c.user_id = 'usr_alice'
  AND m.sent_at = (
      SELECT MAX(m2.sent_at)
      FROM messages m2
      WHERE m2.conversation_id = c.conversation_id
  )
ORDER BY c.last_active_at DESC;

-- ============================================================================
-- QUERY 2: Aggregate Unread Badge Counter
-- Counts total unread messages (PENDING, SENT, DELIVERED from peers) for the
-- notification badge on the application icon.
-- Indexes used: idx_messages_delivery_status, idx_messages_conversation_sent_at
-- ============================================================================
SELECT
    c.user_id,
    COUNT(m.message_id) AS total_unread
FROM messages m
JOIN conversations c ON m.conversation_id = c.conversation_id
JOIN contacts ct ON c.contact_id = ct.contact_id
WHERE c.user_id = 'usr_alice'
  AND m.delivery_status IN ('PENDING', 'SENT', 'DELIVERED')
  AND m.sender_address = ct.peer_address;

-- Per-conversation unread badge breakdown
SELECT
    c.conversation_id,
    c.title,
    COUNT(m.message_id) AS unread_count
FROM messages m
JOIN conversations c ON m.conversation_id = c.conversation_id
JOIN contacts ct ON c.contact_id = ct.contact_id
WHERE c.user_id = 'usr_alice'
  AND m.delivery_status IN ('PENDING', 'SENT', 'DELIVERED')
  AND m.sender_address = ct.peer_address
GROUP BY c.conversation_id, c.title
HAVING COUNT(m.message_id) > 0
ORDER BY unread_count DESC;

-- ============================================================================
-- QUERY 3: Message Thread History with Cursor Pagination
-- Retrieves the N most recent messages in a conversation, using the
-- (conversation_id, sent_at DESC) compound covering index for O(log N) seek.
-- Supports infinite scroll pagination via cursor-based offset.
-- Indexes used: idx_messages_conversation_sent_at
-- ============================================================================
-- Query 3A: Initial page load — most recent 20 messages
SELECT
    m.message_id,
    m.sender_address,
    m.recipient_address,
    m.ciphertext,
    m.nonce,
    m.ratchet_counter,
    m.delivery_status,
    m.sent_at,
    m.delivered_at
FROM messages m
WHERE m.conversation_id = 'conv_ab'
ORDER BY m.sent_at DESC
LIMIT 20;

-- Query 3B: Cursor-based next page (older messages before a given timestamp)
SELECT
    m.message_id,
    m.sender_address,
    m.ciphertext,
    m.delivery_status,
    m.sent_at
FROM messages m
WHERE m.conversation_id = 'conv_ab'
  AND m.sent_at < '2026-11-10 10:02:00'   -- cursor: last timestamp from previous page
ORDER BY m.sent_at DESC
LIMIT 20;

-- ============================================================================
-- QUERY 4: Double Ratchet State Machine Lookup
-- Retrieves the active cryptographic session state for a conversation.
-- Called on every inbound/outbound packet to load the ratchet state.
-- Indexes used: idx_ratchet_sessions_conversation
-- ============================================================================
SELECT
    rs.session_id,
    rs.peer_dh_ratchet_pub,
    rs.local_dh_ratchet_pub,
    rs.root_key_hash,
    rs.send_chain_key_hash,
    rs.recv_chain_key_hash,
    rs.send_counter,
    rs.recv_counter,
    rs.last_ratchet_at,
    -- Skipped key count for diagnostics
    (SELECT COUNT(*) FROM skipped_keys sk WHERE sk.session_id = rs.session_id) AS cached_skipped_keys
FROM ratchet_sessions rs
WHERE rs.conversation_id = 'conv_ab';

-- Skipped keys lookup for a specific DH ratchet step and counter (out-of-order decrypt)
SELECT
    sk.session_id,
    sk.dh_ratchet_pub,
    sk.message_counter,
    sk.message_key,
    sk.expires_at
FROM skipped_keys sk
JOIN ratchet_sessions rs ON sk.session_id = rs.session_id
WHERE rs.conversation_id = 'conv_ab'
  AND sk.dh_ratchet_pub = 'bob_eph_dh_pub_ratchet_r5'
  AND sk.message_counter = 3;

-- ============================================================================
-- QUERY 5: Multi-Transport Outbox Dispatcher
-- Fetches all QUEUED envelopes eligible for immediate retry (next_retry_at <= now),
-- ordered by retry urgency. Used by the daemon background transport dispatcher.
-- Indexes used: idx_outbox_queue_priority
-- ============================================================================
SELECT
    oq.envelope_id,
    oq.conversation_id,
    oq.destination_address,
    oq.transport_type,
    oq.raw_payload,
    oq.retry_count,
    oq.max_retries,
    oq.next_retry_at,
    oq.status,
    oq.created_at,
    ct.reachable_socket
FROM outbox_queue oq
JOIN conversations c ON oq.conversation_id = c.conversation_id
JOIN contacts ct ON c.contact_id = ct.contact_id
WHERE oq.status = 'QUEUED'
  AND oq.next_retry_at <= CURRENT_TIMESTAMP
ORDER BY oq.retry_count ASC, oq.created_at ASC
LIMIT 50;

-- Transport-specific breakdown: how many envelopes are waiting per carrier
SELECT
    transport_type,
    status,
    COUNT(*) AS envelope_count
FROM outbox_queue
GROUP BY transport_type, status
ORDER BY envelope_count DESC;

-- ============================================================================
-- QUERY 6: X3DH Prekey Bundle Fetch
-- Retrieves the full prekey bundle for a user to allow another node to initiate
-- an X3DH asynchronous key agreement (establish session while offline).
-- Returns one unconsumed One-Time Prekey and the Signed Prekey.
-- ============================================================================
SELECT
    ik.public_key   AS identity_key,
    spk.public_key  AS signed_prekey,
    spk.signature   AS signed_prekey_sig,
    otk.prekey_id   AS otk_id,
    otk.public_key  AS one_time_prekey
FROM users u
JOIN identity_keys ik ON u.user_id = ik.user_id AND ik.key_type = 'ED25519_SIGNING'
JOIN prekey_bundles spk ON u.user_id = spk.user_id
                        AND spk.key_type = 'SIGNED_PREKEY'
                        AND spk.is_consumed = 0
LEFT JOIN prekey_bundles otk ON u.user_id = otk.user_id
                             AND otk.key_type = 'ONE_TIME_PREKEY'
                             AND otk.is_consumed = 0
WHERE u.user_id = 'usr_alice'
ORDER BY otk.key_id_num ASC
LIMIT 1;

-- ============================================================================
-- QUERY 7: Peer Contact Discovery & Verification Lookup
-- Given an inbound packet's sender address, verify they exist in the contact
-- book, check trust level, and retrieve their public key for signature
-- verification. Used during packet ingress processing.
-- Indexes used: idx_contacts_user_peer
-- ============================================================================
SELECT
    ct.contact_id,
    ct.peer_name,
    ct.peer_address,
    ct.reachable_socket,
    ct.public_key,
    ct.fingerprint,
    ct.trust_status,
    ct.updated_at
FROM contacts ct
WHERE ct.user_id = 'usr_alice'
  AND ct.peer_address = 'xchat_bob_3c4d5e';

-- Blocked peer check: return NULL if peer is BLOCKED (drop their packets)
SELECT ct.contact_id
FROM contacts ct
WHERE ct.user_id = 'usr_alice'
  AND ct.peer_address = 'xchat_bob_3c4d5e'
  AND ct.trust_status != 'BLOCKED';

-- ============================================================================
-- QUERY 8: Relay Cache Mesh Routing Lookup & TTL Garbage Collection
-- (a) Lookup: Given a route_token from an incoming mesh packet header, retrieve
--     the opaque envelope for forwarding to the destination.
-- (b) Garbage Collection: Delete all expired relay envelopes.
-- Indexes used: idx_relay_cache_ttl
-- ============================================================================
-- Query 8A: Blind relay routing token lookup
SELECT
    rc.route_token,
    rc.opaque_envelope,
    rc.hop_count,
    rc.ttl_seconds,
    rc.received_at,
    rc.expires_at
FROM relay_cache rc
WHERE rc.route_token = 'tok_relay_hop1_ab'
  AND rc.expires_at > CURRENT_TIMESTAMP;

-- Query 8B: Relay cache TTL garbage collection (deletes expired packets)
DELETE FROM relay_cache
WHERE expires_at <= CURRENT_TIMESTAMP;

-- Query 8C: Relay cache statistics for diagnostics dashboard
SELECT
    COUNT(*) AS total_buffered,
    COUNT(CASE WHEN expires_at > CURRENT_TIMESTAMP THEN 1 END) AS active_count,
    COUNT(CASE WHEN expires_at <= CURRENT_TIMESTAMP THEN 1 END) AS expired_count,
    SUM(hop_count) AS total_hops_relayed
FROM relay_cache;

-- ============================================================================
-- QUERY 9: Delivery Status Update (Read Receipt Processing)
-- Updates the delivery_status of messages when a P2P ACK is received.
-- Called by the daemon on delivery confirmation from the peer node.
-- ============================================================================
-- Mark a specific message as DELIVERED when the peer's TCP ACK arrives
UPDATE messages
SET delivery_status = 'DELIVERED',
    delivered_at    = CURRENT_TIMESTAMP
WHERE message_id = 'msg_ab_05'
  AND delivery_status = 'SENT';

-- Bulk mark all DELIVERED messages in a conversation as READ
-- (triggered when user opens the conversation thread)
UPDATE messages
SET delivery_status = 'READ'
WHERE conversation_id = 'conv_ab'
  AND delivery_status = 'DELIVERED'
  AND sender_address != 'xchat_alice_1a2b3c';  -- Only mark incoming messages as read

-- ============================================================================
-- QUERY 10: Outbox Lifecycle Management (Retry & Dead-Letter)
-- (a) Advance a failed envelope to the next retry with exponential backoff.
-- (b) Mark a fully exhausted envelope as DEAD_LETTER.
-- (c) Remove acknowledged envelopes from the queue.
-- ============================================================================
-- Query 10A: Mark envelope as IN_FLIGHT when dispatch begins
UPDATE outbox_queue
SET status = 'IN_FLIGHT'
WHERE envelope_id = 'env_ab_01'
  AND status = 'QUEUED';

-- Query 10B: Retry with exponential backoff after transient failure
-- Backoff: 2^retry_count * 60 seconds (60s, 120s, 240s, 480s, 960s)
UPDATE outbox_queue
SET status        = 'QUEUED',
    retry_count   = retry_count + 1,
    next_retry_at = datetime(CURRENT_TIMESTAMP, '+' || (60 * (1 << retry_count)) || ' seconds')
WHERE envelope_id = 'env_ab_01'
  AND status = 'IN_FLIGHT'
  AND retry_count < max_retries;

-- Query 10C: Promote exhausted envelope to DEAD_LETTER
UPDATE outbox_queue
SET status = 'DEAD_LETTER'
WHERE status = 'QUEUED'
  AND retry_count >= max_retries;

-- Query 10D: Purge acknowledged envelopes from queue
DELETE FROM outbox_queue
WHERE status = 'ACKNOWLEDGED'
  AND created_at < datetime(CURRENT_TIMESTAMP, '-7 days');

-- ============================================================================
-- SECTION C: AGGREGATE & ANALYTICS QUERIES
-- ============================================================================

-- ============================================================================
-- QUERY 11: Per-User Cryptographic Identity Summary
-- Full diagnostic view of a node's cryptographic posture.
-- ============================================================================
SELECT
    u.user_id,
    u.username,
    u.master_fingerprint,
    u.created_at,
    COUNT(DISTINCT ik.key_id)                                            AS total_identity_keys,
    COUNT(DISTINCT CASE WHEN pb.is_consumed = 0
                        AND pb.key_type = 'ONE_TIME_PREKEY' THEN pb.prekey_id END) AS available_otks,
    COUNT(DISTINCT ct.contact_id)                                        AS total_contacts,
    COUNT(DISTINCT CASE WHEN ct.trust_status = 'VERIFIED' THEN ct.contact_id END) AS verified_contacts,
    COUNT(DISTINCT cv.conversation_id)                                   AS total_conversations
FROM users u
LEFT JOIN identity_keys ik  ON u.user_id = ik.user_id
LEFT JOIN prekey_bundles pb  ON u.user_id = pb.user_id
LEFT JOIN contacts ct        ON u.user_id = ct.user_id
LEFT JOIN conversations cv   ON u.user_id = cv.user_id
WHERE u.user_id = 'usr_alice'
GROUP BY u.user_id, u.username, u.master_fingerprint, u.created_at;

-- ============================================================================
-- QUERY 12: Message Volume & Delivery Statistics
-- How many messages are in each delivery state, and what is the average
-- delivery latency (sent_at to delivered_at) in seconds.
-- ============================================================================
SELECT
    m.delivery_status,
    COUNT(*)                            AS message_count,
    ROUND(
        AVG(
            CASE WHEN m.delivered_at IS NOT NULL
                 THEN (julianday(m.delivered_at) - julianday(m.sent_at)) * 86400.0
            END
        ), 3
    )                                   AS avg_delivery_latency_sec,
    MIN(
        CASE WHEN m.delivered_at IS NOT NULL
             THEN (julianday(m.delivered_at) - julianday(m.sent_at)) * 86400.0
        END
    )                                   AS min_latency_sec,
    MAX(
        CASE WHEN m.delivered_at IS NOT NULL
             THEN (julianday(m.delivered_at) - julianday(m.sent_at)) * 86400.0
        END
    )                                   AS max_latency_sec
FROM messages m
JOIN conversations c ON m.conversation_id = c.conversation_id
WHERE c.user_id = 'usr_alice'
GROUP BY m.delivery_status
ORDER BY message_count DESC;

-- ============================================================================
-- QUERY 13: Double Ratchet Health Report
-- Reports ratchet advancement state across all sessions — detects stale or
-- desynchronized ratchets (high skipped key accumulation is an alert signal).
-- ============================================================================
SELECT
    rs.session_id,
    c.title               AS conversation_title,
    ct.peer_name,
    rs.send_counter,
    rs.recv_counter,
    (rs.send_counter - rs.recv_counter) AS ratchet_send_recv_delta,
    COUNT(sk.session_id)                AS cached_skipped_keys,
    rs.last_ratchet_at,
    CASE
        WHEN COUNT(sk.session_id) > 100 THEN 'ALERT: HIGH_SKIPPED_KEY_COUNT'
        WHEN (rs.send_counter - rs.recv_counter) > 50 THEN 'WARN: LARGE_RATCHET_DELTA'
        ELSE 'HEALTHY'
    END AS ratchet_health_status
FROM ratchet_sessions rs
JOIN conversations c ON rs.conversation_id = c.conversation_id
JOIN contacts ct ON c.contact_id = ct.contact_id
LEFT JOIN skipped_keys sk ON rs.session_id = sk.session_id
GROUP BY rs.session_id, c.title, ct.peer_name, rs.send_counter, rs.recv_counter, rs.last_ratchet_at
ORDER BY cached_skipped_keys DESC;

-- ============================================================================
-- QUERY 14: Transport Carrier Reliability Audit
-- Analyses the success and failure rates per transport carrier type.
-- Useful for the UI "Carrier Hub" diagnostics panel.
-- ============================================================================
SELECT
    oq.transport_type,
    COUNT(*)                                                            AS total_dispatches,
    COUNT(CASE WHEN oq.status = 'ACKNOWLEDGED' THEN 1 END)             AS acknowledged,
    COUNT(CASE WHEN oq.status = 'DEAD_LETTER'  THEN 1 END)             AS dead_letter,
    COUNT(CASE WHEN oq.status IN ('QUEUED','IN_FLIGHT') THEN 1 END)    AS in_progress,
    ROUND(
        100.0 * COUNT(CASE WHEN oq.status = 'ACKNOWLEDGED' THEN 1 END)
              / NULLIF(COUNT(*), 0),
    2)                                                                  AS success_rate_pct,
    ROUND(AVG(oq.retry_count), 2)                                       AS avg_retries
FROM outbox_queue oq
GROUP BY oq.transport_type
ORDER BY success_rate_pct DESC NULLS LAST;

-- ============================================================================
-- SECTION D: MULTI-TABLE JOIN QUERIES
-- ============================================================================

-- ============================================================================
-- QUERY 15: Full Session Diagnostic Join
-- Comprehensive join across users → contacts → conversations →
-- ratchet_sessions → skipped_keys for a full session health dashboard.
-- ============================================================================
SELECT
    u.username,
    ct.peer_name,
    ct.reachable_socket,
    ct.trust_status,
    c.conv_type,
    c.last_active_at,
    rs.send_counter,
    rs.recv_counter,
    rs.last_ratchet_at,
    COUNT(sk.session_id) AS skipped_key_count,
    COUNT(m.message_id)  AS total_messages
FROM users u
JOIN contacts ct          ON u.user_id = ct.user_id
JOIN conversations c      ON ct.contact_id = c.contact_id AND u.user_id = c.user_id
LEFT JOIN ratchet_sessions rs ON c.conversation_id = rs.conversation_id
LEFT JOIN skipped_keys sk     ON rs.session_id = sk.session_id
LEFT JOIN messages m          ON c.conversation_id = m.conversation_id
WHERE u.user_id = 'usr_alice'
GROUP BY
    u.username, ct.peer_name, ct.reachable_socket, ct.trust_status,
    c.conv_type, c.last_active_at, rs.send_counter, rs.recv_counter, rs.last_ratchet_at
ORDER BY c.last_active_at DESC;

-- ============================================================================
-- QUERY 16: Prekey Exhaustion Alert
-- Identifies users whose One-Time Prekey supply has fallen critically low
-- (fewer than 3 unconsumed OTKs remaining). Triggers prekey replenishment.
-- ============================================================================
SELECT
    u.user_id,
    u.username,
    COUNT(pb.prekey_id) AS available_otk_count
FROM users u
LEFT JOIN prekey_bundles pb ON u.user_id = pb.user_id
                           AND pb.key_type = 'ONE_TIME_PREKEY'
                           AND pb.is_consumed = 0
GROUP BY u.user_id, u.username
HAVING COUNT(pb.prekey_id) < 3
ORDER BY available_otk_count ASC;

-- ============================================================================
-- SECTION E: TRANSACTION MANAGEMENT
-- ============================================================================

-- ============================================================================
-- TRANSACTION T1: Atomic Message Send with Outbox Enqueue
-- Guarantees that a sent message is always paired with an outbox envelope.
-- If either INSERT fails, the entire transaction is rolled back — preventing
-- the "ghost message" anomaly (message shown as sent but never queued).
-- ============================================================================
BEGIN TRANSACTION;

INSERT INTO messages (message_id, conversation_id, sender_address, recipient_address,
                      ciphertext, nonce, ratchet_counter, delivery_status, sent_at)
VALUES ('msg_ab_07', 'conv_ab', 'xchat_alice_1a2b3c', 'xchat_bob_3c4d5e',
        X'4E45574D4553534147455F414207', 'nonce_ab_07', 4, 'PENDING', CURRENT_TIMESTAMP);

INSERT INTO outbox_queue (envelope_id, conversation_id, destination_address,
                          transport_type, raw_payload, status, next_retry_at)
VALUES ('env_ab_07', 'conv_ab', 'xchat_bob_3c4d5e',
        'TCP_LAN', X'574952455F414207', 'QUEUED', CURRENT_TIMESTAMP);

COMMIT;

-- ============================================================================
-- TRANSACTION T2: Atomic Double Ratchet State Advance
-- Updates the ratchet state machine and purges expired skipped keys in a
-- single atomic transaction. Prevents ratchet desynchronization anomaly:
-- if the state update succeeds but cleanup fails (or vice versa), the
-- transaction is rolled back to the previous consistent ratchet state.
-- ============================================================================
BEGIN TRANSACTION;

-- Step 1: Advance ratchet counters and rotate DH keys
UPDATE ratchet_sessions
SET peer_dh_ratchet_pub  = 'bob_eph_dh_pub_ratchet_r8',
    root_key_hash        = 'rk_hash_ab_step_8',
    send_chain_key_hash  = 'cks_hash_ab_step_8',
    recv_chain_key_hash  = 'ckr_hash_ab_step_8',
    send_counter         = 8,
    recv_counter         = 6
WHERE conversation_id = 'conv_ab';

-- Step 2: Insert skipped key for out-of-order message gap
INSERT OR IGNORE INTO skipped_keys (session_id, dh_ratchet_pub, message_counter, message_key, expires_at)
VALUES ('ses_ab', 'bob_eph_dh_pub_ratchet_r7', 6, 'mk_ab_r7_n6_encrypted',
        datetime(CURRENT_TIMESTAMP, '+14 days'));

-- Step 3: Prune expired skipped keys for this session
DELETE FROM skipped_keys
WHERE session_id = 'ses_ab'
  AND expires_at <= CURRENT_TIMESTAMP;

COMMIT;

-- ============================================================================
-- TRANSACTION T3: Atomic Contact Verification Update
-- When a user verifies a peer's fingerprint out-of-band (QR code scan or
-- voice confirmation), the trust status is atomically upgraded. This prevents
-- the race condition where an adversary could slip a packet in between a
-- trust-level read and write.
-- ============================================================================
BEGIN TRANSACTION;

UPDATE contacts
SET trust_status = 'VERIFIED',
    updated_at   = CURRENT_TIMESTAMP
WHERE user_id    = 'usr_bob'
  AND peer_address = 'xchat_unkn_000000'
  AND trust_status = 'UNVERIFIED';

-- Verify the update succeeded (rowcount > 0 means contact was found and updated)
SELECT changes() AS rows_updated;

COMMIT;

-- ============================================================================
-- TRANSACTION T4: Atomic Outbox Acknowledgement & Cleanup
-- When a P2P delivery ACK is received, atomically:
--   (1) Mark the message as DELIVERED
--   (2) Mark the outbox envelope as ACKNOWLEDGED
--   (3) Update the conversation's last_active_at timestamp
-- ============================================================================
BEGIN TRANSACTION;

UPDATE messages
SET delivery_status = 'DELIVERED',
    delivered_at    = CURRENT_TIMESTAMP
WHERE message_id = 'msg_ab_07'
  AND delivery_status IN ('PENDING', 'SENT');

UPDATE outbox_queue
SET status = 'ACKNOWLEDGED'
WHERE envelope_id = 'env_ab_07'
  AND status = 'IN_FLIGHT';

UPDATE conversations
SET last_active_at = CURRENT_TIMESTAMP
WHERE conversation_id = 'conv_ab';

COMMIT;

-- ============================================================================
-- TRANSACTION T5: Safe Prekey Consumption (X3DH One-Time Prekey Burn)
-- Atomically marks a One-Time Prekey as consumed during X3DH session
-- initiation. Prevents the double-spend anomaly where the same OTK could
-- be used for two simultaneous session initiations (catastrophic for forward
-- secrecy — two parties would share the same session key material).
-- ============================================================================
BEGIN TRANSACTION;

UPDATE prekey_bundles
SET is_consumed = 1
WHERE prekey_id = 'pre_alice_otk1'
  AND is_consumed = 0;    -- Atomic guard: only consume if still available

SELECT changes() AS otk_consumed;  -- Must be 1; if 0, OTK was already consumed

COMMIT;

-- ============================================================================
-- SECTION F: VIEWS
-- Reusable virtual relation definitions for the application layer
-- ============================================================================

-- ============================================================================
-- VIEW 1: v_active_conversations
-- A denormalized view exposing the full context needed by the sidebar UI:
-- conversation metadata, peer info, and unread count in one flat relation.
-- ============================================================================
CREATE VIEW IF NOT EXISTS v_active_conversations AS
SELECT
    c.conversation_id,
    c.user_id,
    c.conv_type,
    c.title,
    c.is_muted,
    c.last_active_at,
    ct.peer_name,
    ct.peer_address,
    ct.reachable_socket,
    ct.trust_status,
    ct.fingerprint,
    COUNT(CASE WHEN m.delivery_status IN ('PENDING','SENT','DELIVERED')
               AND m.sender_address = ct.peer_address THEN 1 END) AS unread_count
FROM conversations c
JOIN contacts ct    ON c.contact_id = ct.contact_id
LEFT JOIN messages m ON c.conversation_id = m.conversation_id
GROUP BY c.conversation_id, c.user_id, c.conv_type, c.title, c.is_muted,
         c.last_active_at, ct.peer_name, ct.peer_address,
         ct.reachable_socket, ct.trust_status, ct.fingerprint;

-- Usage: fetch Alice's active conversation feed
SELECT * FROM v_active_conversations
WHERE user_id = 'usr_alice'
ORDER BY last_active_at DESC;

-- ============================================================================
-- VIEW 2: v_ratchet_session_health
-- Exposes ratchet session health diagnostics across all sessions — surfaces
-- desynchronized or high-skipped-key-count sessions to the security dashboard.
-- ============================================================================
CREATE VIEW IF NOT EXISTS v_ratchet_session_health AS
SELECT
    rs.session_id,
    rs.conversation_id,
    ct.peer_name,
    u.username,
    rs.send_counter,
    rs.recv_counter,
    (rs.send_counter - rs.recv_counter)  AS delta,
    COUNT(sk.session_id)                  AS skipped_key_count,
    rs.last_ratchet_at,
    CASE
        WHEN COUNT(sk.session_id) > 100               THEN 'CRITICAL'
        WHEN COUNT(sk.session_id) > 20                THEN 'WARNING'
        WHEN ABS(rs.send_counter - rs.recv_counter) > 50 THEN 'WARNING'
        ELSE 'HEALTHY'
    END AS health
FROM ratchet_sessions rs
JOIN conversations c   ON rs.conversation_id = c.conversation_id
JOIN users u           ON c.user_id = u.user_id
JOIN contacts ct       ON c.contact_id = ct.contact_id
LEFT JOIN skipped_keys sk ON rs.session_id = sk.session_id
GROUP BY rs.session_id, rs.conversation_id, ct.peer_name, u.username,
         rs.send_counter, rs.recv_counter, rs.last_ratchet_at;

-- Usage: surface all non-healthy sessions
SELECT * FROM v_ratchet_session_health
WHERE health != 'HEALTHY';

-- ============================================================================
-- VIEW 3: v_outbox_pending_dispatch
-- Ready-to-dispatch outbox queue: QUEUED envelopes with retry times in the
-- past, joined with peer socket telemetry for the transport dispatcher.
-- ============================================================================
CREATE VIEW IF NOT EXISTS v_outbox_pending_dispatch AS
SELECT
    oq.envelope_id,
    oq.transport_type,
    oq.destination_address,
    oq.raw_payload,
    oq.retry_count,
    oq.next_retry_at,
    ct.reachable_socket,
    ct.trust_status
FROM outbox_queue oq
JOIN conversations c ON oq.conversation_id = c.conversation_id
JOIN contacts ct     ON c.contact_id = ct.contact_id
WHERE oq.status = 'QUEUED'
  AND oq.next_retry_at <= CURRENT_TIMESTAMP
  AND ct.trust_status != 'BLOCKED';

-- Usage: get the next batch of envelopes for the BLE mesh dispatcher
SELECT * FROM v_outbox_pending_dispatch
WHERE transport_type = 'BLE_MESH'
ORDER BY retry_count ASC
LIMIT 10;

-- ============================================================================
-- VIEW 4: v_prekey_inventory
-- Reports available One-Time Prekey counts per user, flagging users whose
-- OTK supply is critically low and need to replenish their prekey bundle.
-- ============================================================================
CREATE VIEW IF NOT EXISTS v_prekey_inventory AS
SELECT
    u.user_id,
    u.username,
    SUM(CASE WHEN pb.key_type = 'SIGNED_PREKEY'   AND pb.is_consumed = 0 THEN 1 ELSE 0 END) AS signed_prekeys_available,
    SUM(CASE WHEN pb.key_type = 'ONE_TIME_PREKEY' AND pb.is_consumed = 0 THEN 1 ELSE 0 END) AS otk_available,
    SUM(CASE WHEN pb.key_type = 'ONE_TIME_PREKEY' AND pb.is_consumed = 1 THEN 1 ELSE 0 END) AS otk_consumed,
    CASE
        WHEN SUM(CASE WHEN pb.key_type = 'ONE_TIME_PREKEY' AND pb.is_consumed = 0 THEN 1 ELSE 0 END) = 0 THEN 'CRITICAL: NO OTK'
        WHEN SUM(CASE WHEN pb.key_type = 'ONE_TIME_PREKEY' AND pb.is_consumed = 0 THEN 1 ELSE 0 END) < 3 THEN 'WARNING: LOW OTK'
        ELSE 'OK'
    END AS prekey_status
FROM users u
LEFT JOIN prekey_bundles pb ON u.user_id = pb.user_id
GROUP BY u.user_id, u.username;

-- Usage: check OTK status for all nodes
SELECT * FROM v_prekey_inventory ORDER BY otk_available ASC;

-- ============================================================================
-- SECTION G: EXECUTION PLAN ANALYSIS (EXPLAIN QUERY PLAN)
-- Demonstrates index utilization for key queries
-- ============================================================================

-- Plan G1: Conversation feed — confirm idx_conversations_user_active is used
EXPLAIN QUERY PLAN
SELECT c.conversation_id, c.last_active_at, ct.peer_name
FROM conversations c
JOIN contacts ct ON c.contact_id = ct.contact_id
WHERE c.user_id = 'usr_alice'
ORDER BY c.last_active_at DESC;

-- Plan G2: Message thread pagination — confirm idx_messages_conversation_sent_at
EXPLAIN QUERY PLAN
SELECT message_id, sent_at, delivery_status
FROM messages
WHERE conversation_id = 'conv_ab'
ORDER BY sent_at DESC
LIMIT 20;

-- Plan G3: Outbox dispatcher — confirm idx_outbox_queue_priority
EXPLAIN QUERY PLAN
SELECT envelope_id, transport_type, destination_address
FROM outbox_queue
WHERE status = 'QUEUED'
  AND next_retry_at <= CURRENT_TIMESTAMP
ORDER BY retry_count ASC;

-- Plan G4: Ratchet session lookup — confirm idx_ratchet_sessions_conversation
EXPLAIN QUERY PLAN
SELECT session_id, send_counter, recv_counter
FROM ratchet_sessions
WHERE conversation_id = 'conv_ab';

-- Plan G5: Relay cache TTL scan — confirm idx_relay_cache_ttl
EXPLAIN QUERY PLAN
SELECT route_token FROM relay_cache
WHERE expires_at <= CURRENT_TIMESTAMP;

-- ============================================================================
-- SECTION H: INTEGRITY VERIFICATION
-- Post-query correctness assertions
-- ============================================================================

-- H1: Verify all messages have exactly one parent conversation
SELECT 'PASS: All messages have valid conversation' AS assertion
WHERE NOT EXISTS (
    SELECT 1 FROM messages m
    LEFT JOIN conversations c ON m.conversation_id = c.conversation_id
    WHERE c.conversation_id IS NULL
);

-- H2: Verify no active ratchet sessions with 0 send_counter AND 0 recv_counter
-- (every active session must have exchanged at least one message)
SELECT
    CASE WHEN COUNT(*) = 0 THEN 'PASS: No idle zero-counter sessions with messages'
    ELSE 'WARN: ' || COUNT(*) || ' zero-counter sessions with messages exist'
    END AS assertion
FROM ratchet_sessions rs
JOIN messages m ON rs.conversation_id = m.conversation_id
WHERE rs.send_counter = 0 AND rs.recv_counter = 0;

-- H3: Verify no outbox envelopes exceed their max_retries
SELECT 'PASS: No outbox envelope exceeds max_retries' AS assertion
WHERE NOT EXISTS (
    SELECT 1 FROM outbox_queue
    WHERE retry_count > max_retries
);

-- H4: Verify referential integrity across all foreign key chains
PRAGMA foreign_key_check;

-- H5: Verify only one ratchet session exists per conversation (1:1 constraint)
SELECT
    CASE WHEN COUNT(*) = 0 THEN 'PASS: Strict 1:1 ratchet session per conversation'
    ELSE 'FAIL: ' || COUNT(*) || ' conversations have duplicate ratchet sessions'
    END AS assertion
FROM (
    SELECT conversation_id, COUNT(*) AS cnt
    FROM ratchet_sessions
    GROUP BY conversation_id
    HAVING cnt > 1
);

-- H6: Verify SIGNED_PREKEY signatures are never NULL
SELECT
    CASE WHEN COUNT(*) = 0 THEN 'PASS: All signed prekeys have non-null signatures'
    ELSE 'FAIL: ' || COUNT(*) || ' signed prekeys missing signature'
    END AS assertion
FROM prekey_bundles
WHERE key_type = 'SIGNED_PREKEY' AND (signature IS NULL OR signature = '');
