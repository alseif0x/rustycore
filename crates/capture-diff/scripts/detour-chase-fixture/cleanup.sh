detour_chase_restore_fixture_guard() {
  local creature_expression identity_expression stable_expression
  local current_creature_sha current_identity_sha current_stable_sha
  local updated restored_sha restored_sql world_aux_state world_aux_sha
  local current_aux_json current_respawn_json current_database_sha
  local current_account_json
  local current_character_sha current_creature_exists expected_database_sha
  local auth_identity_count character_cas_result character_post_predicate
  local restore_xtrace=0

  [ "$DETOUR_FIXTURE_ENABLED" = "1" ] || return 0
  if [[ "$-" == *x* ]]; then
    restore_xtrace=1
    set +x
  fi
  if [ ! -e "$WOW_BOT_FIXTURE_JOURNAL" ] \
      && [ ! -L "$WOW_BOT_FIXTURE_JOURNAL" ]; then
    if [ -e "$LOOT_FIXTURE_CLEANUP_MARKER" ] \
        || [ -L "$LOOT_FIXTURE_CLEANUP_MARKER" ]; then
      detour_chase_complete_fixture_journal \
        && DETOUR_FIXTURE_CLEANUP_VERIFIED=1
      return
    fi
    [ "$DETOUR_FIXTURE_DB_APPLIED" = 0 ] || {
      echo "WARNING: armed detour DB state lost its recovery journal" >&2
      return 1
    }
    # No DB write was reached.  Still consume the same durable marker contract
    # so the wrapper has one unambiguous cleanup gate.
    detour_chase_write_fixture_journal \
      && detour_chase_complete_fixture_journal \
      && DETOUR_FIXTURE_CLEANUP_VERIFIED=1
    return
  fi
  detour_chase_load_fixture_journal || {
    echo "WARNING: detour fixture journal is unsafe or invalid; refusing DB writes" >&2
    return 1
  }
  if [ "$DETOUR_FIXTURE_DB_APPLIED" = 0 ]; then
    return 0
  fi
  if [ "$DETOUR_FIXTURE_DB_RESTORED" = 1 ]; then
    return 0
  fi
  detour_chase_validate_recovery_anchor_files || {
    echo "WARNING: detour DB/config recovery provenance drifted; refusing cleanup writes" >&2
    return 1
  }
  LOOT_FIXTURE_DB_CONF="$DETOUR_FIXTURE_DB_CONF"
  load_loot_fixture_database_credentials || return 1
  loot_fixture_wait_until_all_characters_offline || return 1
  detour_chase_load_auth_database_credentials || return 1
  auth_identity_count="$(detour_chase_auth_mysql -e "
    SELECT
      (SELECT COUNT(*) FROM account
        WHERE id=${DETOUR_FIXTURE_CHARACTER_ACCOUNT}
          AND battlenet_account=${DETOUR_FIXTURE_BNET_ACCOUNT_ID})
      + (SELECT COUNT(*) FROM battlenet_accounts
        WHERE id=${DETOUR_FIXTURE_BNET_ACCOUNT_ID}
          AND LOWER(email)='testbot2@bot.local')")" || return 1
  [ "$auth_identity_count" = 2 ] || {
    echo "WARNING: detour auth identity drifted externally; refusing cleanup writes" >&2
    return 1
  }
  detour_chase_checkpoint_fixture_poststate || return 1

  creature_expression="$(detour_chase_creature_state_sha_query)" || return 1
  identity_expression="$(detour_chase_character_identity_sha_query)" || return 1
  stable_expression="$(detour_chase_character_stable_sha_query)" || return 1
  restored_sql="$(detour_chase_snapshot_single_character_update_sql)" \
    || return 1
  restored_sha="$(detour_chase_sha256_of_text "$restored_sql")" || return 1
  if [ "$restored_sha" != "$DETOUR_FIXTURE_PRIOR_CHARACTER_SHA256" ]; then
    [ "$restored_sha" = "$DETOUR_FIXTURE_POST_CHARACTER_SHA256" ] || {
        echo "WARNING: complete detour characters row differs from both prior and checkpointed fixture poststate; refusing cleanup writes" >&2
        return 1
    }
    character_post_predicate="$(
      detour_chase_normalize_legacy_singleton_cas_predicate_sql \
        characters "guid=${DETOUR_FIXTURE_CHARACTER_GUID}" \
        "$DETOUR_FIXTURE_POST_CHARACTER_PREDICATE_SQL"
    )" || return 1
    character_cas_result="$(loot_fixture_character_mysql -e "
      SET autocommit=0;
      LOCK TABLES characters WRITE;
      SET @detour_cas=IF((${character_post_predicate}),1,0);
      ${DETOUR_FIXTURE_CHARACTER_RESTORE_SQL}
        AND @detour_cas=1;
      COMMIT;
      SELECT @detour_cas;
      UNLOCK TABLES;
      SET autocommit=1;")" || return 1
    [ "$character_cas_result" = 1 ] || {
      echo "WARNING: complete detour characters row CAS failed under WRITE lock; journal retained" >&2
      return 1
    }
    restored_sql="$(detour_chase_snapshot_single_character_update_sql)" \
      || return 1
    restored_sha="$(detour_chase_sha256_of_text "$restored_sql")" || return 1
    [ "$restored_sha" = "$DETOUR_FIXTURE_PRIOR_CHARACTER_SHA256" ] || {
      echo "WARNING: complete detour characters row did not restore exactly" >&2
      return 1
    }
  fi
  detour_chase_restore_account_state || return 1
  detour_chase_restore_character_auxiliary_state || return 1
  detour_chase_restore_respawn_state || return 1

  world_aux_state="$(detour_chase_world_auxiliary_state)" || return 1
  world_aux_sha="$(detour_chase_sha256_of_text "$world_aux_state")" || return 1
  if [ "$world_aux_sha" != "$DETOUR_FIXTURE_WORLD_AUX_SHA256" ]; then
    echo "WARNING: detour world auxiliary state drifted; the fixture never owns these rows, so cleanup refuses to delete them" >&2
    return 1
  fi

  current_creature_sha="$(loot_fixture_world_mysql -e \
    "SELECT ${creature_expression} FROM creature WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID}")" \
    || return 1
  if [ "$DETOUR_FIXTURE_PRIOR_CREATURE_EXISTS" = 1 ]; then
    if [ "$current_creature_sha" != "$DETOUR_FIXTURE_PRIOR_CREATURE_SHA256" ]; then
      [ "$current_creature_sha" = "$DETOUR_FIXTURE_FIXTURE_CREATURE_SHA256" ] || {
        echo "WARNING: detour creature fixture drifted externally; refusing to overwrite it" >&2
        return 1
      }
      updated="$(loot_fixture_world_mysql -e "
        START TRANSACTION;
        DELETE FROM creature
         WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID}
           AND ${creature_expression}='${DETOUR_FIXTURE_FIXTURE_CREATURE_SHA256}';
        SET @detour_deleted=ROW_COUNT();
        ${DETOUR_FIXTURE_CREATURE_RESTORE_SQL}
        SELECT @detour_deleted;
        COMMIT;")" || return 1
      [ "$updated" = 1 ] || return 1
      restored_sha="$(loot_fixture_world_mysql -e \
        "SELECT ${creature_expression} FROM creature WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID}")" \
        || return 1
      [ "$restored_sha" = "$DETOUR_FIXTURE_PRIOR_CREATURE_SHA256" ] || {
        echo "WARNING: prior detour creature row did not restore exactly" >&2
        return 1
      }
    fi
  else
    if [ -n "$current_creature_sha" ]; then
      [ "$current_creature_sha" = "$DETOUR_FIXTURE_FIXTURE_CREATURE_SHA256" ] || {
        echo "WARNING: detour creature fixture drifted externally; refusing to overwrite it" >&2
        return 1
      }
      updated="$(loot_fixture_world_mysql -e "
        DELETE FROM creature
         WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID}
           AND ${creature_expression}='${DETOUR_FIXTURE_FIXTURE_CREATURE_SHA256}';
        SELECT ROW_COUNT();")" || return 1
      [ "$updated" = 1 ] || return 1
    fi
    [ "$(loot_fixture_world_mysql -e \
      "SELECT COUNT(*) FROM creature WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID}")" = 0 ] \
      || {
        echo "WARNING: synthetic detour creature was not removed exactly" >&2
        return 1
      }
  fi

  # Re-snapshot every guarded domain after cleanup and require the exact same
  # digest that was emitted by both side manifests. This is stronger than
  # checking only the fields the fixture itself intentionally changed.
  current_creature_exists="$(loot_fixture_world_mysql -e \
    "SELECT COUNT(*) FROM creature WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID}")" \
    || return 1
  current_creature_sha="$(loot_fixture_world_mysql -e \
    "SELECT ${creature_expression} FROM creature WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID}")" \
    || return 1
  restored_sql="$(detour_chase_snapshot_single_character_update_sql)" \
    || return 1
  current_character_sha="$(detour_chase_sha256_of_text "$restored_sql")" \
    || return 1
  current_aux_json="$(detour_chase_snapshot_character_auxiliary_state)" \
    || return 1
  current_respawn_json="$(detour_chase_snapshot_respawn_state)" \
    || return 1
  current_account_json="$(detour_chase_snapshot_account_state)" \
    || return 1
  # Predicate SQL is an implementation detail and can evolve between journal
  # creation and crash recovery. The per-domain prior_sha256 values above
  # prove the restored rows; hash the original journal representation after
  # validating that its scope metadata and prior hashes match the live
  # snapshots, so a formatting-only predicate change cannot fake DB drift.
  current_aux_json="$(
    detour_chase_prior_snapshot_representation_for_digest \
      "$current_aux_json" "$DETOUR_FIXTURE_CHARACTER_AUX_SNAPSHOTS_JSON"
  )" || return 1
  current_respawn_json="$(
    detour_chase_prior_snapshot_representation_for_digest \
      "$current_respawn_json" "$DETOUR_FIXTURE_RESPAWN_SNAPSHOT_JSON"
  )" || return 1
  current_account_json="$(
    detour_chase_prior_snapshot_representation_for_digest \
      "$current_account_json" "$DETOUR_FIXTURE_ACCOUNT_SNAPSHOTS_JSON"
  )" || return 1
  expected_database_sha="$DETOUR_FIXTURE_DATABASE_SNAPSHOT_SHA256"
  current_database_sha="$({
    printf 'creature-exists\0%s\0' "$current_creature_exists"
    printf 'creature\0%s\0' "$current_creature_sha"
    printf 'character\0%s\0' "$current_character_sha"
    printf 'character-aux\0%s\0' "$current_aux_json"
    printf 'respawn\0%s\0' "$current_respawn_json"
    printf 'world-aux\0%s\0' "$world_aux_sha"
    printf 'account\0%s\0' "$current_account_json"
  } | sha256sum | awk '{print $1}')" || return 1
  [ "$current_database_sha" = "$expected_database_sha" ] || {
    echo "WARNING: integral detour DB snapshot digest differs after restoration" >&2
    return 1
  }

  detour_chase_mark_db_restored || return 1
  echo "detour fixture: integral character/respawn/world DB snapshot restored and verified"
  if [ "$restore_xtrace" -eq 1 ]; then
    set -x
  fi
}

detour_chase_remove_private_data_dir() {
  local path="$DETOUR_FIXTURE_PRIVATE_DATA_DIR"

  [ "$DETOUR_FIXTURE_ENABLED" = "1" ] || return 0
  [ -n "$path" ] || return 0
  if [ ! -e "$path" ] && [ ! -L "$path" ]; then
    return 0
  fi
  [ -d "$path" ] && [ ! -L "$path" ] \
    && [ "$(stat -c '%d:%i' -- "$path" 2>/dev/null)" \
      = "$DETOUR_FIXTURE_PRIVATE_DATA_DIR_IDENTITY" ] || {
      echo "WARNING: private detour DataDir changed identity; refusing recursive removal" >&2
      return 1
    }
  for child in dbc gt maps vmaps cameras; do
    [ -L "$path/$child" ] \
      && [ "$(readlink -- "$path/$child")" \
        = "$DETOUR_FIXTURE_NORMAL_DATA_DIR/$child" ] || {
        echo "WARNING: private detour DataDir link ${child} drifted" >&2
        return 1
      }
  done
  [ -f "$path/mmaps/0001.mmap" ] \
    && [ -f "$path/mmaps/00015026.mmtile" ] \
    && [ "$(detour_chase_sha256_of_file "$path/mmaps/0001.mmap")" \
      = "$DETOUR_FIXTURE_MAP_SHA256" ] \
    && [ "$(detour_chase_sha256_of_file "$path/mmaps/00015026.mmtile")" \
      = "$DETOUR_FIXTURE_TILE_SHA256" ] || {
      echo "WARNING: private detour MMap assets drifted; refusing recursive removal" >&2
      return 1
    }
  rm -rf -- "$path" || return 1
  [ ! -e "$path" ] && [ ! -L "$path" ] || return 1
}

detour_chase_discard_unarmed_private_data_dir() {
  local path="$DETOUR_FIXTURE_PRIVATE_DATA_DIR"
  local parent base owner mode

  [ -n "$path" ] || return 0
  [ "$DETOUR_FIXTURE_DB_APPLIED" = 0 ] || return 1
  if [ ! -e "$path" ] && [ ! -L "$path" ]; then
    return 0
  fi
  parent="$(dirname -- "$path")"
  base="${path##*/}"
  [[ "$base" =~ ^rustycore-detour-(cpp|rust)\.[A-Za-z0-9]+$ ]] || return 1
  [ -d "$parent" ] && [ ! -L "$parent" ] \
    && [ "$(realpath -e -- "$parent" 2>/dev/null)" = "$parent" ] || return 1
  [ -d "$path" ] && [ ! -L "$path" ] \
    && [ "$(stat -c '%d:%i' -- "$path" 2>/dev/null)" \
      = "$DETOUR_FIXTURE_PRIVATE_DATA_DIR_IDENTITY" ] || return 1
  owner="$(stat -c '%u' -- "$path" 2>/dev/null)" || return 1
  mode="$(stat -c '%a' -- "$path" 2>/dev/null)" || return 1
  [ "$owner" = "$(id -u)" ] && [ "$mode" = 700 ] || return 1
  rm -rf -- "$path" || return 1
  [ ! -e "$path" ] && [ ! -L "$path" ]
}

detour_chase_remove_unarmed_owned_file() {
  local path="$1"
  local identity="$2"
  [ -n "$path" ] || return 0
  if [ ! -e "$path" ] && [ ! -L "$path" ]; then
    return 0
  fi
  [ -f "$path" ] && [ ! -L "$path" ] \
    && [ "$(stat -c '%d:%i' -- "$path")" = "$identity" ] || return 1
  rm -- "$path"
}

detour_chase_discard_uncheckpointed_rust_artifacts() {
  [ "$DETOUR_FIXTURE_SIDE" = rust ] \
    && [ "$DETOUR_FIXTURE_DB_APPLIED" = 0 ] \
    && [ -z "$DETOUR_FIXTURE_RUST_CONFIG" ] || return 1
  detour_chase_discard_unarmed_private_data_dir \
    && detour_chase_remove_unarmed_owned_file \
      "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE" \
      "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE_IDENTITY" \
    && detour_chase_remove_unarmed_owned_file \
      "$DETOUR_FIXTURE_PM2_RESTORE_FILE" \
      "$DETOUR_FIXTURE_PM2_RESTORE_FILE_IDENTITY"
}

detour_chase_remove_rust_capture_config() {
  local path="$DETOUR_FIXTURE_RUST_CONFIG"

  [ -n "$path" ] || return 0
  if [ ! -e "$path" ] && [ ! -L "$path" ]; then
    return 0
  fi
  [ -f "$path" ] && [ ! -L "$path" ] \
    && [ "$(stat -c '%d:%i' -- "$path" 2>/dev/null)" \
      = "$DETOUR_FIXTURE_RUST_CONFIG_IDENTITY" ] \
    && [ "$(detour_chase_sha256_of_file "$path" 2>/dev/null)" \
      = "$DETOUR_FIXTURE_RUST_CONFIG_SHA256" ] || {
      echo "WARNING: temporary Rust detour config drifted; refusing removal" >&2
      return 1
    }
  rm -- "$path" || return 1
  [ ! -e "$path" ] && [ ! -L "$path" ]
}

detour_chase_remove_rust_pm2_capture_file() {
  local path="$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE"

  [ -n "$path" ] || return 0
  if [ ! -e "$path" ] && [ ! -L "$path" ]; then
    return 0
  fi
  [ -f "$path" ] && [ ! -L "$path" ] \
    && [ "$(stat -c '%d:%i' -- "$path" 2>/dev/null)" \
      = "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE_IDENTITY" ] \
    && [ "$(detour_chase_sha256_of_file "$path" 2>/dev/null)" \
      = "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE_SHA256" ] || {
      echo "WARNING: temporary Rust detour PM2 capture file drifted; refusing removal" >&2
      return 1
    }
  rm -- "$path" || return 1
  [ ! -e "$path" ] && [ ! -L "$path" ]
}
