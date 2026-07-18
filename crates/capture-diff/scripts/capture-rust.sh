#!/usr/bin/env bash
# capture-rust.sh — record a RustyCore packet dump for one flow.
#
# Restarts the RustyCore world server with RUSTYCORE_PACKET_DUMP_DIR pointed at a
# fresh directory, pauses for you to perform the flow with a client, then leaves
# the dump in place and restarts the server cleanly.
#
# Usage:   crates/capture-diff/scripts/capture-rust.sh <flow> [--yes]
# Output:  target/captures/<flow>/rust/   (gitignored; .bin/.meta per packet)
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
CAPTURE_EXEC=""
CAPTURE_EXEC_SHA256=""

sha256_of_file() {
  local output digest
  output="$(sha256sum <"$1")" || return 1
  digest="${output%% *}"
  [[ "$digest" =~ ^[0-9a-f]{64}$ ]] || return 1
  printf '%s\n' "$digest"
}

capture_exec_source_matches() {
  [ -n "$CAPTURE_EXEC" ] || return 0

  local canonical digest
  canonical="$(realpath -e -- "$CAPTURE_EXEC" 2>/dev/null)" || return 1
  [ "$canonical" = "$CAPTURE_EXEC" ] || return 1
  [ -f "$CAPTURE_EXEC" ] && [ -x "$CAPTURE_EXEC" ] || return 1
  digest="$(sha256_of_file "$CAPTURE_EXEC")" || return 1
  [ "$digest" = "$CAPTURE_EXEC_SHA256" ]
}

capture_process_exec_matches() {
  local identity="$1"
  [ -n "$CAPTURE_EXEC" ] || return 0

  local pid proc_exe live_exec source_digest live_digest
  [[ "$identity" == *$'\t'* ]] || return 1
  pid="${identity%%$'\t'*}"
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
  [ -f "$CAPTURE_EXEC" ] && [ -x "$CAPTURE_EXEC" ] || {
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
elif [ -n "$RUST_CAPTURE_EXEC_SHA256" ]; then
  echo "error: RUST_CAPTURE_EXEC_SHA256 requires RUST_CAPTURE_EXEC" >&2
  exit 1
fi

DUMP_DIR="${REPO_ROOT}/target/captures/${FLOW}/rust"

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

echo "flow      : ${FLOW}"
echo "dump dir  : ${DUMP_DIR}"
echo "pm2 world : ${PM2_RUST_WORLD}"
echo
echo "This will restart ${PM2_RUST_WORLD} with RUSTYCORE_PACKET_DUMP_DIR set."

if [ "$CONFIRM" != "--yes" ]; then
  read -r -p "Proceed? [y/N] " ans
  [ "$ans" = "y" ] || [ "$ans" = "Y" ] || { echo "aborted"; exit 1; }
fi

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

rust_world_ports_ready() {
  ss -H -ltn | rg -q ":${RUST_WORLD_PORT}\\b" \
    && ss -H -ltn | rg -q ":${RUST_INSTANCE_PORT}\\b"
}

cleanup() {
  local capture_status=$?
  trap - EXIT
  trap '' HUP INT TERM
  if [ "$CAPTURE_MUTATED" -eq 0 ]; then
    rm -f "$RESTORE_FILE" "$CAPTURE_CONFIG_FILE"
    exit "$capture_status"
  fi
  echo "recreating ${PM2_RUST_WORLD} without RUSTYCORE_PACKET_DUMP_DIR..."
  set +e
  if [ "$RESTORE_READY" -ne 1 ]; then
    echo "WARNING: PM2 restore snapshot is incomplete; inspect PM2 manually" >&2
    rm -f "$RESTORE_FILE" "$CAPTURE_CONFIG_FILE"
    exit 1
  fi
  unset RUSTYCORE_PACKET_DUMP_DIR
  local restore_status=0
  if ! pm2 delete "$PM2_RUST_WORLD" >/dev/null 2>&1; then
    restore_status=1
  elif ! env -i \
      HOME="$HOME" \
      PATH="$PATH" \
      PM2_HOME="${PM2_HOME:-$HOME/.pm2}" \
      "$PM2_BIN" start "$RESTORE_FILE" --only "$PM2_RUST_WORLD" >/dev/null 2>&1; then
    restore_status=1
  else
    restore_status=1
    local last_identity=""
    local stable_samples=0
    local identity=""
    for _ in $(seq 1 40); do
      if identity="$(snapshot_process_identity "$RESTORE_FILE" 2>/dev/null)" \
          && rust_world_ports_ready; then
        if [ "$identity" = "$last_identity" ]; then
          stable_samples=$((stable_samples + 1))
        else
          last_identity="$identity"
          stable_samples=1
        fi
        if [ "$stable_samples" -ge 4 ]; then
          restore_status=0
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
    exit 1
  fi
  rm -f "$RESTORE_FILE" "$CAPTURE_CONFIG_FILE"
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
if ! jq \
    --arg dump_dir "$DUMP_DIR" \
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
RESTORE_READY=1

# Recheck immediately before touching either server. The earlier validation is
# intentionally repeated because confirmation and snapshotting may take time.
if ! capture_exec_source_matches; then
  echo "error: RUST_CAPTURE_EXEC changed or no longer matches its pinned SHA-256" >&2
  exit 1
fi

# Make sure the C++ swap server is not holding the ports.
pm2 stop "$PM2_CPP_WORLD" >/dev/null 2>&1 || true

# Fresh dump directory so the capture only contains this flow.
rm -rf "$DUMP_DIR"
mkdir -p "$DUMP_DIR"

echo "recreating ${PM2_RUST_WORLD} from the clean snapshot with dump enabled..."
CAPTURE_MUTATED=1
pm2 delete "$PM2_RUST_WORLD" >/dev/null
env -i \
  HOME="$HOME" \
  PATH="$PATH" \
  PM2_HOME="${PM2_HOME:-$HOME/.pm2}" \
  "$PM2_BIN" start "$CAPTURE_CONFIG_FILE" --only "$PM2_RUST_WORLD" >/dev/null

CAPTURE_READY=0
CAPTURE_IDENTITY=""
for _ in $(seq 1 40); do
  if CAPTURE_IDENTITY="$(snapshot_process_identity "$CAPTURE_CONFIG_FILE" 2>/dev/null)" \
      && rust_world_ports_ready \
      && capture_process_exec_matches "$CAPTURE_IDENTITY"; then
    CAPTURE_READY=1
    break
  fi
  sleep 0.25
done
[ "$CAPTURE_READY" -eq 1 ] || {
  echo "error: ${PM2_RUST_WORLD} did not start online with the pinned capture executable and packet dumping enabled" >&2
  exit 1
}

echo
echo ">>> Perform the '${FLOW}' flow with the client now."
read -r -p ">>> Press ENTER when the flow is complete to finish the capture... " _

FINAL_CAPTURE_IDENTITY=""
if ! FINAL_CAPTURE_IDENTITY="$(snapshot_process_identity "$CAPTURE_CONFIG_FILE" 2>/dev/null)" \
    || ! rust_world_ports_ready \
    || ! capture_process_exec_matches "$FINAL_CAPTURE_IDENTITY"; then
  echo "error: ${PM2_RUST_WORLD} changed configuration, executable provenance, or stopped serving during capture" >&2
  exit 1
fi
if [ "$FINAL_CAPTURE_IDENTITY" != "$CAPTURE_IDENTITY" ]; then
  echo "error: ${PM2_RUST_WORLD} restarted during capture; refusing a mixed packet dump" >&2
  exit 1
fi

COUNT=$(find "$DUMP_DIR" -name '*.meta' | wc -l)
echo "collected ${COUNT} packets -> ${DUMP_DIR}"
echo
echo "diff against the C++ golden:"
echo "  cargo run -p capture-diff -- diff ${FLOW} --rust ${DUMP_DIR}"
