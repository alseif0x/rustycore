creature_spell_fixture_write_journal() {
  local mode="$1"
  local phase="$2"
  local journal="$CREATURE_SPELL_FIXTURE_JOURNAL"
  local parent stage old_sha old_identity
  case "$mode" in create|replace) ;; *) return 1 ;; esac
  case "$phase" in armed|applied|captured|restored) ;; *) return 1 ;; esac
  case "$CREATURE_SPELL_FIXTURE_SIDE" in cpp|rust) ;; *) return 1 ;; esac
  [[ "$CREATURE_SPELL_FIXTURE_WORLD_PORT" =~ ^[1-9][0-9]*$ ]] \
    && [[ "$CREATURE_SPELL_FIXTURE_INSTANCE_PORT" =~ ^[1-9][0-9]*$ ]] \
    && [ "$CREATURE_SPELL_FIXTURE_WORLD_PORT" \
      != "$CREATURE_SPELL_FIXTURE_INSTANCE_PORT" ] || return 1
  creature_spell_fixture_validate_character_original_tsv \
    "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV" \
    && [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_SHA256" \
      =~ ^[0-9a-f]{64}$ ]] \
    && [ "$(creature_spell_fixture_sha256_of_text \
      "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV")" \
      = "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_SHA256" ] \
    && [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_ROW_SHA256" \
      =~ ^[0-9a-f]{64}$ ]] \
    && [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256" \
      =~ ^[0-9a-f]{64}$ ]] \
    && [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256" \
      =~ ^[0-9a-f]{64}$ ]] || return 1
  case "$phase" in
    armed|applied)
      [ -z "$CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256" ] || return 1
      ;;
    captured)
      [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256" \
        =~ ^[0-9a-f]{64}$ ]] || return 1
      ;;
    restored)
      if [ -n "$CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256" ]; then
        [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256" \
          =~ ^[0-9a-f]{64}$ ]] || return 1
      fi
      ;;
  esac
  parent="$(dirname -- "$journal")"
  stage="$(mktemp "${parent}/.creature-spell-journal.XXXXXX")" || return 1
  chmod 600 "$stage" || { rm -f -- "$stage"; return 1; }
  if ! jq -n \
      --arg contract "$CREATURE_SPELL_FIXTURE_CONTRACT" \
      --arg flow "$CREATURE_SPELL_FIXTURE_FLOW" \
      --arg side "$CREATURE_SPELL_FIXTURE_SIDE" \
      --arg phase "$phase" \
      --arg created_at "$CREATURE_SPELL_FIXTURE_CREATED_AT" \
      --arg manifest "$CREATURE_SPELL_FIXTURE_MANIFEST" \
      --arg manifest_sha256 "$CREATURE_SPELL_FIXTURE_MANIFEST_SHA256" \
      --arg snapshot_sha256 "$CREATURE_SPELL_FIXTURE_DATABASE_SNAPSHOT_SHA256" \
      --arg db_conf "$CREATURE_SPELL_FIXTURE_DB_CONF" \
      --arg db_conf_sha256 "$CREATURE_SPELL_FIXTURE_DB_CONF_SHA256" \
      --arg db_conf_identity "$CREATURE_SPELL_FIXTURE_DB_CONF_IDENTITY" \
      --arg lock "$CREATURE_SPELL_FIXTURE_ORCHESTRATION_LOCK" \
      --arg pm2_rust "$CREATURE_SPELL_FIXTURE_PM2_RUST_WORLD" \
      --arg pm2_cpp "$CREATURE_SPELL_FIXTURE_PM2_CPP_WORLD" \
      --arg original_ai "$CREATURE_SPELL_FIXTURE_ORIGINAL_AI" \
      --arg temporary_ai "$CREATURE_SPELL_FIXTURE_TEMP_AI" \
      --arg character_account "$CREATURE_SPELL_FIXTURE_ACCOUNT" \
      --arg character_name_hex "$CREATURE_SPELL_FIXTURE_CHARACTER_NAME_HEX" \
      --arg character_original "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV" \
      --arg character_original_sha256 "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_SHA256" \
      --arg character_original_row_sha256 "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_ROW_SHA256" \
      --arg character_prelogin_row_sha256 "$CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256" \
      --arg character_post_login_row_sha256 "$CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256" \
      --arg character_immutable_sha256 "$CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256" \
      --arg character_schema_sha256 "$CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_SHA256" \
      --argjson world_port "$CREATURE_SPELL_FIXTURE_WORLD_PORT" \
      --argjson instance_port "$CREATURE_SPELL_FIXTURE_INSTANCE_PORT" \
      --argjson entry "$CREATURE_SPELL_FIXTURE_ENTRY" \
      --argjson difficulty_id "$CREATURE_SPELL_FIXTURE_DIFFICULTY_ID" \
      --argjson original_static_flags_1 "$CREATURE_SPELL_FIXTURE_ORIGINAL_STATIC_FLAGS_1" \
      --argjson temporary_static_flags_1 "$CREATURE_SPELL_FIXTURE_TEMP_STATIC_FLAGS_1" \
      --argjson spawn_guid "$CREATURE_SPELL_FIXTURE_SPAWN_GUID" \
      --argjson spell_slot "$CREATURE_SPELL_FIXTURE_SPELL_SLOT" \
      --argjson spell_id "$CREATURE_SPELL_FIXTURE_SPELL_ID" '
        {
          version: 2,
          contract: $contract,
          flow: $flow,
          side: $side,
          phase: $phase,
          created_at: $created_at,
          fixture_manifest: $manifest,
          fixture_manifest_sha256: $manifest_sha256,
          database_snapshot_sha256: $snapshot_sha256,
          original: {
            creature_entry: $entry,
            creature_spawn_guid: $spawn_guid,
            ai_name: $original_ai,
            difficulty_id: $difficulty_id,
            static_flags_1: $original_static_flags_1,
            spawn_count: 1,
            spell_slot: $spell_slot,
            spell_id: $spell_id,
            spell_count: 1
          },
          temporary: {
            ai_name: $temporary_ai,
            static_flags_1: $temporary_static_flags_1
          },
          character: {
            account: $character_account,
            account_id: 9,
            guid: 15,
            name_hex: $character_name_hex,
            race: 1,
            class: 2,
            level: 80,
            schema_column_count: 87,
            schema_metadata_bytes: 2888,
            schema_sha256: $character_schema_sha256,
            restore_projection: "CHAR_UPD_CHARACTER-73-fields-v1",
            original_projection_sha256: $character_original_sha256,
            original_fields: ($character_original | split("\t")),
            original_row_sha256: $character_original_row_sha256,
            prelogin_row_sha256: $character_prelogin_row_sha256,
            immutable_sha256: $character_immutable_sha256,
            post_login_row_sha256: (
              if $character_post_login_row_sha256 == ""
              then null else $character_post_login_row_sha256 end
            ),
            temporary: {
              map: 530,
              zone: 0,
              instance_id: 0,
              start_x: -2749.52,
              start_y: 5431.19,
              start_z: -34.4548,
              pull_x: -2760.52,
              pull_y: 5431.19,
              pull_z: -34.4548,
              orientation: 0,
              health: 50000
            }
          },
          recovery: {
            db_conf: $db_conf,
            db_conf_sha256: $db_conf_sha256,
            db_conf_identity: $db_conf_identity,
            orchestration_lock: $lock,
            pm2_rust_world: $pm2_rust,
            pm2_cpp_world: $pm2_cpp,
            world_port: $world_port,
            instance_port: $instance_port
          }
        }
      ' >"$stage"; then
    rm -f -- "$stage"
    return 1
  fi
  sync -f "$stage" || { rm -f -- "$stage"; return 1; }

  if [ "$mode" = create ]; then
    [ ! -e "$journal" ] && [ ! -L "$journal" ] \
      && ln -- "$stage" "$journal" || {
        rm -f -- "$stage"
        echo "error: refusing to replace an existing creature spell recovery journal" >&2
        return 1
      }
    rm -- "$stage" || return 1
  else
    [ -f "$journal" ] && [ ! -L "$journal" ] \
      && [ "$(stat -c '%a' -- "$journal" 2>/dev/null)" = 600 ] || {
        rm -f -- "$stage"
        return 1
      }
    old_sha="$(creature_spell_fixture_sha256_of_file "$journal")" || {
      rm -f -- "$stage"; return 1;
    }
    old_identity="$(stat -c '%d:%i' -- "$journal")" || {
      rm -f -- "$stage"; return 1;
    }
    [ "$old_sha" = "$CREATURE_SPELL_FIXTURE_CURRENT_JOURNAL_SHA256" ] \
      && [ "$old_identity" \
        = "$CREATURE_SPELL_FIXTURE_CURRENT_JOURNAL_IDENTITY" ] || {
        rm -f -- "$stage"
        echo "error: creature spell recovery journal changed before phase update" >&2
        return 1
      }
    mv -f -- "$stage" "$journal" || { rm -f -- "$stage"; return 1; }
  fi
  sync -f "$journal" && sync -f "$parent" || return 1
  [ -f "$journal" ] && [ ! -L "$journal" ] \
    && [ "$(stat -c '%a' -- "$journal")" = 600 ] || return 1
  CREATURE_SPELL_FIXTURE_PHASE="$phase"
  CREATURE_SPELL_FIXTURE_CURRENT_JOURNAL_SHA256="$(
    creature_spell_fixture_sha256_of_file "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CURRENT_JOURNAL_IDENTITY="$(
    stat -c '%d:%i' -- "$journal"
  )" || return 1
}

creature_spell_fixture_load_journal() {
  local journal="$CREATURE_SPELL_FIXTURE_JOURNAL"
  local expected_manifest_sha
  [ -f "$journal" ] && [ ! -L "$journal" ] \
    && [ "$(stat -c '%a' -- "$journal" 2>/dev/null)" = 600 ] \
    && [ "$(stat -c '%u' -- "$journal" 2>/dev/null)" = "$(id -u)" ] \
    || return 1
  jq -e '
    keys == [
      "character", "contract", "created_at", "database_snapshot_sha256", "fixture_manifest",
      "fixture_manifest_sha256", "flow", "original", "phase", "recovery",
      "side", "temporary", "version"
    ]
    and .version == 2
    and .contract == "creature-spell-casting-shell-fixture-v2"
    and .flow == "creature-spell-casting"
    and (.side == "cpp" or .side == "rust")
    and (.phase == "armed" or .phase == "applied" or .phase == "captured" or .phase == "restored")
    and (.created_at | type == "string" and length > 0)
    and (.fixture_manifest | type == "string" and startswith("/"))
    and (.fixture_manifest_sha256 | test("^[0-9a-f]{64}$"))
    and (.database_snapshot_sha256 | test("^[0-9a-f]{64}$"))
    and .original == {
      creature_entry: 22378,
      creature_spawn_guid: 78686,
      ai_name: "SmartAI",
      difficulty_id: 0,
      static_flags_1: 0,
      spawn_count: 1,
      spell_slot: 0,
      spell_id: 15691,
      spell_count: 1
    }
    and .temporary == {ai_name: "CombatAI", static_flags_1: 1048576}
    and .character.account == "TESTBOT2@bot.local"
    and .character.account_id == 9
    and .character.guid == 15
    and .character.name_hex == "4C66676865616C"
    and .character.race == 1
    and .character.class == 2
    and .character.level == 80
    and (.character | keys) == [
      "account", "account_id", "class", "guid", "immutable_sha256", "level",
      "name_hex", "original_fields", "original_projection_sha256", "original_row_sha256",
      "post_login_row_sha256", "prelogin_row_sha256", "race",
      "restore_projection", "schema_column_count", "schema_metadata_bytes",
      "schema_sha256", "temporary"
    ]
    and .character.schema_column_count == 87
    and .character.schema_metadata_bytes == 2888
    and .character.schema_sha256 == "1c8ef9a9367734daced44acf567cc5453357498c04d57c37f2cce3e5108aa24c"
    and .character.restore_projection == "CHAR_UPD_CHARACTER-73-fields-v1"
    and (.character.original_projection_sha256 | test("^[0-9a-f]{64}$"))
    and (.character.original_row_sha256 | test("^[0-9a-f]{64}$"))
    and (.character.prelogin_row_sha256 | test("^[0-9a-f]{64}$"))
    and (.character.immutable_sha256 | test("^[0-9a-f]{64}$"))
    and (.character.original_fields | type == "array" and length == 73)
    and all(.character.original_fields[]; type == "string")
    and (
      if (.phase == "armed" or .phase == "applied") then
        .character.post_login_row_sha256 == null
      elif .phase == "captured" then
        (.character.post_login_row_sha256 | test("^[0-9a-f]{64}$"))
      else
        (.character.post_login_row_sha256 == null
          or (.character.post_login_row_sha256 | test("^[0-9a-f]{64}$")))
      end
    )
    and .character.temporary == {
      map: 530,
      zone: 0,
      instance_id: 0,
      start_x: -2749.52,
      start_y: 5431.19,
      start_z: -34.4548,
      pull_x: -2760.52,
      pull_y: 5431.19,
      pull_z: -34.4548,
      orientation: 0,
      health: 50000
    }
    and (.recovery.db_conf | type == "string" and startswith("/"))
    and (.recovery.db_conf_sha256 | test("^[0-9a-f]{64}$"))
    and (.recovery.db_conf_identity | test("^[0-9]+:[0-9]+$"))
    and (.recovery.orchestration_lock | type == "string" and startswith("/"))
    and (.recovery.pm2_rust_world | type == "string" and length > 0)
    and (.recovery.pm2_cpp_world | type == "string" and length > 0)
    and (.recovery.world_port | type == "number" and . >= 1 and . <= 65535)
    and (.recovery.instance_port | type == "number" and . >= 1 and . <= 65535)
    and .recovery.world_port != .recovery.instance_port
  ' "$journal" >/dev/null || return 1

  CREATURE_SPELL_FIXTURE_SIDE="$(jq -r '.side' "$journal")" || return 1
  CREATURE_SPELL_FIXTURE_PHASE="$(jq -r '.phase' "$journal")" || return 1
  CREATURE_SPELL_FIXTURE_CREATED_AT="$(jq -r '.created_at' "$journal")" || return 1
  CREATURE_SPELL_FIXTURE_MANIFEST="$(jq -r '.fixture_manifest' "$journal")" || return 1
  CREATURE_SPELL_FIXTURE_MANIFEST_SHA256="$(
    jq -r '.fixture_manifest_sha256' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_DATABASE_SNAPSHOT_SHA256="$(
    jq -r '.database_snapshot_sha256' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV="$(
    jq -r '.character.original_fields | @tsv' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_SHA256="$(
    jq -r '.character.original_projection_sha256' "$journal"
  )" || return 1
  creature_spell_fixture_validate_character_original_tsv \
    "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV" || return 1
  [ "$(creature_spell_fixture_sha256_of_text \
      "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV")" \
    = "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_SHA256" ] || return 1
  CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_ROW_SHA256="$(
    jq -r '.character.original_row_sha256' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256="$(
    jq -r '.character.prelogin_row_sha256' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256="$(
    jq -r '.character.post_login_row_sha256 // ""' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256="$(
    jq -r '.character.immutable_sha256' "$journal"
  )" || return 1
  [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_ROW_SHA256" \
    =~ ^[0-9a-f]{64}$ ]] \
    && [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256" \
      =~ ^[0-9a-f]{64}$ ]] \
    && [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256" \
      =~ ^[0-9a-f]{64}$ ]] \
    && { [ -z "$CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256" ] \
      || [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256" \
        =~ ^[0-9a-f]{64}$ ]]; } || return 1
  CREATURE_SPELL_FIXTURE_DB_CONF="$(jq -r '.recovery.db_conf' "$journal")" || return 1
  CREATURE_SPELL_FIXTURE_DB_CONF_SHA256="$(
    jq -r '.recovery.db_conf_sha256' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_DB_CONF_IDENTITY="$(
    jq -r '.recovery.db_conf_identity' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_ORCHESTRATION_LOCK="$(
    jq -r '.recovery.orchestration_lock' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_PM2_RUST_WORLD="$(
    jq -r '.recovery.pm2_rust_world' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_PM2_CPP_WORLD="$(
    jq -r '.recovery.pm2_cpp_world' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_WORLD_PORT="$(
    jq -r '.recovery.world_port' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_INSTANCE_PORT="$(
    jq -r '.recovery.instance_port' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CURRENT_JOURNAL_SHA256="$(
    creature_spell_fixture_sha256_of_file "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CURRENT_JOURNAL_IDENTITY="$(
    stat -c '%d:%i' -- "$journal"
  )" || return 1

  creature_spell_fixture_validate_manifest_file \
    "$CREATURE_SPELL_FIXTURE_MANIFEST" || return 1
  expected_manifest_sha="$(creature_spell_fixture_sha256_of_file \
    "$CREATURE_SPELL_FIXTURE_MANIFEST")" || return 1
  [ "$expected_manifest_sha" = "$CREATURE_SPELL_FIXTURE_MANIFEST_SHA256" ] \
    && creature_spell_fixture_validate_db_config \
    && creature_spell_fixture_validate_character_schema
}

creature_spell_fixture_apply_guard() {
  local snapshot after_snapshot
  CREATURE_SPELL_FIXTURE_GHOST_PREFLIGHT_VERIFIED=0
  CREATURE_SPELL_FIXTURE_GHOST_POST_CAPTURE_VERIFIED=0
  creature_spell_fixture_validate_fresh_journal || return 1
  creature_spell_fixture_validate_manifest_file \
    "$CREATURE_SPELL_FIXTURE_MANIFEST" || return 1
  [ "$(creature_spell_fixture_sha256_of_file \
      "$CREATURE_SPELL_FIXTURE_MANIFEST")" \
    = "$CREATURE_SPELL_FIXTURE_MANIFEST_SHA256" ] || return 1
  creature_spell_fixture_validate_db_config || return 1
  creature_spell_fixture_require_safe_db_window || return 1
  creature_spell_fixture_verify_exact_state \
    "$CREATURE_SPELL_FIXTURE_ORIGINAL_AI" || return 1
  creature_spell_fixture_verify_no_persisted_ghost_state || return 1
  CREATURE_SPELL_FIXTURE_GHOST_PREFLIGHT_VERIFIED=1
  creature_spell_fixture_snapshot_character || return 1
  creature_spell_fixture_verify_character_original || return 1
  snapshot="$(creature_spell_fixture_static_snapshot_sha256)" || return 1
  CREATURE_SPELL_FIXTURE_DATABASE_SNAPSHOT_SHA256="$snapshot"
  CREATURE_SPELL_FIXTURE_CREATED_AT="$(date -u +'%Y-%m-%dT%H:%M:%SZ')" \
    || return 1
  creature_spell_fixture_write_journal create armed || return 1

  # The durable journal exists before the owned world mutation. AIName and
  # StaticFlags1 change in one multi-table InnoDB UPDATE, so a kill after
  # COMMIT but before the phase update is recoverable from either exact pair.
  creature_spell_fixture_cas_runtime_state \
    "$CREATURE_SPELL_FIXTURE_ORIGINAL_AI" \
    "$CREATURE_SPELL_FIXTURE_TEMP_AI" || return 1
  CREATURE_SPELL_FIXTURE_DB_APPLIED=1
  creature_spell_fixture_cas_character_to_prelogin || return 1
  creature_spell_fixture_verify_exact_state \
    "$CREATURE_SPELL_FIXTURE_TEMP_AI" || return 1
  creature_spell_fixture_verify_character_prelogin || return 1
  after_snapshot="$(creature_spell_fixture_static_snapshot_sha256)" || return 1
  [ "$after_snapshot" = "$snapshot" ] || {
    echo "error: creature spell fixture topology drifted during activation; journal retained" >&2
    return 1
  }
  creature_spell_fixture_write_journal replace applied || return 1
  echo "creature spell fixture: entry ${CREATURE_SPELL_FIXTURE_ENTRY} AIName ${CREATURE_SPELL_FIXTURE_ORIGINAL_AI} -> ${CREATURE_SPELL_FIXTURE_TEMP_AI}, difficulty ${CREATURE_SPELL_FIXTURE_DIFFICULTY_ID} StaticFlags1 ${CREATURE_SPELL_FIXTURE_ORIGINAL_STATIC_FLAGS_1} -> ${CREATURE_SPELL_FIXTURE_TEMP_STATIC_FLAGS_1}; spawn ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}, spell ${CREATURE_SPELL_FIXTURE_SPELL_ID} (restore journal armed)"
}

# Normal capture wrappers call this after they have stopped and accredited the
# capture world. Recovery deliberately does not synthesize a post-login hash:
# an applied journal is recoverable only while the complete row still equals
# the deterministic pre-login hash; any other row remains fail-closed.
creature_spell_fixture_record_post_login_snapshot() {
  CREATURE_SPELL_FIXTURE_GHOST_POST_CAPTURE_VERIFIED=0
  creature_spell_fixture_load_journal || {
    echo "WARNING: creature spell fixture journal is unsafe; refusing to snapshot post-login state" >&2
    return 1
  }
  creature_spell_fixture_reconcile_owned_online_marker || return 1
  creature_spell_fixture_require_safe_db_window || return 1
  case "$CREATURE_SPELL_FIXTURE_PHASE" in
    armed)
      # Activation did not reach its durable applied phase. No world could have
      # been started by a successful caller, and restore uses the exact known
      # original/pre-login states instead.
      return 0
      ;;
    applied)
      creature_spell_fixture_snapshot_character_post_login || return 1
      # Re-read by exact full-row hash before the journal transition. A later
      # change is still caught atomically by the restoration UPDATE CAS.
      creature_spell_fixture_verify_character_post_login || return 1
      creature_spell_fixture_write_journal replace captured || return 1
      creature_spell_fixture_verify_no_persisted_ghost_state || return 1
      CREATURE_SPELL_FIXTURE_GHOST_POST_CAPTURE_VERIFIED=1
      ;;
    captured)
      if creature_spell_fixture_verify_character_post_login \
          || creature_spell_fixture_verify_character_original; then
        creature_spell_fixture_verify_no_persisted_ghost_state || return 1
        CREATURE_SPELL_FIXTURE_GHOST_POST_CAPTURE_VERIFIED=1
        return 0
      fi
      echo "WARNING: journaled creature spell post-login state changed before restoration" >&2
      return 1
      ;;
    restored)
      creature_spell_fixture_verify_character_original
      ;;
    *) return 1 ;;
  esac
}

creature_spell_fixture_validate_cleanup_marker() {
  local marker="$CREATURE_SPELL_FIXTURE_CLEANUP_MARKER"
  local parent
  [ -n "$marker" ] || marker="${CREATURE_SPELL_FIXTURE_JOURNAL}.cleanup-complete"
  CREATURE_SPELL_FIXTURE_CLEANUP_MARKER="$marker"
  creature_spell_fixture_validate_private_path \
    "$marker" CREATURE_SPELL_FIXTURE_CLEANUP_MARKER || return 1
  [ -f "$marker" ] && [ ! -L "$marker" ] \
    && [ "$(stat -c '%a' -- "$marker" 2>/dev/null)" = 600 ] \
    && [ "$(stat -c '%u' -- "$marker" 2>/dev/null)" = "$(id -u)" ] \
    || return 1
  jq -e '
    keys == [
      "character_account", "character_account_id", "character_guid",
      "character_immutable_sha256",
      "character_original_projection_sha256", "character_original_row_sha256",
      "character_post_login_row_sha256", "character_prelogin_row_sha256",
      "character_schema_sha256", "cleanup_pid", "contract", "creature_entry", "creature_spawn_guid",
      "database_snapshot_sha256", "difficulty_id", "fixture_manifest_sha256", "flow",
      "journal_sha256", "original_ai_name", "original_static_flags_1", "side", "spell_id",
      "temporary_ai_name", "temporary_static_flags_1", "version"
    ]
    and .version == 2
    and .contract == "creature-spell-casting-shell-fixture-v2"
    and .flow == "creature-spell-casting"
    and (.side == "cpp" or .side == "rust")
    and .creature_entry == 22378
    and .difficulty_id == 0
    and .creature_spawn_guid == 78686
    and .spell_id == 15691
    and .character_account == "TESTBOT2@bot.local"
    and .character_account_id == 9
    and .character_guid == 15
    and (.character_original_projection_sha256 | test("^[0-9a-f]{64}$"))
    and (.character_original_row_sha256 | test("^[0-9a-f]{64}$"))
    and (.character_prelogin_row_sha256 | test("^[0-9a-f]{64}$"))
    and (.character_immutable_sha256 | test("^[0-9a-f]{64}$"))
    and (.character_post_login_row_sha256 == null
      or (.character_post_login_row_sha256 | test("^[0-9a-f]{64}$")))
    and .character_schema_sha256 == "1c8ef9a9367734daced44acf567cc5453357498c04d57c37f2cce3e5108aa24c"
    and .original_ai_name == "SmartAI"
    and .original_static_flags_1 == 0
    and .temporary_ai_name == "CombatAI"
    and .temporary_static_flags_1 == 1048576
    and (.cleanup_pid | type == "number" and . > 0)
    and (.journal_sha256 | test("^[0-9a-f]{64}$"))
    and (.database_snapshot_sha256 | test("^[0-9a-f]{64}$"))
    and (.fixture_manifest_sha256 | test("^[0-9a-f]{64}$"))
  ' "$marker" >/dev/null || return 1
  CREATURE_SPELL_FIXTURE_SIDE="$(jq -r '.side' "$marker")" || return 1
  CREATURE_SPELL_FIXTURE_JOURNAL_SHA256="$(
    jq -r '.journal_sha256' "$marker"
  )" || return 1
  CREATURE_SPELL_FIXTURE_DATABASE_SNAPSHOT_SHA256="$(
    jq -r '.database_snapshot_sha256' "$marker"
  )" || return 1
  CREATURE_SPELL_FIXTURE_MANIFEST_SHA256="$(
    jq -r '.fixture_manifest_sha256' "$marker"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_SHA256="$(
    jq -r '.character_original_projection_sha256' "$marker"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_ROW_SHA256="$(
    jq -r '.character_original_row_sha256' "$marker"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256="$(
    jq -r '.character_prelogin_row_sha256' "$marker"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256="$(
    jq -r '.character_post_login_row_sha256 // ""' "$marker"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256="$(
    jq -r '.character_immutable_sha256' "$marker"
  )" || return 1
  parent="$(dirname -- "$marker")"
  [ "$(realpath -e -- "$parent")" = "$parent" ] || return 1
  CREATURE_SPELL_FIXTURE_CLEANUP_VERIFIED=1
}

creature_spell_fixture_complete_journal() {
  local journal="$CREATURE_SPELL_FIXTURE_JOURNAL"
  local marker="${journal}.cleanup-complete"
  local parent stage journal_sha marker_sha
  CREATURE_SPELL_FIXTURE_CLEANUP_MARKER="$marker"
  [ -f "$journal" ] && [ ! -L "$journal" ] \
    && [ "$CREATURE_SPELL_FIXTURE_PHASE" = restored ] || return 1
  journal_sha="$(creature_spell_fixture_sha256_of_file "$journal")" || return 1
  [ "$journal_sha" = "$CREATURE_SPELL_FIXTURE_CURRENT_JOURNAL_SHA256" ] \
    || return 1
  parent="$(dirname -- "$journal")"

  if [ -e "$marker" ] || [ -L "$marker" ]; then
    creature_spell_fixture_validate_cleanup_marker || return 1
    marker_sha="$CREATURE_SPELL_FIXTURE_JOURNAL_SHA256"
    [ "$marker_sha" = "$journal_sha" ] || return 1
  else
    stage="$(mktemp "${parent}/.creature-spell-cleanup.XXXXXX")" || return 1
    chmod 600 "$stage" || { rm -f -- "$stage"; return 1; }
    if ! jq -n \
        --arg contract "$CREATURE_SPELL_FIXTURE_CONTRACT" \
        --arg flow "$CREATURE_SPELL_FIXTURE_FLOW" \
        --arg side "$CREATURE_SPELL_FIXTURE_SIDE" \
        --arg journal_sha256 "$journal_sha" \
        --arg snapshot_sha256 "$CREATURE_SPELL_FIXTURE_DATABASE_SNAPSHOT_SHA256" \
        --arg fixture_manifest_sha256 "$CREATURE_SPELL_FIXTURE_MANIFEST_SHA256" \
        --arg original_ai "$CREATURE_SPELL_FIXTURE_ORIGINAL_AI" \
        --arg temporary_ai "$CREATURE_SPELL_FIXTURE_TEMP_AI" \
        --arg character_account "$CREATURE_SPELL_FIXTURE_ACCOUNT" \
        --arg character_original_sha256 "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_SHA256" \
        --arg character_original_row_sha256 "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_ROW_SHA256" \
        --arg character_prelogin_row_sha256 "$CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256" \
        --arg character_post_login_row_sha256 "$CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256" \
        --arg character_immutable_sha256 "$CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256" \
        --arg character_schema_sha256 "$CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_SHA256" \
        --argjson cleanup_pid "$$" \
        --argjson creature_entry "$CREATURE_SPELL_FIXTURE_ENTRY" \
        --argjson difficulty_id "$CREATURE_SPELL_FIXTURE_DIFFICULTY_ID" \
        --argjson original_static_flags_1 "$CREATURE_SPELL_FIXTURE_ORIGINAL_STATIC_FLAGS_1" \
        --argjson temporary_static_flags_1 "$CREATURE_SPELL_FIXTURE_TEMP_STATIC_FLAGS_1" \
        --argjson creature_spawn_guid "$CREATURE_SPELL_FIXTURE_SPAWN_GUID" \
        --argjson spell_id "$CREATURE_SPELL_FIXTURE_SPELL_ID" '
          {
            version: 2,
            contract: $contract,
            flow: $flow,
            side: $side,
            creature_entry: $creature_entry,
            difficulty_id: $difficulty_id,
            creature_spawn_guid: $creature_spawn_guid,
            spell_id: $spell_id,
            character_account: $character_account,
            character_account_id: 9,
            character_guid: 15,
            character_schema_sha256: $character_schema_sha256,
            character_immutable_sha256: $character_immutable_sha256,
            character_original_projection_sha256: $character_original_sha256,
            character_original_row_sha256: $character_original_row_sha256,
            character_prelogin_row_sha256: $character_prelogin_row_sha256,
            character_post_login_row_sha256: (
              if $character_post_login_row_sha256 == ""
              then null else $character_post_login_row_sha256 end
            ),
            original_ai_name: $original_ai,
            original_static_flags_1: $original_static_flags_1,
            temporary_ai_name: $temporary_ai,
            temporary_static_flags_1: $temporary_static_flags_1,
            fixture_manifest_sha256: $fixture_manifest_sha256,
            database_snapshot_sha256: $snapshot_sha256,
            journal_sha256: $journal_sha256,
            cleanup_pid: $cleanup_pid
          }
        ' >"$stage"; then
      rm -f -- "$stage"
      return 1
    fi
    sync -f "$stage" \
      && ln -- "$stage" "$marker" \
      && rm -- "$stage" \
      && sync -f "$marker" \
      && sync -f "$parent" || {
        rm -f -- "$stage"
        return 1
      }
  fi
  rm -- "$journal" && sync -f "$parent" || return 1
  CREATURE_SPELL_FIXTURE_JOURNAL_SHA256="$journal_sha"
  CREATURE_SPELL_FIXTURE_CURRENT_JOURNAL_SHA256=""
  CREATURE_SPELL_FIXTURE_CURRENT_JOURNAL_IDENTITY=""
  CREATURE_SPELL_FIXTURE_DB_APPLIED=0
  creature_spell_fixture_validate_cleanup_marker
}
