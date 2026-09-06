#!/usr/bin/env bash
# Run only under the authorized qa-runtime.sh login swap. This adds no fixtures.
set -euo pipefail
set +x
cd "$(dirname "$0")"
report="${WOW_BOT_REPORT:?guarded report path required}"
log="${WOW_BOT_LOG:?guarded log path required}"
[[ "${WOW_BOT_ENSURE_TEST_ACCOUNTS:-}" == 0 && "${WOW_BOT_GENERATE_LOCAL_PASSWORD:-}" == 0 ]]
[[ "${WOW_BOT_ACCOUNT:-TESTBOT1@bot.local}" == TESTBOT1@bot.local ]]
export WOW_BOT_ACCOUNT=TESTBOT1@bot.local WOW_BOT_LOGIN_SAVE_CHECK=1
for phase in first second; do
  WOW_BOT_REPORT="$report.$phase.json" WOW_BOT_LOG="$log.$phase.log" \
    ./run_rustycore_login_smoke.sh
done
jq -e -s -f login_save_relog.jq "$report.first.json" "$report.second.json" > "$report"
