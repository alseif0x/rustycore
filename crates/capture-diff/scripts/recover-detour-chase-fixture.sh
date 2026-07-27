#!/usr/bin/env bash
# Explicit crash recovery for detour-chase-around-obstacle.
#
# Usage:
#   WOW_BOT_FIXTURE_JOURNAL=/absolute/private/fixture.journal \
#     crates/capture-diff/scripts/recover-detour-chase-fixture.sh
#
# The journal carries the exact side, DB config, PM2 names/ports, orchestration
# lock, private DataDir identities, config backups, and PM2 restore snapshot.
# No DB or filesystem mutation occurs before the same capture lock is held.
set -euo pipefail

SCRIPT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WOW_BOT_FIXTURE_JOURNAL="${WOW_BOT_FIXTURE_JOURNAL:-}"
LOOT_FIXTURE_CLEANUP_MARKER="${WOW_BOT_FIXTURE_JOURNAL}.cleanup-complete"
LOOT_FIXTURE_GUARD_ENABLED=1
LOOT_FIXTURE_DB_CONF=""
CAPTURE_ORCHESTRATION_LOCK_FD=""

# shellcheck source=loot-fixture-common.sh
source "$SCRIPT_ROOT/loot-fixture-common.sh"
# shellcheck source=capture-service-common.sh
source "$SCRIPT_ROOT/capture-service-common.sh"
# shellcheck source=detour-chase-fixture-common.sh
source "$SCRIPT_ROOT/detour-chase-fixture-common.sh"

detour_recovery_validate_existing_path() {
  local parent canonical owner mode
  [[ "$WOW_BOT_FIXTURE_JOURNAL" = /* \
    && "$WOW_BOT_FIXTURE_JOURNAL" != *$'\n'* ]] || {
      echo "error: WOW_BOT_FIXTURE_JOURNAL must be an absolute single-line path" >&2
      return 1
    }
  parent="$(dirname -- "$WOW_BOT_FIXTURE_JOURNAL")"
  canonical="$(realpath -e -- "$parent" 2>/dev/null)" || return 1
  owner="$(stat -c '%u' -- "$parent")" || return 1
  mode="$(stat -c '%a' -- "$parent")" || return 1
  [ "$canonical" = "$parent" ] && [ ! -L "$parent" ] \
    && [ "$owner" = "$(id -u)" ] && [ "$mode" = 700 ]
}

detour_recovery_pm2_entry_absent() {
  local name="$1"
  pm2 jlist | jq -e --arg name "$name" \
    '[.[] | select(.name == $name)] | length == 0' >/dev/null
}

detour_chase_recovery_runtime_is_normal() {
  local mode="${1:-wait}"
  local identity profile config
  capture_pm2_process_stopped "$DETOUR_FIXTURE_PM2_CPP_WORLD" || return 1
  case "$mode" in
    probe)
      identity="$(capture_world_ready_once \
        "$DETOUR_FIXTURE_PM2_RUST_WORLD" 2>/dev/null)" || return 1
      ;;
    wait)
      identity="$(capture_wait_for_world_ready \
        "$DETOUR_FIXTURE_PM2_RUST_WORLD" 2>/dev/null)" || return 1
      ;;
    *) return 1 ;;
  esac
  [ -n "$identity" ] || return 1
  profile="$(capture_pm2_profile_redacted_sha256 \
    "$DETOUR_FIXTURE_PM2_RUST_WORLD")" || return 1
  config="$(capture_pm2_effective_config_path \
    "$DETOUR_FIXTURE_PM2_RUST_WORLD")" || return 1
  [ "$profile" = "$DETOUR_FIXTURE_NORMAL_RUST_PM2_PROFILE_SHA256" ] \
    && [ "$config" = "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG" ] \
    && pm2 jlist | jq -e --arg name "$DETOUR_FIXTURE_PM2_RUST_WORLD" '
      [.[] | select(.name == $name)] as $entries
      | ($entries | length) == 1
        and (($entries[0].pm2_env | has("RUSTYCORE_PACKET_DUMP_DIR")) | not)
        and (((($entries[0].pm2_env.env // {})
          | has("RUSTYCORE_PACKET_DUMP_DIR"))) | not)
    ' >/dev/null
}

detour_chase_recovery_stop_capture_runtime() {
  local identity="" root="" tree="" listener="" listener_identity=""
  local profile="" config=""

  case "$DETOUR_FIXTURE_SIDE" in
    cpp)
      capture_pm2_process_stopped "$DETOUR_FIXTURE_PM2_RUST_WORLD" || {
        echo "error: refusing C++ recovery because the Rust PM2 entry is not stopped" >&2
        return 1
      }
      if capture_pm2_process_stopped "$DETOUR_FIXTURE_PM2_CPP_WORLD" \
          && capture_world_ports_absent; then
        return 0
      fi
      identity="$(capture_wait_for_world_ready \
        "$DETOUR_FIXTURE_PM2_CPP_WORLD")" || {
          echo "error: C++ capture runtime is neither safely ready nor stopped" >&2
          return 1
        }
      [ -f "$DETOUR_FIXTURE_CPP_CONFIG" ] \
        && [ ! -L "$DETOUR_FIXTURE_CPP_CONFIG" ] \
        && [ "$(detour_chase_read_config_value \
          "$DETOUR_FIXTURE_CPP_CONFIG" DataDir)" \
          = "${DETOUR_FIXTURE_PRIVATE_DATA_DIR}/" ] || {
          echo "error: online C++ process is not accredited to the journaled private fixture config" >&2
          return 1
        }
      pm2 stop "$DETOUR_FIXTURE_PM2_CPP_WORLD" >/dev/null 2>&1 || return 1
      capture_wait_for_world_stopped \
        "$DETOUR_FIXTURE_PM2_CPP_WORLD" "$identity"
      ;;
    rust)
      capture_pm2_process_stopped "$DETOUR_FIXTURE_PM2_CPP_WORLD" || {
        echo "error: refusing Rust recovery because the C++ PM2 entry is not stopped" >&2
        return 1
      }
      [ -n "$DETOUR_FIXTURE_RUST_CONFIG" ] || {
        echo "error: early filesystem journal proves no capture-runtime mutation; refusing to terminate an unaccredited Rust process" >&2
        return 1
      }
      if capture_world_ports_absent; then
        pm2 delete "$DETOUR_FIXTURE_PM2_RUST_WORLD" >/dev/null 2>&1 || true
        detour_recovery_pm2_entry_absent "$DETOUR_FIXTURE_PM2_RUST_WORLD" \
          && capture_world_ports_absent
        return
      fi
      identity="$(capture_wait_for_world_ready \
        "$DETOUR_FIXTURE_PM2_RUST_WORLD")" || {
          echo "error: Rust capture runtime is neither safely ready nor stopped" >&2
          return 1
        }
      profile="$(capture_pm2_profile_redacted_sha256 \
        "$DETOUR_FIXTURE_PM2_RUST_WORLD")" || return 1
      config="$(capture_pm2_effective_config_path \
        "$DETOUR_FIXTURE_PM2_RUST_WORLD")" || return 1
      [ "$profile" != "$DETOUR_FIXTURE_NORMAL_RUST_PM2_PROFILE_SHA256" ] \
        && [ "$config" = "$DETOUR_FIXTURE_RUST_CONFIG" ] \
        && pm2 jlist | jq -e \
          --arg name "$DETOUR_FIXTURE_PM2_RUST_WORLD" '
            [.[] | select(.name == $name)] as $entries
            | ($entries | length) == 1
              and (
                ($entries[0].pm2_env | has("RUSTYCORE_PACKET_DUMP_DIR"))
                or (($entries[0].pm2_env.env // {})
                  | has("RUSTYCORE_PACKET_DUMP_DIR"))
              )
          ' >/dev/null || {
            echo "error: online Rust process is not the journaled capture profile; refusing termination" >&2
            return 1
          }
      root="${identity%%$'\t'*}"
      listener="${identity#*$'\t'}"
      tree="$(capture_process_tree_identity "$root")" || return 1
      listener_identity="$(capture_pid_identity "$listener")" || return 1
      pm2 delete "$DETOUR_FIXTURE_PM2_RUST_WORLD" >/dev/null 2>&1 || true
      capture_process_tree_absent "$tree" \
        || capture_terminate_process_tree "$tree" || return 1
      capture_pid_identity_absent "$listener_identity" || return 1
      detour_recovery_pm2_entry_absent "$DETOUR_FIXTURE_PM2_RUST_WORLD" \
        && capture_world_ports_absent
      ;;
    *) return 1 ;;
  esac
}

detour_recovery_remove_exact_file() {
  local path="$1"
  local identity="$2"
  local digest="$3"
  [ -n "$path" ] || return 0
  if [ ! -e "$path" ] && [ ! -L "$path" ]; then
    return 0
  fi
  [ -f "$path" ] && [ ! -L "$path" ] \
    && [ "$(stat -c '%d:%i' -- "$path")" = "$identity" ] \
    && [ "$(detour_chase_sha256_of_file "$path")" = "$digest" ] || return 1
  rm -- "$path"
}

detour_chase_recovery_restore_filesystem() {
  case "$DETOUR_FIXTURE_SIDE" in
    cpp)
      if [ -e "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP" ] \
          || [ -L "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP" ]; then
        [ -f "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP" ] \
          && [ ! -L "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP" ] \
          && [ "$(stat -c '%d:%i' -- \
            "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP")" \
            = "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP_IDENTITY" ] \
          && [ "$(detour_chase_sha256_of_file \
            "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP")" \
            = "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP_SHA256" ] || return 1
        mv -f -- "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP" \
          "$DETOUR_FIXTURE_CPP_CONFIG" || return 1
      fi
      [ -f "$DETOUR_FIXTURE_CPP_CONFIG" ] \
        && [ ! -L "$DETOUR_FIXTURE_CPP_CONFIG" ] \
        && [ "$(stat -c '%d:%i' -- "$DETOUR_FIXTURE_CPP_CONFIG")" \
          = "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP_IDENTITY" ] \
        && [ "$(detour_chase_sha256_of_file "$DETOUR_FIXTURE_CPP_CONFIG")" \
          = "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP_SHA256" ] || return 1
      ;;
    rust)
      if [ "$DETOUR_FIXTURE_DB_APPLIED" = 0 ] \
          && [ -z "$DETOUR_FIXTURE_RUST_CONFIG" ]; then
        # The first journal is published before any credential-bearing copy
        # and before the PM2 JSON files are populated. A crash can leave
        # unknown partial bytes, but only in these inode-pinned files/root.
        detour_chase_discard_uncheckpointed_rust_artifacts
        return
      fi
      detour_chase_remove_rust_capture_config || return 1
      detour_chase_remove_rust_pm2_capture_file || return 1
      ;;
    *) return 1 ;;
  esac
  if [ "$DETOUR_FIXTURE_DB_APPLIED" = 1 ]; then
    detour_chase_remove_private_data_dir
  else
    detour_chase_discard_unarmed_private_data_dir
  fi
}

detour_chase_recovery_start_normal_runtime() {
  case "$DETOUR_FIXTURE_SIDE" in
    cpp)
      pm2 start "$DETOUR_FIXTURE_PM2_RUST_WORLD" >/dev/null 2>&1 || return 1
      ;;
    rust)
      [ -f "$DETOUR_FIXTURE_PM2_RESTORE_FILE" ] \
        && [ ! -L "$DETOUR_FIXTURE_PM2_RESTORE_FILE" ] \
        && [ "$(stat -c '%a' -- "$DETOUR_FIXTURE_PM2_RESTORE_FILE")" = 600 ] \
        && [ "$(stat -c '%d:%i' -- "$DETOUR_FIXTURE_PM2_RESTORE_FILE")" \
          = "$DETOUR_FIXTURE_PM2_RESTORE_FILE_IDENTITY" ] \
        && [ "$(detour_chase_sha256_of_file \
          "$DETOUR_FIXTURE_PM2_RESTORE_FILE")" \
          = "$DETOUR_FIXTURE_PM2_RESTORE_FILE_SHA256" ] || return 1
      env -i \
        HOME="$HOME" \
        PATH="$PATH" \
        PM2_HOME="${PM2_HOME:-$HOME/.pm2}" \
        pm2 start "$DETOUR_FIXTURE_PM2_RESTORE_FILE" \
          --only "$DETOUR_FIXTURE_PM2_RUST_WORLD" >/dev/null 2>&1 || return 1
      ;;
    *) return 1 ;;
  esac
  detour_chase_recovery_runtime_is_normal
}

detour_recovery_validate_existing_path || exit 2
if [ ! -e "$WOW_BOT_FIXTURE_JOURNAL" ] \
    && [ -f "$LOOT_FIXTURE_CLEANUP_MARKER" ] \
    && [ ! -L "$LOOT_FIXTURE_CLEANUP_MARKER" ]; then
  loot_fixture_bot_cleanup_complete
  rm -- "$LOOT_FIXTURE_CLEANUP_MARKER"
  echo "detour recovery: already complete; consumed retained cleanup marker"
  exit 0
fi
[ -f "$WOW_BOT_FIXTURE_JOURNAL" ] \
  && [ ! -L "$WOW_BOT_FIXTURE_JOURNAL" ] \
  && [ "$(stat -c '%a' -- "$WOW_BOT_FIXTURE_JOURNAL")" = 600 ] || {
    echo "error: no safe mode-0600 detour recovery journal exists" >&2
    exit 2
  }

DETOUR_FIXTURE_SIDE=""
detour_chase_load_fixture_journal || {
  echo "error: detour recovery journal failed schema validation" >&2
  exit 2
}
PRELOCK_JOURNAL_SHA256="$DETOUR_FIXTURE_JOURNAL_SHA256"
CAPTURE_WORLD_PORT="$DETOUR_FIXTURE_WORLD_PORT"
CAPTURE_INSTANCE_PORT="$DETOUR_FIXTURE_INSTANCE_PORT"
CAPTURE_ORCHESTRATION_LOCK="$DETOUR_FIXTURE_ORCHESTRATION_LOCK"
capture_validate_world_timeouts || exit 2
capture_acquire_orchestration_lock "$CAPTURE_ORCHESTRATION_LOCK" || {
  echo "error: another capture/QA process holds ${CAPTURE_ORCHESTRATION_LOCK}" >&2
  exit 1
}
trap capture_release_orchestration_lock EXIT

DETOUR_FIXTURE_SIDE=""
detour_chase_load_fixture_journal \
  && [ "$DETOUR_FIXTURE_JOURNAL_SHA256" = "$PRELOCK_JOURNAL_SHA256" ] || {
    echo "error: detour recovery journal changed while acquiring the lock" >&2
    exit 1
  }
detour_chase_run_recovery_state_machine || {
  echo "error: detour recovery stopped fail-closed; journal retained" >&2
  exit 1
}
detour_chase_recovery_runtime_is_normal || {
  echo "error: normal runtime failed post-recovery accreditation" >&2
  exit 1
}
if [ "$DETOUR_FIXTURE_SIDE" = rust ]; then
  detour_recovery_remove_exact_file \
    "$DETOUR_FIXTURE_PM2_RESTORE_FILE" \
    "$DETOUR_FIXTURE_PM2_RESTORE_FILE_IDENTITY" \
    "$DETOUR_FIXTURE_PM2_RESTORE_FILE_SHA256" || {
      echo "error: recovered runtime is online but PM2 recovery snapshot cleanup failed" >&2
      exit 1
    }
fi
rm -- "$LOOT_FIXTURE_CLEANUP_MARKER"
echo "detour recovery: DB/filesystem restored, normal Rust accredited, journal consumed"
