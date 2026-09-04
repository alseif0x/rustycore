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
# Which worktree must be clean for the swapped build to be identifiable. The
# self-test points this at its own clean fixture repository.
QA_GIT_DIR="${QA_GIT_DIR:-$REPO_ROOT}"
# Per-account bot passwords. The login-smoke wrapper loads this file; the bot
# binary does not, so a run that invokes the binary directly must.
QA_ENV_FILE="${QA_ENV_FILE:-$REPO_ROOT/tools/wow-test-bot/.env.local}"
# The deterministic loot fixtures. `loot-item` kills one creature, so the guard
# lowers that creature's HealthModifier before the world starts and restores it
# afterwards. The guard itself is the shared, PM2-free one the capture wrappers
# use; #373 makes this the second caller rather than a second copy.
QA_FIXTURE_GUARD="${QA_FIXTURE_GUARD:-$REPO_ROOT/crates/capture-diff/scripts/loot-fixture-common.sh}"
QA_LOOT_FIXTURE_DB_CONF="${QA_LOOT_FIXTURE_DB_CONF:-/home/server/trinity-legacy-install/etc/worldserver.conf}"
QA_LOOT_FIXTURE_ENTRY="${QA_LOOT_FIXTURE_ENTRY:-21779}"
QA_LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER="${QA_LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER:-1}"
QA_LOOT_FIXTURE_TEMP_HEALTH_MODIFIER="${QA_LOOT_FIXTURE_TEMP_HEALTH_MODIFIER:-0.0001}"
# The chest scenario's guard still lives inside the PM2-driven capture wrapper
# and was never extracted (#373). Checked before anything is stopped.
QA_LOOT_RACE_CHEST_SPAWN="${QA_LOOT_RACE_CHEST_SPAWN:-9106001}"

DRY_RUN=0
ALLOW_RUNTIME_QA=0
ACK_LOOT_RACE=0
WORLD_EXEC=""
REPORT=""

RESTORE_PENDING=0
RESTORE_FROM=""
ORIGINAL_SHA=""
LIVE_PATH=""

# Globals the shared fixture guard reads. It restores only what it armed, so a
# run that never reached the mutation leaves these alone.
LOOT_FIXTURE_DB_CONF=""
LOOT_FIXTURE_ENTRY=""
LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER=""
LOOT_FIXTURE_TEMP_HEALTH_MODIFIER=""
LOOT_FIXTURE_SNAPSHOT_READY=0
LOOT_FIXTURE_CLEANUP_MARKER=""

usage() {
  cat <<'USAGE'
RustyCore guarded runtime QA

Usage:
  ./tools/qa-runtime.sh [OPTIONS] <COMMAND>

Commands:
  self-test     Exercise every guard and the restore path against fake services.
  snapshot      Print the live world build identity and exit. Touches nothing.
  login         Swap in a build, verify login/world entry, restore. No fixture setup.
  loot-race     Swap in a build, run the destructive two-session chest smoke, restore.
  loot-item     Swap in a build, guard one creature's health, run the destructive
                creature-kill capture, restore both the fixture and the build.

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
    if [[ "${COMMAND:-}" == login && -n "$REPORT" ]]; then
      write_report login restore-failed "${LOGIN_CANDIDATE_SHA:-}" "${LOGIN_BOT_STATUS:-null}"
    fi
    printf 'error: THE LIVE BUILD OR FIXTURE WAS NOT RESTORED CLEANLY. Original kept at %s\n' \
      "$RESTORE_FROM" >&2
    exit 70
  fi
  if [[ "${COMMAND:-}" == login && -n "$REPORT" ]]; then
    write_report login "$([[ $status -eq 0 ]] && echo passed-restored || echo failed-restored)" \
      "${LOGIN_CANDIDATE_SHA:-}" "${LOGIN_BOT_STATUS:-null}"
  fi
  log "Original build restored and serving"
  exit "$status"
}

# Load the shared fixture guard. It is source-only and documents the globals a
# caller must define; this sets them from the QA knobs above.
assert_chest_fixture_present() {
  arm_loot_fixture_guard
  local chest_rows
  chest_rows="$(loot_fixture_world_mysql -e \
    "SELECT COUNT(*) FROM gameobject WHERE guid = ${QA_LOOT_RACE_CHEST_SPAWN}")" \
    || die "could not query the chest fixture spawn"
  [[ "$chest_rows" == "1" ]] || die \
    "chest fixture spawn ${QA_LOOT_RACE_CHEST_SPAWN} is absent, and this harness cannot install it: that guard still lives in crates/capture-diff/scripts/capture-rust.sh, which drives PM2 (#373). Use the loot-item command for a creature kill, or install the chest fixture first."
}

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
  LOGIN_CANDIDATE_SHA="$(sha256_of "$candidate")"
  LOGIN_BOT_STATUS=null
  if ((DRY_RUN == 1)); then
    log "Dry run: nothing is stopped, copied or started"
    printf 'would install   %s (%s)\n' "$candidate" "$LOGIN_CANDIDATE_SHA"
    printf 'would run       login-only with account provisioning and fixture modes disabled\n'
    printf 'would restore   the snapshot on every exit path\n'
    return 0
  fi

  local identity pid bot_sha evidence_dir
  identity="$(live_identity)"
  packet_dump_absent || die "a packet dump directory is configured; refusing to run QA"
  ORIGINAL_SHA="$(cut -f3 <<<"$identity")"
  pid="$(cut -f1 <<<"$identity")"
  ports_ready "$pid" || die "ports $QA_WORLD_PORT/$QA_INSTANCE_PORT are not owned by PID $pid"
  bot_sha="$(sha256_of "$QA_BOT")"
  evidence_dir="$(mktemp -d "${TMPDIR:-/tmp}/rustycore-login-qa.XXXXXX")"
  log "Login evidence directory: $evidence_dir"
  RESTORE_FROM="$evidence_dir/original-world-server"
  cp -- "$LIVE_PATH" "$RESTORE_FROM"
  [[ "$(sha256_of "$RESTORE_FROM")" == "$ORIGINAL_SHA" ]] || die "snapshot copy does not match"
  write_report login running "$LOGIN_CANDIDATE_SHA"
  RESTORE_PENDING=1
  trap restore_live_build EXIT
  trap 'exit 130' HUP INT TERM
  log "Installing the candidate build $LOGIN_CANDIDATE_SHA"
  # shellcheck disable=SC2086
  $QA_SYSTEMCTL stop "$QA_SERVICE"
  cp -- "$candidate" "$LIVE_PATH"
  [[ "$(sha256_of "$LIVE_PATH")" == "$LOGIN_CANDIDATE_SHA" ]] || die "candidate did not install"
  # shellcheck disable=SC2086
  $QA_SYSTEMCTL start "$QA_SERVICE"
  pid="$(wait_until_serving)" || die "the candidate build did not start serving"
  log "Candidate serving on PID $pid"

  LOGIN_BOT_STATUS=0
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
  ) >"$evidence_dir/wrapper.log" 2>&1 || LOGIN_BOT_STATUS=$?
  if ((LOGIN_BOT_STATUS == 0)) && ! jq -e \
    '.login_only == true and (.results | length == 1) and
     all(.results[]; .world_auth == true and .enum_characters == true and .player_login_verified == true)' \
    "$evidence_dir/bot.json" >/dev/null 2>&1; then
    warn "login wrapper returned success without verified world-entry evidence"
    LOGIN_BOT_STATUS=65
  fi
  log "Login bot status: $LOGIN_BOT_STATUS; evidence: $evidence_dir/bot.json"
  return "$LOGIN_BOT_STATUS"
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

  local candidate_sha
  candidate_sha="$(sha256_of "$candidate")"

  if ((DRY_RUN == 1)); then
    log "Dry run: nothing is stopped, copied or started"
    printf 'would snapshot  %s\n' "$LIVE_PATH"
    printf 'would install   %s (%s)\n' "$candidate" "$candidate_sha"
    printf 'would run       %s with WOW_BOT_LOOT_RACE_SMOKE=1\n' "$QA_SMOKE"
    printf 'would journal   a fresh path under %s\n' "$QA_JOURNAL_DIR"
    printf 'would restore   the snapshot on every exit path\n'
    return 0
  fi

  # This scenario needs a wrapper-installed chest spawn, and the guard that
  # installs it was never extracted from the PM2-driven capture wrapper (#373).
  # Checked before the service is stopped, not after the swap.
  assert_chest_fixture_present

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
  [[ -d "$QA_BOT_DIR" ]] || die "bot directory is missing: $QA_BOT_DIR"
  mkdir -p "$QA_JOURNAL_DIR"
  [[ -d "$QA_JOURNAL_DIR" && ! -L "$QA_JOURNAL_DIR" ]] \
    || die "fixture-journal directory must be a real directory: $QA_JOURNAL_DIR"
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
  write_report loot-race "$([[ $bot_status -eq 0 ]] && echo passed || echo failed)" \
    "$candidate_sha" "$bot_status"
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
  ports_ready "$pid" || die "ports $QA_WORLD_PORT/$QA_INSTANCE_PORT are not owned by PID $pid"
  log "Live build $ORIGINAL_SHA serving on PID $pid"

  RESTORE_FROM="$(mktemp "${TMPDIR:-/tmp}/rustycore-live-build.XXXXXX")"
  cp -- "$LIVE_PATH" "$RESTORE_FROM"
  [[ "$(sha256_of "$RESTORE_FROM")" == "$ORIGINAL_SHA" ]] || die "snapshot copy does not match"
  RESTORE_PENDING=1
  trap restore_live_build EXIT
  trap 'exit 130' HUP INT TERM

  # The fixture must be in place before the world starts: the map generates the
  # creature's health from the template at load time.
  # shellcheck disable=SC2086
  $QA_SYSTEMCTL stop "$QA_SERVICE"
  log "Arming the loot fixture"
  apply_creature_health_fixture_guard || die "could not arm the loot fixture"

  log "Installing the candidate build $candidate_sha"
  cp -- "$candidate" "$LIVE_PATH"
  # shellcheck disable=SC2086
  $QA_SYSTEMCTL start "$QA_SERVICE"
  local candidate_pid
  candidate_pid="$(wait_until_serving)" || die "$QA_SERVICE did not come up with the candidate"
  log "Candidate serving on PID $candidate_pid"

  local bot_status=0
  load_bot_environment
  [[ -d "$QA_BOT_DIR" ]] || die "bot directory is missing: $QA_BOT_DIR"
  mkdir -p "$QA_JOURNAL_DIR"
  chmod 700 "$QA_JOURNAL_DIR"
  [[ -d "$QA_JOURNAL_DIR" && ! -L "$QA_JOURNAL_DIR" ]] \
    || die "fixture-journal directory must be a real directory: $QA_JOURNAL_DIR"
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
  write_report loot-item "$([[ $bot_status -eq 0 ]] && echo passed || echo failed)" \
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
  help) usage ;;
  *) usage >&2; die "unknown command: $COMMAND" ;;
esac
