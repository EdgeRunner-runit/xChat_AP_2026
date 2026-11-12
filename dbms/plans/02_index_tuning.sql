-- ============================================================================
-- CP4: Index Tuning Script (Optimized & Refined Iteration)
-- Eliminates temporary B-trees, drops redundant indexes, and tunes column orders
-- ============================================================================

-- 1. DROP REDUNDANT & FLAWED INDEXES FROM EARLIER DRAFT
-- Drop redundant index that duplicated PRIMARY KEY on users
DROP INDEX IF EXISTS idx_users_id_redundant;

-- Drop flawed index where range column (next_retry_at) preceded equality check (status)
DROP INDEX IF EXISTS idx_outbox_queue_dispatch_v2;

-- 2. PARTIAL COVERING INDEX FOR OUTBOX PRIORITY DISPATCHER (Query 5)
-- Eliminates 'USE TEMP B-TREE FOR ORDER BY' by storing ready-to-dispatch rows
-- pre-sorted by retry urgency (retry_count ASC, created_at ASC)
CREATE INDEX IF NOT EXISTS idx_outbox_ready_dispatch 
    ON outbox_queue(retry_count ASC, created_at ASC, next_retry_at) 
    WHERE status = 'QUEUED';

-- 3. COVERING INDEX FOR CONVERSATION ACTIVE FEED (Query 1A & View 1)
-- Eliminates table lookups by storing all projection columns in the B-tree leaf nodes
CREATE INDEX IF NOT EXISTS idx_conversations_active_feed_covering 
    ON conversations(user_id, last_active_at DESC, conversation_id, contact_id, title, conv_type, is_muted);

-- 4. COVERING INDEX FOR UNREAD MESSAGE BADGE (Query 2)
-- Enables zero-table-access index-only scan to compute unread counts per thread
CREATE INDEX IF NOT EXISTS idx_messages_unread_covering 
    ON messages(conversation_id, sender_address, delivery_status);

-- 5. PARTIAL INDEX FOR UNCONSUMED ONE-TIME PREKEYS (Query 6 & Query 16)
-- Drastically shrinks index size by indexing ONLY active, unconsumed prekeys
CREATE INDEX IF NOT EXISTS idx_prekeys_active_otk 
    ON prekey_bundles(user_id, key_id_num) 
    WHERE key_type = 'ONE_TIME_PREKEY' AND is_consumed = 0;

-- 6. AUDIT TABLE STATISTICS FOR QUERY PLANNER OPTIMIZATION
ANALYZE;
