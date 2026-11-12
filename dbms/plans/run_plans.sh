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
echo "[*] Step 1: Initializing SQLite database from schema..."
sqlite3 "${DB_FILE}" < "${DBMS_DIR}/03_schema.sql"

echo "[*] Step 2: Populating seed data and views from 04_queries.sql..."
sqlite3 "${DB_FILE}" < "${DBMS_DIR}/04_queries.sql" > /dev/null 2>&1

echo "[*] Step 3: Generating Baseline Execution Plans (01_baseline_output.txt)..."
sqlite3 "${DB_FILE}" < "${SCRIPT_DIR}/00_baseline_plans.sql" > "${SCRIPT_DIR}/01_baseline_output.txt" 2>&1

echo "[*] Step 4: Applying CP4 Index Tuning (02_index_tuning.sql)..."
sqlite3 "${DB_FILE}" < "${SCRIPT_DIR}/02_index_tuning.sql"

echo "[*] Step 5: Generating Optimized Execution Plans (03_optimized_output.txt)..."
sqlite3 "${DB_FILE}" < "${SCRIPT_DIR}/03_optimized_queries.sql" > "${SCRIPT_DIR}/03_optimized_output.txt" 2>&1

echo "[*] Step 6: Running Automated Performance Benchmark..."
"${SCRIPT_DIR}/04_benchmark.sh" > /dev/null 2>&1

echo "[+] Done! All CP4 plans and benchmark files successfully generated:"
echo "    - ${SCRIPT_DIR}/01_baseline_output.txt"
echo "    - ${SCRIPT_DIR}/03_optimized_output.txt"
echo "    - ${SCRIPT_DIR}/04_benchmark_results.txt"
