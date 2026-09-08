#!/usr/bin/env bash
# Hermetic acceptance checks; no server, credentials or DB.
set -euo pipefail
cd "$(dirname "$0")"
fixture='[{"results":[{"spell_acquisition":{"expected_spell":133,"learned_on_instance":true,"verified_at_login":false,"saved_spell":true,"money_before":100,"saved_money":90,"expected_saved_money":90}}]},{"results":[{"spell_acquisition":{"expected_spell":133,"learned_on_instance":false,"verified_at_login":true,"saved_spell":true,"money_before":90,"saved_money":90,"expected_saved_money":90}}]}]'
fixture="$(jq 'map(.results[0].spell_acquisition += {
  persistence: {kind: "direct_spell"}, persistence_verified: true, observed_skill_root: null
})' <<< "$fixture")"
jq -e -f spell_acquisition_relog.jq <<< "$fixture" > /dev/null
for mutation in \
  '.[0].results[0].spell_acquisition = null' \
  '.[0].results[0].spell_acquisition.learned_on_instance = false' \
  '.[1].results[0].spell_acquisition.verified_at_login = false' \
  '.[0].results[0].spell_acquisition.saved_spell = false' \
  '.[1].results[0].spell_acquisition.expected_spell = 134' \
  '.[1].results[0].spell_acquisition.saved_money = 80' \
  '.[1].results[0].spell_acquisition.expected_saved_money = 80' \
  '.[0].results[0].spell_acquisition.persistence_verified = false' \
  'del(.[1].results[0].spell_acquisition.persistence)' \
  '.[0].results[0].spell_acquisition.persistence.kind = "unknown"'; do
  changed="$(jq "$mutation" <<< "$fixture")"
  if jq -e -f spell_acquisition_relog.jq <<< "$changed" > /dev/null 2>&1; then
    echo "accepted invalid acquisition evidence: $mutation" >&2
    exit 1
  fi
done
skill_fixture="$(jq 'map(.results[0].spell_acquisition |= (. + {
  expected_spell: 674, saved_spell: false,
  persistence: {kind: "skill", id: 118, value: 1, max: 1},
  observed_skill_root: {id: 118, value: 1, max: 1}
}))' <<< "$fixture")"
jq -e -f spell_acquisition_relog.jq <<< "$skill_fixture" > /dev/null
for mutation in \
  '.[0].results[0].spell_acquisition.observed_skill_root = null' \
  '.[1].results[0].spell_acquisition.observed_skill_root.value = 0' \
  '.[1].results[0].spell_acquisition.observed_skill_root.max = 2' \
  '.[1].results[0].spell_acquisition.observed_skill_root.id = 119' \
  '.[1].results[0].spell_acquisition |= (.persistence.id = 119 | .observed_skill_root.id = 119)' \
  '.[1].results[0].spell_acquisition.verified_at_login = false' \
  '.[0].results[0].spell_acquisition.saved_spell = true' \
  '.[1].results[0].spell_acquisition.persistence_verified = false' \
  '.[0].results[0].spell_acquisition.persistence.value = "1"'; do
  changed="$(jq "$mutation" <<< "$skill_fixture")"
  if jq -e -f spell_acquisition_relog.jq <<< "$changed" > /dev/null 2>&1; then
    echo "accepted invalid skill acquisition evidence: $mutation" >&2
    exit 1
  fi
done
for plan in \
  '{"action":"trainer","expected_spell":6197}' \
  '{"action":"cast","expected_spell":674,"persistence":{"kind":"skill","id":118,"value":1,"max":1}}'; do
  verify="$(jq -e -f spell_acquisition_verify_plan.jq <<< "$plan")"
  jq -e --argjson original "$plan" '. == {
    action: "verify", expected_spell: $original.expected_spell,
    persistence: ($original.persistence // {kind: "direct_spell"})
  }' <<< "$verify" > /dev/null
done
echo 'acquisition report acceptance: PASS (direct + skill persistence, negative cases, plan propagation)'
