#!/usr/bin/env bash
# Static/filesystem-only regression coverage for the issue-#24 fixture guard.
# It never invokes mysql, PM2, or either world server.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
SCRIPT_ROOT="$REPO_ROOT/crates/capture-diff/scripts"
TEST_ROOT="$(mktemp -d)"
trap 'rm -rf -- "$TEST_ROOT"' EXIT
chmod 700 "$TEST_ROOT"

WOW_BOT_FIXTURE_JOURNAL="$TEST_ROOT/fixture-journal.json"
LOOT_FIXTURE_CLEANUP_MARKER=""
LOOT_FIXTURE_GUARD_ENABLED=1

# shellcheck source=../scripts/loot-fixture-common.sh
source "$SCRIPT_ROOT/loot-fixture-common.sh"
# shellcheck source=../scripts/detour-chase-fixture-common.sh
source "$SCRIPT_ROOT/detour-chase-fixture-common.sh"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_eq() {
  local expected="$1"
  local actual="$2"
  local message="$3"
  [ "$actual" = "$expected" ] || fail "${message}: expected '${expected}', got '${actual}'"
}

line_of() {
  local pattern="$1"
  local path="$2"
  local line
  line="$(grep -n -m1 -- "$pattern" "$path" | cut -d: -f1)" \
    || fail "missing '${pattern}' in ${path}"
  printf '%s\n' "$line"
}

last_line_of() {
  local pattern="$1"
  local path="$2"
  local line
  line="$(grep -n -- "$pattern" "$path" | tail -n 1 | cut -d: -f1)" \
    || fail "missing '${pattern}' in ${path}"
  [ -n "$line" ] || fail "missing '${pattern}' in ${path}"
  printf '%s\n' "$line"
}

detour_chase_validate_committed_fixture "$REPO_ROOT"
assert_eq "$DETOUR_FIXTURE_FLOW" \
  "$(jq -r '.flow' "$DETOUR_FIXTURE_MANIFEST")" \
  "fixture manifest flow"
assert_eq 1380930628 "$(detour_chase_ping_fence_serial)" \
  "DTOR little-endian ping fence"

CPP_WRAPPER="$SCRIPT_ROOT/capture-cpp.sh"
RUST_WRAPPER="$SCRIPT_ROOT/capture-rust.sh"
RECOVERY_WRAPPER="$SCRIPT_ROOT/recover-detour-chase-fixture.sh"
bash -n "$CPP_WRAPPER" "$RUST_WRAPPER" "$RECOVERY_WRAPPER" \
  "$SCRIPT_ROOT/detour-chase-fixture-common.sh"
if grep -q 'detour_chase_require_capture_orchestration' \
    "$CPP_WRAPPER" "$RUST_WRAPPER"; then
  fail "permanent detour fail gate remains in a capture wrapper"
fi
grep -q 'detour evidence requires CPP_CAPTURE_EXEC and CPP_CAPTURE_EXEC_SHA256' \
  "$CPP_WRAPPER" || fail "C++ detour capture does not require a pinned executable"
grep -q 'cpp_capture_embedded_source_head "$CPP_CAPTURE_EXEC"' \
  "$CPP_WRAPPER" || fail "C++ binary is not tied to its embedded source revision"
grep -q 'source_exec_revision:' "$CPP_WRAPPER" \
  || fail "C++ raw manifest omits the binary/source revision link"
eval "$(
  sed -n '/^cpp_capture_embedded_source_head() {$/,/^}$/p' "$CPP_WRAPPER"
)"
fake_cpp_revision=5100ce3d8921872d50c00cf1db31e26787f689a2
fake_cpp_exec="$TEST_ROOT/worldserver"
printf '%s\n' \
  '#!/usr/bin/env bash' \
  "printf '%s\\n' 'TrinityCore rev. ${fake_cpp_revision} 2026-07-26'" \
  >"$fake_cpp_exec"
chmod 700 "$fake_cpp_exec"
assert_eq "$fake_cpp_revision" \
  "$(cpp_capture_embedded_source_head "$fake_cpp_exec")" \
  "C++ embedded source revision parser"
grep -q 'detour evidence requires RUST_CAPTURE_EXEC and RUST_CAPTURE_EXEC_SHA256' \
  "$RUST_WRAPPER" || fail "Rust detour capture does not require a pinned executable"
grep -q 'rust_capture_embedded_source_head "$CAPTURE_EXEC"' \
  "$RUST_WRAPPER" || fail "Rust binary is not tied to its embedded source revision"
grep -q 'source_exec_revision:' "$RUST_WRAPPER" \
  || fail "Rust raw manifest omits the binary/source revision link"
eval "$(
  sed -n '/^rust_capture_embedded_source_head() {$/,/^}$/p' "$RUST_WRAPPER"
)"
fake_rust_revision=f7888f5523883d82054cfb38fcfeadf9604c394e
fake_rust_exec="$TEST_ROOT/rust-world-server"
printf '%s\n' \
  '#!/usr/bin/env bash' \
  "printf '%s\\n' 'RustyCore World Server 0.1.0 (rev ${fake_rust_revision})'" \
  >"$fake_rust_exec"
chmod 700 "$fake_rust_exec"
assert_eq "$fake_rust_revision" \
  "$(rust_capture_embedded_source_head "$fake_rust_exec")" \
  "Rust embedded source revision parser"
grep -q 'detour evidence requires RUST_CAPTURE_EFFECTIVE_CONFIG' \
  "$RUST_WRAPPER" || fail "Rust detour capture does not require its effective config"
cpp_trap_line="$(line_of 'trap restore EXIT' "$CPP_WRAPPER")"
cpp_allocate_line="$(line_of 'detour_chase_allocate_private_data_dir' "$CPP_WRAPPER")"
cpp_arm_line="$(line_of 'detour_chase_arm_filesystem_recovery_journal' "$CPP_WRAPPER")"
cpp_populate_line="$(line_of 'detour_chase_populate_private_data_dir' "$CPP_WRAPPER")"
cpp_patch_line="$(line_of 'detour_chase_patch_config_data_dir' "$CPP_WRAPPER")"
cpp_apply_line="$(line_of 'detour_chase_apply_fixture_guard' "$CPP_WRAPPER")"
cpp_start_line="$(line_of 'pm2 start "$PM2_CPP_WORLD"' "$CPP_WRAPPER")"
cpp_stop_line="$(line_of 'capture_wait_for_world_stopped "$PM2_RUST_WORLD"' "$CPP_WRAPPER")"
cpp_peer_stop_line="$(last_line_of 'capture_pm2_process_stopped "$PM2_CPP_WORLD"' "$CPP_WRAPPER")"
((cpp_trap_line < cpp_allocate_line \
  && cpp_allocate_line < cpp_arm_line \
  && cpp_arm_line < cpp_populate_line \
  && cpp_populate_line < cpp_patch_line \
  && cpp_allocate_line < cpp_stop_line \
  && cpp_stop_line < cpp_peer_stop_line \
  && cpp_peer_stop_line < cpp_apply_line \
  && cpp_allocate_line < cpp_apply_line \
  && cpp_apply_line < cpp_start_line)) \
  || fail "C++ wrapper guard ordering is unsafe"
cpp_restore_line="$(line_of 'detour_chase_restore_fixture_guard' "$CPP_WRAPPER")"
cpp_remove_line="$(line_of 'detour_chase_remove_private_data_dir' "$CPP_WRAPPER")"
cpp_normal_start_line="$(line_of 'pm2 start "$PM2_RUST_WORLD"' "$CPP_WRAPPER")"
((cpp_restore_line < cpp_remove_line \
  && cpp_remove_line < cpp_normal_start_line)) \
  || fail "C++ wrapper restarts normal runtime before fixture cleanup"
rust_trap_line="$(line_of 'trap cleanup EXIT' "$RUST_WRAPPER")"
rust_allocate_line="$(line_of 'detour_chase_allocate_private_data_dir' "$RUST_WRAPPER")"
rust_arm_line="$(line_of 'detour_chase_arm_filesystem_recovery_journal' "$RUST_WRAPPER")"
rust_populate_line="$(line_of 'detour_chase_populate_private_data_dir' "$RUST_WRAPPER")"
rust_config_line="$(line_of 'detour_chase_create_rust_capture_config' "$RUST_WRAPPER")"
rust_checkpoint_line="$(
  line_of 'detour_chase_checkpoint_filesystem_recovery_metadata' "$RUST_WRAPPER"
)"
rust_apply_line="$(line_of 'detour_chase_apply_fixture_guard' "$RUST_WRAPPER")"
rust_start_line="$(line_of '"$PM2_BIN" start "$CAPTURE_CONFIG_FILE"' "$RUST_WRAPPER")"
rust_stop_line="$(last_line_of 'rust_remove_world_and_verify \\' "$RUST_WRAPPER")"
rust_peer_stop_line="$(last_line_of 'capture_pm2_process_stopped "$PM2_CPP_WORLD"' "$RUST_WRAPPER")"
((rust_trap_line < rust_allocate_line \
  && rust_allocate_line < rust_arm_line \
  && rust_arm_line < rust_populate_line \
  && rust_populate_line < rust_config_line \
  && rust_config_line < rust_checkpoint_line \
  && rust_checkpoint_line < rust_stop_line \
  && rust_populate_line < rust_stop_line \
  && rust_allocate_line < rust_stop_line \
  && rust_stop_line < rust_peer_stop_line \
  && rust_peer_stop_line < rust_apply_line \
  && rust_allocate_line < rust_apply_line \
  && rust_apply_line < rust_start_line)) \
  || fail "Rust wrapper guard ordering is unsafe"
rust_restore_line="$(line_of 'detour_chase_restore_fixture_guard' "$RUST_WRAPPER")"
rust_remove_config_line="$(last_line_of 'detour_chase_remove_rust_capture_config' "$RUST_WRAPPER")"
rust_remove_line="$(line_of 'detour_chase_remove_private_data_dir' "$RUST_WRAPPER")"
rust_normal_start_line="$(line_of '"$PM2_BIN" start "$RESTORE_FILE"' "$RUST_WRAPPER")"
((rust_restore_line < rust_remove_config_line \
  && rust_remove_config_line < rust_remove_line \
  && rust_remove_line < rust_normal_start_line)) \
  || fail "Rust wrapper restarts normal runtime before fixture cleanup"
grep -q 'capture_acquire_orchestration_lock' "$RECOVERY_WRAPPER" \
  || fail "explicit recovery does not acquire the capture lock"
grep -q 'DETOUR_FIXTURE_SIDE=""' "$RECOVERY_WRAPPER" \
  || fail "explicit recovery cannot load a fresh-shell journal"
if grep -q -- '--arg restore_sql' \
    "$SCRIPT_ROOT/detour-chase-fixture-common.sh"; then
  fail "sensitive recovery SQL is still passed through jq argv"
fi

NORMAL_DATA="$TEST_ROOT/normal-data"
mkdir "$NORMAL_DATA"
for child in dbc gt maps vmaps cameras; do
  mkdir "$NORMAL_DATA/$child"
done
CONFIG="$TEST_ROOT/worldserver.conf"
printf 'DataDir = "%s/"\nWorldServerPort = 8085\n' "$NORMAL_DATA" >"$CONFIG"
chmod 600 "$CONFIG"
detour_chase_validate_normal_data_dir "$CONFIG"
assert_eq "$NORMAL_DATA" "$DETOUR_FIXTURE_NORMAL_DATA_DIR" "normal DataDir"

DETOUR_FIXTURE_ENABLED=1
DETOUR_FIXTURE_SIDE=rust
DETOUR_CAPTURE_PRIVATE_PARENT="$TEST_ROOT"
detour_chase_prepare_private_data_dir
[ -f "$DETOUR_FIXTURE_PRIVATE_DATA_DIR/mmaps/0001.mmap" ] \
  || fail "private map asset missing"
[ ! -L "$DETOUR_FIXTURE_PRIVATE_DATA_DIR/mmaps/0001.mmap" ] \
  || fail "private map asset must be a real copy"
for child in dbc gt maps vmaps cameras; do
  [ -L "$DETOUR_FIXTURE_PRIVATE_DATA_DIR/$child" ] \
    || fail "private ${child} must be a symlink"
  assert_eq "$NORMAL_DATA/$child" \
    "$(readlink "$DETOUR_FIXTURE_PRIVATE_DATA_DIR/$child")" \
    "private ${child} target"
done

TEMP_CONFIG="$TEST_ROOT/capture-worldserver.conf"
cp "$CONFIG" "$TEMP_CONFIG"
detour_chase_patch_config_data_dir \
  "$TEMP_CONFIG" "$DETOUR_FIXTURE_PRIVATE_DATA_DIR"
assert_eq "${DETOUR_FIXTURE_PRIVATE_DATA_DIR}/" \
  "$(detour_chase_read_config_value "$TEMP_CONFIG" DataDir)" \
  "temporary config DataDir"

for args_case in separate equals absent; do
  PM2_FILE="$TEST_ROOT/pm2-${args_case}.json"
  case "$args_case" in
    separate)
      jq -n '{apps:[{args:["-c","/old/worldserver.conf","--flag"]}]}' >"$PM2_FILE"
      ;;
    equals)
      jq -n '{apps:[{args:["--config=/old/worldserver.conf"]}]}' >"$PM2_FILE"
      ;;
    absent)
      jq -n '{apps:[{args:["--flag"]}]}' >"$PM2_FILE"
      ;;
  esac
  chmod 600 "$PM2_FILE"
  detour_chase_patch_rust_pm2_capture_config "$PM2_FILE" "$TEMP_CONFIG"
  jq -e --arg config "$TEMP_CONFIG" '
    .apps[0].args as $args
    | ([range(0; $args | length) as $index
        | select(
            ($args[$index] == "--config=" + $config)
            or ($index > 0 and $args[$index] == $config
              and ($args[$index - 1] == "-c"
                or $args[$index - 1] == "--config"))
          )] | length) == 1
  ' "$PM2_FILE" >/dev/null || fail "PM2 config rewrite failed for ${args_case}"
done

REPORT="$TEST_ROOT/detour-report.json"
jq -n '{
  detour_chase_capture:true,
  loot_race_smoke:false,
  loot_item_capture:false,
  results:[{
    account:"TESTBOT2@bot.local",
    account_id:9,
    character_guid:15,
    world_auth:true,
    enum_characters:true,
    player_login_verified:true,
    detour_chase_capture:true,
    detour_chase_capture_passed:true,
    detour_chase_target_entry:15271,
    detour_chase_target_spawn_guid:9102401,
    detour_chase_target_runtime_counter:9102401,
    detour_chase_target_discovered:true,
    detour_chase_active_mover_ack_sent:true,
    detour_chase_attack_start_confirmed:true,
    detour_chase_first_swing_confirmed:true,
    detour_chase_prewindow_target_moves:0,
    detour_chase_heartbeat_sent:true,
    detour_chase_heartbeat_sha256:
      "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    detour_chase_window_target_moves:1,
    detour_chase_monster_move_sha256:
      "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    detour_chase_monster_move_bytes:128,
    detour_chase_ping_serial:1380930628,
    detour_chase_pong_confirmed:true,
    detour_chase_logout_confirmed:true,
    detour_chase_failure:null
  }]
}' >"$REPORT"
detour_chase_report_proves_exact_success "$REPORT" \
  || fail "exact detour report should pass"
jq '.results[0].detour_chase_window_target_moves=2' "$REPORT" \
  >"$TEST_ROOT/bad-report.json"
if detour_chase_report_proves_exact_success "$TEST_ROOT/bad-report.json"; then
  fail "two target moves must fail the report contract"
fi

validate_fresh_loot_fixture_journal
DETOUR_FIXTURE_SIDE=rust
DETOUR_FIXTURE_NORMAL_DATA_DIR="$NORMAL_DATA"
DETOUR_FIXTURE_MANIFEST_SHA256="$(
  detour_chase_sha256_of_file "$DETOUR_FIXTURE_MANIFEST"
)"
DETOUR_FIXTURE_PRIOR_CREATURE_EXISTS=0
DETOUR_FIXTURE_PRIOR_CREATURE_SHA256=""
DETOUR_FIXTURE_CREATURE_RESTORE_SQL=""
DETOUR_FIXTURE_FIXTURE_CREATURE_SHA256="$(
  printf fixture-creature | sha256sum | awk '{print $1}'
)"
DETOUR_FIXTURE_CHARACTER_IDENTITY_SHA256="$(
  printf character-identity | sha256sum | awk '{print $1}'
)"
DETOUR_FIXTURE_CHARACTER_STABLE_SHA256="$(
  printf character-stable | sha256sum | awk '{print $1}'
)"
DETOUR_FIXTURE_PRIOR_CHARACTER_SHA256="$(
  printf character-prior | sha256sum | awk '{print $1}'
)"
DETOUR_FIXTURE_CHARACTER_RESTORE_SQL="UPDATE characters SET map='0' WHERE guid=15;"
DETOUR_FIXTURE_CHARACTER_AUX_SNAPSHOTS_JSON="$(
  detour_chase_expected_character_auxiliary_scopes_json \
    | jq -c 'map(. + {
        prior_sha256:"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        restore_sql:"",
        predicate_sql:"((SELECT COUNT(*) FROM fixture_scope)=0)",
        post_sha256:null,
        post_predicate_sql:null
      })'
)"
DETOUR_FIXTURE_RESPAWN_SNAPSHOT_JSON="$(
  jq -cn '{
    table:"respawn",
    scope_column:"spawnId",
    scope_value:9102401,
    prior_sha256:"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    restore_sql:"",
    predicate_sql:"((SELECT COUNT(*) FROM respawn)=0)",
    post_sha256:null,
    post_predicate_sql:null
  }'
)"
DETOUR_FIXTURE_WORLD_AUX_SHA256="$(
  printf '0#0#0#0#0#0#0#0#0#0#0' | sha256sum | awk '{print $1}'
)"
DETOUR_FIXTURE_BNET_ACCOUNT_ID=1
DETOUR_FIXTURE_ACCOUNT_SNAPSHOTS_JSON="$(
  jq -cn '
    def snapshot($database;$strategy;$table;$column;$value):
      {
        database:$database,
        strategy:$strategy,
        table:$table,
        scope_column:$column,
        scope_value:$value,
        prior_sha256:
          "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        restore_sql:"",
        predicate_sql:"((SELECT COUNT(*) FROM fixture_scope)=0)",
        post_sha256:null,
        post_predicate_sql:null
      };
    [
      snapshot("auth";"update";"account";"id";9),
      snapshot("auth";"update";"battlenet_accounts";"id";1),
      snapshot("auth";"delete_insert";
        "battlenet_account_toys";"accountId";1),
      snapshot("auth";"delete_insert";
        "battlenet_account_heirlooms";"accountId";1),
      snapshot("auth";"delete_insert";
        "battlenet_account_mounts";"battlenetAccountId";1),
      snapshot("auth";"delete_insert";
        "battlenet_account_transmog_illusions";"battlenetAccountId";1),
      snapshot("auth";"delete_insert";
        "battlenet_item_appearances";"battlenetAccountId";1),
      snapshot("auth";"delete_insert";
        "battlenet_item_favorite_appearances";"battlenetAccountId";1),
      snapshot("auth";"delete_insert";
        "battle_pet_slots";"battlenetAccountId";1),
      snapshot("auth";"delete_insert";
        "account_last_played_character";"accountId";9),
      snapshot("auth";"delete_insert";"realmcharacters";"acctid";9),
      snapshot("characters";"delete_insert";"account_data";"accountId";9),
      snapshot("characters";"delete_insert";
        "account_instance_times";"accountId";9),
      snapshot("characters";"delete_insert";"account_tutorial";"accountId";9)
    ]
  '
)"
DETOUR_FIXTURE_DATABASE_SNAPSHOT_SHA256="$(
  detour_chase_compute_database_snapshot_sha256
)"
RESTORE_PM2="$TEST_ROOT/restore-pm2.json"
CAPTURE_PM2="$TEST_ROOT/capture-pm2.json"
jq -n '{apps:[{name:"rustycore-world",script:"/bin/true",args:[]}]}' \
  >"$RESTORE_PM2"
jq -n '{apps:[{name:"rustycore-world",script:"/bin/true",args:[]}]}' \
  >"$CAPTURE_PM2"
chmod 600 "$RESTORE_PM2" "$CAPTURE_PM2" "$TEMP_CONFIG"
DETOUR_FIXTURE_DB_CONF="$(realpath -e -- "$CONFIG")"
DETOUR_FIXTURE_DB_CONF_SHA256="$(detour_chase_sha256_of_file "$CONFIG")"
DETOUR_FIXTURE_DB_CONF_IDENTITY="$(stat -c '%d:%i' -- "$CONFIG")"
DETOUR_FIXTURE_ORCHESTRATION_LOCK="$TEST_ROOT/capture.lock.d"
DETOUR_FIXTURE_PM2_RUST_WORLD=rustycore-world
DETOUR_FIXTURE_PM2_CPP_WORLD=cpp-world
DETOUR_FIXTURE_WORLD_PORT=8085
DETOUR_FIXTURE_INSTANCE_PORT=8086
DETOUR_FIXTURE_PM2_RESTORE_FILE="$RESTORE_PM2"
DETOUR_FIXTURE_PM2_RESTORE_FILE_SHA256="$(
  detour_chase_sha256_of_file "$RESTORE_PM2"
)"
DETOUR_FIXTURE_PM2_RESTORE_FILE_IDENTITY="$(
  stat -c '%d:%i' -- "$RESTORE_PM2"
)"
DETOUR_FIXTURE_NORMAL_RUST_PM2_PROFILE_SHA256="$(
  printf normal-profile | sha256sum | awk '{print $1}'
)"
DETOUR_FIXTURE_NORMAL_RUST_CONFIG="$CONFIG"
DETOUR_FIXTURE_NORMAL_RUST_CONFIG_SHA256="$(
  detour_chase_sha256_of_file "$CONFIG"
)"
DETOUR_FIXTURE_NORMAL_RUST_CONFIG_IDENTITY="$(stat -c '%d:%i' -- "$CONFIG")"
DETOUR_FIXTURE_CAPTURE_CONFIG_FILE="$CAPTURE_PM2"
DETOUR_FIXTURE_CAPTURE_CONFIG_FILE_SHA256="$(
  detour_chase_sha256_of_file "$CAPTURE_PM2"
)"
DETOUR_FIXTURE_CAPTURE_CONFIG_FILE_IDENTITY="$(
  stat -c '%d:%i' -- "$CAPTURE_PM2"
)"
DETOUR_FIXTURE_RUST_CONFIG="$TEMP_CONFIG"
DETOUR_FIXTURE_RUST_CONFIG_SHA256="$(
  detour_chase_sha256_of_file "$TEMP_CONFIG"
)"
DETOUR_FIXTURE_RUST_CONFIG_IDENTITY="$(stat -c '%d:%i' -- "$TEMP_CONFIG")"
DETOUR_FIXTURE_DB_APPLIED=1
detour_chase_write_fixture_journal create
assert_eq 600 "$(stat -c '%a' "$WOW_BOT_FIXTURE_JOURNAL")" "journal mode"
EXPECTED_NORMAL_CONFIG="$DETOUR_FIXTURE_NORMAL_RUST_CONFIG"
EXPECTED_DB_CONF_SHA="$DETOUR_FIXTURE_DB_CONF_SHA256"
DETOUR_FIXTURE_SIDE=""
DETOUR_FIXTURE_DB_CONF=""
DETOUR_FIXTURE_DB_CONF_SHA256=""
DETOUR_FIXTURE_DB_CONF_IDENTITY=""
DETOUR_FIXTURE_NORMAL_RUST_CONFIG=""
DETOUR_FIXTURE_NORMAL_RUST_CONFIG_SHA256=""
DETOUR_FIXTURE_NORMAL_RUST_CONFIG_IDENTITY=""
DETOUR_FIXTURE_ACCOUNT_SNAPSHOTS_JSON="[]"
DETOUR_FIXTURE_CHARACTER_AUX_SNAPSHOTS_JSON="[]"
DETOUR_FIXTURE_RESPAWN_SNAPSHOT_JSON="{}"
detour_chase_load_fixture_journal
assert_eq rust "$DETOUR_FIXTURE_SIDE" "fresh-shell side restoration"
assert_eq "$EXPECTED_NORMAL_CONFIG" "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG" \
  "fresh-shell normal config restoration"
assert_eq "$EXPECTED_DB_CONF_SHA" "$DETOUR_FIXTURE_DB_CONF_SHA256" \
  "fresh-shell DB config provenance restoration"
if detour_chase_complete_fixture_journal; then
  fail "armed journal completed before DB/filesystem/runtime recovery"
fi
DETOUR_FIXTURE_POST_CHARACTER_SHA256="$DETOUR_FIXTURE_PRIOR_CHARACTER_SHA256"
DETOUR_FIXTURE_POST_CHARACTER_PREDICATE_SQL="(1=1)"
DETOUR_FIXTURE_CHARACTER_AUX_SNAPSHOTS_JSON="$(
  jq -c 'map(
    .post_sha256=.prior_sha256
    | .post_predicate_sql=.predicate_sql
  )' <<<"$DETOUR_FIXTURE_CHARACTER_AUX_SNAPSHOTS_JSON"
)"
DETOUR_FIXTURE_RESPAWN_SNAPSHOT_JSON="$(
  jq -c '
    .post_sha256=.prior_sha256
    | .post_predicate_sql=.predicate_sql
  ' <<<"$DETOUR_FIXTURE_RESPAWN_SNAPSHOT_JSON"
)"
DETOUR_FIXTURE_ACCOUNT_SNAPSHOTS_JSON="$(
  jq -c 'map(
    .post_sha256=.prior_sha256
    | .post_predicate_sql=.predicate_sql
  )' <<<"$DETOUR_FIXTURE_ACCOUNT_SNAPSHOTS_JSON"
)"
DETOUR_FIXTURE_POST_WORLD_AUX_SHA256="$DETOUR_FIXTURE_WORLD_AUX_SHA256"
DETOUR_FIXTURE_POSTSTATE_CHECKPOINTED=1
detour_chase_write_fixture_journal replace
detour_chase_mark_db_restored
detour_chase_remove_rust_capture_config
detour_chase_remove_rust_pm2_capture_file
detour_chase_remove_private_data_dir
detour_chase_mark_filesystem_restored
detour_chase_mark_normal_runtime_restored
detour_chase_complete_fixture_journal
detour_chase_complete_fixture_journal \
  || fail "journal completion must be idempotent"
loot_fixture_bot_cleanup_complete
[ ! -e "$WOW_BOT_FIXTURE_JOURNAL" ] || fail "completed journal remains present"
[ -f "$LOOT_FIXTURE_CLEANUP_MARKER" ] || fail "cleanup marker missing"

detour_chase_remove_private_data_dir
[ ! -e "$DETOUR_FIXTURE_PRIVATE_DATA_DIR" ] \
  || fail "private DataDir remains after cleanup"
detour_chase_remove_private_data_dir \
  || fail "private DataDir cleanup must be idempotent"

BOT_EXEC="$TEST_ROOT/wow-test-bot"
cp -- /bin/true "$BOT_EXEC"
chmod 700 "$BOT_EXEC"
DETOUR_FIXTURE_BOT_EXEC="$BOT_EXEC"
DETOUR_FIXTURE_BOT_EXEC_SHA256="$(detour_chase_sha256_of_file "$BOT_EXEC")"
DETOUR_FIXTURE_BOT_REPORT="$REPORT"
DETOUR_FIXTURE_CLEANUP_VERIFIED=1
EVIDENCE="$(detour_chase_capture_evidence)" \
  || fail "detour evidence generation failed"
jq -e \
  --arg normal_data "$NORMAL_DATA" \
  --arg private_data "$DETOUR_FIXTURE_PRIVATE_DATA_DIR" \
  --arg fixture_manifest "$DETOUR_FIXTURE_MANIFEST" \
  --arg fixture_manifest_sha "$DETOUR_FIXTURE_MANIFEST_SHA256" \
  --arg journal_sha "$DETOUR_FIXTURE_JOURNAL_SHA256" \
  --arg database_snapshot_sha "$DETOUR_FIXTURE_DATABASE_SNAPSHOT_SHA256" \
  --arg bot_exec "$BOT_EXEC" \
  --arg bot_exec_sha "$DETOUR_FIXTURE_BOT_EXEC_SHA256" \
  --arg report "$REPORT" '
    .fixture_guard.enabled == true
    and .fixture_guard.contract
      == "detour-chase-around-obstacle-shell-fixture-v1"
    and .fixture_guard.account == "TESTBOT2@bot.local"
    and .fixture_guard.account_id == 9
    and .fixture_guard.character_guid == 15
    and .fixture_guard.character_account_id == 9
    and .fixture_guard.creature_entry == 15271
    and .fixture_guard.creature_spawn_guid == 9102401
    and .fixture_guard.normal_data_dir == $normal_data
    and .fixture_guard.private_data_dir == $private_data
    and .fixture_guard.private_data_dir_removed_before_normal_runtime == true
    and .fixture_guard.fixture_manifest_path == $fixture_manifest
    and .fixture_guard.fixture_manifest_sha256 == $fixture_manifest_sha
    and .fixture_guard.journal_sha256 == $journal_sha
    and (.fixture_guard.database_snapshot_sha256
      | test("^[0-9a-f]{64}$"))
    and .fixture_guard.database_snapshot_sha256 == $database_snapshot_sha
    and .fixture_guard.cleanup_verified == true
    and (.fixture_guard.synthetic_mmaps | length) == 2
    and (.fixture_guard.linked_read_only_data | length) == 5
    and .bot_report.contract
      == "wow-test-bot-detour-chase-capture-report-v1"
    and .bot_report.exec_path == $bot_exec
    and .bot_report.exec_sha256 == $bot_exec_sha
    and .bot_report.report_path == $report
    and .bot_report.report_validated == true
  ' <<<"$EVIDENCE" >/dev/null || fail "detour evidence schema is incomplete"

rm -- "$LOOT_FIXTURE_CLEANUP_MARKER"

prepare_unarmed_recovery_case() {
  validate_fresh_loot_fixture_journal
  DETOUR_FIXTURE_ENABLED=1
  DETOUR_FIXTURE_SIDE=rust
  DETOUR_FIXTURE_DB_APPLIED=0
  DETOUR_CAPTURE_PRIVATE_PARENT="$TEST_ROOT"
  DETOUR_FIXTURE_PRIVATE_DATA_DIR=""
  DETOUR_FIXTURE_PRIVATE_DATA_DIR_IDENTITY=""
  detour_chase_prepare_private_data_dir
  detour_chase_create_rust_capture_config \
    "$CONFIG" "$DETOUR_FIXTURE_PRIVATE_DATA_DIR"

  DETOUR_FIXTURE_PM2_RESTORE_FILE="$(
    mktemp "$TEST_ROOT/recovery-restore.XXXXXX.json"
  )"
  DETOUR_FIXTURE_CAPTURE_CONFIG_FILE="$(
    mktemp "$TEST_ROOT/recovery-capture.XXXXXX.json"
  )"
  jq -n '{apps:[{name:"rustycore-world",script:"/bin/true",args:[]}]}' \
    >"$DETOUR_FIXTURE_PM2_RESTORE_FILE"
  jq -n '{apps:[{name:"rustycore-world",script:"/bin/true",args:[]}]}' \
    >"$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE"
  chmod 600 \
    "$DETOUR_FIXTURE_PM2_RESTORE_FILE" \
    "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE"
  DETOUR_FIXTURE_PM2_RESTORE_FILE_SHA256="$(
    detour_chase_sha256_of_file "$DETOUR_FIXTURE_PM2_RESTORE_FILE"
  )"
  DETOUR_FIXTURE_PM2_RESTORE_FILE_IDENTITY="$(
    stat -c '%d:%i' -- "$DETOUR_FIXTURE_PM2_RESTORE_FILE"
  )"
  DETOUR_FIXTURE_CAPTURE_CONFIG_FILE_SHA256="$(
    detour_chase_sha256_of_file "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE"
  )"
  DETOUR_FIXTURE_CAPTURE_CONFIG_FILE_IDENTITY="$(
    stat -c '%d:%i' -- "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE"
  )"
  DETOUR_FIXTURE_DB_CONF="$CONFIG"
  DETOUR_FIXTURE_DB_CONF_SHA256="$(detour_chase_sha256_of_file "$CONFIG")"
  DETOUR_FIXTURE_DB_CONF_IDENTITY="$(stat -c '%d:%i' -- "$CONFIG")"
  DETOUR_FIXTURE_ORCHESTRATION_LOCK="$TEST_ROOT/capture.lock.d"
  DETOUR_FIXTURE_PM2_RUST_WORLD=rustycore-world
  DETOUR_FIXTURE_PM2_CPP_WORLD=cpp-world
  DETOUR_FIXTURE_WORLD_PORT=8085
  DETOUR_FIXTURE_INSTANCE_PORT=8086
  DETOUR_FIXTURE_NORMAL_RUST_PM2_PROFILE_SHA256="$(
    printf normal-profile | sha256sum | awk '{print $1}'
  )"
  DETOUR_FIXTURE_NORMAL_RUST_CONFIG="$CONFIG"
  DETOUR_FIXTURE_NORMAL_RUST_CONFIG_SHA256="$(
    detour_chase_sha256_of_file "$CONFIG"
  )"
  DETOUR_FIXTURE_NORMAL_RUST_CONFIG_IDENTITY="$(
    stat -c '%d:%i' -- "$CONFIG"
  )"
  DETOUR_FIXTURE_CPP_CONFIG=""
  DETOUR_FIXTURE_CPP_CONFIG_BACKUP=""
  DETOUR_FIXTURE_CPP_CONFIG_BACKUP_IDENTITY=""
  DETOUR_FIXTURE_CPP_CONFIG_BACKUP_SHA256=""
  detour_chase_arm_filesystem_recovery_journal
  CASE_RESTORE_FILE="$DETOUR_FIXTURE_PM2_RESTORE_FILE"
}

prepare_early_recovery_case() {
  validate_fresh_loot_fixture_journal
  DETOUR_FIXTURE_ENABLED=1
  DETOUR_FIXTURE_SIDE=rust
  DETOUR_FIXTURE_DB_APPLIED=0
  DETOUR_CAPTURE_PRIVATE_PARENT="$TEST_ROOT"
  DETOUR_FIXTURE_PRIVATE_DATA_DIR=""
  DETOUR_FIXTURE_PRIVATE_DATA_DIR_IDENTITY=""
  detour_chase_allocate_private_data_dir
  DETOUR_FIXTURE_PM2_RESTORE_FILE="$(
    mktemp "$TEST_ROOT/early-restore.XXXXXX.json"
  )"
  DETOUR_FIXTURE_CAPTURE_CONFIG_FILE="$(
    mktemp "$TEST_ROOT/early-capture.XXXXXX.json"
  )"
  chmod 600 \
    "$DETOUR_FIXTURE_PM2_RESTORE_FILE" \
    "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE"
  DETOUR_FIXTURE_PM2_RESTORE_FILE_SHA256="$(
    detour_chase_sha256_of_file "$DETOUR_FIXTURE_PM2_RESTORE_FILE"
  )"
  DETOUR_FIXTURE_PM2_RESTORE_FILE_IDENTITY="$(
    stat -c '%d:%i' -- "$DETOUR_FIXTURE_PM2_RESTORE_FILE"
  )"
  DETOUR_FIXTURE_CAPTURE_CONFIG_FILE_SHA256="$(
    detour_chase_sha256_of_file "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE"
  )"
  DETOUR_FIXTURE_CAPTURE_CONFIG_FILE_IDENTITY="$(
    stat -c '%d:%i' -- "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE"
  )"
  DETOUR_FIXTURE_DB_CONF="$CONFIG"
  DETOUR_FIXTURE_DB_CONF_SHA256="$(detour_chase_sha256_of_file "$CONFIG")"
  DETOUR_FIXTURE_DB_CONF_IDENTITY="$(stat -c '%d:%i' -- "$CONFIG")"
  DETOUR_FIXTURE_ORCHESTRATION_LOCK="$TEST_ROOT/capture.lock.d"
  DETOUR_FIXTURE_PM2_RUST_WORLD=rustycore-world
  DETOUR_FIXTURE_PM2_CPP_WORLD=cpp-world
  DETOUR_FIXTURE_WORLD_PORT=8085
  DETOUR_FIXTURE_INSTANCE_PORT=8086
  DETOUR_FIXTURE_NORMAL_RUST_PM2_PROFILE_SHA256="$(
    printf normal-profile | sha256sum | awk '{print $1}'
  )"
  DETOUR_FIXTURE_NORMAL_RUST_CONFIG="$CONFIG"
  DETOUR_FIXTURE_NORMAL_RUST_CONFIG_SHA256="$(
    detour_chase_sha256_of_file "$CONFIG"
  )"
  DETOUR_FIXTURE_NORMAL_RUST_CONFIG_IDENTITY="$(
    stat -c '%d:%i' -- "$CONFIG"
  )"
  DETOUR_FIXTURE_RUST_CONFIG=""
  DETOUR_FIXTURE_RUST_CONFIG_SHA256=""
  DETOUR_FIXTURE_RUST_CONFIG_IDENTITY=""
  detour_chase_arm_filesystem_recovery_journal
  CASE_RESTORE_FILE="$DETOUR_FIXTURE_PM2_RESTORE_FILE"
}

load_recovery_case_as_fresh_shell() {
  DETOUR_FIXTURE_SIDE=""
  DETOUR_FIXTURE_NORMAL_DATA_DIR=""
  DETOUR_FIXTURE_PRIVATE_DATA_DIR=""
  DETOUR_FIXTURE_PRIVATE_DATA_DIR_IDENTITY=""
  DETOUR_FIXTURE_DB_CONF=""
  DETOUR_FIXTURE_DB_CONF_SHA256=""
  DETOUR_FIXTURE_DB_CONF_IDENTITY=""
  DETOUR_FIXTURE_NORMAL_RUST_CONFIG=""
  DETOUR_FIXTURE_NORMAL_RUST_CONFIG_SHA256=""
  DETOUR_FIXTURE_NORMAL_RUST_CONFIG_IDENTITY=""
  DETOUR_FIXTURE_RUST_CONFIG=""
  DETOUR_FIXTURE_RUST_CONFIG_SHA256=""
  DETOUR_FIXTURE_RUST_CONFIG_IDENTITY=""
  DETOUR_FIXTURE_PM2_RESTORE_FILE=""
  DETOUR_FIXTURE_CAPTURE_CONFIG_FILE=""
  DETOUR_FIXTURE_DB_APPLIED=1
  DETOUR_FIXTURE_FILESYSTEM_RESTORED=1
  DETOUR_FIXTURE_NORMAL_RUNTIME_RESTORED=1
  detour_chase_load_fixture_journal \
    || fail "fresh-shell recovery journal load failed"
}

RECOVERY_LOG=""
MOCK_NORMAL_READY=0
detour_chase_recovery_runtime_is_normal() {
  [ "$MOCK_NORMAL_READY" = 1 ]
}
detour_chase_recovery_stop_capture_runtime() {
  RECOVERY_LOG="${RECOVERY_LOG:+${RECOVERY_LOG},}stop"
}
detour_chase_recovery_restore_filesystem() {
  RECOVERY_LOG="${RECOVERY_LOG:+${RECOVERY_LOG},}filesystem"
  if [ -z "$DETOUR_FIXTURE_RUST_CONFIG" ]; then
    detour_chase_discard_uncheckpointed_rust_artifacts
  else
    detour_chase_discard_unarmed_private_data_dir \
      && detour_chase_remove_rust_capture_config \
      && detour_chase_remove_rust_pm2_capture_file
  fi
}
detour_chase_recovery_start_normal_runtime() {
  RECOVERY_LOG="${RECOVERY_LOG:+${RECOVERY_LOG},}start"
  MOCK_NORMAL_READY=1
}

# Crash while credential/config snapshots are only partially written. Their
# bytes may differ from the initial hash, but their inode and private root are
# already journaled, so fresh-shell cleanup remains deterministic.
prepare_early_recovery_case
printf 'DatabaseInfo = "secret-partial"\n' \
  >"$DETOUR_FIXTURE_PRIVATE_DATA_DIR/partial-worldserver.conf"
printf partial-restore >"$DETOUR_FIXTURE_PM2_RESTORE_FILE"
printf partial-capture >"$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE"
load_recovery_case_as_fresh_shell
RECOVERY_LOG=""
MOCK_NORMAL_READY=1
detour_chase_run_recovery_state_machine
assert_eq filesystem "$RECOVERY_LOG" "early pre-journal-upgrade recovery"
[ ! -e "$CASE_RESTORE_FILE" ] || fail "partial PM2 restore file remains"
rm -- "$LOOT_FIXTURE_CLEANUP_MARKER"

# Crash immediately after the durable recovery journal is armed.
prepare_unarmed_recovery_case
load_recovery_case_as_fresh_shell
assert_eq 0 "$DETOUR_FIXTURE_DB_APPLIED" "unarmed journal DB state"
RECOVERY_LOG=""
MOCK_NORMAL_READY=0
detour_chase_run_recovery_state_machine
assert_eq "stop,filesystem,start" "$RECOVERY_LOG" \
  "fresh-shell full recovery ordering"
[ ! -e "$WOW_BOT_FIXTURE_JOURNAL" ] || fail "recovered journal remains"
loot_fixture_bot_cleanup_complete
rm -- "$LOOT_FIXTURE_CLEANUP_MARKER" "$CASE_RESTORE_FILE"

# Crash after filesystem restoration is durably marked but before normal start.
prepare_unarmed_recovery_case
RECOVERY_LOG=""
detour_chase_recovery_restore_filesystem
detour_chase_mark_filesystem_restored
load_recovery_case_as_fresh_shell
RECOVERY_LOG=""
MOCK_NORMAL_READY=0
detour_chase_run_recovery_state_machine
assert_eq start "$RECOVERY_LOG" "filesystem-restored crash resume"
rm -- "$LOOT_FIXTURE_CLEANUP_MARKER" "$CASE_RESTORE_FILE"

# Crash after normal runtime accreditation but before journal consumption.
prepare_unarmed_recovery_case
RECOVERY_LOG=""
detour_chase_recovery_restore_filesystem
detour_chase_mark_filesystem_restored
MOCK_NORMAL_READY=1
detour_chase_mark_normal_runtime_restored
load_recovery_case_as_fresh_shell
RECOVERY_LOG=""
MOCK_NORMAL_READY=1
detour_chase_run_recovery_state_machine
assert_eq "" "$RECOVERY_LOG" "normal-restored crash resume"
rm -- "$LOOT_FIXTURE_CLEANUP_MARKER" "$CASE_RESTORE_FILE"

# DB-free adversarial CAS coverage: exact checkpointed poststate may restore;
# any third state fails closed before SQL and leaves its journal untouched.
(
  CAS_STATE="$TEST_ROOT/cas-state"
  CAS_EXECUTED="$TEST_ROOT/cas-executed"
  CAS_JOURNAL="$TEST_ROOT/cas-journal"
  PRIOR_SQL='UPDATE `account` SET `username`=FROM_BASE64('"'"'UFJJT1I='"'"') WHERE `id`=9'
  POST_SQL='UPDATE `account` SET `username`=FROM_BASE64('"'"'UE9TVA=='"'"') WHERE `id`=9'
  printf '%s' "$POST_SQL" >"$CAS_STATE"
  printf retained >"$CAS_JOURNAL"
  WOW_BOT_FIXTURE_JOURNAL="$CAS_JOURNAL"
  DETOUR_FIXTURE_POSTSTATE_CHECKPOINTED=1
  DETOUR_FIXTURE_ACCOUNT_SNAPSHOTS_JSON="$(
    jq -cn \
      --arg prior_sha "$(detour_chase_sha256_of_text "$PRIOR_SQL")" \
      --arg post_sha "$(detour_chase_sha256_of_text "$POST_SQL")" \
      --arg restore_sql "$PRIOR_SQL" '[
        {
          database:"auth",
          strategy:"update",
          table:"account",
          scope_column:"id",
          scope_value:9,
          prior_sha256:$prior_sha,
          restore_sql:$restore_sql,
          predicate_sql:"(prior predicate)",
          post_sha256:$post_sha,
          post_predicate_sql:"(`id`=9 AND `username` <=> FROM_BASE64('\''UE9TVA=='\''))"
        }
      ]'
  )"
  detour_chase_account_restore_guard() {
    printf '%s\n' '`id`=9'
  }
  detour_chase_snapshot_single_row_update_sql() {
    cat "$CAS_STATE"
  }
  detour_chase_execute_recovery_sql() {
    local database="$1"
    local sql="$2"
    [ "$database" = auth ] \
      && [[ "$sql" == *'SET autocommit=0;'* ]] \
      && [[ "$sql" == *'LOCK TABLES `account` WRITE;'* ]] \
      && [[ "$sql" != *'START TRANSACTION;'* ]] \
      && [[ "$sql" == *'@detour_cas=IF('* ]] || return 1
    : >"$CAS_EXECUTED"
    printf '%s' "$PRIOR_SQL" >"$CAS_STATE"
    printf '1\n'
  }
  detour_chase_restore_account_state \
    || fail "exact checkpointed account poststate did not restore"
  assert_eq "$PRIOR_SQL" "$(cat "$CAS_STATE")" "CAS exact prior restore"
  [ -e "$CAS_EXECUTED" ] || fail "exact poststate did not execute CAS SQL"

  rm -- "$CAS_EXECUTED"
  printf external-drift >"$CAS_STATE"
  if detour_chase_restore_account_state; then
    fail "third-state account drift was overwritten"
  fi
  [ ! -e "$CAS_EXECUTED" ] || fail "drift reached recovery SQL"
  assert_eq retained "$(cat "$CAS_JOURNAL")" "drift journal retention"
)

echo "detour chase fixture common: PASS"
