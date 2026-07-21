#!/usr/bin/env bash
# capture-rust.sh — record a RustyCore packet dump for one flow.
#
# Restarts the RustyCore world server with RUSTYCORE_PACKET_DUMP_DIR pointed at a
# fresh directory, pauses for you to perform the flow with a client, then leaves
# the dump in place and restarts the server cleanly.
#
# Usage:   crates/capture-diff/scripts/capture-rust.sh <flow> [--yes]
# Output:  target/captures/<flow>/rust/   (gitignored; .bin/.meta per packet,
#          rust.capture-manifest.json, plus the retained race bot report for
#          loot-two-session-atomic-race)
#
# Honored env vars:
#   PM2_RUST_WORLD  pm2 name of the Rust world (default: rustycore-world)
#   PM2_CPP_WORLD   pm2 name of the C++ world  (default: cpp-world) — stopped first
#   RUST_WORLD_PORT realm listener readiness port (default: 8085)
#   RUST_INSTANCE_PORT instance listener readiness port (default: 8086)
#   RUST_CAPTURE_EXEC optional absolute canonical executable used only while
#                     capturing; the original PM2 executable is still restored
#   RUST_CAPTURE_EXEC_SHA256 mandatory 64-hex SHA-256 when RUST_CAPTURE_EXEC is
#                            set; both the file and live /proc executable must match
#   RUST_CAPTURE_LOOT_FIXTURE_GUARD set to 1 only for the versioned
#                            loot-single-item-claim or loot-two-session-atomic-race
#                            fixtures; the single-item flow temporarily lowers
#                            its creature HealthModifier, while the two-session
#                            flow installs a guarded shared QA chest. Every
#                            mutation is restored before the original PM2
#                            profile starts again
#   RUST_CAPTURE_ACK_LOOT_FIXTURE_MUTATION must be 1 with the fixture guard
#   RUST_CAPTURE_DB_CONF worldserver.conf containing WorldDatabaseInfo and
#                            CharacterDatabaseInfo (default: legacy runtime conf)
#   RUST_CAPTURE_EFFECTIVE_CONFIG exact config file used by the Rust capture
#                            process for capture-relevant settings; required
#                            explicitly for guarded evidence (defaults to
#                            RUST_CAPTURE_DB_CONF for non-guarded captures)
#   WOW_BOT_FIXTURE_JOURNAL absolute path to the bot's mode-0600 recovery
#                            journal. Guarded loot captures require the bot to
#                            remove this pending journal and atomically create
#                            ${WOW_BOT_FIXTURE_JOURNAL}.cleanup-complete before
#                            the normal PM2 world may be restored
#   CAPTURE_ORCHESTRATION_LOCK optional absolute private lock directory shared
#                            with capture-cpp.sh (default: /tmp, keyed by uid+ports)
#   CAPTURE_WORLD_READY_TIMEOUT_SECONDS bounded wait for a stable ready world
#                            (default: 180, range: 3 through 3600)
#   WOW_BOT_EXEC / WOW_BOT_EXEC_SHA256 pinned bot executable used for guarded
#                            #106 loot and vendor-extended-cost-purchase evidence
#   WOW_BOT_REPORT           fresh absolute bot JSON report path. The guarded
#                            wrapper independently validates the exact selected
#                            loot or vendor contract before publish
#
# This restarts the live world server (disconnecting players). Pass --yes to skip
# the confirmation prompt.
set -euo pipefail

FLOW="${1:-}"
[ -n "$FLOW" ] || { echo "usage: $0 <flow> [--yes]" >&2; exit 2; }
[[ "$FLOW" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]] || {
  echo "error: invalid flow name '${FLOW}' (use one ASCII path component: letters, digits, '.', '_', '-')" >&2
  exit 2
}
CONFIRM="${2:-}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
PM2_RUST_WORLD="${PM2_RUST_WORLD:-rustycore-world}"
PM2_CPP_WORLD="${PM2_CPP_WORLD:-cpp-world}"
RUST_WORLD_PORT="${RUST_WORLD_PORT:-8085}"
RUST_INSTANCE_PORT="${RUST_INSTANCE_PORT:-8086}"
RUST_CAPTURE_EXEC="${RUST_CAPTURE_EXEC:-}"
RUST_CAPTURE_EXEC_SHA256="${RUST_CAPTURE_EXEC_SHA256:-}"
CAPTURE_WORLD_PORT="$RUST_WORLD_PORT"
CAPTURE_INSTANCE_PORT="$RUST_INSTANCE_PORT"
CAPTURE_ORCHESTRATION_LOCK="${CAPTURE_ORCHESTRATION_LOCK:-${XDG_RUNTIME_DIR:-/tmp}/rustycore-capture-$(id -u)-${RUST_WORLD_PORT}-${RUST_INSTANCE_PORT}.lock.d}"
CAPTURE_ORCHESTRATION_LOCK_FD=""
RUST_CAPTURE_LOOT_FIXTURE_GUARD="${RUST_CAPTURE_LOOT_FIXTURE_GUARD:-0}"
LOOT_FIXTURE_GUARD_ENABLED="$RUST_CAPTURE_LOOT_FIXTURE_GUARD"
RUST_CAPTURE_ACK_LOOT_FIXTURE_MUTATION="${RUST_CAPTURE_ACK_LOOT_FIXTURE_MUTATION:-0}"
RUST_CAPTURE_DB_CONF="${RUST_CAPTURE_DB_CONF:-/home/server/trinity-legacy-install/bin/worldserver.conf}"
RUST_CAPTURE_EFFECTIVE_CONFIG_WAS_SET="${RUST_CAPTURE_EFFECTIVE_CONFIG+x}"
RUST_CAPTURE_EFFECTIVE_CONFIG="${RUST_CAPTURE_EFFECTIVE_CONFIG:-$RUST_CAPTURE_DB_CONF}"
LOOT_FIXTURE_DB_CONF="$RUST_CAPTURE_DB_CONF"
WOW_BOT_FIXTURE_JOURNAL="${WOW_BOT_FIXTURE_JOURNAL:-}"
WOW_BOT_EXEC="${WOW_BOT_EXEC:-}"
WOW_BOT_EXEC_SHA256="${WOW_BOT_EXEC_SHA256:-}"
WOW_BOT_REPORT="${WOW_BOT_REPORT:-}"
CAPTURE_EXEC=""
CAPTURE_EXEC_SHA256=""
CAPTURE_EXPECTED_EXEC=""
CAPTURE_EXPECTED_SHA256=""
CAPTURE_SOURCE_EXEC=""
CAPTURE_SOURCE_SHA256=""
CAPTURE_HARNESS_REPO_HEAD=""
CAPTURE_SOURCE_REPO_HEAD=""
CAPTURE_HARNESS_WORKTREE_SHA256=""
CAPTURE_PM2_ENTRY_PID=""
CAPTURE_PM2_ENTRY_STARTTIME=""
CAPTURE_PM2_EXEC_PATH=""
CAPTURE_PM2_EXEC_SHA256=""
CAPTURE_PM2_PROFILE_SHA256=""
CAPTURE_RESTART_COUNT=""
CAPTURE_LISTENER_STARTTIME=""
CAPTURE_EFFECTIVE_CONFIG_PATH=""
CAPTURE_EFFECTIVE_CONFIG_SHA256=""
CAPTURE_RESTORE_FAILURE_STATUS=74
LOOT_FIXTURE_KIND=""
LOOT_FIXTURE_ENTRY=""
LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER=""
LOOT_FIXTURE_TEMP_HEALTH_MODIFIER="0.0001"
LOOT_FIXTURE_SNAPSHOT_READY=0
LOOT_FIXTURE_CHEST_TEMPLATE_ENTRY=2846
LOOT_FIXTURE_CHEST_LOOT_ENTRY=2278
LOOT_FIXTURE_CHEST_ITEM=38
LOOT_FIXTURE_CHEST_GUID=9106001
LOOT_FIXTURE_CHEST_ADDON_FACTION=101
LOOT_FIXTURE_CHEST_ADDON_RESTORE_READY=0
LOOT_FIXTURE_CHEST_SPAWN_DELETE_READY=0
LOOT_FIXTURE_CHEST_RESPAWN_DELETE_READY=0
LOOT_FIXTURE_CLEANUP_MARKER=""
LOOT_FIXTURE_WORLD_HOST=""
LOOT_FIXTURE_WORLD_PORT=""
LOOT_FIXTURE_WORLD_USER=""
LOOT_FIXTURE_WORLD_PASSWORD=""
LOOT_FIXTURE_WORLD_DATABASE=""
LOOT_FIXTURE_CHARACTER_HOST=""
LOOT_FIXTURE_CHARACTER_PORT=""
LOOT_FIXTURE_CHARACTER_USER=""
LOOT_FIXTURE_CHARACTER_PASSWORD=""
LOOT_FIXTURE_CHARACTER_DATABASE=""
CAPTURE_PROCESS_PID=""
CAPTURE_LIVE_EXEC=""
CAPTURE_LIVE_SHA256=""
CAPTURE_ARTIFACT_READY=0
DUMP_STAGE_DIR=""
CAPTURE_PROCESS_TREE_IDENTITIES=""
ORIGINAL_PM2_ENTRY_PID=""
ORIGINAL_PM2_ENTRY_STARTTIME=""
ORIGINAL_LISTENER_PID=""
ORIGINAL_LISTENER_STARTTIME=""
ORIGINAL_PROCESS_TREE_IDENTITIES=""
CAPTURE_RUNTIME_CLEANUP_VERIFIED=0
CAPTURE_NORMAL_RUNTIME_RESTORED=0
CAPTURE_FIXTURE_CLEANUP_VERIFIED=0
CAPTURE_BOT_EXEC=""
CAPTURE_BOT_EXEC_SHA256=""
CAPTURE_BOT_REPORT=""
CAPTURE_BOT_REPORT_SHA256=""
CAPTURE_BOT_READY=0
RESTORE_FILE_SHA256=""
CAPTURE_CONFIG_FILE_SHA256=""

# Keep the bounded SQL mutation and durable cleanup-marker contract identical
# for C++ and Rust recordings.
# shellcheck source=loot-fixture-common.sh
source "$(dirname "${BASH_SOURCE[0]}")/loot-fixture-common.sh"
# shellcheck source=capture-service-common.sh
source "$(dirname "${BASH_SOURCE[0]}")/capture-service-common.sh"
capture_validate_world_timeouts || exit 2

[[ "$RUST_WORLD_PORT" =~ ^[1-9][0-9]*$ ]] \
  && ((RUST_WORLD_PORT <= 65535)) || {
    echo "error: RUST_WORLD_PORT must be an integer from 1 through 65535" >&2
    exit 2
  }
[[ "$RUST_INSTANCE_PORT" =~ ^[1-9][0-9]*$ ]] \
  && ((RUST_INSTANCE_PORT <= 65535)) || {
    echo "error: RUST_INSTANCE_PORT must be an integer from 1 through 65535" >&2
    exit 2
  }
[ "$RUST_WORLD_PORT" != "$RUST_INSTANCE_PORT" ] || {
  echo "error: RUST_WORLD_PORT and RUST_INSTANCE_PORT must be distinct" >&2
  exit 2
}

if [ "$FLOW" = "loot-single-item-claim" ] \
    && [ "$RUST_CAPTURE_LOOT_FIXTURE_GUARD" != "1" ]; then
  echo "error: loot-single-item-claim requires RUST_CAPTURE_LOOT_FIXTURE_GUARD=1" >&2
  exit 2
fi

case "$RUST_CAPTURE_LOOT_FIXTURE_GUARD" in
  0) ;;
  1)
    [ "$RUST_CAPTURE_ACK_LOOT_FIXTURE_MUTATION" = "1" ] || {
      echo "error: RUST_CAPTURE_LOOT_FIXTURE_GUARD=1 requires RUST_CAPTURE_ACK_LOOT_FIXTURE_MUTATION=1" >&2
      exit 2
    }
    [ -n "$RUST_CAPTURE_EXEC" ] && [ -n "$RUST_CAPTURE_EXEC_SHA256" ] || {
      echo "error: guarded Rust evidence requires RUST_CAPTURE_EXEC and RUST_CAPTURE_EXEC_SHA256" >&2
      exit 2
    }
    [ "$RUST_CAPTURE_EFFECTIVE_CONFIG_WAS_SET" = "x" ] \
      && [ -n "$RUST_CAPTURE_EFFECTIVE_CONFIG" ] || {
      echo "error: guarded Rust evidence requires RUST_CAPTURE_EFFECTIVE_CONFIG" >&2
      exit 2
    }
    case "$FLOW" in
      loot-single-item-claim)
        LOOT_FIXTURE_KIND=creature-health
        LOOT_FIXTURE_ENTRY=21779
        LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER=1
        ;;
      loot-two-session-atomic-race)
        LOOT_FIXTURE_KIND=shared-chest
        ;;
      *)
        echo "error: the loot fixture guard is not defined for flow '${FLOW}'" >&2
        exit 2
        ;;
    esac
    [ -n "$WOW_BOT_EXEC" ] && [ -n "$WOW_BOT_EXEC_SHA256" ] \
      && [ -n "$WOW_BOT_REPORT" ] || {
      echo "error: guarded #106 evidence requires WOW_BOT_EXEC, WOW_BOT_EXEC_SHA256, and WOW_BOT_REPORT" >&2
      exit 2
    }
    [[ "$WOW_BOT_EXEC_SHA256" =~ ^[0-9A-Fa-f]{64}$ ]] || {
      echo "error: WOW_BOT_EXEC_SHA256 must contain exactly 64 hexadecimal characters" >&2
      exit 2
    }
    WOW_BOT_EXEC_SHA256="${WOW_BOT_EXEC_SHA256,,}"
    [[ "$WOW_BOT_REPORT" = /* && "$WOW_BOT_REPORT" != *$'\n'* ]] \
      && [ -d "$(dirname -- "$WOW_BOT_REPORT")" ] \
      && [ ! -e "$WOW_BOT_REPORT" ] && [ ! -L "$WOW_BOT_REPORT" ] || {
      echo "error: WOW_BOT_REPORT must be a fresh absolute path with an existing parent" >&2
      exit 2
    }
    WOW_BOT_REPORT_PARENT="$(dirname -- "$WOW_BOT_REPORT")"
    [ "$(realpath -e -- "$WOW_BOT_REPORT_PARENT" 2>/dev/null)" \
      = "$WOW_BOT_REPORT_PARENT" ] \
      && [ ! -L "$WOW_BOT_REPORT_PARENT" ] || {
      echo "error: WOW_BOT_REPORT parent must be canonical and non-symlink" >&2
      exit 2
    }
    capture_exec_source_matches "$WOW_BOT_EXEC" "$WOW_BOT_EXEC_SHA256" || {
      echo "error: WOW_BOT_EXEC is not a canonical pinned executable" >&2
      exit 2
    }
    validate_fresh_loot_fixture_journal || exit 2
    ;;
  *)
    echo "error: RUST_CAPTURE_LOOT_FIXTURE_GUARD must be 0 or 1" >&2
    exit 2
    ;;
esac

if [ "$FLOW" = "vendor-extended-cost-purchase" ]; then
  [ -n "$RUST_CAPTURE_EXEC" ] && [ -n "$RUST_CAPTURE_EXEC_SHA256" ] || {
    echo "error: vendor evidence requires RUST_CAPTURE_EXEC and RUST_CAPTURE_EXEC_SHA256" >&2
    exit 2
  }
  [ "$RUST_CAPTURE_EFFECTIVE_CONFIG_WAS_SET" = "x" ] \
    && [ -n "$RUST_CAPTURE_EFFECTIVE_CONFIG" ] || {
    echo "error: vendor evidence requires RUST_CAPTURE_EFFECTIVE_CONFIG" >&2
    exit 2
  }
  [ -n "$WOW_BOT_EXEC" ] && [ -n "$WOW_BOT_EXEC_SHA256" ] \
    && [ -n "$WOW_BOT_REPORT" ] || {
    echo "error: vendor evidence requires WOW_BOT_EXEC, WOW_BOT_EXEC_SHA256, and WOW_BOT_REPORT" >&2
    exit 2
  }
  [[ "$WOW_BOT_EXEC_SHA256" =~ ^[0-9A-Fa-f]{64}$ ]] || {
    echo "error: WOW_BOT_EXEC_SHA256 must contain exactly 64 hexadecimal characters" >&2
    exit 2
  }
  WOW_BOT_EXEC_SHA256="${WOW_BOT_EXEC_SHA256,,}"
  capture_validate_fresh_bot_inputs \
    "$WOW_BOT_EXEC" "$WOW_BOT_EXEC_SHA256" "$WOW_BOT_REPORT" || {
    echo "error: vendor bot executable/report inputs are not fresh, canonical, and pinned" >&2
    exit 2
  }
fi

sha256_of_file() {
  local output digest
  output="$(sha256sum <"$1")" || return 1
  digest="${output%% *}"
  [[ "$digest" =~ ^[0-9a-f]{64}$ ]] || return 1
  printf '%s\n' "$digest"
}

apply_loot_fixture_guard() {
  [ "$RUST_CAPTURE_LOOT_FIXTURE_GUARD" = "1" ] || return 0

  loot_fixture_wait_until_all_characters_offline || return 1

  if [ "$LOOT_FIXTURE_KIND" = "shared-chest" ]; then
    apply_shared_chest_fixture_guard
    return
  fi
  apply_creature_health_fixture_guard
}

restore_loot_fixture_guard() {
  if [ "$LOOT_FIXTURE_KIND" = "shared-chest" ]; then
    restore_shared_chest_fixture_guard
    return
  fi

  restore_creature_health_fixture_guard
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

capture_exec_source_matches() {
  [ -n "$CAPTURE_EXEC" ] || return 0

  local canonical digest
  canonical="$(realpath -e -- "$CAPTURE_EXEC" 2>/dev/null)" || return 1
  [ "$canonical" = "$CAPTURE_EXEC" ] || return 1
  [ -f "$CAPTURE_EXEC" ] && [ -x "$CAPTURE_EXEC" ] \
    && [ ! -L "$CAPTURE_EXEC" ] || return 1
  digest="$(sha256_of_file "$CAPTURE_EXEC")" || return 1
  [ "$digest" = "$CAPTURE_EXEC_SHA256" ]
}

capture_process_exec_matches() {
  local identity="$1"
  [ -n "$CAPTURE_EXEC" ] || return 0

  local pm2_pid pid proc_exe live_exec source_digest live_digest
  [[ "$identity" == *$'\t'* ]] || return 1
  pm2_pid="${identity%%$'\t'*}"
  pid="$(capture_world_listener_pid)" || return 1
  capture_pid_is_self_or_descendant "$pid" "$pm2_pid" || return 1
  [[ "$pid" =~ ^[1-9][0-9]*$ ]] || return 1
  proc_exe="/proc/${pid}/exe"
  [ -L "$proc_exe" ] || return 1

  # Resolve through /proc so an unlinked/replaced executable (reported as
  # "(deleted)") is rejected instead of being confused with the supplied path.
  live_exec="$(realpath -e -- "$proc_exe" 2>/dev/null)" || return 1
  [ "$live_exec" = "$CAPTURE_EXEC" ] || return 1

  # Hash both names: CAPTURE_EXEC proves the source path still names the pinned
  # bytes, while /proc/<pid>/exe proves those are the bytes PM2 actually ran.
  source_digest="$(sha256_of_file "$CAPTURE_EXEC")" || return 1
  live_digest="$(sha256_of_file "$proc_exe")" || return 1
  [ "$source_digest" = "$CAPTURE_EXEC_SHA256" ] \
    && [ "$live_digest" = "$CAPTURE_EXEC_SHA256" ]
}

if [ -n "$RUST_CAPTURE_EXEC" ]; then
  command -v realpath >/dev/null 2>&1 || {
    echo "error: realpath is required when RUST_CAPTURE_EXEC is set" >&2
    exit 1
  }
  [[ "$RUST_CAPTURE_EXEC" = /* ]] || {
    echo "error: RUST_CAPTURE_EXEC must be an absolute canonical path" >&2
    exit 1
  }
  if ! CAPTURE_EXEC="$(realpath -e -- "$RUST_CAPTURE_EXEC" 2>/dev/null)"; then
    echo "error: RUST_CAPTURE_EXEC does not exist: ${RUST_CAPTURE_EXEC}" >&2
    exit 1
  fi
  [ "$CAPTURE_EXEC" = "$RUST_CAPTURE_EXEC" ] || {
    echo "error: RUST_CAPTURE_EXEC must already be canonical: ${CAPTURE_EXEC}" >&2
    exit 1
  }
  [ -f "$CAPTURE_EXEC" ] && [ -x "$CAPTURE_EXEC" ] \
    && [ ! -L "$CAPTURE_EXEC" ] || {
    echo "error: RUST_CAPTURE_EXEC is not an executable regular file" >&2
    exit 1
  }
  [ -n "$RUST_CAPTURE_EXEC_SHA256" ] || {
    echo "error: RUST_CAPTURE_EXEC_SHA256 is required when RUST_CAPTURE_EXEC is set" >&2
    exit 1
  }
  [[ "$RUST_CAPTURE_EXEC_SHA256" =~ ^[0-9A-Fa-f]{64}$ ]] || {
    echo "error: RUST_CAPTURE_EXEC_SHA256 must contain exactly 64 hexadecimal characters" >&2
    exit 1
  }
  command -v sha256sum >/dev/null 2>&1 || {
    echo "error: sha256sum is required when RUST_CAPTURE_EXEC is set" >&2
    exit 1
  }
  CAPTURE_EXEC_SHA256="${RUST_CAPTURE_EXEC_SHA256,,}"
  if ! capture_exec_source_matches; then
    echo "error: RUST_CAPTURE_EXEC does not match RUST_CAPTURE_EXEC_SHA256" >&2
    exit 1
  fi
  CAPTURE_EXPECTED_EXEC="$CAPTURE_EXEC"
  CAPTURE_EXPECTED_SHA256="$CAPTURE_EXEC_SHA256"
  CAPTURE_SOURCE_EXEC="$CAPTURE_EXEC"
  CAPTURE_SOURCE_SHA256="$CAPTURE_EXEC_SHA256"
elif [ -n "$RUST_CAPTURE_EXEC_SHA256" ]; then
  echo "error: RUST_CAPTURE_EXEC_SHA256 requires RUST_CAPTURE_EXEC" >&2
  exit 1
fi

DUMP_PARENT_DIR="${REPO_ROOT}/target/captures/${FLOW}"
DUMP_DIR="${DUMP_PARENT_DIR}/rust"
capture_require_canonical_directory "${REPO_ROOT}/target/captures" \
  && capture_require_canonical_directory "$DUMP_PARENT_DIR" || {
  echo "error: capture output root is not canonical or contains a symlink" >&2
  exit 2
}
[ ! -e "$DUMP_DIR" ] && [ ! -L "$DUMP_DIR" ] || {
  echo "error: raw Rust capture directory already exists; archive/remove it before recording a new atomic generation: ${DUMP_DIR}" >&2
  exit 2
}

CAPTURE_HARNESS_REPO_HEAD="$(git -C "$REPO_ROOT" rev-parse HEAD 2>/dev/null)" || {
  echo "error: cannot resolve RustyCore harness repository HEAD" >&2
  exit 1
}
CAPTURE_SOURCE_REPO_HEAD="$CAPTURE_HARNESS_REPO_HEAD"
capture_git_repo_clean_at_head "$REPO_ROOT" "$CAPTURE_HARNESS_REPO_HEAD" || {
  echo "error: capture evidence requires a clean committed RustyCore harness/source worktree (including untracked files)" >&2
  exit 1
}
CAPTURE_HARNESS_WORKTREE_SHA256="$(
  capture_git_worktree_state_sha256 "$REPO_ROOT"
)" || {
  echo "error: cannot fingerprint the RustyCore harness/source worktree" >&2
  exit 1
}
CAPTURE_EFFECTIVE_CONFIG_PATH="$(realpath -e -- "$RUST_CAPTURE_EFFECTIVE_CONFIG" 2>/dev/null)" || {
  echo "error: RUST_CAPTURE_EFFECTIVE_CONFIG does not resolve" >&2
  exit 1
}
[ "$CAPTURE_EFFECTIVE_CONFIG_PATH" = "$RUST_CAPTURE_EFFECTIVE_CONFIG" ] \
  && [ -f "$CAPTURE_EFFECTIVE_CONFIG_PATH" ] \
  && [ ! -L "$CAPTURE_EFFECTIVE_CONFIG_PATH" ] || {
  echo "error: RUST_CAPTURE_EFFECTIVE_CONFIG must be an absolute canonical regular non-symlink file" >&2
  exit 1
}

rust_capture_effective_config_sha256() {
  capture_effective_config_redacted_sha256 \
    "$CAPTURE_EFFECTIVE_CONFIG_PATH" \
    "capture.world_port=${RUST_WORLD_PORT}
capture.instance_port=${RUST_INSTANCE_PORT}
capture.packet_dump=enabled" \
    PacketLogFile LogsDir Bot.AccountPrefix WorldServerPort InstanceServerPort \
    LoginDatabaseInfo WorldDatabaseInfo CharacterDatabaseInfo \
    Rate.Drop.Item.Poor Rate.Drop.Item.Normal Rate.Drop.Item.Uncommon \
    Rate.Drop.Item.Rare Rate.Drop.Item.Epic Rate.Drop.Item.Legendary \
    Rate.Drop.Item.Artifact Rate.Drop.Item.Referenced Rate.Drop.Money
}

CAPTURE_EFFECTIVE_CONFIG_SHA256="$(rust_capture_effective_config_sha256)" || {
  echo "error: cannot hash the canonical redacted effective Rust capture config" >&2
  exit 1
}

rust_capture_tree_digest() {
  local directory="$1"
  local output digest file_sha path
  rust_capture_flat_tree_is_safe "$directory" || return 1
  output="$({
    cd "$directory" || exit 1
    while IFS= read -r -d '' path; do
      file_sha="$(sha256_of_file "$path")" || exit 1
      printf '%s\0%s\0' "${path#./}" "$file_sha"
    done < <(find . -mindepth 1 -maxdepth 1 -type f -print0 | LC_ALL=C sort -z)
  } | sha256sum)" || return 1
  digest="${output%% *}"
  [[ "$digest" =~ ^[0-9a-f]{64}$ ]] || return 1
  printf '%s\n' "$digest"
}

rust_capture_flat_tree_is_safe() {
  local directory="$1"
  local path base
  [ -d "$directory" ] && [ ! -L "$directory" ] || return 1
  while IFS= read -r -d '' path; do
    [ -f "$path" ] && [ ! -L "$path" ] || return 1
    base="${path##*/}"
    case "$base" in
      *.bin|*.meta|rust.capture-manifest.json) ;;
      race.bot-report.json)
        [ "$FLOW" = "loot-two-session-atomic-race" ] || return 1
        ;;
      *) return 1 ;;
    esac
  done < <(find "$directory" -mindepth 1 -maxdepth 1 -print0)
}

finalize_rust_capture_artifact() {
  [ "$CAPTURE_ARTIFACT_READY" -eq 1 ] \
    && [ -d "$DUMP_STAGE_DIR" ] \
    && [ -n "$CAPTURE_LIVE_EXEC" ] \
    && [ -n "$CAPTURE_LIVE_SHA256" ] || return 1

  local bot_evidence="" capture_evidence created_at manifest packet_count
  local retained_bot_report path tree_sha
  [ "$CAPTURE_RUNTIME_CLEANUP_VERIFIED" -eq 1 ] \
    && [ "$CAPTURE_NORMAL_RUNTIME_RESTORED" -eq 1 ] || return 1
  capture_fixture_cleanup_verified_for_publication \
    "$RUST_CAPTURE_LOOT_FIXTURE_GUARD" \
    "$CAPTURE_FIXTURE_CLEANUP_VERIFIED" || return 1
  case "$FLOW" in
    loot-single-item-claim)
      bot_evidence="$(capture_loot_item_bot_evidence \
        "$WOW_BOT_REPORT" "$WOW_BOT_EXEC" "$WOW_BOT_EXEC_SHA256")" || return 1
      ;;
    loot-two-session-atomic-race)
      bot_evidence="$(capture_loot_race_bot_evidence \
        "$WOW_BOT_REPORT" "$WOW_BOT_EXEC" "$WOW_BOT_EXEC_SHA256")" || return 1
      ;;
    vendor-extended-cost-purchase)
      bot_evidence="$(capture_vendor_bot_evidence \
        "$WOW_BOT_REPORT" "$WOW_BOT_EXEC" "$WOW_BOT_EXEC_SHA256")" || return 1
      ;;
  esac
  if [ -n "$bot_evidence" ]; then
    IFS=$'\t' read -r CAPTURE_BOT_EXEC CAPTURE_BOT_EXEC_SHA256 \
      CAPTURE_BOT_REPORT CAPTURE_BOT_REPORT_SHA256 <<<"$bot_evidence"
  fi
  if [ "$FLOW" = "loot-two-session-atomic-race" ]; then
    retained_bot_report="$DUMP_STAGE_DIR/race.bot-report.json"
    [ ! -e "$retained_bot_report" ] && [ ! -L "$retained_bot_report" ] \
      && cp --no-clobber -- "$CAPTURE_BOT_REPORT" "$retained_bot_report" \
      && chmod 600 "$retained_bot_report" \
      && [ "$(sha256_of_file "$retained_bot_report")" \
        = "$CAPTURE_BOT_REPORT_SHA256" ] \
      && [ "$(sha256_of_file "$CAPTURE_BOT_REPORT")" \
        = "$CAPTURE_BOT_REPORT_SHA256" ] || return 1
    CAPTURE_BOT_REPORT="$DUMP_DIR/race.bot-report.json"
  fi
  capture_evidence="$(capture_bot_manifest_evidence \
    "$FLOW" "$CAPTURE_BOT_EXEC" "$CAPTURE_BOT_EXEC_SHA256" \
    "$CAPTURE_BOT_REPORT" "$CAPTURE_BOT_REPORT_SHA256")" || return 1
  tree_sha="$(rust_capture_tree_digest "$DUMP_STAGE_DIR")" || return 1
  packet_count="$(find "$DUMP_STAGE_DIR" -mindepth 1 -maxdepth 1 \
    -type f -name '*.meta' -print | wc -l)" \
    || return 1
  [[ "$packet_count" =~ ^[0-9]+$ ]] || return 1
  created_at="$(date -u +'%Y-%m-%dT%H:%M:%SZ')" || return 1
  capture_git_repo_clean_at_head "$REPO_ROOT" "$CAPTURE_HARNESS_REPO_HEAD" \
    && [ "$(capture_git_worktree_state_sha256 "$REPO_ROOT")" \
      = "$CAPTURE_HARNESS_WORKTREE_SHA256" ] || return 1
  manifest="$DUMP_STAGE_DIR/rust.capture-manifest.json"
  if ! jq -n \
      --arg flow "$FLOW" \
      --arg created_at "$created_at" \
      --arg harness_repo_head "$CAPTURE_HARNESS_REPO_HEAD" \
      --arg source_repo_head "$CAPTURE_SOURCE_REPO_HEAD" \
      --arg harness_worktree_sha256 "$CAPTURE_HARNESS_WORKTREE_SHA256" \
      --arg expected_exec_path "$CAPTURE_EXPECTED_EXEC" \
      --arg expected_exec_sha256 "$CAPTURE_EXPECTED_SHA256" \
      --arg source_exec_path "$CAPTURE_SOURCE_EXEC" \
      --arg source_exec_sha256 "$CAPTURE_SOURCE_SHA256" \
      --arg live_exec_path "$CAPTURE_LIVE_EXEC" \
      --arg live_exec_sha256 "$CAPTURE_LIVE_SHA256" \
      --arg pm2_exec_path "$CAPTURE_PM2_EXEC_PATH" \
      --arg pm2_exec_sha256 "$CAPTURE_PM2_EXEC_SHA256" \
      --arg pm2_profile_sha256 "$CAPTURE_PM2_PROFILE_SHA256" \
      --arg effective_config_path "$CAPTURE_EFFECTIVE_CONFIG_PATH" \
      --arg effective_config_sha256 "$CAPTURE_EFFECTIVE_CONFIG_SHA256" \
      --arg tree_sha256 "$tree_sha" \
      --argjson capture_evidence "$capture_evidence" \
      --argjson pm2_entry_pid "$CAPTURE_PM2_ENTRY_PID" \
      --argjson pm2_entry_starttime "$CAPTURE_PM2_ENTRY_STARTTIME" \
      --argjson listener_runtime_pid "$CAPTURE_PROCESS_PID" \
      --argjson listener_runtime_starttime "$CAPTURE_LISTENER_STARTTIME" \
      --argjson restart_count "$CAPTURE_RESTART_COUNT" \
      --argjson packet_count "$packet_count" \
      --argjson pinned "$([ -n "$CAPTURE_EXEC" ] && printf true || printf false)" \
      '{
        version: 3,
        flow: $flow,
        side: "rust",
        completed: true,
        created_at: $created_at,
        harness_repo_head: $harness_repo_head,
        source_repo_head: $source_repo_head,
        harness_worktree_clean: true,
        harness_worktree_state_sha256: $harness_worktree_sha256,
        source_worktree_dirty: false,
        source_worktree_state_sha256: $harness_worktree_sha256,
        worktree_state_algorithm: "git-head-path-mode-content-sha256-v1",
        expected_exec_path: $expected_exec_path,
        expected_exec_sha256: $expected_exec_sha256,
        source_exec_path: $source_exec_path,
        source_exec_sha256: $source_exec_sha256,
        live_exec_path: $live_exec_path,
        live_exec_sha256: $live_exec_sha256,
        executable_pin_enforced: $pinned,
        pm2_entry_pid: $pm2_entry_pid,
        pm2_entry_starttime: $pm2_entry_starttime,
        pm2_exec_path: $pm2_exec_path,
        pm2_exec_sha256: $pm2_exec_sha256,
        pm2_profile_redacted_sha256: $pm2_profile_sha256,
        listener_runtime_pid: $listener_runtime_pid,
        listener_runtime_starttime: $listener_runtime_starttime,
        listener_relationship_verified: true,
        restart_count: $restart_count,
        effective_config_path: $effective_config_path,
        effective_config_redacted_sha256: $effective_config_sha256,
        effective_config_algorithm: "capture-relevant-redacted-v1",
        runtime_cleanup_verified: true,
        normal_runtime_restored: true,
        fixture_guard: $capture_evidence.fixture_guard,
        bot_report: $capture_evidence.bot_report,
        artifact: {
          path: "rust",
          packet_count: $packet_count,
          tree_sha256: $tree_sha256
        }
      }' >"$manifest"; then
    rm -f -- "$manifest"
    return 1
  fi
  chmod 600 "$manifest" || return 1
  rust_capture_flat_tree_is_safe "$DUMP_STAGE_DIR" || return 1
  while IFS= read -r -d '' path; do
    sync -f "$path" || return 1
  done < <(find "$DUMP_STAGE_DIR" -mindepth 1 -maxdepth 1 -type f -print0)
  sync -f "$DUMP_STAGE_DIR" || return 1
  capture_require_canonical_directory "$DUMP_PARENT_DIR" \
    && [ ! -e "$DUMP_DIR" ] && [ ! -L "$DUMP_DIR" ] || return 1
  capture_publish_noreplace "$DUMP_STAGE_DIR" "$DUMP_DIR" || return 1
  DUMP_STAGE_DIR=""
  sync -f "$(dirname -- "$DUMP_DIR")" || return 1
  CAPTURE_ARTIFACT_READY=0
  echo "collected ${packet_count} packets -> ${DUMP_DIR}"
  echo "provenance -> ${DUMP_DIR}/rust.capture-manifest.json"
  echo "diff against the C++ golden:"
  echo "  cargo run -p capture-diff -- diff ${FLOW} --rust ${DUMP_DIR}"
}

command -v jq >/dev/null 2>&1 || {
  echo "error: jq is required to snapshot and safely restore the PM2 process" >&2
  exit 1
}
PM2_BIN="$(command -v pm2)" || {
  echo "error: pm2 is required to capture and restore the Rust world" >&2
  exit 1
}
command -v ss >/dev/null 2>&1 || {
  echo "error: ss is required to verify Rust world listener readiness" >&2
  exit 1
}
command -v rg >/dev/null 2>&1 || {
  echo "error: rg is required to verify Rust world listener readiness" >&2
  exit 1
}
for dependency in awk chmod date dirname find flock git id mkdir mktemp mv \
  realpath sed sha256sum sleep sort stat sync wc; do
  command -v "$dependency" >/dev/null 2>&1 || {
    echo "error: required command not found: $dependency" >&2
    exit 1
  }
done
if [ "$RUST_CAPTURE_LOOT_FIXTURE_GUARD" = "1" ]; then
  command -v mysql >/dev/null 2>&1 || {
    echo "error: mysql is required by the loot fixture guard" >&2
    exit 1
  }
  load_loot_fixture_database_credentials || exit 1
fi

echo "flow      : ${FLOW}"
echo "dump dir  : ${DUMP_DIR}"
echo "pm2 world : ${PM2_RUST_WORLD}"
if [ "$RUST_CAPTURE_LOOT_FIXTURE_GUARD" = "1" ]; then
  if [ "$LOOT_FIXTURE_KIND" = "shared-chest" ]; then
    echo "DB fixture : shared chest guid ${LOOT_FIXTURE_CHEST_GUID}, item ${LOOT_FIXTURE_CHEST_ITEM}, money 10"
  else
    echo "DB fixture : entry ${LOOT_FIXTURE_ENTRY}, temporary HealthModifier ${LOOT_FIXTURE_TEMP_HEALTH_MODIFIER}"
  fi
fi
echo
echo "This will restart ${PM2_RUST_WORLD} with RUSTYCORE_PACKET_DUMP_DIR set."

if [ "$CONFIRM" != "--yes" ]; then
  read -r -p "Proceed? [y/N] " ans
  [ "$ans" = "y" ] || [ "$ans" = "Y" ] || { echo "aborted"; exit 1; }
fi

capture_acquire_orchestration_lock "$CAPTURE_ORCHESTRATION_LOCK" || {
  echo "error: another capture/QA process holds ${CAPTURE_ORCHESTRATION_LOCK}" >&2
  exit 1
}
capture_pm2_process_stopped "$PM2_CPP_WORLD" || {
  echo "error: ${PM2_CPP_WORLD} must be one exact stopped PM2 process before Rust capture" >&2
  exit 1
}

# `pm2 restart --update-env` merges the caller's whole environment and does not
# remove a key merely because the caller later unsets it. After confirmation,
# snapshot a supported one-process ecosystem entry, then derive a second config
# that differs only by RUSTYCORE_PACKET_DUMP_DIR. Both temporary files can
# contain environment secrets, so install the cleanup trap immediately, keep
# their mode at 0600, and never pass their environment through argv.
RESTORE_FILE="$(mktemp --suffix=.capture-rust.pm2.json)"
CAPTURE_CONFIG_FILE="$(mktemp --suffix=.capture-rust.pm2.json)"
RESTORE_READY=0
CAPTURE_MUTATED=0

snapshot_process_identity() {
  local snapshot_file="$1"
  pm2 jlist | jq -er \
    --arg name "$PM2_RUST_WORLD" \
    --slurpfile snapshot "$snapshot_file" '
      [.[] | select(.name == $name)] as $running
      | $snapshot[0].apps[0] as $wanted
      | (($wanted.env // {}) | has("RUSTYCORE_PACKET_DUMP_DIR")) as $wants_dump
      | ((($running | length) == 1
        and $running[0].pm2_env.status == "online"
        and ($running[0].pid // 0) > 0
        and $running[0].pm2_env.pm_exec_path == $wanted.script
        and $running[0].pm2_env.pm_cwd == $wanted.cwd
        and ($running[0].pm2_env.exec_interpreter // "none") == $wanted.interpreter
        and ($running[0].pm2_env.args // []) == ($wanted.args // [])
        and $running[0].pm2_env.exec_mode == "fork_mode"
        and $running[0].pm2_env.instances == 1
        and $running[0].pm2_env.autorestart == $wanted.autorestart
        and ($running[0].pm2_env.watch // false) == false
        and (($running[0].pm2_env.node_args // []) | length) == 0
        and ($running[0].pm2_env.restart_delay // 0) == 0
        and ($running[0].pm2_env.exp_backoff_restart_delay // 0) == 0
        and ($running[0].pm2_env.wait_ready // false) == false
        and ($running[0].pm2_env.shutdown_with_message // false) == false
        and ($running[0].pm2_env.max_memory_restart // null) == null
        and ($running[0].pm2_env.cron_restart // null) == null
        and ($running[0].pm2_env.stop_exit_codes // null) == null
        and ($running[0].pm2_env.instance_var // "NODE_APP_INSTANCE") == "NODE_APP_INSTANCE"
        and ($running[0].pm2_env.treekill != false)
        and ($running[0].pm2_env.vizion != false)
        and ($running[0].pm2_env.windowsHide != false)
        and ($running[0].pm2_env.kill_timeout // null) == null
        and ($running[0].pm2_env.listen_timeout // null) == null
        and ($running[0].pm2_env.min_uptime // null) == null
        and ($running[0].pm2_env.max_restarts // null) == null
        and ($running[0].pm2_env.kill_retry_time // 100) == 100
        and ($running[0].pm2_env.source_map_support // null) == null
        and ($running[0].pm2_env.time // false) == false
        and ($running[0].pm2_env.disable_logs // false) == false
        and ($running[0].pm2_env.automation != false)
        and ($running[0].pm2_env.pmx != false)
        and ($running[0].pm2_env.autostart != false)
        and ($running[0].pm2_env.increment_var // null) == null
        and ($running[0].pm2_env.filter_env // []) == ($wanted.filter_env // [])
        and ($running[0].pm2_env.append_env_to_name // false) == false
        and ($running[0].pm2_env.log_type // null) == null
        and ($running[0].pm2_env.log_date_format // null) == null
        and ($running[0].pm2_env.disable_trace // false) == false
        and ($running[0].pm2_env.uid // null) == null
        and ($running[0].pm2_env.gid // null) == null
        and ($running[0].pm2_env.namespace // "default") == $wanted.namespace
        and $running[0].pm2_env.pm_out_log_path == $wanted.out_file
        and $running[0].pm2_env.pm_err_log_path == $wanted.error_file
        and ($running[0].pm2_env.merge_logs // false) == $wanted.merge_logs
        and (if $wants_dump then
               $running[0].pm2_env.RUSTYCORE_PACKET_DUMP_DIR
                 == $wanted.env.RUSTYCORE_PACKET_DUMP_DIR
             else
               (($running[0].pm2_env | has("RUSTYCORE_PACKET_DUMP_DIR")) | not)
             end)
        and (
          (($running[0].pm2_env.env // {})
            | del(
                .unique_id,
                .PM2_JSON_PROCESSING,
                .[$name]
              ))
          == (($wanted.env // {})
            | del(
                .unique_id,
                .PM2_JSON_PROCESSING,
                .[$name]
              ))
        ))) as $matches
      | if $matches then
          [$running[0].pid, ($running[0].pm2_env.restart_time // 0)] | @tsv
        else empty end
    '
}

resolve_rust_profile_effective_config() {
  local snapshot_file="$1"
  local cwd config_arg="" config_dir_arg="" candidate index
  local -a args=()

  cwd="$(jq -er '.apps[0].cwd | select(type == "string" and length > 0)' \
    "$snapshot_file")" || return 1
  jq -e '.apps[0].args | type == "array"' "$snapshot_file" >/dev/null \
    || return 1
  mapfile -t args < <(jq -r '.apps[0].args[]' "$snapshot_file")
  for ((index = 0; index < ${#args[@]}; index++)); do
    case "${args[index]}" in
      -c|--config)
        ((index + 1 < ${#args[@]})) || return 1
        config_arg="${args[index + 1]}"
        index=$((index + 1))
        ;;
      --config=*) config_arg="${args[index]#--config=}" ;;
      -cd|--config-dir)
        ((index + 1 < ${#args[@]})) || return 1
        config_dir_arg="${args[index + 1]}"
        index=$((index + 1))
        ;;
      --config-dir=*) config_dir_arg="${args[index]#--config-dir=}" ;;
    esac
  done

  if [ -z "$config_arg" ]; then
    for candidate in worldserver.conf worldserver.conf.dist WorldServer.conf WorldServer.conf.dist; do
      if [ -f "$cwd/$candidate" ]; then
        config_arg="$cwd/$candidate"
        break
      fi
    done
  elif [[ "$config_arg" != /* ]]; then
    config_arg="$cwd/$config_arg"
  fi
  [ -n "$config_arg" ] || return 1

  if [ -z "$config_dir_arg" ]; then
    config_dir_arg="$cwd/worldserver.conf.d"
  elif [[ "$config_dir_arg" != /* ]]; then
    config_dir_arg="$cwd/$config_dir_arg"
  fi
  if [ -d "$config_dir_arg" ] \
      && find "$config_dir_arg" -type f -name '*.conf' -print -quit | rg -q .; then
    echo "error: guarded capture provenance does not yet support additional config overlays in ${config_dir_arg}" >&2
    return 1
  fi
  if [ "$RUST_CAPTURE_LOOT_FIXTURE_GUARD" = "1" ] \
      && ! jq -e '
        (.apps[0].env // {}) | keys
        | map(select(startswith("TC_"))) | length == 0
      ' "$snapshot_file" >/dev/null; then
    echo "error: guarded capture provenance does not permit unrecorded TC_* config overrides" >&2
    return 1
  fi
  realpath -e -- "$config_arg"
}

rust_world_ports_ready() {
  local identity="$1"
  local pm2_pid listener_pid
  pm2_pid="${identity%%$'\t'*}"
  [[ "$pm2_pid" =~ ^[1-9][0-9]*$ ]] || return 1
  listener_pid="$(capture_world_listener_pid)" || return 1
  capture_pid_is_self_or_descendant "$listener_pid" "$pm2_pid" \
    && capture_world_ports_owned_by_pid "$listener_pid"
}

rust_world_ports_absent() {
  local sockets
  sockets="$(ss -H -ltn)" || return 1
  ! rg -q ":(${RUST_WORLD_PORT}|${RUST_INSTANCE_PORT})\\b" <<<"$sockets"
}

rust_world_pm2_process_absent() {
  pm2 jlist | jq -e --arg name "$PM2_RUST_WORLD" \
    '[.[] | select(.name == $name)] | length == 0' >/dev/null
}

rust_remove_world_and_verify() {
  local process_tree="$1"
  local listener_identity="$2"
  local current_root="" current_tree=""

  pm2 delete "$PM2_RUST_WORLD" >/dev/null 2>&1 || true
  capture_process_tree_absent "$process_tree" \
    || capture_terminate_process_tree "$process_tree" || true
  current_root="$(capture_pm2_online_pid "$PM2_RUST_WORLD" 2>/dev/null || true)"
  if [ -n "$current_root" ]; then
    current_tree="$(capture_process_tree_identity "$current_root" 2>/dev/null || true)"
    process_tree="$(printf '%s\n%s\n' "$process_tree" "$current_tree" \
      | sed '/^$/d' | LC_ALL=C sort -t: -k1,1n -k2,2n -u)"
  fi
  # If PM2 retained an autorestart-capable entry while its old tree was being
  # terminated, delete that registration again before accepting absence.
  pm2 delete "$PM2_RUST_WORLD" >/dev/null 2>&1 || true
  capture_process_tree_absent "$process_tree" \
    || capture_terminate_process_tree "$process_tree" || true
  pm2 delete "$PM2_RUST_WORLD" >/dev/null 2>&1 || true
  capture_process_tree_absent "$process_tree" \
    && { [ -z "$listener_identity" ] \
      || capture_pid_identity_absent "$listener_identity"; } \
    && rust_world_pm2_process_absent \
    && rust_world_ports_absent
}

cleanup() {
  local capture_status=$?
  local restore_status=0
  local current_root="" current_tree="" listener_identity=""
  local process_tree_to_stop="$(printf '%s\n%s\n' \
    "$ORIGINAL_PROCESS_TREE_IDENTITIES" "$CAPTURE_PROCESS_TREE_IDENTITIES" \
    | sed '/^$/d' | LC_ALL=C sort -t: -k1,1n -k2,2n -u)"
  trap - EXIT
  trap '' HUP INT TERM
  if [ "$CAPTURE_MUTATED" -eq 0 ]; then
    rm -f "$RESTORE_FILE" "$CAPTURE_CONFIG_FILE"
    [ -z "$DUMP_STAGE_DIR" ] || rm -rf -- "$DUMP_STAGE_DIR"
    capture_release_orchestration_lock
    exit "$capture_status"
  fi
  echo "recreating ${PM2_RUST_WORLD} without RUSTYCORE_PACKET_DUMP_DIR..."
  set +e
  if [ "$RESTORE_READY" -ne 1 ]; then
    echo "WARNING: PM2 restore snapshot is incomplete; inspect PM2 manually" >&2
    rm -f "$RESTORE_FILE" "$CAPTURE_CONFIG_FILE"
    [ -z "$DUMP_STAGE_DIR" ] || rm -rf -- "$DUMP_STAGE_DIR"
    capture_release_orchestration_lock
    exit "$CAPTURE_RESTORE_FAILURE_STATUS"
  fi
  unset RUSTYCORE_PACKET_DUMP_DIR
  if [ -n "$CAPTURE_PM2_ENTRY_PID" ] \
      && [ -n "$CAPTURE_PM2_ENTRY_STARTTIME" ] \
      && capture_pid_identity_is_live \
        "${CAPTURE_PM2_ENTRY_PID}:${CAPTURE_PM2_ENTRY_STARTTIME}"; then
    current_root="$CAPTURE_PM2_ENTRY_PID"
  else
    current_root="$(capture_pm2_online_pid "$PM2_RUST_WORLD" 2>/dev/null || true)"
  fi
  if [ -n "$current_root" ]; then
    current_tree="$(capture_process_tree_identity "$current_root" 2>/dev/null || true)"
    process_tree_to_stop="$(printf '%s\n%s\n' \
      "$process_tree_to_stop" "$current_tree" \
      | sed '/^$/d' | LC_ALL=C sort -t: -k1,1n -k2,2n -u)"
  fi
  if [ -n "$CAPTURE_PROCESS_PID" ] && [ -n "$CAPTURE_LISTENER_STARTTIME" ]; then
    listener_identity="${CAPTURE_PROCESS_PID}:${CAPTURE_LISTENER_STARTTIME}"
  elif [ -n "$ORIGINAL_LISTENER_PID" ] \
      && [ -n "$ORIGINAL_LISTENER_STARTTIME" ]; then
    listener_identity="${ORIGINAL_LISTENER_PID}:${ORIGINAL_LISTENER_STARTTIME}"
  else
    CAPTURE_PROCESS_PID="$(capture_world_listener_pid 2>/dev/null || true)"
    if [ -n "$CAPTURE_PROCESS_PID" ]; then
      listener_identity="$(capture_pid_identity "$CAPTURE_PROCESS_PID" 2>/dev/null || true)"
      process_tree_to_stop="$(printf '%s\n%s\n' \
        "$process_tree_to_stop" "$listener_identity" \
        | sed '/^$/d' | LC_ALL=C sort -t: -k1,1n -k2,2n -u)"
    fi
  fi
  if ! rust_remove_world_and_verify \
      "$process_tree_to_stop" "$listener_identity"; then
    echo "WARNING: capture PM2 entry/listener/descendant tree is still present or serving; refusing fixture restoration while it may still own runtime state" >&2
    restore_status=1
  else
    CAPTURE_RUNTIME_CLEANUP_VERIFIED=1
  fi
  if [ "$restore_status" -eq 0 ] && ! restore_loot_fixture_guard; then
    echo "WARNING: failed to restore the loot fixture; the normal world will remain stopped" >&2
    restore_status=1
  fi
  if [ "$restore_status" -eq 0 ] \
      && ! loot_fixture_bot_cleanup_safe_for_capture_state \
        "$CAPTURE_BOT_READY"; then
    echo "WARNING: bot fixture cleanup is unproven; the normal world will remain stopped" >&2
    restore_status=1
  fi
  if [ "$restore_status" -eq 0 ] \
      && [ "$RUST_CAPTURE_LOOT_FIXTURE_GUARD" = "1" ]; then
    CAPTURE_FIXTURE_CLEANUP_VERIFIED=1
  fi
  if [ "$restore_status" -eq 0 ] \
      && { [ ! -f "$RESTORE_FILE" ] || [ -L "$RESTORE_FILE" ] \
        || [ "$(sha256_of_file "$RESTORE_FILE" 2>/dev/null)" \
          != "$RESTORE_FILE_SHA256" ]; }; then
    echo "WARNING: PM2 restore snapshot changed or became unsafe; fixture is clean but the normal world will remain stopped" >&2
    restore_status=1
  fi
  if [ "$restore_status" -eq 0 ] && ! env -i \
      HOME="$HOME" \
      PATH="$PATH" \
      PM2_HOME="${PM2_HOME:-$HOME/.pm2}" \
      "$PM2_BIN" start "$RESTORE_FILE" --only "$PM2_RUST_WORLD" >/dev/null 2>&1; then
    restore_status=1
  elif [ "$restore_status" -eq 0 ]; then
    restore_status=1
    local last_identity=""
    local stable_samples=0
    local identity=""
    local restore_deadline=$((SECONDS + CAPTURE_WORLD_READY_TIMEOUT_SECONDS))
    while ((SECONDS < restore_deadline)); do
      if identity="$(snapshot_process_identity "$RESTORE_FILE" 2>/dev/null)" \
          && rust_world_ports_ready "$identity"; then
        if [ "$identity" = "$last_identity" ]; then
          stable_samples=$((stable_samples + 1))
        else
          last_identity="$identity"
          stable_samples=1
        fi
        if [ "$stable_samples" -ge 4 ]; then
          restore_status=0
          CAPTURE_NORMAL_RUNTIME_RESTORED=1
          break
        fi
      else
        last_identity=""
        stable_samples=0
      fi
      sleep 0.5
    done
  fi
  if [ "$restore_status" -ne 0 ]; then
    echo "WARNING: failed to restore ${PM2_RUST_WORLD} exactly; inspect PM2 before another capture" >&2
    echo "WARNING: mode-0600 recovery snapshot retained at ${RESTORE_FILE}" >&2
    rm -f "$CAPTURE_CONFIG_FILE"
    [ -z "$DUMP_STAGE_DIR" ] || rm -rf -- "$DUMP_STAGE_DIR"
    CAPTURE_ARTIFACT_READY=0
    capture_release_orchestration_lock
    exit "$CAPTURE_RESTORE_FAILURE_STATUS"
  fi
  rm -f "$RESTORE_FILE" "$CAPTURE_CONFIG_FILE"
  if [ "$RUST_CAPTURE_LOOT_FIXTURE_GUARD" = "1" ] \
      && ! rm -f -- "$LOOT_FIXTURE_CLEANUP_MARKER"; then
    echo "WARNING: failed to remove the consumed bot cleanup marker" >&2
    restore_status=1
  fi
  if [ "$restore_status" -eq 0 ] && [ "$capture_status" -eq 0 ]; then
    if ! finalize_rust_capture_artifact; then
      echo "WARNING: capture cleanup succeeded, but atomic dump/manifest publication failed" >&2
      restore_status=1
    fi
  fi
  if [ "$restore_status" -ne 0 ] || [ "$capture_status" -ne 0 ]; then
    [ -z "$DUMP_STAGE_DIR" ] || rm -rf -- "$DUMP_STAGE_DIR"
    CAPTURE_ARTIFACT_READY=0
  fi
  capture_release_orchestration_lock
  [ "$restore_status" -eq 0 ] || exit "$CAPTURE_RESTORE_FAILURE_STATUS"
  exit "$capture_status"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

# This harness intentionally supports the repository's documented PM2 profile:
# one online fork-mode process, one instance, no watch mode. Reject a different
# topology before mutating it rather than reconstructing it approximately.
if ! pm2 jlist | jq -e --arg name "$PM2_RUST_WORLD" '
  [.[] | select(.name == $name)] as $matches
  | if ($matches | length) != 1 then
      error("PM2 process must exist exactly once")
    else $matches[0] end
  | if .pm2_env.status != "online"
      or .pm2_env.exec_mode != "fork_mode"
      or .pm2_env.instances != 1
      or (.pm2_env | has("RUSTYCORE_PACKET_DUMP_DIR"))
      or ((.pm2_env.env // {}) | has("RUSTYCORE_PACKET_DUMP_DIR"))
      or (.pm2_env.watch // false) != false
      or ((.pm2_env.node_args // []) | length) != 0
      or (.pm2_env.restart_delay // 0) != 0
      or (.pm2_env.exp_backoff_restart_delay // 0) != 0
      or (.pm2_env.wait_ready // false) != false
      or (.pm2_env.shutdown_with_message // false) != false
      or (.pm2_env.max_memory_restart // null) != null
      or (.pm2_env.cron_restart // null) != null
      or (.pm2_env.stop_exit_codes // null) != null
      or (.pm2_env.instance_var // "NODE_APP_INSTANCE") != "NODE_APP_INSTANCE"
      or (.pm2_env.treekill == false)
      or (.pm2_env.vizion == false)
      or (.pm2_env.windowsHide == false)
      or (.pm2_env.kill_timeout // null) != null
      or (.pm2_env.listen_timeout // null) != null
      or (.pm2_env.min_uptime // null) != null
      or (.pm2_env.max_restarts // null) != null
      or (.pm2_env.kill_retry_time // 100) != 100
      or (.pm2_env.source_map_support // null) != null
      or (.pm2_env.time // false) != false
      or (.pm2_env.disable_logs // false) != false
      or (.pm2_env.automation == false)
      or (.pm2_env.pmx == false)
      or (.pm2_env.autostart == false)
      or (.pm2_env.increment_var // null) != null
      or ((.pm2_env.filter_env // []) | type) != "array"
      or ((.pm2_env.filter_env // []) | length) != 0
      or (.pm2_env.append_env_to_name // false) != false
      or (.pm2_env.log_type // null) != null
      or (.pm2_env.log_date_format // null) != null
      or (.pm2_env.disable_trace // false) != false
      or (.pm2_env.uid // null) != null
      or (.pm2_env.gid // null) != null
      or (.pm2_env.pm_exec_path | type) != "string"
      or (.pm2_env.pm_cwd | type) != "string"
      or ((.pm2_env.env // {}) | type) != "object"
    then error("unsupported PM2 profile")
    else . end
  | .name as $app_name
  | {
      apps: [{
        name: $app_name,
        script: .pm2_env.pm_exec_path,
        cwd: .pm2_env.pm_cwd,
        interpreter: (.pm2_env.exec_interpreter // "none"),
        args: (.pm2_env.args // []),
        env: ((.pm2_env.env // {})
          | del(
              .RUSTYCORE_PACKET_DUMP_DIR,
              .unique_id,
              .PM2_JSON_PROCESSING,
              .[$app_name]
            )),
        exec_mode: "fork",
        instances: 1,
        autorestart: .pm2_env.autorestart,
        watch: false,
        filter_env: (.pm2_env.filter_env // []),
        namespace: (.pm2_env.namespace // "default"),
        out_file: .pm2_env.pm_out_log_path,
        error_file: .pm2_env.pm_err_log_path,
        merge_logs: (.pm2_env.merge_logs // false)
      }]
    }
' >"$RESTORE_FILE"; then
  echo "error: '${PM2_RUST_WORLD}' is missing or uses an unsupported PM2 profile" >&2
  exit 1
fi
[ -s "$RESTORE_FILE" ] || {
  echo "error: failed to create PM2 restore snapshot" >&2
  exit 1
}
PROFILE_EFFECTIVE_CONFIG="$(resolve_rust_profile_effective_config "$RESTORE_FILE")" || {
  echo "error: cannot resolve the exact effective Rust config from the PM2 profile" >&2
  exit 1
}
[ "$PROFILE_EFFECTIVE_CONFIG" = "$CAPTURE_EFFECTIVE_CONFIG_PATH" ] || {
  echo "error: RUST_CAPTURE_EFFECTIVE_CONFIG does not match the config selected by the PM2 cwd/args" >&2
  exit 1
}
capture_require_canonical_directory "$DUMP_PARENT_DIR" \
  && [ ! -e "$DUMP_DIR" ] && [ ! -L "$DUMP_DIR" ] || {
  echo "error: Rust capture output path changed or became unsafe" >&2
  exit 1
}
DUMP_STAGE_DIR="$(mktemp -d "${DUMP_DIR}.partial.XXXXXX")" || {
  echo "error: failed to create private Rust capture staging directory" >&2
  exit 1
}
chmod 700 "$DUMP_STAGE_DIR" || {
  echo "error: failed to make Rust capture staging directory private" >&2
  exit 1
}
if ! jq \
    --arg dump_dir "$DUMP_STAGE_DIR" \
    --arg capture_exec "$CAPTURE_EXEC" '
      .apps[0].env.RUSTYCORE_PACKET_DUMP_DIR = $dump_dir
      | if $capture_exec == "" then .
        else .apps[0].script = $capture_exec
        end
    ' "$RESTORE_FILE" >"$CAPTURE_CONFIG_FILE"; then
  echo "error: failed to create PM2 capture snapshot" >&2
  exit 1
fi
[ -s "$CAPTURE_CONFIG_FILE" ] || {
  echo "error: failed to create PM2 capture snapshot" >&2
  exit 1
}
chmod 600 "$RESTORE_FILE" "$CAPTURE_CONFIG_FILE" || {
  echo "error: failed to keep PM2 snapshots private" >&2
  exit 1
}
RESTORE_FILE_SHA256="$(sha256_of_file "$RESTORE_FILE")" || exit 1
CAPTURE_CONFIG_FILE_SHA256="$(sha256_of_file "$CAPTURE_CONFIG_FILE")" || exit 1
RESTORE_READY=1

# Recheck immediately before touching either server. The earlier validation is
# intentionally repeated because confirmation and snapshotting may take time.
if ! capture_exec_source_matches; then
  echo "error: RUST_CAPTURE_EXEC changed or no longer matches its pinned SHA-256" >&2
  exit 1
fi
capture_git_repo_clean_at_head "$REPO_ROOT" "$CAPTURE_HARNESS_REPO_HEAD" \
  && [ "$(capture_git_worktree_state_sha256 "$REPO_ROOT")" \
    = "$CAPTURE_HARNESS_WORKTREE_SHA256" ] || {
  echo "error: RustyCore harness/source worktree changed before service mutation" >&2
  exit 1
}
[ "$(sha256_of_file "$RESTORE_FILE")" = "$RESTORE_FILE_SHA256" ] \
  && [ "$(sha256_of_file "$CAPTURE_CONFIG_FILE")" \
    = "$CAPTURE_CONFIG_FILE_SHA256" ] || {
  echo "error: PM2 capture/restore snapshot changed before service mutation" >&2
  exit 1
}

# Revalidate the mutually exclusive C++ service state immediately before the
# first PM2/fixture mutation. Never silently stop an unexpected C++ process.
capture_pm2_process_stopped "$PM2_CPP_WORLD" || {
  echo "error: ${PM2_CPP_WORLD} changed state before Rust capture mutation" >&2
  exit 1
}

echo "recreating ${PM2_RUST_WORLD} from the clean snapshot with dump enabled..."
ORIGINAL_SNAPSHOT_IDENTITY="$(snapshot_process_identity "$RESTORE_FILE")" || {
  echo "error: original Rust PM2 profile changed before capture mutation" >&2
  exit 1
}
rust_world_ports_ready "$ORIGINAL_SNAPSHOT_IDENTITY" || {
  echo "error: original Rust PM2 profile is not the sole owner of both listeners" >&2
  exit 1
}
ORIGINAL_PM2_ENTRY_PID="${ORIGINAL_SNAPSHOT_IDENTITY%%$'\t'*}"
ORIGINAL_LISTENER_PID="$(capture_world_listener_pid)" || {
  echo "error: cannot discover the original Rust listener PID" >&2
  exit 1
}
[[ "$ORIGINAL_PM2_ENTRY_PID" =~ ^[1-9][0-9]*$ \
  && "$ORIGINAL_LISTENER_PID" =~ ^[1-9][0-9]*$ ]] \
  && capture_pid_is_self_or_descendant \
    "$ORIGINAL_LISTENER_PID" "$ORIGINAL_PM2_ENTRY_PID" || {
  echo "error: original Rust PM2 entry/listener relationship is invalid" >&2
  exit 1
}
ORIGINAL_PM2_ENTRY_STARTTIME="$(
  capture_pid_starttime "$ORIGINAL_PM2_ENTRY_PID"
)" || {
  echo "error: cannot bind the original PM2 entry PID to its start time" >&2
  exit 1
}
ORIGINAL_LISTENER_STARTTIME="$(
  capture_pid_starttime "$ORIGINAL_LISTENER_PID"
)" || {
  echo "error: cannot bind the original listener PID to its start time" >&2
  exit 1
}
ORIGINAL_PROCESS_TREE_IDENTITIES="$(
  capture_process_tree_identity "$ORIGINAL_PM2_ENTRY_PID"
)" || {
  echo "error: cannot accredit the original Rust process tree" >&2
  exit 1
}
CAPTURE_MUTATED=1
rust_remove_world_and_verify \
  "$ORIGINAL_PROCESS_TREE_IDENTITIES" \
  "${ORIGINAL_LISTENER_PID}:${ORIGINAL_LISTENER_STARTTIME}" || {
  echo "error: original Rust PM2 entry/listener/descendants did not stop; refusing fixture mutation" >&2
  exit 1
}
apply_loot_fixture_guard
env -i \
  HOME="$HOME" \
  PATH="$PATH" \
  PM2_HOME="${PM2_HOME:-$HOME/.pm2}" \
  "$PM2_BIN" start "$CAPTURE_CONFIG_FILE" --only "$PM2_RUST_WORLD" >/dev/null

CAPTURE_READY=0
CAPTURE_IDENTITY=""
CAPTURE_LAST_IDENTITY=""
CAPTURE_STABLE_SAMPLES=0
CAPTURE_START_DEADLINE=$((SECONDS + CAPTURE_WORLD_READY_TIMEOUT_SECONDS))
while ((SECONDS < CAPTURE_START_DEADLINE)); do
  if CAPTURE_IDENTITY="$(snapshot_process_identity "$CAPTURE_CONFIG_FILE" 2>/dev/null)" \
      && rust_world_ports_ready "$CAPTURE_IDENTITY" \
      && capture_process_exec_matches "$CAPTURE_IDENTITY"; then
    if [ "$CAPTURE_IDENTITY" = "$CAPTURE_LAST_IDENTITY" ]; then
      CAPTURE_STABLE_SAMPLES=$((CAPTURE_STABLE_SAMPLES + 1))
    else
      CAPTURE_LAST_IDENTITY="$CAPTURE_IDENTITY"
      CAPTURE_STABLE_SAMPLES=1
    fi
    if [ "$CAPTURE_STABLE_SAMPLES" -ge 4 ]; then
      CAPTURE_READY=1
      break
    fi
  else
    CAPTURE_LAST_IDENTITY=""
    CAPTURE_STABLE_SAMPLES=0
  fi
  sleep 0.5
done
[ "$CAPTURE_READY" -eq 1 ] || {
  echo "error: ${PM2_RUST_WORLD} did not start online with the pinned capture executable and packet dumping enabled" >&2
  exit 1
}
CAPTURE_PM2_ENTRY_PID="${CAPTURE_IDENTITY%%$'\t'*}"
CAPTURE_PROCESS_PID="$(capture_world_listener_pid)" || {
  echo "error: cannot discover the unique Rust world/instance listener PID" >&2
  exit 1
}
[[ "$CAPTURE_PM2_ENTRY_PID" =~ ^[1-9][0-9]*$ \
  && "$CAPTURE_PROCESS_PID" =~ ^[1-9][0-9]*$ ]] \
  && capture_pid_is_self_or_descendant \
    "$CAPTURE_PROCESS_PID" "$CAPTURE_PM2_ENTRY_PID" || {
  echo "error: capture PM2 entry/listener process relationship is invalid" >&2
  exit 1
}
CAPTURE_LIVE_EXEC="$(realpath -e -- "/proc/${CAPTURE_PROCESS_PID}/exe" 2>/dev/null)" \
  || {
    echo "error: cannot resolve the live Rust capture executable" >&2
    exit 1
  }
CAPTURE_LIVE_SHA256="$(sha256_of_file "/proc/${CAPTURE_PROCESS_PID}/exe")" || {
  echo "error: cannot hash the live Rust capture executable" >&2
  exit 1
}
capture_live_exec_matches \
  "$CAPTURE_PROCESS_PID" "$CAPTURE_LIVE_EXEC" "$CAPTURE_LIVE_SHA256" || {
    echo "error: live Rust capture executable provenance is unstable" >&2
    exit 1
  }
CAPTURE_RESTART_COUNT="${CAPTURE_IDENTITY#*$'\t'}"
[[ "$CAPTURE_RESTART_COUNT" =~ ^[0-9]+$ ]] || {
  echo "error: capture PM2 restart count is invalid" >&2
  exit 1
}
CAPTURE_PM2_METADATA="$(
  capture_pm2_entrypoint_identity "$PM2_RUST_WORLD" "$CAPTURE_PM2_ENTRY_PID"
)" || {
  echo "error: cannot accredit Rust PM2 entrypoint metadata" >&2
  exit 1
}
IFS=$'\t' read -r _pm2_pid _pm2_restart \
  CAPTURE_PM2_EXEC_PATH CAPTURE_PM2_EXEC_SHA256 <<<"$CAPTURE_PM2_METADATA"
[ "$_pm2_pid" = "$CAPTURE_PM2_ENTRY_PID" ] \
  && [ "$_pm2_restart" = "$CAPTURE_RESTART_COUNT" ] || {
    echo "error: Rust PM2 entry PID/restart metadata is inconsistent" >&2
    exit 1
  }
CAPTURE_PM2_ENTRY_STARTTIME="$(capture_pid_starttime "$CAPTURE_PM2_ENTRY_PID")" \
  || {
    echo "error: cannot bind Rust PM2 entry PID to its process start time" >&2
    exit 1
  }
CAPTURE_LISTENER_STARTTIME="$(capture_pid_starttime "$CAPTURE_PROCESS_PID")" \
  || {
    echo "error: cannot bind Rust listener PID to its process start time" >&2
    exit 1
  }
CAPTURE_PM2_PROFILE_SHA256="$(
  capture_pm2_profile_redacted_sha256 "$PM2_RUST_WORLD"
)" || {
  echo "error: cannot hash the stable redacted Rust capture PM2 profile" >&2
  exit 1
}
CAPTURE_PROCESS_TREE_IDENTITIES="$(
  capture_process_tree_identity "$CAPTURE_PM2_ENTRY_PID"
)" || {
  echo "error: cannot accredit the Rust capture process tree" >&2
  exit 1
}
if [ -z "$CAPTURE_EXEC" ]; then
  CAPTURE_EXPECTED_EXEC="$CAPTURE_LIVE_EXEC"
  CAPTURE_EXPECTED_SHA256="$CAPTURE_LIVE_SHA256"
  CAPTURE_SOURCE_EXEC="$CAPTURE_LIVE_EXEC"
  CAPTURE_SOURCE_SHA256="$CAPTURE_LIVE_SHA256"
fi

CAPTURE_BOT_READY=1
echo
echo ">>> Perform the '${FLOW}' flow with the client now."
read -r -p ">>> Press ENTER when the flow is complete to finish the capture... " _

FINAL_CAPTURE_IDENTITY=""
if ! FINAL_CAPTURE_IDENTITY="$(snapshot_process_identity "$CAPTURE_CONFIG_FILE" 2>/dev/null)" \
    || ! rust_world_ports_ready "$FINAL_CAPTURE_IDENTITY" \
    || ! capture_process_exec_matches "$FINAL_CAPTURE_IDENTITY"; then
  echo "error: ${PM2_RUST_WORLD} changed configuration, executable provenance, or stopped serving during capture" >&2
  exit 1
fi
if [ "$FINAL_CAPTURE_IDENTITY" != "$CAPTURE_IDENTITY" ]; then
  echo "error: ${PM2_RUST_WORLD} restarted during capture; refusing a mixed packet dump" >&2
  exit 1
fi
capture_live_exec_matches \
  "$CAPTURE_PROCESS_PID" "$CAPTURE_LIVE_EXEC" "$CAPTURE_LIVE_SHA256" || {
    echo "error: ${PM2_RUST_WORLD} executable path/bytes changed during capture" >&2
    exit 1
  }
[ "$(capture_pm2_entrypoint_identity \
  "$PM2_RUST_WORLD" "$CAPTURE_PM2_ENTRY_PID")" \
  = "$CAPTURE_PM2_METADATA" ] \
  && [ "$(capture_world_listener_pid)" = "$CAPTURE_PROCESS_PID" ] \
  && capture_pid_is_self_or_descendant \
    "$CAPTURE_PROCESS_PID" "$CAPTURE_PM2_ENTRY_PID" \
  && [ "$(capture_pid_starttime "$CAPTURE_PM2_ENTRY_PID")" \
    = "$CAPTURE_PM2_ENTRY_STARTTIME" ] \
  && [ "$(capture_pid_starttime "$CAPTURE_PROCESS_PID")" \
    = "$CAPTURE_LISTENER_STARTTIME" ] \
  && [ "$(capture_pm2_profile_redacted_sha256 "$PM2_RUST_WORLD")" \
    = "$CAPTURE_PM2_PROFILE_SHA256" ] || {
    echo "error: ${PM2_RUST_WORLD} PM2 entry/listener metadata changed during capture" >&2
    exit 1
  }
[ "$(rust_capture_effective_config_sha256)" = "$CAPTURE_EFFECTIVE_CONFIG_SHA256" ] || {
  echo "error: effective Rust capture configuration changed during capture" >&2
  exit 1
}
[ "$(sha256_of_file "$CAPTURE_CONFIG_FILE")" \
  = "$CAPTURE_CONFIG_FILE_SHA256" ] || {
  echo "error: PM2 capture snapshot changed during capture" >&2
  exit 1
}

CURRENT_CAPTURE_TREE="$(capture_process_tree_identity "$CAPTURE_PM2_ENTRY_PID")" || {
  echo "error: cannot re-accredit the Rust capture process tree" >&2
  exit 1
}
CAPTURE_PROCESS_TREE_IDENTITIES="$(printf '%s\n%s\n' \
  "$CAPTURE_PROCESS_TREE_IDENTITIES" "$CURRENT_CAPTURE_TREE" \
  | sed '/^$/d' | LC_ALL=C sort -t: -k1,1n -k2,2n -u)"

rust_capture_flat_tree_is_safe "$DUMP_STAGE_DIR" || {
  echo "error: Rust packet dump contains a symlink, subdirectory, special file, or unexpected filename" >&2
  exit 1
}
COUNT=$(find "$DUMP_STAGE_DIR" -mindepth 1 -maxdepth 1 \
  -type f -name '*.meta' -print | wc -l)
[[ "$COUNT" =~ ^[0-9]+$ ]] || {
  echo "error: failed to count staged Rust packets" >&2
  exit 1
}
CAPTURE_ARTIFACT_READY=1
echo "${COUNT} packet(s) staged; dump and provenance publish only after guarded cleanup succeeds"
