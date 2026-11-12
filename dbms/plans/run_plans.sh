#!/usr/bin/env bash
# ==============================================================================
# Execution Plan Runner for CP4: xChat DBMS Optimization Analysis
# Runs baseline, benchmark, and optimized plans against test SQLite DB
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DBMS_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
DB_FILE="/tmp/xchat_cp4_eval.db"

rm -f "${DB_FILE}"
echo "[*] Initializing SQLite database from schema..."
sqlite3 "${DB_FILE}" < "${DBMS_DIR}/03_schema.sql"

echo "[*] Populating seed data and views from 04_queries.sql..."
sqlite3 "${DB_FILE}" < "${DBMS_DIR}/04_queries.sql" > /dev/null 2>&1

echo "[*] Generating Baseline Execution Plans (01_baseline_output.txt)..."
sqlite3 "${DB_FILE}" < "${SCRIPT_DIR}/00_baseline_plans.sql" > "${SCRIPT_DIR}/01_baseline_output.txt" 2>&1

echo "[+] Done! Captured baseline plans in ${SCRIPT_DIR}/01_baseline_output.txt"
