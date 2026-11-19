# CHAR_UPD_CHARACTER in the 3.4.3 C++ reference and Rust statement layer owns
# these 73 fields. Text is represented as H<uppercase-hex>, nullable text as N
# or H<hex>; every other token is numeric. This projection supplies the SET
# values for rollback, while an independent hash over all 87 columns is the
# atomic CAS predicate and detects drift even in columns outside this list.
creature_spell_fixture_validate_character_original_tsv() {
  local value="$1"
  jq -Rn -e --arg value "$value" '
    def number: test("^-?[0-9]+([.][0-9]+)?([eE][+-]?[0-9]+)?$");
    def hex: test("^H[0-9A-F]*$");
    def nullable_hex: test("^(N|H[0-9A-F]*)$");
    ($value | split("\t")) as $f
    | ($f | length) == 73
      and ($f[0] | hex)
      and all($f[1:26][]; number)
      and ($f[26] | hex)
      and all($f[27:43][]; number)
      and ($f[43] | nullable_hex)
      and all($f[44:63][]; number)
      and all($f[63:66][]; nullable_hex)
      and all($f[66:73][]; number)
  ' >/dev/null
}

creature_spell_fixture_validate_character_schema() {
  local schema
  schema="$(loot_fixture_character_mysql -e "
    SET SESSION group_concat_max_len = 1048576;
    SELECT COUNT(*),
           LENGTH(GROUP_CONCAT(CONCAT_WS(CHAR(31),
             COLUMN_NAME, COLUMN_TYPE, IS_NULLABLE,
             IFNULL(COLUMN_DEFAULT, '<NULL>'), EXTRA)
             ORDER BY ORDINAL_POSITION SEPARATOR '|')),
           LOWER(SHA2(GROUP_CONCAT(CONCAT_WS(CHAR(31),
             COLUMN_NAME, COLUMN_TYPE, IS_NULLABLE,
             IFNULL(COLUMN_DEFAULT, '<NULL>'), EXTRA)
             ORDER BY ORDINAL_POSITION SEPARATOR '|'), 256))
      FROM information_schema.COLUMNS
     WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'characters';
  ")" || return 1
  [ "$schema" = "${CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_COLUMN_COUNT}"$'\t'"${CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_METADATA_BYTES}"$'\t'"${CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_SHA256}" ] || {
    echo "error: characters table schema differs from the pinned 87-column creature spell CAS contract (${schema:-query-failed})" >&2
    return 1
  }
}

creature_spell_fixture_character_row_hash_expression() {
  local projection="${1:-current}"
  local column expression separator="" joined=""
  local -a columns=(
    guid account name slot race class gender level xp money inventorySlots
    bankSlots restState playerFlags playerFlagsEx position_x position_y
    position_z map instance_id dungeonDifficulty raidDifficulty
    legacyRaidDifficulty orientation taximask online createTime createMode
    cinematic totaltime leveltime logout_time is_logout_resting rest_bonus
    resettalents_cost resettalents_time numRespecs activeTalentGroup
    bonusTalentGroups trans_x trans_y trans_z trans_o transguid extra_flags
    summonedPetNumber at_login zone death_expire_time taxi_path totalKills
    todayKills yesterdayKills chosenTitle watchedFaction drunk health power1
    power2 power3 power4 power5 power6 power7 power8 power9 power10 ammoId
    latency lootSpecId exploredZones equipmentCache knownTitles actionBars
    deleteInfos_Account deleteInfos_Name deleteDate honor honorLevel
    honorRestState honorRestBonus lastLoginBuild personalTabardEmblemStyle
    personalTabardEmblemColor personalTabardBorderStyle
    personalTabardBorderColor personalTabardBackgroundColor
  )
  case "$projection" in current|prelogin|forced-offline) ;; *) return 1 ;; esac
  for column in "${columns[@]}"; do
    expression="\`${column}\`"
    if [ "$projection" = prelogin ]; then
      case "$column" in
        map) expression=530 ;;
        zone|instance_id) expression=0 ;;
        position_x) expression="CAST(${CREATURE_SPELL_FIXTURE_CHARACTER_START_X} AS FLOAT)" ;;
        position_y) expression="CAST(${CREATURE_SPELL_FIXTURE_CHARACTER_START_Y} AS FLOAT)" ;;
        position_z) expression="CAST(${CREATURE_SPELL_FIXTURE_CHARACTER_START_Z} AS FLOAT)" ;;
        orientation) expression="CAST(${CREATURE_SPELL_FIXTURE_CHARACTER_ORIENTATION} AS FLOAT)" ;;
        health) expression="$CREATURE_SPELL_FIXTURE_CHARACTER_TEMP_HEALTH" ;;
      esac
    elif [ "$projection" = forced-offline ] && [ "$column" = online ]; then
      expression=0
    fi
    joined+="${separator}${expression}"
    separator=,
  done
  printf 'LOWER(SHA2(JSON_COMPACT(JSON_ARRAY(%s)), 256))' "$joined"
}

# These are the 14 characters columns not written by CHAR_UPD_CHARACTER. They
# must remain byte-for-byte equal to the original snapshot before rollback;
# otherwise the 73-field SET could not reproduce the original 87-column hash.
creature_spell_fixture_character_immutable_hash_expression() {
  printf '%s' "LOWER(SHA2(JSON_COMPACT(JSON_ARRAY(
    \`guid\`, \`account\`, \`slot\`, \`createTime\`, \`createMode\`,
    \`ammoId\`, \`deleteInfos_Account\`, \`deleteInfos_Name\`, \`deleteDate\`,
    \`personalTabardEmblemStyle\`, \`personalTabardEmblemColor\`,
    \`personalTabardBorderStyle\`, \`personalTabardBorderColor\`,
    \`personalTabardBackgroundColor\`)), 256))"
}

creature_spell_fixture_character_structural_count() {
  loot_fixture_character_mysql -e "
    SELECT COUNT(*) FROM characters
     WHERE guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID}
       AND account = ${CREATURE_SPELL_FIXTURE_ACCOUNT_ID}
       AND HEX(name) = '${CREATURE_SPELL_FIXTURE_CHARACTER_NAME_HEX}'
       AND race = ${CREATURE_SPELL_FIXTURE_CHARACTER_RACE}
       AND class = ${CREATURE_SPELL_FIXTURE_CHARACTER_CLASS}
       AND level = ${CREATURE_SPELL_FIXTURE_CHARACTER_LEVEL}
       AND online = 0
       AND trans_x = 0 AND trans_y = 0 AND trans_z = 0 AND trans_o = 0
       AND transguid = 0
       AND COALESCE(taxi_path, '') = ''
       AND death_expire_time = 0
       AND NOT EXISTS (
         SELECT 1 FROM corpse
          WHERE corpse.guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID});
  "
}

creature_spell_fixture_snapshot_character() {
  local structural row remainder current_hash prelogin_hash immutable_hash
  local current_expression prelogin_expression immutable_expression
  creature_spell_fixture_validate_character_schema || return 1
  structural="$(creature_spell_fixture_character_structural_count)" || return 1
  [ "$structural" = 1 ] || {
    echo "error: creature spell fixture character ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID} is missing, online, transported/taxied/dead, or has unexpected identity" >&2
    return 1
  }
  current_expression="$(
    creature_spell_fixture_character_row_hash_expression current
  )" || return 1
  prelogin_expression="$(
    creature_spell_fixture_character_row_hash_expression prelogin
  )" || return 1
  immutable_expression="$(
    creature_spell_fixture_character_immutable_hash_expression
  )" || return 1
  row="$(loot_fixture_character_mysql -e "
    SELECT ${current_expression}, ${prelogin_expression}, ${immutable_expression},
           CONCAT('H', HEX(name)), race, class, gender, level, xp, money,
           inventorySlots, bankSlots, restState, playerFlags, playerFlagsEx,
           map, instance_id, dungeonDifficulty, raidDifficulty,
           legacyRaidDifficulty, position_x, position_y, position_z,
           orientation, trans_x, trans_y, trans_z, trans_o, transguid,
           CONCAT('H', HEX(taximask)), cinematic, totaltime, leveltime,
           rest_bonus, logout_time, is_logout_resting, resettalents_cost,
           resettalents_time, numRespecs, activeTalentGroup,
           bonusTalentGroups, extra_flags, summonedPetNumber, at_login, zone,
           death_expire_time,
           IF(taxi_path IS NULL, 'N', CONCAT('H', HEX(taxi_path))),
           totalKills, todayKills, yesterdayKills, chosenTitle,
           watchedFaction, drunk, health, power1, power2, power3, power4,
           power5, power6, power7, power8, power9, power10, latency,
           lootSpecId,
           IF(exploredZones IS NULL, 'N', CONCAT('H', HEX(exploredZones))),
           IF(equipmentCache IS NULL, 'N', CONCAT('H', HEX(equipmentCache))),
           IF(knownTitles IS NULL, 'N', CONCAT('H', HEX(knownTitles))),
           actionBars, online, honor, honorLevel, honorRestState,
           honorRestBonus, lastLoginBuild
      FROM characters
     WHERE guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID}
       AND account = ${CREATURE_SPELL_FIXTURE_ACCOUNT_ID}
       AND online = 0;
  ")" || return 1
  current_hash="${row%%$'\t'*}"
  remainder="${row#*$'\t'}"
  prelogin_hash="${remainder%%$'\t'*}"
  remainder="${remainder#*$'\t'}"
  immutable_hash="${remainder%%$'\t'*}"
  remainder="${remainder#*$'\t'}"
  [[ "$current_hash" =~ ^[0-9a-f]{64}$ \
    && "$prelogin_hash" =~ ^[0-9a-f]{64}$ \
    && "$immutable_hash" =~ ^[0-9a-f]{64}$ ]] \
    && creature_spell_fixture_validate_character_original_tsv "$remainder" || {
    echo "error: creature spell fixture character snapshot has an unsafe shape" >&2
    return 1
  }
  CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV="$remainder"
  CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_SHA256="$(
    creature_spell_fixture_sha256_of_text "$remainder"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_ROW_SHA256="$current_hash"
  CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256="$prelogin_hash"
  CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256="$immutable_hash"
}

creature_spell_fixture_verify_character_state() {
  local expected_row_sha256="$1"
  local structural current_expression current_hash
  [[ "$expected_row_sha256" =~ ^[0-9a-f]{64}$ ]] || return 1
  creature_spell_fixture_validate_character_schema || return 1
  structural="$(creature_spell_fixture_character_structural_count)" || return 1
  [ "$structural" = 1 ] || return 1
  current_expression="$(
    creature_spell_fixture_character_row_hash_expression current
  )" || return 1
  current_hash="$(loot_fixture_character_mysql -e "
    SELECT ${current_expression} FROM characters
     WHERE guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID}
       AND account = ${CREATURE_SPELL_FIXTURE_ACCOUNT_ID}
       AND online = 0;
  ")" || return 1
  [ "$current_hash" = "$expected_row_sha256" ]
}

creature_spell_fixture_verify_character_original() {
  creature_spell_fixture_validate_character_original_tsv \
    "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV" \
    && [ "$(creature_spell_fixture_sha256_of_text \
      "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV")" \
      = "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_SHA256" ] \
    && creature_spell_fixture_verify_character_state \
      "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_ROW_SHA256"
}

creature_spell_fixture_verify_character_prelogin() {
  creature_spell_fixture_verify_character_state \
    "$CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256"
}

creature_spell_fixture_verify_character_immutable() {
  local expression current_hash
  [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256" \
    =~ ^[0-9a-f]{64}$ ]] || return 1
  expression="$(
    creature_spell_fixture_character_immutable_hash_expression
  )" || return 1
  current_hash="$(loot_fixture_character_mysql -e "
    SELECT ${expression} FROM characters
     WHERE guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID}
       AND account = ${CREATURE_SPELL_FIXTURE_ACCOUNT_ID}
       AND online = 0;
  ")" || return 1
  [ "$current_hash" = "$CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256" ]
}

# Snapshot every field that restoration owns only after both worlds are stopped.
# The bounded WHERE proves this is the fixture's completed live-session state;
# the durable hash then becomes the sole CAS authority for restoration.
creature_spell_fixture_snapshot_character_post_login() {
  local row_hash current_expression
  creature_spell_fixture_validate_character_schema || return 1
  current_expression="$(
    creature_spell_fixture_character_row_hash_expression current
  )" || return 1
  row_hash="$(loot_fixture_character_mysql -e "
    SELECT ${current_expression}
      FROM characters
     WHERE guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID}
       AND account = ${CREATURE_SPELL_FIXTURE_ACCOUNT_ID}
       AND HEX(name) = '${CREATURE_SPELL_FIXTURE_CHARACTER_NAME_HEX}'
       AND race = ${CREATURE_SPELL_FIXTURE_CHARACTER_RACE}
       AND class = ${CREATURE_SPELL_FIXTURE_CHARACTER_CLASS}
       AND level = ${CREATURE_SPELL_FIXTURE_CHARACTER_LEVEL}
       AND online = 0
       AND map = 530 AND instance_id = 0
       AND health > 0
       AND SQRT(
         POW(position_x - CAST(-2764.52 AS FLOAT), 2)
         + POW(position_y - CAST(5431.19 AS FLOAT), 2)
         + POW(position_z - CAST(-34.4548 AS FLOAT), 2)) <= 30
       AND trans_x = 0 AND trans_y = 0 AND trans_z = 0 AND trans_o = 0
       AND transguid = 0 AND COALESCE(taxi_path, '') = ''
       AND death_expire_time = 0
       AND NOT EXISTS (
         SELECT 1 FROM corpse
          WHERE corpse.guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID});
  ")" || return 1
  [[ "$row_hash" =~ ^[0-9a-f]{64}$ ]] || {
    echo "error: creature spell post-login character snapshot is absent, ambiguous, or outside the owned live-session envelope" >&2
    return 1
  }
  CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256="$row_hash"
  creature_spell_fixture_verify_character_immutable || {
    echo "error: creature spell live session changed a column outside CHAR_UPD_CHARACTER; refusing an incomplete rollback contract" >&2
    return 1
  }
}

creature_spell_fixture_verify_character_post_login() {
  creature_spell_fixture_verify_character_state \
    "$CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256"
}

creature_spell_fixture_cas_character_to_prelogin() {
  local current_expression updated
  current_expression="$(
    creature_spell_fixture_character_row_hash_expression current
  )" || return 1
  updated="$(loot_fixture_character_mysql -e "
    UPDATE characters
       SET map = 530, zone = 0, instance_id = 0,
           position_x = CAST(${CREATURE_SPELL_FIXTURE_CHARACTER_START_X} AS FLOAT),
           position_y = CAST(${CREATURE_SPELL_FIXTURE_CHARACTER_START_Y} AS FLOAT),
           position_z = CAST(${CREATURE_SPELL_FIXTURE_CHARACTER_START_Z} AS FLOAT),
           orientation = CAST(${CREATURE_SPELL_FIXTURE_CHARACTER_ORIENTATION} AS FLOAT),
           health = ${CREATURE_SPELL_FIXTURE_CHARACTER_TEMP_HEALTH}
     WHERE guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID}
       AND account = ${CREATURE_SPELL_FIXTURE_ACCOUNT_ID}
       AND online = 0
       AND ${current_expression} = '${CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_ROW_SHA256}'
       AND NOT EXISTS (
         SELECT 1 FROM corpse
          WHERE corpse.guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID});
    SELECT ROW_COUNT();
  ")" || return 1
  [ "$updated" = 1 ] || {
    echo "error: creature spell fixture character relocation CAS changed ${updated:-unknown} row(s)" >&2
    return 1
  }
}

creature_spell_fixture_nullable_hex_sql() {
  local token="$1"
  case "$token" in
    N) printf 'NULL' ;;
    H|H[0-9A-F]*) printf "UNHEX('%s')" "${token#H}" ;;
    *) return 1 ;;
  esac
}

creature_spell_fixture_restore_character() {
  local -a f
  local source_row_sha256 current_expression immutable_expression updated
  local taxi_path_sql explored_zones_sql equipment_cache_sql known_titles_sql
  if creature_spell_fixture_verify_character_original; then
    return 0
  fi
  case "$CREATURE_SPELL_FIXTURE_PHASE" in
    armed)
      source_row_sha256="$CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256"
      ;;
    applied)
      source_row_sha256="$CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256"
      ;;
    captured)
      source_row_sha256="$CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256"
      ;;
    *)
      echo "WARNING: creature spell fixture has no exact source state authorized for character restoration" >&2
      return 1
      ;;
  esac
  creature_spell_fixture_verify_character_state "$source_row_sha256" || {
    echo "WARNING: creature spell fixture character differs from the exact journaled CAS source; refusing overwrite" >&2
    return 1
  }
  creature_spell_fixture_verify_character_immutable || {
    echo "WARNING: a non-restored character column differs from the original immutable projection; refusing rollback" >&2
    return 1
  }
  mapfile -t f < <(jq -Rr 'split("\t")[]' \
    <<<"$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV")
  [ "${#f[@]}" = 73 ] || return 1
  taxi_path_sql="$(creature_spell_fixture_nullable_hex_sql "${f[43]}")" \
    || return 1
  explored_zones_sql="$(creature_spell_fixture_nullable_hex_sql "${f[63]}")" \
    || return 1
  equipment_cache_sql="$(creature_spell_fixture_nullable_hex_sql "${f[64]}")" \
    || return 1
  known_titles_sql="$(creature_spell_fixture_nullable_hex_sql "${f[65]}")" \
    || return 1
  current_expression="$(
    creature_spell_fixture_character_row_hash_expression current
  )" || return 1
  immutable_expression="$(
    creature_spell_fixture_character_immutable_hash_expression
  )" || return 1
  updated="$(loot_fixture_character_mysql -e "
    UPDATE characters
       SET name = UNHEX('${f[0]#H}'), race = ${f[1]}, class = ${f[2]},
           gender = ${f[3]}, level = ${f[4]}, xp = ${f[5]}, money = ${f[6]},
           inventorySlots = ${f[7]}, bankSlots = ${f[8]}, restState = ${f[9]},
           playerFlags = ${f[10]}, playerFlagsEx = ${f[11]}, map = ${f[12]},
           instance_id = ${f[13]}, dungeonDifficulty = ${f[14]},
           raidDifficulty = ${f[15]}, legacyRaidDifficulty = ${f[16]},
           position_x = CAST(${f[17]} AS FLOAT),
           position_y = CAST(${f[18]} AS FLOAT),
           position_z = CAST(${f[19]} AS FLOAT),
           orientation = CAST(${f[20]} AS FLOAT),
           trans_x = CAST(${f[21]} AS FLOAT),
           trans_y = CAST(${f[22]} AS FLOAT),
           trans_z = CAST(${f[23]} AS FLOAT),
           trans_o = CAST(${f[24]} AS FLOAT), transguid = ${f[25]},
           taximask = UNHEX('${f[26]#H}'), cinematic = ${f[27]},
           totaltime = ${f[28]}, leveltime = ${f[29]},
           rest_bonus = CAST(${f[30]} AS FLOAT), logout_time = ${f[31]},
           is_logout_resting = ${f[32]}, resettalents_cost = ${f[33]},
           resettalents_time = ${f[34]}, numRespecs = ${f[35]},
           activeTalentGroup = ${f[36]}, bonusTalentGroups = ${f[37]},
           extra_flags = ${f[38]}, summonedPetNumber = ${f[39]},
           at_login = ${f[40]}, zone = ${f[41]}, death_expire_time = ${f[42]},
           taxi_path = ${taxi_path_sql}, totalKills = ${f[44]},
           todayKills = ${f[45]}, yesterdayKills = ${f[46]},
           chosenTitle = ${f[47]}, watchedFaction = ${f[48]}, drunk = ${f[49]},
           health = ${f[50]}, power1 = ${f[51]}, power2 = ${f[52]},
           power3 = ${f[53]}, power4 = ${f[54]}, power5 = ${f[55]},
           power6 = ${f[56]}, power7 = ${f[57]}, power8 = ${f[58]},
           power9 = ${f[59]}, power10 = ${f[60]}, latency = ${f[61]},
           lootSpecId = ${f[62]}, exploredZones = ${explored_zones_sql},
           equipmentCache = ${equipment_cache_sql}, knownTitles = ${known_titles_sql},
           actionBars = ${f[66]}, online = ${f[67]}, honor = ${f[68]},
           honorLevel = ${f[69]}, honorRestState = ${f[70]},
           honorRestBonus = CAST(${f[71]} AS FLOAT), lastLoginBuild = ${f[72]}
     WHERE guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID}
       AND account = ${CREATURE_SPELL_FIXTURE_ACCOUNT_ID}
       AND online = 0
       AND ${current_expression} = '${source_row_sha256}'
       AND ${immutable_expression} = '${CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256}'
       AND NOT EXISTS (
         SELECT 1 FROM corpse
          WHERE corpse.guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID});
    SELECT ROW_COUNT();
  ")" || return 1
  [ "$updated" = 1 ] || {
    echo "WARNING: creature spell fixture character restoration CAS changed ${updated:-unknown} row(s)" >&2
    return 1
  }
  creature_spell_fixture_verify_character_original
}

# Hash every fixture-relevant row except the two fields changed atomically by
# this guard: creature_template.AIName and creature_template_difficulty.StaticFlags1.
# The other 24 difficulty fields plus the full spawn/spell/SmartAI rows make
# the digest useful across both capture sides, while the explicit state checks
# below produce actionable errors.
creature_spell_fixture_static_snapshot_sha256() {
  local world_snapshot character_snapshot snapshot
  creature_spell_fixture_validate_character_original_tsv \
    "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV" || return 1
  [ "$(creature_spell_fixture_sha256_of_text \
      "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV")" \
    = "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_SHA256" ] || return 1
  [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_ROW_SHA256" \
    =~ ^[0-9a-f]{64}$ ]] \
    && [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256" \
      =~ ^[0-9a-f]{64}$ ]] \
    && [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256" \
      =~ ^[0-9a-f]{64}$ ]] \
    && creature_spell_fixture_validate_character_schema || return 1
  world_snapshot="$(loot_fixture_world_mysql -e "
    SELECT entry, HEX(name), HEX(ScriptName), VerifiedBuild
      FROM creature_template WHERE entry = ${CREATURE_SPELL_FIXTURE_ENTRY};
    SELECT Entry, DifficultyID, MinLevel, MaxLevel,
           HealthScalingExpansion, HealthModifier, ManaModifier,
           ArmorModifier, DamageModifier, CreatureDifficultyID,
           TypeFlags, TypeFlags2, LootID, PickPocketLootID, SkinLootID,
           GoldMin, GoldMax, StaticFlags2, StaticFlags3, StaticFlags4,
           StaticFlags5, StaticFlags6, StaticFlags7, StaticFlags8
      FROM creature_template_difficulty
      WHERE Entry = ${CREATURE_SPELL_FIXTURE_ENTRY}
      ORDER BY Entry, DifficultyID;
    SELECT * FROM creature
      WHERE guid = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}
      ORDER BY guid;
    SELECT * FROM creature_template_spell
      WHERE CreatureID = ${CREATURE_SPELL_FIXTURE_ENTRY}
      ORDER BY \`Index\`;
    SELECT * FROM smart_scripts
      WHERE entryorguid = ${CREATURE_SPELL_FIXTURE_ENTRY}
        AND source_type = 0
      ORDER BY id, link;
    SELECT
      (SELECT COUNT(*) FROM pool_members
        WHERE type = 0 AND spawnId = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}),
      (SELECT COUNT(*) FROM game_event_creature
        WHERE guid = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}),
      (SELECT COUNT(*) FROM linked_respawn
        WHERE guid = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}
           OR linkedGuid = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}),
      (SELECT COUNT(*) FROM creature_addon
        WHERE guid = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}),
      (SELECT COUNT(*) FROM spawn_group
        WHERE spawnType = 0 AND spawnId = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID});
  ")" || return 1
  character_snapshot="$(loot_fixture_character_mysql -e "
    SELECT COUNT(*) FROM respawn
      WHERE type = 0 AND spawnId = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID};
    SELECT * FROM respawn
      WHERE type = 0 AND spawnId = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}
      ORDER BY instanceId, mapId;
  ")" || return 1
  snapshot="${world_snapshot}"$'\n--characters--\n'"${character_snapshot}"$'\n--fixture-character-schema--\n'"${CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_COLUMN_COUNT}"$'\t'"${CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_METADATA_BYTES}"$'\t'"${CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_SHA256}"$'\n--fixture-character-original-row--\n'"${CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_ROW_SHA256}"$'\n--fixture-character-prelogin-row--\n'"${CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256}"$'\n--fixture-character-immutable--\n'"${CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256}"$'\n--fixture-character-restore-projection--\n'"${CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV}"
  creature_spell_fixture_sha256_of_text "$snapshot"
}

# Spell 8326 applies C++ ghost server-side visibility. A stale persisted aura
# makes the living Cabal fixture invisible even when the core character row,
# corpse table, and creature respawn state are all clean. Keep this check
# intentionally narrow: reject the aura and any orphaned associated effects
# for the one pinned fixture character without claiming ownership of other
# character auras.
creature_spell_fixture_verify_no_persisted_ghost_state() {
  local counts
  counts="$(loot_fixture_character_mysql -e "
    SELECT
      (SELECT COUNT(*) FROM character_aura
        WHERE guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID}
          AND spell = ${CREATURE_SPELL_FIXTURE_GHOST_SPELL_ID}),
      (SELECT COUNT(*) FROM character_aura_effect
        WHERE guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID}
          AND spell = ${CREATURE_SPELL_FIXTURE_GHOST_SPELL_ID});
  ")" || return 1
  [ "$counts" = $'0\t0' ] || {
    echo "error: creature spell fixture character ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID} has persisted ghost spell ${CREATURE_SPELL_FIXTURE_GHOST_SPELL_ID} aura/effect state (${counts:-query-failed})" >&2
    return 1
  }
}
