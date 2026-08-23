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
QA_SERVICE="${QA_SERVICE:-world-server}"
QA_LIVE_DIR="${QA_LIVE_DIR:-$REPO_ROOT/target/deploy/live}"
QA_LIVE_NAME="${QA_LIVE_NAME:-world-server}"
QA_BOT="${QA_BOT:-$REPO_ROOT/tools/wow-test-bot/target/debug/wow-test-bot}"
QA_LOCK="${QA_LOCK:-/tmp/rustycore-qa-runtime.lock}"
QA_WORLD_PORT="${QA_WORLD_PORT:-8085}"
QA_INSTANCE_PORT="${QA_INSTANCE_PORT:-8086}"
QA_READY_TIMEOUT_SECONDS="${QA_READY_TIMEOUT_SECONDS:-60}"
QA_BOT_TIMEOUT_SECONDS="${QA_BOT_TIMEOUT_SECONDS:-600}"
QA_SKIP_PORT_GUARD="${QA_SKIP_PORT_GUARD:-0}"
# How the running executable of a PID is resolved. Overridden only by the
# self-test, which has no /proc entry pointing at its fake build.
QA_EXE_RESOLVER="${QA_EXE_RESOLVER:-realpath -e --}"
# Which worktree must be clean for the swapped build to be identifiable. The
# self-test points this at its own clean fixture repository.
QA_GIT_DIR="${QA_GIT_DIR:-$REPO_ROOT}"
# Per-account bot passwords. The login-smoke wrapper loads this file; the bot
# binary does not, so a run that invokes the binary directly must.
QA_ENV_FILE="${QA_ENV_FILE:-$REPO_ROOT/tools/wow-test-bot/.env.local}"

DRY_RUN=0
ALLOW_RUNTIME_QA=0
ACK_LOOT_RACE=0
WORLD_EXEC=""
REPORT=""

RESTORE_PENDING=0
RESTORE_FROM=""
ORIGINAL_SHA=""
LIVE_PATH=""

usage() {
  cat <<'USAGE'
RustyCore guarded runtime QA

Usage:
  ./tools/qa-runtime.sh [OPTIONS] <COMMAND>

Commands:
  self-test     Exercise every guard and the restore path against fake services.
  snapshot      Print the live world build identity and exit. Touches nothing.
  loot-race     Swap in a build, run the destructive two-session loot smoke, restore.

Options:
  --dry-run                              Print the plan; start, stop and copy nothing.
  --allow-runtime-qa                     Required: this stops and starts a live service.
  --ack-disposable-overworld-loot-race   Required for loot-race: it mutates a world
                                         GameObject fixture and two disposable characters.
  --world-exec PATH                      Build to swap in (default target/release/world-server).
  --report PATH                          Write a JSON result document.
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

service_active() {
  [[ "$(service_property ActiveState)" == "active" ]]
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
  # shellcheck disable=SC2086
  $QA_SYSTEMCTL stop "$QA_SERVICE" || restore_status=1
  cp -- "$RESTORE_FROM" "$LIVE_PATH" || restore_status=1
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
  if ((restore_status != 0)); then
    printf 'error: THE LIVE BUILD WAS NOT RESTORED CLEANLY. Original kept at %s\n' \
      "$RESTORE_FROM" >&2
    exit 70
  fi
  log "Original build restored and serving"
  exit "$status"
}

write_report() {
  [[ -n "$REPORT" ]] || return 0
  printf '{"command":"%s","outcome":"%s","service":"%s","original_sha256":"%s","candidate_sha256":"%s","bot_status":%s}\n' \
    "$1" "$2" "$QA_SERVICE" "$ORIGINAL_SHA" "${3:-}" "${4:-null}" >"$REPORT"
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
  require_clean_worktree
  have_bot_credentials || die \
    "no bot credentials: set WOW_BOT_PASSWORD or provide $QA_ENV_FILE before swapping a build"

  local candidate_sha
  candidate_sha="$(sha256_of "$candidate")"

  if ((DRY_RUN == 1)); then
    log "Dry run: nothing is stopped, copied or started"
    printf 'would snapshot  %s\n' "$LIVE_PATH"
    printf 'would install   %s (%s)\n' "$candidate" "$candidate_sha"
    printf 'would run       %s --loot-race-smoke --ack-disposable-overworld-loot-race\n' "$QA_BOT"
    printf 'would restore   the snapshot on every exit path\n'
    return 0
  fi

  local identity
  identity="$(live_identity)"
  packet_dump_absent || die "a packet dump directory is configured; refusing to run QA"
  ORIGINAL_SHA="$(cut -f3 <<<"$identity")"
  local pid
  pid="$(cut -f1 <<<"$identity")"
  ports_ready "$pid" || die "ports $QA_WORLD_PORT/$QA_INSTANCE_PORT are not owned by PID $pid"
  log "Live build $ORIGINAL_SHA serving on PID $pid"

  RESTORE_FROM="$(mktemp "${TMPDIR:-/tmp}/rustycore-live-build.XXXXXX")"
  cp -- "$LIVE_PATH" "$RESTORE_FROM"
  [[ "$(sha256_of "$RESTORE_FROM")" == "$ORIGINAL_SHA" ]] || die "snapshot copy does not match"
  RESTORE_PENDING=1
  trap restore_live_build EXIT
  trap 'exit 130' HUP INT TERM

  log "Installing the candidate build $candidate_sha"
  # shellcheck disable=SC2086
  $QA_SYSTEMCTL stop "$QA_SERVICE"
  cp -- "$candidate" "$LIVE_PATH"
  [[ "$(sha256_of "$LIVE_PATH")" == "$candidate_sha" ]] || die "candidate did not install"
  # shellcheck disable=SC2086
  $QA_SYSTEMCTL start "$QA_SERVICE"
  pid="$(wait_until_serving)" || die "the candidate build did not start serving"
  log "Candidate serving on PID $pid"

  local bot_status=0
  load_bot_environment
  timeout --foreground --signal=TERM --kill-after=30 "${QA_BOT_TIMEOUT_SECONDS}s" \
    "$QA_BOT" --loot-race-smoke --ack-disposable-overworld-loot-race || bot_status=$?
  if ((bot_status == 0)); then
    log "Loot-race smoke passed"
  else
    warn "loot-race smoke failed with status $bot_status"
  fi
  write_report loot-race "$([[ $bot_status -eq 0 ]] && echo passed || echo failed)" \
    "$candidate_sha" "$bot_status"
  return "$bot_status"
}

while (($#)); do
  case "$1" in
    --dry-run) DRY_RUN=1; shift ;;
    --allow-runtime-qa) ALLOW_RUNTIME_QA=1; shift ;;
    --ack-disposable-overworld-loot-race) ACK_LOOT_RACE=1; shift ;;
    --world-exec) [[ $# -ge 2 ]] || die "--world-exec needs a path"; WORLD_EXEC="$2"; shift 2 ;;
    --report) [[ $# -ge 2 ]] || die "--report needs a path"; REPORT="$2"; shift 2 ;;
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
  loot-race)
    require_command ss
    require_command timeout
    exec 9>"$QA_LOCK"
    flock -n 9 || die "another runtime QA run holds $QA_LOCK"
    run_loot_race
    ;;
  help) usage ;;
  *) usage >&2; die "unknown command: $COMMAND" ;;
esac
