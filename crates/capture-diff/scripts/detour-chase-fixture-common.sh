#!/usr/bin/env bash
# Shared, source-only guard for the deterministic issue-#24 chase fixture.
#
# The caller owns the service lifecycle.  It must:
#   1. call detour_chase_validate_capture_orchestration before confirmation;
#   2. install its EXIT trap;
#   3. call detour_chase_prepare_private_data_dir;
#   4. stop both worlds and call detour_chase_apply_fixture_guard;
#   5. after stopping the capture world, call
#      detour_chase_restore_fixture_guard and
#      detour_chase_remove_private_data_dir before restarting normal PM2.
#
# WOW_BOT_FIXTURE_JOURNAL is deliberately shared with the existing guarded-bot
# contract.  The wrapper, not the bot, owns this journal for this flow.  It
# durably records the DB restore plan before the first write and turns it into
# the existing mode-0600 cleanup-complete marker after verified restoration.

DETOUR_FIXTURE_FLOW="detour-chase-around-obstacle"
DETOUR_FIXTURE_CREATURE_GUID=9102401
DETOUR_FIXTURE_CREATURE_ENTRY=15271
DETOUR_FIXTURE_CHARACTER_GUID=15
DETOUR_FIXTURE_CHARACTER_ACCOUNT=9
DETOUR_FIXTURE_MAP_ID=1
DETOUR_FIXTURE_MAP_SHA256="3ff3365bbd0aafb383f4c2984389d07df133dd86cdb0b9340c25361db32d8f5a"
DETOUR_FIXTURE_TILE_SHA256="693b93ac3ac605fea8b846a0e1fcf6ca2d0b0dce2f8c5d9c34739febc3731f47"
DETOUR_FIXTURE_MANIFEST_EXPECTED_SHA256="3a6c2aa6081974ef9cf13b8f63f739c402b18799f9833f80819f5e7e0de8d013"
DETOUR_FIXTURE_MAP_BYTES=28
DETOUR_FIXTURE_TILE_BYTES=1496
DETOUR_FIXTURE_PLAYER_X="-10118.333"
DETOUR_FIXTURE_PLAYER_Y="2670.667"
DETOUR_FIXTURE_PLAYER_Z="218.49"
DETOUR_FIXTURE_PLAYER_O="1.5707964"
DETOUR_FIXTURE_CREATURE_X="-10118.333"
DETOUR_FIXTURE_CREATURE_Y="2671.667"
DETOUR_FIXTURE_CREATURE_Z="218.49"
DETOUR_FIXTURE_PING_FENCE_WIRE="DTOR"

DETOUR_FIXTURE_ENABLED="${DETOUR_FIXTURE_ENABLED:-0}"
DETOUR_FIXTURE_SIDE="${DETOUR_FIXTURE_SIDE:-}"
DETOUR_FIXTURE_REPO_ROOT="${DETOUR_FIXTURE_REPO_ROOT:-}"
DETOUR_FIXTURE_DB_CONF="${DETOUR_FIXTURE_DB_CONF:-}"
DETOUR_FIXTURE_DB_CONF_SHA256="${DETOUR_FIXTURE_DB_CONF_SHA256:-}"
DETOUR_FIXTURE_DB_CONF_IDENTITY="${DETOUR_FIXTURE_DB_CONF_IDENTITY:-}"
DETOUR_FIXTURE_CONFIG="${DETOUR_FIXTURE_CONFIG:-}"
DETOUR_FIXTURE_MANIFEST="${DETOUR_FIXTURE_MANIFEST:-}"
DETOUR_FIXTURE_MANIFEST_SHA256="${DETOUR_FIXTURE_MANIFEST_SHA256:-}"
DETOUR_FIXTURE_SOURCE_ROOT="${DETOUR_FIXTURE_SOURCE_ROOT:-}"
DETOUR_FIXTURE_NORMAL_DATA_DIR="${DETOUR_FIXTURE_NORMAL_DATA_DIR:-}"
DETOUR_FIXTURE_PRIVATE_DATA_DIR="${DETOUR_FIXTURE_PRIVATE_DATA_DIR:-}"
DETOUR_FIXTURE_PRIVATE_DATA_DIR_IDENTITY="${DETOUR_FIXTURE_PRIVATE_DATA_DIR_IDENTITY:-}"
DETOUR_FIXTURE_RUST_CONFIG="${DETOUR_FIXTURE_RUST_CONFIG:-}"
DETOUR_FIXTURE_RUST_CONFIG_SHA256="${DETOUR_FIXTURE_RUST_CONFIG_SHA256:-}"
DETOUR_FIXTURE_RUST_CONFIG_IDENTITY="${DETOUR_FIXTURE_RUST_CONFIG_IDENTITY:-}"
DETOUR_FIXTURE_JOURNAL_SHA256="${DETOUR_FIXTURE_JOURNAL_SHA256:-}"
DETOUR_FIXTURE_PRIOR_CREATURE_EXISTS="${DETOUR_FIXTURE_PRIOR_CREATURE_EXISTS:-0}"
DETOUR_FIXTURE_PRIOR_CREATURE_SHA256="${DETOUR_FIXTURE_PRIOR_CREATURE_SHA256:-}"
DETOUR_FIXTURE_CREATURE_RESTORE_SQL="${DETOUR_FIXTURE_CREATURE_RESTORE_SQL:-}"
DETOUR_FIXTURE_FIXTURE_CREATURE_SHA256="${DETOUR_FIXTURE_FIXTURE_CREATURE_SHA256:-}"
DETOUR_FIXTURE_CHARACTER_IDENTITY_SHA256="${DETOUR_FIXTURE_CHARACTER_IDENTITY_SHA256:-}"
DETOUR_FIXTURE_CHARACTER_STABLE_SHA256="${DETOUR_FIXTURE_CHARACTER_STABLE_SHA256:-}"
DETOUR_FIXTURE_PRIOR_CHARACTER_SHA256="${DETOUR_FIXTURE_PRIOR_CHARACTER_SHA256:-}"
DETOUR_FIXTURE_CHARACTER_RESTORE_SQL="${DETOUR_FIXTURE_CHARACTER_RESTORE_SQL:-}"
DETOUR_FIXTURE_CHARACTER_PRIOR_PREDICATE_SQL="${DETOUR_FIXTURE_CHARACTER_PRIOR_PREDICATE_SQL:-}"
DETOUR_FIXTURE_CHARACTER_AUX_SNAPSHOTS_JSON="${DETOUR_FIXTURE_CHARACTER_AUX_SNAPSHOTS_JSON:-[]}"
DETOUR_FIXTURE_RESPAWN_SNAPSHOT_JSON="${DETOUR_FIXTURE_RESPAWN_SNAPSHOT_JSON:-{}}"
DETOUR_FIXTURE_WORLD_AUX_SHA256="${DETOUR_FIXTURE_WORLD_AUX_SHA256:-}"
DETOUR_FIXTURE_ACCOUNT_SNAPSHOTS_JSON="${DETOUR_FIXTURE_ACCOUNT_SNAPSHOTS_JSON:-[]}"
DETOUR_FIXTURE_DATABASE_SNAPSHOT_SHA256="${DETOUR_FIXTURE_DATABASE_SNAPSHOT_SHA256:-}"
DETOUR_FIXTURE_POSTSTATE_CHECKPOINTED="${DETOUR_FIXTURE_POSTSTATE_CHECKPOINTED:-0}"
DETOUR_FIXTURE_POST_CHARACTER_SHA256="${DETOUR_FIXTURE_POST_CHARACTER_SHA256:-}"
DETOUR_FIXTURE_POST_CHARACTER_PREDICATE_SQL="${DETOUR_FIXTURE_POST_CHARACTER_PREDICATE_SQL:-}"
DETOUR_FIXTURE_POST_WORLD_AUX_SHA256="${DETOUR_FIXTURE_POST_WORLD_AUX_SHA256:-}"
DETOUR_FIXTURE_DB_APPLIED="${DETOUR_FIXTURE_DB_APPLIED:-0}"
DETOUR_FIXTURE_DB_RESTORED="${DETOUR_FIXTURE_DB_RESTORED:-0}"
DETOUR_FIXTURE_FILESYSTEM_RESTORED="${DETOUR_FIXTURE_FILESYSTEM_RESTORED:-0}"
DETOUR_FIXTURE_NORMAL_RUNTIME_RESTORED="${DETOUR_FIXTURE_NORMAL_RUNTIME_RESTORED:-0}"
DETOUR_FIXTURE_CLEANUP_VERIFIED="${DETOUR_FIXTURE_CLEANUP_VERIFIED:-0}"
DETOUR_FIXTURE_BOT_READY="${DETOUR_FIXTURE_BOT_READY:-0}"
DETOUR_FIXTURE_BOT_EXEC="${DETOUR_FIXTURE_BOT_EXEC:-}"
DETOUR_FIXTURE_BOT_EXEC_SHA256="${DETOUR_FIXTURE_BOT_EXEC_SHA256:-}"
DETOUR_FIXTURE_BOT_REPORT="${DETOUR_FIXTURE_BOT_REPORT:-}"
DETOUR_FIXTURE_BOT_REPORT_SHA256="${DETOUR_FIXTURE_BOT_REPORT_SHA256:-}"
DETOUR_FIXTURE_AUTH_HOST="${DETOUR_FIXTURE_AUTH_HOST:-}"
DETOUR_FIXTURE_AUTH_PORT="${DETOUR_FIXTURE_AUTH_PORT:-}"
DETOUR_FIXTURE_AUTH_USER="${DETOUR_FIXTURE_AUTH_USER:-}"
DETOUR_FIXTURE_AUTH_PASSWORD="${DETOUR_FIXTURE_AUTH_PASSWORD:-}"
DETOUR_FIXTURE_AUTH_DATABASE="${DETOUR_FIXTURE_AUTH_DATABASE:-}"
DETOUR_FIXTURE_BNET_ACCOUNT_ID="${DETOUR_FIXTURE_BNET_ACCOUNT_ID:-0}"
DETOUR_FIXTURE_ORCHESTRATION_LOCK="${DETOUR_FIXTURE_ORCHESTRATION_LOCK:-}"
DETOUR_FIXTURE_PM2_RUST_WORLD="${DETOUR_FIXTURE_PM2_RUST_WORLD:-}"
DETOUR_FIXTURE_PM2_CPP_WORLD="${DETOUR_FIXTURE_PM2_CPP_WORLD:-}"
DETOUR_FIXTURE_WORLD_PORT="${DETOUR_FIXTURE_WORLD_PORT:-0}"
DETOUR_FIXTURE_INSTANCE_PORT="${DETOUR_FIXTURE_INSTANCE_PORT:-0}"
DETOUR_FIXTURE_PM2_RESTORE_FILE="${DETOUR_FIXTURE_PM2_RESTORE_FILE:-}"
DETOUR_FIXTURE_PM2_RESTORE_FILE_SHA256="${DETOUR_FIXTURE_PM2_RESTORE_FILE_SHA256:-}"
DETOUR_FIXTURE_PM2_RESTORE_FILE_IDENTITY="${DETOUR_FIXTURE_PM2_RESTORE_FILE_IDENTITY:-}"
DETOUR_FIXTURE_NORMAL_RUST_PM2_PROFILE_SHA256="${DETOUR_FIXTURE_NORMAL_RUST_PM2_PROFILE_SHA256:-}"
DETOUR_FIXTURE_NORMAL_RUST_CONFIG="${DETOUR_FIXTURE_NORMAL_RUST_CONFIG:-}"
DETOUR_FIXTURE_NORMAL_RUST_CONFIG_SHA256="${DETOUR_FIXTURE_NORMAL_RUST_CONFIG_SHA256:-}"
DETOUR_FIXTURE_NORMAL_RUST_CONFIG_IDENTITY="${DETOUR_FIXTURE_NORMAL_RUST_CONFIG_IDENTITY:-}"
DETOUR_FIXTURE_CAPTURE_CONFIG_FILE="${DETOUR_FIXTURE_CAPTURE_CONFIG_FILE:-}"
DETOUR_FIXTURE_CAPTURE_CONFIG_FILE_SHA256="${DETOUR_FIXTURE_CAPTURE_CONFIG_FILE_SHA256:-}"
DETOUR_FIXTURE_CAPTURE_CONFIG_FILE_IDENTITY="${DETOUR_FIXTURE_CAPTURE_CONFIG_FILE_IDENTITY:-}"
DETOUR_FIXTURE_CPP_CONFIG="${DETOUR_FIXTURE_CPP_CONFIG:-}"
DETOUR_FIXTURE_CPP_CONFIG_BACKUP="${DETOUR_FIXTURE_CPP_CONFIG_BACKUP:-}"
DETOUR_FIXTURE_CPP_CONFIG_BACKUP_IDENTITY="${DETOUR_FIXTURE_CPP_CONFIG_BACKUP_IDENTITY:-}"
DETOUR_FIXTURE_CPP_CONFIG_BACKUP_SHA256="${DETOUR_FIXTURE_CPP_CONFIG_BACKUP_SHA256:-}"

detour_chase_sha256_of_file() {
  local output digest
  output="$(sha256sum <"$1")" || return 1
  digest="${output%% *}"
  [[ "$digest" =~ ^[0-9a-f]{64}$ ]] || return 1
  printf '%s\n' "$digest"
}

detour_chase_sha256_of_text() {
  local output digest
  output="$(printf '%s' "$1" | sha256sum)" || return 1
  digest="${output%% *}"
  [[ "$digest" =~ ^[0-9a-f]{64}$ ]] || return 1
  printf '%s\n' "$digest"
}

detour_chase_ping_fence_serial() {
  local wire="$DETOUR_FIXTURE_PING_FENCE_WIRE"
  local b0 b1 b2 b3
  [ "${#wire}" = 4 ] || return 1
  printf -v b0 '%d' "'${wire:0:1}"
  printf -v b1 '%d' "'${wire:1:1}"
  printf -v b2 '%d' "'${wire:2:1}"
  printf -v b3 '%d' "'${wire:3:1}"
  printf '%u\n' "$((b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)))"
}

detour_chase_validate_sql_identifier() {
  [[ "$1" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]
}

detour_chase_secure_temp_file() {
  local parent
  if [ -n "${WOW_BOT_FIXTURE_JOURNAL:-}" ]; then
    parent="$(dirname -- "$WOW_BOT_FIXTURE_JOURNAL")"
  else
    parent="${TMPDIR:-/tmp}"
  fi
  [ -d "$parent" ] && [ ! -L "$parent" ] || return 1
  local path
  path="$(mktemp "${parent}/.detour-sensitive.XXXXXX")" || return 1
  chmod 600 "$path" || {
    rm -f -- "$path"
    return 1
  }
  printf '%s\n' "$path"
}

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
  local columns column data_type column_list="" values_expression="" delimiter=""
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
    column_list+="${delimiter}\`${column}\`"
    values_expression+="${delimiter}${serialized}"
    delimiter=","
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

detour_chase_restore_account_state() {
  local encoded snapshot database strategy table scope_column scope_value
  local prior_sha post_sha restore_sql post_predicate_sql mysql_function
  local restore_where current_sql current_sha cas_result recovery_sql
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
    if [ "$current_sha" != "$prior_sha" ]; then
      [ "$DETOUR_FIXTURE_POSTSTATE_CHECKPOINTED" = 1 ] \
        && [ "$current_sha" = "$post_sha" ] || {
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
      [ "$current_sha" = "$prior_sha" ] || {
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

detour_chase_validate_committed_fixture() {
  local repo_root="$1"
  local fixture_root manifest map_asset tile_asset
  local map_sha tile_sha map_size tile_size

  fixture_root="${repo_root}/crates/capture-diff/flows/${DETOUR_FIXTURE_FLOW}/fixture"
  manifest="${fixture_root}/fixture.json"
  map_asset="${fixture_root}/mmaps/0001.mmap"
  tile_asset="${fixture_root}/mmaps/00015026.mmtile"

  for dependency in base64 jq realpath sha256sum stat; do
    command -v "$dependency" >/dev/null 2>&1 || {
      echo "error: detour fixture validation requires ${dependency}" >&2
      return 1
    }
  done
  for path in "$manifest" "$map_asset" "$tile_asset"; do
    [ -f "$path" ] && [ ! -L "$path" ] \
      && [ "$(realpath -e -- "$path" 2>/dev/null)" = "$path" ] || {
        echo "error: detour fixture prerequisite is missing/non-canonical: ${path}" >&2
        return 1
      }
  done

  jq -e '
    (keys | sort) == ["assets","character","creature","flow","generator","geometry","map","schema_version"]
    and .schema_version == 1
    and .flow == "detour-chase-around-obstacle"
    and .generator == {
      "crate":"wow-recastdetour",
      "example":"generate_detour_chase_fixture",
      "feature":"test-fixtures"
    }
    and .map == {"id":1,"grid_x":50,"grid_y":26}
    and .geometry == {
      "centre":{"x":-10118.333,"y":2681.667,"z":218.49},
      "creature_start":{"x":-10118.333,"y":2671.667,"z":218.49},
      "player_start":{"x":-10118.333,"y":2670.667,"z":218.49,"orientation":1.5707964},
      "player_destination":{"x":-10118.333,"y":2691.667,"z":218.49,"orientation":-1.5707964},
      "obstacle":{"min_x":-10123.333,"max_x":-10113.333,"min_y":2676.667,"max_y":2686.667}
    }
    and .creature == {"entry":15271,"spawn_guid":9102401}
    and .character == {"guid":15}
    and .assets == [
      {
        "path":"mmaps/0001.mmap",
        "bytes":28,
        "sha256":"3ff3365bbd0aafb383f4c2984389d07df133dd86cdb0b9340c25361db32d8f5a"
      },
      {
        "path":"mmaps/00015026.mmtile",
        "bytes":1496,
        "sha256":"693b93ac3ac605fea8b846a0e1fcf6ca2d0b0dce2f8c5d9c34739febc3731f47"
      }
    ]
  ' "$manifest" >/dev/null || {
    echo "error: detour fixture manifest differs from the reviewed issue-#24 schema" >&2
    return 1
  }

  map_size="$(stat -c '%s' -- "$map_asset")" || return 1
  tile_size="$(stat -c '%s' -- "$tile_asset")" || return 1
  map_sha="$(detour_chase_sha256_of_file "$map_asset")" || return 1
  tile_sha="$(detour_chase_sha256_of_file "$tile_asset")" || return 1
  [ "$map_size" = "$DETOUR_FIXTURE_MAP_BYTES" ] \
    && [ "$tile_size" = "$DETOUR_FIXTURE_TILE_BYTES" ] \
    && [ "$map_sha" = "$DETOUR_FIXTURE_MAP_SHA256" ] \
    && [ "$tile_sha" = "$DETOUR_FIXTURE_TILE_SHA256" ] || {
      echo "error: committed detour fixture assets differ from fixture.json" >&2
      return 1
    }

  DETOUR_FIXTURE_SOURCE_ROOT="$fixture_root"
  DETOUR_FIXTURE_MANIFEST="$manifest"
  DETOUR_FIXTURE_MANIFEST_SHA256="$(detour_chase_sha256_of_file "$manifest")" \
    || return 1
  [ "$DETOUR_FIXTURE_MANIFEST_SHA256" \
      = "$DETOUR_FIXTURE_MANIFEST_EXPECTED_SHA256" ] || {
    echo "error: detour fixture manifest bytes differ from the reviewed digest" >&2
    return 1
  }
}

detour_chase_read_config_value() {
  local config_path="$1"
  local key="$2"
  local value
  value="$(
    awk -F '"' -v key="$key" '
      $0 ~ "^[[:space:]]*" key "[[:space:]]*=" { value=$2 }
      END { if (value != "") print value }
    ' "$config_path"
  )" || return 1
  [ -n "$value" ] || return 1
  printf '%s\n' "$value"
}

detour_chase_validate_normal_data_dir() {
  local config_path="$1"
  local configured canonical child

  [ -f "$config_path" ] && [ ! -L "$config_path" ] \
    && [ "$(realpath -e -- "$config_path" 2>/dev/null)" = "$config_path" ] \
    || {
      echo "error: detour capture config must be a canonical regular non-symlink file" >&2
      return 1
    }
  configured="$(detour_chase_read_config_value "$config_path" DataDir)" || {
    echo "error: detour capture config has no quoted DataDir" >&2
    return 1
  }
  [[ "$configured" = /* ]] || {
    echo "error: detour capture requires an absolute DataDir to avoid cwd-dependent evidence" >&2
    return 1
  }
  canonical="$(realpath -e -- "$configured" 2>/dev/null)" || {
    echo "error: configured DataDir does not resolve: ${configured}" >&2
    return 1
  }
  [ -d "$canonical" ] && [ ! -L "$canonical" ] || {
    echo "error: normal DataDir must resolve to a real directory" >&2
    return 1
  }
  for child in dbc gt maps vmaps cameras; do
    [ -d "$canonical/$child" ] && [ ! -L "$canonical/$child" ] \
      && [ "$(realpath -e -- "$canonical/$child" 2>/dev/null)" \
        = "$canonical/$child" ] || {
        echo "error: normal DataDir/${child} is missing, non-canonical, or a symlink" >&2
        return 1
      }
  done
  DETOUR_FIXTURE_NORMAL_DATA_DIR="$canonical"
}

detour_chase_validate_bot_inputs() {
  [ -n "${WOW_BOT_EXEC:-}" ] \
    && [ -n "${WOW_BOT_EXEC_SHA256:-}" ] \
    && [ -n "${WOW_BOT_REPORT:-}" ] || {
      echo "error: detour capture requires WOW_BOT_EXEC, WOW_BOT_EXEC_SHA256, and WOW_BOT_REPORT" >&2
      return 1
    }
  [[ "$WOW_BOT_EXEC" != *$'\t'* && "$WOW_BOT_EXEC" != *$'\n'* \
    && "$WOW_BOT_REPORT" != *$'\t'* && "$WOW_BOT_REPORT" != *$'\n'* ]] || {
    echo "error: detour bot executable/report paths cannot contain tabs or newlines" >&2
    return 1
  }
  [[ "$WOW_BOT_EXEC_SHA256" =~ ^[0-9A-Fa-f]{64}$ ]] || {
    echo "error: WOW_BOT_EXEC_SHA256 must contain exactly 64 hexadecimal characters" >&2
    return 1
  }
  WOW_BOT_EXEC_SHA256="${WOW_BOT_EXEC_SHA256,,}"
  capture_validate_fresh_bot_inputs \
    "$WOW_BOT_EXEC" "$WOW_BOT_EXEC_SHA256" "$WOW_BOT_REPORT" || {
      echo "error: detour bot executable/report inputs are not fresh, canonical, and pinned" >&2
      return 1
    }
  DETOUR_FIXTURE_BOT_EXEC="$(realpath -e -- "$WOW_BOT_EXEC")" || return 1
  DETOUR_FIXTURE_BOT_EXEC_SHA256="$WOW_BOT_EXEC_SHA256"
  DETOUR_FIXTURE_BOT_REPORT="$WOW_BOT_REPORT"
}

detour_chase_validate_recovery_metadata() {
  [[ "$DETOUR_FIXTURE_DB_CONF" = /* \
    && "$DETOUR_FIXTURE_ORCHESTRATION_LOCK" = /* ]] || return 1
  [ -d "$DETOUR_FIXTURE_PRIVATE_DATA_DIR" ] \
    && [ ! -L "$DETOUR_FIXTURE_PRIVATE_DATA_DIR" ] \
    && [ "$(stat -c '%a' -- "$DETOUR_FIXTURE_PRIVATE_DATA_DIR")" = 700 ] \
    && [ "$(stat -c '%d:%i' -- "$DETOUR_FIXTURE_PRIVATE_DATA_DIR")" \
      = "$DETOUR_FIXTURE_PRIVATE_DATA_DIR_IDENTITY" ] || return 1
  [ -f "$DETOUR_FIXTURE_DB_CONF" ] \
    && [ ! -L "$DETOUR_FIXTURE_DB_CONF" ] \
    && [ "$(realpath -e -- "$DETOUR_FIXTURE_DB_CONF" 2>/dev/null)" \
      = "$DETOUR_FIXTURE_DB_CONF" ] \
    && [ "$(stat -c '%d:%i' -- "$DETOUR_FIXTURE_DB_CONF")" \
      = "$DETOUR_FIXTURE_DB_CONF_IDENTITY" ] \
    && [ "$(detour_chase_sha256_of_file "$DETOUR_FIXTURE_DB_CONF")" \
      = "$DETOUR_FIXTURE_DB_CONF_SHA256" ] || return 1
  [[ "$DETOUR_FIXTURE_PM2_RUST_WORLD" =~ ^[A-Za-z0-9._-]+$ \
    && "$DETOUR_FIXTURE_PM2_CPP_WORLD" =~ ^[A-Za-z0-9._-]+$ \
    && "$DETOUR_FIXTURE_WORLD_PORT" =~ ^[1-9][0-9]*$ \
    && "$DETOUR_FIXTURE_INSTANCE_PORT" =~ ^[1-9][0-9]*$ ]] \
    && ((DETOUR_FIXTURE_WORLD_PORT <= 65535)) \
    && ((DETOUR_FIXTURE_INSTANCE_PORT <= 65535)) \
    && [ "$DETOUR_FIXTURE_WORLD_PORT" != "$DETOUR_FIXTURE_INSTANCE_PORT" ] \
    && [[ "$DETOUR_FIXTURE_NORMAL_RUST_PM2_PROFILE_SHA256" \
      =~ ^[0-9a-f]{64}$ ]] \
    && [[ "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG" = /* ]] \
    || return 1
  [ -f "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG" ] \
    && [ ! -L "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG" ] \
    && [ "$(realpath -e -- "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG")" \
      = "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG" ] \
    && [ "$(stat -c '%d:%i' -- "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG")" \
      = "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG_IDENTITY" ] \
    && [ "$(detour_chase_sha256_of_file \
      "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG")" \
      = "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG_SHA256" ] || return 1
  case "$DETOUR_FIXTURE_SIDE" in
    cpp)
      [ -f "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP" ] \
        && [ ! -L "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP" ] \
        && [[ "$DETOUR_FIXTURE_CPP_CONFIG" = /* ]] \
        && [ "$(stat -c '%d:%i' -- "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP")" \
          = "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP_IDENTITY" ] \
        && [ "$(detour_chase_sha256_of_file \
          "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP")" \
          = "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP_SHA256" ] || return 1
      ;;
    rust)
      [ -f "$DETOUR_FIXTURE_PM2_RESTORE_FILE" ] \
        && [ ! -L "$DETOUR_FIXTURE_PM2_RESTORE_FILE" ] \
        && [ "$(stat -c '%a' -- "$DETOUR_FIXTURE_PM2_RESTORE_FILE")" = 600 ] \
        && [ "$(detour_chase_sha256_of_file \
          "$DETOUR_FIXTURE_PM2_RESTORE_FILE")" \
          = "$DETOUR_FIXTURE_PM2_RESTORE_FILE_SHA256" ] \
        && [ "$(stat -c '%d:%i' -- "$DETOUR_FIXTURE_PM2_RESTORE_FILE")" \
          = "$DETOUR_FIXTURE_PM2_RESTORE_FILE_IDENTITY" ] \
        && [ -f "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE" ] \
        && [ ! -L "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE" ] \
        && [ "$(stat -c '%a' -- "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE")" \
          = 600 ] \
        && [ "$(stat -c '%d:%i' -- "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE")" \
          = "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE_IDENTITY" ] \
        && [ "$(detour_chase_sha256_of_file \
          "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE")" \
          = "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE_SHA256" ] || return 1
      if [ -n "$DETOUR_FIXTURE_RUST_CONFIG" ]; then
        [ -f "$DETOUR_FIXTURE_RUST_CONFIG" ] \
          && [ ! -L "$DETOUR_FIXTURE_RUST_CONFIG" ] \
          && [ "$(stat -c '%a' -- "$DETOUR_FIXTURE_RUST_CONFIG")" = 600 ] \
          && [ "$(stat -c '%d:%i' -- "$DETOUR_FIXTURE_RUST_CONFIG")" \
            = "$DETOUR_FIXTURE_RUST_CONFIG_IDENTITY" ] \
          && [ "$(detour_chase_sha256_of_file \
            "$DETOUR_FIXTURE_RUST_CONFIG")" \
            = "$DETOUR_FIXTURE_RUST_CONFIG_SHA256" ] || return 1
      else
        [ "$DETOUR_FIXTURE_DB_APPLIED" = 0 ] \
          && [ -z "$DETOUR_FIXTURE_RUST_CONFIG_SHA256" ] \
          && [ -z "$DETOUR_FIXTURE_RUST_CONFIG_IDENTITY" ] || return 1
      fi
      ;;
    *) return 1 ;;
  esac
}

detour_chase_pinned_file_matches() {
  local path="$1"
  local identity="$2"
  local digest="$3"
  [ -f "$path" ] && [ ! -L "$path" ] \
    && [ "$(realpath -e -- "$path" 2>/dev/null)" = "$path" ] \
    && [ "$(stat -c '%d:%i' -- "$path" 2>/dev/null)" = "$identity" ] \
    && [ "$(detour_chase_sha256_of_file "$path" 2>/dev/null)" = "$digest" ]
}

# These two files decide which databases recovery writes and how the normal
# service restarts. Re-accredit them under the orchestration lock before any
# recovery mutation. On the C++ side the exact backup inode is atomically moved
# back over the active config; accept that one journaled relocation only.
detour_chase_validate_recovery_anchor_files() {
  if ! detour_chase_pinned_file_matches \
      "$DETOUR_FIXTURE_DB_CONF" \
      "$DETOUR_FIXTURE_DB_CONF_IDENTITY" \
      "$DETOUR_FIXTURE_DB_CONF_SHA256"; then
    [ "$DETOUR_FIXTURE_SIDE" = cpp ] \
      && detour_chase_pinned_file_matches \
        "$DETOUR_FIXTURE_CPP_CONFIG" \
        "$DETOUR_FIXTURE_DB_CONF_IDENTITY" \
        "$DETOUR_FIXTURE_DB_CONF_SHA256" || return 1
    DETOUR_FIXTURE_DB_CONF="$DETOUR_FIXTURE_CPP_CONFIG"
  fi
  if [ "$DETOUR_FIXTURE_SIDE" = cpp ] \
      && [ "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG" \
        = "$DETOUR_FIXTURE_CPP_CONFIG" ]; then
    if [ -f "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG" ] \
        && [ ! -L "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG" ] \
        && [ "$(detour_chase_sha256_of_file \
          "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG" 2>/dev/null)" \
          = "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG_SHA256" ]; then
      return 0
    fi
    detour_chase_pinned_file_matches \
      "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP" \
      "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP_IDENTITY" \
      "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG_SHA256"
    return
  fi
  detour_chase_pinned_file_matches \
    "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG" \
    "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG_IDENTITY" \
    "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG_SHA256"
}

detour_chase_arm_filesystem_recovery_journal() {
  [ "$DETOUR_FIXTURE_ENABLED" = 1 ] \
    && [ "$DETOUR_FIXTURE_DB_APPLIED" = 0 ] || return 1
  detour_chase_validate_recovery_metadata || return 1
  DETOUR_FIXTURE_PRIOR_CREATURE_EXISTS=0
  DETOUR_FIXTURE_PRIOR_CREATURE_SHA256=""
  DETOUR_FIXTURE_CREATURE_RESTORE_SQL=""
  DETOUR_FIXTURE_FIXTURE_CREATURE_SHA256=""
  DETOUR_FIXTURE_CHARACTER_IDENTITY_SHA256=""
  DETOUR_FIXTURE_CHARACTER_STABLE_SHA256=""
  DETOUR_FIXTURE_PRIOR_CHARACTER_SHA256=""
  DETOUR_FIXTURE_CHARACTER_RESTORE_SQL=""
  DETOUR_FIXTURE_CHARACTER_AUX_SNAPSHOTS_JSON="[]"
  DETOUR_FIXTURE_RESPAWN_SNAPSHOT_JSON="{}"
  DETOUR_FIXTURE_WORLD_AUX_SHA256=""
  DETOUR_FIXTURE_ACCOUNT_SNAPSHOTS_JSON="[]"
  DETOUR_FIXTURE_DATABASE_SNAPSHOT_SHA256=""
  DETOUR_FIXTURE_POSTSTATE_CHECKPOINTED=0
  DETOUR_FIXTURE_POST_CHARACTER_SHA256=""
  DETOUR_FIXTURE_POST_CHARACTER_PREDICATE_SQL=""
  DETOUR_FIXTURE_POST_WORLD_AUX_SHA256=""
  DETOUR_FIXTURE_BNET_ACCOUNT_ID=0
  DETOUR_FIXTURE_DB_APPLIED=0
  DETOUR_FIXTURE_DB_RESTORED=0
  DETOUR_FIXTURE_FILESYSTEM_RESTORED=0
  DETOUR_FIXTURE_NORMAL_RUNTIME_RESTORED=0
  detour_chase_write_fixture_journal create
}

detour_chase_checkpoint_filesystem_recovery_metadata() {
  [ "$DETOUR_FIXTURE_DB_APPLIED" = 0 ] \
    && [ -n "$DETOUR_FIXTURE_RUST_CONFIG" ] \
    && detour_chase_validate_recovery_metadata \
    && detour_chase_write_fixture_journal replace
}

detour_chase_validate_capture_orchestration() {
  local repo_root="$1"
  local config_path="$2"
  local side="$3"

  [ "${DETOUR_CAPTURE_ACK_FIXTURE_MUTATION:-0}" = "1" ] || {
    echo "error: detour capture requires DETOUR_CAPTURE_ACK_FIXTURE_MUTATION=1" >&2
    return 1
  }
  case "$side" in
    cpp|rust) ;;
    *)
      echo "error: internal detour fixture side must be cpp or rust" >&2
      return 1
      ;;
  esac
  for dependency in awk cat chmod cp dirname id jq ln mkdir mktemp mv readlink \
    realpath rm sha256sum stat sync tr wc; do
    command -v "$dependency" >/dev/null 2>&1 || {
      echo "error: detour fixture orchestration requires ${dependency}" >&2
      return 1
    }
  done
  detour_chase_validate_committed_fixture "$repo_root" || return 1
  detour_chase_validate_normal_data_dir "$config_path" || return 1
  validate_fresh_loot_fixture_journal || return 1
  detour_chase_validate_bot_inputs || return 1

  DETOUR_FIXTURE_ENABLED=1
  DETOUR_FIXTURE_SIDE="$side"
  DETOUR_FIXTURE_REPO_ROOT="$repo_root"
  DETOUR_FIXTURE_CONFIG="$config_path"
}

detour_chase_allocate_private_data_dir() {
  local private_parent private_root

  [ "$DETOUR_FIXTURE_ENABLED" = "1" ] \
    && [ -n "$DETOUR_FIXTURE_NORMAL_DATA_DIR" ] \
    && [ -n "$DETOUR_FIXTURE_SOURCE_ROOT" ] || return 1
  [ -z "$DETOUR_FIXTURE_PRIVATE_DATA_DIR" ] || return 1

  private_parent="${DETOUR_CAPTURE_PRIVATE_PARENT:-${XDG_RUNTIME_DIR:-/tmp}}"
  [ -d "$private_parent" ] && [ ! -L "$private_parent" ] \
    && [ "$(realpath -e -- "$private_parent" 2>/dev/null)" = "$private_parent" ] \
    || {
      echo "error: detour private-DataDir parent must be a canonical real directory" >&2
      return 1
    }
  private_root="$(mktemp -d \
    "${private_parent}/rustycore-detour-${DETOUR_FIXTURE_SIDE}.XXXXXX")" \
    || return 1
  chmod 700 "$private_root" || {
    rm -rf -- "$private_root"
    return 1
  }
  [ "$(realpath -e -- "$private_root" 2>/dev/null)" = "$private_root" ] \
    && [ ! -L "$private_root" ] || {
      rm -rf -- "$private_root"
      return 1
    }
  DETOUR_FIXTURE_PRIVATE_DATA_DIR="$private_root"
  DETOUR_FIXTURE_PRIVATE_DATA_DIR_IDENTITY="$(stat -c '%d:%i' -- "$private_root")" \
    || return 1
}

detour_chase_populate_private_data_dir() {
  local private_root="$DETOUR_FIXTURE_PRIVATE_DATA_DIR"
  local child source target fixture_map fixture_tile

  [ "$DETOUR_FIXTURE_ENABLED" = "1" ] \
    && [ -n "$DETOUR_FIXTURE_NORMAL_DATA_DIR" ] \
    && [ -n "$DETOUR_FIXTURE_SOURCE_ROOT" ] \
    && [ -d "$private_root" ] \
    && [ ! -L "$private_root" ] \
    && [ "$(stat -c '%d:%i' -- "$private_root" 2>/dev/null)" \
      = "$DETOUR_FIXTURE_PRIVATE_DATA_DIR_IDENTITY" ] || return 1
  for child in dbc gt maps vmaps cameras; do
    source="$DETOUR_FIXTURE_NORMAL_DATA_DIR/$child"
    target="$private_root/$child"
    ln -s -- "$source" "$target" || return 1
    [ -L "$target" ] \
      && [ "$(readlink -- "$target")" = "$source" ] \
      && [ "$(realpath -e -- "$target" 2>/dev/null)" = "$source" ] || return 1
  done
  mkdir -- "$private_root/mmaps" || return 1
  chmod 700 "$private_root/mmaps" || return 1
  fixture_map="$DETOUR_FIXTURE_SOURCE_ROOT/mmaps/0001.mmap"
  fixture_tile="$DETOUR_FIXTURE_SOURCE_ROOT/mmaps/00015026.mmtile"
  cp --update=none -- "$fixture_map" "$private_root/mmaps/0001.mmap" \
    && cp --update=none -- "$fixture_tile" \
      "$private_root/mmaps/00015026.mmtile" || return 1
  chmod 400 \
    "$private_root/mmaps/0001.mmap" \
    "$private_root/mmaps/00015026.mmtile" || return 1
  [ ! -L "$private_root/mmaps/0001.mmap" ] \
    && [ ! -L "$private_root/mmaps/00015026.mmtile" ] \
    && [ "$(detour_chase_sha256_of_file \
      "$private_root/mmaps/0001.mmap")" = "$DETOUR_FIXTURE_MAP_SHA256" ] \
    && [ "$(detour_chase_sha256_of_file \
      "$private_root/mmaps/00015026.mmtile")" = "$DETOUR_FIXTURE_TILE_SHA256" ] \
    && [ "$(stat -c '%s' -- "$private_root/mmaps/0001.mmap")" \
      = "$DETOUR_FIXTURE_MAP_BYTES" ] \
    && [ "$(stat -c '%s' -- "$private_root/mmaps/00015026.mmtile")" \
      = "$DETOUR_FIXTURE_TILE_BYTES" ] || {
      echo "error: private DataDir synthetic MMap copy failed accreditation" >&2
      return 1
    }
  sync -f "$private_root/mmaps/0001.mmap" \
    && sync -f "$private_root/mmaps/00015026.mmtile" \
    && sync -f "$private_root/mmaps" \
    && sync -f "$private_root" || return 1
}

detour_chase_prepare_private_data_dir() {
  detour_chase_allocate_private_data_dir \
    && detour_chase_populate_private_data_dir
}

detour_chase_patch_config_data_dir() {
  local config_path="$1"
  local private_data_dir="$2"
  local stage

  [ -f "$config_path" ] && [ ! -L "$config_path" ] \
    && [ -d "$private_data_dir" ] && [ ! -L "$private_data_dir" ] || return 1
  stage="$(mktemp "${config_path}.detour-stage.XXXXXX")" || return 1
  if ! awk -v data_dir="${private_data_dir}/" '
      BEGIN { replaced=0 }
      /^[[:space:]]*DataDir[[:space:]]*=/ {
        if (!replaced) {
          print "DataDir = \"" data_dir "\""
          replaced=1
        }
        next
      }
      { print }
      END {
        if (!replaced)
          print "DataDir = \"" data_dir "\""
      }
    ' "$config_path" >"$stage"; then
    rm -f -- "$stage"
    return 1
  fi
  chmod --reference="$config_path" "$stage" || {
    rm -f -- "$stage"
    return 1
  }
  mv -f -- "$stage" "$config_path" || return 1
  [ "$(detour_chase_read_config_value "$config_path" DataDir)" \
    = "${private_data_dir}/" ]
}

detour_chase_create_rust_capture_config() {
  local original_config="$1"
  local private_data_dir="$2"
  local private_parent temporary_config temporary_sha temporary_identity

  # Keep the credential-bearing temporary config inside the journaled private
  # root. If SIGKILL lands before its exact file metadata is checkpointed, the
  # unarmed recovery path can still remove the whole root by pinned inode.
  private_parent="$private_data_dir"
  temporary_config="$(mktemp \
    "${private_parent}/rustycore-detour-worldserver.XXXXXX.conf")" \
    || return 1
  cp -- "$original_config" "$temporary_config" || {
    rm -f -- "$temporary_config"
    return 1
  }
  chmod 600 "$temporary_config" || {
    rm -f -- "$temporary_config"
    return 1
  }
  detour_chase_patch_config_data_dir \
    "$temporary_config" "$private_data_dir" || {
      rm -f -- "$temporary_config"
      return 1
    }
  temporary_sha="$(detour_chase_sha256_of_file "$temporary_config")" || {
    rm -f -- "$temporary_config"
    return 1
  }
  temporary_identity="$(stat -c '%d:%i' -- "$temporary_config")" || {
    rm -f -- "$temporary_config"
    return 1
  }
  DETOUR_FIXTURE_RUST_CONFIG="$temporary_config"
  DETOUR_FIXTURE_RUST_CONFIG_SHA256="$temporary_sha"
  DETOUR_FIXTURE_RUST_CONFIG_IDENTITY="$temporary_identity"
}

detour_chase_patch_rust_pm2_capture_config() {
  local pm2_capture_file="$1"
  local temporary_config="$2"
  local stage

  stage="$(mktemp "${pm2_capture_file}.detour-stage.XXXXXX")" || return 1
  if ! jq --arg config "$temporary_config" '
      .apps[0].args as $args
      | ([range(0; ($args | length)) as $index
          | if $index > 0
              and ($args[$index - 1] == "-c"
                or $args[$index - 1] == "--config")
            then $config
            elif ($args[$index] | startswith("--config="))
            then "--config=" + $config
            else $args[$index]
            end]) as $rewritten
      | if any($args[];
          . == "-c" or . == "--config" or startswith("--config="))
        then .apps[0].args = $rewritten
        else .apps[0].args = ($args + ["--config", $config])
        end
    ' "$pm2_capture_file" >"$stage"; then
    rm -f -- "$stage"
    return 1
  fi
  chmod 600 "$stage" || {
    rm -f -- "$stage"
    return 1
  }
  mv -f -- "$stage" "$pm2_capture_file" || return 1
  jq -e --arg config "$temporary_config" '
    .apps[0].args as $args
    | ([range(0; ($args | length)) as $index
        | select(
            ($args[$index] == "--config=" + $config)
            or ($index > 0 and $args[$index] == $config
              and ($args[$index - 1] == "-c"
                or $args[$index - 1] == "--config"))
          )] | length) == 1
  ' "$pm2_capture_file" >/dev/null
}

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
      map=1,zone=0,instance_id=0,
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
           AND map=1 AND zone=0 AND instance_id=0
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
  local post_predicate_sql current_sql current_sha cas_result

  while IFS= read -r encoded; do
    snapshot="$(printf '%s' "$encoded" | base64 --decode)" || return 1
    table="$(jq -er '.table' <<<"$snapshot")" || return 1
    scope_column="$(jq -er '.scope_column' <<<"$snapshot")" || return 1
    prior_sha="$(jq -er '.prior_sha256' <<<"$snapshot")" || return 1
    post_sha="$(jq -er '.post_sha256' <<<"$snapshot")" || return 1
    restore_sql="$(jq -er '.restore_sql' <<<"$snapshot")" || return 1
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
    if [ "$current_sha" != "$prior_sha" ]; then
      [ "$DETOUR_FIXTURE_POSTSTATE_CHECKPOINTED" = 1 ] \
        && [ "$current_sha" = "$post_sha" ] || {
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
      [ "$current_sha" = "$prior_sha" ] || {
        echo "WARNING: detour character auxiliary table ${table} did not restore exactly" >&2
        return 1
      }
    fi
  done < <(jq -r '.[] | @base64' \
    <<<"$DETOUR_FIXTURE_CHARACTER_AUX_SNAPSHOTS_JSON")
}

detour_chase_restore_respawn_state() {
  local prior_sha post_sha restore_sql post_predicate_sql
  local current_sql current_sha cas_result

  prior_sha="$(jq -er '.prior_sha256' \
    <<<"$DETOUR_FIXTURE_RESPAWN_SNAPSHOT_JSON")" || return 1
  restore_sql="$(jq -er '.restore_sql' \
    <<<"$DETOUR_FIXTURE_RESPAWN_SNAPSHOT_JSON")" || return 1
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
  if [ "$current_sha" != "$prior_sha" ]; then
    [ "$DETOUR_FIXTURE_POSTSTATE_CHECKPOINTED" = 1 ] \
      && [ "$current_sha" = "$post_sha" ] || {
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
    [ "$current_sha" = "$prior_sha" ] || {
      echo "WARNING: detour creature respawn rows did not restore exactly" >&2
      return 1
    }
  fi
}

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
    and .results[0].detour_chase_target_runtime_counter == 9102401
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
