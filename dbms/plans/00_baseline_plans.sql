-- ============================================================================
-- CP4: Baseline Execution Plans (PRE-OPTIMIZATION)
-- Captured BEFORE any index additions or query rewrites.
-- These plans reveal which queries hit indexes and which do full table scans.
-- ============================================================================
-- NOTE: Run against: dbms/03_schema.sql (schema) + dbms/04_queries.sql (seed data)
-- ============================================================================

-- PLAN 1: Active Conversation Feed (Q1A)
-- Expected: idx_conversations_user_active + autoindex on contacts
EXPLAIN QUERY PLAN
SELECT c.conversation_id, c.title, c.last_active_at, ct.peer_name, ct.trust_status
FROM conversations c
JOIN contacts ct ON c.contact_id = ct.contact_id
LEFT JOIN messages m ON c.conversation_id = m.conversation_id
WHERE c.user_id = 'usr_alice'
GROUP BY c.conversation_id
ORDER BY c.last_active_at DESC;

-- PLAN 2: Unread Badge Counter (Q2)
-- Expected: idx_messages_delivery_status
EXPLAIN QUERY PLAN
SELECT COUNT(m.message_id) AS total_unread
FROM messages m
JOIN conversations c ON m.conversation_id = c.conversation_id
JOIN contacts ct ON c.contact_id = ct.contact_id
WHERE c.user_id = 'usr_alice'
  AND m.delivery_status IN ('PENDING', 'SENT', 'DELIVERED')
  AND m.sender_address = ct.peer_address;

-- PLAN 3: Message Thread Pagination (Q3A)
-- Expected: idx_messages_conversation_sent_at
EXPLAIN QUERY PLAN
SELECT message_id, sender_address, ciphertext, delivery_status, sent_at
FROM messages
WHERE conversation_id = 'conv_ab'
ORDER BY sent_at DESC
LIMIT 20;

-- PLAN 4: Double Ratchet State Machine Lookup (Q4)
-- Expected: autoindex on ratchet_sessions.conversation_id (UNIQUE)
EXPLAIN QUERY PLAN
SELECT session_id, send_counter, recv_counter, root_key_hash
FROM ratchet_sessions
WHERE conversation_id = 'conv_ab';

-- PLAN 5: Outbox Dispatcher (Q5)
-- Expected: idx_outbox_queue_priority
EXPLAIN QUERY PLAN
SELECT envelope_id, transport_type, destination_address, raw_payload
FROM outbox_queue
WHERE status = 'QUEUED'
  AND next_retry_at <= CURRENT_TIMESTAMP
ORDER BY retry_count ASC
LIMIT 50;

-- PLAN 6: Peer Contact Discovery (Q7)
-- Expected: idx_contacts_user_peer
EXPLAIN QUERY PLAN
SELECT contact_id, peer_name, public_key, trust_status
FROM contacts
WHERE user_id = 'usr_alice'
  AND peer_address = 'xchat_bob_3c4d5e';

-- PLAN 7: Relay Cache TTL Garbage Collection (Q8B)
-- Expected: idx_relay_cache_ttl
EXPLAIN QUERY PLAN
SELECT route_token FROM relay_cache
WHERE expires_at <= CURRENT_TIMESTAMP;

-- PLAN 8: Full Session Diagnostic Join (Q15)
-- WARNING: This is a 7-way join — expect multiple index seeks
EXPLAIN QUERY PLAN
SELECT u.username, ct.peer_name, rs.send_counter, rs.recv_counter, COUNT(sk.session_id)
FROM users u
JOIN contacts ct ON u.user_id = ct.user_id
JOIN conversations c ON ct.contact_id = c.contact_id AND u.user_id = c.user_id
LEFT JOIN ratchet_sessions rs ON c.conversation_id = rs.conversation_id
LEFT JOIN skipped_keys sk ON rs.session_id = sk.session_id
LEFT JOIN messages m ON c.conversation_id = m.conversation_id
WHERE u.user_id = 'usr_alice'
GROUP BY u.username, ct.peer_name, rs.send_counter, rs.recv_counter;

-- PLAN 9: Skipped Keys Lookup (Q4 subquery)
-- Expected: idx_skipped_keys_lookup (session_id, expires_at)
-- BUG: This plan actually needs (session_id, dh_ratchet_pub, message_counter) — wrong index!
EXPLAIN QUERY PLAN
SELECT session_id, dh_ratchet_pub, message_counter, message_key
FROM skipped_keys
WHERE session_id = 'ses_ab'
  AND dh_ratchet_pub = 'bob_eph_dh_pub_ratchet_r5'
  AND message_counter = 3;

-- PLAN 10: Prekey Bundle Exhaustion (Q16)
EXPLAIN QUERY PLAN
SELECT u.user_id, COUNT(pb.prekey_id) AS available_otk_count
FROM users u
LEFT JOIN prekey_bundles pb ON u.user_id = pb.user_id
                           AND pb.key_type = 'ONE_TIME_PREKEY'
                           AND pb.is_consumed = 0
GROUP BY u.user_id
HAVING COUNT(pb.prekey_id) < 3;
