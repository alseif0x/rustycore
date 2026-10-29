#!/usr/bin/env bash
# Guarded live runtime QA for RustyCore.
#
# #331 retired tools/pr-preflight.sh, and with it the orchestration around the
# destructive loot-race smoke: it drove PM2, which this host no longer runs.
# #334 rebuilds it against the real process model - systemd units started from
# a deploy directory - and keeps the property that made the old wrapper safe:
# the live build is snapshotted before anything is swapped in, and it is put
# back afterwards even when the run fails, is interrupted, or the bot hangs.
#
# Validation lives in tools/validation-v2; code review in tools/local-review.sh.
# This starts and stops a live server, so it never runs in CI.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Injection points. Production defaults address the real host; the self-test
# points them at fakes so the restore path is exercised, not asserted.
QA_SYSTEMCTL="${QA_SYSTEMCTL:-sudo -n systemctl}"
QA_PRIVILEGED="${QA_PRIVILEGED:-sudo -n}"
QA_SERVICE="${QA_SERVICE:-world-server}"
QA_LIVE_DIR="${QA_LIVE_DIR:-$REPO_ROOT/target/deploy/live}"
QA_LIVE_NAME="${QA_LIVE_NAME:-world-server}"
QA_BOT="${QA_BOT:-$REPO_ROOT/tools/wow-test-bot/target/debug/wow-test-bot}"
QA_BOT_DIR="${QA_BOT_DIR:-$REPO_ROOT/tools/wow-test-bot}"
# The scenario runs through the maintained wrapper, which owns the bot's
# environment contract - credentials, working directory, mode selection - rather
# than this script re-deriving it (#349).
QA_SMOKE="${QA_SMOKE:-$REPO_ROOT/tools/wow-test-bot/run_rustycore_login_smoke.sh}"
# Fixture recovery journals. The bot requires an absolute path under a real
# directory, unused by any previous run.
QA_JOURNAL_DIR="${QA_JOURNAL_DIR:-/tmp/rustycore-loot-race-qa}"
QA_LOCK="${QA_LOCK:-/tmp/rustycore-qa-runtime.lock}"
QA_WORLD_PORT="${QA_WORLD_PORT:-8085}"
QA_INSTANCE_PORT="${QA_INSTANCE_PORT:-8086}"
QA_READY_TIMEOUT_SECONDS="${QA_READY_TIMEOUT_SECONDS:-60}"
QA_BOT_TIMEOUT_SECONDS="${QA_BOT_TIMEOUT_SECONDS:-600}"
QA_SKIP_PORT_GUARD="${QA_SKIP_PORT_GUARD:-0}"
# How the running executable of a PID is resolved. Overridden only by the
# self-test, which has no /proc entry pointing at its fake build.
QA_EXE_RESOLVER="${QA_EXE_RESOLVER:-realpath -e --}"
QA_PROCESS_ENV_READER="${QA_PROCESS_ENV_READER:-proc_environment}"
QA_PROC_STARTTIME_READER="${QA_PROC_STARTTIME_READER:-proc_starttime}"
# Whether a PID still exists. Overridden only by the self-test, whose fake
# service PID is not a real process.
QA_PROCESS_ALIVE="${QA_PROCESS_ALIVE:-pid_exists}"
# Which worktree must be clean for the swapped build to be identifiable. The
# self-test points this at its own clean fixture repository.
QA_GIT_DIR="${QA_GIT_DIR:-$REPO_ROOT}"
# Per-account bot passwords. The login-smoke wrapper loads this file; the bot
# binary does not, so a run that invokes the binary directly must.
QA_ENV_FILE="${QA_ENV_FILE:-$REPO_ROOT/tools/wow-test-bot/.env.local}"
QA_CAPTURE_FILEOPS="${QA_CAPTURE_FILEOPS:-$REPO_ROOT/tools/qa/capture_fileops.py}"
QA_CAPTURE_WORLD_CONFIG="${QA_CAPTURE_WORLD_CONFIG:-${QA_CAPTURE_CPP_CONFIG:-/home/server/trinity-legacy-install/etc/worldserver.conf}}"
QA_CAPTURE_CPP_CONFIG="${QA_CAPTURE_CPP_CONFIG:-$QA_CAPTURE_WORLD_CONFIG}"
QA_SYSTEMD_RUNTIME_DIR="${QA_SYSTEMD_RUNTIME_DIR:-/run/systemd/system}"
# The deterministic loot fixtures. `loot-item` kills one creature, so the guard
# lowers that creature's HealthModifier before the world starts and restores it
# afterwards. The guard itself is the shared, PM2-free one the capture wrappers
# use; #373 makes this the second caller rather than a second copy.
QA_FIXTURE_GUARD="${QA_FIXTURE_GUARD:-$REPO_ROOT/crates/capture-diff/scripts/loot-fixture-common.sh}"
QA_LOOT_FIXTURE_DB_CONF="${QA_LOOT_FIXTURE_DB_CONF:-/home/server/trinity-legacy-install/etc/worldserver.conf}"
QA_LOOT_FIXTURE_ENTRY="${QA_LOOT_FIXTURE_ENTRY:-21779}"
QA_LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER="${QA_LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER:-1}"
QA_LOOT_FIXTURE_TEMP_HEALTH_MODIFIER="${QA_LOOT_FIXTURE_TEMP_HEALTH_MODIFIER:-0.0001}"
# The chest scenario's wrapper-owned spawn. The shared guard pins this exact
# spawn row (position, template 2846, loot 2278 / item 38), so it is a fixed
# identity rather than a knob: the bot and the capture wrapper use the same id.
QA_LOOT_RACE_CHEST_SPAWN=9106001

DRY_RUN=0
ALLOW_RUNTIME_QA=0
ACK_LOOT_RACE=0
ACK_TRADE=0
WORLD_EXEC=""
REPORT=""
CAPTURE_BACKEND=""
CAPTURE_OUTPUT_DIR=""
CAPTURE_CANDIDATE_SHA=""
CAPTURE_RUNNER=""
CAPTURE_SOURCE_REVISION=""
CAPTURE_DERIVATION=""
CAPTURE_SOURCE_PATH=""

RESTORE_PENDING=0
RESTORE_FROM=""
ORIGINAL_SHA=""
LIVE_PATH=""
# The last world PID this run saw serving; a stop is verified against it.
WORLD_PID=""
# The final report is written by the restore trap, after the restore, so a
# run whose restore failed can never be reported as passed.
RUN_CANDIDATE_SHA=""
RUN_CANDIDATE_PATH=""
RUN_BOT_STATUS=null
RUN_CAPTURE_EXIT=null
RUN_CAPTURE_STATUS=0
RUN_CAPTURE_ERROR=""
CAPTURE_RUNNER_SHA=""
CAPTURE_BOT_SHA=""
CAPTURE_SOURCE_HEAD=""
CAPTURE_SOURCE_DIRTY=false
CAPTURE_CONFIG_BEFORE_SHA=""
CAPTURE_CONFIG_ACTIVE_SHA=""
CAPTURE_JOURNAL_PATH=""
CAPTURE_BACKUP_PATH=""
CAPTURE_CONFIG_ARMED=0
CAPTURE_SCENARIO_REPORT_SHA=""
CAPTURE_ARCH=""
CAPTURE_CONFIG_RESTORED=1
CAPTURE_RUNTIME_RESTORED=0
CAPTURE_BEFORE_PID=""
CAPTURE_BEFORE_STARTTIME=""
CAPTURE_BEFORE_RESTARTS=""
CAPTURE_AFTER_PID=""
CAPTURE_AFTER_STARTTIME=""
CAPTURE_AFTER_RESTARTS=""

# Globals the shared fixture guard reads. It restores only what it armed, so a
# run that never reached the mutation leaves these alone.
LOOT_FIXTURE_DB_CONF=""
LOOT_FIXTURE_ENTRY=""
LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER=""
LOOT_FIXTURE_TEMP_HEALTH_MODIFIER=""
LOOT_FIXTURE_SNAPSHOT_READY=0
LOOT_FIXTURE_CLEANUP_MARKER=""
LOOT_FIXTURE_CHEST_TEMPLATE_ENTRY=2846
LOOT_FIXTURE_CHEST_LOOT_ENTRY=2278
LOOT_FIXTURE_CHEST_ITEM=38
LOOT_FIXTURE_CHEST_GUID="$QA_LOOT_RACE_CHEST_SPAWN"
LOOT_FIXTURE_CHEST_ADDON_FACTION=101
LOOT_FIXTURE_CHEST_ADDON_RESTORE_READY=0
LOOT_FIXTURE_CHEST_SPAWN_DELETE_READY=0
LOOT_FIXTURE_CHEST_RESPAWN_DELETE_READY=0
# Set before the chest guard's first write; the restore trap then runs the
# guard's fail-closed cleanup while the world is stopped (#373).
LOOT_FIXTURE_CHEST_ARMED=0

usage() {
  cat <<'USAGE'
RustyCore guarded runtime QA

Usage:
  ./tools/qa-runtime.sh [OPTIONS] <COMMAND>

Commands:
  self-test     Exercise every guard and the restore path against fake services.
  snapshot      Print the live world build identity and exit. Touches nothing.
  login         Swap in a build, verify login/world entry, restore. No fixture setup.
  capture       Collect native Rust (.bin/.meta) or C++ (.pkt) packets around an
                explicit scenario runner, then restore the original runtime/config.
  loot-race     Swap in a build, install the chest fixture with the world stopped,
                run the destructive two-session chest smoke, then restore the
                fixture (world stopped) and the build.
  loot-item     Swap in a build, guard one creature's health, run the destructive
                creature-kill capture, restore both the fixture and the build.
  trade         Swap in a build, run the two-session player-trade smoke (the bot
                seeds and restores its own character fixture under a recovery
                journal), then restore the build.

Options:
  --dry-run                              Print the plan; start, stop and copy nothing.
  --allow-runtime-qa                     Required: this stops and starts a live service.
  --ack-disposable-overworld-loot-race   Required for loot-race: it mutates a world
                                         GameObject fixture and two disposable characters.
  --ack-disposable-trade                 Required for trade: it mutates two disposable
                                         characters (money, position, one item).
  --world-exec PATH                      Build to swap in (existing modes default to
                                         target/release; capture defaults to target/debug).
  --report PATH                          Write a JSON result document.
  --backend rust|cpp                     Required by capture.
  --output-dir ABSOLUTE_PATH             Fresh private 0700 capture directory.
  --candidate-sha256 SHA256              Required pinned candidate executable hash.
  --scenario-runner ABSOLUTE_PATH        Required executable runner; hashed before swap.
  --source-revision REV                  Required for C++ candidate provenance.
  --derivation TEXT                      Required for C++ candidate provenance.
  -h, --help                             Show this help.

The live build is snapshotted by path and SHA-256 before the swap and restored
afterwards on every exit path. A failed restore is reported as the run's outcome
even when the QA itself passed, because the normal server is then not the build
it was.
USAGE
}

log() { printf '\n==> %s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

require_command() {
  command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"
}

sha256_of() {
  sha256sum "$1" | awk '{print $1}'
}

service_property() {
  # shellcheck disable=SC2086
  $QA_SYSTEMCTL show "$QA_SERVICE" --property="$1" --value 2>/dev/null || true
}

# /proc, not kill -0: a live world owned by another user answers EPERM.
pid_exists() {
  [[ -e "/proc/$1" ]]
}

service_active() {
  [[ "$(service_property ActiveState)" == "active" ]]
}

# Stopped means: systemd reports the unit inactive (or failed) with no main
# PID, the PID it last served from is gone, and nothing listens on the world
# or instance port - the same absence capture-rust.sh requires before it
# touches the database. An unreadable state is not stopped.
world_stopped() {
  local pid="${1:-}" state main_pid sockets
  state="$(service_property ActiveState)"
  [[ "$state" == inactive || "$state" == failed ]] || return 1
  main_pid="$(service_property MainPID)"
  [[ -z "$main_pid" || "$main_pid" == 0 ]] || return 1
  if [[ "$pid" =~ ^[1-9][0-9]*$ ]]; then
    # shellcheck disable=SC2086
    ! $QA_PROCESS_ALIVE "$pid" 2>/dev/null || return 1
  fi
  ((QA_SKIP_PORT_GUARD == 1)) && return 0
  sockets="$(ss -H -ltn)" || return 1
  ! grep -Eq ":(${QA_WORLD_PORT}|${QA_INSTANCE_PORT})([^0-9]|$)" <<<"$sockets"
}

# Stop the service and prove it stopped. Returns non-zero without retrying.
stop_world_verified() {
  # shellcheck disable=SC2086
  $QA_SYSTEMCTL stop "$QA_SERVICE" || return 1
  world_stopped "$WORLD_PID"
}

service_main_pid() {
  local pid
  pid="$(service_property MainPID)"
  [[ "$pid" =~ ^[1-9][0-9]*$ ]] || return 1
  printf '%s' "$pid"
}

# Verbatim from the retired wrapper: every listening socket on the port must be
# owned by exactly the process we think is serving it.
port_owned_exclusively_by_pid() {
  local port="$1" pid="$2" sockets socket remaining seen_pid matched_pid
  [[ "$pid" =~ ^[1-9][0-9]*$ ]] || return 1
  sockets="$(ss -H -ltnp "sport = :${port}" 2>/dev/null)" || return 1
  [[ -n "$sockets" ]] || return 1
  while IFS= read -r socket; do
    remaining="$socket"
    seen_pid=0
    while [[ "$remaining" =~ pid=([0-9]+), ]]; do
      matched_pid="${BASH_REMATCH[1]}"
      [[ "$matched_pid" == "$pid" ]] || return 1
      seen_pid=1
      remaining="${remaining#*"${BASH_REMATCH[0]}"}"
    done
    ((seen_pid == 1)) || return 1
  done <<<"$sockets"
}

ports_ready() {
  local pid="$1"
  ((QA_SKIP_PORT_GUARD == 1)) && return 0
  [[ "$QA_WORLD_PORT" != "$QA_INSTANCE_PORT" ]] || return 1
  port_owned_exclusively_by_pid "$QA_WORLD_PORT" "$pid" \
    && port_owned_exclusively_by_pid "$QA_INSTANCE_PORT" "$pid"
}

# A packet dump would capture the QA traffic to disk; the old wrapper refused to
# run with one configured, and so does this.
packet_dump_absent() {
  local pid environment
  environment="$(service_property Environment)"
  [[ "$environment" != *RUSTYCORE_PACKET_DUMP_DIR* ]] || return 1
  if pid="$(service_main_pid)" && [[ -r "/proc/${pid}/environ" ]]; then
    ! tr '\0' '\n' <"/proc/${pid}/environ" | grep -q '^RUSTYCORE_PACKET_DUMP_DIR='
  fi
}

wait_until_serving() {
  local deadline=$((SECONDS + QA_READY_TIMEOUT_SECONDS)) pid
  while ((SECONDS < deadline)); do
    if service_active && pid="$(service_main_pid)" && ports_ready "$pid"; then
      printf '%s' "$pid"
      return 0
    fi
    sleep 1
  done
  return 1
}

live_identity() {
  local pid exec_path live_sha
  service_active || die "$QA_SERVICE is not active; refusing to touch it"
  pid="$(service_main_pid)" || die "$QA_SERVICE reports no main PID"
  # shellcheck disable=SC2086
  exec_path="$($QA_EXE_RESOLVER "/proc/${pid}/exe" 2>/dev/null)" \
    || die "cannot resolve the running executable of PID $pid"
  [[ "$exec_path" == "$LIVE_PATH" ]] \
    || die "running executable $exec_path is not the deploy path $LIVE_PATH"
  live_sha="$(sha256_of "$LIVE_PATH")"
  printf '%s\t%s\t%s\t%s' "$pid" "$exec_path" "$live_sha" "$(service_property NRestarts)"
}

restore_live_build() {
  local status=$?
  trap - EXIT
  trap '' HUP INT TERM
  ((RESTORE_PENDING == 1)) || exit "$status"
  RESTORE_PENDING=0
  log "Restoring the original live build"
  local restore_status=0
  if ! stop_world_verified; then
    # The candidate may still be running. Removing the chest under a live map,
    # or copying over a running build, would leave a state nobody chose, so
    # nothing else is touched: the chest stays armed and the run fails loudly.
    warn "$QA_SERVICE did not stop (or is still active/listening); the candidate may still be running"
    if ((LOOT_FIXTURE_CHEST_ARMED == 1)); then
      warn "the chest fixture was NOT restored; spawn ${LOOT_FIXTURE_CHEST_GUID} is still installed (cleanup skipped while the world may run)"
    fi
    if ((LOOT_FIXTURE_SNAPSHOT_READY == 1)); then
      warn "the loot fixture was NOT restored; creature ${LOOT_FIXTURE_ENTRY} may still have HealthModifier ${LOOT_FIXTURE_TEMP_HEALTH_MODIFIER}"
    fi
    if [[ "${COMMAND:-}" == capture ]]; then
      CAPTURE_RUNTIME_RESTORED=0
      if ((CAPTURE_CONFIG_ARMED == 1)) \
        || [[ -e "$CAPTURE_JOURNAL_PATH" || -L "$CAPTURE_JOURNAL_PATH" ]]; then
        CAPTURE_CONFIG_RESTORED=0
      fi
      warn "native capture restoration is held; private recovery journal: $CAPTURE_JOURNAL_PATH"
    fi
    write_final_report restore-failed
    printf 'error: THE LIVE BUILD OR FIXTURE WAS NOT RESTORED CLEANLY. Original kept at %s\n' \
      "$RESTORE_FROM" >&2
    exit 70
  fi
  # A GameObject spawn is loaded at world start, so the chest fixture is
  # removed while no world runs: the restored build must not load it (#373).
  if ((LOOT_FIXTURE_CHEST_ARMED == 1)); then
    log "Restoring the chest fixture with the world stopped"
    if restore_shared_chest_fixture_guard; then
      LOOT_FIXTURE_CHEST_ARMED=0
      log "Chest fixture restored"
    else
      restore_status=1
      warn "the chest fixture was NOT restored; spawn ${LOOT_FIXTURE_CHEST_GUID} or its addon money may still be installed"
    fi
  fi
  if [[ "${COMMAND:-}" == capture ]]; then
    CAPTURE_CONFIG_RESTORED=1
    if [[ -e "$CAPTURE_JOURNAL_PATH" || -L "$CAPTURE_JOURNAL_PATH" ]]; then
      CAPTURE_CONFIG_RESTORED=0
      if capture_privileged restore --journal "$CAPTURE_JOURNAL_PATH"; then
        if [[ "$CAPTURE_BACKEND" == rust ]]; then
          # shellcheck disable=SC2086
          if $QA_SYSTEMCTL daemon-reload && capture_unit_env_absent; then
            CAPTURE_CONFIG_RESTORED=1
          else
            restore_status=1
            warn "capture drop-in was removed but systemd config is unresolved; journal: $CAPTURE_JOURNAL_PATH"
          fi
        else
          CAPTURE_CONFIG_RESTORED=1
        fi
      else
        restore_status=1
        warn "capture config drifted or could not be restored; private recovery journal: $CAPTURE_JOURNAL_PATH"
      fi
    elif ((CAPTURE_CONFIG_ARMED == 1)); then
      CAPTURE_CONFIG_RESTORED=0
      restore_status=1
      warn "capture recovery journal disappeared after config preparation began; leaving the service stopped: $CAPTURE_JOURNAL_PATH"
    fi
    if ((CAPTURE_CONFIG_RESTORED == 0)); then
      CAPTURE_RUNTIME_RESTORED=0
      write_final_report restore-failed
      printf 'error: NATIVE CAPTURE RESTORE FAILED. Original build kept at %s; recovery journal %s\n' \
        "$RESTORE_FROM" "$CAPTURE_JOURNAL_PATH" >&2
      exit 70
    fi
  fi
  if ((restore_status != 0)); then
    if [[ "${COMMAND:-}" == capture ]]; then
      write_final_report restore-failed
      printf 'error: NATIVE CAPTURE RESTORE FAILED. Original build kept at %s; recovery journal %s\n' \
        "$RESTORE_FROM" "$CAPTURE_JOURNAL_PATH" >&2
      exit 70
    fi
  fi
  cp -- "$RESTORE_FROM" "$LIVE_PATH" || restore_status=1
  if [[ "${COMMAND:-}" == capture ]]; then
    if ((restore_status != 0)) || [[ "$(sha256_of "$LIVE_PATH")" != "$ORIGINAL_SHA" ]]; then
      CAPTURE_RUNTIME_RESTORED=0
      write_final_report restore-failed
      printf 'error: NATIVE CAPTURE RESTORE FAILED. Original executable was not restored; recovery journal %s\n' \
        "$CAPTURE_JOURNAL_PATH" >&2
      exit 70
    fi
  fi
  # shellcheck disable=SC2086
  $QA_SYSTEMCTL start "$QA_SERVICE" || restore_status=1
  if [[ "$(sha256_of "$LIVE_PATH")" != "$ORIGINAL_SHA" ]]; then
    restore_status=1
    warn "restored build does not match the snapshot SHA-256"
  fi
  if ! wait_until_serving >/dev/null; then
    restore_status=1
    warn "$QA_SERVICE did not come back up within ${QA_READY_TIMEOUT_SECONDS}s"
  fi
  if [[ "${COMMAND:-}" == capture ]] && ((restore_status == 0)); then
    local restored_identity restored_sha
    if restored_identity="$(live_identity)"; then
      restored_sha="$(cut -f3 <<<"$restored_identity")"
      [[ "$restored_sha" == "$ORIGINAL_SHA" ]] || restore_status=1
    else
      restore_status=1
    fi
    if ! packet_dump_absent; then
      restore_status=1
      warn "original service restarted with a packet dump still configured"
    fi
    if ((restore_status == 0)) && [[ -e "$CAPTURE_JOURNAL_PATH" ]]; then
      if ! capture_privileged complete --journal "$CAPTURE_JOURNAL_PATH"; then
        restore_status=1
        warn "capture recovery journal could not be retired: $CAPTURE_JOURNAL_PATH"
      fi
    fi
    ((restore_status == 0)) && CAPTURE_RUNTIME_RESTORED=1
  fi
  # The world fixture is restored under the same trap as the build: a scenario
  # that mutates creature health and dies must not leave a one-hit-point
  # creature in the world (#373). The guard restores only what it armed.
  if ((LOOT_FIXTURE_SNAPSHOT_READY == 1)); then
    log "Restoring the loot fixture"
    if restore_creature_health_fixture_guard; then
      log "Loot fixture restored"
    else
      restore_status=1
      warn "the loot fixture was NOT restored; creature ${LOOT_FIXTURE_ENTRY} may still have HealthModifier ${LOOT_FIXTURE_TEMP_HEALTH_MODIFIER}"
    fi
  fi
  if ((restore_status != 0)); then
    write_final_report restore-failed
    printf 'error: THE LIVE BUILD OR FIXTURE WAS NOT RESTORED CLEANLY. Original kept at %s\n' \
      "$RESTORE_FROM" >&2
    exit 70
  fi
  if [[ "${COMMAND:-}" == capture ]]; then
    if ((status == 0 && RUN_CAPTURE_STATUS == 0)); then
      write_final_report captured-restored
    else
      write_final_report failed-restored
    fi
  else
    write_final_report "$([[ $status -eq 0 ]] && echo passed-restored || echo failed-restored)"
  fi
  log "Original build restored and serving"
  exit "$status"
}

# Load the shared fixture guard. It is source-only and documents the globals a
# caller must define; this sets them from the QA knobs above.
arm_loot_fixture_guard() {
  [[ -r "$QA_FIXTURE_GUARD" ]] || die "fixture guard is missing: $QA_FIXTURE_GUARD"
  [[ -r "$QA_LOOT_FIXTURE_DB_CONF" ]] \
    || die "fixture guard needs a readable world config: $QA_LOOT_FIXTURE_DB_CONF"
  LOOT_FIXTURE_DB_CONF="$QA_LOOT_FIXTURE_DB_CONF"
  LOOT_FIXTURE_ENTRY="$QA_LOOT_FIXTURE_ENTRY"
  LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER="$QA_LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER"
  LOOT_FIXTURE_TEMP_HEALTH_MODIFIER="$QA_LOOT_FIXTURE_TEMP_HEALTH_MODIFIER"
  # shellcheck source=/dev/null
  source "$QA_FIXTURE_GUARD"
  load_loot_fixture_database_credentials \
    || die "could not load fixture database credentials from $QA_LOOT_FIXTURE_DB_CONF"
}

write_report() {
  [[ -n "$REPORT" ]] || return 0
  printf '{"command":"%s","outcome":"%s","service":"%s","original_sha256":"%s","candidate_sha256":"%s","bot_status":%s}\n' \
    "$1" "$2" "$QA_SERVICE" "$ORIGINAL_SHA" "${3:-}" "${4:-null}" >"$REPORT"
}

# The outcome every swapping command reports, written from the restore trap.
write_final_report() {
  case "${COMMAND:-}" in
    login|loot-race|loot-item|trade)
      write_report "$COMMAND" "$1" "${RUN_CANDIDATE_SHA:-}" "${RUN_BOT_STATUS:-null}" ;;
    capture)
      write_capture_report "$1" ;;
  esac
}

proc_environment() {
  tr '\0' '\n' <"/proc/$1/environ"
}

proc_starttime() {
  awk '{ sub(/^.*\) /, ""); print $20 }' "/proc/$1/stat"
}

capture_privileged() {
  # shellcheck disable=SC2086
  $QA_PRIVILEGED python3 "$QA_CAPTURE_FILEOPS" "$@"
}

capture_unit_env_matches() {
  local expected="$1" environment token actual="" count=0
  environment="$(service_property Environment)"
  environment="${environment//\"/}"
  for token in $environment; do
    case "$token" in
      RUSTYCORE_PACKET_DUMP_DIR=*)
        count=$((count + 1))
        actual="${token#*=}"
        ;;
    esac
  done
  ((count == 1)) && [[ "$actual" == "$expected" ]]
}

capture_unit_env_absent() {
  local environment
  environment="$(service_property Environment)"
  [[ "$environment" != *RUSTYCORE_PACKET_DUMP_DIR* ]]
}

capture_process_env_matches() {
  local pid="$1" expected="$2" environment count
  environment="$("$QA_PROCESS_ENV_READER" "$pid" 2>/dev/null)" || return 1
  count="$(grep -c '^RUSTYCORE_PACKET_DUMP_DIR=' <<<"$environment" || true)"
  [[ "$count" == 1 ]] \
    && grep -Fxq "RUSTYCORE_PACKET_DUMP_DIR=$expected" <<<"$environment"
}

capture_source_identity() {
  local status
  CAPTURE_SOURCE_HEAD="$(git -C "$QA_GIT_DIR" rev-parse HEAD)" \
    || die "cannot identify the RustyCore source HEAD"
  status="$(git -C "$QA_GIT_DIR" status --porcelain=v1 --untracked-files=normal)" \
    || die "cannot inspect the RustyCore source worktree"
  if [[ -n "$status" ]]; then CAPTURE_SOURCE_DIRTY=true; else CAPTURE_SOURCE_DIRTY=false; fi
}

validate_capture_output_path() {
  [[ "$CAPTURE_OUTPUT_DIR" == /* ]] \
    || die "capture output directory must be an absolute path"
  [[ "$CAPTURE_OUTPUT_DIR" =~ ^/[A-Za-z0-9_./-]+$ ]] \
    || die "capture output directory contains unsupported path characters"
  [[ ! -e "$CAPTURE_OUTPUT_DIR" && ! -L "$CAPTURE_OUTPUT_DIR" ]] \
    || die "capture output directory must be fresh and must not already exist"
  local parent base resolved repo_real
  parent="$(dirname -- "$CAPTURE_OUTPUT_DIR")"
  base="$(basename -- "$CAPTURE_OUTPUT_DIR")"
  [[ -d "$parent" && "$base" != . && "$base" != .. ]] \
    || die "capture output parent must already be a directory"
  resolved="$(realpath -e -- "$parent")/$base"
  repo_real="$(realpath -e -- "$QA_GIT_DIR")"
  [[ "$resolved" == "$CAPTURE_OUTPUT_DIR" ]] \
    || die "capture output path must have canonical, non-symlink ancestors"
  [[ "$resolved" != "$repo_real" && "$resolved" != "$repo_real"/* ]] \
    || die "capture output directory must be outside the source worktree"
  if [[ "$CAPTURE_OUTPUT_DIR" == *[$'\n\r\t ']* ]]; then
    die "capture output directory cannot contain whitespace"
  fi
  return 0
}

create_capture_output_dir() {
  validate_capture_output_path
  mkdir -m 700 -- "$CAPTURE_OUTPUT_DIR"
  [[ -d "$CAPTURE_OUTPUT_DIR" && ! -L "$CAPTURE_OUTPUT_DIR" ]] \
    || die "capture output directory was not created safely"
  [[ "$(stat -c %a -- "$CAPTURE_OUTPUT_DIR")" == 700 ]] \
    || die "capture output directory mode is not 0700"
  local service_user service_uid
  service_user="$(service_property User)"
  if [[ -z "$service_user" ]]; then service_uid=0; else service_uid="$(id -u -- "$service_user")"; fi
  [[ "$(stat -c %u -- "$CAPTURE_OUTPUT_DIR")" == "$service_uid" \
    && "$(id -u)" == "$service_uid" ]] \
    || die "capture runner and $QA_SERVICE must share the output directory owner"
}

write_capture_report() {
  local outcome="$1" journal=null backup=null original=null scenario_report=null report temp inventory_file
  local config_restored_json=false runtime_restored_json=false
  report="$CAPTURE_OUTPUT_DIR/capture-report.json"
  inventory_file="$(mktemp "$CAPTURE_OUTPUT_DIR/.capture-inventory.XXXXXX")" || {
    warn "could not create private capture inventory at $CAPTURE_OUTPUT_DIR"; return 1;
  }
  if [[ -d "$CAPTURE_OUTPUT_DIR" && ! -L "$CAPTURE_OUTPUT_DIR" ]]; then
    python3 "$QA_CAPTURE_FILEOPS" inventory \
      --output-dir "$CAPTURE_OUTPUT_DIR" --backend "$CAPTURE_BACKEND" \
      >"$inventory_file" 2>/dev/null || printf '[]\n' >"$inventory_file"
  else
    printf '[]\n' >"$inventory_file"
  fi
  if [[ -n "$CAPTURE_JOURNAL_PATH" ]] \
    && { ((CAPTURE_CONFIG_ARMED == 1)) \
      || [[ -e "$CAPTURE_JOURNAL_PATH" || -L "$CAPTURE_JOURNAL_PATH" ]]; }; then
    journal="$CAPTURE_JOURNAL_PATH"
  fi
  [[ -n "$CAPTURE_BACKUP_PATH" && -e "$CAPTURE_BACKUP_PATH" ]] \
    && backup="$CAPTURE_BACKUP_PATH"
  [[ -n "$RESTORE_FROM" && -e "$RESTORE_FROM" ]] && original="$RESTORE_FROM"
  [[ -n "$CAPTURE_SCENARIO_REPORT_SHA" ]] && scenario_report="$CAPTURE_SCENARIO_REPORT_SHA"
  ((CAPTURE_CONFIG_RESTORED == 1)) && config_restored_json=true
  ((CAPTURE_RUNTIME_RESTORED == 1)) && runtime_restored_json=true
  [[ "$RUN_CAPTURE_EXIT" =~ ^[0-9]+$ ]] || RUN_CAPTURE_EXIT=null
  [[ "$RUN_CAPTURE_STATUS" =~ ^[0-9]+$ ]] || RUN_CAPTURE_STATUS=70
  temp="$CAPTURE_OUTPUT_DIR/.capture-report.tmp"
  ( umask 077
    jq -n \
      --arg outcome "$outcome" \
      --arg backend "$CAPTURE_BACKEND" \
      --arg source "https://github.com/alseif0x/rustycore.git" \
      --arg head "$CAPTURE_SOURCE_HEAD" \
      --argjson dirty "$CAPTURE_SOURCE_DIRTY" \
      --arg architecture "$CAPTURE_ARCH" \
      --arg service "$QA_SERVICE" \
      --arg original_path "$LIVE_PATH" \
      --arg original_sha256 "$ORIGINAL_SHA" \
      --arg candidate_path "$RUN_CANDIDATE_PATH" \
      --arg candidate_live_path "$LIVE_PATH" \
      --arg candidate_source_path "$CAPTURE_SOURCE_PATH" \
      --arg candidate_sha256 "$RUN_CANDIDATE_SHA" \
      --arg candidate_source_revision "$CAPTURE_SOURCE_REVISION" \
      --arg candidate_derivation "$CAPTURE_DERIVATION" \
      --arg runner_sha256 "$CAPTURE_RUNNER_SHA" \
      --arg runner_path "$CAPTURE_RUNNER" \
      --arg bot_sha256 "$CAPTURE_BOT_SHA" \
      --arg bot_path "$QA_BOT" \
      --arg scenario_report_sha256 "$scenario_report" \
      --argjson runner_exit "$RUN_CAPTURE_EXIT" \
      --argjson collection_exit "$RUN_CAPTURE_STATUS" \
      --arg collection_error "$RUN_CAPTURE_ERROR" \
      --arg before_pid "$CAPTURE_BEFORE_PID" \
      --arg before_starttime "$CAPTURE_BEFORE_STARTTIME" \
      --arg before_restarts "$CAPTURE_BEFORE_RESTARTS" \
      --arg after_pid "$CAPTURE_AFTER_PID" \
      --arg after_starttime "$CAPTURE_AFTER_STARTTIME" \
      --arg after_restarts "$CAPTURE_AFTER_RESTARTS" \
      --arg world_port "$QA_WORLD_PORT" \
      --arg instance_port "$QA_INSTANCE_PORT" \
      --arg config_before_redacted_sha256 "$CAPTURE_CONFIG_BEFORE_SHA" \
      --arg config_capture_redacted_sha256 "$CAPTURE_CONFIG_ACTIVE_SHA" \
      --argjson original_config_restored "$config_restored_json" \
      --argjson original_runtime_restored "$runtime_restored_json" \
      --argjson packet_dump_absent "$runtime_restored_json" \
      --arg recovery_journal "$journal" \
      --arg recovery_backup "$backup" \
      --arg recovery_original "$original" \
      --slurpfile raw_artifacts "$inventory_file" \
      '$raw_artifacts[0] as $raw_artifacts
      | {schema: "rustycore-native-capture-v1", collection_only: true,
        gameplay_acceptance: false, outcome: $outcome, backend: $backend,
        source: {repository: $source, head: $head, dirty: $dirty, architecture: $architecture},
        service: $service,
        original: {executable_path: $original_path, sha256: $original_sha256},
        candidate: {path: $candidate_path, live_path: $candidate_live_path,
                    source_path: $candidate_source_path, sha256: $candidate_sha256,
                    source_revision: $candidate_source_revision, derivation: $candidate_derivation},
        runner: {path: $runner_path, sha256: $runner_sha256,
                 scenario_report_sha256: (if $scenario_report_sha256 == "null" then null else $scenario_report_sha256 end)},
        bot: {path: $bot_path, sha256: $bot_sha256},
        runner_exit: $runner_exit, collection_exit: $collection_exit,
        collection_error: $collection_error,
        listener_before_runner: {pid: $before_pid, proc_starttime: $before_starttime,
                                 restart_count: $before_restarts},
        listener_after_runner: {pid: $after_pid, proc_starttime: $after_starttime,
                                restart_count: $after_restarts},
        route_ports: {world: $world_port, instance: $instance_port},
        config_redacted_sha256: {before: $config_before_redacted_sha256,
                                 capture: $config_capture_redacted_sha256},
        original_config_restored: $original_config_restored,
        original_runtime_restored: $original_runtime_restored,
        packet_dump_absent_after_restore: $packet_dump_absent,
        recovery_paths: {
          journal: (if $recovery_journal == "null" then null else $recovery_journal end),
          config_backup: (if $recovery_backup == "null" then null else $recovery_backup end),
          original_executable: (if $recovery_original == "null" then null else $recovery_original end)},
        raw_artifact_count: ($raw_artifacts | length), raw_artifacts: $raw_artifacts}' \
      >"$temp"
  ) || { warn "could not write private capture report at $report"; return 1; }
  chmod 600 -- "$temp" || return 1
  mv -f -- "$temp" "$report" || return 1
  rm -- "$inventory_file"
  log "Native capture collection report: $report"
}

# The fixture-journal directory must be a real directory owned by this run's
# rules before its mode is changed: chmod follows a symlink.
prepare_journal_dir() {
  [[ ! -L "$QA_JOURNAL_DIR" ]] \
    || die "fixture-journal directory must be a real directory: $QA_JOURNAL_DIR"
  mkdir -p -m 700 -- "$QA_JOURNAL_DIR"
  [[ -d "$QA_JOURNAL_DIR" && ! -L "$QA_JOURNAL_DIR" ]] \
    || die "fixture-journal directory must be a real directory: $QA_JOURNAL_DIR"
  chmod 700 -- "$QA_JOURNAL_DIR"
}

# Values are exported, never echoed: this file holds passwords.
load_bot_environment() {
  [[ -f "$QA_ENV_FILE" ]] || return 0
  set -a
  # shellcheck disable=SC1090
  source "$QA_ENV_FILE"
  set +a
}

have_bot_credentials() {
  local name
  for name in $(compgen -v | grep '^WOW_BOT_PASSWORD' || true); do
    [[ -n "${!name}" ]] && return 0
  done
  # A declared-but-empty password is not a credential.
  [[ -f "$QA_ENV_FILE" ]] && grep -qE '^WOW_BOT_PASSWORD[A-Z0-9_]*=.' "$QA_ENV_FILE"
}

require_clean_worktree() {
  [[ -z "$(git -C "$QA_GIT_DIR" status --porcelain=v1 --untracked-files=normal)" ]] \
    || die "runtime QA requires a clean worktree so the swapped build is identifiable"
}

run_snapshot() {
  LIVE_PATH="$QA_LIVE_DIR/$QA_LIVE_NAME"
  [[ -f "$LIVE_PATH" ]] || die "no live build at $LIVE_PATH"
  local identity
  identity="$(live_identity)"
  printf 'service      %s\n' "$QA_SERVICE"
  printf 'main pid     %s\n' "$(cut -f1 <<<"$identity")"
  printf 'executable   %s\n' "$(cut -f2 <<<"$identity")"
  printf 'sha256       %s\n' "$(cut -f3 <<<"$identity")"
  printf 'restarts     %s\n' "$(cut -f4 <<<"$identity")"
  packet_dump_absent || die "a packet dump directory is configured; refusing to run QA"
  printf 'packet dump  absent\n'
}

run_capture() {
  ((ALLOW_RUNTIME_QA == 1)) || die \
    "capture stops and starts $QA_SERVICE; rerun with --allow-runtime-qa"
  [[ "$CAPTURE_BACKEND" == rust || "$CAPTURE_BACKEND" == cpp ]] \
    || die "capture requires --backend rust or --backend cpp"
  [[ -n "$CAPTURE_OUTPUT_DIR" ]] || die "capture requires --output-dir"
  [[ "$CAPTURE_CANDIDATE_SHA" =~ ^[[:xdigit:]]{64}$ ]] \
    || die "capture requires a 64-digit --candidate-sha256 pin"
  [[ -n "$CAPTURE_RUNNER" && "$CAPTURE_RUNNER" == /* && -x "$CAPTURE_RUNNER" ]] \
    || die "capture requires an absolute executable --scenario-runner"
  [[ -z "$REPORT" ]] || die "capture writes its report inside --output-dir; omit --report"
  [[ -r "$QA_CAPTURE_FILEOPS" && -f "$QA_CAPTURE_FILEOPS" ]] \
    || die "capture file-operations helper is missing: $QA_CAPTURE_FILEOPS"
  [[ -x "$QA_BOT" ]] || die "QA bot is not built: $QA_BOT"
  [[ -x "$CAPTURE_RUNNER" ]] || die "scenario runner is not executable: $CAPTURE_RUNNER"
  if [[ "$CAPTURE_BACKEND" == cpp ]]; then
    [[ -n "$WORLD_EXEC" ]] \
      || die "C++ capture requires an explicit --world-exec reference executable"
    [[ -n "$CAPTURE_SOURCE_REVISION" && -n "$CAPTURE_DERIVATION" ]] \
      || die "C++ capture requires --source-revision and --derivation provenance"
    [[ "$QA_CAPTURE_WORLD_CONFIG" == /* && -f "$QA_CAPTURE_WORLD_CONFIG" ]] \
      || die "active C++ config is missing: $QA_CAPTURE_WORLD_CONFIG"
    CAPTURE_SOURCE_PATH="${CAPTURE_SOURCE_PATH:-/home/server/woltk-trinity-legacy}"
    [[ -d "$CAPTURE_SOURCE_PATH" \
      && "$(realpath -e -- "$CAPTURE_SOURCE_PATH")" == "$CAPTURE_SOURCE_PATH" ]] \
      || die "C++ source path must be a canonical existing directory"
  else
    CAPTURE_SOURCE_PATH="$QA_GIT_DIR"
  fi

  LIVE_PATH="$QA_LIVE_DIR/$QA_LIVE_NAME"
  local candidate="${WORLD_EXEC:-$REPO_ROOT/target/debug/world-server}"
  [[ -x "$candidate" ]] || die "candidate build is not executable: $candidate"
  [[ ! -L "$candidate" && "$(realpath -e -- "$candidate")" == "$candidate" ]] \
    || die "candidate build path must be canonical and not a symlink"
  [[ ! -L "$CAPTURE_RUNNER" && "$(realpath -e -- "$CAPTURE_RUNNER")" == "$CAPTURE_RUNNER" ]] \
    || die "scenario runner path must be canonical and not a symlink"
  [[ ! -L "$QA_BOT" && "$(realpath -e -- "$QA_BOT")" == "$QA_BOT" ]] \
    || die "QA bot path must be canonical and not a symlink"
  [[ -f "$LIVE_PATH" ]] || die "no live build at $LIVE_PATH"
  require_clean_worktree
  have_bot_credentials || die "no bot credentials before swapping a build"
  RUN_CANDIDATE_SHA="$(sha256_of "$candidate")"
  RUN_CANDIDATE_PATH="$candidate"
  CAPTURE_CANDIDATE_SHA="${CAPTURE_CANDIDATE_SHA,,}"
  [[ "$RUN_CANDIDATE_SHA" == "$CAPTURE_CANDIDATE_SHA" ]] \
    || die "candidate executable does not match the pinned SHA-256"
  CAPTURE_RUNNER_SHA="$(sha256_of "$CAPTURE_RUNNER")"
  CAPTURE_BOT_SHA="$(sha256_of "$QA_BOT")"
  capture_source_identity
  CAPTURE_ARCH="$(uname -m)"
  [[ "$CAPTURE_SOURCE_DIRTY" == false ]] \
    || die "runtime QA requires a clean worktree so the swapped build is identifiable"
  if [[ "$CAPTURE_BACKEND" == rust ]]; then
    CAPTURE_SOURCE_REVISION="$CAPTURE_SOURCE_HEAD"
    CAPTURE_DERIVATION="RustyCore source candidate"
  fi

  validate_capture_output_path
  local identity original_pid service_user service_uid
  identity="$(live_identity)"
  packet_dump_absent || die "a packet dump directory is configured; refusing to run QA"
  original_pid="$(cut -f1 <<<"$identity")"
  ORIGINAL_SHA="$(cut -f3 <<<"$identity")"
  WORLD_PID="$original_pid"
  ports_ready "$original_pid" || die "ports $QA_WORLD_PORT/$QA_INSTANCE_PORT are not owned by PID $original_pid"
  service_user="$(service_property User)"
  if [[ -z "$service_user" ]]; then service_uid=0; else service_uid="$(id -u -- "$service_user")"; fi
  [[ "$(id -u)" == "$service_uid" ]] \
    || die "capture must run as the same account as $QA_SERVICE for a private 0700 output directory"

  if ((DRY_RUN == 1)); then
    log "Dry run: nothing is stopped, copied, configured or started"
    printf 'backend        %s\n' "$CAPTURE_BACKEND"
    printf 'candidate      %s (%s)\n' "$candidate" "$RUN_CANDIDATE_SHA"
    printf 'runner         %s (%s)\n' "$CAPTURE_RUNNER" "$CAPTURE_RUNNER_SHA"
    printf 'bot sha256     %s\n' "$CAPTURE_BOT_SHA"
    printf 'output         %s (fresh mode 0700)\n' "$CAPTURE_OUTPUT_DIR"
    printf 'would restore  original executable and capture configuration\n'
    return 0
  fi

  create_capture_output_dir
  CAPTURE_JOURNAL_PATH="$CAPTURE_OUTPUT_DIR/.qa-capture-journal.json"
  if [[ "$CAPTURE_BACKEND" == cpp ]]; then
    CAPTURE_BACKUP_PATH="$CAPTURE_OUTPUT_DIR/.qa-original-worldserver.conf"
  fi
  if [[ "$CAPTURE_BACKEND" == rust ]]; then
    if ! capture_privileged preflight-rust --output-dir "$CAPTURE_OUTPUT_DIR" \
      --systemd-root "$QA_SYSTEMD_RUNTIME_DIR" --unit "$QA_SERVICE" \
      --config "$QA_CAPTURE_WORLD_CONFIG"; then
      rmdir -- "$CAPTURE_OUTPUT_DIR" 2>/dev/null || true
      die "Rust capture drop-in path is not available without overwriting existing state"
    fi
  elif ! capture_privileged preflight-cpp --output-dir "$CAPTURE_OUTPUT_DIR" \
    --config "$QA_CAPTURE_WORLD_CONFIG"; then
    rmdir -- "$CAPTURE_OUTPUT_DIR" 2>/dev/null || true
    die "active C++ config is not safe to update for capture"
  fi
  RUN_CAPTURE_EXIT=null
  RUN_CAPTURE_STATUS=0
  RUN_CAPTURE_ERROR=""
  CAPTURE_CONFIG_RESTORED=1
  CAPTURE_RUNTIME_RESTORED=0
  local evidence_dir snapshot_copy
  evidence_dir="$(mktemp -d "${TMPDIR:-/tmp}/rustycore-capture-qa.XXXXXX")"
  chmod 700 -- "$evidence_dir"
  RESTORE_FROM="$evidence_dir/original-world-server"
  snapshot_copy="$RESTORE_FROM"
  cp -- "$LIVE_PATH" "$snapshot_copy"
  [[ "$(sha256_of "$snapshot_copy")" == "$ORIGINAL_SHA" ]] \
    || die "original executable snapshot does not match"

  RESTORE_PENDING=1
  trap restore_live_build EXIT
  trap 'exit 130' HUP INT TERM
  log "Installing the pinned capture candidate $RUN_CANDIDATE_SHA"
  stop_world_verified \
    || die "$QA_SERVICE did not stop (still active, running or listening); refusing to swap the build"

  CAPTURE_CONFIG_ARMED=1
  CAPTURE_CONFIG_RESTORED=0
  local fileops_result
  if [[ "$CAPTURE_BACKEND" == rust ]]; then
    fileops_result="$(capture_privileged prepare-rust \
      --output-dir "$CAPTURE_OUTPUT_DIR" --systemd-root "$QA_SYSTEMD_RUNTIME_DIR" \
      --unit "$QA_SERVICE" --world-port "$QA_WORLD_PORT" \
      --instance-port "$QA_INSTANCE_PORT" --config "$QA_CAPTURE_WORLD_CONFIG")"
    IFS=$'\t' read -r CAPTURE_CONFIG_BEFORE_SHA CAPTURE_CONFIG_ACTIVE_SHA <<<"$fileops_result"
    # shellcheck disable=SC2086
    $QA_SYSTEMCTL daemon-reload
    capture_unit_env_matches "$CAPTURE_OUTPUT_DIR" \
      || die "systemd did not load exactly the expected Rust packet-capture environment"
  else
    fileops_result="$(capture_privileged prepare-cpp \
      --output-dir "$CAPTURE_OUTPUT_DIR" --config "$QA_CAPTURE_WORLD_CONFIG" \
      --world-port "$QA_WORLD_PORT" --instance-port "$QA_INSTANCE_PORT")"
    IFS=$'\t' read -r CAPTURE_CONFIG_BEFORE_SHA CAPTURE_CONFIG_ACTIVE_SHA <<<"$fileops_result"
  fi

  cp -- "$candidate" "$LIVE_PATH"
  [[ "$(sha256_of "$LIVE_PATH")" == "$RUN_CANDIDATE_SHA" ]] \
    || die "candidate did not install with its pinned SHA-256"
  # shellcheck disable=SC2086
  $QA_SYSTEMCTL start "$QA_SERVICE"
  local pid running_identity
  pid="$(wait_until_serving)" || die "the capture candidate did not start serving"
  WORLD_PID="$pid"
  running_identity="$(live_identity)"
  [[ "$(cut -f3 <<<"$running_identity")" == "$RUN_CANDIDATE_SHA" ]] \
    || die "the running capture executable does not match its pinned SHA-256"
  if [[ "$CAPTURE_BACKEND" == rust ]]; then
    capture_unit_env_matches "$CAPTURE_OUTPUT_DIR" \
      && capture_process_env_matches "$pid" "$CAPTURE_OUTPUT_DIR" \
      || die "Rust packet capture environment is not present exactly on the candidate process"
  else
    packet_dump_absent \
      || die "unexpected Rust packet-dump environment on the C++ candidate"
  fi

  CAPTURE_BEFORE_PID="$pid"
  CAPTURE_BEFORE_STARTTIME="$("$QA_PROC_STARTTIME_READER" "$pid")" \
    || die "cannot read candidate listener process start time"
  CAPTURE_BEFORE_RESTARTS="$(service_property NRestarts)"
  [[ "$CAPTURE_BEFORE_RESTARTS" =~ ^[0-9]+$ ]] \
    || die "cannot read candidate service restart count"

  log "Running the explicit native-capture scenario; report remains a collection record"
  local scenario_report="$CAPTURE_OUTPUT_DIR/scenario-report.json" runner_status=0
  [[ "$(sha256_of "$CAPTURE_RUNNER")" == "$CAPTURE_RUNNER_SHA" \
    && "$(sha256_of "$QA_BOT")" == "$CAPTURE_BOT_SHA" ]] \
    || die "scenario runner or bot changed after it was pinned"
  (
    set +x
    load_bot_environment
    cd "$QA_BOT_DIR"
    umask 077
    exec timeout --foreground --signal=TERM --kill-after=30 "${QA_BOT_TIMEOUT_SECONDS}s" env \
      WOW_BOT_ENV_FILE=/dev/null WOW_BOT_EXEC="$QA_BOT" WOW_BOT_EXEC_SHA256="$CAPTURE_BOT_SHA" \
      WOW_BOT_GENERATE_LOCAL_PASSWORD=0 WOW_BOT_ENSURE_TEST_ACCOUNTS=0 \
      WORLD_HOST=127.0.0.1 WORLD_PORT="$QA_WORLD_PORT" \
      INSTANCE_HOST=127.0.0.1 INSTANCE_PORT="$QA_INSTANCE_PORT" \
      WOW_BOT_REPORT="$scenario_report" \
      RUSTYCORE_CAPTURE_REPORT="$scenario_report" \
      RUSTYCORE_CAPTURE_DIR="$CAPTURE_OUTPUT_DIR" \
      RUSTYCORE_CAPTURE_BACKEND="$CAPTURE_BACKEND" \
      RUSTYCORE_CAPTURE_CANDIDATE_SHA256="$RUN_CANDIDATE_SHA" \
      "$CAPTURE_RUNNER"
  ) >"$CAPTURE_OUTPUT_DIR/runner.log" 2>&1 || runner_status=$?
  RUN_CAPTURE_EXIT="$runner_status"
  if [[ -L "$scenario_report" ]]; then
    RUN_CAPTURE_STATUS=65
    RUN_CAPTURE_ERROR="scenario_report_is_symlink"
  elif [[ -f "$scenario_report" ]]; then
    CAPTURE_SCENARIO_REPORT_SHA="$(sha256_of "$scenario_report")"
  fi

  local after_identity after_pid after_starttime after_restarts artifacts_json artifact_count
  if after_identity="$(live_identity)"; then
    after_pid="$(cut -f1 <<<"$after_identity")"
    CAPTURE_AFTER_PID="$after_pid"
    if ! ports_ready "$after_pid"; then
      RUN_CAPTURE_STATUS=66
      RUN_CAPTURE_ERROR="candidate_listeners_changed_during_runner"
    elif CAPTURE_AFTER_STARTTIME="$("$QA_PROC_STARTTIME_READER" "$after_pid")"; then
      CAPTURE_AFTER_RESTARTS="$(service_property NRestarts)"
      if [[ "$(cut -f3 <<<"$after_identity")" != "$RUN_CANDIDATE_SHA" \
        || "$CAPTURE_AFTER_PID" != "$CAPTURE_BEFORE_PID" \
        || "$CAPTURE_AFTER_STARTTIME" != "$CAPTURE_BEFORE_STARTTIME" \
        || "$CAPTURE_AFTER_RESTARTS" != "$CAPTURE_BEFORE_RESTARTS" ]]; then
        RUN_CAPTURE_STATUS=66
        RUN_CAPTURE_ERROR="candidate_listener_identity_changed_during_runner"
      fi
    else
      RUN_CAPTURE_STATUS=66
      RUN_CAPTURE_ERROR="candidate_listener_starttime_unavailable_after_runner"
    fi
  else
    RUN_CAPTURE_STATUS=66
    RUN_CAPTURE_ERROR="candidate_listener_unavailable_after_runner"
  fi
  if [[ "$CAPTURE_BACKEND" == rust ]]; then
    if ! capture_process_env_matches "$CAPTURE_AFTER_PID" "$CAPTURE_OUTPUT_DIR"; then
      RUN_CAPTURE_STATUS=66
      RUN_CAPTURE_ERROR="candidate_capture_environment_changed_during_runner"
    fi
  elif ! packet_dump_absent; then
    RUN_CAPTURE_STATUS=66
    RUN_CAPTURE_ERROR="unexpected_packet_dump_environment_during_cpp_capture"
  fi
  if ((runner_status != 0)); then
    RUN_CAPTURE_STATUS="$runner_status"
    RUN_CAPTURE_ERROR="scenario_runner_failed"
  elif ((RUN_CAPTURE_STATUS == 0)); then
    if artifacts_json="$(python3 "$QA_CAPTURE_FILEOPS" inventory \
      --output-dir "$CAPTURE_OUTPUT_DIR" --backend "$CAPTURE_BACKEND")"; then
      artifact_count="$(jq '[.[] | select(.size > 0)] | length' <<<"$artifacts_json")"
      if ((artifact_count == 0)); then
        RUN_CAPTURE_STATUS=65
        RUN_CAPTURE_ERROR="no_native_capture_artifacts"
      fi
    else
      RUN_CAPTURE_STATUS=65
      RUN_CAPTURE_ERROR="native_capture_inventory_failed"
    fi
  fi
  if ((RUN_CAPTURE_STATUS != 0)); then
    warn "capture collection status $RUN_CAPTURE_STATUS (runner exit $RUN_CAPTURE_EXIT); no gameplay acceptance is implied"
    return "$RUN_CAPTURE_STATUS"
  fi
  log "Native capture collection completed; runner exit $RUN_CAPTURE_EXIT"
  return 0
}

run_login() {
  ((ALLOW_RUNTIME_QA == 1)) || die \
    "login stops and starts $QA_SERVICE; rerun with --allow-runtime-qa"
  LIVE_PATH="$QA_LIVE_DIR/$QA_LIVE_NAME"
  local candidate="${WORLD_EXEC:-$REPO_ROOT/target/release/world-server}"
  [[ -x "$candidate" ]] || die "candidate build is not executable: $candidate"
  [[ -f "$LIVE_PATH" ]] || die "no live build at $LIVE_PATH"
  [[ -x "$QA_BOT" ]] || die "QA bot is not built: $QA_BOT"
  [[ -x "$QA_SMOKE" ]] || die "smoke wrapper is missing: $QA_SMOKE"
  require_clean_worktree
  have_bot_credentials || die "no bot credentials before swapping a build"
  RUN_CANDIDATE_SHA="$(sha256_of "$candidate")"
  RUN_BOT_STATUS=null
  if ((DRY_RUN == 1)); then
    log "Dry run: nothing is stopped, copied or started"
    printf 'would install   %s (%s)\n' "$candidate" "$RUN_CANDIDATE_SHA"
    printf 'would run       login-only with account provisioning and fixture modes disabled\n'
    printf 'would restore   the snapshot on every exit path\n'
    return 0
  fi

  local identity pid bot_sha evidence_dir
  identity="$(live_identity)"
  packet_dump_absent || die "a packet dump directory is configured; refusing to run QA"
  ORIGINAL_SHA="$(cut -f3 <<<"$identity")"
  pid="$(cut -f1 <<<"$identity")"
  WORLD_PID="$pid"
  ports_ready "$pid" || die "ports $QA_WORLD_PORT/$QA_INSTANCE_PORT are not owned by PID $pid"
  bot_sha="$(sha256_of "$QA_BOT")"
  evidence_dir="$(mktemp -d "${TMPDIR:-/tmp}/rustycore-login-qa.XXXXXX")"
  log "Login evidence directory: $evidence_dir"
  RESTORE_FROM="$evidence_dir/original-world-server"
  cp -- "$LIVE_PATH" "$RESTORE_FROM"
  [[ "$(sha256_of "$RESTORE_FROM")" == "$ORIGINAL_SHA" ]] || die "snapshot copy does not match"
  write_report login running "$RUN_CANDIDATE_SHA"
  RESTORE_PENDING=1
  trap restore_live_build EXIT
  trap 'exit 130' HUP INT TERM
  log "Installing the candidate build $RUN_CANDIDATE_SHA"
  stop_world_verified || die "$QA_SERVICE did not stop (still active, running or listening); refusing to swap the build"
  cp -- "$candidate" "$LIVE_PATH"
  [[ "$(sha256_of "$LIVE_PATH")" == "$RUN_CANDIDATE_SHA" ]] || die "candidate did not install"
  # shellcheck disable=SC2086
  $QA_SYSTEMCTL start "$QA_SERVICE"
  pid="$(wait_until_serving)" || die "the candidate build did not start serving"
  WORLD_PID="$pid"
  log "Candidate serving on PID $pid"

  RUN_BOT_STATUS=0
  (
    # Load credentials once, then pin the maintained wrapper's mode and binary.
    # Do not allow ignored defaults to silently turn this into fixture QA.
    set +x
    load_bot_environment
    local name
    while IFS= read -r name; do
      case "$name" in
        WOW_BOT_*_SMOKE|WOW_BOT_*_CAPTURE|WOW_BOT_ACK_*) export "$name=0" ;;
      esac
    done < <(compgen -v)
    # The bot parses this numeric override even in login-only mode. Empty is
    # not absence: remove it instead of exporting an invalid empty integer.
    unset WOW_BOT_STAND_STATE
    cd "$QA_BOT_DIR"
    exec timeout --foreground --signal=TERM --kill-after=30 "${QA_BOT_TIMEOUT_SECONDS}s" env \
      WOW_BOT_ENV_FILE=/dev/null WOW_BOT_EXEC="$QA_BOT" WOW_BOT_EXEC_SHA256="$bot_sha" \
      WOW_BOT_GENERATE_LOCAL_PASSWORD=0 WOW_BOT_ENSURE_TEST_ACCOUNTS=0 \
      WORLD_HOST=127.0.0.1 WORLD_PORT="$QA_WORLD_PORT" \
      INSTANCE_HOST=127.0.0.1 INSTANCE_PORT="$QA_INSTANCE_PORT" \
      WOW_BOT_REPORT="$evidence_dir/bot.json" WOW_BOT_LOG="$evidence_dir/bot.log" \
      "$QA_SMOKE"
  ) >"$evidence_dir/wrapper.log" 2>&1 || RUN_BOT_STATUS=$?
  if ((RUN_BOT_STATUS == 0)) && ! jq -e \
    '.login_only == true and (.results | length == 1) and
     all(.results[]; .world_auth == true and .enum_characters == true and .player_login_verified == true and .login_stream_drained == true)' \
    "$evidence_dir/bot.json" >/dev/null 2>&1; then
    warn "login wrapper returned success without verified world-entry evidence"
    RUN_BOT_STATUS=65
  fi
  log "Login bot status: $RUN_BOT_STATUS; evidence: $evidence_dir/bot.json"
  return "$RUN_BOT_STATUS"
}

run_loot_race() {
  ((ALLOW_RUNTIME_QA == 1)) || die \
    "loot-race stops and starts $QA_SERVICE; rerun with --allow-runtime-qa"
  ((ACK_LOOT_RACE == 1)) || die \
    "loot-race mutates a world GameObject fixture and two disposable characters; rerun with --ack-disposable-overworld-loot-race"

  LIVE_PATH="$QA_LIVE_DIR/$QA_LIVE_NAME"
  local candidate="${WORLD_EXEC:-$REPO_ROOT/target/release/world-server}"
  [[ -x "$candidate" ]] || die "candidate build is not executable: $candidate"
  [[ -f "$LIVE_PATH" ]] || die "no live build at $LIVE_PATH"
  [[ -x "$QA_BOT" ]] || die "QA bot is not built: $QA_BOT"
  [[ -x "$QA_SMOKE" ]] || die "smoke wrapper is missing: $QA_SMOKE"
  require_clean_worktree
  have_bot_credentials || die \
    "no bot credentials: set WOW_BOT_PASSWORD or provide $QA_ENV_FILE before swapping a build"
  arm_loot_fixture_guard

  local candidate_sha
  candidate_sha="$(sha256_of "$candidate")"
  RUN_CANDIDATE_SHA="$candidate_sha"

  if ((DRY_RUN == 1)); then
    log "Dry run: nothing is stopped, copied or started"
    printf 'would snapshot  %s\n' "$LIVE_PATH"
    printf 'would install   %s (%s)\n' "$candidate" "$candidate_sha"
    printf 'would guard     chest spawn %s (template %s, loot %s) with the world stopped\n' \
      "$LOOT_FIXTURE_CHEST_GUID" "$LOOT_FIXTURE_CHEST_TEMPLATE_ENTRY" "$LOOT_FIXTURE_CHEST_LOOT_ENTRY"
    printf 'would run       %s with WOW_BOT_LOOT_RACE_SMOKE=1\n' "$QA_SMOKE"
    printf 'would journal   a fresh path under %s\n' "$QA_JOURNAL_DIR"
    printf 'would restore   the snapshot on every exit path\n'
    return 0
  fi

  local identity
  identity="$(live_identity)"
  packet_dump_absent || die "a packet dump directory is configured; refusing to run QA"
  ORIGINAL_SHA="$(cut -f3 <<<"$identity")"
  local pid
  pid="$(cut -f1 <<<"$identity")"
  WORLD_PID="$pid"
  ports_ready "$pid" || die "ports $QA_WORLD_PORT/$QA_INSTANCE_PORT are not owned by PID $pid"
  log "Live build $ORIGINAL_SHA serving on PID $pid"

  RESTORE_FROM="$(mktemp "${TMPDIR:-/tmp}/rustycore-live-build.XXXXXX")"
  cp -- "$LIVE_PATH" "$RESTORE_FROM"
  [[ "$(sha256_of "$RESTORE_FROM")" == "$ORIGINAL_SHA" ]] || die "snapshot copy does not match"
  RESTORE_PENDING=1
  trap restore_live_build EXIT
  trap 'exit 130' HUP INT TERM
  write_report loot-race running "$candidate_sha"

  # The chest spawn must exist before the world starts: a GameObject spawn is
  # loaded with its map. The shared guard (also used by the PM2 capture
  # wrapper) fails closed on template/loot/addon drift or a pre-existing spawn,
  # and the EXIT trap removes exactly what it installed, world stopped (#373).
  # No database write happens until the world is proven stopped.
  stop_world_verified || die "$QA_SERVICE did not stop (still active, running or listening); refusing to install the chest fixture"
  log "Installing the chest fixture"
  loot_fixture_wait_until_all_characters_offline \
    || die "characters are still online after stopping $QA_SERVICE"
  LOOT_FIXTURE_CHEST_ARMED=1
  apply_shared_chest_fixture_guard || die "could not install the chest fixture"

  log "Installing the candidate build $candidate_sha"
  cp -- "$candidate" "$LIVE_PATH"
  [[ "$(sha256_of "$LIVE_PATH")" == "$candidate_sha" ]] || die "candidate did not install"
  # shellcheck disable=SC2086
  $QA_SYSTEMCTL start "$QA_SERVICE"
  pid="$(wait_until_serving)" || die "the candidate build did not start serving"
  WORLD_PID="$pid"
  log "Candidate serving on PID $pid"

  local bot_status=0
  load_bot_environment
  [[ -d "$QA_BOT_DIR" ]] || die "bot directory is missing: $QA_BOT_DIR"
  prepare_journal_dir
  local journal="$QA_JOURNAL_DIR/fixture-$$-$(date -u +%Y%m%dT%H%M%SZ).journal"
  [[ ! -e "$journal" && ! -e "${journal}.cleanup-complete" ]] \
    || die "fixture journal path is not fresh: $journal"
  log "Fixture recovery journal: $journal"
  ( cd "$QA_BOT_DIR" && exec timeout --foreground --signal=TERM --kill-after=30 \
      "${QA_BOT_TIMEOUT_SECONDS}s" env \
      WOW_BOT_LOOT_RACE_SMOKE=1 \
      WOW_BOT_ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=1 \
      WOW_BOT_FIXTURE_JOURNAL="$journal" \
      WOW_BOT_ENSURE_TEST_ACCOUNTS=0 \
      "$QA_SMOKE" ) || bot_status=$?
  # A journal that outlives its run means the world fixture is still mutated.
  if [[ -e "$journal" ]]; then
    warn "fixture journal $journal survived the run; the world fixture is still mutated"
    warn "recover it with: cd $QA_BOT_DIR && WOW_BOT_FIXTURE_JOURNAL=$journal ./target/debug/wow-test-bot --recover-loot-fixture"
    ((bot_status == 0)) && bot_status=75
  fi
  if ((bot_status == 0)); then
    log "Loot-race smoke passed"
  else
    warn "loot-race smoke failed with status $bot_status"
  fi
  # The final outcome is reported by the restore trap, after the restore.
  RUN_BOT_STATUS="$bot_status"
  return "$bot_status"
}

# One creature kill, under the guarded swap.
#
# #373: this is what #28's stop-condition probe needs and the chest scenario
# cannot provide. The fixture is a temporarily lowered HealthModifier on one
# creature template, applied before the candidate starts — the map reads it at
# startup — and restored by the same trap that restores the build.
run_loot_item() {
  ((ALLOW_RUNTIME_QA == 1)) || die \
    "loot-item stops and starts $QA_SERVICE; rerun with --allow-runtime-qa"
  ((ACK_LOOT_RACE == 1)) || die \
    "loot-item kills an exact overworld creature fixture and mutates two disposable characters; rerun with --ack-disposable-overworld-loot-race"

  LIVE_PATH="$QA_LIVE_DIR/$QA_LIVE_NAME"
  local candidate="${WORLD_EXEC:-$REPO_ROOT/target/release/world-server}"
  [[ -x "$candidate" ]] || die "candidate build is not executable: $candidate"
  [[ -f "$LIVE_PATH" ]] || die "no live build at $LIVE_PATH"
  [[ -x "$QA_BOT" ]] || die "QA bot is not built: $QA_BOT"
  [[ -x "$QA_SMOKE" ]] || die "smoke wrapper is missing: $QA_SMOKE"
  require_clean_worktree
  have_bot_credentials || die \
    "no bot credentials: set WOW_BOT_PASSWORD or provide $QA_ENV_FILE before swapping a build"
  arm_loot_fixture_guard

  local candidate_sha
  candidate_sha="$(sha256_of "$candidate")"
  RUN_CANDIDATE_SHA="$candidate_sha"

  if ((DRY_RUN == 1)); then
    log "Dry run: nothing is stopped, copied, mutated or started"
    printf 'would snapshot  %s\n' "$LIVE_PATH"
    printf 'would install   %s (%s)\n' "$candidate" "$candidate_sha"
    printf 'would guard     creature %s HealthModifier %s -> %s\n' \
      "$QA_LOOT_FIXTURE_ENTRY" "$QA_LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER" \
      "$QA_LOOT_FIXTURE_TEMP_HEALTH_MODIFIER"
    printf 'would run       %s with WOW_BOT_LOOT_ITEM_CAPTURE=1\n' "$QA_SMOKE"
    printf 'would restore   the fixture and the snapshot on every exit path\n'
    return 0
  fi

  local identity
  identity="$(live_identity)"
  packet_dump_absent || die "a packet dump directory is configured; refusing to run QA"
  ORIGINAL_SHA="$(cut -f3 <<<"$identity")"
  local pid
  pid="$(cut -f1 <<<"$identity")"
  WORLD_PID="$pid"
  ports_ready "$pid" || die "ports $QA_WORLD_PORT/$QA_INSTANCE_PORT are not owned by PID $pid"
  log "Live build $ORIGINAL_SHA serving on PID $pid"

  RESTORE_FROM="$(mktemp "${TMPDIR:-/tmp}/rustycore-live-build.XXXXXX")"
  cp -- "$LIVE_PATH" "$RESTORE_FROM"
  [[ "$(sha256_of "$RESTORE_FROM")" == "$ORIGINAL_SHA" ]] || die "snapshot copy does not match"
  RESTORE_PENDING=1
  trap restore_live_build EXIT
  trap 'exit 130' HUP INT TERM
  write_report loot-item running "$candidate_sha"

  # The fixture must be in place before the world starts: the map generates the
  # creature's health from the template at load time.
  stop_world_verified || die "$QA_SERVICE did not stop (still active, running or listening); refusing to arm the loot fixture"
  log "Arming the loot fixture"
  apply_creature_health_fixture_guard || die "could not arm the loot fixture"

  log "Installing the candidate build $candidate_sha"
  cp -- "$candidate" "$LIVE_PATH"
  # shellcheck disable=SC2086
  $QA_SYSTEMCTL start "$QA_SERVICE"
  local candidate_pid
  candidate_pid="$(wait_until_serving)" || die "$QA_SERVICE did not come up with the candidate"
  WORLD_PID="$candidate_pid"
  log "Candidate serving on PID $candidate_pid"

  local bot_status=0
  load_bot_environment
  [[ -d "$QA_BOT_DIR" ]] || die "bot directory is missing: $QA_BOT_DIR"
  prepare_journal_dir
  local journal="$QA_JOURNAL_DIR/fixture-$$-$(date -u +%Y%m%dT%H%M%SZ).journal"
  [[ ! -e "$journal" && ! -e "${journal}.cleanup-complete" ]] \
    || die "fixture journal path is not fresh: $journal"
  log "Fixture recovery journal: $journal"
  ( cd "$QA_BOT_DIR" && exec timeout --foreground --signal=TERM --kill-after=30 \
      "${QA_BOT_TIMEOUT_SECONDS}s" env \
      WOW_BOT_LOOT_ITEM_CAPTURE=1 \
      WOW_BOT_ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=1 \
      WOW_BOT_FIXTURE_JOURNAL="$journal" \
      WOW_BOT_ENSURE_TEST_ACCOUNTS=0 \
      "$QA_SMOKE" ) || bot_status=$?
  if [[ -e "$journal" ]]; then
    warn "fixture journal $journal survived the run; the world fixture is still mutated"
    warn "recover it with: cd $QA_BOT_DIR && WOW_BOT_FIXTURE_JOURNAL=$journal ./target/debug/wow-test-bot --recover-loot-fixture"
    ((bot_status == 0)) && bot_status=75
  fi
  if ((bot_status == 0)); then
    log "Loot-item capture passed"
  else
    warn "loot-item capture failed with status $bot_status"
  fi
  # The final outcome is reported by the restore trap, after the restore.
  RUN_BOT_STATUS="$bot_status"
  return "$bot_status"
}

# Two disposable characters trade one isolated item and gold, under the guarded
# swap (#38). Unlike the loot scenarios there is no world fixture: the bot puts
# its characters-DB fixture (item row, money, co-location) in place while both
# characters are offline, records it in the recovery journal first, and
# restores it on every exit path. A journal that survives the run is reported
# with its recovery command, exactly like loot-item.
run_trade() {
  ((ALLOW_RUNTIME_QA == 1)) || die \
    "trade stops and starts $QA_SERVICE; rerun with --allow-runtime-qa"
  ((ACK_TRADE == 1)) || die \
    "trade mutates two disposable characters (money, position, one item); rerun with --ack-disposable-trade"

  LIVE_PATH="$QA_LIVE_DIR/$QA_LIVE_NAME"
  local candidate="${WORLD_EXEC:-$REPO_ROOT/target/release/world-server}"
  [[ -x "$candidate" ]] || die "candidate build is not executable: $candidate"
  [[ -f "$LIVE_PATH" ]] || die "no live build at $LIVE_PATH"
  [[ -x "$QA_BOT" ]] || die "QA bot is not built: $QA_BOT"
  [[ -x "$QA_SMOKE" ]] || die "smoke wrapper is missing: $QA_SMOKE"
  require_clean_worktree
  have_bot_credentials || die \
    "no bot credentials: set WOW_BOT_PASSWORD or provide $QA_ENV_FILE before swapping a build"

  local candidate_sha
  candidate_sha="$(sha256_of "$candidate")"
  RUN_CANDIDATE_SHA="$candidate_sha"

  if ((DRY_RUN == 1)); then
    log "Dry run: nothing is stopped, copied, mutated or started"
    printf 'would snapshot  %s\n' "$LIVE_PATH"
    printf 'would install   %s (%s)\n' "$candidate" "$candidate_sha"
    printf 'would run       %s with WOW_BOT_TRADE_SMOKE=1\n' "$QA_SMOKE"
    printf 'would journal   a fresh path under %s\n' "$QA_JOURNAL_DIR"
    printf 'would restore   the snapshot on every exit path\n'
    return 0
  fi

  local identity
  identity="$(live_identity)"
  packet_dump_absent || die "a packet dump directory is configured; refusing to run QA"
  ORIGINAL_SHA="$(cut -f3 <<<"$identity")"
  local pid
  pid="$(cut -f1 <<<"$identity")"
  WORLD_PID="$pid"
  ports_ready "$pid" || die "ports $QA_WORLD_PORT/$QA_INSTANCE_PORT are not owned by PID $pid"
  log "Live build $ORIGINAL_SHA serving on PID $pid"

  RESTORE_FROM="$(mktemp "${TMPDIR:-/tmp}/rustycore-live-build.XXXXXX")"
  cp -- "$LIVE_PATH" "$RESTORE_FROM"
  [[ "$(sha256_of "$RESTORE_FROM")" == "$ORIGINAL_SHA" ]] || die "snapshot copy does not match"
  RESTORE_PENDING=1
  trap restore_live_build EXIT
  trap 'exit 130' HUP INT TERM
  write_report trade running "$candidate_sha"

  log "Installing the candidate build $candidate_sha"
  stop_world_verified || die "$QA_SERVICE did not stop (still active, running or listening); refusing to swap the build"
  cp -- "$candidate" "$LIVE_PATH"
  [[ "$(sha256_of "$LIVE_PATH")" == "$candidate_sha" ]] || die "candidate did not install"
  # shellcheck disable=SC2086
  $QA_SYSTEMCTL start "$QA_SERVICE"
  local candidate_pid
  candidate_pid="$(wait_until_serving)" || die "$QA_SERVICE did not come up with the candidate"
  WORLD_PID="$candidate_pid"
  log "Candidate serving on PID $candidate_pid"

  local bot_status=0 evidence_dir
  load_bot_environment
  [[ -d "$QA_BOT_DIR" ]] || die "bot directory is missing: $QA_BOT_DIR"
  prepare_journal_dir
  local journal="$QA_JOURNAL_DIR/trade-$$-$(date -u +%Y%m%dT%H%M%SZ).journal"
  [[ ! -e "$journal" && ! -e "${journal}.cleanup-complete" ]] \
    || die "fixture journal path is not fresh: $journal"
  evidence_dir="$(mktemp -d "${TMPDIR:-/tmp}/rustycore-trade-qa.XXXXXX")"
  log "Fixture recovery journal: $journal"
  log "Trade evidence directory: $evidence_dir"
  ( cd "$QA_BOT_DIR" && exec timeout --foreground --signal=TERM --kill-after=30 \
      "${QA_BOT_TIMEOUT_SECONDS}s" env \
      WOW_BOT_TRADE_SMOKE=1 \
      WOW_BOT_ACK_DISPOSABLE_TRADE=1 \
      WOW_BOT_FIXTURE_JOURNAL="$journal" \
      WOW_BOT_ENSURE_TEST_ACCOUNTS=0 \
      WOW_BOT_REPORT="$evidence_dir/bot.json" WOW_BOT_LOG="$evidence_dir/bot.log" \
      "$QA_SMOKE" ) || bot_status=$?
  if [[ -e "$journal" ]]; then
    warn "fixture journal $journal survived the run; the trade characters are still mutated"
    warn "recover it with: cd $QA_BOT_DIR && WOW_BOT_FIXTURE_JOURNAL=$journal ./target/debug/wow-test-bot --recover-trade-fixture"
    ((bot_status == 0)) && bot_status=75
  fi
  if ((bot_status == 0)) && ! jq -e \
    '.trade_smoke == true and (.results | length == 2) and
     all(.results[]; .trade_smoke_passed == true and .trade_completed == true and
       .trade_db_verified_online == true and .trade_db_verified_after_logout == true and
       .trade_relog_verified == true and .trade_db_verified_after_relog == true)' \
    "$evidence_dir/bot.json" >/dev/null 2>&1; then
    warn "trade smoke returned success without complete trade evidence"
    bot_status=65
  fi
  if ((bot_status == 0)); then
    log "Trade smoke passed; evidence: $evidence_dir/bot.json"
  else
    warn "trade smoke failed with status $bot_status; evidence: $evidence_dir"
  fi
  # The final outcome is reported by the restore trap, after the restore.
  RUN_BOT_STATUS="$bot_status"
  return "$bot_status"
}

# Sourced callers use these helpers without entering the CLI or starting services.
[[ ${BASH_SOURCE[0]} != $0 ]] && return

while (($#)); do
  case "$1" in
    --dry-run) DRY_RUN=1; shift ;;
    --allow-runtime-qa) ALLOW_RUNTIME_QA=1; shift ;;
    --ack-disposable-overworld-loot-race) ACK_LOOT_RACE=1; shift ;;
    --ack-disposable-trade) ACK_TRADE=1; shift ;;
    --world-exec) [[ $# -ge 2 ]] || die "--world-exec needs a path"; WORLD_EXEC="$2"; shift 2 ;;
    --report) [[ $# -ge 2 ]] || die "--report needs a path"; REPORT="$2"; shift 2 ;;
    --backend) [[ $# -ge 2 ]] || die "--backend needs rust or cpp"; CAPTURE_BACKEND="$2"; shift 2 ;;
    --output-dir) [[ $# -ge 2 ]] || die "--output-dir needs a path"; CAPTURE_OUTPUT_DIR="$2"; shift 2 ;;
    --candidate-sha256) [[ $# -ge 2 ]] || die "--candidate-sha256 needs a hash"; CAPTURE_CANDIDATE_SHA="$2"; shift 2 ;;
    --scenario-runner) [[ $# -ge 2 ]] || die "--scenario-runner needs a path"; CAPTURE_RUNNER="$2"; shift 2 ;;
    --source-revision) [[ $# -ge 2 ]] || die "--source-revision needs a revision"; CAPTURE_SOURCE_REVISION="$2"; shift 2 ;;
    --derivation) [[ $# -ge 2 ]] || die "--derivation needs a description"; CAPTURE_DERIVATION="$2"; shift 2 ;;
    --source-path) [[ $# -ge 2 ]] || die "--source-path needs a path"; CAPTURE_SOURCE_PATH="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    --) shift; break ;;
    *) break ;;
  esac
done

COMMAND="${1:-help}"
(($# > 0)) && shift || true

case "$COMMAND" in
  self-test)
    exec "$REPO_ROOT/tools/qa_runtime_self_test.sh"
    ;;
  snapshot)
    require_command ss
    run_snapshot
    ;;
  login)
    require_command ss
    require_command timeout
    require_command jq
    exec 9>"$QA_LOCK"
    flock -n 9 || die "another runtime QA run holds $QA_LOCK"
    run_login
    ;;
  loot-race)
    require_command ss
    require_command timeout
    exec 9>"$QA_LOCK"
    flock -n 9 || die "another runtime QA run holds $QA_LOCK"
    run_loot_race
    ;;
  loot-item)
    require_command ss
    require_command timeout
    exec 9>"$QA_LOCK"
    flock -n 9 || die "another runtime QA run holds $QA_LOCK"
    run_loot_item
    ;;
  trade)
    require_command ss
    require_command timeout
    require_command jq
    exec 9>"$QA_LOCK"
    flock -n 9 || die "another runtime QA run holds $QA_LOCK"
    run_trade
    ;;
  capture)
    require_command ss
    require_command timeout
    require_command jq
    require_command python3
    require_command realpath
    require_command stat
    exec 9>"$QA_LOCK"
    flock -n 9 || die "another runtime QA run holds $QA_LOCK"
    run_capture
    ;;
  help) usage ;;
  *) usage >&2; die "unknown command: $COMMAND" ;;
esac
