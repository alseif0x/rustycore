#!/usr/bin/env bash
# Shared, source-only guard for the deterministic loot capture fixtures.
#
# Callers own service lifecycle and define these globals before using it:
#   LOOT_FIXTURE_DB_CONF
#   WOW_BOT_FIXTURE_JOURNAL
#   LOOT_FIXTURE_CLEANUP_MARKER
#   LOOT_FIXTURE_ENTRY
#   LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER
#   LOOT_FIXTURE_TEMP_HEALTH_MODIFIER
#   LOOT_FIXTURE_SNAPSHOT_READY
#
# The shared-chest guard (apply/restore_shared_chest_fixture_guard) also
# needs, before use:
#   LOOT_FIXTURE_CHEST_TEMPLATE_ENTRY  LOOT_FIXTURE_CHEST_LOOT_ENTRY
#   LOOT_FIXTURE_CHEST_ITEM            LOOT_FIXTURE_CHEST_GUID
#   LOOT_FIXTURE_CHEST_ADDON_FACTION
#   LOOT_FIXTURE_CHEST_ADDON_RESTORE_READY=0
#   LOOT_FIXTURE_CHEST_SPAWN_DELETE_READY=0
#   LOOT_FIXTURE_CHEST_RESPAWN_DELETE_READY=0
# Both the PM2 capture wrapper (capture-rust.sh) and the systemd runtime
# harness (tools/qa-runtime.sh loot-race, #373) source it; the caller keeps
# the world stopped across apply and restore.

validate_fresh_loot_fixture_journal() {
  local journal_parent canonical_parent owner mode
  [ -n "$WOW_BOT_FIXTURE_JOURNAL" ] || {
    echo "error: guarded loot capture requires WOW_BOT_FIXTURE_JOURNAL" >&2
    return 1
  }
  [[ "$WOW_BOT_FIXTURE_JOURNAL" = /* \
    && "$WOW_BOT_FIXTURE_JOURNAL" != *$'\n'* ]] || {
    echo "error: WOW_BOT_FIXTURE_JOURNAL must be an absolute single-line path" >&2
    return 1
  }
  journal_parent="$(dirname -- "$WOW_BOT_FIXTURE_JOURNAL")"
  [ -d "$journal_parent" ] && [ ! -L "$journal_parent" ] || {
    echo "error: WOW_BOT_FIXTURE_JOURNAL parent directory does not exist" >&2
    return 1
  }
  canonical_parent="$(realpath -e -- "$journal_parent" 2>/dev/null)" \
    || return 1
  owner="$(stat -c '%u' -- "$journal_parent" 2>/dev/null)" || return 1
  mode="$(stat -c '%a' -- "$journal_parent" 2>/dev/null)" || return 1
  [ "$canonical_parent" = "$journal_parent" ] \
    && [ "$owner" = "$(id -u)" ] && [ "$mode" = 700 ] || {
    echo "error: WOW_BOT_FIXTURE_JOURNAL parent must be canonical, owned by this uid, mode 0700, and non-symlink" >&2
    return 1
  }
  LOOT_FIXTURE_CLEANUP_MARKER="${WOW_BOT_FIXTURE_JOURNAL}.cleanup-complete"
  [ ! -e "$WOW_BOT_FIXTURE_JOURNAL" ] \
    && [ ! -L "$WOW_BOT_FIXTURE_JOURNAL" ] \
    && [ ! -e "$LOOT_FIXTURE_CLEANUP_MARKER" ] \
    && [ ! -L "$LOOT_FIXTURE_CLEANUP_MARKER" ] || {
      echo "error: fixture journal/cleanup marker already exists; recover or remove it explicitly before capture" >&2
      return 1
    }
}

read_database_info() {
  local key="$1"
  local value
  value="$({
    awk -F '"' -v key="$key" '
      $0 ~ "^[[:space:]]*" key "[[:space:]]*=" { print $2; exit }
    ' "$LOOT_FIXTURE_DB_CONF"
  })"
  [ -n "$value" ] || return 1
  printf '%s\n' "$value"
}

_load_loot_fixture_database_credentials_untraced() {
  local world_info character_info extra
  [ -f "$LOOT_FIXTURE_DB_CONF" ] || {
    echo "error: loot fixture DB conf not found: ${LOOT_FIXTURE_DB_CONF}" >&2
    return 1
  }
  world_info="$(read_database_info WorldDatabaseInfo)" || {
    echo "error: WorldDatabaseInfo not found in ${LOOT_FIXTURE_DB_CONF}" >&2
    return 1
  }
  character_info="$(read_database_info CharacterDatabaseInfo)" || {
    echo "error: CharacterDatabaseInfo not found in ${LOOT_FIXTURE_DB_CONF}" >&2
    return 1
  }
  IFS=';' read -r \
    LOOT_FIXTURE_WORLD_HOST \
    LOOT_FIXTURE_WORLD_PORT \
    LOOT_FIXTURE_WORLD_USER \
    LOOT_FIXTURE_WORLD_PASSWORD \
    LOOT_FIXTURE_WORLD_DATABASE \
    extra <<<"$world_info"
  [ -z "${extra:-}" ] || {
    echo "error: WorldDatabaseInfo has unsupported extra fields" >&2
    return 1
  }
  IFS=';' read -r \
    LOOT_FIXTURE_CHARACTER_HOST \
    LOOT_FIXTURE_CHARACTER_PORT \
    LOOT_FIXTURE_CHARACTER_USER \
    LOOT_FIXTURE_CHARACTER_PASSWORD \
    LOOT_FIXTURE_CHARACTER_DATABASE \
    extra <<<"$character_info"
  [ -z "${extra:-}" ] || {
    echo "error: CharacterDatabaseInfo has unsupported extra fields" >&2
    return 1
  }
  [ -n "$LOOT_FIXTURE_WORLD_HOST" ] \
    && [[ "$LOOT_FIXTURE_WORLD_PORT" =~ ^[1-9][0-9]*$ ]] \
    && [ -n "$LOOT_FIXTURE_WORLD_USER" ] \
    && [ -n "$LOOT_FIXTURE_WORLD_DATABASE" ] \
    && [ -n "$LOOT_FIXTURE_CHARACTER_HOST" ] \
    && [[ "$LOOT_FIXTURE_CHARACTER_PORT" =~ ^[1-9][0-9]*$ ]] \
    && [ -n "$LOOT_FIXTURE_CHARACTER_USER" ] \
    && [ -n "$LOOT_FIXTURE_CHARACTER_DATABASE" ] || {
      echo "error: loot fixture DatabaseInfo is incomplete" >&2
      return 1
    }
}

load_loot_fixture_database_credentials() {
  local restore_xtrace=0 status
  if [[ "$-" == *x* ]]; then
    restore_xtrace=1
    set +x
  fi
  if _load_loot_fixture_database_credentials_untraced; then
    status=0
  else
    status=$?
  fi
  if [ "$restore_xtrace" -eq 1 ]; then
    set -x
  fi
  return "$status"
}

loot_fixture_world_mysql() {
  local restore_xtrace=0 status
  if [[ "$-" == *x* ]]; then
    restore_xtrace=1
    set +x
  fi
  if MYSQL_PWD="$LOOT_FIXTURE_WORLD_PASSWORD" mysql \
      --protocol=TCP \
      -h "$LOOT_FIXTURE_WORLD_HOST" \
      -P "$LOOT_FIXTURE_WORLD_PORT" \
      -u "$LOOT_FIXTURE_WORLD_USER" \
      --batch --raw --skip-column-names \
      "$LOOT_FIXTURE_WORLD_DATABASE" "$@"; then
    status=0
  else
    status=$?
  fi
  if [ "$restore_xtrace" -eq 1 ]; then
    set -x
  fi
  return "$status"
}

loot_fixture_character_mysql() {
  local restore_xtrace=0 status
  if [[ "$-" == *x* ]]; then
    restore_xtrace=1
    set +x
  fi
  if MYSQL_PWD="$LOOT_FIXTURE_CHARACTER_PASSWORD" mysql \
      --protocol=TCP \
      -h "$LOOT_FIXTURE_CHARACTER_HOST" \
      -P "$LOOT_FIXTURE_CHARACTER_PORT" \
      -u "$LOOT_FIXTURE_CHARACTER_USER" \
      --batch --raw --skip-column-names \
      "$LOOT_FIXTURE_CHARACTER_DATABASE" "$@"; then
    status=0
  else
    status=$?
  fi
  if [ "$restore_xtrace" -eq 1 ]; then
    set -x
  fi
  return "$status"
}

loot_fixture_wait_until_all_characters_offline() {
  local attempt online=""
  for ((attempt = 0; attempt < 300; attempt++)); do
    online="$(loot_fixture_character_mysql \
      -e 'SELECT COUNT(*) FROM characters WHERE online <> 0')" || return 1
    [ "$online" = "0" ] && return 0
    sleep 0.1
  done
  echo "error: refusing loot fixture mutation while ${online:-unknown} character(s) remain online" >&2
  return 1
}

apply_creature_health_fixture_guard() {
  local matching updated
  matching="$(loot_fixture_world_mysql -e \
    "SELECT COUNT(*) FROM creature_template_difficulty
       WHERE Entry = ${LOOT_FIXTURE_ENTRY}
         AND DifficultyID = 0
         AND ABS(HealthModifier - ${LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER}) < 0.0000001")" \
    || return 1
  [ "$matching" = "1" ] || {
    echo "error: loot fixture ${LOOT_FIXTURE_ENTRY} is missing or its original HealthModifier is not ${LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER}" >&2
    return 1
  }

  # Arm cleanup before the CAS. If the caller exits after the UPDATE but before
  # verification, restoration still inspects the exact temporary value.
  LOOT_FIXTURE_SNAPSHOT_READY=1
  updated="$(loot_fixture_world_mysql -e \
    "UPDATE creature_template_difficulty
        SET HealthModifier = ${LOOT_FIXTURE_TEMP_HEALTH_MODIFIER}
      WHERE Entry = ${LOOT_FIXTURE_ENTRY}
        AND DifficultyID = 0
        AND ABS(HealthModifier - ${LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER}) < 0.0000001;
     SELECT ROW_COUNT();")" || return 1
  [ "$updated" = "1" ] || {
    echo "error: loot fixture ${LOOT_FIXTURE_ENTRY} changed during activation (ROW_COUNT=${updated:-unknown})" >&2
    return 1
  }
  matching="$(loot_fixture_world_mysql -e \
    "SELECT COUNT(*) FROM creature_template_difficulty
       WHERE Entry = ${LOOT_FIXTURE_ENTRY}
         AND DifficultyID = 0
         AND ABS(HealthModifier - ${LOOT_FIXTURE_TEMP_HEALTH_MODIFIER}) < 0.0000001")" \
    || return 1
  [ "$matching" = "1" ] || {
    echo "error: failed to activate loot fixture ${LOOT_FIXTURE_ENTRY}" >&2
    return 1
  }
  echo "loot fixture: entry ${LOOT_FIXTURE_ENTRY} HealthModifier ${LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER} -> ${LOOT_FIXTURE_TEMP_HEALTH_MODIFIER} (restore armed)"
}

restore_creature_health_fixture_guard() {
  [ "$LOOT_FIXTURE_SNAPSHOT_READY" -eq 1 ] || return 0

  local original_matches temporary_matches updated
  original_matches="$(loot_fixture_world_mysql -e \
    "SELECT COUNT(*) FROM creature_template_difficulty
       WHERE Entry = ${LOOT_FIXTURE_ENTRY}
         AND DifficultyID = 0
         AND ABS(HealthModifier - ${LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER}) < 0.0000001")" \
    || return 1
  if [ "$original_matches" = "1" ]; then
    LOOT_FIXTURE_SNAPSHOT_READY=0
    return 0
  fi
  temporary_matches="$(loot_fixture_world_mysql -e \
    "SELECT COUNT(*) FROM creature_template_difficulty
       WHERE Entry = ${LOOT_FIXTURE_ENTRY}
         AND DifficultyID = 0
         AND ABS(HealthModifier - ${LOOT_FIXTURE_TEMP_HEALTH_MODIFIER}) < 0.0000001")" \
    || return 1
  [ "$temporary_matches" = "1" ] || {
    echo "WARNING: loot fixture ${LOOT_FIXTURE_ENTRY} changed externally; refusing to overwrite it" >&2
    return 1
  }
  updated="$(loot_fixture_world_mysql -e \
    "UPDATE creature_template_difficulty
        SET HealthModifier = ${LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER}
      WHERE Entry = ${LOOT_FIXTURE_ENTRY}
        AND DifficultyID = 0
        AND ABS(HealthModifier - ${LOOT_FIXTURE_TEMP_HEALTH_MODIFIER}) < 0.0000001;
     SELECT ROW_COUNT();")" || return 1
  [ "$updated" = "1" ] || {
    echo "WARNING: loot fixture ${LOOT_FIXTURE_ENTRY} changed during restoration (ROW_COUNT=${updated:-unknown})" >&2
    return 1
  }
  original_matches="$(loot_fixture_world_mysql -e \
    "SELECT COUNT(*) FROM creature_template_difficulty
       WHERE Entry = ${LOOT_FIXTURE_ENTRY}
         AND DifficultyID = 0
         AND ABS(HealthModifier - ${LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER}) < 0.0000001")" \
    || return 1
  [ "$original_matches" = "1" ] || {
    echo "WARNING: failed to verify restoration of loot fixture ${LOOT_FIXTURE_ENTRY}" >&2
    return 1
  }
  LOOT_FIXTURE_SNAPSHOT_READY=0
  echo "loot fixture: restored entry ${LOOT_FIXTURE_ENTRY} HealthModifier ${LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER}"
}

loot_fixture_bot_cleanup_complete() {
  [ "${LOOT_FIXTURE_GUARD_ENABLED:-0}" = "1" ] || return 0

  if [ -e "$WOW_BOT_FIXTURE_JOURNAL" ] \
      || [ -L "$WOW_BOT_FIXTURE_JOURNAL" ]; then
    echo "WARNING: bot fixture recovery journal is still pending at ${WOW_BOT_FIXTURE_JOURNAL}; refusing to start the normal PM2 world" >&2
    return 1
  fi
  if [ ! -f "$LOOT_FIXTURE_CLEANUP_MARKER" ] \
      || [ -L "$LOOT_FIXTURE_CLEANUP_MARKER" ]; then
    echo "WARNING: bot cleanup-complete marker is missing or unsafe at ${LOOT_FIXTURE_CLEANUP_MARKER}; refusing to start the normal PM2 world" >&2
    return 1
  fi
  if [ "$(stat -c '%a' -- "$LOOT_FIXTURE_CLEANUP_MARKER" 2>/dev/null)" != "600" ] \
      || ! jq -e '
        .version == 1
        and (.cleanup_pid | type == "number" and . > 0)
        and (.journal_sha256 | type == "string" and test("^[0-9a-f]{64}$"))
      ' "$LOOT_FIXTURE_CLEANUP_MARKER" >/dev/null 2>&1; then
    echo "WARNING: bot cleanup-complete marker failed its mode-0600/schema contract; refusing to start the normal PM2 world" >&2
    return 1
  fi
}

# Before the flow is exposed to the client/bot, absence of both recovery files
# proves that no bot-side mutation needs recovery. Once exposed, require the
# durable cleanup marker contract above. Ambiguous state always fails closed.
loot_fixture_bot_cleanup_safe_for_capture_state() {
  local capture_bot_ready="${1:-}"

  [ "${LOOT_FIXTURE_GUARD_ENABLED:-0}" = "1" ] || return 0

  [ -n "${WOW_BOT_FIXTURE_JOURNAL:-}" ] \
    && [ -n "${LOOT_FIXTURE_CLEANUP_MARKER:-}" ] \
    && [ "$LOOT_FIXTURE_CLEANUP_MARKER" \
      = "${WOW_BOT_FIXTURE_JOURNAL}.cleanup-complete" ] || {
    echo "WARNING: bot fixture cleanup paths are missing or inconsistent; refusing to start the normal PM2 world" >&2
    return 1
  }

  case "$capture_bot_ready" in
    0)
      if [ -e "$WOW_BOT_FIXTURE_JOURNAL" ] \
          || [ -L "$WOW_BOT_FIXTURE_JOURNAL" ] \
          || [ -e "$LOOT_FIXTURE_CLEANUP_MARKER" ] \
          || [ -L "$LOOT_FIXTURE_CLEANUP_MARKER" ]; then
        echo "WARNING: bot fixture recovery state appeared before capture readiness; refusing to start the normal PM2 world" >&2
        return 1
      fi
      ;;
    1)
      loot_fixture_bot_cleanup_complete
      ;;
    *)
      echo "WARNING: invalid capture bot-readiness state; refusing to start the normal PM2 world" >&2
      return 1
      ;;
  esac
}

shared_chest_spawn_exact_count() {
  loot_fixture_world_mysql -e \
    "SELECT COUNT(*) FROM gameobject
      WHERE guid = ${LOOT_FIXTURE_CHEST_GUID}
        AND id = ${LOOT_FIXTURE_CHEST_TEMPLATE_ENTRY}
        AND map = 0
        AND zoneId = 0
        AND areaId = 0
        AND spawnDifficulties = '0'
        AND phaseUseFlags = 0
        AND PhaseId = 0
        AND PhaseGroup = 0
        AND terrainSwapMap = -1
        AND position_x = CAST(-8946.95 AS FLOAT)
        AND position_y = CAST(-132.493 AS FLOAT)
        AND position_z = CAST(83.5312 AS FLOAT)
        AND orientation = 0
        AND rotation0 = 0
        AND rotation1 = 0
        AND rotation2 = 0
        AND rotation3 = 0
        AND spawntimesecs = 300
        AND animprogress = 255
        AND state = 1
        AND ScriptName = ''
        AND StringId IS NULL
        AND VerifiedBuild = 0"
}

shared_chest_spawn_cleanup_counts() {
  loot_fixture_world_mysql -e \
    "SELECT
       COUNT(*),
       COALESCE(SUM(
         id = ${LOOT_FIXTURE_CHEST_TEMPLATE_ENTRY}
         AND map = 0
         AND zoneId = 0
         AND areaId = 0
         AND spawnDifficulties = '0'
         AND phaseUseFlags = 0
         AND PhaseId = 0
         AND PhaseGroup = 0
         AND terrainSwapMap = -1
         AND position_x = CAST(-8946.95 AS FLOAT)
         AND position_y = CAST(-132.493 AS FLOAT)
         AND position_z = CAST(83.5312 AS FLOAT)
         AND orientation = 0
         AND rotation0 = 0
         AND rotation1 = 0
         AND rotation2 = 0
         AND rotation3 = 0
         AND spawntimesecs = 300
         AND animprogress = 255
         AND state = 1
         AND ScriptName = ''
         AND StringId IS NULL
         AND VerifiedBuild = 0
       ), 0),
       (SELECT COUNT(*) FROM pool_members
         WHERE type = 1 AND spawnId = ${LOOT_FIXTURE_CHEST_GUID}),
       (SELECT COUNT(*) FROM game_event_gameobject
         WHERE guid = ${LOOT_FIXTURE_CHEST_GUID}),
       (SELECT COUNT(*) FROM linked_respawn
         WHERE guid = ${LOOT_FIXTURE_CHEST_GUID}
            OR linkedGuid = ${LOOT_FIXTURE_CHEST_GUID}),
       (SELECT COUNT(*) FROM gameobject_addon
         WHERE guid = ${LOOT_FIXTURE_CHEST_GUID}),
       (SELECT COUNT(*) FROM gameobject_overrides
         WHERE spawnId = ${LOOT_FIXTURE_CHEST_GUID}),
       (SELECT COUNT(*) FROM spawn_group
         WHERE spawnType = 1 AND spawnId = ${LOOT_FIXTURE_CHEST_GUID})
     FROM gameobject
     WHERE guid = ${LOOT_FIXTURE_CHEST_GUID}"
}

shared_chest_addon_exact_count() {
  local min_gold="$1"
  local max_gold="$2"
  loot_fixture_world_mysql -e \
    "SELECT COUNT(*) FROM gameobject_template_addon
      WHERE entry = ${LOOT_FIXTURE_CHEST_TEMPLATE_ENTRY}
        AND faction = ${LOOT_FIXTURE_CHEST_ADDON_FACTION}
        AND flags = 0
        AND mingold = ${min_gold}
        AND maxgold = ${max_gold}
        AND artkit0 = 0
        AND artkit1 = 0
        AND artkit2 = 0
        AND artkit3 = 0
        AND artkit4 = 0
        AND WorldEffectID = 0
        AND AIAnimKitID = 0"
}

shared_chest_spawn_metadata_counts() {
  loot_fixture_world_mysql -e \
    "SELECT
       (SELECT COUNT(*) FROM pool_members
         WHERE type = 1 AND spawnId = ${LOOT_FIXTURE_CHEST_GUID}),
       (SELECT COUNT(*) FROM game_event_gameobject
         WHERE guid = ${LOOT_FIXTURE_CHEST_GUID}),
       (SELECT COUNT(*) FROM linked_respawn
         WHERE guid = ${LOOT_FIXTURE_CHEST_GUID}
            OR linkedGuid = ${LOOT_FIXTURE_CHEST_GUID}),
       (SELECT COUNT(*) FROM gameobject_addon
         WHERE guid = ${LOOT_FIXTURE_CHEST_GUID}),
       (SELECT COUNT(*) FROM gameobject_overrides
         WHERE spawnId = ${LOOT_FIXTURE_CHEST_GUID}),
       (SELECT COUNT(*) FROM spawn_group
         WHERE spawnType = 1 AND spawnId = ${LOOT_FIXTURE_CHEST_GUID})"
}

apply_shared_chest_fixture_guard() {
  local total matching conditions inserted ownership

  matching="$(loot_fixture_world_mysql -e \
    "SELECT COUNT(*) FROM gameobject_template
      WHERE entry = ${LOOT_FIXTURE_CHEST_TEMPLATE_ENTRY}
        AND type = 3
        AND Data1 = ${LOOT_FIXTURE_CHEST_LOOT_ENTRY}
        AND Data15 = 1")" || return 1
  [ "$matching" = "1" ] || {
    echo "error: shared chest template ${LOOT_FIXTURE_CHEST_TEMPLATE_ENTRY} must exist with type=3, Data1=${LOOT_FIXTURE_CHEST_LOOT_ENTRY}, Data15=1" >&2
    return 1
  }

  total="$(loot_fixture_world_mysql -e \
    "SELECT COUNT(*) FROM gameobject_loot_template
      WHERE Entry = ${LOOT_FIXTURE_CHEST_LOOT_ENTRY}")" || return 1
  matching="$(loot_fixture_world_mysql -e \
    "SELECT COUNT(*) FROM gameobject_loot_template
      WHERE Entry = ${LOOT_FIXTURE_CHEST_LOOT_ENTRY}
        AND Item = ${LOOT_FIXTURE_CHEST_ITEM}
        AND Reference = 0
        AND Chance = 100
        AND QuestRequired = 0
        AND LootMode = 1
        AND GroupId = 0
        AND MinCount = 1
        AND MaxCount = 1")" || return 1
  [ "$total" = "1" ] && [ "$matching" = "1" ] || {
    echo "error: shared chest loot ${LOOT_FIXTURE_CHEST_LOOT_ENTRY} must contain exactly item ${LOOT_FIXTURE_CHEST_ITEM} at 100% with the pinned normal-loot fields" >&2
    return 1
  }

  conditions="$(loot_fixture_world_mysql -e \
    "SELECT COUNT(*) FROM conditions
      WHERE SourceTypeOrReferenceId = 4
        AND SourceGroup = ${LOOT_FIXTURE_CHEST_LOOT_ENTRY}")" || return 1
  [ "$conditions" = "0" ] || {
    echo "error: shared chest loot ${LOOT_FIXTURE_CHEST_LOOT_ENTRY} must not have gameobject-loot conditions" >&2
    return 1
  }

  total="$(loot_fixture_world_mysql -e \
    "SELECT COUNT(*) FROM gameobject_template_addon
      WHERE entry = ${LOOT_FIXTURE_CHEST_TEMPLATE_ENTRY}")" || return 1
  matching="$(shared_chest_addon_exact_count 0 0)" || return 1
  [ "$total" = "1" ] && [ "$matching" = "1" ] || {
    echo "error: shared chest addon ${LOOT_FIXTURE_CHEST_TEMPLATE_ENTRY} does not match the pinned faction/flags/artkits/effects and 0/0 money contract" >&2
    return 1
  }

  total="$(loot_fixture_world_mysql -e \
    "SELECT COUNT(*) FROM gameobject WHERE guid = ${LOOT_FIXTURE_CHEST_GUID}")" || return 1
  [ "$total" = "0" ] || {
    echo "error: refusing to replace pre-existing gameobject guid ${LOOT_FIXTURE_CHEST_GUID}" >&2
    return 1
  }

  # Reject every spawn-scoped row loaded by ObjectMgr for this GameObject. A
  # nominally absent `gameobject` row is not safe to claim when orphan addon,
  # override, spawn-group, pool, event, or linked-respawn metadata still exists.
  ownership="$(shared_chest_spawn_metadata_counts)" || return 1
  [ "$ownership" = $'0\t0\t0\t0\t0\t0' ] || {
    echo "error: shared chest guid ${LOOT_FIXTURE_CHEST_GUID} has spawn-scoped metadata (pool/event/linked/addon/override/spawn-group: ${ownership:-query-failed})" >&2
    return 1
  }

  total="$(loot_fixture_character_mysql -e \
    "SELECT COUNT(*) FROM respawn
      WHERE type = 1
        AND spawnId = ${LOOT_FIXTURE_CHEST_GUID}")" || return 1
  [ "$total" = "0" ] || {
    echo "error: shared chest guid ${LOOT_FIXTURE_CHEST_GUID} already owns ${total} gameobject respawn row(s)" >&2
    return 1
  }

  # The exact world process may create this row after the chest is consumed.
  # Absence was proven above; arm its fail-closed cleanup before that process
  # can start, even if a later fixture mutation or verification is interrupted.
  LOOT_FIXTURE_CHEST_RESPAWN_DELETE_READY=1

  # Arm restoration before each write so EXIT/HUP/INT/TERM between the write
  # and verification still runs a fail-closed cleanup.
  LOOT_FIXTURE_CHEST_ADDON_RESTORE_READY=1
  matching="$(loot_fixture_world_mysql -e \
    "UPDATE gameobject_template_addon
        SET mingold = 10, maxgold = 10
      WHERE entry = ${LOOT_FIXTURE_CHEST_TEMPLATE_ENTRY}
        AND faction = ${LOOT_FIXTURE_CHEST_ADDON_FACTION}
        AND flags = 0
        AND mingold = 0
        AND maxgold = 0
        AND artkit0 = 0
        AND artkit1 = 0
        AND artkit2 = 0
        AND artkit3 = 0
        AND artkit4 = 0
        AND WorldEffectID = 0
        AND AIAnimKitID = 0;
      SELECT ROW_COUNT();")" || return 1
  [ "$matching" = "1" ] \
    && [ "$(shared_chest_addon_exact_count 10 10)" = "1" ] || {
    echo "error: failed to activate shared chest money fixture" >&2
    return 1
  }

  LOOT_FIXTURE_CHEST_SPAWN_DELETE_READY=1
  inserted="$(loot_fixture_world_mysql -e \
    "INSERT INTO gameobject
      (guid, id, map, zoneId, areaId, spawnDifficulties, phaseUseFlags,
       PhaseId, PhaseGroup, terrainSwapMap, position_x, position_y, position_z,
       orientation, rotation0, rotation1, rotation2, rotation3, spawntimesecs,
       animprogress, state, ScriptName, StringId, VerifiedBuild)
      SELECT ${LOOT_FIXTURE_CHEST_GUID}, ${LOOT_FIXTURE_CHEST_TEMPLATE_ENTRY},
             0, 0, 0, '0', 0, 0, 0, -1,
             CAST(-8946.95 AS FLOAT), CAST(-132.493 AS FLOAT),
             CAST(83.5312 AS FLOAT),
             0, 0, 0, 0, 0, 300, 255, 1, '', NULL, 0
      WHERE NOT EXISTS
        (SELECT 1 FROM gameobject WHERE guid = ${LOOT_FIXTURE_CHEST_GUID});
      SELECT ROW_COUNT();")" || return 1
  [ "$inserted" = "1" ] || {
    # A successful zero-row statement proves this invocation did not own the
    # row; disarm deletion so cleanup never removes somebody else's spawn.
    LOOT_FIXTURE_CHEST_SPAWN_DELETE_READY=0
    echo "error: shared chest guid ${LOOT_FIXTURE_CHEST_GUID} appeared before fixture insertion" >&2
    return 1
  }
  matching="$(shared_chest_spawn_exact_count)" || return 1
  [ "$matching" = "1" ] || {
    echo "error: failed to verify exact shared chest spawn ${LOOT_FIXTURE_CHEST_GUID}" >&2
    return 1
  }

  echo "loot fixture: installed shared chest guid ${LOOT_FIXTURE_CHEST_GUID} (item ${LOOT_FIXTURE_CHEST_ITEM}, money 10; restore armed)"
}

restore_shared_chest_fixture_guard() {
  local total matching respawn_time ownership deleted spawn_counts

  # Preflight every wrapper-owned surface before the first cleanup write. In
  # particular, do not erase a generated respawn and only afterwards discover
  # that the world spawn's persisted `state` or spawn metadata drifted.
  if [ "$LOOT_FIXTURE_CHEST_SPAWN_DELETE_READY" -eq 1 ]; then
    spawn_counts="$(shared_chest_spawn_cleanup_counts)" || spawn_counts=""
    if [ "$spawn_counts" = $'0\t0\t0\t0\t0\t0\t0\t0' ]; then
      # The flag is armed before INSERT. A failure between those operations,
      # or a successful prior DELETE whose verification query failed, already
      # satisfies the exact owned-spawn cleanup postcondition.
      LOOT_FIXTURE_CHEST_SPAWN_DELETE_READY=0
    elif [ "$spawn_counts" != $'1\t1\t0\t0\t0\t0\t0\t0' ]; then
      echo "WARNING: shared chest spawn/state/metadata drifted; refusing every cleanup write (${spawn_counts:-query-failed})" >&2
      return 1
    fi
  fi
  if [ "$LOOT_FIXTURE_CHEST_ADDON_RESTORE_READY" -eq 1 ] \
      && [ "$(shared_chest_addon_exact_count 0 0 2>/dev/null || true)" != "1" ] \
      && [ "$(shared_chest_addon_exact_count 10 10 2>/dev/null || true)" != "1" ]; then
    echo "WARNING: shared chest template addon drifted; refusing every cleanup write" >&2
    return 1
  fi

  if [ "$LOOT_FIXTURE_CHEST_RESPAWN_DELETE_READY" -eq 1 ]; then
    total="$(loot_fixture_character_mysql -e \
      "SELECT COUNT(*) FROM respawn
        WHERE type = 1
          AND spawnId = ${LOOT_FIXTURE_CHEST_GUID}")" || total=""
    if [ "$total" = "0" ]; then
      LOOT_FIXTURE_CHEST_RESPAWN_DELETE_READY=0
    elif [ "$total" = "1" ]; then
      respawn_time="$(loot_fixture_character_mysql -e \
        "SELECT respawnTime FROM respawn
          WHERE type = 1
            AND spawnId = ${LOOT_FIXTURE_CHEST_GUID}
            AND mapId = 0
            AND instanceId = 0")" || respawn_time=""
      if [[ "$respawn_time" =~ ^[1-9][0-9]*$ ]]; then
        deleted="$(loot_fixture_character_mysql -e \
            "DELETE FROM respawn
              WHERE type = 1
                AND spawnId = ${LOOT_FIXTURE_CHEST_GUID}
                AND respawnTime = ${respawn_time}
                AND mapId = 0
                AND instanceId = 0;
              SELECT ROW_COUNT();")" || deleted=""
      else
        deleted=""
      fi
      if [ "$deleted" = "1" ] \
          && [ "$(loot_fixture_character_mysql -e \
            "SELECT COUNT(*) FROM respawn
              WHERE type = 1
                AND spawnId = ${LOOT_FIXTURE_CHEST_GUID}")" = "0" ]; then
        LOOT_FIXTURE_CHEST_RESPAWN_DELETE_READY=0
        echo "loot fixture: removed generated respawn for shared chest guid ${LOOT_FIXTURE_CHEST_GUID}"
      else
        echo "WARNING: shared chest respawn ${LOOT_FIXTURE_CHEST_GUID} changed externally; refusing a non-exact delete" >&2
      fi
    else
      echo "WARNING: shared chest guid ${LOOT_FIXTURE_CHEST_GUID} has unexpected respawn ownership (${total:-query-failed}); refusing to delete it" >&2
    fi
  fi

  if [ "$LOOT_FIXTURE_CHEST_SPAWN_DELETE_READY" -eq 1 ]; then
    spawn_counts="$(shared_chest_spawn_cleanup_counts)" || spawn_counts=""
    if [ "$spawn_counts" = $'0\t0\t0\t0\t0\t0\t0\t0' ]; then
      LOOT_FIXTURE_CHEST_SPAWN_DELETE_READY=0
    elif [ "$spawn_counts" = $'1\t1\t0\t0\t0\t0\t0\t0' ]; then
      deleted="$(loot_fixture_world_mysql -e \
          "DELETE FROM gameobject
            WHERE guid = ${LOOT_FIXTURE_CHEST_GUID}
              AND id = ${LOOT_FIXTURE_CHEST_TEMPLATE_ENTRY}
              AND map = 0
              AND zoneId = 0
              AND areaId = 0
              AND spawnDifficulties = '0'
              AND phaseUseFlags = 0
              AND PhaseId = 0
              AND PhaseGroup = 0
              AND terrainSwapMap = -1
              AND position_x = CAST(-8946.95 AS FLOAT)
              AND position_y = CAST(-132.493 AS FLOAT)
              AND position_z = CAST(83.5312 AS FLOAT)
              AND orientation = 0
              AND rotation0 = 0
              AND rotation1 = 0
              AND rotation2 = 0
              AND rotation3 = 0
              AND spawntimesecs = 300
              AND animprogress = 255
              AND state = 1
              AND ScriptName = ''
              AND StringId IS NULL
              AND VerifiedBuild = 0;
            SELECT ROW_COUNT();")" \
        || deleted=""
      if [ "$deleted" = "1" ] \
          && [ "$(shared_chest_spawn_cleanup_counts)" \
            = $'0\t0\t0\t0\t0\t0\t0\t0' ]; then
        LOOT_FIXTURE_CHEST_SPAWN_DELETE_READY=0
        echo "loot fixture: removed shared chest guid ${LOOT_FIXTURE_CHEST_GUID}"
      else
        echo "WARNING: failed to remove shared chest guid ${LOOT_FIXTURE_CHEST_GUID}" >&2
      fi
    else
      echo "WARNING: shared chest guid ${LOOT_FIXTURE_CHEST_GUID} changed externally; refusing to delete it" >&2
    fi
  fi

  if [ "$LOOT_FIXTURE_CHEST_ADDON_RESTORE_READY" -eq 1 ]; then
    matching="$(shared_chest_addon_exact_count 0 0)" || matching=""
    if [ "$matching" = "1" ]; then
      LOOT_FIXTURE_CHEST_ADDON_RESTORE_READY=0
    else
      matching="$(shared_chest_addon_exact_count 10 10)" || matching=""
      if [ "$matching" != "1" ]; then
        echo "WARNING: shared chest addon ${LOOT_FIXTURE_CHEST_TEMPLATE_ENTRY} changed externally; refusing to overwrite it" >&2
      else
        deleted="$(loot_fixture_world_mysql -e \
          "UPDATE gameobject_template_addon
              SET mingold = 0, maxgold = 0
            WHERE entry = ${LOOT_FIXTURE_CHEST_TEMPLATE_ENTRY}
              AND faction = ${LOOT_FIXTURE_CHEST_ADDON_FACTION}
              AND flags = 0
              AND mingold = 10
              AND maxgold = 10
              AND artkit0 = 0
              AND artkit1 = 0
              AND artkit2 = 0
              AND artkit3 = 0
              AND artkit4 = 0
              AND WorldEffectID = 0
              AND AIAnimKitID = 0;
            SELECT ROW_COUNT();")" || deleted=""
        if [ "$deleted" = "1" ] \
            && [ "$(shared_chest_addon_exact_count 0 0)" = "1" ]; then
          LOOT_FIXTURE_CHEST_ADDON_RESTORE_READY=0
          echo "loot fixture: restored chest addon ${LOOT_FIXTURE_CHEST_TEMPLATE_ENTRY} mingold/maxgold 0/0"
        else
          echo "WARNING: failed to restore shared chest addon ${LOOT_FIXTURE_CHEST_TEMPLATE_ENTRY}" >&2
        fi
      fi
    fi
  fi

  # Reconcile each still-armed surface from fresh DB postconditions. This
  # self-heals a cleanup write that committed successfully when only its first
  # verification query failed, without ever treating a present/drifted row as
  # clean.
  if [ "$LOOT_FIXTURE_CHEST_RESPAWN_DELETE_READY" -eq 1 ] \
      && [ "$(loot_fixture_character_mysql -e \
        "SELECT COUNT(*) FROM respawn
          WHERE type = 1
            AND spawnId = ${LOOT_FIXTURE_CHEST_GUID}" 2>/dev/null || true)" = "0" ]; then
    LOOT_FIXTURE_CHEST_RESPAWN_DELETE_READY=0
  fi
  if [ "$LOOT_FIXTURE_CHEST_SPAWN_DELETE_READY" -eq 1 ]; then
    spawn_counts="$(shared_chest_spawn_cleanup_counts 2>/dev/null || true)"
    if [ "$spawn_counts" = $'0\t0\t0\t0\t0\t0\t0\t0' ]; then
      LOOT_FIXTURE_CHEST_SPAWN_DELETE_READY=0
    fi
  fi
  if [ "$LOOT_FIXTURE_CHEST_ADDON_RESTORE_READY" -eq 1 ] \
      && [ "$(shared_chest_addon_exact_count 0 0 2>/dev/null || true)" = "1" ]; then
    LOOT_FIXTURE_CHEST_ADDON_RESTORE_READY=0
  fi

  # The armed flags are the durable cleanup invariants: each one is cleared
  # only after its owned row is absent or its exact original value is restored.
  # Derive success from those postconditions instead of a parallel accumulator,
  # so the wrapper cannot report failure after every owned surface is clean.
  if [ "$LOOT_FIXTURE_CHEST_RESPAWN_DELETE_READY" -eq 0 ] \
      && [ "$LOOT_FIXTURE_CHEST_SPAWN_DELETE_READY" -eq 0 ] \
      && [ "$LOOT_FIXTURE_CHEST_ADDON_RESTORE_READY" -eq 0 ]; then
    return 0
  fi

  echo "WARNING: shared chest cleanup remains armed (respawn=${LOOT_FIXTURE_CHEST_RESPAWN_DELETE_READY}, spawn=${LOOT_FIXTURE_CHEST_SPAWN_DELETE_READY}, addon=${LOOT_FIXTURE_CHEST_ADDON_RESTORE_READY})" >&2
  return 1
}
