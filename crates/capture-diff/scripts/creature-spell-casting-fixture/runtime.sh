creature_spell_fixture_verify_exact_state() {
  local expected_ai="$1"
  local expected_static_flags_1 counts respawns expected
  case "$expected_ai" in
    "$CREATURE_SPELL_FIXTURE_ORIGINAL_AI")
      expected_static_flags_1="$CREATURE_SPELL_FIXTURE_ORIGINAL_STATIC_FLAGS_1"
      ;;
    "$CREATURE_SPELL_FIXTURE_TEMP_AI")
      expected_static_flags_1="$CREATURE_SPELL_FIXTURE_TEMP_STATIC_FLAGS_1"
      ;;
    *) return 1 ;;
  esac

  counts="$(loot_fixture_world_mysql -e "
    SELECT
      (SELECT COUNT(*) FROM creature_template
        WHERE entry = ${CREATURE_SPELL_FIXTURE_ENTRY}),
      (SELECT COUNT(*) FROM creature_template
        WHERE entry = ${CREATURE_SPELL_FIXTURE_ENTRY}
          AND name = 'Cabal Interrogator'
          AND AIName = '${expected_ai}'
          AND ScriptName = ''
          AND VerifiedBuild = 52237),
      (SELECT COUNT(*) FROM creature_template_difficulty
        WHERE Entry = ${CREATURE_SPELL_FIXTURE_ENTRY}),
      (SELECT COUNT(*) FROM creature_template_difficulty
        WHERE Entry = ${CREATURE_SPELL_FIXTURE_ENTRY}
          AND DifficultyID = ${CREATURE_SPELL_FIXTURE_DIFFICULTY_ID}
          AND MinLevel = 64 AND MaxLevel = 65
          AND HealthScalingExpansion = 0
          AND HealthModifier = 1 AND ManaModifier = 1
          AND ArmorModifier = 1 AND DamageModifier = 1
          AND CreatureDifficultyID = 18203
          AND TypeFlags = 0 AND TypeFlags2 = 0
          AND LootID = 22378 AND PickPocketLootID = 22378
          AND SkinLootID = 0 AND GoldMin = 153 AND GoldMax = 205
          AND StaticFlags1 = ${expected_static_flags_1}
          AND StaticFlags2 = 0 AND StaticFlags3 = 0
          AND StaticFlags4 = 0 AND StaticFlags5 = 0
          AND StaticFlags6 = 0 AND StaticFlags7 = 0
          AND StaticFlags8 = 0),
      (SELECT COUNT(*) FROM creature
        WHERE id = ${CREATURE_SPELL_FIXTURE_ENTRY}),
      (SELECT COUNT(*) FROM creature
        WHERE guid = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}
          AND id = ${CREATURE_SPELL_FIXTURE_ENTRY}
          AND map = 530
          AND zoneId = 0
          AND areaId = 0
          AND spawnDifficulties = '0'
          AND phaseUseFlags = 0
          AND COALESCE(PhaseId, 0) = 0
          AND COALESCE(PhaseGroup, 0) = 0
          AND terrainSwapMap = -1
          AND position_x = CAST(-2764.52 AS FLOAT)
          AND position_y = CAST(5431.19 AS FLOAT)
          AND position_z = CAST(-34.4548 AS FLOAT)
          AND orientation = CAST(3.735 AS FLOAT)
          AND spawntimesecs = 300
          AND wander_distance = 0
          AND MovementType = 0
          AND VerifiedBuild = 0),
      (SELECT COUNT(*) FROM creature_template_spell
        WHERE CreatureID = ${CREATURE_SPELL_FIXTURE_ENTRY}),
      (SELECT COUNT(*) FROM creature_template_spell
        WHERE CreatureID = ${CREATURE_SPELL_FIXTURE_ENTRY}
          AND \`Index\` = ${CREATURE_SPELL_FIXTURE_SPELL_SLOT}
          AND Spell = ${CREATURE_SPELL_FIXTURE_SPELL_ID}
          AND VerifiedBuild = 41031),
      (SELECT COUNT(*) FROM smart_scripts
        WHERE entryorguid = ${CREATURE_SPELL_FIXTURE_ENTRY}
          AND source_type = 0),
      (SELECT COUNT(*) FROM smart_scripts
        WHERE entryorguid = ${CREATURE_SPELL_FIXTURE_ENTRY}
          AND source_type = 0
          AND id = 1 AND link = 0
          AND event_type = 0
          AND event_param1 = 2000 AND event_param2 = 5000
          AND event_param3 = 6000 AND event_param4 = 9000
          AND action_type = 11 AND action_param1 = ${CREATURE_SPELL_FIXTURE_SPELL_ID}
          AND target_type = 2),
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
  expected=$'1\t1\t1\t1\t1\t1\t1\t1\t3\t1\t0\t0\t0\t0\t0'
  [ "$counts" = "$expected" ] || {
    echo "error: creature spell fixture DB topology/state differs from the pinned Cabal 22378/78686/15691 contract (${counts:-query-failed})" >&2
    return 1
  }
  respawns="$(loot_fixture_character_mysql -e "
    SELECT COUNT(*) FROM respawn
      WHERE type = 0 AND spawnId = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}
  ")" || return 1
  [ "$respawns" = 0 ] || {
    echo "error: creature spell fixture spawn ${CREATURE_SPELL_FIXTURE_SPAWN_GUID} has ${respawns:-unknown} persisted respawn row(s)" >&2
    return 1
  }
}

creature_spell_fixture_pm2_inactive() {
  local process_name="$1"
  pm2 jlist | jq -e --arg name "$process_name" '
    [.[] | select(.name == $name)] as $entries
    | ($entries | length) == 0
      or (($entries | length) == 1
        and $entries[0].pm2_env.status == "stopped"
        and (($entries[0].pid // 0) == 0))
  ' >/dev/null
}

creature_spell_fixture_require_worlds_stopped() {
  [ -n "$CREATURE_SPELL_FIXTURE_PM2_RUST_WORLD" ] \
    && [ -n "$CREATURE_SPELL_FIXTURE_PM2_CPP_WORLD" ] \
    && creature_spell_fixture_pm2_inactive \
      "$CREATURE_SPELL_FIXTURE_PM2_RUST_WORLD" \
    && creature_spell_fixture_pm2_inactive \
      "$CREATURE_SPELL_FIXTURE_PM2_CPP_WORLD" \
    && capture_world_ports_absent || {
      echo "error: both world PM2 entries must be stopped/absent and both listeners absent before creature spell fixture DB access" >&2
      return 1
    }
}

creature_spell_fixture_require_safe_db_window() {
  creature_spell_fixture_require_worlds_stopped || return 1
  loot_fixture_wait_until_all_characters_offline || {
    echo "error: all characters must be offline before creature spell fixture DB access" >&2
    return 1
  }
}

# Stock C++ retains WorldSession for roughly 60 seconds after an abrupt socket
# close. During that interval its persisted characters.online marker can remain
# 1 even though the accredited wrapper has already stopped both worlds and
# proved both listeners absent. Repair only that marker, and only when changing
# online to 0 would reproduce the exact journaled pre-login row. All gameplay
# state remains outside this narrow CAS.
creature_spell_fixture_reconcile_owned_online_marker() {
  local online_shape forced_offline_expression immutable_expression updated
  creature_spell_fixture_require_worlds_stopped || return 1
  online_shape="$(loot_fixture_character_mysql -e "
    SELECT COUNT(*),
           COALESCE(SUM(guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID}
             AND account = ${CREATURE_SPELL_FIXTURE_ACCOUNT_ID}
             AND online = 1), 0)
      FROM characters
     WHERE online <> 0;
  ")" || return 1
  case "$online_shape" in
    $'0\t0') return 0 ;;
    $'1\t1') ;;
    *)
      echo "error: post-capture online marker shape is not the one owned guid ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID}/account ${CREATURE_SPELL_FIXTURE_ACCOUNT_ID} row (${online_shape:-query-failed})" >&2
      return 1
      ;;
  esac
  [ "$CREATURE_SPELL_FIXTURE_PHASE" = applied ] || {
    echo "error: owned post-capture online marker may be reconciled only from an applied journal" >&2
    return 1
  }
  forced_offline_expression="$(
    creature_spell_fixture_character_row_hash_expression forced-offline
  )" || return 1
  immutable_expression="$(
    creature_spell_fixture_character_immutable_hash_expression
  )" || return 1
  updated="$(loot_fixture_character_mysql -e "
    UPDATE characters AS fixture_character
    JOIN (
      SELECT COUNT(*) AS online_count,
             COALESCE(SUM(online_guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID}
               AND online_account = ${CREATURE_SPELL_FIXTURE_ACCOUNT_ID}
               AND online_value = 1), 0) AS owned_online_count
        FROM (
          SELECT guid AS online_guid, account AS online_account,
                 online AS online_value
            FROM characters
           WHERE online <> 0
        ) AS materialized_online_rows
    ) AS online_guard
      ON online_guard.online_count = 1
     AND online_guard.owned_online_count = 1
       SET fixture_character.online = 0
     WHERE fixture_character.guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID}
       AND fixture_character.account = ${CREATURE_SPELL_FIXTURE_ACCOUNT_ID}
       AND fixture_character.online = 1
       AND ${forced_offline_expression} = '${CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256}'
       AND ${immutable_expression} = '${CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256}'
       AND NOT EXISTS (
         SELECT 1 FROM corpse
          WHERE corpse.guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID})
       AND NOT EXISTS (
         SELECT 1 FROM character_aura
          WHERE character_aura.guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID}
            AND character_aura.spell = ${CREATURE_SPELL_FIXTURE_GHOST_SPELL_ID})
       AND NOT EXISTS (
         SELECT 1 FROM character_aura_effect
          WHERE character_aura_effect.guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID}
            AND character_aura_effect.spell = ${CREATURE_SPELL_FIXTURE_GHOST_SPELL_ID});
    SELECT ROW_COUNT();
  ")" || return 1
  [ "$updated" = 1 ] || {
    echo "error: owned post-capture online marker CAS changed ${updated:-unknown} row(s); full-row/immutable/death/ghost proof failed" >&2
    return 1
  }
}

creature_spell_fixture_cas_runtime_state() {
  local from="$1"
  local to="$2"
  local from_static_flags_1 to_static_flags_1 updated
  case "${from}:${to}" in
    "${CREATURE_SPELL_FIXTURE_ORIGINAL_AI}:${CREATURE_SPELL_FIXTURE_TEMP_AI}")
      from_static_flags_1="$CREATURE_SPELL_FIXTURE_ORIGINAL_STATIC_FLAGS_1"
      to_static_flags_1="$CREATURE_SPELL_FIXTURE_TEMP_STATIC_FLAGS_1"
      ;;
    "${CREATURE_SPELL_FIXTURE_TEMP_AI}:${CREATURE_SPELL_FIXTURE_ORIGINAL_AI}")
      from_static_flags_1="$CREATURE_SPELL_FIXTURE_TEMP_STATIC_FLAGS_1"
      to_static_flags_1="$CREATURE_SPELL_FIXTURE_ORIGINAL_STATIC_FLAGS_1"
      ;;
    *) return 1 ;;
  esac
  updated="$(loot_fixture_world_mysql -e "
    UPDATE creature_template AS ct
    JOIN creature_template_difficulty AS ctd
      ON ctd.Entry = ct.entry
     AND ctd.DifficultyID = ${CREATURE_SPELL_FIXTURE_DIFFICULTY_ID}
       SET ct.AIName = '${to}',
           ctd.StaticFlags1 = ${to_static_flags_1}
     WHERE ct.entry = ${CREATURE_SPELL_FIXTURE_ENTRY}
       AND ct.name = 'Cabal Interrogator'
       AND ct.AIName = '${from}'
       AND ct.ScriptName = ''
       AND ct.VerifiedBuild = 52237
       AND ctd.MinLevel = 64 AND ctd.MaxLevel = 65
       AND ctd.HealthScalingExpansion = 0
       AND ctd.HealthModifier = 1 AND ctd.ManaModifier = 1
       AND ctd.ArmorModifier = 1 AND ctd.DamageModifier = 1
       AND ctd.CreatureDifficultyID = 18203
       AND ctd.TypeFlags = 0 AND ctd.TypeFlags2 = 0
       AND ctd.LootID = 22378 AND ctd.PickPocketLootID = 22378
       AND ctd.SkinLootID = 0 AND ctd.GoldMin = 153 AND ctd.GoldMax = 205
       AND ctd.StaticFlags1 = ${from_static_flags_1}
       AND ctd.StaticFlags2 = 0 AND ctd.StaticFlags3 = 0
       AND ctd.StaticFlags4 = 0 AND ctd.StaticFlags5 = 0
       AND ctd.StaticFlags6 = 0 AND ctd.StaticFlags7 = 0
       AND ctd.StaticFlags8 = 0;
    SELECT ROW_COUNT();
  ")" || return 1
  # Both tables are InnoDB and this multi-table UPDATE changes exactly one row
  # in each table. Any mixed or drifted pair changes zero rows and fails closed.
  [ "$updated" = 2 ] || {
    echo "error: creature spell fixture runtime CAS ${from}+${from_static_flags_1}->${to}+${to_static_flags_1} changed ${updated:-unknown} row(s), expected 2" >&2
    return 1
  }
}
