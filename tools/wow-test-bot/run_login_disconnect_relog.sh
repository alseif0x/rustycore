#!/usr/bin/env bash
# Guarded #585 EOF/save/relogin scenario; no SQL fixture setup or cleanup.
set -euo pipefail
set +x
cd "$(dirname "$0")"
report="${WOW_BOT_REPORT:?guarded report path required}"
log="${WOW_BOT_LOG:?guarded log path required}"
[[ "${WOW_BOT_ENSURE_TEST_ACCOUNTS:-}" == 0 && "${WOW_BOT_GENERATE_LOCAL_PASSWORD:-}" == 0 ]]
[[ "${WOW_BOT_ACCOUNT:-TESTBOT1@bot.local}" == TESTBOT1@bot.local ]]
export WOW_BOT_ACCOUNT=TESTBOT1@bot.local
WOW_BOT_LOGIN_SAVE_CHECK=0 WOW_BOT_LOGIN_DISCONNECT_CHECK=1 \
  WOW_BOT_REPORT="$report.first.json" WOW_BOT_LOG="$log.first.log" \
  ./run_rustycore_login_smoke.sh
WOW_BOT_LOGIN_SAVE_CHECK=1 WOW_BOT_LOGIN_DISCONNECT_CHECK=0 \
  WOW_BOT_REPORT="$report.second.json" WOW_BOT_LOG="$log.second.log" \
  ./run_rustycore_login_smoke.sh
jq -e -s -f login_disconnect_relog.jq "$report.first.json" "$report.second.json" > "$report"
