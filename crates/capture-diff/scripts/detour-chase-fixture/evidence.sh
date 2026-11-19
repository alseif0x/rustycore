detour_chase_report_proves_exact_success() {
  local report_path="$1"
  local ping_serial
  ping_serial="$(detour_chase_ping_fence_serial)" || return 1
  jq -e --argjson ping_serial "$ping_serial" '
    .detour_chase_capture == true
    and .loot_race_smoke == false
    and .loot_item_capture == false
    and (.results | type == "array" and length == 1)
    and .results[0].account == "TESTBOT2@bot.local"
    and .results[0].account_id == 9
    and .results[0].character_guid == 15
    and .results[0].world_auth == true
    and .results[0].enum_characters == true
    and .results[0].player_login_verified == true
    and .results[0].detour_chase_capture == true
    and .results[0].detour_chase_capture_passed == true
    and .results[0].detour_chase_target_entry == 15271
    and .results[0].detour_chase_target_spawn_guid == 9102401
    and (.results[0].detour_chase_target_runtime_counter
      | type == "number" and . > 0)
    and .results[0].detour_chase_target_discovered == true
    and .results[0].detour_chase_active_mover_ack_sent == true
    and .results[0].detour_chase_attack_start_confirmed == true
    and .results[0].detour_chase_first_swing_confirmed == true
    and .results[0].detour_chase_prewindow_target_moves == 0
    and .results[0].detour_chase_heartbeat_sent == true
    and (.results[0].detour_chase_heartbeat_sha256
      | type == "string" and test("^[0-9a-f]{64}$"))
    and .results[0].detour_chase_window_target_moves == 1
    and (.results[0].detour_chase_monster_move_sha256
      | type == "string" and test("^[0-9a-f]{64}$"))
    and (.results[0].detour_chase_monster_move_bytes
      | type == "number" and . > 0)
    and .results[0].detour_chase_ping_serial == $ping_serial
    and .results[0].detour_chase_pong_confirmed == true
    and .results[0].detour_chase_logout_confirmed == true
    and .results[0].detour_chase_failure == null
  ' "$report_path" >/dev/null
}

detour_chase_bot_evidence() {
  local canonical_report canonical_exec report_sha report_sha_after
  local bot_sha bot_sha_after

  canonical_report="$(realpath -e -- "$DETOUR_FIXTURE_BOT_REPORT" 2>/dev/null)" \
    || return 1
  canonical_exec="$(realpath -e -- "$DETOUR_FIXTURE_BOT_EXEC" 2>/dev/null)" \
    || return 1
  [ "$canonical_report" = "$DETOUR_FIXTURE_BOT_REPORT" ] \
    && [ -f "$canonical_report" ] && [ ! -L "$canonical_report" ] \
    && [ "$canonical_exec" = "$DETOUR_FIXTURE_BOT_EXEC" ] \
    && [ -f "$canonical_exec" ] && [ -x "$canonical_exec" ] \
    && [ ! -L "$canonical_exec" ] || return 1
  bot_sha="$(detour_chase_sha256_of_file "$canonical_exec")" || return 1
  [ "$bot_sha" = "$DETOUR_FIXTURE_BOT_EXEC_SHA256" ] || return 1
  report_sha="$(detour_chase_sha256_of_file "$canonical_report")" || return 1
  detour_chase_report_proves_exact_success "$canonical_report" || return 1
  report_sha_after="$(detour_chase_sha256_of_file "$canonical_report")" \
    || return 1
  bot_sha_after="$(detour_chase_sha256_of_file "$canonical_exec")" || return 1
  [ "$report_sha_after" = "$report_sha" ] \
    && [ "$bot_sha_after" = "$bot_sha" ] || return 1
  DETOUR_FIXTURE_BOT_REPORT_SHA256="$report_sha"
  printf '%s\t%s\t%s\t%s\n' \
    "$canonical_exec" "$bot_sha" "$canonical_report" "$report_sha"
}

detour_chase_capture_evidence() {
  local bot_evidence exec_path exec_sha report_path report_sha

  [ "$DETOUR_FIXTURE_ENABLED" = "1" ] \
    && [ "$DETOUR_FIXTURE_CLEANUP_VERIFIED" = 1 ] \
    && [ -n "$DETOUR_FIXTURE_JOURNAL_SHA256" ] \
    && [ -n "$DETOUR_FIXTURE_PRIVATE_DATA_DIR" ] \
    && [ ! -e "$DETOUR_FIXTURE_PRIVATE_DATA_DIR" ] \
    && [ ! -L "$DETOUR_FIXTURE_PRIVATE_DATA_DIR" ] || return 1
  bot_evidence="$(detour_chase_bot_evidence)" || return 1
  IFS=$'\t' read -r exec_path exec_sha report_path report_sha \
    <<<"$bot_evidence"
  jq -n \
    --arg normal_data_dir "$DETOUR_FIXTURE_NORMAL_DATA_DIR" \
    --arg private_data_dir "$DETOUR_FIXTURE_PRIVATE_DATA_DIR" \
    --arg manifest_path "$DETOUR_FIXTURE_MANIFEST" \
    --arg manifest_sha256 "$DETOUR_FIXTURE_MANIFEST_SHA256" \
    --arg map_sha256 "$DETOUR_FIXTURE_MAP_SHA256" \
    --arg tile_sha256 "$DETOUR_FIXTURE_TILE_SHA256" \
    --arg journal_sha256 "$DETOUR_FIXTURE_JOURNAL_SHA256" \
    --arg database_snapshot_sha256 "$DETOUR_FIXTURE_DATABASE_SNAPSHOT_SHA256" \
    --arg dbc_path "$DETOUR_FIXTURE_NORMAL_DATA_DIR/dbc" \
    --arg gt_path "$DETOUR_FIXTURE_NORMAL_DATA_DIR/gt" \
    --arg maps_path "$DETOUR_FIXTURE_NORMAL_DATA_DIR/maps" \
    --arg vmaps_path "$DETOUR_FIXTURE_NORMAL_DATA_DIR/vmaps" \
    --arg cameras_path "$DETOUR_FIXTURE_NORMAL_DATA_DIR/cameras" \
    --arg exec_path "$exec_path" \
    --arg exec_sha256 "$exec_sha" \
    --arg report_path "$report_path" \
    --arg report_sha256 "$report_sha" '
      {
        fixture_guard: {
          enabled: true,
          contract: "detour-chase-around-obstacle-shell-fixture-v1",
          account: "TESTBOT2@bot.local",
          account_id: 9,
          character_guid: 15,
          peer_account: "",
          peer_account_id: 0,
          peer_character_guid: 0,
          creature_entry: 15271,
          creature_spawn_guid: 9102401,
          character_account_id: 9,
          item_entry: 0,
          normal_data_dir: $normal_data_dir,
          private_data_dir: $private_data_dir,
          private_data_dir_removed_before_normal_runtime: true,
          fixture_manifest_path: $manifest_path,
          fixture_manifest_sha256: $manifest_sha256,
          synthetic_mmaps: [
            {
              path: "mmaps/0001.mmap",
              size: 28,
              sha256: $map_sha256
            },
            {
              path: "mmaps/00015026.mmtile",
              size: 1496,
              sha256: $tile_sha256
            }
          ],
          linked_read_only_data: [
            {name:"dbc",target_path:$dbc_path},
            {name:"gt",target_path:$gt_path},
            {name:"maps",target_path:$maps_path},
            {name:"vmaps",target_path:$vmaps_path},
            {name:"cameras",target_path:$cameras_path}
          ],
          journal_sha256: $journal_sha256,
          database_snapshot_sha256: $database_snapshot_sha256,
          cleanup_verified: true
        },
        bot_report: {
          contract: "wow-test-bot-detour-chase-capture-report-v1",
          exec_path: $exec_path,
          exec_sha256: $exec_sha256,
          report_path: $report_path,
          report_sha256: $report_sha256,
          account: "TESTBOT2@bot.local",
          account_id: 9,
          character_guid: 15,
          report_validated: true
        }
      }
    '
}
