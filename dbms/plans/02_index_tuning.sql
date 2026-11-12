-- ============================================================================
-- CP4: Index Tuning Script (Draft Iteration 1)
-- Target: Eliminate temporary B-trees and optimize critical query paths
-- ============================================================================

-- 1. Redundant index on users primary key (to be audited)
CREATE INDEX IF NOT EXISTS idx_users_id_redundant 
    ON users(user_id);

-- 2. Covering index for Unread Message Badge (Query 2)
CREATE INDEX IF NOT EXISTS idx_messages_unread_covering 
    ON messages(conversation_id, sender_address, delivery_status);

-- 3. Outbox priority dispatcher index (Draft 1: range column first - SUBOPTIMAL)
-- Intended for:
--   WHERE status = 'QUEUED' AND next_retry_at <= CURRENT_TIMESTAMP
--   ORDER BY retry_count ASC, created_at ASC
-- Suboptimal ordering: next_retry_at placed ahead of status equality check
CREATE INDEX IF NOT EXISTS idx_outbox_queue_dispatch_v2 
    ON outbox_queue(next_retry_at, status, retry_count);

-- 4. Partial index for unconsumed prekeys (Query 6 and Query 16)
CREATE INDEX IF NOT EXISTS idx_prekeys_active_otk 
    ON prekey_bundles(user_id, key_id_num) 
    WHERE key_type = 'ONE_TIME_PREKEY' AND is_consumed = 0;
