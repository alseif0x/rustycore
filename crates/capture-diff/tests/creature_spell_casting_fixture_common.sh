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
  be6302866ad2d09e1117ec30f1a81e5c0b3b2cacbebe93dc54fe9eecf814af8b \
  "$CREATURE_SPELL_FIXTURE_MANIFEST_SHA256" \
  "fixture manifest digest"

current_row_expression="$(
  creature_spell_fixture_character_row_hash_expression current
)"
prelogin_row_expression="$(
  creature_spell_fixture_character_row_hash_expression prelogin
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
assert_eq 14 \
  "$(grep -oE '`[^`]+`' <<<"$immutable_expression" | wc -l | tr -d ' ')" \
  "non-restored character projection column count"
[[ "$current_row_expression" == *'`slot`'* \
  && "$current_row_expression" == *'`personalTabardBackgroundColor`'* ]] \
  || fail "complete-row CAS omits fields outside the former partial snapshot"
[[ "$prelogin_row_expression" == *'CAST(-2749.52 AS FLOAT)'* \
  && "$prelogin_row_expression" == *'50000'* ]] \
  || fail "pre-login row hash does not encode the guarded relocation/health"
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

CPP_WRAPPER="$SCRIPT_ROOT/capture-cpp.sh"
RUST_WRAPPER="$SCRIPT_ROOT/capture-rust.sh"
RECOVERY_WRAPPER="$SCRIPT_ROOT/recover-creature-spell-casting-fixture.sh"
bash -n "$CPP_WRAPPER" "$RUST_WRAPPER" "$RECOVERY_WRAPPER" \
  "$SCRIPT_ROOT/creature-spell-casting-fixture-common.sh"
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

echo "creature spell fixture shell tests passed"
