detour_chase_load_fixture_journal() {
  local journal="$WOW_BOT_FIXTURE_JOURNAL"
  local expected_side="$DETOUR_FIXTURE_SIDE"
  local restore_xtrace=0

  if [[ "$-" == *x* ]]; then
    restore_xtrace=1
    set +x
  fi

  [ -f "$journal" ] && [ ! -L "$journal" ] \
    && [ "$(stat -c '%a' -- "$journal" 2>/dev/null)" = 600 ] || return 1
  jq -e \
    --arg flow "$DETOUR_FIXTURE_FLOW" \
    --arg side "$expected_side" '
      .phases.poststate_checkpointed as $checkpointed
      | .version == 3
      and .contract == "detour-chase-around-obstacle-shell-fixture-v1"
      and .flow == $flow
      and (.side == "cpp" or .side == "rust")
      and ($side == "" or .side == $side)
      and .creature.guid == 9102401 and .creature.entry == 15271
      and (.creature.prior_exists | type == "boolean")
      and (.creature.prior_sha256 | type == "string")
      and (.creature.restore_sql | type == "string")
      and .character.guid == 15 and .character.account_id == 9
      and (.character_aux | type == "array")
      and (.respawn | type == "object")
      and (.account_snapshots | type == "array")
      and (.db_applied | type == "boolean")
      and .phases == {
        poststate_checkpointed:.phases.poststate_checkpointed,
        db_restored:.phases.db_restored,
        filesystem_restored:.phases.filesystem_restored,
        normal_runtime_restored:.phases.normal_runtime_restored
      }
      and all(.phases[]; type == "boolean")
      and ((.phases.db_restored | not) or .phases.poststate_checkpointed)
      and ((.phases.filesystem_restored | not)
        or .phases.db_restored or (.db_applied | not))
      and ((.phases.normal_runtime_restored | not) or .phases.filesystem_restored)
      and (.recovery | type == "object")
      and (.recovery.db_conf | type == "string" and startswith("/"))
      and (.recovery.db_conf_sha256 | test("^[0-9a-f]{64}$"))
      and (.recovery.db_conf_identity
        | type == "string" and test("^[0-9]+:[0-9]+$"))
      and (.recovery.normal_rust_config
        | type == "string" and startswith("/"))
      and (.recovery.normal_rust_config_sha256
        | test("^[0-9a-f]{64}$"))
      and (.recovery.normal_rust_config_identity
        | type == "string" and test("^[0-9]+:[0-9]+$"))
      and (if .side == "rust" then
          (.recovery.pm2_restore_file
            | type == "string" and startswith("/"))
          and (.recovery.pm2_restore_file_sha256
            | test("^[0-9a-f]{64}$"))
          and (.recovery.pm2_restore_file_identity
            | type == "string" and test("^[0-9]+:[0-9]+$"))
          and (.recovery.capture_config_file
            | type == "string" and startswith("/"))
          and (.recovery.capture_config_file_sha256
            | test("^[0-9a-f]{64}$"))
          and (.recovery.capture_config_file_identity
            | type == "string" and test("^[0-9]+:[0-9]+$"))
          and (if .recovery.rust_config == "" then
              (.db_applied | not)
              and .recovery.rust_config_sha256 == ""
              and .recovery.rust_config_identity == ""
            else
              (.recovery.rust_config
                | type == "string" and startswith("/"))
              and (.recovery.rust_config_sha256
                | test("^[0-9a-f]{64}$"))
              and (.recovery.rust_config_identity
                | type == "string" and test("^[0-9]+:[0-9]+$"))
            end)
        else
          (.recovery.cpp_config
            | type == "string" and startswith("/"))
          and (.recovery.cpp_config_backup
            | type == "string" and startswith("/"))
          and (.recovery.cpp_config_backup_sha256
            | test("^[0-9a-f]{64}$"))
          and (.recovery.cpp_config_backup_identity
            | type == "string" and test("^[0-9]+:[0-9]+$"))
        end)
      and if .db_applied then
        (.creature.fixture_sha256 | test("^[0-9a-f]{64}$"))
        and (.character.identity_sha256 | test("^[0-9a-f]{64}$"))
        and (.character.stable_sha256 | test("^[0-9a-f]{64}$"))
        and (.character.prior_sha256 | test("^[0-9a-f]{64}$"))
        and (.character.restore_sql | type == "string" and length > 0)
        and (.world_aux_sha256 | test("^[0-9a-f]{64}$"))
        and (.database_snapshot_sha256 | test("^[0-9a-f]{64}$"))
        and .respawn.table == "respawn"
        and .respawn.scope_column == "spawnId"
        and .respawn.scope_value == 9102401
        and (.respawn.prior_sha256 | test("^[0-9a-f]{64}$"))
        and (.respawn.restore_sql | type == "string")
        and (.respawn.predicate_sql | type == "string" and length > 0)
        and (.bnet_account_id | type == "number" and . > 0)
        and (.recovery.orchestration_lock | type == "string" and startswith("/"))
        and (.recovery.pm2_rust_world | type == "string" and length > 0)
        and (.recovery.pm2_cpp_world | type == "string" and length > 0)
        and (.recovery.normal_rust_pm2_profile_sha256
          | test("^[0-9a-f]{64}$"))
        and (.recovery.world_port | type == "number" and . > 0 and . <= 65535)
        and (.recovery.instance_port | type == "number" and . > 0 and . <= 65535)
        and .recovery.world_port != .recovery.instance_port
        and (.account_snapshots | length == 14)
        and (.account_snapshots | map({
          database,strategy,table,scope_column,scope_value
        })) == [
          {database:"auth",strategy:"update",table:"account",
            scope_column:"id",scope_value:9},
          {database:"auth",strategy:"update",table:"battlenet_accounts",
            scope_column:"id",scope_value:.bnet_account_id},
          {database:"auth",strategy:"delete_insert",
            table:"battlenet_account_toys",
            scope_column:"accountId",scope_value:.bnet_account_id},
          {database:"auth",strategy:"delete_insert",
            table:"battlenet_account_heirlooms",
            scope_column:"accountId",scope_value:.bnet_account_id},
          {database:"auth",strategy:"delete_insert",
            table:"battlenet_account_mounts",
            scope_column:"battlenetAccountId",
            scope_value:.bnet_account_id},
          {database:"auth",strategy:"delete_insert",
            table:"battlenet_account_transmog_illusions",
            scope_column:"battlenetAccountId",
            scope_value:.bnet_account_id},
          {database:"auth",strategy:"delete_insert",
            table:"battlenet_item_appearances",
            scope_column:"battlenetAccountId",
            scope_value:.bnet_account_id},
          {database:"auth",strategy:"delete_insert",
            table:"battlenet_item_favorite_appearances",
            scope_column:"battlenetAccountId",
            scope_value:.bnet_account_id},
          {database:"auth",strategy:"delete_insert",
            table:"battle_pet_slots",
            scope_column:"battlenetAccountId",
            scope_value:.bnet_account_id},
          {database:"auth",strategy:"delete_insert",
            table:"account_last_played_character",
            scope_column:"accountId",scope_value:9},
          {database:"auth",strategy:"delete_insert",table:"realmcharacters",
            scope_column:"acctid",scope_value:9},
          {database:"characters",strategy:"delete_insert",
            table:"account_data",scope_column:"accountId",scope_value:9},
          {database:"characters",strategy:"delete_insert",
            table:"account_instance_times",
            scope_column:"accountId",scope_value:9},
          {database:"characters",strategy:"delete_insert",
            table:"account_tutorial",scope_column:"accountId",scope_value:9}
        ]
        and all(.account_snapshots[];
          (.prior_sha256 | test("^[0-9a-f]{64}$"))
          and (.restore_sql | type == "string")
          and (.predicate_sql | type == "string" and length > 0)
          and (if $checkpointed then
              (.post_sha256 | test("^[0-9a-f]{64}$"))
              and (.post_predicate_sql | type == "string" and length > 0)
            else .post_sha256 == null and .post_predicate_sql == null end))
        and (if $checkpointed then
            (.character.post_sha256 | test("^[0-9a-f]{64}$"))
            and (.character.post_predicate_sql | type == "string" and length > 0)
            and (.post_world_aux_sha256 | test("^[0-9a-f]{64}$"))
            and (.respawn.post_sha256 | test("^[0-9a-f]{64}$"))
            and (.respawn.post_predicate_sql | type == "string" and length > 0)
          else
            .character.post_sha256 == ""
            and .character.post_predicate_sql == ""
            and .post_world_aux_sha256 == ""
            and .respawn.post_sha256 == null
            and .respawn.post_predicate_sql == null
          end)
      else
        (.creature.fixture_sha256 | type == "string")
        and (.character.identity_sha256 | type == "string")
        and (.character.stable_sha256 | type == "string")
        and (.character.prior_sha256 | type == "string")
        and (.character.restore_sql | type == "string")
        and (.world_aux_sha256 | type == "string")
        and (.database_snapshot_sha256 | type == "string")
        and (.bnet_account_id == 0)
        and (.account_snapshots == [])
        and (.phases.poststate_checkpointed | not)
        and (.phases.db_restored | not)
        and ((.phases.normal_runtime_restored | not)
          or .phases.filesystem_restored)
      end
    ' "$journal" >/dev/null || return 1
  local expected_auxiliary_scopes
  expected_auxiliary_scopes="$(
    detour_chase_expected_character_auxiliary_scopes_json
  )" || return 1
  jq -e --argjson expected "$expected_auxiliary_scopes" '
    .phases.poststate_checkpointed as $checkpointed
    | if .db_applied then
      (.character_aux | map({table,scope_column})) == $expected
      and all(.character_aux[];
        (.prior_sha256 | test("^[0-9a-f]{64}$"))
        and (.restore_sql | type == "string")
        and (.predicate_sql | type == "string" and length > 0)
        and (if $checkpointed then
            (.post_sha256 | test("^[0-9a-f]{64}$"))
            and (.post_predicate_sql | type == "string" and length > 0)
          else .post_sha256 == null and .post_predicate_sql == null end))
    else
      .character_aux == []
    end
  ' "$journal" >/dev/null || return 1
  DETOUR_FIXTURE_SIDE="$(jq -r '.side' "$journal")" || return 1
  DETOUR_FIXTURE_NORMAL_DATA_DIR="$(
    jq -r '.normal_data_dir' "$journal"
  )" || return 1
  DETOUR_FIXTURE_PRIVATE_DATA_DIR="$(
    jq -r '.private_data_dir' "$journal"
  )" || return 1
  DETOUR_FIXTURE_PRIVATE_DATA_DIR_IDENTITY="$(
    jq -r '.private_data_dir_identity' "$journal"
  )" || return 1
  DETOUR_FIXTURE_MANIFEST="$(jq -r '.fixture_manifest' "$journal")" || return 1
  DETOUR_FIXTURE_MANIFEST_SHA256="$(
    jq -r '.fixture_manifest_sha256' "$journal"
  )" || return 1
  DETOUR_FIXTURE_PRIOR_CREATURE_EXISTS="$(
    jq -r 'if .creature.prior_exists then 1 else 0 end' "$journal"
  )" || return 1
  DETOUR_FIXTURE_PRIOR_CREATURE_SHA256="$(
    jq -r '.creature.prior_sha256' "$journal"
  )" || return 1
  DETOUR_FIXTURE_CREATURE_RESTORE_SQL="$(
    jq -r '.creature.restore_sql' "$journal"
  )" || return 1
  DETOUR_FIXTURE_FIXTURE_CREATURE_SHA256="$(
    jq -r '.creature.fixture_sha256' "$journal"
  )" || return 1
  DETOUR_FIXTURE_CHARACTER_IDENTITY_SHA256="$(
    jq -r '.character.identity_sha256' "$journal"
  )" || return 1
  DETOUR_FIXTURE_CHARACTER_STABLE_SHA256="$(
    jq -r '.character.stable_sha256' "$journal"
  )" || return 1
  DETOUR_FIXTURE_PRIOR_CHARACTER_SHA256="$(
    jq -r '.character.prior_sha256' "$journal"
  )" || return 1
  DETOUR_FIXTURE_CHARACTER_RESTORE_SQL="$(
    jq -r '.character.restore_sql' "$journal"
  )" || return 1
  DETOUR_FIXTURE_POST_CHARACTER_SHA256="$(
    jq -r '.character.post_sha256' "$journal"
  )" || return 1
  DETOUR_FIXTURE_POST_CHARACTER_PREDICATE_SQL="$(
    jq -r '.character.post_predicate_sql' "$journal"
  )" || return 1
  DETOUR_FIXTURE_CHARACTER_AUX_SNAPSHOTS_JSON="$(
    jq -c '.character_aux' "$journal"
  )" || return 1
  DETOUR_FIXTURE_RESPAWN_SNAPSHOT_JSON="$(
    jq -c '.respawn' "$journal"
  )" || return 1
  DETOUR_FIXTURE_BNET_ACCOUNT_ID="$(
    jq -r '.bnet_account_id' "$journal"
  )" || return 1
  DETOUR_FIXTURE_ACCOUNT_SNAPSHOTS_JSON="$(
    jq -c '.account_snapshots' "$journal"
  )" || return 1
  DETOUR_FIXTURE_WORLD_AUX_SHA256="$(
    jq -r '.world_aux_sha256' "$journal"
  )" || return 1
  DETOUR_FIXTURE_POST_WORLD_AUX_SHA256="$(
    jq -r '.post_world_aux_sha256' "$journal"
  )" || return 1
  DETOUR_FIXTURE_DATABASE_SNAPSHOT_SHA256="$(
    jq -r '.database_snapshot_sha256' "$journal"
  )" || return 1
  DETOUR_FIXTURE_DB_APPLIED="$(
    jq -r 'if .db_applied then 1 else 0 end' "$journal"
  )" || return 1
  DETOUR_FIXTURE_POSTSTATE_CHECKPOINTED="$(
    jq -r 'if .phases.poststate_checkpointed then 1 else 0 end' "$journal"
  )" || return 1
  DETOUR_FIXTURE_DB_RESTORED="$(
    jq -r 'if .phases.db_restored then 1 else 0 end' "$journal"
  )" || return 1
  DETOUR_FIXTURE_FILESYSTEM_RESTORED="$(
    jq -r 'if .phases.filesystem_restored then 1 else 0 end' "$journal"
  )" || return 1
  DETOUR_FIXTURE_NORMAL_RUNTIME_RESTORED="$(
    jq -r 'if .phases.normal_runtime_restored then 1 else 0 end' "$journal"
  )" || return 1
  DETOUR_FIXTURE_DB_CONF="$(jq -r '.recovery.db_conf' "$journal")" || return 1
  DETOUR_FIXTURE_DB_CONF_SHA256="$(
    jq -r '.recovery.db_conf_sha256' "$journal"
  )" || return 1
  DETOUR_FIXTURE_DB_CONF_IDENTITY="$(
    jq -r '.recovery.db_conf_identity' "$journal"
  )" || return 1
  DETOUR_FIXTURE_ORCHESTRATION_LOCK="$(
    jq -r '.recovery.orchestration_lock' "$journal"
  )" || return 1
  DETOUR_FIXTURE_PM2_RUST_WORLD="$(
    jq -r '.recovery.pm2_rust_world' "$journal"
  )" || return 1
  DETOUR_FIXTURE_PM2_CPP_WORLD="$(
    jq -r '.recovery.pm2_cpp_world' "$journal"
  )" || return 1
  DETOUR_FIXTURE_WORLD_PORT="$(jq -r '.recovery.world_port' "$journal")" \
    || return 1
  DETOUR_FIXTURE_INSTANCE_PORT="$(jq -r '.recovery.instance_port' "$journal")" \
    || return 1
  DETOUR_FIXTURE_PM2_RESTORE_FILE="$(
    jq -r '.recovery.pm2_restore_file' "$journal"
  )" || return 1
  DETOUR_FIXTURE_PM2_RESTORE_FILE_SHA256="$(
    jq -r '.recovery.pm2_restore_file_sha256' "$journal"
  )" || return 1
  DETOUR_FIXTURE_PM2_RESTORE_FILE_IDENTITY="$(
    jq -r '.recovery.pm2_restore_file_identity' "$journal"
  )" || return 1
  DETOUR_FIXTURE_NORMAL_RUST_PM2_PROFILE_SHA256="$(
    jq -r '.recovery.normal_rust_pm2_profile_sha256' "$journal"
  )" || return 1
  DETOUR_FIXTURE_NORMAL_RUST_CONFIG="$(
    jq -r '.recovery.normal_rust_config' "$journal"
  )" || return 1
  DETOUR_FIXTURE_NORMAL_RUST_CONFIG_SHA256="$(
    jq -r '.recovery.normal_rust_config_sha256' "$journal"
  )" || return 1
  DETOUR_FIXTURE_NORMAL_RUST_CONFIG_IDENTITY="$(
    jq -r '.recovery.normal_rust_config_identity' "$journal"
  )" || return 1
  DETOUR_FIXTURE_CAPTURE_CONFIG_FILE="$(
    jq -r '.recovery.capture_config_file' "$journal"
  )" || return 1
  DETOUR_FIXTURE_CAPTURE_CONFIG_FILE_SHA256="$(
    jq -r '.recovery.capture_config_file_sha256' "$journal"
  )" || return 1
  DETOUR_FIXTURE_CAPTURE_CONFIG_FILE_IDENTITY="$(
    jq -r '.recovery.capture_config_file_identity' "$journal"
  )" || return 1
  DETOUR_FIXTURE_RUST_CONFIG="$(jq -r '.recovery.rust_config' "$journal")" \
    || return 1
  DETOUR_FIXTURE_RUST_CONFIG_SHA256="$(
    jq -r '.recovery.rust_config_sha256' "$journal"
  )" || return 1
  DETOUR_FIXTURE_RUST_CONFIG_IDENTITY="$(
    jq -r '.recovery.rust_config_identity' "$journal"
  )" || return 1
  DETOUR_FIXTURE_CPP_CONFIG="$(jq -r '.recovery.cpp_config' "$journal")" \
    || return 1
  DETOUR_FIXTURE_CPP_CONFIG_BACKUP="$(
    jq -r '.recovery.cpp_config_backup' "$journal"
  )" || return 1
  DETOUR_FIXTURE_CPP_CONFIG_BACKUP_IDENTITY="$(
    jq -r '.recovery.cpp_config_backup_identity' "$journal"
  )" || return 1
  DETOUR_FIXTURE_CPP_CONFIG_BACKUP_SHA256="$(
    jq -r '.recovery.cpp_config_backup_sha256' "$journal"
  )" || return 1
  DETOUR_FIXTURE_JOURNAL_SHA256="$(detour_chase_sha256_of_file "$journal")" \
    || return 1
  if [ "$restore_xtrace" -eq 1 ]; then
    set -x
  fi
}

detour_chase_merge_poststate_snapshots() {
  local prior_json="$1"
  local post_json="$2"
  local prior_file post_file output restore_xtrace=0 status

  if [[ "$-" == *x* ]]; then
    restore_xtrace=1
    set +x
  fi
  prior_file="$(detour_chase_secure_temp_file)" || return 1
  post_file="$(detour_chase_secure_temp_file)" || {
    rm -f -- "$prior_file"
    return 1
  }
  printf '%s\n' "$prior_json" >"$prior_file" \
    && printf '%s\n' "$post_json" >"$post_file" || {
      rm -f -- "$prior_file" "$post_file"
      return 1
    }
  output="$(
    jq -cn \
      --slurpfile prior "$prior_file" \
      --slurpfile post "$post_file" '
        def metadata:
          del(
            .prior_sha256,
            .restore_sql,
            .predicate_sql,
            .post_sha256,
            .post_predicate_sql
          );
        if ($prior[0] | type) != "array"
            or ($post[0] | type) != "array"
            or ($prior[0] | length) != ($post[0] | length)
          then error("snapshot domains differ")
          else [
            range(0; $prior[0] | length) as $index
            | $prior[0][$index] as $before
            | $post[0][$index] as $after
            | if ($before | metadata) != ($after | metadata)
                then error("snapshot scope metadata differs")
                else $before + {
                  post_sha256:$after.prior_sha256,
                  post_predicate_sql:$after.predicate_sql
                }
              end
          ]
        end
      '
  )"
  status=$?
  rm -f -- "$prior_file" "$post_file" || status=1
  if [ "$status" -eq 0 ]; then
    printf '%s\n' "$output"
  fi
  if [ "$restore_xtrace" -eq 1 ]; then
    set -x
  fi
  return "$status"
}

detour_chase_prior_snapshot_representation_for_digest() {
  local current_json="$1"
  local journal_json="$2"
  local current_file journal_file output status=0

  current_file="$(detour_chase_secure_temp_file)" || return 1
  journal_file="$(detour_chase_secure_temp_file)" || {
    rm -f -- "$current_file"
    return 1
  }
  printf '%s\n' "$current_json" >"$current_file" \
    && printf '%s\n' "$journal_json" >"$journal_file" || {
      rm -f -- "$current_file" "$journal_file"
      return 1
    }
  output="$(
    jq -cn \
      --slurpfile current "$current_file" \
      --slurpfile journal "$journal_file" '
        def invariant:
          del(
            .prior_sha256,
            .restore_sql,
            .predicate_sql,
            .post_sha256,
            .post_predicate_sql
          );
        def prior:
          .post_sha256 = null
          | .post_predicate_sql = null;
        if ($current[0] | type) != ($journal[0] | type)
          then error("snapshot representation type differs")
        elif ($current[0] | type) == "array"
          then if ($current[0] | map(invariant))
              != ($journal[0] | map(invariant))
            then error("snapshot prior state differs")
            else $journal[0] | map(prior)
            end
        elif ($current[0] | type) == "object"
          then if ($current[0] | invariant) != ($journal[0] | invariant)
            then error("snapshot prior state differs")
            else $journal[0] | prior
            end
        else error("unsupported snapshot representation")
        end
      '
  )" || status=$?
  rm -f -- "$current_file" "$journal_file" || status=1
  [ "$status" -eq 0 ] || return "$status"
  printf '%s\n' "$output"
}

detour_chase_checkpoint_fixture_poststate() {
  local current_character_sql current_aux_json current_respawn_json
  local current_account_json world_aux_state creature_expression current_creature_sha
  local restore_xtrace=0

  [ "$DETOUR_FIXTURE_DB_APPLIED" = 1 ] || return 0
  [ "$DETOUR_FIXTURE_DB_RESTORED" = 0 ] || return 0
  [ "$DETOUR_FIXTURE_POSTSTATE_CHECKPOINTED" = 0 ] || return 0
  if [[ "$-" == *x* ]]; then
    restore_xtrace=1
    set +x
  fi

  creature_expression="$(detour_chase_creature_state_sha_query)" || return 1
  current_creature_sha="$(loot_fixture_world_mysql -e \
    "SELECT ${creature_expression} FROM creature WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID}")" \
    || return 1
  [ "$current_creature_sha" = "$DETOUR_FIXTURE_FIXTURE_CREATURE_SHA256" ] || {
    echo "WARNING: detour creature differs before poststate checkpoint; refusing recovery" >&2
    return 1
  }
  current_character_sql="$(detour_chase_snapshot_single_character_update_sql)" \
    || return 1
  DETOUR_FIXTURE_POST_CHARACTER_SHA256="$(
    detour_chase_sha256_of_text "$current_character_sql"
  )" || return 1
  DETOUR_FIXTURE_POST_CHARACTER_PREDICATE_SQL="$(
    detour_chase_snapshot_table_cas_predicate_sql \
      loot_fixture_character_mysql characters \
      "guid=${DETOUR_FIXTURE_CHARACTER_GUID}"
  )" || return 1
  current_aux_json="$(detour_chase_snapshot_character_auxiliary_state)" \
    || return 1
  DETOUR_FIXTURE_CHARACTER_AUX_SNAPSHOTS_JSON="$(
    detour_chase_merge_poststate_snapshots \
      "$DETOUR_FIXTURE_CHARACTER_AUX_SNAPSHOTS_JSON" "$current_aux_json"
  )" || return 1
  current_respawn_json="$(detour_chase_snapshot_respawn_state)" || return 1
  DETOUR_FIXTURE_RESPAWN_SNAPSHOT_JSON="$(
    local prior_file post_file
    prior_file="$(detour_chase_secure_temp_file)" || exit 1
    post_file="$(detour_chase_secure_temp_file)" || {
      rm -f -- "$prior_file"
      exit 1
    }
    printf '%s\n' "$DETOUR_FIXTURE_RESPAWN_SNAPSHOT_JSON" >"$prior_file" \
      && printf '%s\n' "$current_respawn_json" >"$post_file" \
      && jq -cn --slurpfile prior "$prior_file" --slurpfile post "$post_file" '
        if ($prior[0] | {
              table,scope_column,scope_value
            }) != ($post[0] | {
              table,scope_column,scope_value
            })
          then error("respawn snapshot scope differs")
          else $prior[0] + {
            post_sha256:$post[0].prior_sha256,
            post_predicate_sql:$post[0].predicate_sql
          }
        end
      '
    status=$?
    rm -f -- "$prior_file" "$post_file"
    exit "$status"
  )" || return 1
  current_account_json="$(detour_chase_snapshot_account_state)" || return 1
  DETOUR_FIXTURE_ACCOUNT_SNAPSHOTS_JSON="$(
    detour_chase_merge_poststate_snapshots \
      "$DETOUR_FIXTURE_ACCOUNT_SNAPSHOTS_JSON" "$current_account_json"
  )" || return 1
  world_aux_state="$(detour_chase_world_auxiliary_state)" || return 1
  DETOUR_FIXTURE_POST_WORLD_AUX_SHA256="$(
    detour_chase_sha256_of_text "$world_aux_state"
  )" || return 1
  # The capture does not own auxiliary world rows. Any such row is external
  # drift, not a fixture mutation that recovery may delete.
  [ "$DETOUR_FIXTURE_POST_WORLD_AUX_SHA256" \
    = "$DETOUR_FIXTURE_WORLD_AUX_SHA256" ] || {
      echo "WARNING: reserved detour spawn gained world auxiliary rows; refusing cleanup writes" >&2
      return 1
    }
  DETOUR_FIXTURE_POSTSTATE_CHECKPOINTED=1
  detour_chase_write_fixture_journal replace || return 1
  if [ "$restore_xtrace" -eq 1 ]; then
    set -x
  fi
}

detour_chase_mark_db_restored() {
  [ "$DETOUR_FIXTURE_POSTSTATE_CHECKPOINTED" = 1 ] || return 1
  DETOUR_FIXTURE_DB_RESTORED=1
  detour_chase_write_fixture_journal replace
}

detour_chase_mark_filesystem_restored() {
  [ "$DETOUR_FIXTURE_DB_APPLIED" = 0 ] \
    || [ "$DETOUR_FIXTURE_DB_RESTORED" = 1 ] || return 1
  DETOUR_FIXTURE_FILESYSTEM_RESTORED=1
  detour_chase_write_fixture_journal replace
}

detour_chase_mark_normal_runtime_restored() {
  [ "$DETOUR_FIXTURE_FILESYSTEM_RESTORED" = 1 ] || return 1
  DETOUR_FIXTURE_NORMAL_RUNTIME_RESTORED=1
  detour_chase_write_fixture_journal replace
}

detour_chase_run_recovery_state_machine() {
  local normal_ready=0

  DETOUR_FIXTURE_ENABLED=1
  LOOT_FIXTURE_GUARD_ENABLED=1
  LOOT_FIXTURE_CLEANUP_MARKER="${WOW_BOT_FIXTURE_JOURNAL}.cleanup-complete"
  detour_chase_validate_recovery_anchor_files || {
    echo "error: recovery DB/normal-config provenance no longer matches the journal" >&2
    return 1
  }
  if detour_chase_recovery_runtime_is_normal probe; then
    normal_ready=1
  fi

  if [ "$DETOUR_FIXTURE_DB_APPLIED" = 1 ] \
      && [ "$DETOUR_FIXTURE_DB_RESTORED" = 0 ]; then
    [ "$normal_ready" = 0 ] || {
      echo "error: journal says DB recovery is pending but normal runtime is already online" >&2
      return 1
    }
    detour_chase_recovery_stop_capture_runtime || return 1
    LOOT_FIXTURE_DB_CONF="$DETOUR_FIXTURE_DB_CONF"
    load_loot_fixture_database_credentials || return 1
    detour_chase_restore_fixture_guard || return 1
  elif [ "$DETOUR_FIXTURE_DB_APPLIED" = 0 ] \
      && [ "$DETOUR_FIXTURE_FILESYSTEM_RESTORED" = 0 ] \
      && [ "$normal_ready" = 0 ]; then
    detour_chase_recovery_stop_capture_runtime || return 1
  fi

  if [ "$DETOUR_FIXTURE_FILESYSTEM_RESTORED" = 0 ]; then
    detour_chase_recovery_restore_filesystem || return 1
    detour_chase_mark_filesystem_restored || return 1
  fi
  if [ "$DETOUR_FIXTURE_NORMAL_RUNTIME_RESTORED" = 0 ]; then
    if [ "$normal_ready" = 0 ]; then
      detour_chase_recovery_start_normal_runtime || return 1
    else
      detour_chase_recovery_runtime_is_normal wait || return 1
    fi
    detour_chase_mark_normal_runtime_restored || return 1
  else
    detour_chase_recovery_runtime_is_normal wait || return 1
  fi
  detour_chase_complete_fixture_journal \
    && loot_fixture_bot_cleanup_complete
}

detour_chase_complete_fixture_journal() {
  local journal="$WOW_BOT_FIXTURE_JOURNAL"
  local marker="$LOOT_FIXTURE_CLEANUP_MARKER"
  local parent stage journal_sha marker_sha

  if [ ! -e "$journal" ] && [ ! -L "$journal" ]; then
    [ -f "$marker" ] && [ ! -L "$marker" ] \
      && [ "$(stat -c '%a' -- "$marker" 2>/dev/null)" = 600 ] \
      || return 1
    marker_sha="$(jq -er '
      select(
        .version == 1
        and (.cleanup_pid | type == "number" and . > 0)
        and (.journal_sha256
          | type == "string" and test("^[0-9a-f]{64}$"))
      )
      | .journal_sha256
    ' "$marker")" || return 1
    DETOUR_FIXTURE_JOURNAL_SHA256="$marker_sha"
    loot_fixture_bot_cleanup_complete
    return
  fi

  [ -f "$journal" ] && [ ! -L "$journal" ] \
    && [ "$(stat -c '%a' -- "$journal" 2>/dev/null)" = 600 ] || return 1
  jq -e '
    if .db_applied then
        .phases.poststate_checkpointed
        and .phases.db_restored
        and .phases.filesystem_restored
        and .phases.normal_runtime_restored
      else
        .phases.filesystem_restored
        and .phases.normal_runtime_restored
      end
  ' "$journal" >/dev/null || {
    echo "WARNING: refusing to consume detour journal before integral recovery completes" >&2
    return 1
  }
  journal_sha="$(detour_chase_sha256_of_file "$journal")" || return 1
  parent="$(dirname -- "$marker")"
  if [ -e "$marker" ] || [ -L "$marker" ]; then
    [ -f "$marker" ] && [ ! -L "$marker" ] \
      && [ "$(stat -c '%a' -- "$marker" 2>/dev/null)" = 600 ] \
      || return 1
    marker_sha="$(jq -er '
      select(
        .version == 1
        and (.cleanup_pid | type == "number" and . > 0)
        and (.journal_sha256
          | type == "string" and test("^[0-9a-f]{64}$"))
      )
      | .journal_sha256
    ' "$marker")" || return 1
    [ "$marker_sha" = "$journal_sha" ] || return 1
  else
    stage="$(mktemp "${parent}/.detour-cleanup-complete.XXXXXX")" || return 1
    chmod 600 "$stage" || {
      rm -f -- "$stage"
      return 1
    }
    jq -n \
      --arg journal_sha256 "$journal_sha" \
      --argjson cleanup_pid "$$" \
      '{version:1,journal_sha256:$journal_sha256,cleanup_pid:$cleanup_pid}' \
      >"$stage" || {
        rm -f -- "$stage"
        return 1
      }
    sync -f "$stage" || {
      rm -f -- "$stage"
      return 1
    }
    if ! mv -- "$stage" "$marker"; then
      rm -f -- "$stage"
      return 1
    fi
    sync -f "$marker" && sync -f "$parent" || return 1
  fi
  rm -- "$journal" || return 1
  sync -f "$parent" || return 1
  DETOUR_FIXTURE_JOURNAL_SHA256="$journal_sha"
  loot_fixture_bot_cleanup_complete
}

detour_chase_restore_character_auxiliary_state() {
  local encoded snapshot table scope_column prior_sha post_sha restore_sql
  local post_predicate_sql current_sql current_sha legacy_current_sha
  local normalized_prior_sha cas_result

  while IFS= read -r encoded; do
    snapshot="$(printf '%s' "$encoded" | base64 --decode)" || return 1
    table="$(jq -er '.table' <<<"$snapshot")" || return 1
    scope_column="$(jq -er '.scope_column' <<<"$snapshot")" || return 1
    prior_sha="$(jq -er '.prior_sha256' <<<"$snapshot")" || return 1
    post_sha="$(jq -er '.post_sha256' <<<"$snapshot")" || return 1
    restore_sql="$(jq -er '.restore_sql' <<<"$snapshot")" || return 1
    restore_sql="$(
      detour_chase_normalize_legacy_insert_restore_sql "$restore_sql"
    )" || return 1
    post_predicate_sql="$(jq -er '.post_predicate_sql' <<<"$snapshot")" \
      || return 1
    detour_chase_validate_sql_identifier "$table" \
      && detour_chase_validate_sql_identifier "$scope_column" || return 1
    current_sql="$(
      detour_chase_snapshot_table_insert_sql \
        loot_fixture_character_mysql "$table" \
        "\`${scope_column}\`=${DETOUR_FIXTURE_CHARACTER_GUID}"
    )" || return 1
    current_sha="$(detour_chase_sha256_of_text "$current_sql")" || return 1
    normalized_prior_sha="$(
      detour_chase_sha256_of_text "$restore_sql"
    )" || return 1
    legacy_current_sha="$(
      detour_chase_sha256_of_text "$(
        detour_chase_legacy_insert_restore_sql "$current_sql"
      )"
    )" || return 1
    if [ "$current_sha" != "$normalized_prior_sha" ] \
        && [ "$legacy_current_sha" != "$prior_sha" ]; then
      [ "$DETOUR_FIXTURE_POSTSTATE_CHECKPOINTED" = 1 ] \
        && { [ "$current_sha" = "$post_sha" ] \
          || [ "$legacy_current_sha" = "$post_sha" ]; } || {
          echo "WARNING: detour character auxiliary table ${table} differs from both prior and checkpointed fixture poststate; refusing cleanup writes" >&2
          return 1
        }
      cas_result="$(loot_fixture_character_mysql -e "
        SET autocommit=0;
        LOCK TABLES \`${table}\` WRITE;
        SET @detour_cas=IF((${post_predicate_sql}),1,0);
        DELETE FROM \`${table}\`
          WHERE \`${scope_column}\`=${DETOUR_FIXTURE_CHARACTER_GUID}
            AND @detour_cas=1;
        ${restore_sql}
        COMMIT;
        SELECT @detour_cas;
        UNLOCK TABLES;
        SET autocommit=1;")" || return 1
      [ "$cas_result" = 1 ] || {
        echo "WARNING: detour character auxiliary table ${table} CAS failed under WRITE lock; journal retained" >&2
        return 1
      }
      current_sql="$(
        detour_chase_snapshot_table_insert_sql \
          loot_fixture_character_mysql "$table" \
          "\`${scope_column}\`=${DETOUR_FIXTURE_CHARACTER_GUID}"
      )" || return 1
      current_sha="$(detour_chase_sha256_of_text "$current_sql")" || return 1
      legacy_current_sha="$(
        detour_chase_sha256_of_text "$(
          detour_chase_legacy_insert_restore_sql "$current_sql"
        )"
      )" || return 1
      [ "$current_sha" = "$normalized_prior_sha" ] \
        || [ "$legacy_current_sha" = "$prior_sha" ] || {
        echo "WARNING: detour character auxiliary table ${table} did not restore exactly" >&2
        return 1
      }
    fi
  done < <(jq -r '.[] | @base64' \
    <<<"$DETOUR_FIXTURE_CHARACTER_AUX_SNAPSHOTS_JSON")
}

detour_chase_restore_respawn_state() {
  local prior_sha post_sha restore_sql post_predicate_sql
  local current_sql current_sha legacy_current_sha normalized_prior_sha
  local cas_result

  prior_sha="$(jq -er '.prior_sha256' \
    <<<"$DETOUR_FIXTURE_RESPAWN_SNAPSHOT_JSON")" || return 1
  restore_sql="$(jq -er '.restore_sql' \
    <<<"$DETOUR_FIXTURE_RESPAWN_SNAPSHOT_JSON")" || return 1
  restore_sql="$(
    detour_chase_normalize_legacy_insert_restore_sql "$restore_sql"
  )" || return 1
  post_sha="$(jq -er '.post_sha256' \
    <<<"$DETOUR_FIXTURE_RESPAWN_SNAPSHOT_JSON")" || return 1
  post_predicate_sql="$(jq -er '.post_predicate_sql' \
    <<<"$DETOUR_FIXTURE_RESPAWN_SNAPSHOT_JSON")" || return 1
  current_sql="$(
    detour_chase_snapshot_table_insert_sql \
      loot_fixture_character_mysql respawn \
      "spawnId=${DETOUR_FIXTURE_CREATURE_GUID}"
  )" || return 1
  current_sha="$(detour_chase_sha256_of_text "$current_sql")" || return 1
  normalized_prior_sha="$(
    detour_chase_sha256_of_text "$restore_sql"
  )" || return 1
  legacy_current_sha="$(
    detour_chase_sha256_of_text "$(
      detour_chase_legacy_insert_restore_sql "$current_sql"
    )"
  )" || return 1
  if [ "$current_sha" != "$normalized_prior_sha" ] \
      && [ "$legacy_current_sha" != "$prior_sha" ]; then
    [ "$DETOUR_FIXTURE_POSTSTATE_CHECKPOINTED" = 1 ] \
      && { [ "$current_sha" = "$post_sha" ] \
        || [ "$legacy_current_sha" = "$post_sha" ]; } || {
        echo "WARNING: detour respawn domain differs from both prior and checkpointed fixture poststate; refusing cleanup writes" >&2
        return 1
      }
    cas_result="$(loot_fixture_character_mysql -e "
      SET autocommit=0;
      LOCK TABLES respawn WRITE;
      SET @detour_cas=IF((${post_predicate_sql}),1,0);
      DELETE FROM respawn
        WHERE spawnId=${DETOUR_FIXTURE_CREATURE_GUID}
          AND @detour_cas=1;
      ${restore_sql}
      COMMIT;
      SELECT @detour_cas;
      UNLOCK TABLES;
      SET autocommit=1;")" || return 1
    [ "$cas_result" = 1 ] || {
      echo "WARNING: detour respawn CAS failed under WRITE lock; journal retained" >&2
      return 1
    }
    current_sql="$(
      detour_chase_snapshot_table_insert_sql \
        loot_fixture_character_mysql respawn \
        "spawnId=${DETOUR_FIXTURE_CREATURE_GUID}"
    )" || return 1
    current_sha="$(detour_chase_sha256_of_text "$current_sql")" || return 1
    legacy_current_sha="$(
      detour_chase_sha256_of_text "$(
        detour_chase_legacy_insert_restore_sql "$current_sql"
      )"
    )" || return 1
    [ "$current_sha" = "$normalized_prior_sha" ] \
      || [ "$legacy_current_sha" = "$prior_sha" ] || {
      echo "WARNING: detour creature respawn rows did not restore exactly" >&2
      return 1
    }
  fi
}
