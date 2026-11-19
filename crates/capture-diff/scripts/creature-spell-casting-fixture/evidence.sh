creature_spell_fixture_report_proves_exact_success() {
  local report_path="$1"
  jq -e --arg manifest_sha "$CREATURE_SPELL_FIXTURE_MANIFEST_SHA256" '
    .creature_spell_capture == true
    and .detour_chase_capture == false
    and .loot_item_capture == false
    and .loot_race_smoke == false
    and (.results | type == "array" and length == 1)
    and (.results[0]
      | .account == "TESTBOT2@bot.local"
      and .account_id == 9
      and .character_guid == 15
      and .world_auth == true
      and .enum_characters == true
      and .player_login_verified == true
      and .creature_spell_capture == true
      and .creature_spell_capture_passed == true
      and .creature_spell_fixture_manifest_sha256 == $manifest_sha
      and .creature_spell_target_entry == 22378
      and .creature_spell_target_spawn_guid == 78686
      and (.creature_spell_target_runtime_counter | type == "number" and . > 0)
      and .creature_spell_target_discovered == true
      and .creature_spell_heartbeat_sent == true
      and (.creature_spell_heartbeat_sha256 | test("^[0-9a-f]{64}$"))
      and .creature_spell_start_opcode == 11319
      and (.creature_spell_start_body_sha256 | test("^[0-9a-f]{64}$"))
      and (.creature_spell_start_body_bytes | type == "number" and . > 0)
      and .creature_spell_go_opcode == 11318
      and (.creature_spell_go_body_sha256 | test("^[0-9a-f]{64}$"))
      and (.creature_spell_go_body_bytes | type == "number" and . > 0)
      and (.creature_spell_cast_id_low | type == "number" and . >= 0)
      and (.creature_spell_cast_id_high | type == "number" and . >= 0)
      and ((.creature_spell_cast_id_low + .creature_spell_cast_id_high) > 0)
      and .creature_spell_caster_guid_low == .creature_spell_target_runtime_counter
      and (.creature_spell_caster_guid_high | type == "number" and . > 0)
      and .creature_spell_victim_guid_low == 15
      and (.creature_spell_victim_guid_high | type == "number" and . > 0)
      and .creature_spell_spell_id == 15691
      and .creature_spell_start_cast_flags == 2
      and .creature_spell_go_cast_flags == 256
      and .creature_spell_cast_flags_ex == 0
      and .creature_spell_go_hit_target_count == 1
      and .creature_spell_go_miss_target_count == 0
      and .creature_spell_full_combat_log == false
      and .creature_spell_advanced_logging_sent == false
      and .creature_spell_adjacent_start_go == true
      and .creature_spell_disconnect_confirmed == true
      and .creature_spell_logout_confirmed == false
      and .creature_spell_failure == null)
  ' "$report_path" >/dev/null
}

creature_spell_fixture_bot_evidence() {
  local report_path="$1"
  local bot_exec="$2"
  local expected_bot_sha="$3"
  local canonical_report canonical_exec report_sha bot_sha
  [[ "$report_path" = /* && "$bot_exec" = /* \
    && "$expected_bot_sha" =~ ^[0-9a-f]{64}$ ]] || return 1
  canonical_report="$(realpath -e -- "$report_path" 2>/dev/null)" || return 1
  canonical_exec="$(realpath -e -- "$bot_exec" 2>/dev/null)" || return 1
  [ "$canonical_report" = "$report_path" ] \
    && [ -f "$report_path" ] && [ ! -L "$report_path" ] \
    && [ "$canonical_exec" = "$bot_exec" ] \
    && [ -f "$bot_exec" ] && [ -x "$bot_exec" ] && [ ! -L "$bot_exec" ] \
    || return 1
  bot_sha="$(creature_spell_fixture_sha256_of_file "$bot_exec")" || return 1
  [ "$bot_sha" = "$expected_bot_sha" ] || return 1
  creature_spell_fixture_report_proves_exact_success "$report_path" || return 1
  report_sha="$(creature_spell_fixture_sha256_of_file "$report_path")" || return 1
  printf '%s\t%s\t%s\t%s\n' \
    "$canonical_exec" "$bot_sha" "$canonical_report" "$report_sha"
}

creature_spell_fixture_capture_evidence() {
  local bot_exec="$1"
  local bot_exec_sha256="$2"
  local bot_report="$3"
  local bot_report_sha256="$4"
  [ "$CREATURE_SPELL_FIXTURE_CLEANUP_VERIFIED" -eq 1 ] \
    && [ "$CREATURE_SPELL_FIXTURE_GHOST_PREFLIGHT_VERIFIED" -eq 1 ] \
    && [ "$CREATURE_SPELL_FIXTURE_GHOST_POST_CAPTURE_VERIFIED" -eq 1 ] \
    && [[ "$CREATURE_SPELL_FIXTURE_JOURNAL_SHA256" =~ ^[0-9a-f]{64}$ ]] \
    && [[ "$CREATURE_SPELL_FIXTURE_DATABASE_SNAPSHOT_SHA256" =~ ^[0-9a-f]{64}$ ]] \
    && [[ "$CREATURE_SPELL_FIXTURE_MANIFEST_SHA256" =~ ^[0-9a-f]{64}$ ]] \
    && [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256" =~ ^[0-9a-f]{64}$ ]] \
    && [[ "$bot_exec" = /* && "$bot_exec" != *$'\t'* \
      && "$bot_exec" != *$'\n'* ]] \
    && [[ "$bot_report" = /* && "$bot_report" != *$'\t'* \
      && "$bot_report" != *$'\n'* ]] \
    && [[ "$bot_exec_sha256" =~ ^[0-9a-f]{64}$ ]] \
    && [[ "$bot_report_sha256" =~ ^[0-9a-f]{64}$ ]] \
    && creature_spell_fixture_validate_manifest_file \
      "$CREATURE_SPELL_FIXTURE_MANIFEST" \
    && [ "$(creature_spell_fixture_sha256_of_file \
      "$CREATURE_SPELL_FIXTURE_MANIFEST")" \
      = "$CREATURE_SPELL_FIXTURE_MANIFEST_SHA256" ] || return 1
  jq -n \
    --arg contract "$CREATURE_SPELL_FIXTURE_CONTRACT" \
    --arg manifest "$CREATURE_SPELL_FIXTURE_MANIFEST" \
    --arg manifest_sha256 "$CREATURE_SPELL_FIXTURE_MANIFEST_SHA256" \
    --arg journal_sha256 "$CREATURE_SPELL_FIXTURE_JOURNAL_SHA256" \
    --arg snapshot_sha256 "$CREATURE_SPELL_FIXTURE_DATABASE_SNAPSHOT_SHA256" \
    --arg bot_exec "$bot_exec" \
    --arg bot_exec_sha256 "$bot_exec_sha256" \
    --arg bot_report "$bot_report" \
    --arg bot_report_sha256 "$bot_report_sha256" \
    --argjson entry "$CREATURE_SPELL_FIXTURE_ENTRY" \
    --argjson spawn_guid "$CREATURE_SPELL_FIXTURE_SPAWN_GUID" '
      {
        fixture_guard: {
          enabled: true,
          contract: $contract,
          account: "TESTBOT2@bot.local",
          account_id: 9,
          character_guid: 15,
          peer_account: "",
          peer_account_id: 0,
          peer_character_guid: 0,
          creature_entry: $entry,
          creature_spawn_guid: $spawn_guid,
          item_entry: 0,
          fixture_manifest_path: $manifest,
          fixture_manifest_sha256: $manifest_sha256,
          journal_sha256: $journal_sha256,
          database_snapshot_sha256: $snapshot_sha256,
          cleanup_verified: true
        },
        bot_report: {
          contract: "wow-test-bot-creature-spell-casting-report-v1",
          exec_path: $bot_exec,
          exec_sha256: $bot_exec_sha256,
          report_path: $bot_report,
          report_sha256: $bot_report_sha256,
          account: "TESTBOT2@bot.local",
          account_id: 9,
          character_guid: 15,
          report_validated: true
        }
      }
    '
}
