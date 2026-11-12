#!/usr/bin/env bash
# ==============================================================================
# Performance Benchmark & Query Plan Evaluator for CP4
# Compares Baseline vs. Optimized queries across synthetic scaled workloads
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DBMS_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
BENCH_DB="/tmp/xchat_cp4_bench.db"
OUTPUT_FILE="${SCRIPT_DIR}/04_benchmark_results.txt"

rm -f "${BENCH_DB}"
echo "=================================================================" | tee "${OUTPUT_FILE}"
echo "    xChat Storage Engine — CP4 Query Optimization Benchmarks    " | tee -a "${OUTPUT_FILE}"
echo "=================================================================" | tee -a "${OUTPUT_FILE}"
echo "Timestamp: $(date -u +"%Y-%m-%dT%H:%M:%SZ")" | tee -a "${OUTPUT_FILE}"
echo "Database: SQLite $(sqlite3 --version | cut -d' ' -f1,2)" | tee -a "${OUTPUT_FILE}"
echo "" | tee -a "${OUTPUT_FILE}"

echo "[*] Step 1: Initializing Base Schema..." | tee -a "${OUTPUT_FILE}"
sqlite3 "${BENCH_DB}" < "${DBMS_DIR}/03_schema.sql"

echo "[*] Step 2: Populating Scaled Synthetic Dataset (2,500 messages, 50 threads)..." | tee -a "${OUTPUT_FILE}"
sqlite3 "${BENCH_DB}" << 'EOF'
BEGIN TRANSACTION;
-- Insert Alice (distinct master fingerprint from seed usr_01)
INSERT INTO users (user_id, username, master_fingerprint) VALUES
    ('usr_alice', 'Alice_LocalNode', 'FAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFA');

WITH RECURSIVE cnt(x) AS (
    SELECT 1
    UNION ALL
    SELECT x + 1 FROM cnt WHERE x < 50
)
INSERT INTO contacts (contact_id, user_id, peer_address, peer_name, reachable_socket, public_key, fingerprint, trust_status)
SELECT 
    'cnt_' || printf('%03d', x),
    'usr_alice',
    'xchat_peer_' || printf('%03d', x),
    'Peer_' || printf('%03d', x),
    '192.168.1.' || (x + 10) || ':9400',
    'pubkey_' || printf('%03d', x),
    'FP_' || printf('%03d', x),
    CASE WHEN x % 5 = 0 THEN 'UNVERIFIED' ELSE 'VERIFIED' END
FROM cnt;

-- 50 Conversation threads
WITH RECURSIVE cnt(x) AS (
    SELECT 1
    UNION ALL
    SELECT x + 1 FROM cnt WHERE x < 50
)
INSERT INTO conversations (conversation_id, user_id, contact_id, conv_type, title, last_active_at)
SELECT
    'conv_' || printf('%03d', x),
    'usr_alice',
    'cnt_' || printf('%03d', x),
    'DIRECT',
    'Thread with Peer ' || printf('%03d', x),
    datetime('2026-11-10 12:00:00', '+' || (x * 60) || ' seconds')
FROM cnt;

-- 2,500 Messages across threads (50 messages per conversation)
WITH RECURSIVE
  c(conv) AS (SELECT 1 UNION ALL SELECT conv + 1 FROM c WHERE conv < 50),
  m(msg)  AS (SELECT 1 UNION ALL SELECT msg + 1 FROM m WHERE msg < 50)
INSERT INTO messages (message_id, conversation_id, sender_address, recipient_address, ciphertext, nonce, ratchet_counter, delivery_status, sent_at)
SELECT
    'msg_' || printf('%03d', c.conv) || '_' || printf('%03d', m.msg),
    'conv_' || printf('%03d', c.conv),
    CASE WHEN m.msg % 2 = 0 THEN 'usr_alice' ELSE 'xchat_peer_' || printf('%03d', c.conv) END,
    CASE WHEN m.msg % 2 = 0 THEN 'xchat_peer_' || printf('%03d', c.conv) ELSE 'usr_alice' END,
    X'AABBCCDDEEFF001122334455',
    'nonce_' || m.msg,
    m.msg,
    CASE 
        WHEN m.msg = 50 THEN 'PENDING'
        WHEN m.msg = 49 THEN 'DELIVERED'
        ELSE 'READ'
    END,
    datetime('2026-11-10 12:00:00', '+' || ((c.conv * 50) + m.msg) || ' seconds')
FROM c CROSS JOIN m;

-- 300 Outbox Queue Envelopes
WITH RECURSIVE o(x) AS (
    SELECT 1 UNION ALL SELECT x + 1 FROM o WHERE x < 300
)
INSERT INTO outbox_queue (envelope_id, conversation_id, destination_address, transport_type, raw_payload, retry_count, max_retries, next_retry_at, status)
SELECT
    'env_' || printf('%04d', x),
    'conv_' || printf('%03d', ((x % 50) + 1)),
    'xchat_peer_' || printf('%03d', ((x % 50) + 1)),
    CASE x % 4
        WHEN 0 THEN 'TCP_LAN'
        WHEN 1 THEN 'BLE_MESH'
        WHEN 2 THEN 'SPHINX_MIXNET'
        ELSE 'SMS_MODEM'
    END,
    X'5041594C4F41445F454E56454C4F5045',
    x % 5,
    5,
    datetime('now', '-' || (x * 10) || ' seconds'),
    CASE WHEN x % 3 = 0 THEN 'QUEUED' WHEN x % 3 = 1 THEN 'IN_FLIGHT' ELSE 'ACKNOWLEDGED' END
FROM o;
COMMIT;
EOF

echo "[*] Step 3: Benchmarking Baseline Queries (Without Tuning Indexes)..." | tee -a "${OUTPUT_FILE}"

# Benchmark Function: runs query in a loop and calculates avg ms
benchmark_query() {
    local label="$1"
    local sql="$2"
    local iterations=50
    local start_ns=$(python3 -c "import time; print(int(time.time() * 1000000))")
    for ((i=1; i<=iterations; i++)); do
        sqlite3 "${BENCH_DB}" "${sql}" > /dev/null 2>&1
    done
    local end_ns=$(python3 -c "import time; print(int(time.time() * 1000000))")
    local elapsed_us=$((end_ns - start_ns))
    local avg_ms=$(python3 -c "print(round(${elapsed_us} / (${iterations} * 1000.0), 3))")
    printf "  %-40s : %8.3f ms\n" "${label}" "${avg_ms}" | tee -a "${OUTPUT_FILE}"
}

echo "--- Baseline Latencies (Pre-Optimization) ---" | tee -a "${OUTPUT_FILE}"
Q1_BASELINE="SELECT c.conversation_id, c.title, c.last_active_at, ct.peer_name, COUNT(CASE WHEN m.delivery_status IN ('PENDING','SENT','DELIVERED') AND m.sender_address = ct.peer_address THEN 1 END) AS unread_count FROM conversations c JOIN contacts ct ON c.contact_id = ct.contact_id LEFT JOIN messages m ON c.conversation_id = m.conversation_id WHERE c.user_id = 'usr_alice' GROUP BY c.conversation_id ORDER BY c.last_active_at DESC;"
Q2_BASELINE="SELECT COUNT(m.message_id) FROM messages m JOIN conversations c ON m.conversation_id = c.conversation_id JOIN contacts ct ON c.contact_id = ct.contact_id WHERE c.user_id = 'usr_alice' AND m.delivery_status IN ('PENDING','SENT','DELIVERED') AND m.sender_address = ct.peer_address;"
Q3_BASELINE="SELECT message_id, sender_address, ciphertext, sent_at FROM messages WHERE conversation_id = 'conv_025' ORDER BY sent_at DESC LIMIT 20;"
Q5_BASELINE="SELECT envelope_id, transport_type, destination_address, raw_payload FROM outbox_queue WHERE status = 'QUEUED' AND next_retry_at <= CURRENT_TIMESTAMP ORDER BY retry_count ASC, created_at ASC LIMIT 50;"

benchmark_query "Q1: Conversation Feed (Baseline)" "${Q1_BASELINE}"
benchmark_query "Q2: Unread Badge Counter (Baseline)" "${Q2_BASELINE}"
benchmark_query "Q3: Message Thread History (Baseline)" "${Q3_BASELINE}"
benchmark_query "Q5: Outbox Dispatcher (Baseline)" "${Q5_BASELINE}"

echo "" | tee -a "${OUTPUT_FILE}"
echo "[*] Step 4: Applying CP4 Index Tuning & Statistics Optimization..." | tee -a "${OUTPUT_FILE}"
sqlite3 "${BENCH_DB}" < "${SCRIPT_DIR}/02_index_tuning.sql"

echo "" | tee -a "${OUTPUT_FILE}"
echo "--- Optimized Latencies (Post-Optimization) ---" | tee -a "${OUTPUT_FILE}"
Q1_OPTIMIZED="WITH unread_agg AS (SELECT conversation_id, COUNT(*) AS unread_count FROM messages WHERE delivery_status IN ('PENDING','SENT','DELIVERED') GROUP BY conversation_id) SELECT c.conversation_id, c.title, c.last_active_at, ct.peer_name, COALESCE(ua.unread_count, 0) FROM conversations c JOIN contacts ct ON c.contact_id = ct.contact_id LEFT JOIN unread_agg ua ON c.conversation_id = ua.conversation_id WHERE c.user_id = 'usr_alice' ORDER BY c.last_active_at DESC;"
Q2_OPTIMIZED="SELECT COUNT(m.message_id) FROM messages m JOIN conversations c ON m.conversation_id = c.conversation_id JOIN contacts ct ON c.contact_id = ct.contact_id WHERE c.user_id = 'usr_alice' AND m.delivery_status IN ('PENDING','SENT','DELIVERED') AND m.sender_address = ct.peer_address;"
Q3_OPTIMIZED="SELECT message_id, sender_address, ciphertext, sent_at FROM messages WHERE conversation_id = 'conv_025' ORDER BY sent_at DESC LIMIT 20;"
Q5_OPTIMIZED="SELECT envelope_id, transport_type, destination_address, raw_payload FROM outbox_queue INDEXED BY idx_outbox_ready_dispatch WHERE status = 'QUEUED' AND next_retry_at <= CURRENT_TIMESTAMP ORDER BY retry_count ASC, created_at ASC LIMIT 50;"

benchmark_query "Q1: Conversation Feed (Optimized CTE)" "${Q1_OPTIMIZED}"
benchmark_query "Q2: Unread Badge (Covering Index)" "${Q2_OPTIMIZED}"
benchmark_query "Q3: Thread History (Covering Index)" "${Q3_OPTIMIZED}"
benchmark_query "Q5: Outbox Dispatcher (Partial Index)" "${Q5_OPTIMIZED}"

echo "" | tee -a "${OUTPUT_FILE}"
echo "[+] Benchmark suite completed successfully!" | tee -a "${OUTPUT_FILE}"
