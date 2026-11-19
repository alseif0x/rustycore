detour_chase_creature_state_sha_query() {
  cat <<'SQL'
SHA2(CONCAT_WS('#',
COALESCE(HEX(guid),'~'),COALESCE(HEX(id),'~'),COALESCE(HEX(map),'~'),
COALESCE(HEX(zoneId),'~'),COALESCE(HEX(areaId),'~'),
COALESCE(HEX(spawnDifficulties),'~'),COALESCE(HEX(phaseUseFlags),'~'),
COALESCE(HEX(PhaseId),'~'),COALESCE(HEX(PhaseGroup),'~'),
COALESCE(HEX(terrainSwapMap),'~'),COALESCE(HEX(modelid),'~'),
COALESCE(HEX(equipment_id),'~'),COALESCE(HEX(CAST(position_x AS CHAR)),'~'),
COALESCE(HEX(CAST(position_y AS CHAR)),'~'),
COALESCE(HEX(CAST(position_z AS CHAR)),'~'),
COALESCE(HEX(CAST(orientation AS CHAR)),'~'),
COALESCE(HEX(spawntimesecs),'~'),
COALESCE(HEX(CAST(wander_distance AS CHAR)),'~'),
COALESCE(HEX(currentwaypoint),'~'),COALESCE(HEX(curhealth),'~'),
COALESCE(HEX(curmana),'~'),COALESCE(HEX(MovementType),'~'),
COALESCE(HEX(npcflag),'~'),COALESCE(HEX(unit_flags),'~'),
COALESCE(HEX(unit_flags2),'~'),COALESCE(HEX(unit_flags3),'~'),
COALESCE(HEX(ScriptName),'~'),COALESCE(HEX(StringId),'~'),
COALESCE(HEX(VerifiedBuild),'~')),256)
SQL
}

detour_chase_character_identity_sha_query() {
  cat <<'SQL'
SHA2(CONCAT_WS('#',
COALESCE(HEX(guid),'~'),COALESCE(HEX(account),'~'),
COALESCE(HEX(name),'~'),COALESCE(HEX(race),'~'),
COALESCE(HEX(class),'~'),COALESCE(HEX(gender),'~'),
COALESCE(HEX(createTime),'~'),COALESCE(HEX(deleteInfos_Account),'~'),
COALESCE(HEX(deleteInfos_Name),'~'),COALESCE(HEX(deleteDate),'~')),256)
SQL
}

detour_chase_character_stable_sha_query() {
  cat <<'SQL'
SHA2(CONCAT_WS('#',
COALESCE(HEX(account),'~'),COALESCE(HEX(at_login),'~'),
COALESCE(HEX(level),'~'),COALESCE(HEX(xp),'~'),COALESCE(HEX(money),'~'),
COALESCE(HEX(chosenTitle),'~')),256)
SQL
}

detour_chase_snapshot_creature_restore_sql() {
  loot_fixture_world_mysql -e "
    SELECT CONCAT(
      'INSERT INTO creature (guid,id,map,zoneId,areaId,spawnDifficulties,phaseUseFlags,PhaseId,PhaseGroup,terrainSwapMap,modelid,equipment_id,position_x,position_y,position_z,orientation,spawntimesecs,wander_distance,currentwaypoint,curhealth,curmana,MovementType,npcflag,unit_flags,unit_flags2,unit_flags3,ScriptName,StringId,VerifiedBuild) VALUES (',
      QUOTE(guid),',',QUOTE(id),',',QUOTE(map),',',QUOTE(zoneId),',',QUOTE(areaId),',',
      QUOTE(spawnDifficulties),',',QUOTE(phaseUseFlags),',',QUOTE(PhaseId),',',
      QUOTE(PhaseGroup),',',QUOTE(terrainSwapMap),',',QUOTE(modelid),',',
      QUOTE(equipment_id),',',QUOTE(position_x),',',QUOTE(position_y),',',
      QUOTE(position_z),',',QUOTE(orientation),',',QUOTE(spawntimesecs),',',
      QUOTE(wander_distance),',',QUOTE(currentwaypoint),',',QUOTE(curhealth),',',
      QUOTE(curmana),',',QUOTE(MovementType),',',
      IF(npcflag IS NULL,'NULL',QUOTE(npcflag)),',',
      IF(unit_flags IS NULL,'NULL',QUOTE(unit_flags)),',',
      IF(unit_flags2 IS NULL,'NULL',QUOTE(unit_flags2)),',',
      IF(unit_flags3 IS NULL,'NULL',QUOTE(unit_flags3)),',',
      QUOTE(ScriptName),',',IF(StringId IS NULL,'NULL',QUOTE(StringId)),',',
      QUOTE(VerifiedBuild),');')
    FROM creature WHERE guid = ${DETOUR_FIXTURE_CREATURE_GUID}"
}

detour_chase_write_fixture_journal() {
  local write_mode="${1:-create}"
  local journal="$WOW_BOT_FIXTURE_JOURNAL"
  local journal_parent stage character_aux_stage respawn_stage account_stage
  local creature_restore_stage character_restore_stage character_post_predicate_stage
  local old_identity="" old_sha="" restore_xtrace=0

  case "$write_mode" in
    create|replace) ;;
    *) return 1 ;;
  esac
  if [[ "$-" == *x* ]]; then
    restore_xtrace=1
    set +x
  fi

  journal_parent="$(dirname -- "$journal")"
  stage="$(mktemp "${journal_parent}/.detour-fixture-journal.XXXXXX")" \
    || return 1
  character_aux_stage="$(
    mktemp "${journal_parent}/.detour-character-aux.XXXXXX"
  )" || {
    rm -f -- "$stage"
    return 1
  }
  respawn_stage="$(mktemp "${journal_parent}/.detour-respawn.XXXXXX")" || {
    rm -f -- "$stage" "$character_aux_stage"
    return 1
  }
  account_stage="$(mktemp "${journal_parent}/.detour-account.XXXXXX")" || {
    rm -f -- "$stage" "$character_aux_stage" "$respawn_stage"
    return 1
  }
  creature_restore_stage="$(mktemp "${journal_parent}/.detour-creature-restore.XXXXXX")" || {
    rm -f -- "$stage" "$character_aux_stage" "$respawn_stage" "$account_stage"
    return 1
  }
  character_restore_stage="$(mktemp "${journal_parent}/.detour-character-restore.XXXXXX")" || {
    rm -f -- "$stage" "$character_aux_stage" "$respawn_stage" "$account_stage" \
      "$creature_restore_stage"
    return 1
  }
  character_post_predicate_stage="$(
    mktemp "${journal_parent}/.detour-character-post-predicate.XXXXXX"
  )" || {
    rm -f -- "$stage" "$character_aux_stage" "$respawn_stage" "$account_stage" \
      "$creature_restore_stage" "$character_restore_stage"
    return 1
  }
  chmod 600 "$stage" "$character_aux_stage" "$respawn_stage" "$account_stage" \
    "$creature_restore_stage" "$character_restore_stage" \
    "$character_post_predicate_stage" \
    || {
    rm -f -- \
      "$stage" "$character_aux_stage" "$respawn_stage" "$account_stage" \
      "$creature_restore_stage" "$character_restore_stage" \
      "$character_post_predicate_stage"
    return 1
  }
  printf '%s\n' "$DETOUR_FIXTURE_CHARACTER_AUX_SNAPSHOTS_JSON" \
    >"$character_aux_stage" \
    && printf '%s\n' "$DETOUR_FIXTURE_RESPAWN_SNAPSHOT_JSON" \
      >"$respawn_stage" \
    && printf '%s\n' "$DETOUR_FIXTURE_ACCOUNT_SNAPSHOTS_JSON" \
      >"$account_stage" \
    && printf '%s' "$DETOUR_FIXTURE_CREATURE_RESTORE_SQL" \
      >"$creature_restore_stage" \
    && printf '%s' "$DETOUR_FIXTURE_CHARACTER_RESTORE_SQL" \
      >"$character_restore_stage" \
    && printf '%s' "$DETOUR_FIXTURE_POST_CHARACTER_PREDICATE_SQL" \
      >"$character_post_predicate_stage" || {
      rm -f -- \
        "$stage" "$character_aux_stage" "$respawn_stage" "$account_stage" \
        "$creature_restore_stage" "$character_restore_stage" \
        "$character_post_predicate_stage"
      return 1
    }
  if ! jq -n \
      --arg flow "$DETOUR_FIXTURE_FLOW" \
      --arg side "$DETOUR_FIXTURE_SIDE" \
      --arg normal_data_dir "$DETOUR_FIXTURE_NORMAL_DATA_DIR" \
      --arg private_data_dir "$DETOUR_FIXTURE_PRIVATE_DATA_DIR" \
      --arg private_data_dir_identity "$DETOUR_FIXTURE_PRIVATE_DATA_DIR_IDENTITY" \
      --arg manifest "$DETOUR_FIXTURE_MANIFEST" \
      --arg manifest_sha256 "$DETOUR_FIXTURE_MANIFEST_SHA256" \
      --arg prior_creature_sha256 "$DETOUR_FIXTURE_PRIOR_CREATURE_SHA256" \
      --rawfile creature_restore_sql "$creature_restore_stage" \
      --arg fixture_creature_sha256 "$DETOUR_FIXTURE_FIXTURE_CREATURE_SHA256" \
      --arg character_identity_sha256 "$DETOUR_FIXTURE_CHARACTER_IDENTITY_SHA256" \
      --arg character_stable_sha256 "$DETOUR_FIXTURE_CHARACTER_STABLE_SHA256" \
      --arg prior_character_sha256 "$DETOUR_FIXTURE_PRIOR_CHARACTER_SHA256" \
      --rawfile character_restore_sql "$character_restore_stage" \
      --arg post_character_sha256 "$DETOUR_FIXTURE_POST_CHARACTER_SHA256" \
      --rawfile post_character_predicate_sql "$character_post_predicate_stage" \
      --slurpfile character_aux "$character_aux_stage" \
      --slurpfile respawn "$respawn_stage" \
      --slurpfile account_snapshots "$account_stage" \
      --argjson bnet_account_id "$DETOUR_FIXTURE_BNET_ACCOUNT_ID" \
      --arg world_aux_sha256 "$DETOUR_FIXTURE_WORLD_AUX_SHA256" \
      --arg post_world_aux_sha256 "$DETOUR_FIXTURE_POST_WORLD_AUX_SHA256" \
      --arg database_snapshot_sha256 "$DETOUR_FIXTURE_DATABASE_SNAPSHOT_SHA256" \
      --arg db_conf "$DETOUR_FIXTURE_DB_CONF" \
      --arg db_conf_sha256 "$DETOUR_FIXTURE_DB_CONF_SHA256" \
      --arg db_conf_identity "$DETOUR_FIXTURE_DB_CONF_IDENTITY" \
      --arg orchestration_lock "$DETOUR_FIXTURE_ORCHESTRATION_LOCK" \
      --arg pm2_rust_world "$DETOUR_FIXTURE_PM2_RUST_WORLD" \
      --arg pm2_cpp_world "$DETOUR_FIXTURE_PM2_CPP_WORLD" \
      --arg pm2_restore_file "$DETOUR_FIXTURE_PM2_RESTORE_FILE" \
      --arg pm2_restore_file_sha256 "$DETOUR_FIXTURE_PM2_RESTORE_FILE_SHA256" \
      --arg pm2_restore_file_identity "$DETOUR_FIXTURE_PM2_RESTORE_FILE_IDENTITY" \
      --arg normal_rust_pm2_profile_sha256 "$DETOUR_FIXTURE_NORMAL_RUST_PM2_PROFILE_SHA256" \
      --arg normal_rust_config "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG" \
      --arg normal_rust_config_sha256 "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG_SHA256" \
      --arg normal_rust_config_identity "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG_IDENTITY" \
      --arg capture_config_file "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE" \
      --arg capture_config_file_sha256 "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE_SHA256" \
      --arg capture_config_file_identity "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE_IDENTITY" \
      --arg rust_config "$DETOUR_FIXTURE_RUST_CONFIG" \
      --arg rust_config_sha256 "$DETOUR_FIXTURE_RUST_CONFIG_SHA256" \
      --arg rust_config_identity "$DETOUR_FIXTURE_RUST_CONFIG_IDENTITY" \
      --arg cpp_config "$DETOUR_FIXTURE_CPP_CONFIG" \
      --arg cpp_config_backup "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP" \
      --arg cpp_config_backup_identity "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP_IDENTITY" \
      --arg cpp_config_backup_sha256 "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP_SHA256" \
      --argjson world_port "$DETOUR_FIXTURE_WORLD_PORT" \
      --argjson instance_port "$DETOUR_FIXTURE_INSTANCE_PORT" \
      --argjson created_by_pid "$$" \
      --argjson prior_creature_exists \
        "$([ "$DETOUR_FIXTURE_PRIOR_CREATURE_EXISTS" = 1 ] && printf true || printf false)" \
      --argjson db_applied \
        "$([ "$DETOUR_FIXTURE_DB_APPLIED" = 1 ] && printf true || printf false)" \
      --argjson poststate_checkpointed \
        "$([ "$DETOUR_FIXTURE_POSTSTATE_CHECKPOINTED" = 1 ] && printf true || printf false)" \
      --argjson db_restored \
        "$([ "$DETOUR_FIXTURE_DB_RESTORED" = 1 ] && printf true || printf false)" \
      --argjson filesystem_restored \
        "$([ "$DETOUR_FIXTURE_FILESYSTEM_RESTORED" = 1 ] && printf true || printf false)" \
      --argjson normal_runtime_restored \
        "$([ "$DETOUR_FIXTURE_NORMAL_RUNTIME_RESTORED" = 1 ] && printf true || printf false)" '
        {
          version: 3,
          contract: "detour-chase-around-obstacle-shell-fixture-v1",
          flow: $flow,
          side: $side,
          created_by_pid: $created_by_pid,
          normal_data_dir: $normal_data_dir,
          private_data_dir: $private_data_dir,
          private_data_dir_identity: $private_data_dir_identity,
          fixture_manifest: $manifest,
          fixture_manifest_sha256: $manifest_sha256,
          creature: {
            guid: 9102401,
            entry: 15271,
            prior_exists: $prior_creature_exists,
            prior_sha256: $prior_creature_sha256,
            restore_sql: $creature_restore_sql,
            fixture_sha256: $fixture_creature_sha256
          },
          character: {
            guid: 15,
            account_id: 9,
            identity_sha256: $character_identity_sha256,
            stable_sha256: $character_stable_sha256,
            prior_sha256: $prior_character_sha256,
            restore_sql: $character_restore_sql,
            post_sha256: $post_character_sha256,
            post_predicate_sql: $post_character_predicate_sql
          },
          character_aux: $character_aux[0],
          respawn: $respawn[0],
          bnet_account_id: $bnet_account_id,
          account_snapshots: $account_snapshots[0],
          world_aux_sha256: $world_aux_sha256,
          post_world_aux_sha256: $post_world_aux_sha256,
          database_snapshot_sha256: $database_snapshot_sha256,
          db_applied: $db_applied,
          phases: {
            poststate_checkpointed: $poststate_checkpointed,
            db_restored: $db_restored,
            filesystem_restored: $filesystem_restored,
            normal_runtime_restored: $normal_runtime_restored
          },
          recovery: {
            db_conf: $db_conf,
            db_conf_sha256: $db_conf_sha256,
            db_conf_identity: $db_conf_identity,
            orchestration_lock: $orchestration_lock,
            pm2_rust_world: $pm2_rust_world,
            pm2_cpp_world: $pm2_cpp_world,
            world_port: $world_port,
            instance_port: $instance_port,
            pm2_restore_file: $pm2_restore_file,
            pm2_restore_file_sha256: $pm2_restore_file_sha256,
            pm2_restore_file_identity: $pm2_restore_file_identity,
            normal_rust_pm2_profile_sha256: $normal_rust_pm2_profile_sha256,
            normal_rust_config: $normal_rust_config,
            normal_rust_config_sha256: $normal_rust_config_sha256,
            normal_rust_config_identity: $normal_rust_config_identity,
            capture_config_file: $capture_config_file,
            capture_config_file_sha256: $capture_config_file_sha256,
            capture_config_file_identity: $capture_config_file_identity,
            rust_config: $rust_config,
            rust_config_sha256: $rust_config_sha256,
            rust_config_identity: $rust_config_identity,
            cpp_config: $cpp_config,
            cpp_config_backup: $cpp_config_backup,
            cpp_config_backup_identity: $cpp_config_backup_identity,
            cpp_config_backup_sha256: $cpp_config_backup_sha256
          }
        }
      ' >"$stage"; then
    rm -f -- \
      "$stage" "$character_aux_stage" "$respawn_stage" "$account_stage" \
      "$creature_restore_stage" "$character_restore_stage" \
      "$character_post_predicate_stage"
    return 1
  fi
  rm -f -- "$character_aux_stage" "$respawn_stage" "$account_stage" \
    "$creature_restore_stage" "$character_restore_stage" \
    "$character_post_predicate_stage" || {
    rm -f -- "$stage"
    return 1
  }
  sync -f "$stage" || {
    rm -f -- "$stage"
    return 1
  }
  if [ "$write_mode" = create ]; then
    if [ -e "$journal" ] || [ -L "$journal" ]; then
      echo "error: refusing to replace an existing detour recovery journal" >&2
      rm -f -- "$stage"
      return 1
    fi
  else
    [ -f "$journal" ] && [ ! -L "$journal" ] \
      && [ "$(stat -c '%a' -- "$journal" 2>/dev/null)" = 600 ] \
      && [[ "$DETOUR_FIXTURE_JOURNAL_SHA256" =~ ^[0-9a-f]{64}$ ]] || {
        rm -f -- "$stage"
        return 1
      }
    old_identity="$(stat -c '%d:%i' -- "$journal")" || {
      rm -f -- "$stage"
      return 1
    }
    old_sha="$(detour_chase_sha256_of_file "$journal")" || {
      rm -f -- "$stage"
      return 1
    }
    [ "$old_sha" = "$DETOUR_FIXTURE_JOURNAL_SHA256" ] \
      && [ "$(stat -c '%d:%i' -- "$journal")" = "$old_identity" ] || {
        echo "error: detour recovery journal changed before atomic phase update" >&2
        rm -f -- "$stage"
        return 1
      }
  fi
  if [ "$write_mode" = create ]; then
    # `mv --no-clobber` reports success even when it declines to move. A hard
    # link gives us an atomic O_EXCL-like publication on this same filesystem:
    # EEXIST is an actual failure and the already-fsynced inode is unchanged.
    if ! ln -- "$stage" "$journal"; then
      rm -f -- "$stage"
      return 1
    fi
    rm -- "$stage" || return 1
  elif ! mv -f -- "$stage" "$journal"; then
    rm -f -- "$stage"
    return 1
  fi
  sync -f "$journal" && sync -f "$journal_parent" || return 1
  [ "$(stat -c '%a' -- "$journal")" = 600 ] \
    && [ ! -L "$journal" ] || return 1
  DETOUR_FIXTURE_JOURNAL_SHA256="$(detour_chase_sha256_of_file "$journal")" \
    || return 1
  if [ "$restore_xtrace" -eq 1 ]; then
    set -x
  fi
}

detour_chase_apply_fixture_guard() {
  local creature_count template_count character_count character_safety
  local creature_expression identity_expression stable_expression
  local updated current_sha disposable_side_rows world_aux_state
  local auth_identity_count account_character_count bnet_game_account_count
  local bnet_battle_pet_count restore_xtrace=0

  [ "$DETOUR_FIXTURE_ENABLED" = "1" ] || return 0
  if [[ "$-" == *x* ]]; then
    restore_xtrace=1
    set +x
  fi
  [ -n "$DETOUR_FIXTURE_PRIVATE_DATA_DIR" ] || return 1
  loot_fixture_wait_until_all_characters_offline || return 1
  detour_chase_load_auth_database_credentials || return 1

  creature_expression="$(detour_chase_creature_state_sha_query)" || return 1
  identity_expression="$(detour_chase_character_identity_sha_query)" || return 1
  stable_expression="$(detour_chase_character_stable_sha_query)" || return 1

  template_count="$(loot_fixture_world_mysql -e \
    "SELECT COUNT(*) FROM creature_template WHERE entry=${DETOUR_FIXTURE_CREATURE_ENTRY}")" \
    || return 1
  [ "$template_count" = 1 ] || {
    echo "error: detour creature template ${DETOUR_FIXTURE_CREATURE_ENTRY} is not unique/present" >&2
    return 1
  }
  creature_count="$(loot_fixture_world_mysql -e \
    "SELECT COUNT(*) FROM creature WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID}")" \
    || return 1
  case "$creature_count" in
    0)
      DETOUR_FIXTURE_PRIOR_CREATURE_EXISTS=0
      DETOUR_FIXTURE_PRIOR_CREATURE_SHA256=""
      DETOUR_FIXTURE_CREATURE_RESTORE_SQL=""
      ;;
    1)
      DETOUR_FIXTURE_PRIOR_CREATURE_EXISTS=1
      DETOUR_FIXTURE_PRIOR_CREATURE_SHA256="$(loot_fixture_world_mysql -e \
        "SELECT ${creature_expression} FROM creature WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID}")" \
        || return 1
      DETOUR_FIXTURE_CREATURE_RESTORE_SQL="$(
        detour_chase_snapshot_creature_restore_sql
      )" || return 1
      [ -n "$DETOUR_FIXTURE_PRIOR_CREATURE_SHA256" ] \
        && [ -n "$DETOUR_FIXTURE_CREATURE_RESTORE_SQL" ] || return 1
      ;;
    *)
      echo "error: detour creature guid is not unique" >&2
      return 1
      ;;
  esac

  character_count="$(loot_fixture_character_mysql -e \
    "SELECT COUNT(*) FROM characters WHERE guid=${DETOUR_FIXTURE_CHARACTER_GUID}")" \
    || return 1
  [ "$character_count" = 1 ] || {
    echo "error: detour character guid ${DETOUR_FIXTURE_CHARACTER_GUID} is missing" >&2
    return 1
  }
  account_character_count="$(loot_fixture_character_mysql -e \
    "SELECT COUNT(*) FROM characters
      WHERE account=${DETOUR_FIXTURE_CHARACTER_ACCOUNT}
        AND deleteDate IS NULL")" || return 1
  [ "$account_character_count" = 1 ] || {
    echo "error: detour account is not exclusively owned by the one disposable character" >&2
    return 1
  }
  DETOUR_FIXTURE_BNET_ACCOUNT_ID="$(detour_chase_auth_mysql -e "
    SELECT a.battlenet_account
      FROM account a
      JOIN battlenet_accounts b ON b.id=a.battlenet_account
     WHERE a.id=${DETOUR_FIXTURE_CHARACTER_ACCOUNT}
       AND LOWER(b.email)='testbot2@bot.local'
       AND a.online=0 AND b.online=0")" || return 1
  [[ "$DETOUR_FIXTURE_BNET_ACCOUNT_ID" =~ ^[1-9][0-9]*$ ]] || {
    echo "error: detour auth account/BNet identity is missing, ambiguous, or online" >&2
    return 1
  }
  auth_identity_count="$(detour_chase_auth_mysql -e "
    SELECT
      (SELECT COUNT(*) FROM account
        WHERE id=${DETOUR_FIXTURE_CHARACTER_ACCOUNT}
          AND battlenet_account=${DETOUR_FIXTURE_BNET_ACCOUNT_ID})
      + (SELECT COUNT(*) FROM battlenet_accounts
        WHERE id=${DETOUR_FIXTURE_BNET_ACCOUNT_ID}
          AND LOWER(email)='testbot2@bot.local')")" || return 1
  [ "$auth_identity_count" = 2 ] || return 1
  bnet_game_account_count="$(detour_chase_auth_mysql -e "
    SELECT COUNT(*) FROM account
     WHERE battlenet_account=${DETOUR_FIXTURE_BNET_ACCOUNT_ID}")" || return 1
  [ "$bnet_game_account_count" = 1 ] || {
    echo "error: detour BNet identity owns another game account; fixture is not exclusive" >&2
    return 1
  }
  bnet_battle_pet_count="$(detour_chase_auth_mysql -e "
    SELECT COUNT(*) FROM battle_pets
     WHERE battlenetAccountId=${DETOUR_FIXTURE_BNET_ACCOUNT_ID}")" || return 1
  [ "$bnet_battle_pet_count" = 0 ] || {
    echo "error: detour BNet identity owns battle pets; fixture requires an empty pet collection" >&2
    return 1
  }
  character_safety="$(loot_fixture_character_mysql -e \
    "SELECT COUNT(*) FROM characters
      WHERE guid=${DETOUR_FIXTURE_CHARACTER_GUID}
        AND account=${DETOUR_FIXTURE_CHARACTER_ACCOUNT}
        AND online=0 AND at_login=0 AND deleteDate IS NULL AND health > 0")" \
    || return 1
  [ "$character_safety" = 1 ] || {
    echo "error: detour character owner/offline/alive/at_login safety contract failed" >&2
    return 1
  }
  disposable_side_rows="$(loot_fixture_character_mysql -e "
    SELECT
      (SELECT COUNT(*) FROM character_inventory
        WHERE guid=${DETOUR_FIXTURE_CHARACTER_GUID})
      + (SELECT COUNT(*) FROM item_instance
        WHERE owner_guid=${DETOUR_FIXTURE_CHARACTER_GUID})
      + (SELECT COUNT(*) FROM character_pet
        WHERE owner=${DETOUR_FIXTURE_CHARACTER_GUID})
      + (SELECT COUNT(*) FROM character_pet_declinedname
        WHERE owner=${DETOUR_FIXTURE_CHARACTER_GUID})
      + (SELECT COUNT(*) FROM group_member
        WHERE memberGuid=${DETOUR_FIXTURE_CHARACTER_GUID})
      + (SELECT COUNT(*) FROM guild_member
        WHERE guid=${DETOUR_FIXTURE_CHARACTER_GUID})
      + (SELECT COUNT(*) FROM corpse
        WHERE guid=${DETOUR_FIXTURE_CHARACTER_GUID})
      + (SELECT COUNT(*) FROM corpse_customizations
        WHERE ownerGuid=${DETOUR_FIXTURE_CHARACTER_GUID})
      + (SELECT COUNT(*) FROM corpse_phases
        WHERE OwnerGuid=${DETOUR_FIXTURE_CHARACTER_GUID})")" || return 1
  [ "$disposable_side_rows" = 0 ] || {
    echo "error: detour character has inventory/pet/group/guild/corpse side state; fixture must remain disposable" >&2
    return 1
  }
  DETOUR_FIXTURE_CHARACTER_IDENTITY_SHA256="$(loot_fixture_character_mysql -e \
    "SELECT ${identity_expression} FROM characters WHERE guid=${DETOUR_FIXTURE_CHARACTER_GUID}")" \
    || return 1
  DETOUR_FIXTURE_CHARACTER_STABLE_SHA256="$(loot_fixture_character_mysql -e \
    "SELECT ${stable_expression} FROM characters WHERE guid=${DETOUR_FIXTURE_CHARACTER_GUID}")" \
    || return 1
  DETOUR_FIXTURE_CHARACTER_RESTORE_SQL="$(
    detour_chase_snapshot_single_character_update_sql
  )" || return 1
  DETOUR_FIXTURE_PRIOR_CHARACTER_SHA256="$(
    detour_chase_sha256_of_text "$DETOUR_FIXTURE_CHARACTER_RESTORE_SQL"
  )" || return 1
  DETOUR_FIXTURE_CHARACTER_PRIOR_PREDICATE_SQL="$(
    detour_chase_snapshot_single_character_predicate_sql
  )" || return 1
  DETOUR_FIXTURE_CHARACTER_AUX_SNAPSHOTS_JSON="$(
    detour_chase_snapshot_character_auxiliary_state
  )" || return 1
  DETOUR_FIXTURE_RESPAWN_SNAPSHOT_JSON="$(
    detour_chase_snapshot_respawn_state
  )" || return 1
  DETOUR_FIXTURE_ACCOUNT_SNAPSHOTS_JSON="$(
    detour_chase_snapshot_account_state
  )" || return 1
  world_aux_state="$(detour_chase_world_auxiliary_state)" || return 1
  [ "$world_aux_state" = "0#0#0#0#0#0#0#0#0#0#0" ] || {
    echo "error: reserved detour spawn has world auxiliary rows; refusing ambiguous fixture activation" >&2
    return 1
  }
  DETOUR_FIXTURE_WORLD_AUX_SHA256="$(
    detour_chase_sha256_of_text "$world_aux_state"
  )" || return 1
  [ -n "$DETOUR_FIXTURE_CHARACTER_IDENTITY_SHA256" ] \
    && [ -n "$DETOUR_FIXTURE_CHARACTER_STABLE_SHA256" ] \
    && [ -n "$DETOUR_FIXTURE_PRIOR_CHARACTER_SHA256" ] \
    && [ -n "$DETOUR_FIXTURE_CHARACTER_RESTORE_SQL" ] \
    && [ -n "$DETOUR_FIXTURE_CHARACTER_PRIOR_PREDICATE_SQL" ] \
    && jq -e 'type == "array"' \
      <<<"$DETOUR_FIXTURE_CHARACTER_AUX_SNAPSHOTS_JSON" >/dev/null \
    && jq -e '.table == "respawn" and .scope_value == 9102401' \
      <<<"$DETOUR_FIXTURE_RESPAWN_SNAPSHOT_JSON" >/dev/null \
    && jq -e 'type == "array" and length == 14' \
      <<<"$DETOUR_FIXTURE_ACCOUNT_SNAPSHOTS_JSON" >/dev/null || return 1

  # Compute the exact fixture creature fingerprint without touching the
  # reserved guid, then durably arm both restore plans before either write.
  DETOUR_FIXTURE_FIXTURE_CREATURE_SHA256="$(loot_fixture_world_mysql -e "
    SELECT ${creature_expression} FROM (
      SELECT
        CAST(${DETOUR_FIXTURE_CREATURE_GUID} AS UNSIGNED) guid,
        CAST(${DETOUR_FIXTURE_CREATURE_ENTRY} AS UNSIGNED) id,
        CAST(1 AS UNSIGNED) map, CAST(0 AS UNSIGNED) zoneId,
        CAST(0 AS UNSIGNED) areaId, CAST('0' AS CHAR) spawnDifficulties,
        CAST(0 AS UNSIGNED) phaseUseFlags, CAST(0 AS SIGNED) PhaseId,
        CAST(0 AS SIGNED) PhaseGroup, CAST(-1 AS SIGNED) terrainSwapMap,
        CAST(0 AS UNSIGNED) modelid, CAST(0 AS SIGNED) equipment_id,
        CAST(${DETOUR_FIXTURE_CREATURE_X} AS FLOAT) position_x,
        CAST(${DETOUR_FIXTURE_CREATURE_Y} AS FLOAT) position_y,
        CAST(${DETOUR_FIXTURE_CREATURE_Z} AS FLOAT) position_z,
        CAST(0 AS FLOAT) orientation, CAST(120 AS UNSIGNED) spawntimesecs,
        CAST(0 AS FLOAT) wander_distance, CAST(0 AS UNSIGNED) currentwaypoint,
        CAST(42 AS UNSIGNED) curhealth, CAST(0 AS UNSIGNED) curmana,
        CAST(0 AS UNSIGNED) MovementType, NULL npcflag, NULL unit_flags,
        NULL unit_flags2, NULL unit_flags3, CAST('' AS CHAR) ScriptName,
        NULL StringId, CAST(0 AS SIGNED) VerifiedBuild
    ) fixture")" || return 1
  [ -n "$DETOUR_FIXTURE_FIXTURE_CREATURE_SHA256" ] || return 1
  DETOUR_FIXTURE_DATABASE_SNAPSHOT_SHA256="$(
    detour_chase_compute_database_snapshot_sha256
  )" || return 1
  [[ "$DETOUR_FIXTURE_DATABASE_SNAPSHOT_SHA256" =~ ^[0-9a-f]{64}$ ]] \
    || return 1
  # `db_applied` means "one or more writes may have happened", not "both
  # writes were observed complete".  Arm that state before the first write so
  # SIGKILL at any later instruction still leads recovery through idempotent
  # per-row inspection.
  DETOUR_FIXTURE_POSTSTATE_CHECKPOINTED=0
  DETOUR_FIXTURE_POST_CHARACTER_SHA256=""
  DETOUR_FIXTURE_POST_CHARACTER_PREDICATE_SQL=""
  DETOUR_FIXTURE_POST_WORLD_AUX_SHA256=""
  DETOUR_FIXTURE_DB_APPLIED=1
  DETOUR_FIXTURE_DB_RESTORED=0
  DETOUR_FIXTURE_FILESYSTEM_RESTORED=0
  DETOUR_FIXTURE_NORMAL_RUNTIME_RESTORED=0
  detour_chase_validate_recovery_metadata || {
    echo "error: detour recovery metadata is incomplete or unsafe" >&2
    DETOUR_FIXTURE_DB_APPLIED=0
    return 1
  }
  if ! detour_chase_write_fixture_journal replace; then
    # No DB write has happened yet.  If the durable rename did happen but its
    # final fsync/validation failed, the journal itself still records
    # db_applied=true and cleanup will reload it.  If it did not, this reset
    # allows the trap to discard the unarmed filesystem fixture safely.
    DETOUR_FIXTURE_DB_APPLIED=0
    return 1
  fi

  if [ "$DETOUR_FIXTURE_PRIOR_CREATURE_EXISTS" = 0 ]; then
    updated="$(loot_fixture_world_mysql -e "
      INSERT INTO creature
        (guid,id,map,zoneId,areaId,spawnDifficulties,phaseUseFlags,PhaseId,PhaseGroup,
         terrainSwapMap,modelid,equipment_id,position_x,position_y,position_z,
         orientation,spawntimesecs,wander_distance,currentwaypoint,curhealth,
         curmana,MovementType,npcflag,unit_flags,unit_flags2,unit_flags3,
         ScriptName,StringId,VerifiedBuild)
      SELECT ${DETOUR_FIXTURE_CREATURE_GUID},${DETOUR_FIXTURE_CREATURE_ENTRY},1,0,0,
        '0',0,0,0,-1,0,0,${DETOUR_FIXTURE_CREATURE_X},
        ${DETOUR_FIXTURE_CREATURE_Y},${DETOUR_FIXTURE_CREATURE_Z},0,120,0,0,42,0,0,
        NULL,NULL,NULL,NULL,'',NULL,0
      WHERE NOT EXISTS (
        SELECT 1 FROM creature WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID});
      SELECT ROW_COUNT();")" || return 1
  else
    if [ "$DETOUR_FIXTURE_PRIOR_CREATURE_SHA256" \
        = "$DETOUR_FIXTURE_FIXTURE_CREATURE_SHA256" ]; then
      updated=1
    else
      updated="$(loot_fixture_world_mysql -e "
        UPDATE creature SET
          id=${DETOUR_FIXTURE_CREATURE_ENTRY},map=1,zoneId=0,areaId=0,
          spawnDifficulties='0',phaseUseFlags=0,PhaseId=0,PhaseGroup=0,
          terrainSwapMap=-1,modelid=0,equipment_id=0,
          position_x=${DETOUR_FIXTURE_CREATURE_X},
          position_y=${DETOUR_FIXTURE_CREATURE_Y},
          position_z=${DETOUR_FIXTURE_CREATURE_Z},
          orientation=0,spawntimesecs=120,wander_distance=0,currentwaypoint=0,
          curhealth=42,curmana=0,MovementType=0,npcflag=NULL,unit_flags=NULL,
          unit_flags2=NULL,unit_flags3=NULL,ScriptName='',StringId=NULL,
          VerifiedBuild=0
        WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID}
          AND ${creature_expression}='${DETOUR_FIXTURE_PRIOR_CREATURE_SHA256}';
        SELECT ROW_COUNT();")" || return 1
    fi
  fi
  [ "$updated" = 1 ] || {
    echo "error: detour creature changed during guarded activation (ROW_COUNT=${updated:-unknown})" >&2
    return 1
  }

  updated="$(loot_fixture_character_mysql -e "
    UPDATE characters SET
      level=1,xp=0,map=1,zone=0,instance_id=0,
      position_x=${DETOUR_FIXTURE_PLAYER_X},
      position_y=${DETOUR_FIXTURE_PLAYER_Y},
      position_z=${DETOUR_FIXTURE_PLAYER_Z},
      orientation=${DETOUR_FIXTURE_PLAYER_O},
      online=0,health=4294967295,
      trans_x=0,trans_y=0,trans_z=0,trans_o=0,transguid=0,
      taxi_path=NULL,death_expire_time=0
    WHERE guid=${DETOUR_FIXTURE_CHARACTER_GUID}
      AND account=${DETOUR_FIXTURE_CHARACTER_ACCOUNT}
      AND online=0 AND at_login=0 AND deleteDate IS NULL
      AND ${identity_expression}='${DETOUR_FIXTURE_CHARACTER_IDENTITY_SHA256}'
      AND ${stable_expression}='${DETOUR_FIXTURE_CHARACTER_STABLE_SHA256}'
      AND (${DETOUR_FIXTURE_CHARACTER_PRIOR_PREDICATE_SQL});
    SELECT ROW_COUNT();")" || return 1
  if [ "$updated" != 1 ]; then
    [ "$updated" = 0 ] \
      && [ "$(loot_fixture_character_mysql -e "
        SELECT COUNT(*) FROM characters
         WHERE guid=${DETOUR_FIXTURE_CHARACTER_GUID}
           AND account=${DETOUR_FIXTURE_CHARACTER_ACCOUNT}
           AND online=0 AND at_login=0 AND deleteDate IS NULL
           AND level=1 AND xp=0 AND map=1 AND zone=0 AND instance_id=0
           AND position_x=CAST(${DETOUR_FIXTURE_PLAYER_X} AS FLOAT)
           AND position_y=CAST(${DETOUR_FIXTURE_PLAYER_Y} AS FLOAT)
           AND position_z=CAST(${DETOUR_FIXTURE_PLAYER_Z} AS FLOAT)
           AND orientation=CAST(${DETOUR_FIXTURE_PLAYER_O} AS FLOAT)
           AND health=4294967295
           AND trans_x=0 AND trans_y=0 AND trans_z=0 AND trans_o=0
           AND transguid=0 AND taxi_path IS NULL AND death_expire_time=0
           AND ${identity_expression}='${DETOUR_FIXTURE_CHARACTER_IDENTITY_SHA256}'
           AND ${stable_expression}='${DETOUR_FIXTURE_CHARACTER_STABLE_SHA256}'")" = 1 ] \
      || {
        echo "error: detour character changed during guarded activation (ROW_COUNT=${updated:-unknown})" >&2
        return 1
      }
  fi
  current_sha="$(loot_fixture_world_mysql -e \
    "SELECT ${creature_expression} FROM creature WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID}")" \
    || return 1
  [ "$current_sha" = "$DETOUR_FIXTURE_FIXTURE_CREATURE_SHA256" ] || {
    echo "error: activated detour creature differs from the journaled fixture" >&2
    return 1
  }
  echo "detour fixture: private DataDir and guarded creature/character DB state armed"
  if [ "$restore_xtrace" -eq 1 ]; then
    set -x
  fi
}
