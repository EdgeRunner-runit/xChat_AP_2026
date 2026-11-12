#!/usr/bin/env python3
"""
Comprehensive Query Timing Benchmark Suite for CP4
Measures exact wall-clock latencies (Mean, Min, P50, P90, P99, Max, StdDev)
across all CP3 queries, views, and transactions in Baseline vs. Optimized modes.
"""
import sqlite3
import time
import statistics
import os
import json

DB_FILE = "/tmp/xchat_timing_eval.db"

def init_db(apply_tuning=False):
    if os.path.exists(DB_FILE):
        os.remove(DB_FILE)
    
    conn = sqlite3.connect(DB_FILE)
    cur = conn.cursor()
    
    # 1. Base Schema
    with open("xChat_AP_2026/dbms/03_schema.sql") as f:
        cur.executescript(f.read())
        
    # 2. Scaled Data: 50 contacts, 50 threads, 2500 messages, 300 outbox items
    cur.execute("""
    INSERT INTO users (user_id, username, master_fingerprint) VALUES
        ('usr_alice', 'Alice_LocalNode', 'FAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFAFA');
    """)
    
    cur.executescript("""
    WITH RECURSIVE cnt(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM cnt WHERE x < 50)
    INSERT INTO contacts (contact_id, user_id, peer_address, peer_name, reachable_socket, public_key, fingerprint, trust_status)
    SELECT 'cnt_' || printf('%03d', x), 'usr_alice', 'xchat_peer_' || printf('%03d', x), 'Peer_' || printf('%03d', x),
           '192.168.1.' || (x + 10) || ':9400', 'pubkey_' || printf('%03d', x), 'FP_' || printf('%03d', x),
           CASE WHEN x % 5 = 0 THEN 'UNVERIFIED' ELSE 'VERIFIED' END FROM cnt;

    WITH RECURSIVE cnt(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM cnt WHERE x < 50)
    INSERT INTO conversations (conversation_id, user_id, contact_id, conv_type, title, last_active_at)
    SELECT 'conv_' || printf('%03d', x), 'usr_alice', 'cnt_' || printf('%03d', x), 'DIRECT', 'Thread with Peer ' || printf('%03d', x),
           datetime('2026-11-10 12:00:00', '+' || (x * 60) || ' seconds') FROM cnt;

    WITH RECURSIVE
      c(conv) AS (SELECT 1 UNION ALL SELECT conv + 1 FROM c WHERE conv < 50),
      m(msg)  AS (SELECT 1 UNION ALL SELECT msg + 1 FROM m WHERE msg < 50)
    INSERT INTO messages (message_id, conversation_id, sender_address, recipient_address, ciphertext, nonce, ratchet_counter, delivery_status, sent_at, delivered_at)
    SELECT 'msg_' || printf('%03d', c.conv) || '_' || printf('%03d', m.msg), 'conv_' || printf('%03d', c.conv),
           CASE WHEN m.msg % 2 = 0 THEN 'usr_alice' ELSE 'xchat_peer_' || printf('%03d', c.conv) END,
           CASE WHEN m.msg % 2 = 0 THEN 'xchat_peer_' || printf('%03d', c.conv) ELSE 'usr_alice' END,
           X'AABBCCDDEEFF001122334455', 'nonce_' || m.msg, m.msg,
           CASE WHEN m.msg = 50 THEN 'PENDING' WHEN m.msg = 49 THEN 'DELIVERED' ELSE 'READ' END,
           datetime('2026-11-10 12:00:00', '+' || ((c.conv * 50) + m.msg) || ' seconds'),
           CASE WHEN m.msg < 49 THEN datetime('2026-11-10 12:00:05', '+' || ((c.conv * 50) + m.msg) || ' seconds') ELSE NULL END
    FROM c CROSS JOIN m;

    WITH RECURSIVE o(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM o WHERE x < 300)
    INSERT INTO outbox_queue (envelope_id, conversation_id, destination_address, transport_type, raw_payload, retry_count, max_retries, next_retry_at, status)
    SELECT 'env_' || printf('%04d', x), 'conv_' || printf('%03d', ((x % 50) + 1)), 'xchat_peer_' || printf('%03d', ((x % 50) + 1)),
           CASE x % 4 WHEN 0 THEN 'TCP_LAN' WHEN 1 THEN 'BLE_MESH' WHEN 2 THEN 'SPHINX_MIXNET' ELSE 'SMS_MODEM' END,
           X'5041594C4F41445F454E56454C4F5045', x % 5, 5, datetime('now', '-' || (x * 10) || ' seconds'),
           CASE WHEN x % 3 = 0 THEN 'QUEUED' WHEN x % 3 = 1 THEN 'IN_FLIGHT' ELSE 'ACKNOWLEDGED' END
    FROM o;

    WITH RECURSIVE r(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM r WHERE x < 100)
    INSERT INTO relay_cache (route_token, opaque_envelope, hop_count, ttl_seconds, received_at, expires_at)
    SELECT 'tok_' || printf('%04d', x), X'52454C41595F5041594C4F4144', (x % 5) + 1, 3600,
           datetime('now', '-' || (x * 30) || ' seconds'),
           datetime('now', '+' || ((100 - x) * 30) || ' seconds')
    FROM r;
    
    INSERT INTO ratchet_sessions (session_id, conversation_id, peer_dh_ratchet_pub, local_dh_ratchet_pub, root_key_hash, send_chain_key_hash, recv_chain_key_hash, send_counter, recv_counter)
    SELECT 'ses_' || printf('%03d', x), 'conv_' || printf('%03d', x), 'peer_dh_' || x, 'local_dh_' || x, 'rk_' || x, 'cks_' || x, 'ckr_' || x, 25, 25
    FROM (WITH RECURSIVE cnt(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM cnt WHERE x < 50) SELECT x FROM cnt);

    WITH RECURSIVE s(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM s WHERE x < 50)
    INSERT INTO skipped_keys (session_id, dh_ratchet_pub, message_counter, message_key, expires_at)
    SELECT 'ses_' || printf('%03d', x), 'peer_dh_' || x, x, 'mk_skipped_' || x, datetime('now', '+14 days')
    FROM s;

    WITH RECURSIVE p(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM p WHERE x < 20)
    INSERT INTO prekey_bundles (prekey_id, user_id, key_id_num, key_type, public_key, signature, is_consumed)
    SELECT 'otk_' || printf('%03d', x), 'usr_alice', x, 'ONE_TIME_PREKEY', 'otk_pub_' || x, NULL, CASE WHEN x < 15 THEN 1 ELSE 0 END
    FROM p;
    """)
    conn.commit()

    # Views from 04_queries.sql
    cur.executescript("""
    CREATE VIEW IF NOT EXISTS v_active_conversations AS
    SELECT c.conversation_id, c.user_id, c.conv_type, c.title, c.is_muted, c.last_active_at,
           ct.peer_name, ct.peer_address, ct.reachable_socket, ct.trust_status, ct.fingerprint,
           COUNT(CASE WHEN m.delivery_status IN ('PENDING','SENT','DELIVERED') AND m.sender_address = ct.peer_address THEN 1 END) AS unread_count
    FROM conversations c JOIN contacts ct ON c.contact_id = ct.contact_id
    LEFT JOIN messages m ON c.conversation_id = m.conversation_id
    GROUP BY c.conversation_id, c.user_id, c.conv_type, c.title, c.is_muted, c.last_active_at,
             ct.peer_name, ct.peer_address, ct.reachable_socket, ct.trust_status, ct.fingerprint;

    CREATE VIEW IF NOT EXISTS v_ratchet_session_health AS
    SELECT rs.session_id, rs.conversation_id, ct.peer_name, u.username, rs.send_counter, rs.recv_counter,
           (rs.send_counter - rs.recv_counter) AS delta, COUNT(sk.session_id) AS skipped_key_count, rs.last_ratchet_at,
           CASE WHEN COUNT(sk.session_id) > 100 THEN 'CRITICAL' WHEN COUNT(sk.session_id) > 20 THEN 'WARNING'
                WHEN ABS(rs.send_counter - rs.recv_counter) > 50 THEN 'WARNING' ELSE 'HEALTHY' END AS health
    FROM ratchet_sessions rs JOIN conversations c ON rs.conversation_id = c.conversation_id
    JOIN users u ON c.user_id = u.user_id JOIN contacts ct ON c.contact_id = ct.contact_id
    LEFT JOIN skipped_keys sk ON rs.session_id = sk.session_id
    GROUP BY rs.session_id, rs.conversation_id, ct.peer_name, u.username, rs.send_counter, rs.recv_counter, rs.last_ratchet_at;

    CREATE VIEW IF NOT EXISTS v_outbox_pending_dispatch AS
    SELECT oq.envelope_id, oq.transport_type, oq.destination_address, oq.raw_payload, oq.retry_count, oq.next_retry_at,
           ct.reachable_socket, ct.trust_status
    FROM outbox_queue oq JOIN conversations c ON oq.conversation_id = c.conversation_id
    JOIN contacts ct ON c.contact_id = ct.contact_id
    WHERE oq.status = 'QUEUED' AND oq.next_retry_at <= CURRENT_TIMESTAMP AND ct.trust_status != 'BLOCKED';

    CREATE VIEW IF NOT EXISTS v_prekey_inventory AS
    SELECT u.user_id, u.username,
           SUM(CASE WHEN pb.key_type = 'SIGNED_PREKEY' AND pb.is_consumed = 0 THEN 1 ELSE 0 END) AS signed_prekeys_available,
           SUM(CASE WHEN pb.key_type = 'ONE_TIME_PREKEY' AND pb.is_consumed = 0 THEN 1 ELSE 0 END) AS otk_available,
           SUM(CASE WHEN pb.key_type = 'ONE_TIME_PREKEY' AND pb.is_consumed = 1 THEN 1 ELSE 0 END) AS otk_consumed,
           CASE WHEN SUM(CASE WHEN pb.key_type = 'ONE_TIME_PREKEY' AND pb.is_consumed = 0 THEN 1 ELSE 0 END) = 0 THEN 'CRITICAL: NO OTK'
                WHEN SUM(CASE WHEN pb.key_type = 'ONE_TIME_PREKEY' AND pb.is_consumed = 0 THEN 1 ELSE 0 END) < 3 THEN 'WARNING: LOW OTK'
                ELSE 'OK' END AS prekey_status
    FROM users u LEFT JOIN prekey_bundles pb ON u.user_id = pb.user_id
    GROUP BY u.user_id, u.username;
    """)
    conn.commit()

    if apply_tuning:
        with open("xChat_AP_2026/dbms/plans/02_index_tuning.sql") as f:
            cur.executescript(f.read())
        conn.commit()
        
    return conn

# Query suite catalog
QUERIES = [
    ("Q1A", "Active Conversation Feed (Sidebar)",
     "SELECT c.conversation_id, c.title, c.last_active_at, ct.peer_name, COUNT(CASE WHEN m.delivery_status IN ('PENDING','SENT','DELIVERED') AND m.sender_address = ct.peer_address THEN 1 END) AS unread_count FROM conversations c JOIN contacts ct ON c.contact_id = ct.contact_id LEFT JOIN messages m ON c.conversation_id = m.conversation_id WHERE c.user_id = 'usr_alice' GROUP BY c.conversation_id ORDER BY c.last_active_at DESC;",
     "WITH unread_agg AS (SELECT conversation_id, COUNT(*) AS unread_count FROM messages WHERE delivery_status IN ('PENDING','SENT','DELIVERED') GROUP BY conversation_id) SELECT c.conversation_id, c.title, c.conv_type, c.last_active_at, c.is_muted, ct.peer_name, ct.peer_address, ct.reachable_socket, ct.trust_status, COALESCE(ua.unread_count, 0) AS unread_count FROM conversations c JOIN contacts ct ON c.contact_id = ct.contact_id LEFT JOIN unread_agg ua ON c.conversation_id = ua.conversation_id WHERE c.user_id = 'usr_alice' ORDER BY c.last_active_at DESC;"),

    ("Q1B", "Latest Message Preview Snippet",
     "SELECT c.conversation_id, c.title, c.last_active_at, m.sender_address, m.delivery_status, m.sent_at FROM conversations c JOIN messages m ON c.conversation_id = m.conversation_id WHERE c.user_id = 'usr_alice' AND m.sent_at = (SELECT MAX(m2.sent_at) FROM messages m2 WHERE m2.conversation_id = c.conversation_id) ORDER BY c.last_active_at DESC;",
     "SELECT c.conversation_id, c.title, c.last_active_at, m.sender_address, m.delivery_status, m.sent_at FROM conversations c JOIN messages m ON c.conversation_id = m.conversation_id WHERE c.user_id = 'usr_alice' AND m.sent_at = (SELECT MAX(m2.sent_at) FROM messages m2 WHERE m2.conversation_id = c.conversation_id) ORDER BY c.last_active_at DESC;"),

    ("Q2", "Aggregate Unread Message Badge",
     "SELECT COUNT(m.message_id) FROM messages m JOIN conversations c ON m.conversation_id = c.conversation_id JOIN contacts ct ON c.contact_id = ct.contact_id WHERE c.user_id = 'usr_alice' AND m.delivery_status IN ('PENDING','SENT','DELIVERED') AND m.sender_address = ct.peer_address;",
     "SELECT COUNT(m.message_id) FROM messages m JOIN conversations c ON m.conversation_id = c.conversation_id JOIN contacts ct ON c.contact_id = ct.contact_id WHERE c.user_id = 'usr_alice' AND m.delivery_status IN ('PENDING','SENT','DELIVERED') AND m.sender_address = ct.peer_address;"),

    ("Q3A", "Message Thread Cursor Pagination (Page 1)",
     "SELECT message_id, sender_address, ciphertext, nonce, ratchet_counter, delivery_status, sent_at FROM messages WHERE conversation_id = 'conv_025' ORDER BY sent_at DESC LIMIT 20;",
     "SELECT message_id, sender_address, ciphertext, nonce, ratchet_counter, delivery_status, sent_at FROM messages WHERE conversation_id = 'conv_025' ORDER BY sent_at DESC LIMIT 20;"),

    ("Q3B", "Message Thread Cursor Pagination (Page 2)",
     "SELECT message_id, sender_address, ciphertext, delivery_status, sent_at FROM messages WHERE conversation_id = 'conv_025' AND sent_at < datetime('2026-11-10 12:00:00', '+1280 seconds') ORDER BY sent_at DESC LIMIT 20;",
     "SELECT message_id, sender_address, ciphertext, delivery_status, sent_at FROM messages WHERE conversation_id = 'conv_025' AND sent_at < datetime('2026-11-10 12:00:00', '+1280 seconds') ORDER BY sent_at DESC LIMIT 20;"),

    ("Q4A", "Double Ratchet Session Point Lookup",
     "SELECT session_id, peer_dh_ratchet_pub, local_dh_ratchet_pub, root_key_hash, send_counter, recv_counter, last_ratchet_at FROM ratchet_sessions WHERE conversation_id = 'conv_025';",
     "SELECT session_id, peer_dh_ratchet_pub, local_dh_ratchet_pub, root_key_hash, send_counter, recv_counter, last_ratchet_at FROM ratchet_sessions WHERE conversation_id = 'conv_025';"),

    ("Q4B", "Skipped Keys Cache Vault Lookup",
     "SELECT sk.session_id, sk.dh_ratchet_pub, sk.message_counter, sk.message_key FROM skipped_keys sk JOIN ratchet_sessions rs ON sk.session_id = rs.session_id WHERE rs.conversation_id = 'conv_025' AND sk.dh_ratchet_pub = 'peer_dh_25' AND sk.message_counter = 25;",
     "SELECT sk.session_id, sk.dh_ratchet_pub, sk.message_counter, sk.message_key FROM skipped_keys sk JOIN ratchet_sessions rs ON sk.session_id = rs.session_id WHERE rs.conversation_id = 'conv_025' AND sk.dh_ratchet_pub = 'peer_dh_25' AND sk.message_counter = 25;"),

    ("Q5A", "Multi-Transport Outbox Priority Queue",
     "SELECT envelope_id, transport_type, destination_address, raw_payload, retry_count, next_retry_at FROM outbox_queue WHERE status = 'QUEUED' AND next_retry_at <= CURRENT_TIMESTAMP ORDER BY retry_count ASC, created_at ASC LIMIT 50;",
     "SELECT envelope_id, transport_type, destination_address, raw_payload, retry_count, next_retry_at FROM outbox_queue INDEXED BY idx_outbox_ready_dispatch WHERE status = 'QUEUED' AND next_retry_at <= CURRENT_TIMESTAMP ORDER BY retry_count ASC, created_at ASC LIMIT 50;"),

    ("Q5B", "Carrier Queue Status Breakdown",
     "SELECT transport_type, status, COUNT(*) AS envelope_count FROM outbox_queue GROUP BY transport_type, status ORDER BY envelope_count DESC;",
     "SELECT transport_type, status, COUNT(*) AS envelope_count FROM outbox_queue GROUP BY transport_type, status ORDER BY envelope_count DESC;"),

    ("Q6", "X3DH Prekey Bundle Negotiation Fetch",
     "SELECT u.user_id, ik.public_key, spk.public_key, spk.signature, otk.prekey_id, otk.public_key FROM users u JOIN identity_keys ik ON u.user_id = ik.user_id AND ik.key_type = 'ED25519_SIGNING' JOIN prekey_bundles spk ON u.user_id = spk.user_id AND spk.key_type = 'SIGNED_PREKEY' AND spk.is_consumed = 0 LEFT JOIN prekey_bundles otk ON u.user_id = otk.user_id AND otk.key_type = 'ONE_TIME_PREKEY' AND otk.is_consumed = 0 WHERE u.user_id = 'usr_alice' ORDER BY otk.key_id_num ASC LIMIT 1;",
     "SELECT u.user_id, ik.public_key, spk.public_key, spk.signature, otk.prekey_id, otk.public_key FROM users u JOIN identity_keys ik ON u.user_id = ik.user_id AND ik.key_type = 'ED25519_SIGNING' JOIN prekey_bundles spk ON u.user_id = spk.user_id AND spk.key_type = 'SIGNED_PREKEY' AND spk.is_consumed = 0 LEFT JOIN prekey_bundles otk INDEXED BY idx_prekeys_active_otk ON u.user_id = otk.user_id AND otk.key_type = 'ONE_TIME_PREKEY' AND otk.is_consumed = 0 WHERE u.user_id = 'usr_alice' ORDER BY otk.key_id_num ASC LIMIT 1;"),

    ("Q7", "Peer Contact Ingress Security Check",
     "SELECT contact_id, peer_name, peer_address, reachable_socket, public_key, fingerprint, trust_status FROM contacts WHERE user_id = 'usr_alice' AND peer_address = 'xchat_peer_025' AND trust_status != 'BLOCKED';",
     "SELECT contact_id, peer_name, peer_address, reachable_socket, public_key, fingerprint, trust_status FROM contacts WHERE user_id = 'usr_alice' AND peer_address = 'xchat_peer_025' AND trust_status != 'BLOCKED';"),

    ("Q8A", "Relay Cache Mesh Routing Lookup",
     "SELECT route_token, opaque_envelope, hop_count, ttl_seconds FROM relay_cache WHERE route_token = 'tok_0025' AND expires_at > CURRENT_TIMESTAMP;",
     "SELECT route_token, opaque_envelope, hop_count, ttl_seconds FROM relay_cache WHERE route_token = 'tok_0025' AND expires_at > CURRENT_TIMESTAMP;"),

    ("Q8B", "Relay Cache TTL Garbage Collection Scan",
     "SELECT COUNT(route_token) FROM relay_cache WHERE expires_at <= CURRENT_TIMESTAMP;",
     "SELECT COUNT(route_token) FROM relay_cache WHERE expires_at <= CURRENT_TIMESTAMP;"),

    ("Q11", "Per-User Cryptographic Identity Summary",
     "SELECT u.user_id, u.username, COUNT(DISTINCT ik.key_id) AS total_identity_keys, COUNT(DISTINCT ct.contact_id) AS total_contacts, COUNT(DISTINCT cv.conversation_id) AS total_conversations FROM users u LEFT JOIN identity_keys ik ON u.user_id = ik.user_id LEFT JOIN contacts ct ON u.user_id = ct.user_id LEFT JOIN conversations cv ON u.user_id = cv.user_id WHERE u.user_id = 'usr_alice' GROUP BY u.user_id, u.username;",
     "SELECT u.user_id, u.username, COUNT(DISTINCT ik.key_id) AS total_identity_keys, COUNT(DISTINCT ct.contact_id) AS total_contacts, COUNT(DISTINCT cv.conversation_id) AS total_conversations FROM users u LEFT JOIN identity_keys ik ON u.user_id = ik.user_id LEFT JOIN contacts ct ON u.user_id = ct.user_id LEFT JOIN conversations cv ON u.user_id = cv.user_id WHERE u.user_id = 'usr_alice' GROUP BY u.user_id, u.username;"),

    ("Q12", "Message Volume & Delivery Latency Stats",
     "SELECT m.delivery_status, COUNT(*), ROUND(AVG((julianday(m.delivered_at) - julianday(m.sent_at)) * 86400.0), 3) FROM messages m JOIN conversations c ON m.conversation_id = c.conversation_id WHERE c.user_id = 'usr_alice' GROUP BY m.delivery_status;",
     "SELECT m.delivery_status, COUNT(*), ROUND(AVG((julianday(m.delivered_at) - julianday(m.sent_at)) * 86400.0), 3) FROM messages m JOIN conversations c ON m.conversation_id = c.conversation_id WHERE c.user_id = 'usr_alice' GROUP BY m.delivery_status;"),

    ("Q13", "Double Ratchet Health Diagnostics",
     "SELECT rs.session_id, c.title, ct.peer_name, rs.send_counter, rs.recv_counter, COUNT(sk.session_id) FROM ratchet_sessions rs JOIN conversations c ON rs.conversation_id = c.conversation_id JOIN contacts ct ON c.contact_id = ct.contact_id LEFT JOIN skipped_keys sk ON rs.session_id = sk.session_id GROUP BY rs.session_id, c.title, ct.peer_name, rs.send_counter, rs.recv_counter;",
     "SELECT rs.session_id, c.title, ct.peer_name, rs.send_counter, rs.recv_counter, COUNT(sk.session_id) FROM ratchet_sessions rs JOIN conversations c ON rs.conversation_id = c.conversation_id JOIN contacts ct ON c.contact_id = ct.contact_id LEFT JOIN skipped_keys sk ON rs.session_id = sk.session_id GROUP BY rs.session_id, c.title, ct.peer_name, rs.send_counter, rs.recv_counter;"),

    ("Q14", "Transport Carrier Reliability Audit",
     "SELECT oq.transport_type, COUNT(*), COUNT(CASE WHEN oq.status = 'ACKNOWLEDGED' THEN 1 END), COUNT(CASE WHEN oq.status = 'DEAD_LETTER' THEN 1 END) FROM outbox_queue oq GROUP BY oq.transport_type;",
     "SELECT oq.transport_type, COUNT(*), COUNT(CASE WHEN oq.status = 'ACKNOWLEDGED' THEN 1 END), COUNT(CASE WHEN oq.status = 'DEAD_LETTER' THEN 1 END) FROM outbox_queue oq GROUP BY oq.transport_type;"),

    ("Q15", "Full Session Diagnostic 7-Way Join",
     "SELECT u.username, ct.peer_name, c.conv_type, rs.send_counter, rs.recv_counter, COUNT(sk.session_id) AS skipped_keys, COUNT(m.message_id) AS msgs FROM users u JOIN contacts ct ON u.user_id = ct.user_id JOIN conversations c ON ct.contact_id = c.contact_id AND u.user_id = c.user_id LEFT JOIN ratchet_sessions rs ON c.conversation_id = rs.conversation_id LEFT JOIN skipped_keys sk ON rs.session_id = sk.session_id LEFT JOIN messages m ON c.conversation_id = m.conversation_id WHERE u.user_id = 'usr_alice' GROUP BY u.username, ct.peer_name, c.conv_type, rs.send_counter, rs.recv_counter;",
     "SELECT u.username, ct.peer_name, c.conv_type, rs.send_counter, rs.recv_counter, COUNT(sk.session_id) AS skipped_keys, COUNT(m.message_id) AS msgs FROM users u JOIN contacts ct ON u.user_id = ct.user_id JOIN conversations c ON ct.contact_id = c.contact_id AND u.user_id = c.user_id LEFT JOIN ratchet_sessions rs ON c.conversation_id = rs.conversation_id LEFT JOIN skipped_keys sk ON rs.session_id = sk.session_id LEFT JOIN messages m ON c.conversation_id = m.conversation_id WHERE u.user_id = 'usr_alice' GROUP BY u.username, ct.peer_name, c.conv_type, rs.send_counter, rs.recv_counter;"),

    ("Q16", "Prekey Exhaustion Alert",
     "SELECT u.user_id, u.username, COUNT(pb.prekey_id) AS available_otk_count FROM users u LEFT JOIN prekey_bundles pb ON u.user_id = pb.user_id AND pb.key_type = 'ONE_TIME_PREKEY' AND pb.is_consumed = 0 GROUP BY u.user_id, u.username HAVING COUNT(pb.prekey_id) < 3;",
     "SELECT u.user_id, u.username, COUNT(pb.prekey_id) AS available_otk_count FROM users u LEFT JOIN prekey_bundles pb INDEXED BY idx_prekeys_active_otk ON u.user_id = pb.user_id AND pb.key_type = 'ONE_TIME_PREKEY' AND pb.is_consumed = 0 GROUP BY u.user_id, u.username HAVING COUNT(pb.prekey_id) < 3;"),

    ("V1", "View 1: v_active_conversations",
     "SELECT * FROM v_active_conversations WHERE user_id = 'usr_alice' ORDER BY last_active_at DESC;",
     "SELECT * FROM v_active_conversations WHERE user_id = 'usr_alice' ORDER BY last_active_at DESC;"),

    ("V2", "View 2: v_ratchet_session_health",
     "SELECT * FROM v_ratchet_session_health WHERE health != 'HEALTHY';",
     "SELECT * FROM v_ratchet_session_health WHERE health != 'HEALTHY';"),

    ("V3", "View 3: v_outbox_pending_dispatch",
     "SELECT * FROM v_outbox_pending_dispatch WHERE transport_type = 'BLE_MESH' LIMIT 10;",
     "SELECT * FROM v_outbox_pending_dispatch WHERE transport_type = 'BLE_MESH' LIMIT 10;"),

    ("V4", "View 4: v_prekey_inventory",
     "SELECT * FROM v_prekey_inventory ORDER BY otk_available ASC;",
     "SELECT * FROM v_prekey_inventory ORDER BY otk_available ASC;")
]

def benchmark_sql(conn, sql, iterations=100):
    cur = conn.cursor()
    times = []
    for _ in range(iterations):
        t0 = time.perf_counter()
        cur.execute(sql)
        cur.fetchall()
        t1 = time.perf_counter()
        times.append((t1 - t0) * 1000.0) # in ms
    
    times.sort()
    mean_val = statistics.mean(times)
    min_val = min(times)
    p50_val = statistics.median(times)
    p90_val = times[int(iterations * 0.90)]
    p99_val = times[int(iterations * 0.99)]
    max_val = max(times)
    std_val = statistics.stdev(times) if len(times) > 1 else 0.0
    return {
        "mean": mean_val, "min": min_val, "p50": p50_val,
        "p90": p90_val, "p99": p99_val, "max": max_val, "std": std_val
    }

def main():
    print("[*] Running Baseline Benchmarks (without CP4 tuning)...")
    base_conn = init_db(apply_tuning=False)
    base_results = {}
    for qid, desc, sql_base, _ in QUERIES:
        base_results[qid] = benchmark_sql(base_conn, sql_base, 100)
    base_conn.close()

    print("[*] Running Optimized Benchmarks (with CP4 tuning)...")
    opt_conn = init_db(apply_tuning=True)
    opt_results = {}
    for qid, desc, _, sql_opt in QUERIES:
        opt_results[qid] = benchmark_sql(opt_conn, sql_opt, 100)
    opt_conn.close()

    output_data = []
    print("\n" + "="*80)
    print(f"{'ID':<6} {'Query Description':<38} {'Base (ms)':<10} {'Opt (ms)':<10} {'Speedup':<8}")
    print("="*80)
    for qid, desc, _, _ in QUERIES:
        b = base_results[qid]
        o = opt_results[qid]
        speedup = ((b['mean'] - o['mean']) / b['mean']) * 100.0
        print(f"{qid:<6} {desc[:36]:<38} {b['mean']:>8.3f} ms {o['mean']:>8.3f} ms {speedup:>+6.1f}%")
        output_data.append({
            "id": qid,
            "description": desc,
            "baseline": b,
            "optimized": o,
            "speedup_pct": speedup
        })
    print("="*80)

    with open("xChat_AP_2026/dbms/plans/benchmark_summary.json", "w") as f:
        json.dump(output_data, f, indent=2)
    print("[+] Saved raw metrics to xChat_AP_2026/dbms/plans/benchmark_summary.json")

if __name__ == "__main__":
    main()
