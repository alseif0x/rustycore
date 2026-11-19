# Append a snapshot without placing either accumulated rows or recovery SQL in
# jq's argv. Account rows contain verifier/session/TOTP material, so all
# transformations use mode-0600 files and run with xtrace disabled.
detour_chase_append_snapshot_json() {
  local snapshots="$1"
  local restore_sql="$2"
  local predicate_sql="$3"
  local metadata_json="$4"
  local snapshots_file restore_file predicate_file output restore_xtrace=0 status

  if [[ "$-" == *x* ]]; then
    restore_xtrace=1
    set +x
  fi
  snapshots_file="$(detour_chase_secure_temp_file)" || return 1
  restore_file="$(detour_chase_secure_temp_file)" || {
    rm -f -- "$snapshots_file"
    return 1
  }
  predicate_file="$(detour_chase_secure_temp_file)" || {
    rm -f -- "$snapshots_file" "$restore_file"
    return 1
  }
  if printf '%s\n' "$snapshots" >"$snapshots_file" \
      && printf '%s' "$restore_sql" >"$restore_file" \
      && printf '%s' "$predicate_sql" >"$predicate_file"; then
    output="$(
      jq -cn \
        --slurpfile snapshots "$snapshots_file" \
        --rawfile restore_sql "$restore_file" \
        --rawfile predicate_sql "$predicate_file" \
        --argjson metadata "$metadata_json" \
        '$snapshots[0] + [
          $metadata + {
            restore_sql:$restore_sql,
            predicate_sql:$predicate_sql,
            post_sha256:null,
            post_predicate_sql:null
          }
        ]'
    )"
    status=$?
  else
    output=""
    status=1
  fi
  rm -f -- "$snapshots_file" "$restore_file" "$predicate_file" || status=1
  if [ "$status" -eq 0 ]; then
    printf '%s\n' "$output"
  fi
  if [ "$restore_xtrace" -eq 1 ]; then
    set -x
  fi
  return "$status"
}

# Every directly character-owned table that Player::SaveToDB and the login /
# logout lifecycle can rewrite. Tables with dependent pet/item identities are
# included but the disposable fixture preflight below requires those domains
# empty, so cleanup never guesses an indirect ownership graph.
detour_chase_character_auxiliary_scopes() {
  cat <<'SCOPES'
character_account_data	guid
character_achievement	guid
character_achievement_progress	guid
character_action	guid
character_arena_stats	guid
character_aura	guid
character_aura_effect	guid
character_aura_stored_location	Guid
character_banned	guid
character_battleground_data	guid
character_battleground_random	guid
character_cuf_profiles	guid
character_currency	CharacterGuid
character_customizations	guid
character_declinedname	guid
character_equipmentsets	guid
character_favorite_auctions	guid
character_fishingsteps	guid
character_gifts	guid
character_glyphs	guid
character_homebind	guid
character_instance_lock	guid
character_inventory	guid
character_pet	owner
character_pet_declinedname	owner
character_pvp_talent	guid
character_queststatus	guid
character_queststatus_daily	guid
character_queststatus_monthly	guid
character_queststatus_objectives	guid
character_queststatus_objectives_criteria	guid
character_queststatus_objectives_criteria_progress	guid
character_queststatus_rewarded	guid
character_queststatus_seasonal	guid
character_queststatus_weekly	guid
character_reputation	guid
character_skills	guid
character_social	guid
character_spell	guid
character_spell_charges	guid
character_spell_cooldown	guid
character_spell_favorite	guid
character_stats	guid
character_talent	guid
character_trade_skill_spells	guid
character_trait_config	guid
character_trait_entry	guid
character_transmog_outfits	guid
character_void_storage	playerGuid
corpse	guid
corpse_customizations	ownerGuid
corpse_phases	OwnerGuid
group_member	memberGuid
item_instance	owner_guid
lfg_data	guid
mail	receiver
mail_items	receiver
SCOPES
}

detour_chase_expected_character_auxiliary_scopes_json() {
  detour_chase_character_auxiliary_scopes \
    | jq -Rsc '
        split("\n")
        | map(select(length > 0) | split("\t"))
        | map({table:.[0], scope_column:.[1]})
      '
}

detour_chase_table_columns() {
  local mysql_function="$1"
  local table="$2"

  detour_chase_validate_sql_identifier "$table" || return 1
  "$mysql_function" -e "
    SELECT COLUMN_NAME,LOWER(DATA_TYPE)
      FROM information_schema.COLUMNS
     WHERE TABLE_SCHEMA=DATABASE()
       AND TABLE_NAME='${table}'
       AND EXTRA NOT LIKE '%GENERATED%'
     ORDER BY ORDINAL_POSITION"
}

detour_chase_sql_serialized_value_expression() {
  local column="$1"
  local data_type="$2"
  detour_chase_validate_sql_identifier "$column" || return 1
  [[ "$data_type" =~ ^[a-z][a-z0-9_]*$ ]] || return 1
  # REPLACE removes TO_BASE64's RFC-2045 line wrapping. The generated recovery
  # SQL is therefore ASCII-only and one row per line even for arbitrary text or
  # binary columns. MariaDB's default FLOAT/DOUBLE text rendering is not a
  # lossless round trip, so serialize those through a high-precision DECIMAL;
  # assignment back to the original column reproduces the same IEEE value and
  # the generated CAS predicate compares equal before mutation.
  case "$data_type" in
    float|double|real)
      printf '%s' \
        "IF(\`${column}\` IS NULL,'NULL',CONCAT('CAST(''',CAST(\`${column}\` AS DECIMAL(65,30)),''' AS DECIMAL(65,30))'))"
      ;;
    *)
      printf '%s' \
        "IF(\`${column}\` IS NULL,'NULL',CONCAT('FROM_BASE64(''',REPLACE(TO_BASE64(\`${column}\`),CHAR(10),''),''')'))"
      ;;
  esac
}

detour_chase_snapshot_table_insert_sql() {
  local mysql_function="$1"
  local table="$2"
  local where_sql="$3"
  local columns column data_type column_list="" values_expression=""
  local column_delimiter="" value_delimiter=""
  local serialized query

  detour_chase_validate_sql_identifier "$table" || return 1
  [ -n "$where_sql" ] || return 1
  columns="$(detour_chase_table_columns "$mysql_function" "$table")" \
    || return 1
  [ -n "$columns" ] || {
    echo "error: detour snapshot table ${table} is missing or has no writable columns" >&2
    return 1
  }
  while IFS=$'\t' read -r column data_type; do
    detour_chase_validate_sql_identifier "$column" || return 1
    serialized="$(
      detour_chase_sql_serialized_value_expression "$column" "$data_type"
    )" \
      || return 1
    column_list+="${column_delimiter}\`${column}\`"
    values_expression+="${value_delimiter}${serialized}"
    # This is a CONCAT() argument list. The comma between SQL VALUES must be
    # emitted as data, not merely used to separate the CONCAT arguments.
    column_delimiter=","
    value_delimiter=",',',"
  done <<<"$columns"
  # Recovery sets @detour_cas only after taking a WRITE lock and evaluating the
  # journaled full-domain predicate. Gating every INSERT keeps a failed CAS from
  # recreating prior rows after a guarded DELETE affected zero rows.
  query="SELECT CONCAT('INSERT INTO \`${table}\` (${column_list}) SELECT ',${values_expression},' WHERE @detour_cas=1;') FROM \`${table}\` WHERE ${where_sql}"
  "$mysql_function" -e "$query" | LC_ALL=C sort
}

detour_chase_snapshot_table_cas_predicate_sql() {
  local mysql_function="$1"
  local table="$2"
  local where_sql="$3"
  local columns column data_type serialized row_expression="" delimiter=""
  local query rows row previous="" duplicate_count=0 total_count=0 predicate

  detour_chase_validate_sql_identifier "$table" || return 1
  [ -n "$where_sql" ] || return 1
  columns="$(detour_chase_table_columns "$mysql_function" "$table")" || return 1
  [ -n "$columns" ] || return 1
  while IFS=$'\t' read -r column data_type; do
    detour_chase_validate_sql_identifier "$column" || return 1
    serialized="$(
      detour_chase_sql_serialized_value_expression "$column" "$data_type"
    )" \
      || return 1
    row_expression+="${delimiter}'\`${column}\` <=> ',${serialized}"
    delimiter=", ' AND ',"
  done <<<"$columns"
  query="SELECT CONCAT('(',${row_expression},')') FROM \`${table}\` WHERE ${where_sql}"
  rows="$("$mysql_function" -e "$query" | LC_ALL=C sort)" || return 1
  if [ -n "$rows" ]; then
    total_count="$(wc -l <<<"$rows" | tr -d '[:space:]')" || return 1
  else
    total_count=0
  fi
  predicate="(SELECT (COUNT(*)=${total_count}"
  while IFS= read -r row; do
    [ -n "$row" ] || continue
    if [ -z "$previous" ]; then
      previous="$row"
      duplicate_count=1
    elif [ "$row" = "$previous" ]; then
      duplicate_count=$((duplicate_count + 1))
    else
      predicate+=" AND COALESCE(SUM(${previous}),0)=${duplicate_count}"
      previous="$row"
      duplicate_count=1
    fi
  done <<<"$rows"
  if [ -n "$previous" ]; then
    predicate+=" AND COALESCE(SUM(${previous}),0)=${duplicate_count}"
  fi
  predicate+=") FROM \`${table}\` WHERE ${where_sql})"
  printf '%s\n' "$predicate"
}

detour_chase_normalize_legacy_singleton_cas_predicate_sql() {
  local table="$1"
  local where_sql="$2"
  local predicate="$3"
  local prefix="((SELECT COUNT(*) FROM \`${table}\` WHERE ${where_sql})=1) AND "

  # Journals written before the single-scan CAS format used one subquery for
  # the row count and another for the complete singleton value. MariaDB
  # requires a separately aliased LOCK TABLES entry for every such reference.
  # The detailed COUNT=1 already proves both existence and cardinality, so
  # dropping only the exact redundant prefix preserves the full CAS contract.
  if [[ "$predicate" == "$prefix"* ]]; then
    printf '%s\n' "${predicate#"$prefix"}"
  else
    printf '%s\n' "$predicate"
  fi
}

detour_chase_snapshot_single_character_update_sql() {
  detour_chase_snapshot_single_row_update_sql \
    loot_fixture_character_mysql characters \
    "guid=${DETOUR_FIXTURE_CHARACTER_GUID}" \
    "guid=${DETOUR_FIXTURE_CHARACTER_GUID} AND account=${DETOUR_FIXTURE_CHARACTER_ACCOUNT} AND online=0 AND deleteDate IS NULL AND map=${DETOUR_FIXTURE_MAP_ID} AND instance_id=0 AND position_x BETWEEN -10170 AND -10070 AND position_y BETWEEN 2620 AND 2740 AND position_z BETWEEN 190 AND 250 AND health > 0"
}

detour_chase_snapshot_single_row_update_sql() {
  local mysql_function="$1"
  local table="$2"
  local snapshot_where="$3"
  local restore_where="$4"
  local columns column data_type assignments="" delimiter="" serialized query

  detour_chase_validate_sql_identifier "$table" || return 1
  [ -n "$snapshot_where" ] && [ -n "$restore_where" ] || return 1
  columns="$(detour_chase_table_columns "$mysql_function" "$table")" || return 1
  [ -n "$columns" ] || return 1
  while IFS=$'\t' read -r column data_type; do
    detour_chase_validate_sql_identifier "$column" || return 1
    serialized="$(
      detour_chase_sql_serialized_value_expression "$column" "$data_type"
    )" \
      || return 1
    assignments+="${delimiter}'\`${column}\`=',${serialized}"
    delimiter=",',',"
  done <<<"$columns"
  query="SELECT CONCAT('UPDATE \`${table}\` SET ',${assignments},' WHERE ${restore_where}') FROM \`${table}\` WHERE ${snapshot_where}"
  "$mysql_function" -e "$query"
}

detour_chase_snapshot_single_character_predicate_sql() {
  local columns column data_type predicates="" delimiter="" serialized query

  columns="$(detour_chase_table_columns loot_fixture_character_mysql characters)" \
    || return 1
  [ -n "$columns" ] || return 1
  while IFS=$'\t' read -r column data_type; do
    detour_chase_validate_sql_identifier "$column" || return 1
    serialized="$(
      detour_chase_sql_serialized_value_expression "$column" "$data_type"
    )" \
      || return 1
    predicates+="${delimiter}'\`${column}\` <=> ',${serialized}"
    delimiter=", ' AND ',"
  done <<<"$columns"
  query="SELECT CONCAT(${predicates}) FROM characters WHERE guid=${DETOUR_FIXTURE_CHARACTER_GUID}"
  loot_fixture_character_mysql -e "$query"
}

detour_chase_snapshot_character_auxiliary_state() {
  local snapshots='[]' table scope_column restore_sql predicate_sql prior_sha
  local metadata_json restore_xtrace=0

  if [[ "$-" == *x* ]]; then
    restore_xtrace=1
    set +x
  fi
  while IFS=$'\t' read -r table scope_column; do
    detour_chase_validate_sql_identifier "$table" \
      && detour_chase_validate_sql_identifier "$scope_column" || return 1
    restore_sql="$(
      detour_chase_snapshot_table_insert_sql \
        loot_fixture_character_mysql "$table" \
        "\`${scope_column}\`=${DETOUR_FIXTURE_CHARACTER_GUID}"
    )" || return 1
    predicate_sql="$(
      detour_chase_snapshot_table_cas_predicate_sql \
        loot_fixture_character_mysql "$table" \
        "\`${scope_column}\`=${DETOUR_FIXTURE_CHARACTER_GUID}"
    )" || return 1
    prior_sha="$(detour_chase_sha256_of_text "$restore_sql")" || return 1
    metadata_json="$(
      jq -cn \
        --arg table "$table" \
        --arg scope_column "$scope_column" \
        --arg prior_sha256 "$prior_sha" \
        '{
          table:$table,
          scope_column:$scope_column,
          prior_sha256:$prior_sha256
        }'
    )" || return 1
    snapshots="$(
      detour_chase_append_snapshot_json \
        "$snapshots" "$restore_sql" "$predicate_sql" "$metadata_json"
    )" || return 1
  done < <(detour_chase_character_auxiliary_scopes)
  printf '%s\n' "$snapshots"
  if [ "$restore_xtrace" -eq 1 ]; then
    set -x
  fi
}

detour_chase_snapshot_respawn_state() {
  local restore_sql predicate_sql prior_sha
  restore_sql="$(
    detour_chase_snapshot_table_insert_sql \
      loot_fixture_character_mysql respawn \
      "spawnId=${DETOUR_FIXTURE_CREATURE_GUID}"
  )" || return 1
  predicate_sql="$(
    detour_chase_snapshot_table_cas_predicate_sql \
      loot_fixture_character_mysql respawn \
      "spawnId=${DETOUR_FIXTURE_CREATURE_GUID}"
  )" || return 1
  prior_sha="$(detour_chase_sha256_of_text "$restore_sql")" || return 1
  local restore_file predicate_file output status
  restore_file="$(detour_chase_secure_temp_file)" || return 1
  predicate_file="$(detour_chase_secure_temp_file)" || {
    rm -f -- "$restore_file"
    return 1
  }
  printf '%s' "$restore_sql" >"$restore_file" \
    && printf '%s' "$predicate_sql" >"$predicate_file" || {
      rm -f -- "$restore_file" "$predicate_file"
      return 1
    }
  output="$(jq -cn \
    --arg prior_sha256 "$prior_sha" \
    --rawfile restore_sql "$restore_file" \
    --rawfile predicate_sql "$predicate_file" \
    '{
      table:"respawn",
      scope_column:"spawnId",
      scope_value:9102401,
      prior_sha256:$prior_sha256,
      restore_sql:$restore_sql,
      predicate_sql:$predicate_sql,
      post_sha256:null,
      post_predicate_sql:null
    }')"
  status=$?
  rm -f -- "$restore_file" "$predicate_file" || status=1
  [ "$status" -eq 0 ] || return "$status"
  printf '%s\n' "$output"
}

detour_chase_world_auxiliary_state() {
  loot_fixture_world_mysql -e "
    SELECT CONCAT_WS('#',
      (SELECT COUNT(*) FROM creature_addon
        WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID}),
      (SELECT COUNT(*) FROM creature_formations
        WHERE leaderGUID=${DETOUR_FIXTURE_CREATURE_GUID}
           OR memberGUID=${DETOUR_FIXTURE_CREATURE_GUID}),
      (SELECT COUNT(*) FROM creature_movement_override
        WHERE SpawnId=${DETOUR_FIXTURE_CREATURE_GUID}),
      (SELECT COUNT(*) FROM game_event_creature
        WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID}),
      (SELECT COUNT(*) FROM game_event_model_equip
        WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID}),
      (SELECT COUNT(*) FROM game_event_npcflag
        WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID}),
      (SELECT COUNT(*) FROM game_event_npc_vendor
        WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID}),
      (SELECT COUNT(*) FROM linked_respawn
        WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID}
           OR linkedGuid=${DETOUR_FIXTURE_CREATURE_GUID}),
      (SELECT COUNT(*) FROM pool_members
        WHERE type=0 AND spawnId=${DETOUR_FIXTURE_CREATURE_GUID}),
      (SELECT COUNT(*) FROM spawn_group
        WHERE spawnType=0 AND spawnId=${DETOUR_FIXTURE_CREATURE_GUID}),
      (SELECT COUNT(*) FROM smart_scripts
        WHERE source_type=0
          AND entryorguid=-${DETOUR_FIXTURE_CREATURE_GUID}))"
}

detour_chase_clear_world_auxiliary_state() {
  loot_fixture_world_mysql -e "
    START TRANSACTION;
    DELETE FROM creature_addon
      WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID};
    DELETE FROM creature_formations
      WHERE leaderGUID=${DETOUR_FIXTURE_CREATURE_GUID}
         OR memberGUID=${DETOUR_FIXTURE_CREATURE_GUID};
    DELETE FROM creature_movement_override
      WHERE SpawnId=${DETOUR_FIXTURE_CREATURE_GUID};
    DELETE FROM game_event_creature
      WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID};
    DELETE FROM game_event_model_equip
      WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID};
    DELETE FROM game_event_npcflag
      WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID};
    DELETE FROM game_event_npc_vendor
      WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID};
    DELETE FROM linked_respawn
      WHERE guid=${DETOUR_FIXTURE_CREATURE_GUID}
         OR linkedGuid=${DETOUR_FIXTURE_CREATURE_GUID};
    DELETE FROM pool_members
      WHERE type=0 AND spawnId=${DETOUR_FIXTURE_CREATURE_GUID};
    DELETE FROM spawn_group
      WHERE spawnType=0 AND spawnId=${DETOUR_FIXTURE_CREATURE_GUID};
    DELETE FROM smart_scripts
      WHERE source_type=0
        AND entryorguid=-${DETOUR_FIXTURE_CREATURE_GUID};
    COMMIT;"
}

detour_chase_load_auth_database_credentials() {
  local login_info extra
  login_info="$(read_database_info LoginDatabaseInfo)" || {
    echo "error: LoginDatabaseInfo not found in ${LOOT_FIXTURE_DB_CONF}" >&2
    return 1
  }
  IFS=';' read -r \
    DETOUR_FIXTURE_AUTH_HOST \
    DETOUR_FIXTURE_AUTH_PORT \
    DETOUR_FIXTURE_AUTH_USER \
    DETOUR_FIXTURE_AUTH_PASSWORD \
    DETOUR_FIXTURE_AUTH_DATABASE \
    extra <<<"$login_info"
  [ -z "${extra:-}" ] \
    && [ -n "$DETOUR_FIXTURE_AUTH_HOST" ] \
    && [[ "$DETOUR_FIXTURE_AUTH_PORT" =~ ^[1-9][0-9]*$ ]] \
    && [ -n "$DETOUR_FIXTURE_AUTH_USER" ] \
    && [ -n "$DETOUR_FIXTURE_AUTH_DATABASE" ] || {
      echo "error: detour LoginDatabaseInfo is incomplete" >&2
      return 1
    }
}

detour_chase_auth_mysql() {
  local restore_xtrace=0 status
  if [[ "$-" == *x* ]]; then
    restore_xtrace=1
    set +x
  fi
  if MYSQL_PWD="$DETOUR_FIXTURE_AUTH_PASSWORD" mysql \
      --protocol=TCP \
      -h "$DETOUR_FIXTURE_AUTH_HOST" \
      -P "$DETOUR_FIXTURE_AUTH_PORT" \
      -u "$DETOUR_FIXTURE_AUTH_USER" \
      --batch --raw --skip-column-names \
      "$DETOUR_FIXTURE_AUTH_DATABASE" "$@"; then
    status=0
  else
    status=$?
  fi
  if [ "$restore_xtrace" -eq 1 ]; then
    set -x
  fi
  return "$status"
}

detour_chase_account_restore_guard() {
  local table="$1"
  local scope_column="$2"
  local scope_value="$3"
  case "$table" in
    account)
      printf 'id=%s AND battlenet_account=%s' \
        "$scope_value" "$DETOUR_FIXTURE_BNET_ACCOUNT_ID"
      ;;
    battlenet_accounts)
      printf 'id=%s' "$scope_value"
      ;;
    *)
      printf '`%s`=%s' "$scope_column" "$scope_value"
      ;;
  esac
}

detour_chase_account_snapshot_scopes() {
  printf 'auth\tupdate\taccount\tid\t%s\n' \
    "$DETOUR_FIXTURE_CHARACTER_ACCOUNT"
  printf 'auth\tupdate\tbattlenet_accounts\tid\t%s\n' \
    "$DETOUR_FIXTURE_BNET_ACCOUNT_ID"
  printf 'auth\tdelete_insert\tbattlenet_account_toys\taccountId\t%s\n' \
    "$DETOUR_FIXTURE_BNET_ACCOUNT_ID"
  printf 'auth\tdelete_insert\tbattlenet_account_heirlooms\taccountId\t%s\n' \
    "$DETOUR_FIXTURE_BNET_ACCOUNT_ID"
  printf 'auth\tdelete_insert\tbattlenet_account_mounts\tbattlenetAccountId\t%s\n' \
    "$DETOUR_FIXTURE_BNET_ACCOUNT_ID"
  printf 'auth\tdelete_insert\tbattlenet_account_transmog_illusions\tbattlenetAccountId\t%s\n' \
    "$DETOUR_FIXTURE_BNET_ACCOUNT_ID"
  printf 'auth\tdelete_insert\tbattlenet_item_appearances\tbattlenetAccountId\t%s\n' \
    "$DETOUR_FIXTURE_BNET_ACCOUNT_ID"
  printf 'auth\tdelete_insert\tbattlenet_item_favorite_appearances\tbattlenetAccountId\t%s\n' \
    "$DETOUR_FIXTURE_BNET_ACCOUNT_ID"
  # BattlePetMgr::SaveToDB unconditionally deletes and reinserts these rows.
  # The fixture preflight requires the pet collection itself empty, avoiding
  # an unsafe dependent battle_pet_declinedname ownership traversal.
  printf 'auth\tdelete_insert\tbattle_pet_slots\tbattlenetAccountId\t%s\n' \
    "$DETOUR_FIXTURE_BNET_ACCOUNT_ID"
  printf 'auth\tdelete_insert\taccount_last_played_character\taccountId\t%s\n' \
    "$DETOUR_FIXTURE_CHARACTER_ACCOUNT"
  printf 'auth\tdelete_insert\trealmcharacters\tacctid\t%s\n' \
    "$DETOUR_FIXTURE_CHARACTER_ACCOUNT"
  printf 'characters\tdelete_insert\taccount_data\taccountId\t%s\n' \
    "$DETOUR_FIXTURE_CHARACTER_ACCOUNT"
  printf 'characters\tdelete_insert\taccount_instance_times\taccountId\t%s\n' \
    "$DETOUR_FIXTURE_CHARACTER_ACCOUNT"
  printf 'characters\tdelete_insert\taccount_tutorial\taccountId\t%s\n' \
    "$DETOUR_FIXTURE_CHARACTER_ACCOUNT"
}

detour_chase_snapshot_account_state() {
  local snapshots='[]' database strategy table scope_column scope_value
  local mysql_function restore_where restore_sql predicate_sql prior_sha
  local metadata_json restore_xtrace=0

  if [[ "$-" == *x* ]]; then
    restore_xtrace=1
    set +x
  fi
  while IFS=$'\t' read -r \
      database strategy table scope_column scope_value; do
    detour_chase_validate_sql_identifier "$table" \
      && detour_chase_validate_sql_identifier "$scope_column" \
      && [[ "$scope_value" =~ ^[0-9]+$ ]] || return 1
    case "$database" in
      auth) mysql_function=detour_chase_auth_mysql ;;
      characters) mysql_function=loot_fixture_character_mysql ;;
      *) return 1 ;;
    esac
    case "$strategy" in
      update)
        restore_where="$(
          detour_chase_account_restore_guard \
            "$table" "$scope_column" "$scope_value"
        )" || return 1
        restore_sql="$(
          detour_chase_snapshot_single_row_update_sql \
            "$mysql_function" "$table" \
            "\`${scope_column}\`=${scope_value}" "$restore_where"
        )" || return 1
        [ -n "$restore_sql" ] || return 1
        ;;
      delete_insert)
        restore_sql="$(
          detour_chase_snapshot_table_insert_sql \
            "$mysql_function" "$table" \
            "\`${scope_column}\`=${scope_value}"
        )" || return 1
        ;;
      *) return 1 ;;
    esac
    predicate_sql="$(
      detour_chase_snapshot_table_cas_predicate_sql \
        "$mysql_function" "$table" \
        "\`${scope_column}\`=${scope_value}"
    )" || return 1
    prior_sha="$(detour_chase_sha256_of_text "$restore_sql")" || return 1
    metadata_json="$(
      jq -cn \
        --arg database "$database" \
        --arg strategy "$strategy" \
        --arg table "$table" \
        --arg scope_column "$scope_column" \
        --argjson scope_value "$scope_value" \
        --arg prior_sha256 "$prior_sha" \
        '{
          database:$database,
          strategy:$strategy,
          table:$table,
          scope_column:$scope_column,
          scope_value:$scope_value,
          prior_sha256:$prior_sha256
        }'
    )" || return 1
    snapshots="$(
      detour_chase_append_snapshot_json \
        "$snapshots" "$restore_sql" "$predicate_sql" "$metadata_json"
    )" || return 1
  done < <(detour_chase_account_snapshot_scopes)
  printf '%s\n' "$snapshots"
  if [ "$restore_xtrace" -eq 1 ]; then
    set -x
  fi
}

detour_chase_execute_recovery_sql() {
  local database="$1"
  local sql="$2"
  local restore_xtrace=0 status
  if [[ "$-" == *x* ]]; then
    restore_xtrace=1
    set +x
  fi
  case "$database" in
    auth)
      if printf '%s\n' "$sql" | detour_chase_auth_mysql; then
        status=0
      else
        status=$?
      fi
      ;;
    characters)
      if printf '%s\n' "$sql" | loot_fixture_character_mysql; then
        status=0
      else
        status=$?
      fi
      ;;
    *) status=1 ;;
  esac
  if [ "$restore_xtrace" -eq 1 ]; then
    set -x
  fi
  return "$status"
}

detour_chase_normalize_legacy_insert_restore_sql() {
  local sql="$1"

  # Journals written before VALUES delimiters were emitted as CONCAT data can
  # contain adjacent binary-safe expressions. Restrict compatibility to that
  # exact generated shape; all newly written journals include literal commas.
  if [[ "$sql" == INSERT\ INTO*FROM_BASE64* ]]; then
    sql="${sql//)FROM_BASE64/),FROM_BASE64}"
    sql="${sql//)CAST/),CAST}"
    sql="${sql//)NULL/),NULL}"
    sql="${sql//NULLFROM_BASE64/NULL,FROM_BASE64}"
    sql="${sql//NULLCAST/NULL,CAST}"
    sql="${sql//NULLNULL/NULL,NULL}"
  fi
  printf '%s\n' "$sql"
}

detour_chase_legacy_insert_restore_sql() {
  local sql="$1"
  sql="${sql//),FROM_BASE64/)FROM_BASE64}"
  sql="${sql//),CAST/)CAST}"
  sql="${sql//),NULL/)NULL}"
  sql="${sql//NULL,FROM_BASE64/NULLFROM_BASE64}"
  sql="${sql//NULL,CAST/NULLCAST}"
  sql="${sql//NULL,NULL/NULLNULL}"
  printf '%s\n' "$sql"
}

detour_chase_restore_account_state() {
  local encoded snapshot database strategy table scope_column scope_value
  local prior_sha post_sha restore_sql post_predicate_sql mysql_function
  local restore_where current_sql current_sha legacy_current_sha
  local normalized_prior_sha cas_result recovery_sql
  local restore_xtrace=0

  if [[ "$-" == *x* ]]; then
    restore_xtrace=1
    set +x
  fi

  while IFS= read -r encoded; do
    snapshot="$(printf '%s' "$encoded" | base64 --decode)" || return 1
    database="$(jq -er '.database' <<<"$snapshot")" || return 1
    strategy="$(jq -er '.strategy' <<<"$snapshot")" || return 1
    table="$(jq -er '.table' <<<"$snapshot")" || return 1
    scope_column="$(jq -er '.scope_column' <<<"$snapshot")" || return 1
    scope_value="$(jq -er '.scope_value' <<<"$snapshot")" || return 1
    prior_sha="$(jq -er '.prior_sha256' <<<"$snapshot")" || return 1
    restore_sql="$(jq -er '.restore_sql' <<<"$snapshot")" || return 1
    restore_sql="$(
      detour_chase_normalize_legacy_insert_restore_sql "$restore_sql"
    )" || return 1
    post_sha="$(jq -er '.post_sha256' <<<"$snapshot")" || return 1
    post_predicate_sql="$(jq -er '.post_predicate_sql' <<<"$snapshot")" \
      || return 1
    case "$database" in
      auth) mysql_function=detour_chase_auth_mysql ;;
      characters) mysql_function=loot_fixture_character_mysql ;;
      *) return 1 ;;
    esac
    case "$strategy" in
      update)
        restore_where="$(
          detour_chase_account_restore_guard \
            "$table" "$scope_column" "$scope_value"
        )" || return 1
        current_sql="$(
          detour_chase_snapshot_single_row_update_sql \
            "$mysql_function" "$table" \
            "\`${scope_column}\`=${scope_value}" "$restore_where"
        )" || return 1
        ;;
      delete_insert)
        current_sql="$(
          detour_chase_snapshot_table_insert_sql \
            "$mysql_function" "$table" \
            "\`${scope_column}\`=${scope_value}"
        )" || return 1
        ;;
      *) return 1 ;;
    esac
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
          echo "WARNING: detour ${database}.${table} differs from both prior and checkpointed fixture poststate; refusing cleanup writes" >&2
          return 1
      }
      if [ "$strategy" = update ]; then
        recovery_sql="
          SET autocommit=0;
          LOCK TABLES \`${table}\` WRITE;
          SET @detour_cas=IF((${post_predicate_sql}),1,0);
          ${restore_sql} AND @detour_cas=1;
          COMMIT;
          SELECT @detour_cas;
          UNLOCK TABLES;
          SET autocommit=1;"
      else
        recovery_sql="
          SET autocommit=0;
          LOCK TABLES \`${table}\` WRITE;
          SET @detour_cas=IF((${post_predicate_sql}),1,0);
          DELETE FROM \`${table}\`
            WHERE \`${scope_column}\`=${scope_value}
              AND @detour_cas=1;
          ${restore_sql}
          COMMIT;
          SELECT @detour_cas;
          UNLOCK TABLES;
          SET autocommit=1;"
      fi
      cas_result="$(
        detour_chase_execute_recovery_sql "$database" "$recovery_sql"
      )" || return 1
      [ "$cas_result" = 1 ] || {
        echo "WARNING: detour ${database}.${table} CAS failed under WRITE lock; journal retained" >&2
        return 1
      }
      if [ "$strategy" = update ]; then
        current_sql="$(
          detour_chase_snapshot_single_row_update_sql \
            "$mysql_function" "$table" \
            "\`${scope_column}\`=${scope_value}" "$restore_where"
        )" || return 1
      else
        current_sql="$(
          detour_chase_snapshot_table_insert_sql \
            "$mysql_function" "$table" \
            "\`${scope_column}\`=${scope_value}"
        )" || return 1
      fi
      current_sha="$(detour_chase_sha256_of_text "$current_sql")" || return 1
      legacy_current_sha="$(
        detour_chase_sha256_of_text "$(
          detour_chase_legacy_insert_restore_sql "$current_sql"
        )"
      )" || return 1
      [ "$current_sha" = "$normalized_prior_sha" ] \
        || [ "$legacy_current_sha" = "$prior_sha" ] || {
        echo "WARNING: detour ${database}.${table} did not restore exactly" >&2
        return 1
      }
    fi
  done < <(jq -r '.[] | @base64' \
    <<<"$DETOUR_FIXTURE_ACCOUNT_SNAPSHOTS_JSON")
  if [ "$restore_xtrace" -eq 1 ]; then
    set -x
  fi
}

detour_chase_compute_database_snapshot_sha256() {
  {
    printf 'creature-exists\0%s\0' "$DETOUR_FIXTURE_PRIOR_CREATURE_EXISTS"
    printf 'creature\0%s\0' "$DETOUR_FIXTURE_PRIOR_CREATURE_SHA256"
    printf 'character\0%s\0' "$DETOUR_FIXTURE_PRIOR_CHARACTER_SHA256"
    printf 'character-aux\0%s\0' "$DETOUR_FIXTURE_CHARACTER_AUX_SNAPSHOTS_JSON"
    printf 'respawn\0%s\0' "$DETOUR_FIXTURE_RESPAWN_SNAPSHOT_JSON"
    printf 'world-aux\0%s\0' "$DETOUR_FIXTURE_WORLD_AUX_SHA256"
    printf 'account\0%s\0' "$DETOUR_FIXTURE_ACCOUNT_SNAPSHOTS_JSON"
  } | sha256sum | awk '{print $1}'
}
