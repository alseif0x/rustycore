#!/usr/bin/env bash
# Bounded #587 acquisition followed by a fresh authenticated retention check.
# No fixture preparation, SQL writes, service control or credential provisioning.
set -euo pipefail
set +x
umask 077
cd "$(dirname "$0")"
report="${WOW_BOT_REPORT:?guarded report path required}"
log="${WOW_BOT_LOG:?guarded log path required}"
plan="${WOW_BOT_ACQUISITION_PLAN:?typed acquisition plan required}"
[[ "$plan" = /* && "$report" = /* && "$log" = /* ]]
[[ "${WOW_BOT_ENSURE_TEST_ACCOUNTS:-}" == 0 && "${WOW_BOT_GENERATE_LOCAL_PASSWORD:-}" == 0 ]]
[[ "${WOW_BOT_ACCOUNT:-TESTBOT1@bot.local}" == TESTBOT1@bot.local ]]
export WOW_BOT_ACCOUNT=TESTBOT1@bot.local WOW_BOT_LOGIN_SAVE_CHECK=1
relog_plan="$(mktemp /tmp/rustycore-acquisition-relog.XXXXXX)"
trap 'rm -f -- "$relog_plan"' EXIT
jq -e 'if (.action == "trainer" or .action == "cast") and
  (.expected_spell | type == "number" and . > 0 and . == floor)
  then {action: "verify", expected_spell: .expected_spell}
  else error("requires a typed trainer or cast acquisition plan") end' "$plan" > "$relog_plan"
WOW_BOT_REPORT="$report.first.json" WOW_BOT_LOG="$log.first.log" \
  ./run_rustycore_login_smoke.sh
WOW_BOT_ACQUISITION_PLAN="$relog_plan" \
  WOW_BOT_REPORT="$report.second.json" WOW_BOT_LOG="$log.second.log" \
  ./run_rustycore_login_smoke.sh
# Preserve the existing six-family retention/identity/logout acceptance in full.
jq -e -s -f login_save_relog.jq "$report.first.json" "$report.second.json" > "$report"
jq -e -s -f spell_acquisition_relog.jq "$report.first.json" "$report.second.json" \
  > "$report.acquisition.json"
