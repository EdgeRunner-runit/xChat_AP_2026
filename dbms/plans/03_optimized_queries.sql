-- ============================================================================
-- CP4: Optimized Queries Suite & Execution Plans (POST-OPTIMIZATION)
-- Demonstrates query rewrites, covering index scans, and elimination of temp B-trees
-- ============================================================================

-- ============================================================================
-- OPTIMIZED PLAN 1: Active Conversation Feed (Q1A Rewrite)
-- REWRITE STRATEGY: Separate conversation pagination from message aggregation.
-- Instead of joining all messages across all threads before grouping, compute
-- unread counts per thread via correlated index-only scan or CTE.
-- Index used: idx_conversations_active_feed_covering + idx_messages_unread_covering
-- Result: Eliminates BOTH temporary B-trees (GROUP BY and ORDER BY)!
-- ============================================================================
EXPLAIN QUERY PLAN
WITH unread_agg AS (
    SELECT 
        conversation_id,
        COUNT(*) AS unread_count
    FROM messages
    WHERE delivery_status IN ('PENDING', 'SENT', 'DELIVERED')
    GROUP BY conversation_id
)
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
    COALESCE(ua.unread_count, 0) AS unread_count
FROM conversations c
JOIN contacts ct ON c.contact_id = ct.contact_id
LEFT JOIN unread_agg ua ON c.conversation_id = ua.conversation_id
WHERE c.user_id = 'usr_alice'
ORDER BY c.last_active_at DESC;

-- ============================================================================
-- OPTIMIZED PLAN 2: Unread Badge Counter (Q2 Covering Index)
-- REWRITE STRATEGY: Leverage idx_messages_unread_covering to compute unread count
-- without scanning message ciphertext, nonce, or payloads from disk blocks.
-- ============================================================================
EXPLAIN QUERY PLAN
SELECT
    c.user_id,
    COUNT(m.message_id) AS total_unread
FROM messages m
JOIN conversations c ON m.conversation_id = c.conversation_id
JOIN contacts ct ON c.contact_id = ct.contact_id
WHERE c.user_id = 'usr_alice'
  AND m.delivery_status IN ('PENDING', 'SENT', 'DELIVERED')
  AND m.sender_address = ct.peer_address;

-- ============================================================================
-- OPTIMIZED PLAN 3: Message Thread Pagination (Q3 Cursor Pagination)
-- Index used: idx_messages_conversation_sent_at (Covering B-tree)
-- ============================================================================
EXPLAIN QUERY PLAN
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

-- ============================================================================
-- OPTIMIZED PLAN 4: Double Ratchet State Point Lookup (Q4)
-- Index used: sqlite_autoindex_ratchet_sessions_2 (UNIQUE on conversation_id)
-- ============================================================================
EXPLAIN QUERY PLAN
SELECT
    rs.session_id,
    rs.peer_dh_ratchet_pub,
    rs.local_dh_ratchet_pub,
    rs.root_key_hash,
    rs.send_chain_key_hash,
    rs.recv_chain_key_hash,
    rs.send_counter,
    rs.recv_counter,
    rs.last_ratchet_at
FROM ratchet_sessions rs
WHERE rs.conversation_id = 'conv_ab';

-- ============================================================================
-- OPTIMIZED PLAN 5: Outbox Priority Dispatcher (Q5 Partial Index Scan)
-- REWRITE STRATEGY: Uses partial index idx_outbox_ready_dispatch
-- Eliminates 'USE TEMP B-TREE FOR ORDER BY' — rows are yielded in exact
-- sort order directly from the leaf nodes!
-- ============================================================================
EXPLAIN QUERY PLAN
SELECT
    envelope_id,
    transport_type,
    destination_address,
    raw_payload
FROM outbox_queue INDEXED BY idx_outbox_ready_dispatch
WHERE status = 'QUEUED'
  AND next_retry_at <= CURRENT_TIMESTAMP
ORDER BY retry_count ASC, created_at ASC
LIMIT 50;

-- ============================================================================
-- OPTIMIZED PLAN 6: Active Prekey Inventory Scan (Q16 Partial Index)
-- REWRITE STRATEGY: Uses partial index idx_prekeys_active_otk
-- Scans only available one-time prekeys instead of whole prekey pool
-- ============================================================================
EXPLAIN QUERY PLAN
SELECT
    u.user_id,
    u.username,
    COUNT(pb.prekey_id) AS available_otk_count
FROM users u
LEFT JOIN prekey_bundles pb INDEXED BY idx_prekeys_active_otk
    ON u.user_id = pb.user_id
   AND pb.key_type = 'ONE_TIME_PREKEY'
   AND pb.is_consumed = 0
GROUP BY u.user_id, u.username
HAVING COUNT(pb.prekey_id) < 3;
