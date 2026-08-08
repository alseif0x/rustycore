#!/usr/bin/env bash
# Static/filesystem-only regression coverage for the issue-#26 fixture guard.
# It never invokes MySQL, PM2, or either world server.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
SCRIPT_ROOT="$REPO_ROOT/crates/capture-diff/scripts"
TEST_ROOT="$(mktemp -d)"
trap 'rm -rf -- "$TEST_ROOT"' EXIT
chmod 700 "$TEST_ROOT"

LOOT_FIXTURE_DB_CONF=""
WOW_BOT_FIXTURE_JOURNAL=""
LOOT_FIXTURE_CLEANUP_MARKER=""

# shellcheck source=../scripts/loot-fixture-common.sh
source "$SCRIPT_ROOT/loot-fixture-common.sh"
# shellcheck source=../scripts/creature-spell-casting-fixture-common.sh
source "$SCRIPT_ROOT/creature-spell-casting-fixture-common.sh"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_eq() {
  local expected="$1"
  local actual="$2"
  local message="$3"
  [ "$actual" = "$expected" ] \
    || fail "${message}: expected '${expected}', got '${actual}'"
}

line_of() {
  local pattern="$1"
  local path="$2"
  local line
  line="$(grep -n -m1 -F -- "$pattern" "$path" | cut -d: -f1)" \
    || fail "missing '${pattern}' in ${path}"
  printf '%s\n' "$line"
}

repeat_digest() {
  local character="$1"
  printf '%064s' '' | tr ' ' "$character"
}

creature_spell_fixture_validate_committed_fixture "$REPO_ROOT"
assert_eq creature-spell-casting \
  "$(jq -r '.flow' "$CREATURE_SPELL_FIXTURE_MANIFEST")" \
  "fixture manifest flow"
assert_eq \
  fe6cea1808e8beb7d648d285ad52b10067611e46c55d30637514508275b63b49 \
  "$CREATURE_SPELL_FIXTURE_MANIFEST_SHA256" \
  "fixture manifest digest"
assert_eq \
  ef8b3c29f46fe537e1ae4e826b5610afcd534999f900ec9554ee0534e7847262 \
  "$(creature_spell_fixture_sha256_of_file \
    "$REPO_ROOT/crates/capture-diff/flows/creature-spell-casting/fixture/cpp-reference.patch")" \
  "reviewed C++ source patch digest"

VALID_BOT_REPORT="$TEST_ROOT/valid-bot-report.json"
MUTATED_BOT_REPORT="$TEST_ROOT/mutated-bot-report.json"
jq -n \
  --arg manifest_sha "$CREATURE_SPELL_FIXTURE_MANIFEST_SHA256" \
  --arg digest "$(repeat_digest a)" '
  {
    creature_spell_capture: true,
    detour_chase_capture: false,
    loot_item_capture: false,
    loot_race_smoke: false,
    results: [{
      account: "TESTBOT2@bot.local",
      account_id: 9,
      character_guid: 15,
      world_auth: true,
      enum_characters: true,
      player_login_verified: true,
      creature_spell_capture: true,
      creature_spell_capture_passed: true,
      creature_spell_fixture_manifest_sha256: $manifest_sha,
      creature_spell_target_entry: 22378,
      creature_spell_target_spawn_guid: 78686,
      creature_spell_target_runtime_counter: 78686,
      creature_spell_target_discovered: true,
      creature_spell_heartbeat_sent: true,
      creature_spell_heartbeat_sha256: $digest,
      creature_spell_start_opcode: 11319,
      creature_spell_start_body_sha256: $digest,
      creature_spell_start_body_bytes: 100,
      creature_spell_go_opcode: 11318,
      creature_spell_go_body_sha256: $digest,
      creature_spell_go_body_bytes: 101,
      creature_spell_cast_id_low: 1,
      creature_spell_cast_id_high: 1,
      creature_spell_caster_guid_low: 78686,
      creature_spell_caster_guid_high: 1,
      creature_spell_victim_guid_low: 15,
      creature_spell_victim_guid_high: 1,
      creature_spell_spell_id: 15691,
      creature_spell_start_cast_flags: 2,
      creature_spell_go_cast_flags: 256,
      creature_spell_cast_flags_ex: 0,
      creature_spell_go_hit_target_count: 1,
      creature_spell_go_miss_target_count: 0,
      creature_spell_full_combat_log: false,
      creature_spell_advanced_logging_sent: false,
      creature_spell_adjacent_start_go: true,
      creature_spell_disconnect_confirmed: true,
      creature_spell_logout_confirmed: false,
      creature_spell_failure: null
    }]
  }' >"$VALID_BOT_REPORT"
creature_spell_fixture_report_proves_exact_success "$VALID_BOT_REPORT" \
  || fail "strict report validator rejected disconnect-without-logout evidence"
jq '.results[0].creature_spell_disconnect_confirmed = false' \
  "$VALID_BOT_REPORT" >"$MUTATED_BOT_REPORT"
if creature_spell_fixture_report_proves_exact_success \
    "$MUTATED_BOT_REPORT"; then
  fail "strict report validator accepted a missing disconnect proof"
fi
jq '.results[0].creature_spell_logout_confirmed = true' \
  "$VALID_BOT_REPORT" >"$MUTATED_BOT_REPORT"
if creature_spell_fixture_report_proves_exact_success \
    "$MUTATED_BOT_REPORT"; then
  fail "strict report validator accepted a combat logout"
fi

# Exercise the source-derivation validator against a fresh local Git graph so
# the test remains independent of any installed C++ checkout or service.
DERIVATION_HARNESS="$TEST_ROOT/source-derivation-harness"
DERIVATION_SOURCE="$TEST_ROOT/source-derivation-repo"
DERIVATION_PATCH_REL=fixture/cpp-reference.patch
DERIVATION_PATCH="$DERIVATION_HARNESS/$DERIVATION_PATCH_REL"
DERIVATION_FIXTURE="$DERIVATION_HARNESS/fixture.json"
DERIVATION_REMOTE_URL=https://example.invalid/legacy-reference.git
DERIVATION_REMOTE_REF=refs/remotes/origin/3.4.3
DERIVATION_CHANGED_PATH=src/server/game/DataStores/DB2Stores.cpp
mkdir -p "$DERIVATION_HARNESS/fixture" \
  "$DERIVATION_SOURCE/src/server/game/DataStores"
git -C "$DERIVATION_SOURCE" init -q
git -C "$DERIVATION_SOURCE" config user.name 'Capture Fixture Test'
git -C "$DERIVATION_SOURCE" config user.email capture-fixture@example.invalid
git -C "$DERIVATION_SOURCE" remote add origin "$DERIVATION_REMOTE_URL"
printf 'base\n' >"$DERIVATION_SOURCE/$DERIVATION_CHANGED_PATH"
git -C "$DERIVATION_SOURCE" add "$DERIVATION_CHANGED_PATH"
git -C "$DERIVATION_SOURCE" commit -q -m base
DERIVATION_BASE_HEAD="$(git -C "$DERIVATION_SOURCE" rev-parse HEAD)"
DERIVATION_BASE_TREE="$(git -C "$DERIVATION_SOURCE" rev-parse 'HEAD^{tree}')"
git -C "$DERIVATION_SOURCE" update-ref \
  "$DERIVATION_REMOTE_REF" "$DERIVATION_BASE_HEAD"
printf 'base\nreviewed\n' >"$DERIVATION_SOURCE/$DERIVATION_CHANGED_PATH"
git -C "$DERIVATION_SOURCE" add "$DERIVATION_CHANGED_PATH"
git -C "$DERIVATION_SOURCE" commit -q -m reviewed
DERIVATION_PATCHED_HEAD="$(git -C "$DERIVATION_SOURCE" rev-parse HEAD)"
DERIVATION_PATCHED_TREE="$(git -C "$DERIVATION_SOURCE" rev-parse 'HEAD^{tree}')"
git -C "$DERIVATION_SOURCE" diff \
  --binary --full-index --no-ext-diff --no-textconv --no-renames --no-color \
  --src-prefix=a/ --dst-prefix=b/ --diff-algorithm=myers \
  "$DERIVATION_BASE_HEAD" "$DERIVATION_PATCHED_HEAD" -- \
  >"$DERIVATION_PATCH"
DERIVATION_PATCH_SHA="$(creature_spell_fixture_sha256_of_file \
  "$DERIVATION_PATCH")"
jq -n \
  --arg contract "$CREATURE_SPELL_FIXTURE_SOURCE_DERIVATION_CONTRACT" \
  --arg remote_url "$DERIVATION_REMOTE_URL" \
  --arg remote_ref "$DERIVATION_REMOTE_REF" \
  --arg base_head "$DERIVATION_BASE_HEAD" \
  --arg base_tree "$DERIVATION_BASE_TREE" \
  --arg patched_head "$DERIVATION_PATCHED_HEAD" \
  --arg patched_tree "$DERIVATION_PATCHED_TREE" \
  --arg patch_path "$DERIVATION_PATCH_REL" \
  --arg patch_sha "$DERIVATION_PATCH_SHA" \
  --arg changed_path "$DERIVATION_CHANGED_PATH" '
    {source_derivation: {
      contract: $contract,
      remote_url: $remote_url,
      remote_ref: $remote_ref,
      base_head: $base_head,
      base_tree: $base_tree,
      patched_head: $patched_head,
      patched_tree: $patched_tree,
      patch_path: $patch_path,
      patch_sha256: $patch_sha,
      changed_paths: [$changed_path]
    }}
  ' >"$DERIVATION_FIXTURE"

creature_spell_fixture_validate_cpp_source_derivation \
  "$DERIVATION_HARNESS" "$DERIVATION_SOURCE" "$DERIVATION_FIXTURE"
assert_eq "$DERIVATION_PATCHED_HEAD" \
  "$(jq -r '.patched_head' \
    <<<"$CREATURE_SPELL_FIXTURE_SOURCE_DERIVATION_JSON")" \
  "validated patched source HEAD"

if GIT_DIR="$DERIVATION_SOURCE/.git" \
    creature_spell_fixture_validate_cpp_source_derivation \
      "$DERIVATION_HARNESS" "$DERIVATION_SOURCE" "$DERIVATION_FIXTURE" \
      2>/dev/null; then
  fail "source derivation accepted a GIT_DIR repository redirect"
fi
if GIT_CONFIG_COUNT=1 \
    GIT_CONFIG_KEY_0=core.abbrev \
    GIT_CONFIG_VALUE_0=12 \
    creature_spell_fixture_validate_cpp_source_derivation \
      "$DERIVATION_HARNESS" "$DERIVATION_SOURCE" "$DERIVATION_FIXTURE" \
      2>/dev/null; then
  fail "source derivation accepted injected Git configuration"
fi
if GIT_CONFIG="$DERIVATION_SOURCE/.git/config" \
    creature_spell_fixture_validate_cpp_source_derivation \
      "$DERIVATION_HARNESS" "$DERIVATION_SOURCE" "$DERIVATION_FIXTURE" \
      2>/dev/null; then
  fail "source derivation accepted a GIT_CONFIG file redirect"
fi

DERIVATION_MUTATED="$DERIVATION_HARNESS/mutated.json"
git -C "$DERIVATION_SOURCE" remote set-url origin \
  https://example.invalid/unreviewed.git
if creature_spell_fixture_validate_cpp_source_derivation \
    "$DERIVATION_HARNESS" "$DERIVATION_SOURCE" "$DERIVATION_FIXTURE" \
    2>/dev/null; then
  fail "source derivation accepted a different origin URL"
fi
git -C "$DERIVATION_SOURCE" remote set-url origin "$DERIVATION_REMOTE_URL"

git -C "$DERIVATION_SOURCE" update-ref \
  "$DERIVATION_REMOTE_REF" "$DERIVATION_PATCHED_HEAD"
if creature_spell_fixture_validate_cpp_source_derivation \
    "$DERIVATION_HARNESS" "$DERIVATION_SOURCE" "$DERIVATION_FIXTURE" \
    2>/dev/null; then
  fail "source derivation accepted a moved remote base ref"
fi
git -C "$DERIVATION_SOURCE" update-ref \
  "$DERIVATION_REMOTE_REF" "$DERIVATION_BASE_HEAD"

git -C "$DERIVATION_SOURCE" checkout -q --detach "$DERIVATION_BASE_HEAD"
if creature_spell_fixture_validate_cpp_source_derivation \
    "$DERIVATION_HARNESS" "$DERIVATION_SOURCE" "$DERIVATION_FIXTURE" \
    2>/dev/null; then
  fail "source derivation accepted the unpatched HEAD"
fi
git -C "$DERIVATION_SOURCE" checkout -q --detach "$DERIVATION_PATCHED_HEAD"

jq '.source_derivation.base_tree = ("0" * 40)' \
  "$DERIVATION_FIXTURE" >"$DERIVATION_MUTATED"
if creature_spell_fixture_validate_cpp_source_derivation \
    "$DERIVATION_HARNESS" "$DERIVATION_SOURCE" "$DERIVATION_MUTATED" \
    2>/dev/null; then
  fail "source derivation accepted the wrong canonical base tree"
fi
jq '.source_derivation.patched_tree = ("0" * 40)' \
  "$DERIVATION_FIXTURE" >"$DERIVATION_MUTATED"
if creature_spell_fixture_validate_cpp_source_derivation \
    "$DERIVATION_HARNESS" "$DERIVATION_SOURCE" "$DERIVATION_MUTATED" \
    2>/dev/null; then
  fail "source derivation accepted the wrong patched tree"
fi

git -C "$DERIVATION_SOURCE" commit -q --allow-empty -m extra-parent
DERIVATION_EXTRA_HEAD="$(git -C "$DERIVATION_SOURCE" rev-parse HEAD)"
jq --arg head "$DERIVATION_EXTRA_HEAD" \
  '.source_derivation.patched_head = $head' \
  "$DERIVATION_FIXTURE" >"$DERIVATION_MUTATED"
if creature_spell_fixture_validate_cpp_source_derivation \
    "$DERIVATION_HARNESS" "$DERIVATION_SOURCE" "$DERIVATION_MUTATED" \
    2>/dev/null; then
  fail "source derivation accepted a patched commit whose parent is not the base"
fi
git -C "$DERIVATION_SOURCE" checkout -q --detach "$DERIVATION_PATCHED_HEAD"

git -C "$DERIVATION_SOURCE" checkout -q -b merge-parent "$DERIVATION_BASE_HEAD"
printf 'second parent\n' >"$DERIVATION_SOURCE/src/second-parent.cpp"
git -C "$DERIVATION_SOURCE" add src/second-parent.cpp
git -C "$DERIVATION_SOURCE" commit -q -m second-parent
DERIVATION_SECOND_PARENT="$(git -C "$DERIVATION_SOURCE" rev-parse HEAD)"
git -C "$DERIVATION_SOURCE" checkout -q --detach "$DERIVATION_PATCHED_HEAD"
git -C "$DERIVATION_SOURCE" merge -q --no-ff -m merge-parent \
  "$DERIVATION_SECOND_PARENT"
DERIVATION_MERGE_HEAD="$(git -C "$DERIVATION_SOURCE" rev-parse HEAD)"
DERIVATION_MERGE_TREE="$(git -C "$DERIVATION_SOURCE" rev-parse 'HEAD^{tree}')"
jq --arg head "$DERIVATION_MERGE_HEAD" --arg tree "$DERIVATION_MERGE_TREE" '
  .source_derivation.patched_head = $head
  | .source_derivation.patched_tree = $tree
' "$DERIVATION_FIXTURE" >"$DERIVATION_MUTATED"
if creature_spell_fixture_validate_cpp_source_derivation \
    "$DERIVATION_HARNESS" "$DERIVATION_SOURCE" "$DERIVATION_MUTATED" \
    2>/dev/null; then
  fail "source derivation accepted a merge commit with multiple parents"
fi
git -C "$DERIVATION_SOURCE" checkout -q --detach "$DERIVATION_PATCHED_HEAD"

printf '\n' >>"$DERIVATION_PATCH"
DERIVATION_TAMPERED_SHA="$(creature_spell_fixture_sha256_of_file \
  "$DERIVATION_PATCH")"
jq --arg sha "$DERIVATION_TAMPERED_SHA" \
  '.source_derivation.patch_sha256 = $sha' \
  "$DERIVATION_FIXTURE" >"$DERIVATION_MUTATED"
if creature_spell_fixture_validate_cpp_source_derivation \
    "$DERIVATION_HARNESS" "$DERIVATION_SOURCE" "$DERIVATION_MUTATED" \
    2>/dev/null; then
  fail "source derivation accepted patch bytes differing from the Git diff"
fi
git -C "$DERIVATION_SOURCE" diff \
  --binary --full-index --no-ext-diff --no-textconv --no-renames --no-color \
  --src-prefix=a/ --dst-prefix=b/ --diff-algorithm=myers \
  "$DERIVATION_BASE_HEAD" "$DERIVATION_PATCHED_HEAD" -- \
  >"$DERIVATION_PATCH"

jq '.source_derivation.changed_paths = ["src/unreviewed.cpp"]' \
  "$DERIVATION_FIXTURE" >"$DERIVATION_MUTATED"
if creature_spell_fixture_validate_cpp_source_derivation \
    "$DERIVATION_HARNESS" "$DERIVATION_SOURCE" "$DERIVATION_MUTATED" \
    2>/dev/null; then
  fail "source derivation accepted a different changed-path set"
fi

current_row_expression="$(
  creature_spell_fixture_character_row_hash_expression current
)"
prelogin_row_expression="$(
  creature_spell_fixture_character_row_hash_expression prelogin
)"
forced_offline_row_expression="$(
  creature_spell_fixture_character_row_hash_expression forced-offline
)"
immutable_expression="$(
  creature_spell_fixture_character_immutable_hash_expression
)"
assert_eq 87 \
  "$(grep -oE '`[^`]+`' <<<"$current_row_expression" | wc -l | tr -d ' ')" \
  "complete character row hash column count"
assert_eq 79 \
  "$(grep -oE '`[^`]+`' <<<"$prelogin_row_expression" | wc -l | tr -d ' ')" \
  "pre-login hash retained-column count after eight deterministic replacements"
assert_eq 87 \
  "$(tr -cd ',' <<<"$prelogin_row_expression" | wc -c | tr -d ' ')" \
  "deterministic pre-login complete-row expression width"
assert_eq 86 \
  "$(grep -oE '`[^`]+`' <<<"$forced_offline_row_expression" | wc -l | tr -d ' ')" \
  "forced-offline hash retains every column except the replaced online marker"
assert_eq 87 \
  "$(tr -cd ',' <<<"$forced_offline_row_expression" | wc -c | tr -d ' ')" \
  "forced-offline complete-row expression width"
assert_eq 14 \
  "$(grep -oE '`[^`]+`' <<<"$immutable_expression" | wc -l | tr -d ' ')" \
  "non-restored character projection column count"
[[ "$current_row_expression" == *'`slot`'* \
  && "$current_row_expression" == *'`personalTabardBackgroundColor`'* ]] \
  || fail "complete-row CAS omits fields outside the former partial snapshot"
[[ "$prelogin_row_expression" == *'CAST(-2749.52 AS FLOAT)'* \
  && "$prelogin_row_expression" == *'50000'* ]] \
  || fail "pre-login row hash does not encode the guarded relocation/health"
[[ "$forced_offline_row_expression" != *'`online`'* \
  && "$forced_offline_row_expression" == *'`position_x`'* \
  && "$forced_offline_row_expression" == *'`health`'* ]] \
  || fail "forced-offline row hash changes fields beyond the online marker"
assert_eq 87 "$CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_COLUMN_COUNT" \
  "pinned characters schema column count"
assert_eq 2888 "$CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_METADATA_BYTES" \
  "pinned characters schema metadata length"
assert_eq \
  1c8ef9a9367734daced44acf567cc5453357498c04d57c37f2cce3e5108aa24c \
  "$CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_SHA256" \
  "pinned characters schema digest"

MOCK_SCHEMA_RESULT="${CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_COLUMN_COUNT}"$'\t'"${CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_METADATA_BYTES}"$'\t'"${CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_SHA256}"
SCHEMA_SQL="$TEST_ROOT/schema.sql"
loot_fixture_character_mysql() {
  [ "$1" = -e ] || return 1
  printf '%s\n' "$2" >"$SCHEMA_SQL"
  printf '%s\n' "$MOCK_SCHEMA_RESULT"
}
creature_spell_fixture_validate_character_schema
grep -q 'group_concat_max_len = 1048576' "$SCHEMA_SQL" \
  || fail "schema validation does not bound GROUP_CONCAT safely"
grep -q "TABLE_NAME = 'characters'" "$SCHEMA_SQL" \
  || fail "schema validation query is not scoped to characters"
MOCK_SCHEMA_RESULT="86"$'\t'"2888"$'\t'"${CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_SHA256}"
if creature_spell_fixture_validate_character_schema 2>/dev/null; then
  fail "schema validation accepted an 86-column table"
fi
MOCK_SCHEMA_RESULT="${CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_COLUMN_COUNT}"$'\t'"${CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_METADATA_BYTES}"$'\t'"${CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_SHA256}"

GHOST_STATE_SQL="$TEST_ROOT/ghost-state.sql"
MOCK_GHOST_STATE=$'0\t0'
loot_fixture_character_mysql() {
  [ "$1" = -e ] || return 1
  printf '%s\n' "$2" >"$GHOST_STATE_SQL"
  printf '%s\n' "$MOCK_GHOST_STATE"
}
creature_spell_fixture_verify_no_persisted_ghost_state
grep -q 'FROM character_aura' "$GHOST_STATE_SQL" \
  || fail "ghost preflight omits character_aura"
grep -q 'FROM character_aura_effect' "$GHOST_STATE_SQL" \
  || fail "ghost preflight omits associated character_aura_effect rows"
grep -q 'guid = 15' "$GHOST_STATE_SQL" \
  || fail "ghost preflight is not pinned to character 15"
grep -q 'spell = 8326' "$GHOST_STATE_SQL" \
  || fail "ghost preflight is not pinned to spell 8326"
for MOCK_GHOST_STATE in $'1\t3' $'1\t0' $'0\t3'; do
  if creature_spell_fixture_verify_no_persisted_ghost_state 2>/dev/null; then
    fail "ghost preflight accepted aura/effect counts ${MOCK_GHOST_STATE}"
  fi
done
MOCK_GHOST_STATE=$'0\t0'

# Abrupt C++ socket shutdown can leave only the owned online marker behind.
# Exercise the reconciler without a DB: it must first prove the stopped-world
# window, no-op when already offline, and otherwise issue one tightly bounded
# online-only CAS.
(
  RECONCILE_SQL="$TEST_ROOT/reconcile-offline-noop.sql"
  RECONCILE_RUNTIME="$TEST_ROOT/reconcile-offline-noop.runtime"
  CREATURE_SPELL_FIXTURE_PHASE=captured
  CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256="$(repeat_digest b)"
  CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256="$(repeat_digest c)"
  creature_spell_fixture_require_worlds_stopped() {
    : >"$RECONCILE_RUNTIME"
  }
  loot_fixture_character_mysql() {
    [ "$1" = -e ] || return 1
    printf '%s\n' "$2" >>"$RECONCILE_SQL"
    [[ "$2" != *'UPDATE characters AS fixture_character'* ]] \
      || fail "already-offline reconciliation attempted an UPDATE"
    printf '0\t0\n'
  }
  creature_spell_fixture_reconcile_owned_online_marker \
    || fail "already-offline marker reconciliation failed"
  [ -f "$RECONCILE_RUNTIME" ] \
    || fail "already-offline reconciliation skipped the stopped-world proof"
  ! grep -q 'UPDATE characters' "$RECONCILE_SQL" \
    || fail "already-offline reconciliation mutated characters"
)

(
  RECONCILE_SQL="$TEST_ROOT/reconcile-owned-success.sql"
  RECONCILE_RUNTIME="$TEST_ROOT/reconcile-owned-success.runtime"
  CREATURE_SPELL_FIXTURE_PHASE=applied
  CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256="$(repeat_digest b)"
  CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256="$(repeat_digest c)"
  creature_spell_fixture_require_worlds_stopped() {
    : >"$RECONCILE_RUNTIME"
  }
  loot_fixture_character_mysql() {
    [ "$1" = -e ] || return 1
    printf '%s\n' "$2" >>"$RECONCILE_SQL"
    if [[ "$2" == *'UPDATE characters AS fixture_character'* ]]; then
      printf '1\n'
    else
      printf '1\t1\n'
    fi
  }
  creature_spell_fixture_reconcile_owned_online_marker \
    || fail "exact owned online marker reconciliation failed"
  [ -f "$RECONCILE_RUNTIME" ] \
    || fail "owned marker reconciliation skipped the stopped-world proof"
  grep -q 'SET fixture_character.online = 0' "$RECONCILE_SQL" \
    || fail "owned marker CAS does not update only online"
  grep -q 'fixture_character.guid = 15' "$RECONCILE_SQL" \
    && grep -q 'fixture_character.account = 9' "$RECONCILE_SQL" \
    && grep -q 'fixture_character.online = 1' "$RECONCILE_SQL" \
    || fail "owned marker CAS is not pinned to guid 15/account 9/online 1"
  grep -q 'online_guard.online_count = 1' "$RECONCILE_SQL" \
    && grep -q 'online_guard.owned_online_count = 1' "$RECONCILE_SQL" \
    || fail "owned marker CAS does not repeat the unique-online-row proof"
  grep -q "= '$(repeat_digest b)'" "$RECONCILE_SQL" \
    || fail "owned marker CAS omits the hypothetical pre-login row hash"
  grep -q "= '$(repeat_digest c)'" "$RECONCILE_SQL" \
    || fail "owned marker CAS omits the immutable-column hash"
  grep -q 'FROM corpse' "$RECONCILE_SQL" \
    && grep -q 'FROM character_aura' "$RECONCILE_SQL" \
    && grep -q 'FROM character_aura_effect' "$RECONCILE_SQL" \
    && grep -q 'spell = 8326' "$RECONCILE_SQL" \
    || fail "owned marker CAS omits corpse/ghost-state exclusions"
  assert_eq 1 \
    "$(grep -c 'SET fixture_character.online = 0' "$RECONCILE_SQL")" \
    "owned marker online-only mutation count"
)

(
  RECONCILE_SQL="$TEST_ROOT/reconcile-foreign-shape.sql"
  CREATURE_SPELL_FIXTURE_PHASE=applied
  CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256="$(repeat_digest b)"
  CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256="$(repeat_digest c)"
  creature_spell_fixture_require_worlds_stopped() { return 0; }
  loot_fixture_character_mysql() {
    [ "$1" = -e ] || return 1
    printf '%s\n' "$2" >>"$RECONCILE_SQL"
    [[ "$2" != *'UPDATE characters AS fixture_character'* ]] \
      || fail "foreign online shape reached the marker UPDATE"
    printf '%s\n' "$RECONCILE_SHAPE"
  }
  for RECONCILE_SHAPE in $'1\t0' $'2\t1'; do
    if creature_spell_fixture_reconcile_owned_online_marker 2>/dev/null; then
      fail "marker reconciler accepted foreign online shape ${RECONCILE_SHAPE}"
    fi
  done
  ! grep -q 'UPDATE characters' "$RECONCILE_SQL" \
    || fail "foreign online shape mutated characters"
)

(
  RECONCILE_SQL="$TEST_ROOT/reconcile-wrong-phase.sql"
  CREATURE_SPELL_FIXTURE_PHASE=captured
  CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256="$(repeat_digest b)"
  CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256="$(repeat_digest c)"
  creature_spell_fixture_require_worlds_stopped() { return 0; }
  loot_fixture_character_mysql() {
    [ "$1" = -e ] || return 1
    printf '%s\n' "$2" >>"$RECONCILE_SQL"
    [[ "$2" != *'UPDATE characters AS fixture_character'* ]] \
      || fail "non-applied journal reached the marker UPDATE"
    printf '1\t1\n'
  }
  if creature_spell_fixture_reconcile_owned_online_marker 2>/dev/null; then
    fail "marker reconciler accepted an online row outside applied phase"
  fi
)

(
  RECONCILE_SQL="$TEST_ROOT/reconcile-cas-rejected.sql"
  CREATURE_SPELL_FIXTURE_PHASE=applied
  CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256="$(repeat_digest b)"
  CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256="$(repeat_digest c)"
  creature_spell_fixture_require_worlds_stopped() { return 0; }
  loot_fixture_character_mysql() {
    [ "$1" = -e ] || return 1
    printf '%s\n' "$2" >>"$RECONCILE_SQL"
    if [[ "$2" == *'UPDATE characters AS fixture_character'* ]]; then
      printf '0\n'
    else
      printf '1\t1\n'
    fi
  }
  if creature_spell_fixture_reconcile_owned_online_marker 2>/dev/null; then
    fail "marker reconciler accepted a rejected hash/death/ghost CAS"
  fi
)

(
  RECONCILE_MYSQL_CALL="$TEST_ROOT/reconcile-unsafe-runtime.mysql"
  CREATURE_SPELL_FIXTURE_PHASE=applied
  creature_spell_fixture_require_worlds_stopped() { return 1; }
  loot_fixture_character_mysql() {
    : >"$RECONCILE_MYSQL_CALL"
    return 1
  }
  if creature_spell_fixture_reconcile_owned_online_marker 2>/dev/null; then
    fail "marker reconciler accepted an active world/listener window"
  fi
  [ ! -e "$RECONCILE_MYSQL_CALL" ] \
    || fail "unsafe runtime state reached marker DB inspection"
)

loot_fixture_character_mysql() {
  [ "$1" = -e ] || return 1
  printf '%s\n' "$2" >"$SCHEMA_SQL"
  printf '%s\n' "$MOCK_SCHEMA_RESULT"
}

declare -a ORIGINAL_FIELDS=()
for ((field_index = 0; field_index < 73; field_index++)); do
  ORIGINAL_FIELDS+=(0)
done
ORIGINAL_FIELDS[0]=H4C66676865616C
ORIGINAL_FIELDS[26]=H
ORIGINAL_FIELDS[43]=N
ORIGINAL_FIELDS[63]=N
ORIGINAL_FIELDS[64]=N
ORIGINAL_FIELDS[65]=N
ORIGINAL_TSV="$(IFS=$'\t'; printf '%s' "${ORIGINAL_FIELDS[*]}")"
creature_spell_fixture_validate_character_original_tsv "$ORIGINAL_TSV"
SHORT_TSV="$(IFS=$'\t'; printf '%s' "${ORIGINAL_FIELDS[*]:0:72}")"
if creature_spell_fixture_validate_character_original_tsv "$SHORT_TSV"; then
  fail "restore projection accepted fewer than 73 CHAR_UPD_CHARACTER fields"
fi

CREATURE_SPELL_FIXTURE_JOURNAL="$TEST_ROOT/fixture-journal.json"
CREATURE_SPELL_FIXTURE_CLEANUP_MARKER="${CREATURE_SPELL_FIXTURE_JOURNAL}.cleanup-complete"
CREATURE_SPELL_FIXTURE_DB_CONF="$TEST_ROOT/worldserver.conf"
printf '%s\n' '# fixture-only mock config' >"$CREATURE_SPELL_FIXTURE_DB_CONF"
chmod 600 "$CREATURE_SPELL_FIXTURE_DB_CONF"
creature_spell_fixture_validate_db_config
CREATURE_SPELL_FIXTURE_SIDE=rust
CREATURE_SPELL_FIXTURE_PM2_RUST_WORLD=rust-world
CREATURE_SPELL_FIXTURE_PM2_CPP_WORLD=cpp-world
CREATURE_SPELL_FIXTURE_WORLD_PORT=8085
CREATURE_SPELL_FIXTURE_INSTANCE_PORT=8086
CREATURE_SPELL_FIXTURE_ORCHESTRATION_LOCK="$TEST_ROOT/orchestration.lock"
CREATURE_SPELL_FIXTURE_DATABASE_SNAPSHOT_SHA256="$(repeat_digest e)"
CREATURE_SPELL_FIXTURE_CREATED_AT=2026-08-08T00:00:00Z
CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV="$ORIGINAL_TSV"
CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_SHA256="$(
  creature_spell_fixture_sha256_of_text "$ORIGINAL_TSV"
)"
CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_ROW_SHA256="$(repeat_digest a)"
CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256="$(repeat_digest b)"
CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256=""
CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256="$(repeat_digest c)"

creature_spell_fixture_write_journal create armed
assert_eq armed "$(jq -r '.phase' "$CREATURE_SPELL_FIXTURE_JOURNAL")" \
  "journal armed phase"
assert_eq 73 \
  "$(jq -r '.character.original_fields | length' "$CREATURE_SPELL_FIXTURE_JOURNAL")" \
  "journal restore projection length"
assert_eq CHAR_UPD_CHARACTER-73-fields-v1 \
  "$(jq -r '.character.restore_projection' "$CREATURE_SPELL_FIXTURE_JOURNAL")" \
  "journal restore projection contract"
assert_eq 87 \
  "$(jq -r '.character.schema_column_count' "$CREATURE_SPELL_FIXTURE_JOURNAL")" \
  "journal schema column count"
creature_spell_fixture_write_journal replace applied
assert_eq applied "$(jq -r '.phase' "$CREATURE_SPELL_FIXTURE_JOURNAL")" \
  "journal applied phase"
CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256="$(repeat_digest d)"
creature_spell_fixture_write_journal replace captured
assert_eq captured "$(jq -r '.phase' "$CREATURE_SPELL_FIXTURE_JOURNAL")" \
  "journal captured phase"
assert_eq "$(repeat_digest d)" \
  "$(jq -r '.character.post_login_row_sha256' "$CREATURE_SPELL_FIXTURE_JOURNAL")" \
  "durable post-login complete-row hash"
creature_spell_fixture_write_journal replace restored
assert_eq restored "$(jq -r '.phase' "$CREATURE_SPELL_FIXTURE_JOURNAL")" \
  "journal restored phase"
CREATURE_SPELL_FIXTURE_PHASE=""
CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV=""
creature_spell_fixture_load_journal
assert_eq restored "$CREATURE_SPELL_FIXTURE_PHASE" "journal load phase"
assert_eq "$ORIGINAL_TSV" "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV" \
  "journal load exact restore projection"

# Recovery must obtain credentials only from a private journal whose DB config
# pathname, bytes, and inode still match the recorded provenance. This preload
# intentionally performs no MySQL query; the full loader does that afterward.
PRELOAD_JOURNAL="$TEST_ROOT/preload-journal.json"
cp -- "$CREATURE_SPELL_FIXTURE_JOURNAL" "$PRELOAD_JOURNAL"
chmod 600 "$PRELOAD_JOURNAL"
CREATURE_SPELL_FIXTURE_JOURNAL="$PRELOAD_JOURNAL"
CREATURE_SPELL_FIXTURE_DB_CONF=""
CREATURE_SPELL_FIXTURE_DB_CONF_SHA256=""
CREATURE_SPELL_FIXTURE_DB_CONF_IDENTITY=""
PRELOAD_MYSQL_CALL="$TEST_ROOT/preload-mysql-call"
loot_fixture_character_mysql() {
  : >"$PRELOAD_MYSQL_CALL"
  return 1
}
creature_spell_fixture_preload_recovery_db_config
[ ! -e "$PRELOAD_MYSQL_CALL" ] \
  || fail "recovery DB config preload queried MySQL before credentials"
assert_eq "$TEST_ROOT/worldserver.conf" \
  "$CREATURE_SPELL_FIXTURE_DB_CONF" "recovery preload DB config"
assert_eq "$(creature_spell_fixture_sha256_of_file \
  "$CREATURE_SPELL_FIXTURE_DB_CONF")" \
  "$CREATURE_SPELL_FIXTURE_DB_CONF_SHA256" "recovery preload DB config SHA"
assert_eq "$(stat -c '%d:%i' -- "$CREATURE_SPELL_FIXTURE_DB_CONF")" \
  "$CREATURE_SPELL_FIXTURE_DB_CONF_IDENTITY" \
  "recovery preload DB config identity"

chmod 640 "$PRELOAD_JOURNAL"
if creature_spell_fixture_preload_recovery_db_config 2>/dev/null; then
  fail "recovery preload accepted a non-private journal"
fi
chmod 600 "$PRELOAD_JOURNAL"

BAD_PRELOAD_JOURNAL="$TEST_ROOT/bad-preload-journal.json"
jq --arg digest "$(repeat_digest 0)" \
  '.recovery.db_conf_sha256 = $digest' "$PRELOAD_JOURNAL" \
  >"$BAD_PRELOAD_JOURNAL"
chmod 600 "$BAD_PRELOAD_JOURNAL"
CREATURE_SPELL_FIXTURE_JOURNAL="$BAD_PRELOAD_JOURNAL"
if creature_spell_fixture_preload_recovery_db_config 2>/dev/null; then
  fail "recovery preload accepted a mismatched DB config SHA"
fi
jq '.recovery.db_conf_identity = "0:0"' "$PRELOAD_JOURNAL" \
  >"$BAD_PRELOAD_JOURNAL"
chmod 600 "$BAD_PRELOAD_JOURNAL"
if creature_spell_fixture_preload_recovery_db_config 2>/dev/null; then
  fail "recovery preload accepted a mismatched DB config identity"
fi
CREATURE_SPELL_FIXTURE_JOURNAL="$TEST_ROOT/fixture-journal.json"

# Simulate a concurrent change to `slot` after the read-only precheck. `slot`
# was outside the former partial snapshot and outside the 73 restored fields.
# MySQL therefore reports ROW_COUNT() = 0 only if the atomic WHERE includes the
# complete-row/immutable hashes; the guard must fail without an overwrite.
TOCTOU_SQL="$TEST_ROOT/toctou-update.sql"
creature_spell_fixture_verify_character_original() { return 1; }
creature_spell_fixture_verify_character_state() {
  [ "$1" = "$CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256" ]
}
creature_spell_fixture_verify_character_immutable() { return 0; }
loot_fixture_character_mysql() {
  [ "$1" = -e ] || return 1
  printf '%s\n' "$2" >"$TOCTOU_SQL"
  printf '0\n'
}
CREATURE_SPELL_FIXTURE_PHASE=applied
CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256=""
if creature_spell_fixture_restore_character 2>/dev/null; then
  fail "character restoration ignored a TOCTOU change to slot"
fi
grep -q 'UPDATE characters' "$TOCTOU_SQL" \
  || fail "TOCTOU test did not reach the character restoration UPDATE"
grep -q '`slot`' "$TOCTOU_SQL" \
  || fail "atomic character CAS omits slot outside the restored projection"
grep -q "= '${CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256}'" \
  "$TOCTOU_SQL" || fail "atomic character CAS omits the journaled source hash"
for restored_field in totaltime leveltime logout_time latency exploredZones \
  equipmentCache knownTitles honorRestBonus lastLoginBuild; do
  grep -q "${restored_field} =" "$TOCTOU_SQL" \
    || fail "73-field restore SQL omits ${restored_field}"
done

# An applied journal is deliberately read-only if the row no longer equals
# the deterministic pre-login hash: no AI or character write may be attempted.
RECOVERY_WRITES="$TEST_ROOT/recovery-writes"
creature_spell_fixture_load_journal() {
  CREATURE_SPELL_FIXTURE_PHASE=applied
  return 0
}
creature_spell_fixture_require_safe_db_window() { return 0; }
creature_spell_fixture_verify_character_prelogin() { return 1; }
creature_spell_fixture_cas_ai_name() {
  printf 'ai\n' >>"$RECOVERY_WRITES"
}
creature_spell_fixture_restore_character() {
  printf 'character\n' >>"$RECOVERY_WRITES"
}
if creature_spell_fixture_restore_guard 2>/dev/null; then
  fail "applied recovery accepted an unjournaled post-login row"
fi
[ ! -e "$RECOVERY_WRITES" ] \
  || fail "unsafe applied recovery attempted a DB write"

# The post-capture verifier must durably record the exact core row first, then
# reject any newly persisted ghost aura/effects before cleanup can be
# accredited. This leaves a captured journal that explicit recovery can use.
POST_CAPTURE_GHOST_CLEAN=1
POST_CAPTURE_WRITTEN_PHASE=""
creature_spell_fixture_load_journal() {
  CREATURE_SPELL_FIXTURE_PHASE=applied
  return 0
}
creature_spell_fixture_reconcile_owned_online_marker() { return 0; }
creature_spell_fixture_require_safe_db_window() { return 0; }
creature_spell_fixture_snapshot_character_post_login() {
  CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256="$(repeat_digest d)"
}
creature_spell_fixture_verify_character_post_login() { return 0; }
creature_spell_fixture_verify_no_persisted_ghost_state() {
  [ "$POST_CAPTURE_GHOST_CLEAN" -eq 1 ]
}
creature_spell_fixture_write_journal() {
  [ "$1" = replace ] && [ "$2" = captured ] || return 1
  POST_CAPTURE_WRITTEN_PHASE=captured
  CREATURE_SPELL_FIXTURE_PHASE=captured
}
CREATURE_SPELL_FIXTURE_GHOST_POST_CAPTURE_VERIFIED=0
creature_spell_fixture_record_post_login_snapshot \
  || fail "post-capture verifier rejected clean ghost state"
assert_eq captured "$POST_CAPTURE_WRITTEN_PHASE" \
  "post-capture verifier durable phase"
assert_eq 1 "$CREATURE_SPELL_FIXTURE_GHOST_POST_CAPTURE_VERIFIED" \
  "post-capture ghost accreditation"

POST_CAPTURE_GHOST_CLEAN=0
POST_CAPTURE_WRITTEN_PHASE=""
CREATURE_SPELL_FIXTURE_GHOST_POST_CAPTURE_VERIFIED=1
if creature_spell_fixture_record_post_login_snapshot 2>/dev/null; then
  fail "post-capture verifier accredited newly persisted ghost state"
fi
assert_eq captured "$POST_CAPTURE_WRITTEN_PHASE" \
  "ghost failure retains a recoverable captured phase"
assert_eq 0 "$CREATURE_SPELL_FIXTURE_GHOST_POST_CAPTURE_VERIFIED" \
  "ghost failure clears post-capture accreditation"

CPP_WRAPPER="$SCRIPT_ROOT/capture-cpp.sh"
RUST_WRAPPER="$SCRIPT_ROOT/capture-rust.sh"
RECOVERY_WRAPPER="$SCRIPT_ROOT/recover-creature-spell-casting-fixture.sh"
bash -n "$CPP_WRAPPER" "$RUST_WRAPPER" "$RECOVERY_WRAPPER" \
  "$SCRIPT_ROOT/creature-spell-casting-fixture-common.sh"
APPLY_GUARD_BODY="$TEST_ROOT/apply-guard.sh"
sed -n '/^creature_spell_fixture_apply_guard() {$/,/^}$/p' \
  "$SCRIPT_ROOT/creature-spell-casting-fixture-common.sh" >"$APPLY_GUARD_BODY"
apply_ghost_line="$(line_of \
  'creature_spell_fixture_verify_no_persisted_ghost_state' "$APPLY_GUARD_BODY")"
apply_snapshot_line="$(line_of \
  'creature_spell_fixture_snapshot_character' "$APPLY_GUARD_BODY")"
apply_mutation_line="$(line_of \
  'creature_spell_fixture_cas_ai_name' "$APPLY_GUARD_BODY")"
((apply_ghost_line < apply_snapshot_line && apply_snapshot_line < apply_mutation_line)) \
  || fail "fixture mutation can run before the persisted ghost preflight"
grep -q 'CREATURE_SPELL_FIXTURE_GHOST_PREFLIGHT_VERIFIED" -eq 1' \
  "$SCRIPT_ROOT/creature-spell-casting-fixture-common.sh" \
  || fail "capture evidence does not require ghost preflight accreditation"
grep -q 'CREATURE_SPELL_FIXTURE_GHOST_POST_CAPTURE_VERIFIED" -eq 1' \
  "$SCRIPT_ROOT/creature-spell-casting-fixture-common.sh" \
  || fail "capture evidence does not require post-capture ghost accreditation"
POST_CAPTURE_BODY="$TEST_ROOT/post-capture-snapshot.sh"
sed -n '/^creature_spell_fixture_record_post_login_snapshot() {$/,/^}$/p' \
  "$SCRIPT_ROOT/creature-spell-casting-fixture-common.sh" >"$POST_CAPTURE_BODY"
post_reconcile_line="$(line_of \
  'creature_spell_fixture_reconcile_owned_online_marker' "$POST_CAPTURE_BODY")"
post_offline_gate_line="$(line_of \
  'creature_spell_fixture_require_safe_db_window' "$POST_CAPTURE_BODY")"
post_snapshot_line="$(line_of \
  'creature_spell_fixture_snapshot_character_post_login' "$POST_CAPTURE_BODY")"
((post_reconcile_line < post_offline_gate_line \
  && post_offline_gate_line < post_snapshot_line)) \
  || fail "post-capture marker reconciliation is not between stopped-world proof and global offline snapshot gate"
for wrapper in "$CPP_WRAPPER" "$RUST_WRAPPER"; do
  grep -q 'creature-spell-casting)' "$wrapper" \
    || fail "$(basename "$wrapper") omits the creature-spell finalize branch"
  grep -q 'creature_spell_fixture_bot_evidence' "$wrapper" \
    || fail "$(basename "$wrapper") omits strict bot report validation"
  grep -q 'creature_spell_fixture_capture_evidence' "$wrapper" \
    || fail "$(basename "$wrapper") omits creature fixture evidence"
  grep -q '"\$CAPTURE_BOT_REPORT" "\$CAPTURE_BOT_REPORT_SHA256"' "$wrapper" \
    || grep -q '"\$CPP_CAPTURE_BOT_REPORT" "\$CPP_CAPTURE_BOT_REPORT_SHA256"' "$wrapper" \
    || fail "$(basename "$wrapper") does not pass all four evidence arguments"
  grep -q -- '--creature-spell-capture --single' "$wrapper" \
    || fail "$(basename "$wrapper") does not print the pinned bot command"
done

cpp_snapshot_line="$(line_of 'creature_spell_fixture_record_post_login_snapshot' "$CPP_WRAPPER")"
cpp_restore_line="$(line_of 'creature_spell_fixture_restore_guard' "$CPP_WRAPPER")"
cpp_normal_start_line="$(line_of 'pm2 start "$PM2_RUST_WORLD"' "$CPP_WRAPPER")"
((cpp_snapshot_line < cpp_restore_line && cpp_restore_line < cpp_normal_start_line)) \
  || fail "C++ wrapper restores/restarts before durable post-login snapshot"
rust_snapshot_line="$(line_of 'creature_spell_fixture_record_post_login_snapshot' "$RUST_WRAPPER")"
rust_restore_line="$(line_of 'creature_spell_fixture_restore_guard' "$RUST_WRAPPER")"
rust_normal_start_line="$(line_of '"$PM2_BIN" start "$RESTORE_FILE"' "$RUST_WRAPPER")"
((rust_snapshot_line < rust_restore_line && rust_restore_line < rust_normal_start_line)) \
  || fail "Rust wrapper restores/restarts before durable post-login snapshot"

CPP_WRAPPER_FLAT="$(tr '\n' ' ' <"$CPP_WRAPPER")"
[[ "$CPP_WRAPPER_FLAT" == *'capture_git_repo_clean_at_head'*'"$CPP_CAPTURE_SOURCE_REPO" "$CPP_CAPTURE_SOURCE_REPO_HEAD"'* ]] \
  || fail "C++ creature capture does not re-prove a clean source worktree"
grep -q 'cpp_capture_embedded_source_head "$CPP_CAPTURE_EXEC"' "$CPP_WRAPPER" \
  || fail "C++ creature capture does not bind the binary to its source revision"
grep -q 'source_exec_revision:' "$CPP_WRAPPER" \
  || fail "C++ raw manifest omits the embedded source revision"
grep -q 'creature_spell_fixture_validate_cpp_source_derivation' "$CPP_WRAPPER" \
  || fail "C++ creature capture does not revalidate the reviewed source patch"
assert_eq 3 \
  "$(grep -c 'creature_spell_fixture_validate_cpp_source_derivation' "$CPP_WRAPPER")" \
  "C++ creature capture derivation validation phase count"
grep -q -- '--argjson source_derivation' "$CPP_WRAPPER" \
  || fail "C++ raw manifest omits source_derivation publication"
grep -q 'else {source_derivation: $source_derivation}' "$CPP_WRAPPER" \
  || fail "C++ raw manifest does not retain the reviewed source derivation"
grep -q 'if $source_derivation == null then {}' "$CPP_WRAPPER" \
  || fail "unrelated C++ flows do not omit source_derivation"
# shellcheck disable=SC2294 # Evaluate only this literal function definition.
eval "$(sed -n '/^cpp_capture_embedded_source_head() {$/,/^}$/p' "$CPP_WRAPPER")"
FAKE_CPP_REVISION=a5f8da2eb001337b48d37807c5b0c9642b461b57
FAKE_CPP_EXEC="$TEST_ROOT/worldserver"
printf '%s\n' '#!/usr/bin/env bash' \
  "printf '%s\\n' 'TrinityCore rev. ${FAKE_CPP_REVISION} 2026-08-08'" \
  >"$FAKE_CPP_EXEC"
chmod 700 "$FAKE_CPP_EXEC"
assert_eq "$FAKE_CPP_REVISION" \
  "$(cpp_capture_embedded_source_head "$FAKE_CPP_EXEC")" \
  "C++ embedded source revision parser"

rust_source_revision_guard="$({
  sed -n '/^CAPTURE_SOURCE_REPO_HEAD=/,/^CAPTURE_HARNESS_WORKTREE_SHA256=/p' \
    "$RUST_WRAPPER"
})"
grep -q '\[ "$FLOW" = "creature-spell-casting" \]' \
  <<<"$rust_source_revision_guard" \
  || fail "Rust creature capture does not enter the source-revision guard"
grep -q 'rust_capture_embedded_source_head "$CAPTURE_EXEC"' \
  <<<"$rust_source_revision_guard" \
  || fail "Rust creature capture does not bind the binary to its source revision"
grep -q 'source_exec_revision:' "$RUST_WRAPPER" \
  || fail "Rust raw manifest omits the embedded source revision"
grep -q '&& \[ "$FLOW" != "creature-spell-casting" \]; }' "$RUST_WRAPPER" \
  || fail "Rust creature capture does not recheck the embedded revision before publication"
eval "$(sed -n '/^rust_capture_embedded_source_head() {$/,/^}$/p' "$RUST_WRAPPER")"
FAKE_RUST_REVISION=7f5e60a1689267482c52451b1afe3eeb9f2668d8
FAKE_RUST_EXEC="$TEST_ROOT/world-server"
printf '%s\n' '#!/usr/bin/env bash' \
  "printf '%s\\n' 'RustyCore World Server test (rev ${FAKE_RUST_REVISION})'" \
  >"$FAKE_RUST_EXEC"
chmod 700 "$FAKE_RUST_EXEC"
assert_eq "$FAKE_RUST_REVISION" \
  "$(rust_capture_embedded_source_head "$FAKE_RUST_EXEC")" \
  "Rust embedded source revision parser"

grep -q 'creature_spell_fixture_restore_guard' "$RECOVERY_WRAPPER" \
  || fail "explicit recovery omits the creature fixture guard"
grep -q 'capture_acquire_orchestration_lock' "$RECOVERY_WRAPPER" \
  || fail "explicit recovery does not acquire the orchestration lock"
recovery_preload_line="$(line_of \
  'creature_spell_fixture_preload_recovery_db_config' "$RECOVERY_WRAPPER")"
recovery_credentials_line="$(line_of \
  'load_loot_fixture_database_credentials' "$RECOVERY_WRAPPER")"
recovery_full_load_line="$(line_of \
  'creature_spell_fixture_load_journal' "$RECOVERY_WRAPPER")"
((recovery_preload_line < recovery_credentials_line \
  && recovery_credentials_line < recovery_full_load_line)) \
  || fail "explicit recovery does not preload DB provenance before credentials and full journal validation"
grep -q 'PRELOAD_JOURNAL_IDENTITY' "$RECOVERY_WRAPPER" \
  || fail "explicit recovery does not bind full validation to the preloaded journal inode"

echo "creature spell fixture shell tests passed"
