#!/usr/bin/env bash
# Hermetic acceptance checks; no server, credentials or DB.
set -euo pipefail
cd "$(dirname "$0")"
fixture='[{"results":[{"spell_acquisition":{"expected_spell":133,"learned_on_instance":true,"verified_at_login":false,"saved_spell":true,"money_before":100,"saved_money":90,"expected_saved_money":90}}]},{"results":[{"spell_acquisition":{"expected_spell":133,"learned_on_instance":false,"verified_at_login":true,"saved_spell":true,"money_before":90,"saved_money":90,"expected_saved_money":90}}]}]'
jq -e -f spell_acquisition_relog.jq <<< "$fixture" > /dev/null
for mutation in \
  '.[0].results[0].spell_acquisition = null' \
  '.[0].results[0].spell_acquisition.learned_on_instance = false' \
  '.[1].results[0].spell_acquisition.verified_at_login = false' \
  '.[0].results[0].spell_acquisition.saved_spell = false' \
  '.[1].results[0].spell_acquisition.expected_spell = 134' \
  '.[1].results[0].spell_acquisition.saved_money = 80'; do
  changed="$(jq "$mutation" <<< "$fixture")"
  if jq -e -f spell_acquisition_relog.jq <<< "$changed" > /dev/null 2>&1; then
    echo "accepted invalid acquisition evidence: $mutation" >&2
    exit 1
  fi
done
echo 'acquisition report acceptance: PASS (positive + 6 negative cases)'
