#!/usr/bin/env bash
# capture-cpp.sh — record a C++ TrinityCore golden capture for one flow.
#
# Swaps the running RustyCore world server for the legacy C++ world server (they
# share DBs and ports), enables the C++ PKT packet log, pauses for you to perform
# the flow with a client, then collects the .pkt and restores the Rust server.
#
# Usage:   crates/capture-diff/scripts/capture-cpp.sh <flow> [--yes]
# Output:  target/captures/<flow>/cpp.pkt   (gitignored)
#          target/captures/<flow>/cpp.capture-manifest.json
#
# Honored env vars (defaults target this machine's layout):
#   CPP_RUNTIME_DIR directory the PM2 wrapper enters before worldserver starts
#                   (default: trinity-legacy-install/bin)
#   CPP_CONF        active legacy worldserver.conf
#                   (default: $CPP_RUNTIME_DIR/worldserver.conf)
#   CPP_LOGS_DIR    PacketLogFile output directory. Defaults to LogsDir from
#                   CPP_CONF, resolved under CPP_RUNTIME_DIR when relative;
#                   an empty LogsDir means CPP_RUNTIME_DIR, matching C++.
#   PM2_CPP_WORLD   pm2 name of the C++ world  (default: cpp-world)
#   PM2_RUST_WORLD  pm2 name of the Rust world (default: rustycore-world)
#   CPP_WORLD_PORT  shared realm listener port (default: 8085)
#   CPP_INSTANCE_PORT shared instance listener port (default: 8086)
#   CPP_CAPTURE_EXEC absolute canonical legacy worldserver executable. It and
#                   CPP_CAPTURE_EXEC_SHA256 are mandatory for the required
#                   loot-single-item-claim evidence flow
#   CPP_CAPTURE_EXEC_SHA256 expected 64-hex SHA-256. The source file and live
#                   /proc/<pid>/exe are checked before and after the flow
#   CPP_CAPTURE_SOURCE_REPO clean legacy C++ source checkout whose exact HEAD
#                   is recorded (default: /home/server/woltk-trinity-legacy)
#   CAPTURE_ORCHESTRATION_LOCK optional absolute private lock directory shared
#                   with capture-rust.sh (default: /tmp, keyed by uid+ports)
#   CAPTURE_WORLD_STOP_TIMEOUT_SECONDS bounded wait for a stopped world/ports
#                   (default: 30, range: 1 through 3600)
#   CAPTURE_WORLD_READY_TIMEOUT_SECONDS bounded wait for a stable ready world
#                   (default: 180, range: 3 through 3600)
#   CPP_CAPTURE_LOOT_FIXTURE_GUARD set to 1 only for the versioned
#                   loot-single-item-claim fixture. It CAS-lowers Doctor
#                   Maleficus 21779 HealthModifier while both worlds are
#                   stopped and restores it before normal Rust PM2 resumes
#   CPP_CAPTURE_ACK_LOOT_FIXTURE_MUTATION must be 1 with the fixture guard
#   CPP_CAPTURE_DB_CONF worldserver.conf containing WorldDatabaseInfo and
#                   CharacterDatabaseInfo (default: CPP_CONF)
#   WOW_BOT_FIXTURE_JOURNAL absolute bot recovery-journal path. Guarded
#                   capture will not restart normal Rust until the journal is
#                   gone and its mode-0600 cleanup marker validates
#   WOW_BOT_EXEC / WOW_BOT_EXEC_SHA256 pinned bot executable used for required
#                   loot-single-item-claim and vendor evidence
#   WOW_BOT_REPORT  fresh absolute bot JSON report path for that exact flow;
#                   mandatory for vendor-extended-cost-purchase
#   DETOUR_CAPTURE_ACK_FIXTURE_MUTATION must be 1 for
#                   detour-chase-around-obstacle. That flow uses the bot
#                   journal path as a shell-owned DB recovery journal and a
#                   private synthetic-MMap DataDir
#   CREATURE_SPELL_CAPTURE_ACK_FIXTURE_MUTATION must be 1 for
#                   creature-spell-casting. The wrapper journals and
#                   CAS-switches Cabal Interrogator 22378 from SmartAI to
#                   CombatAI only while both worlds are stopped
#   CREATURE_SPELL_FIXTURE_JOURNAL fresh absolute recovery-journal path in a
#                   canonical mode-0700 directory. Cleanup replaces it with a
#                   hash-bound marker before normal Rust resumes
#
# This stops the live RustyCore world server (disconnecting players). It refuses
# to run without confirmation; pass --yes to skip the prompt.
set -euo pipefail

FLOW="${1:-}"
[ -n "$FLOW" ] || { echo "usage: $0 <flow> [--yes]" >&2; exit 2; }
[[ "$FLOW" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]] || {
  echo "error: invalid flow name '${FLOW}' (use one ASCII path component: letters, digits, '.', '_', '-')" >&2
  exit 2
}
CONFIRM="${2:-}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CPP_RUNTIME_DIR="${CPP_RUNTIME_DIR:-/home/server/trinity-legacy-install/bin}"
CPP_CONF="${CPP_CONF:-${CPP_RUNTIME_DIR}/worldserver.conf}"
PM2_CPP_WORLD="${PM2_CPP_WORLD:-cpp-world}"
PM2_RUST_WORLD="${PM2_RUST_WORLD:-rustycore-world}"
CPP_WORLD_PORT="${CPP_WORLD_PORT:-8085}"
CPP_INSTANCE_PORT="${CPP_INSTANCE_PORT:-8086}"
CPP_CAPTURE_EXEC="${CPP_CAPTURE_EXEC:-}"
CPP_CAPTURE_EXEC_SHA256="${CPP_CAPTURE_EXEC_SHA256:-}"
CPP_CAPTURE_SOURCE_REPO="${CPP_CAPTURE_SOURCE_REPO:-/home/server/woltk-trinity-legacy}"
CAPTURE_WORLD_PORT="$CPP_WORLD_PORT"
CAPTURE_INSTANCE_PORT="$CPP_INSTANCE_PORT"
CAPTURE_ORCHESTRATION_LOCK="${CAPTURE_ORCHESTRATION_LOCK:-${XDG_RUNTIME_DIR:-/tmp}/rustycore-capture-$(id -u)-${CPP_WORLD_PORT}-${CPP_INSTANCE_PORT}.lock.d}"
CAPTURE_ORCHESTRATION_LOCK_FD=""
CPP_CAPTURE_LOOT_FIXTURE_GUARD="${CPP_CAPTURE_LOOT_FIXTURE_GUARD:-0}"
CPP_CAPTURE_ACK_LOOT_FIXTURE_MUTATION="${CPP_CAPTURE_ACK_LOOT_FIXTURE_MUTATION:-0}"
CPP_CAPTURE_DB_CONF="${CPP_CAPTURE_DB_CONF:-$CPP_CONF}"
LOOT_FIXTURE_DB_CONF="$CPP_CAPTURE_DB_CONF"
LOOT_FIXTURE_GUARD_ENABLED="$CPP_CAPTURE_LOOT_FIXTURE_GUARD"
WOW_BOT_FIXTURE_JOURNAL="${WOW_BOT_FIXTURE_JOURNAL:-}"
WOW_BOT_EXEC="${WOW_BOT_EXEC:-}"
WOW_BOT_EXEC_SHA256="${WOW_BOT_EXEC_SHA256:-}"
WOW_BOT_REPORT="${WOW_BOT_REPORT:-}"
CREATURE_SPELL_CAPTURE_ACK_FIXTURE_MUTATION="${CREATURE_SPELL_CAPTURE_ACK_FIXTURE_MUTATION:-0}"
CREATURE_SPELL_FIXTURE_JOURNAL="${CREATURE_SPELL_FIXTURE_JOURNAL:-}"
CREATURE_SPELL_FIXTURE_CLEANUP_MARKER=""
CREATURE_SPELL_FIXTURE_DB_CONF=""
CREATURE_SPELL_FIXTURE_DB_CONF_SHA256=""
CREATURE_SPELL_FIXTURE_DB_CONF_IDENTITY=""
CREATURE_SPELL_FIXTURE_SIDE=""
CREATURE_SPELL_FIXTURE_PM2_RUST_WORLD=""
CREATURE_SPELL_FIXTURE_PM2_CPP_WORLD=""
CREATURE_SPELL_FIXTURE_WORLD_PORT=""
CREATURE_SPELL_FIXTURE_INSTANCE_PORT=""
CREATURE_SPELL_FIXTURE_ORCHESTRATION_LOCK=""
CREATURE_SPELL_FIXTURE_MANIFEST=""
CREATURE_SPELL_FIXTURE_MANIFEST_SHA256=""
CREATURE_SPELL_FIXTURE_DATABASE_SNAPSHOT_SHA256=""
CREATURE_SPELL_FIXTURE_JOURNAL_SHA256=""
CREATURE_SPELL_FIXTURE_DB_APPLIED=0
CREATURE_SPELL_FIXTURE_CLEANUP_VERIFIED=0
LOOT_FIXTURE_CLEANUP_MARKER=""
LOOT_FIXTURE_ENTRY=21779
LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER=1
LOOT_FIXTURE_TEMP_HEALTH_MODIFIER=0.0001
LOOT_FIXTURE_SNAPSHOT_READY=0
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
CAPTURE_SWAPPED=0
CPP_CAPTURE_BOT_READY=0
CAPTURE_RESTORE_FAILURE_STATUS=74
RUST_ORIGINAL_IDENTITY=""
CPP_CAPTURE_IDENTITY=""
CPP_CAPTURE_PID=""
CPP_CAPTURE_LIVE_EXEC=""
CPP_CAPTURE_LIVE_SHA256=""
CPP_CAPTURE_EXPECTED_EXEC=""
CPP_CAPTURE_EXPECTED_SHA256=""
CPP_CAPTURE_SOURCE_EXEC=""
CPP_CAPTURE_SOURCE_SHA256=""
CPP_CAPTURE_EXEC_SOURCE_HEAD=""
CPP_CAPTURE_HARNESS_REPO_HEAD=""
CPP_CAPTURE_SOURCE_REPO_HEAD=""
CPP_CAPTURE_HARNESS_WORKTREE_CLEAN=0
CPP_CAPTURE_HARNESS_WORKTREE_SHA256=""
CPP_CAPTURE_SOURCE_WORKTREE_DIRTY=0
CPP_CAPTURE_SOURCE_WORKTREE_SHA256=""
CPP_CAPTURE_SOURCE_DERIVATION_JSON=null
CPP_CAPTURE_PM2_ENTRY_PID=""
CPP_CAPTURE_PM2_ENTRY_STARTTIME=""
CPP_CAPTURE_PM2_EXEC_PATH=""
CPP_CAPTURE_PM2_EXEC_SHA256=""
CPP_CAPTURE_PM2_PROFILE_SHA256=""
CPP_CAPTURE_RESTART_COUNT=""
CPP_CAPTURE_LISTENER_STARTTIME=""
CPP_CAPTURE_EFFECTIVE_CONFIG_PATH=""
CPP_CAPTURE_EFFECTIVE_CONFIG_SHA256=""
CPP_CAPTURE_PINNED=0
CAPTURE_ARTIFACT_READY=0
OUT_PKT_STAGE=""
CPP_CAPTURE_FIXTURE_CLEANUP_VERIFIED=0
CPP_CAPTURE_NORMAL_RUNTIME_RESTORED=0
CPP_CAPTURE_BOT_EXEC=""
CPP_CAPTURE_BOT_EXEC_SHA256=""
CPP_CAPTURE_BOT_REPORT=""
CPP_CAPTURE_BOT_REPORT_SHA256=""
CPP_CONF_BACKUP_IDENTITY=""
CPP_CONF_BACKUP_SHA256=""

cpp_capture_embedded_source_head() {
  local executable="$1"
  local output matches revision
  output="$("$executable" --version 2>&1)" || return 1
  matches="$(printf '%s\n' "$output" | sed -nE \
    's/^TrinityCore rev\. ([0-9a-f]{40}|[0-9a-f]{64}) .*/\1/p')" \
    || return 1
  [ -n "$matches" ] && [[ "$matches" != *$'\n'* ]] || return 1
  revision="$matches"
  [[ "$revision" =~ ^[0-9a-f]{40}$|^[0-9a-f]{64}$ ]] || return 1
  printf '%s\n' "$revision"
}

# shellcheck source=loot-fixture-common.sh
source "$(dirname "${BASH_SOURCE[0]}")/loot-fixture-common.sh"
# shellcheck source=capture-service-common.sh
source "$(dirname "${BASH_SOURCE[0]}")/capture-service-common.sh"
# shellcheck source=detour-chase-fixture-common.sh
source "$(dirname "${BASH_SOURCE[0]}")/detour-chase-fixture-common.sh"
# shellcheck source=creature-spell-casting-fixture-common.sh
source "$(dirname "${BASH_SOURCE[0]}")/creature-spell-casting-fixture-common.sh"
capture_validate_world_timeouts || exit 2

if [ "$FLOW" = "detour-chase-around-obstacle" ]; then
  LOOT_FIXTURE_GUARD_ENABLED=1
  DETOUR_FIXTURE_DB_CONF="$CPP_CAPTURE_DB_CONF"
fi

if [ "$FLOW" = "creature-spell-casting" ]; then
  [ "$CREATURE_SPELL_CAPTURE_ACK_FIXTURE_MUTATION" = 1 ] || {
    echo "error: creature-spell-casting requires CREATURE_SPELL_CAPTURE_ACK_FIXTURE_MUTATION=1" >&2
    exit 2
  }
  [ -n "$CPP_CAPTURE_EXEC" ] && [ -n "$CPP_CAPTURE_EXEC_SHA256" ] || {
    echo "error: creature spell evidence requires CPP_CAPTURE_EXEC and CPP_CAPTURE_EXEC_SHA256" >&2
    exit 2
  }
  [ -n "$WOW_BOT_EXEC" ] && [ -n "$WOW_BOT_EXEC_SHA256" ] \
    && [ -n "$WOW_BOT_REPORT" ] || {
    echo "error: creature spell evidence requires WOW_BOT_EXEC, WOW_BOT_EXEC_SHA256, and WOW_BOT_REPORT" >&2
    exit 2
  }
  [[ "$WOW_BOT_EXEC_SHA256" =~ ^[0-9A-Fa-f]{64}$ ]] || {
    echo "error: WOW_BOT_EXEC_SHA256 must contain exactly 64 hexadecimal characters" >&2
    exit 2
  }
  WOW_BOT_EXEC_SHA256="${WOW_BOT_EXEC_SHA256,,}"
  capture_validate_fresh_bot_inputs \
    "$WOW_BOT_EXEC" "$WOW_BOT_EXEC_SHA256" "$WOW_BOT_REPORT" || {
    echo "error: creature spell bot executable/report inputs are not fresh, canonical, and pinned" >&2
    exit 2
  }
  creature_spell_fixture_validate_fresh_journal || exit 2
  creature_spell_fixture_validate_committed_fixture "$REPO_ROOT" || exit 2
fi

[[ "$CPP_WORLD_PORT" =~ ^[1-9][0-9]*$ ]] \
  && ((CPP_WORLD_PORT <= 65535)) || {
    echo "error: CPP_WORLD_PORT must be an integer from 1 through 65535" >&2
    exit 2
  }
[[ "$CPP_INSTANCE_PORT" =~ ^[1-9][0-9]*$ ]] \
  && ((CPP_INSTANCE_PORT <= 65535)) || {
    echo "error: CPP_INSTANCE_PORT must be an integer from 1 through 65535" >&2
    exit 2
  }
[ "$CPP_WORLD_PORT" != "$CPP_INSTANCE_PORT" ] || {
  echo "error: CPP_WORLD_PORT and CPP_INSTANCE_PORT must be distinct" >&2
  exit 2
}

if [ "$FLOW" = "loot-single-item-claim" ] \
    && [ "$CPP_CAPTURE_LOOT_FIXTURE_GUARD" != "1" ]; then
  echo "error: loot-single-item-claim requires CPP_CAPTURE_LOOT_FIXTURE_GUARD=1" >&2
  exit 2
fi

case "$CPP_CAPTURE_LOOT_FIXTURE_GUARD" in
  0) ;;
  1)
    [ "$FLOW" = "loot-single-item-claim" ] || {
      echo "error: the C++ loot fixture guard is defined only for loot-single-item-claim" >&2
      exit 2
    }
    [ "$CPP_CAPTURE_ACK_LOOT_FIXTURE_MUTATION" = "1" ] || {
      echo "error: CPP_CAPTURE_LOOT_FIXTURE_GUARD=1 requires CPP_CAPTURE_ACK_LOOT_FIXTURE_MUTATION=1" >&2
      exit 2
    }
    [ -n "$CPP_CAPTURE_EXEC" ] && [ -n "$CPP_CAPTURE_EXEC_SHA256" ] || {
      echo "error: guarded C++ evidence requires CPP_CAPTURE_EXEC and CPP_CAPTURE_EXEC_SHA256" >&2
      exit 2
    }
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
    for dependency in awk dirname jq mysql stat; do
      command -v "$dependency" >/dev/null 2>&1 || {
        echo "error: required command not found: $dependency" >&2
        exit 2
      }
    done
    load_loot_fixture_database_credentials || exit 2
    ;;
  *)
    echo "error: CPP_CAPTURE_LOOT_FIXTURE_GUARD must be 0 or 1" >&2
    exit 2
    ;;
esac

if [ "$FLOW" = "vendor-extended-cost-purchase" ]; then
  [ -n "$CPP_CAPTURE_EXEC" ] && [ -n "$CPP_CAPTURE_EXEC_SHA256" ] || {
    echo "error: vendor evidence requires CPP_CAPTURE_EXEC and CPP_CAPTURE_EXEC_SHA256" >&2
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

if [ "$FLOW" = "detour-chase-around-obstacle" ]; then
  [ -n "$CPP_CAPTURE_EXEC" ] && [ -n "$CPP_CAPTURE_EXEC_SHA256" ] || {
    echo "error: detour evidence requires CPP_CAPTURE_EXEC and CPP_CAPTURE_EXEC_SHA256" >&2
    exit 2
  }
fi

if [ -n "$CPP_CAPTURE_EXEC" ]; then
  [[ "$CPP_CAPTURE_EXEC_SHA256" =~ ^[0-9A-Fa-f]{64}$ ]] || {
    echo "error: CPP_CAPTURE_EXEC_SHA256 must contain exactly 64 hexadecimal characters" >&2
    exit 2
  }
  CPP_CAPTURE_EXEC_SHA256="${CPP_CAPTURE_EXEC_SHA256,,}"
  CPP_CAPTURE_PINNED=1
elif [ -n "$CPP_CAPTURE_EXEC_SHA256" ]; then
  echo "error: CPP_CAPTURE_EXEC_SHA256 requires CPP_CAPTURE_EXEC" >&2
  exit 2
fi

for dependency in awk chmod cmp cp date dirname flock git grep id jq mkdir mktemp mv \
  pm2 realpath rg sed sha256sum sleep ss stat sync tail; do
  command -v "$dependency" >/dev/null 2>&1 || {
    echo "error: required command not found: $dependency" >&2
    exit 2
  }
done
if [ "$FLOW" = "detour-chase-around-obstacle" ] \
    || [ "$FLOW" = "creature-spell-casting" ]; then
  command -v mysql >/dev/null 2>&1 || {
    echo "error: mysql is required by the selected shell fixture guard" >&2
    exit 2
  }
  load_loot_fixture_database_credentials || exit 2
fi

CPP_CAPTURE_HARNESS_REPO_HEAD="$(git -C "$REPO_ROOT" rev-parse HEAD 2>/dev/null)" || {
  echo "error: cannot resolve RustyCore harness repository HEAD" >&2
  exit 2
}
CPP_CAPTURE_SOURCE_REPO="$(realpath -e -- "$CPP_CAPTURE_SOURCE_REPO" 2>/dev/null)" || {
  echo "error: CPP_CAPTURE_SOURCE_REPO does not resolve" >&2
  exit 2
}
CPP_CAPTURE_SOURCE_REPO_HEAD="$(git -C "$CPP_CAPTURE_SOURCE_REPO" rev-parse HEAD 2>/dev/null)" || {
  echo "error: cannot resolve legacy C++ source repository HEAD" >&2
  exit 2
}
capture_git_repo_clean_at_head "$REPO_ROOT" "$CPP_CAPTURE_HARNESS_REPO_HEAD" \
  && CPP_CAPTURE_HARNESS_WORKTREE_CLEAN=1
CPP_CAPTURE_HARNESS_WORKTREE_SHA256="$(
  capture_git_worktree_state_sha256 "$REPO_ROOT"
)" || {
  echo "error: cannot fingerprint the RustyCore harness worktree" >&2
  exit 2
}
CPP_CAPTURE_SOURCE_WORKTREE_SHA256="$(
  capture_git_worktree_state_sha256 "$CPP_CAPTURE_SOURCE_REPO"
)" || {
  echo "error: cannot fingerprint CPP_CAPTURE_SOURCE_REPO" >&2
  exit 2
}
if capture_git_repo_is_dirty "$CPP_CAPTURE_SOURCE_REPO"; then
  if [ "$FLOW" = "detour-chase-around-obstacle" ] \
      || [ "$FLOW" = "creature-spell-casting" ]; then
    echo "error: capture evidence requires a clean committed legacy C++ source worktree (including untracked files)" >&2
    exit 2
  fi
  CPP_CAPTURE_SOURCE_WORKTREE_DIRTY=1
fi
if [ "$CPP_CAPTURE_HARNESS_WORKTREE_CLEAN" -ne 1 ]; then
  echo "error: capture evidence requires a clean committed RustyCore harness worktree (including untracked files)" >&2
  exit 2
fi
if [ "$FLOW" = "creature-spell-casting" ]; then
  creature_spell_fixture_validate_cpp_source_derivation \
    "$REPO_ROOT" "$CPP_CAPTURE_SOURCE_REPO" || {
      echo "error: legacy C++ source does not match the reviewed creature spell derivation" >&2
      exit 2
    }
  CPP_CAPTURE_SOURCE_DERIVATION_JSON="$CREATURE_SPELL_FIXTURE_SOURCE_DERIVATION_JSON"
fi

if [ "$CPP_CAPTURE_PINNED" -eq 1 ] \
    && ! capture_exec_source_matches "$CPP_CAPTURE_EXEC" "$CPP_CAPTURE_EXEC_SHA256"; then
  echo "error: CPP_CAPTURE_EXEC is not canonical/executable or does not match its pinned SHA-256" >&2
  exit 2
fi
if [ "$CPP_CAPTURE_PINNED" -eq 1 ]; then
  CPP_CAPTURE_EXPECTED_EXEC="$CPP_CAPTURE_EXEC"
  CPP_CAPTURE_EXPECTED_SHA256="$CPP_CAPTURE_EXEC_SHA256"
  CPP_CAPTURE_SOURCE_EXEC="$CPP_CAPTURE_EXEC"
  CPP_CAPTURE_SOURCE_SHA256="$CPP_CAPTURE_EXEC_SHA256"
fi
if [ "$FLOW" = "detour-chase-around-obstacle" ] \
    || [ "$FLOW" = "creature-spell-casting" ]; then
  CPP_CAPTURE_EXEC_SOURCE_HEAD="$(
    cpp_capture_embedded_source_head "$CPP_CAPTURE_EXEC"
  )" || {
    echo "error: pinned C++ worldserver does not expose one embedded Git revision via --version" >&2
    exit 2
  }
  [ "$CPP_CAPTURE_EXEC_SOURCE_HEAD" = "$CPP_CAPTURE_SOURCE_REPO_HEAD" ] || {
    echo "error: pinned C++ binary revision ${CPP_CAPTURE_EXEC_SOURCE_HEAD} does not match clean source HEAD ${CPP_CAPTURE_SOURCE_REPO_HEAD}" >&2
    exit 2
  }
fi

CPP_CONF_CANONICAL="$(realpath -e -- "$CPP_CONF" 2>/dev/null)" || {
  echo "error: CPP_CONF does not resolve" >&2
  exit 2
}
[ "$CPP_CONF_CANONICAL" = "$CPP_CONF" ] \
  && [ -f "$CPP_CONF" ] && [ ! -L "$CPP_CONF" ] || {
  echo "error: CPP_CONF must be an absolute canonical regular non-symlink file" >&2
  exit 2
}
CPP_PROFILE_EFFECTIVE_CONFIG="$(capture_pm2_effective_config_path "$PM2_CPP_WORLD")" || {
  echo "error: cannot derive the effective C++ config from the PM2 profile/entrypoint" >&2
  exit 2
}
[ "$CPP_PROFILE_EFFECTIVE_CONFIG" = "$CPP_CONF" ] || {
  echo "error: CPP_CONF (${CPP_CONF}) differs from PM2's effective config (${CPP_PROFILE_EFFECTIVE_CONFIG})" >&2
  exit 2
}
CPP_CAPTURE_PM2_PROFILE_SHA256="$(capture_pm2_profile_redacted_sha256 "$PM2_CPP_WORLD")" || {
  echo "error: cannot hash the stable redacted C++ PM2 profile" >&2
  exit 2
}
if [ "$FLOW" = "detour-chase-around-obstacle" ]; then
  detour_chase_validate_capture_orchestration \
    "$REPO_ROOT" "$CPP_CONF" cpp || exit 2
fi

if [ -z "${CPP_LOGS_DIR+x}" ]; then
  CONFIGURED_LOGS_DIR="$({
    sed -n -E 's/^[[:space:]]*LogsDir[[:space:]]*=[[:space:]]*"([^"]*)".*/\1/p' "$CPP_CONF" 2>/dev/null || true
  } | tail -n 1)"
  if [ -z "$CONFIGURED_LOGS_DIR" ]; then
    CPP_LOGS_DIR="$CPP_RUNTIME_DIR"
  elif [[ "$CONFIGURED_LOGS_DIR" = /* ]]; then
    CPP_LOGS_DIR="$CONFIGURED_LOGS_DIR"
  else
    CPP_LOGS_DIR="${CPP_RUNTIME_DIR}/${CONFIGURED_LOGS_DIR}"
  fi
fi
CPP_LOGS_DIR_CANONICAL="$(realpath -e -- "$CPP_LOGS_DIR" 2>/dev/null)" || {
  echo "error: CPP_LOGS_DIR does not resolve" >&2
  exit 2
}
[ "$CPP_LOGS_DIR_CANONICAL" = "$CPP_LOGS_DIR" ] \
  && [ -d "$CPP_LOGS_DIR" ] && [ ! -L "$CPP_LOGS_DIR" ] || {
  echo "error: CPP_LOGS_DIR must be an absolute canonical non-symlink directory" >&2
  exit 2
}

PKT_NAME="rustycore-capture-${FLOW}.pkt"
OUT_DIR="${REPO_ROOT}/target/captures/${FLOW}"
OUT_PKT="${OUT_DIR}/cpp.pkt"
OUT_MANIFEST="${OUT_DIR}/cpp.capture-manifest.json"
capture_require_canonical_directory "${REPO_ROOT}/target/captures" \
  && capture_require_canonical_directory "$OUT_DIR" || {
  echo "error: capture output root is not canonical or contains a symlink" >&2
  exit 2
}
[ ! -e "$OUT_PKT" ] && [ ! -L "$OUT_PKT" ] \
  && [ ! -e "$OUT_MANIFEST" ] && [ ! -L "$OUT_MANIFEST" ] || {
    echo "error: raw C++ capture output already exists; archive/remove cpp.pkt and cpp.capture-manifest.json before recording a new generation" >&2
    exit 2
  }

accredit_cpp_capture_executable() {
  local proc_exe="/proc/${CPP_CAPTURE_PID}/exe"
  local live_exec live_sha pm2_identity pm2_pid pm2_restart pm2_exec pm2_exec_sha

  [ -L "$proc_exe" ] || return 1
  live_exec="$(realpath -e -- "$proc_exe" 2>/dev/null)" || return 1
  live_sha="$(capture_sha256_of_file "$proc_exe")" || return 1
  if [ "$CPP_CAPTURE_PINNED" -eq 1 ]; then
    [ "$live_exec" = "$CPP_CAPTURE_EXEC" ] \
      && [ "$live_sha" = "$CPP_CAPTURE_EXEC_SHA256" ] \
      && capture_live_exec_matches \
        "$CPP_CAPTURE_PID" "$CPP_CAPTURE_EXEC" "$CPP_CAPTURE_EXEC_SHA256" \
      || return 1
  fi
  pm2_identity="$(capture_pm2_entrypoint_identity \
    "$PM2_CPP_WORLD" "$CPP_CAPTURE_PM2_ENTRY_PID")" \
    || return 1
  IFS=$'\t' read -r pm2_pid pm2_restart pm2_exec pm2_exec_sha <<<"$pm2_identity"
  [ "$pm2_pid" = "$CPP_CAPTURE_PM2_ENTRY_PID" ] \
    && [[ "$pm2_restart" =~ ^[0-9]+$ ]] \
    && capture_pid_is_self_or_descendant \
      "$CPP_CAPTURE_PID" "$CPP_CAPTURE_PM2_ENTRY_PID" \
    || return 1
  CPP_CAPTURE_PM2_EXEC_PATH="$pm2_exec"
  CPP_CAPTURE_PM2_EXEC_SHA256="$pm2_exec_sha"
  CPP_CAPTURE_RESTART_COUNT="$pm2_restart"
  CPP_CAPTURE_PM2_ENTRY_STARTTIME="$(capture_pid_starttime "$CPP_CAPTURE_PM2_ENTRY_PID")" \
    || return 1
  CPP_CAPTURE_LISTENER_STARTTIME="$(capture_pid_starttime "$CPP_CAPTURE_PID")" \
    || return 1
  CPP_CAPTURE_LIVE_EXEC="$live_exec"
  CPP_CAPTURE_LIVE_SHA256="$live_sha"
  if [ "$CPP_CAPTURE_PINNED" -eq 0 ]; then
    CPP_CAPTURE_EXPECTED_EXEC="$live_exec"
    CPP_CAPTURE_EXPECTED_SHA256="$live_sha"
    CPP_CAPTURE_SOURCE_EXEC="$live_exec"
    CPP_CAPTURE_SOURCE_SHA256="$live_sha"
  fi
}

cpp_capture_effective_config_sha256() {
  capture_effective_config_redacted_sha256 \
    "$CPP_CAPTURE_EFFECTIVE_CONFIG_PATH" \
    "capture.world_port=${CPP_WORLD_PORT}
capture.instance_port=${CPP_INSTANCE_PORT}
capture.packet_log=enabled" \
    PacketLogFile LogsDir Bot.AccountPrefix WorldServerPort InstanceServerPort \
    DataDir \
    LoginDatabaseInfo WorldDatabaseInfo CharacterDatabaseInfo \
    Rate.Drop.Item.Poor Rate.Drop.Item.Normal Rate.Drop.Item.Uncommon \
    Rate.Drop.Item.Rare Rate.Drop.Item.Epic Rate.Drop.Item.Legendary \
    Rate.Drop.Item.Artifact Rate.Drop.Item.Referenced Rate.Drop.Money
}

cpp_capture_executable_unchanged() {
  [ -n "$CPP_CAPTURE_LIVE_EXEC" ] \
    && [ -n "$CPP_CAPTURE_LIVE_SHA256" ] \
    && capture_live_exec_matches \
      "$CPP_CAPTURE_PID" "$CPP_CAPTURE_LIVE_EXEC" "$CPP_CAPTURE_LIVE_SHA256" \
    && [ "$(capture_pm2_entrypoint_identity \
      "$PM2_CPP_WORLD" "$CPP_CAPTURE_PM2_ENTRY_PID")" \
      = "${CPP_CAPTURE_PM2_ENTRY_PID}"$'\t'"${CPP_CAPTURE_RESTART_COUNT}"$'\t'"${CPP_CAPTURE_PM2_EXEC_PATH}"$'\t'"${CPP_CAPTURE_PM2_EXEC_SHA256}" ] \
    && [ "$(capture_world_ready_once "$PM2_CPP_WORLD")" = "$CPP_CAPTURE_IDENTITY" ] \
    && [ "$(capture_pid_starttime "$CPP_CAPTURE_PM2_ENTRY_PID")" \
      = "$CPP_CAPTURE_PM2_ENTRY_STARTTIME" ] \
    && [ "$(capture_pid_starttime "$CPP_CAPTURE_PID")" \
      = "$CPP_CAPTURE_LISTENER_STARTTIME" ] \
    && [ "$(capture_pm2_profile_redacted_sha256 "$PM2_CPP_WORLD")" \
      = "$CPP_CAPTURE_PM2_PROFILE_SHA256" ] \
    && [ "$(capture_pm2_effective_config_path "$PM2_CPP_WORLD")" \
      = "$CPP_CONF" ] \
    && [ "$(cpp_capture_effective_config_sha256)" \
      = "$CPP_CAPTURE_EFFECTIVE_CONFIG_SHA256" ]
}

finalize_cpp_capture_artifact() {
  [ "$CAPTURE_ARTIFACT_READY" -eq 1 ] && [ -f "$OUT_PKT_STAGE" ] || return 1

  local bot_evidence="" capture_evidence created_at manifest_stage packet_sha packet_size
  local fixture_guard_enabled="$CPP_CAPTURE_LOOT_FIXTURE_GUARD"
  [ "$CPP_CAPTURE_NORMAL_RUNTIME_RESTORED" -eq 1 ] || return 1
  if [ "$FLOW" = "detour-chase-around-obstacle" ]; then
    fixture_guard_enabled=1
  fi
  if [ "$FLOW" = "creature-spell-casting" ]; then
    fixture_guard_enabled=1
  fi
  capture_fixture_cleanup_verified_for_publication \
    "$fixture_guard_enabled" \
    "$CPP_CAPTURE_FIXTURE_CLEANUP_VERIFIED" || return 1
  case "$FLOW" in
    loot-single-item-claim)
      bot_evidence="$(capture_loot_item_bot_evidence \
        "$WOW_BOT_REPORT" "$WOW_BOT_EXEC" "$WOW_BOT_EXEC_SHA256")" || return 1
      ;;
    vendor-extended-cost-purchase)
      bot_evidence="$(capture_vendor_bot_evidence \
        "$WOW_BOT_REPORT" "$WOW_BOT_EXEC" "$WOW_BOT_EXEC_SHA256")" || return 1
      ;;
    creature-spell-casting)
      bot_evidence="$(creature_spell_fixture_bot_evidence \
        "$WOW_BOT_REPORT" "$WOW_BOT_EXEC" "$WOW_BOT_EXEC_SHA256")" || return 1
      ;;
  esac
  if [ -n "$bot_evidence" ]; then
    IFS=$'\t' read -r CPP_CAPTURE_BOT_EXEC CPP_CAPTURE_BOT_EXEC_SHA256 \
      CPP_CAPTURE_BOT_REPORT CPP_CAPTURE_BOT_REPORT_SHA256 <<<"$bot_evidence"
  fi
  if [ "$FLOW" = "detour-chase-around-obstacle" ]; then
    capture_evidence="$(detour_chase_capture_evidence)" || return 1
  elif [ "$FLOW" = "creature-spell-casting" ]; then
    capture_evidence="$(creature_spell_fixture_capture_evidence \
      "$CPP_CAPTURE_BOT_EXEC" "$CPP_CAPTURE_BOT_EXEC_SHA256" \
      "$CPP_CAPTURE_BOT_REPORT" "$CPP_CAPTURE_BOT_REPORT_SHA256")" || return 1
  else
    capture_evidence="$(capture_bot_manifest_evidence \
      "$FLOW" "$CPP_CAPTURE_BOT_EXEC" "$CPP_CAPTURE_BOT_EXEC_SHA256" \
      "$CPP_CAPTURE_BOT_REPORT" "$CPP_CAPTURE_BOT_REPORT_SHA256")" || return 1
  fi
  packet_sha="$(capture_sha256_of_file "$OUT_PKT_STAGE")" || return 1
  packet_size="$(stat -c '%s' -- "$OUT_PKT_STAGE")" || return 1
  created_at="$(date -u +'%Y-%m-%dT%H:%M:%SZ')" || return 1
  capture_git_repo_clean_at_head "$REPO_ROOT" "$CPP_CAPTURE_HARNESS_REPO_HEAD" \
    && [ "$(capture_git_worktree_state_sha256 "$REPO_ROOT")" \
      = "$CPP_CAPTURE_HARNESS_WORKTREE_SHA256" ] \
    && [ "$(git -C "$CPP_CAPTURE_SOURCE_REPO" rev-parse HEAD 2>/dev/null)" \
      = "$CPP_CAPTURE_SOURCE_REPO_HEAD" ] \
    && { { [ "$FLOW" != "detour-chase-around-obstacle" ] \
        && [ "$FLOW" != "creature-spell-casting" ]; } \
      || [ "$(cpp_capture_embedded_source_head "$CPP_CAPTURE_EXEC")" \
        = "$CPP_CAPTURE_EXEC_SOURCE_HEAD" ]; } \
    && { [ "$FLOW" != "creature-spell-casting" ] \
      || creature_spell_fixture_validate_cpp_source_derivation \
        "$REPO_ROOT" "$CPP_CAPTURE_SOURCE_REPO"; } \
    && [ "$(capture_git_worktree_state_sha256 "$CPP_CAPTURE_SOURCE_REPO")" \
      = "$CPP_CAPTURE_SOURCE_WORKTREE_SHA256" ] || return 1
  capture_require_canonical_directory "$OUT_DIR" \
    && [ ! -e "$OUT_PKT" ] && [ ! -L "$OUT_PKT" ] \
    && [ ! -e "$OUT_MANIFEST" ] && [ ! -L "$OUT_MANIFEST" ] || return 1
  manifest_stage="$(mktemp "${OUT_DIR}/.cpp.capture-manifest.partial.XXXXXX")" \
    || return 1
  if ! jq -n \
      --arg flow "$FLOW" \
      --arg created_at "$created_at" \
      --arg harness_repo_head "$CPP_CAPTURE_HARNESS_REPO_HEAD" \
      --arg source_repo_head "$CPP_CAPTURE_SOURCE_REPO_HEAD" \
      --arg source_exec_revision "$CPP_CAPTURE_EXEC_SOURCE_HEAD" \
      --arg harness_worktree_sha256 "$CPP_CAPTURE_HARNESS_WORKTREE_SHA256" \
      --arg source_worktree_sha256 "$CPP_CAPTURE_SOURCE_WORKTREE_SHA256" \
      --arg expected_exec_path "$CPP_CAPTURE_EXPECTED_EXEC" \
      --arg expected_exec_sha256 "$CPP_CAPTURE_EXPECTED_SHA256" \
      --arg source_exec_path "$CPP_CAPTURE_SOURCE_EXEC" \
      --arg source_exec_sha256 "$CPP_CAPTURE_SOURCE_SHA256" \
      --arg live_exec_path "$CPP_CAPTURE_LIVE_EXEC" \
      --arg live_exec_sha256 "$CPP_CAPTURE_LIVE_SHA256" \
      --arg pm2_exec_path "$CPP_CAPTURE_PM2_EXEC_PATH" \
      --arg pm2_exec_sha256 "$CPP_CAPTURE_PM2_EXEC_SHA256" \
      --arg pm2_profile_sha256 "$CPP_CAPTURE_PM2_PROFILE_SHA256" \
      --arg effective_config_path "$CPP_CAPTURE_EFFECTIVE_CONFIG_PATH" \
      --arg effective_config_sha256 "$CPP_CAPTURE_EFFECTIVE_CONFIG_SHA256" \
      --arg bot_exec_path "$CPP_CAPTURE_BOT_EXEC" \
      --arg bot_exec_sha256 "$CPP_CAPTURE_BOT_EXEC_SHA256" \
      --arg bot_report_path "$CPP_CAPTURE_BOT_REPORT" \
      --arg bot_report_sha256 "$CPP_CAPTURE_BOT_REPORT_SHA256" \
      --arg packet_sha256 "$packet_sha" \
      --argjson capture_evidence "$capture_evidence" \
      --argjson source_derivation "$CPP_CAPTURE_SOURCE_DERIVATION_JSON" \
      --argjson pm2_entry_pid "$CPP_CAPTURE_PM2_ENTRY_PID" \
      --argjson pm2_entry_starttime "$CPP_CAPTURE_PM2_ENTRY_STARTTIME" \
      --argjson listener_runtime_pid "$CPP_CAPTURE_PID" \
      --argjson listener_runtime_starttime "$CPP_CAPTURE_LISTENER_STARTTIME" \
      --argjson restart_count "$CPP_CAPTURE_RESTART_COUNT" \
      --argjson packet_size "$packet_size" \
      --argjson pinned "$([ "$CPP_CAPTURE_PINNED" -eq 1 ] && printf true || printf false)" \
      --argjson source_worktree_dirty \
        "$([ "$CPP_CAPTURE_SOURCE_WORKTREE_DIRTY" -eq 1 ] && printf true || printf false)" \
      '{
        version: 3,
        flow: $flow,
        side: "cpp",
        completed: true,
        created_at: $created_at,
        harness_repo_head: $harness_repo_head,
        source_repo_head: $source_repo_head,
        source_exec_revision:
          (if $source_exec_revision == "" then null else $source_exec_revision end),
        harness_worktree_clean: true,
        harness_worktree_state_sha256: $harness_worktree_sha256,
        source_worktree_dirty: $source_worktree_dirty,
        source_worktree_state_sha256: $source_worktree_sha256,
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
          path: "cpp.pkt",
          size: $packet_size,
          sha256: $packet_sha256
        }
      }
      + (if $source_derivation == null then {}
         else {source_derivation: $source_derivation}
         end)' >"$manifest_stage"; then
    rm -f -- "$manifest_stage"
    return 1
  fi
  chmod 600 "$OUT_PKT_STAGE" "$manifest_stage" || {
    rm -f -- "$manifest_stage"
    return 1
  }
  sync -f "$OUT_PKT_STAGE" && sync -f "$manifest_stage" || {
    rm -f -- "$manifest_stage"
    return 1
  }
  capture_publish_noreplace "$OUT_PKT_STAGE" "$OUT_PKT" || {
    rm -f -- "$manifest_stage"
    return 1
  }
  OUT_PKT_STAGE=""
  capture_publish_noreplace "$manifest_stage" "$OUT_MANIFEST" || return 1
  sync -f "$OUT_DIR" || return 1
  CAPTURE_ARTIFACT_READY=0
  echo "collected ${packet_size} bytes -> ${OUT_PKT}"
  echo "provenance   -> ${OUT_MANIFEST}"
}

echo "flow         : ${FLOW}"
echo "C++ conf     : ${CPP_CONF}"
echo "C++ logs dir : ${CPP_LOGS_DIR}"
if [ "$CPP_CAPTURE_PINNED" -eq 1 ]; then
  echo "C++ exec     : ${CPP_CAPTURE_EXEC}"
  echo "C++ SHA-256  : ${CPP_CAPTURE_EXEC_SHA256}"
fi
echo "pkt file     : ${CPP_LOGS_DIR}/${PKT_NAME}"
echo "output       : ${OUT_PKT}"
if [ "$FLOW" = "creature-spell-casting" ]; then
  echo "DB fixture   : Cabal entry ${CREATURE_SPELL_FIXTURE_ENTRY}, spawn ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}, spell ${CREATURE_SPELL_FIXTURE_SPELL_ID}; SmartAI -> CombatAI"
fi
echo
echo "This will STOP ${PM2_RUST_WORLD} and START ${PM2_CPP_WORLD} (shared DBs/ports)."

if [ "$CONFIRM" != "--yes" ]; then
  read -r -p "Proceed? [y/N] " ans
  [ "$ans" = "y" ] || [ "$ans" = "Y" ] || { echo "aborted"; exit 1; }
fi

capture_acquire_orchestration_lock "$CAPTURE_ORCHESTRATION_LOCK" || {
  echo "error: another capture/QA process holds ${CAPTURE_ORCHESTRATION_LOCK}" >&2
  exit 1
}
if [ "$CPP_CAPTURE_PINNED" -eq 1 ] \
    && ! capture_exec_source_matches "$CPP_CAPTURE_EXEC" "$CPP_CAPTURE_EXEC_SHA256"; then
  echo "error: pinned C++ executable changed before service mutation" >&2
  exit 1
fi
capture_git_repo_clean_at_head "$REPO_ROOT" "$CPP_CAPTURE_HARNESS_REPO_HEAD" \
  && [ "$(capture_git_worktree_state_sha256 "$REPO_ROOT")" \
    = "$CPP_CAPTURE_HARNESS_WORKTREE_SHA256" ] \
  && [ "$(git -C "$CPP_CAPTURE_SOURCE_REPO" rev-parse HEAD 2>/dev/null)" \
    = "$CPP_CAPTURE_SOURCE_REPO_HEAD" ] \
  && { { [ "$FLOW" != "detour-chase-around-obstacle" ] \
      && [ "$FLOW" != "creature-spell-casting" ]; } \
    || capture_git_repo_clean_at_head \
      "$CPP_CAPTURE_SOURCE_REPO" "$CPP_CAPTURE_SOURCE_REPO_HEAD"; } \
  && { { [ "$FLOW" != "detour-chase-around-obstacle" ] \
      && [ "$FLOW" != "creature-spell-casting" ]; } \
    || [ "$(cpp_capture_embedded_source_head "$CPP_CAPTURE_EXEC")" \
      = "$CPP_CAPTURE_EXEC_SOURCE_HEAD" ]; } \
  && { [ "$FLOW" != "creature-spell-casting" ] \
    || creature_spell_fixture_validate_cpp_source_derivation \
      "$REPO_ROOT" "$CPP_CAPTURE_SOURCE_REPO"; } \
  && [ "$(capture_git_worktree_state_sha256 "$CPP_CAPTURE_SOURCE_REPO")" \
    = "$CPP_CAPTURE_SOURCE_WORKTREE_SHA256" ] || {
  echo "error: harness/source worktree provenance changed before service mutation" >&2
  exit 1
}

[ -f "$CPP_CONF" ] || { echo "error: conf not found: $CPP_CONF" >&2; exit 1; }
[ -d "$CPP_LOGS_DIR" ] || { echo "error: packet log directory not found: $CPP_LOGS_DIR" >&2; exit 1; }

CONF_BAK="${CPP_CONF}.capture-diff.bak"
# A leftover backup means a prior run was killed before restoring. Refuse to
# overwrite it with the (possibly already-edited) conf — that would lose the
# pristine original. The operator must inspect/restore it manually first.
if [ -e "$CONF_BAK" ] || [ -L "$CONF_BAK" ]; then
  echo "error: stale backup ${CONF_BAK} exists (a prior run did not restore)." >&2
  echo "       restore it over ${CPP_CONF} and delete it before re-running." >&2
  exit 1
fi
RUST_ORIGINAL_IDENTITY="$(capture_wait_for_world_ready "$PM2_RUST_WORLD")" || {
  echo "error: ${PM2_RUST_WORLD} is not one exact online PM2 process owning both configured listeners" >&2
  exit 1
}
capture_pm2_process_stopped "$PM2_CPP_WORLD" || {
  echo "error: ${PM2_CPP_WORLD} must be one exact stopped PM2 process before capture" >&2
  exit 1
}
# Preserve the active config's ownership/mode/timestamps exactly. A restrictive
# caller umask must not turn the restored worldserver.conf into a different
# runtime file after the capture.
cp -a "$CPP_CONF" "$CONF_BAK"
[ -f "$CONF_BAK" ] && [ ! -L "$CONF_BAK" ] \
  && [ "$(realpath -e -- "$CONF_BAK" 2>/dev/null)" = "$CONF_BAK" ] || {
  echo "error: failed to create a canonical regular config backup" >&2
  exit 1
}
CPP_CONF_BACKUP_IDENTITY="$(stat -c '%d:%i' -- "$CONF_BAK")" || exit 1
CPP_CONF_BACKUP_SHA256="$(capture_sha256_of_file "$CONF_BAK")" || exit 1

restore() {
  local capture_status=$?
  local restored_rust_pid=""
  local restore_status=0
  trap - EXIT HUP INT TERM
  trap '' HUP INT TERM
  set +e

  if [ "$CAPTURE_SWAPPED" -eq 1 ]; then
    echo "restoring ${PM2_CPP_WORLD} -> ${PM2_RUST_WORLD} and conf..."
    if ! pm2 stop "$PM2_CPP_WORLD" >/dev/null 2>&1; then
      echo "WARNING: failed to stop ${PM2_CPP_WORLD}; refusing fixture restoration while it may still own runtime state" >&2
      restore_status=1
    fi
    if ! capture_wait_for_world_stopped "$PM2_CPP_WORLD" "$CPP_CAPTURE_IDENTITY"; then
      echo "WARNING: ${PM2_CPP_WORLD} PID/PM2 entry or ports ${CPP_WORLD_PORT}/${CPP_INSTANCE_PORT} remain active; refusing DB restoration" >&2
      restore_status=1
    fi
    if [ "$restore_status" -eq 0 ] \
        && [ "$CPP_CAPTURE_LOOT_FIXTURE_GUARD" = "1" ] \
        && ! loot_fixture_wait_until_all_characters_offline; then
      echo "WARNING: characters remain online after stopping C++; refusing fixture restoration" >&2
      restore_status=1
    fi
    if [ "$restore_status" -eq 0 ] \
        && [ "$CPP_CAPTURE_LOOT_FIXTURE_GUARD" = "1" ] \
        && ! restore_creature_health_fixture_guard; then
      echo "WARNING: failed to restore the bounded Doctor loot fixture" >&2
      restore_status=1
    fi
    if [ "$restore_status" -eq 0 ] \
        && [ "$FLOW" = "creature-spell-casting" ] \
        && ! creature_spell_fixture_record_post_login_snapshot; then
      echo "WARNING: failed to durably snapshot the exact post-login creature spell character state" >&2
      restore_status=1
    fi
    if [ "$restore_status" -eq 0 ] \
        && [ "$FLOW" = "creature-spell-casting" ] \
        && ! creature_spell_fixture_restore_guard; then
      echo "WARNING: failed to restore the guarded Cabal creature spell fixture" >&2
      restore_status=1
    fi
    if [ "$restore_status" -eq 0 ] \
        && [ "$FLOW" = "detour-chase-around-obstacle" ] \
        && ! detour_chase_restore_fixture_guard; then
      echo "WARNING: failed to restore the guarded detour creature/character fixture" >&2
      restore_status=1
    fi
    if [ "$restore_status" -eq 0 ] \
        && [ "$FLOW" != "detour-chase-around-obstacle" ] \
        && ! loot_fixture_bot_cleanup_safe_for_capture_state \
          "$CPP_CAPTURE_BOT_READY"; then
      echo "WARNING: bot fixture cleanup is unproven; the normal Rust world will remain stopped" >&2
      restore_status=1
    fi
    if [ "$restore_status" -eq 0 ] \
        && [ "$CPP_CAPTURE_LOOT_FIXTURE_GUARD" = "1" ]; then
      CPP_CAPTURE_FIXTURE_CLEANUP_VERIFIED=1
    fi
    if [ "$restore_status" -eq 0 ] \
        && [ "$FLOW" = "creature-spell-casting" ]; then
      CPP_CAPTURE_FIXTURE_CLEANUP_VERIFIED=1
    fi
  fi

  if [ ! -f "$CONF_BAK" ] || [ -L "$CONF_BAK" ] \
      || [ "$(stat -c '%d:%i' -- "$CONF_BAK" 2>/dev/null)" \
        != "$CPP_CONF_BACKUP_IDENTITY" ] \
      || [ "$(capture_sha256_of_file "$CONF_BAK" 2>/dev/null)" \
        != "$CPP_CONF_BACKUP_SHA256" ]; then
    echo "WARNING: ${CONF_BAK} changed or became unsafe; refusing to install it over ${CPP_CONF}" >&2
    restore_status=1
  elif ! mv -f "$CONF_BAK" "$CPP_CONF"; then
    echo "WARNING: failed to restore ${CPP_CONF} from ${CONF_BAK} — packet logging may still be ON; restore it manually." >&2
    restore_status=1
  fi
  if [ "$restore_status" -eq 0 ] \
      && { [ ! -f "$CPP_CONF" ] || [ -L "$CPP_CONF" ] \
        || [ "$(stat -c '%d:%i' -- "$CPP_CONF" 2>/dev/null)" \
          != "$CPP_CONF_BACKUP_IDENTITY" ] \
        || [ "$(capture_sha256_of_file "$CPP_CONF" 2>/dev/null)" \
          != "$CPP_CONF_BACKUP_SHA256" ]; }; then
    echo "WARNING: restored ${CPP_CONF} does not match the accredited backup; normal Rust will remain stopped" >&2
    restore_status=1
  fi
  if [ "$restore_status" -eq 0 ] \
      && [ "$FLOW" = "detour-chase-around-obstacle" ] \
      && [ -e "$WOW_BOT_FIXTURE_JOURNAL" ]; then
    if [ "$DETOUR_FIXTURE_DB_APPLIED" = 0 ]; then
      if ! detour_chase_discard_unarmed_private_data_dir; then
        echo "WARNING: failed to discard the unarmed private detour DataDir; normal Rust will remain stopped" >&2
        restore_status=1
      fi
    elif ! detour_chase_remove_private_data_dir; then
      echo "WARNING: failed to remove the private detour DataDir; normal Rust will remain stopped" >&2
      restore_status=1
    fi
  fi
  if [ "$restore_status" -eq 0 ] \
      && [ "$FLOW" = "detour-chase-around-obstacle" ] \
      && [ -e "$WOW_BOT_FIXTURE_JOURNAL" ] \
      && ! detour_chase_mark_filesystem_restored; then
    echo "WARNING: failed to durably mark detour filesystem recovery; normal Rust will remain stopped" >&2
    restore_status=1
  fi
  if [ "$CAPTURE_SWAPPED" -eq 1 ] && [ "$restore_status" -eq 0 ]; then
    if ! pm2 start "$PM2_RUST_WORLD" >/dev/null 2>&1; then
      echo "WARNING: failed to restart ${PM2_RUST_WORLD}; inspect PM2 before another capture" >&2
      restore_status=1
    elif ! restored_rust_pid="$(capture_wait_for_world_ready "$PM2_RUST_WORLD")"; then
      echo "WARNING: restored ${PM2_RUST_WORLD} did not become one stable PID owning both configured listeners" >&2
      restore_status=1
    else
      CPP_CAPTURE_NORMAL_RUNTIME_RESTORED=1
    fi
  fi
  if [ "$restore_status" -eq 0 ] \
      && [ "$FLOW" = "detour-chase-around-obstacle" ]; then
    if [ "$CAPTURE_SWAPPED" -eq 0 ] \
        && ! capture_wait_for_world_ready "$PM2_RUST_WORLD" >/dev/null; then
      echo "WARNING: normal Rust changed during pre-mutation detour cleanup" >&2
      restore_status=1
    elif ! detour_chase_mark_normal_runtime_restored; then
      echo "WARNING: normal Rust is online but detour recovery phase could not be persisted" >&2
      restore_status=1
    elif ! detour_chase_complete_fixture_journal \
        || ! loot_fixture_bot_cleanup_complete; then
      echo "WARNING: detour recovery journal could not be completed after normal runtime restoration" >&2
      restore_status=1
    else
      DETOUR_FIXTURE_CLEANUP_VERIFIED=1
      CPP_CAPTURE_FIXTURE_CLEANUP_VERIFIED=1
    fi
  fi
  if [ "$restore_status" -eq 0 ] \
      && [ "$CAPTURE_SWAPPED" -eq 1 ] \
      && { [ "$CPP_CAPTURE_LOOT_FIXTURE_GUARD" = "1" ] \
        || [ "$FLOW" = "detour-chase-around-obstacle" ]; } \
      && ! rm -f -- "$LOOT_FIXTURE_CLEANUP_MARKER"; then
    echo "WARNING: failed to remove the consumed bot cleanup marker" >&2
    restore_status=1
  fi
  if [ "$restore_status" -eq 0 ] \
      && [ "$CAPTURE_SWAPPED" -eq 1 ] \
      && [ "$FLOW" = "creature-spell-casting" ] \
      && ! creature_spell_fixture_remove_cleanup_marker; then
    echo "WARNING: failed to remove the consumed creature spell cleanup marker" >&2
    restore_status=1
  fi
  if [ "$restore_status" -eq 0 ] && [ "$capture_status" -eq 0 ]; then
    if ! finalize_cpp_capture_artifact; then
      echo "WARNING: capture cleanup succeeded, but atomic packet/manifest publication failed" >&2
      restore_status=1
    fi
  fi
  if [ "$restore_status" -ne 0 ] || [ "$capture_status" -ne 0 ]; then
    [ -z "$OUT_PKT_STAGE" ] || rm -f -- "$OUT_PKT_STAGE"
    CAPTURE_ARTIFACT_READY=0
  fi
  capture_release_orchestration_lock
  if [ "$restore_status" -ne 0 ]; then
    echo "WARNING: guarded C++ capture cleanup or provenance publication failed; recover explicitly before reuse" >&2
    exit "$CAPTURE_RESTORE_FAILURE_STATUS"
  fi
  exit "$capture_status"
}
trap restore EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

if [ "$FLOW" = "creature-spell-casting" ]; then
  # If the credential source is the active C++ config, pin its pristine backup:
  # PacketLogFile/Bot.AccountPrefix are edited before the DB CAS, and a fresh
  # recovery shell must never trust those mutable bytes.
  CPP_CAPTURE_DB_CONF_PATH="$(realpath -e -- "$CPP_CAPTURE_DB_CONF")" || exit 1
  if [ "$CPP_CAPTURE_DB_CONF_PATH" = "$CPP_CONF" ]; then
    CREATURE_SPELL_FIXTURE_DB_CONF="$CONF_BAK"
    CREATURE_SPELL_FIXTURE_DB_CONF_SHA256="$CPP_CONF_BACKUP_SHA256"
    CREATURE_SPELL_FIXTURE_DB_CONF_IDENTITY="$CPP_CONF_BACKUP_IDENTITY"
  else
    CREATURE_SPELL_FIXTURE_DB_CONF="$CPP_CAPTURE_DB_CONF_PATH"
    CREATURE_SPELL_FIXTURE_DB_CONF_SHA256="$(
      creature_spell_fixture_sha256_of_file \
        "$CREATURE_SPELL_FIXTURE_DB_CONF"
    )"
    CREATURE_SPELL_FIXTURE_DB_CONF_IDENTITY="$(
      stat -c '%d:%i' -- "$CREATURE_SPELL_FIXTURE_DB_CONF"
    )"
  fi
  CREATURE_SPELL_FIXTURE_SIDE=cpp
  CREATURE_SPELL_FIXTURE_PM2_RUST_WORLD="$PM2_RUST_WORLD"
  CREATURE_SPELL_FIXTURE_PM2_CPP_WORLD="$PM2_CPP_WORLD"
  CREATURE_SPELL_FIXTURE_WORLD_PORT="$CPP_WORLD_PORT"
  CREATURE_SPELL_FIXTURE_INSTANCE_PORT="$CPP_INSTANCE_PORT"
  CREATURE_SPELL_FIXTURE_ORCHESTRATION_LOCK="$CAPTURE_ORCHESTRATION_LOCK"
  creature_spell_fixture_validate_db_config || {
    echo "error: creature spell recovery DB config is not canonical and pinned" >&2
    exit 1
  }
fi

if [ "$FLOW" = "detour-chase-around-obstacle" ]; then
  detour_chase_allocate_private_data_dir
  DETOUR_FIXTURE_DB_APPLIED=0
  # Arm recovery before the active C++ config or either service changes. The
  # pristine backup is the pinned DB-credential source for a fresh-shell
  # recovery; it is never necessary to trust the subsequently edited config.
  CPP_CAPTURE_DB_CONF_PATH="$(realpath -e -- "$CPP_CAPTURE_DB_CONF")"
  if [ "$CPP_CAPTURE_DB_CONF_PATH" = "$CPP_CONF" ]; then
    DETOUR_FIXTURE_DB_CONF="$(realpath -e -- "$CONF_BAK")"
    DETOUR_FIXTURE_DB_CONF_SHA256="$CPP_CONF_BACKUP_SHA256"
    DETOUR_FIXTURE_DB_CONF_IDENTITY="$CPP_CONF_BACKUP_IDENTITY"
  else
    DETOUR_FIXTURE_DB_CONF="$CPP_CAPTURE_DB_CONF_PATH"
    DETOUR_FIXTURE_DB_CONF_SHA256="$(
      capture_sha256_of_file "$DETOUR_FIXTURE_DB_CONF"
    )"
    DETOUR_FIXTURE_DB_CONF_IDENTITY="$(
      stat -c '%d:%i' -- "$DETOUR_FIXTURE_DB_CONF"
    )"
  fi
  DETOUR_FIXTURE_ORCHESTRATION_LOCK="$CAPTURE_ORCHESTRATION_LOCK"
  DETOUR_FIXTURE_PM2_RUST_WORLD="$PM2_RUST_WORLD"
  DETOUR_FIXTURE_PM2_CPP_WORLD="$PM2_CPP_WORLD"
  DETOUR_FIXTURE_NORMAL_RUST_PM2_PROFILE_SHA256="$(
    capture_pm2_profile_redacted_sha256 "$PM2_RUST_WORLD"
  )"
  DETOUR_FIXTURE_NORMAL_RUST_CONFIG="$(
    capture_pm2_effective_config_path "$PM2_RUST_WORLD"
  )"
  DETOUR_FIXTURE_NORMAL_RUST_CONFIG_SHA256="$(
    capture_sha256_of_file "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG"
  )"
  DETOUR_FIXTURE_NORMAL_RUST_CONFIG_IDENTITY="$(
    stat -c '%d:%i' -- "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG"
  )"
  DETOUR_FIXTURE_WORLD_PORT="$CPP_WORLD_PORT"
  DETOUR_FIXTURE_INSTANCE_PORT="$CPP_INSTANCE_PORT"
  DETOUR_FIXTURE_CPP_CONFIG="$CPP_CONF"
  DETOUR_FIXTURE_CPP_CONFIG_BACKUP="$CONF_BAK"
  DETOUR_FIXTURE_CPP_CONFIG_BACKUP_IDENTITY="$CPP_CONF_BACKUP_IDENTITY"
  DETOUR_FIXTURE_CPP_CONFIG_BACKUP_SHA256="$CPP_CONF_BACKUP_SHA256"
  detour_chase_arm_filesystem_recovery_journal
  detour_chase_populate_private_data_dir
  detour_chase_patch_config_data_dir \
    "$CPP_CONF" "$DETOUR_FIXTURE_PRIVATE_DATA_DIR"
fi

# Enable PacketLogFile in the conf (replace existing line or append).
if grep -qE '^[[:space:]]*PacketLogFile' "$CPP_CONF"; then
  sed -i -E "s|^[[:space:]]*PacketLogFile.*|PacketLogFile = \"${PKT_NAME}\"|" "$CPP_CONF"
else
  printf '\nPacketLogFile = "%s"\n' "$PKT_NAME" >>"$CPP_CONF"
fi

# A local C++ test-harness extension can bypass SMSG_CONNECT_TO for accounts
# matching Bot.AccountPrefix. Golden captures must exercise stock realm→instance
# routing, so disable that shortcut inside the already-backed-up config. The
# EXIT trap restores the original value and file metadata exactly.
if grep -qE '^[[:space:]]*Bot\.AccountPrefix[[:space:]]*=' "$CPP_CONF"; then
  sed -i -E 's|^[[:space:]]*Bot\.AccountPrefix[[:space:]]*=.*|Bot.AccountPrefix = ""|' "$CPP_CONF"
else
  printf '\nBot.AccountPrefix = ""\n' >>"$CPP_CONF"
fi

CPP_CAPTURE_EFFECTIVE_CONFIG_PATH="$(realpath -e -- "$CPP_CONF" 2>/dev/null)" || {
  echo "error: cannot canonicalize effective C++ capture config" >&2
  exit 1
}
CPP_CAPTURE_EFFECTIVE_CONFIG_SHA256="$(cpp_capture_effective_config_sha256)" || {
  echo "error: cannot hash the canonical redacted effective C++ capture config" >&2
  exit 1
}
rm -f "${CPP_LOGS_DIR}/${PKT_NAME}"

echo "swapping to C++ world server..."
CAPTURE_SWAPPED=1
pm2 stop "$PM2_RUST_WORLD" >/dev/null 2>&1
capture_wait_for_world_stopped "$PM2_RUST_WORLD" "$RUST_ORIGINAL_IDENTITY" || {
  echo "error: ${PM2_RUST_WORLD} PID/PM2 entry or ports ${CPP_WORLD_PORT}/${CPP_INSTANCE_PORT} remain active after stop" >&2
  exit 1
}
capture_pm2_process_stopped "$PM2_CPP_WORLD" || {
  echo "error: ${PM2_CPP_WORLD} changed state before fixture mutation" >&2
  exit 1
}
if [ "$CPP_CAPTURE_LOOT_FIXTURE_GUARD" = "1" ]; then
  loot_fixture_wait_until_all_characters_offline
  apply_creature_health_fixture_guard
fi
if [ "$FLOW" = "creature-spell-casting" ]; then
  creature_spell_fixture_apply_guard
fi
if [ "$FLOW" = "detour-chase-around-obstacle" ]; then
  detour_chase_apply_fixture_guard
fi
pm2 start "$PM2_CPP_WORLD"
CPP_CAPTURE_IDENTITY="$(capture_wait_for_world_ready "$PM2_CPP_WORLD")" || {
  echo "error: ${PM2_CPP_WORLD} did not become one stable PID owning both configured listeners" >&2
  exit 1
}
IFS=$'\t' read -r CPP_CAPTURE_PM2_ENTRY_PID CPP_CAPTURE_PID \
  <<<"$CPP_CAPTURE_IDENTITY"
[[ "$CPP_CAPTURE_PM2_ENTRY_PID" =~ ^[1-9][0-9]*$ \
  && "$CPP_CAPTURE_PID" =~ ^[1-9][0-9]*$ ]] || {
  echo "error: ${PM2_CPP_WORLD} returned an invalid PM2-parent/listener identity" >&2
  exit 1
}
accredit_cpp_capture_executable || {
  echo "error: ${PM2_CPP_WORLD} live /proc executable does not match the pinned C++ path/SHA-256" >&2
  exit 1
}

CPP_CAPTURE_BOT_READY=1
if [ "$FLOW" = "detour-chase-around-obstacle" ]; then
  DETOUR_FIXTURE_BOT_READY=1
fi
echo
if [ "$FLOW" = "creature-spell-casting" ]; then
  echo ">>> Run the pinned bot once while this capture world is active:"
  printf '>>> WOW_BOT_REPORT=%q %q --creature-spell-capture --single %q --creature-spell-fixture-manifest %q\n' \
    "$WOW_BOT_REPORT" "$WOW_BOT_EXEC" "$CREATURE_SPELL_FIXTURE_ACCOUNT" \
    "$CREATURE_SPELL_FIXTURE_MANIFEST"
else
  echo ">>> Perform the '${FLOW}' flow with the client now."
fi
read -r -p ">>> Press ENTER when the flow is complete to collect the capture... " _

[ "$(capture_world_ready_once "$PM2_CPP_WORLD")" = "$CPP_CAPTURE_IDENTITY" ] || {
  echo "error: ${PM2_CPP_WORLD} PID/listener identity changed during capture" >&2
  exit 1
}
cpp_capture_executable_unchanged || {
  echo "error: ${PM2_CPP_WORLD} executable path/bytes changed during capture" >&2
  exit 1
}

CPP_PKT_SOURCE="${CPP_LOGS_DIR}/${PKT_NAME}"
[ -f "$CPP_PKT_SOURCE" ] && [ ! -L "$CPP_PKT_SOURCE" ] \
  && [ "$(realpath -e -- "$CPP_PKT_SOURCE" 2>/dev/null)" \
    = "$CPP_PKT_SOURCE" ] || {
  echo "error: C++ packet logger output is missing, non-canonical, or a symlink" >&2
  exit 1
}
capture_require_canonical_directory "$OUT_DIR" \
  && [ ! -e "$OUT_PKT" ] && [ ! -L "$OUT_PKT" ] \
  && [ ! -e "$OUT_MANIFEST" ] && [ ! -L "$OUT_MANIFEST" ] || {
  echo "error: C++ capture output path changed or became unsafe during capture" >&2
  exit 1
}
OUT_PKT_STAGE="$(mktemp "${OUT_DIR}/.cpp.pkt.partial.XXXXXX")"
cp -f -- "$CPP_PKT_SOURCE" "$OUT_PKT_STAGE"
CAPTURE_ARTIFACT_READY=1
echo "packet artifact staged; it will publish only after guarded cleanup succeeds"
echo "next (after this command exits 0): crates/capture-diff/scripts/capture-rust.sh ${FLOW}"
