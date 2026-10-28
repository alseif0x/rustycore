#!/usr/bin/env bash
# Contract tests for tools/qa-runtime.sh.
#
# The point of the orchestration is that the live build comes back. That cannot
# be asserted by reading the script, so every case here drives the real script
# against a fake service and then compares the bytes of the "live" build.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
QA="$REPO_ROOT/tools/qa-runtime.sh"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/qa-runtime-self-test.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

PASSED=0
check() {
  local description="$1"
  shift
  if "$@"; then
    PASSED=$((PASSED + 1))
  else
    printf 'FAIL: %s\n' "$description" >&2
    exit 1
  fi
}

mkdir -p "$WORK/live" "$WORK/bin" "$WORK/repo" "$WORK/botdir" "$WORK/systemd" "$WORK/cpp-source"
# A clean fixture repository: the guard under test is "the worktree is clean",
# not "this development checkout happens to be".
git -C "$WORK/repo" init -q
git -C "$WORK/repo" config user.email qa@example.invalid
git -C "$WORK/repo" config user.name "QA Runtime"
printf 'fixture\n' >"$WORK/repo/README"
git -C "$WORK/repo" add README
git -C "$WORK/repo" commit -qm fixture
printf 'ORIGINAL-BUILD\n' >"$WORK/live/world-server"
printf 'CANDIDATE-BUILD\n' >"$WORK/candidate"
chmod +x "$WORK/candidate"

# A fake systemd: it records what it was asked to do and can be told to fail.
cat >"$WORK/bin/systemctl" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
state="${QA_FAKE_STATE:?}"
printf '%s\n' "$*" >>"${state}.calls"
# The world is up until a stop succeeds. A stop can be told to fail, or to
# "succeed" while the unit stays active, on its Nth call (#1427).
world_up() {
  [[ -f "${state}.still-active" ]] && return 0
  [[ ! -f "${state}.log" || "$(tail -1 "${state}.log")" != stopped ]]
}
case "${1:-}" in
  show)
    case "$*" in
      *ActiveState*) if world_up; then printf 'active\n'; else printf 'inactive\n'; fi ;;
      *MainPID*) if world_up; then printf '4242\n'; else printf '0\n'; fi ;;
      *NRestarts*) printf '7\n' ;;
      *Environment*)
        environment="${QA_FAKE_ENVIRONMENT:-}"
        if [[ -f "${state}.capture-env" ]]; then
          environment="$environment $(cat "${state}.capture-env")"
        fi
        printf '%s\n' "$environment"
        ;;
      *User*) id -un ;;
      *) printf '\n' ;;
    esac
    ;;
  stop)
    stops=$(( $(cat "${state}.stops" 2>/dev/null || echo 0) + 1 ))
    printf '%s\n' "$stops" >"${state}.stops"
    if [[ "${QA_FAKE_FAIL_STOP_AT:-0}" == "$stops" ]]; then
      printf 'refusing to stop\n' >&2
      exit 1
    fi
    [[ "${QA_FAKE_STILL_ACTIVE_AT:-0}" == "$stops" ]] && : >"${state}.still-active"
    printf 'stopped\n' >>"${state}.log"
    ;;
  start)
    if [[ "${QA_FAKE_FAIL_START:-0}" == "1" && -f "${state}.swapped" ]]; then
      printf 'refusing to start\n' >&2
      exit 1
    fi
    printf 'started\n' >>"${state}.log"
    ;;
  daemon-reload)
    if [[ -f "${QA_FAKE_DROPIN:?}" && ! -L "${QA_FAKE_DROPIN}" ]]; then
      sed -n 's/^Environment=//p' "${QA_FAKE_DROPIN}" >"${state}.capture-env"
    else
      rm -f "${state}.capture-env"
    fi
    ;;
esac
FAKE
chmod +x "$WORK/bin/systemctl"

# A fake bot that records which build was live when it ran.
cat >"$WORK/bin/bot" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
pwd >"${QA_FAKE_STATE:?}.bot-cwd"
printf '%s\n' "${WOW_BOT_FIXTURE_JOURNAL:-none}" >"${QA_FAKE_STATE:?}.bot-journal"
printf '%s\n' "${WOW_BOT_LOOT_RACE_SMOKE:-0}" >"${QA_FAKE_STATE:?}.bot-mode"
printf '%s %s\n' "${WOW_BOT_TRADE_SMOKE:-0}" "${WOW_BOT_ACK_DISPOSABLE_TRADE:-0}" >"${QA_FAKE_STATE:?}.bot-trade-mode"
# Some runs leave the fixture mutated; the fixture decides which.
if [[ "${QA_FAKE_LEAVE_JOURNAL:-0}" == "1" ]]; then : >"${WOW_BOT_FIXTURE_JOURNAL:?}"; fi
cat "${QA_FAKE_LIVE:?}" >"${QA_FAKE_STATE:?}.bot-saw"
if [[ "${WOW_BOT_TRADE_SMOKE:-0}" == 1 && -n "${WOW_BOT_REPORT:-}" && "${QA_FAKE_NO_LOGIN_REPORT:-0}" != 1 ]]; then
  trade_result='{"trade_smoke_passed":true,"trade_completed":true,"trade_db_verified_online":true,"trade_db_verified_after_logout":true,"trade_relog_verified":true,"trade_db_verified_after_relog":true}'
  printf '{"trade_smoke":true,"results":[%s,%s]}\n' "$trade_result" "$trade_result" >"$WOW_BOT_REPORT"
elif [[ -n "${WOW_BOT_REPORT:-}" && "${QA_FAKE_NO_LOGIN_REPORT:-0}" != 1 ]]; then
  printf '{"login_only":true,"results":[{"world_auth":true,"enum_characters":true,"player_login_verified":true,"login_stream_drained":true}]}\n' >"$WOW_BOT_REPORT"
  printf '%s\n' "${WOW_BOT_ENSURE_TEST_ACCOUNTS:-unset}" >"${QA_FAKE_STATE:?}.bot-provisioning"
  printf '%s\n' "${WOW_BOT_EXEC_SHA256:-unset}" >"${QA_FAKE_STATE:?}.bot-sha"
  printf '%s\n' "${WOW_BOT_STAND_STATE+present}" >"${QA_FAKE_STATE:?}.bot-stand-state"
fi
# Record only whether a credential arrived, never its value.
if [[ -n "${WOW_BOT_PASSWORD_TESTBOT2_BOT_LOCAL:-}" ]]; then
  printf 'credentials-present\n' >"${QA_FAKE_STATE:?}.bot-env"
fi
exit "${QA_FAKE_BOT_STATUS:-0}"
FAKE
chmod +x "$WORK/bin/bot"

# The fake world PID is a real process only when a case says so.
cat >"$WORK/bin/alive" <<'FAKE'
#!/usr/bin/env bash
[[ "${QA_FAKE_PROCESS_ALIVE:-0}" == 1 ]]
FAKE
chmod +x "$WORK/bin/alive"

cat >"$WORK/bin/resolver" <<'FAKE'
#!/usr/bin/env bash
printf '%s\n' "${QA_FAKE_LIVE:?}"
FAKE
chmod +x "$WORK/bin/resolver"

cat >"$WORK/bin/privileged" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
exec "$@"
FAKE
chmod +x "$WORK/bin/privileged"

cat >"$WORK/bin/process-env" <<'FAKE'
#!/usr/bin/env bash
cat "${QA_FAKE_STATE:?}.capture-env" 2>/dev/null || true
FAKE
chmod +x "$WORK/bin/process-env"

cat >"$WORK/bin/starttime" <<'FAKE'
#!/usr/bin/env bash
printf '4242\n'
FAKE
chmod +x "$WORK/bin/starttime"

cat >"$WORK/bin/capture-runner" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "${RUSTYCORE_CAPTURE_BACKEND:?} ${RUSTYCORE_CAPTURE_CANDIDATE_SHA256:?}" \
  >"${QA_FAKE_STATE:?}.capture-runner"
case "$RUSTYCORE_CAPTURE_BACKEND" in
  rust)
    printf 'AUTH-PAYLOAD\n' >"$RUSTYCORE_CAPTURE_DIR/client.bin"
    printf 'fixture metadata\n' >"$RUSTYCORE_CAPTURE_DIR/client.meta"
    if [[ "${QA_FAKE_CAPTURE_DRIFT:-}" == rust ]]; then
      printf '# concurrent drift\n' >>"${QA_FAKE_DROPIN:?}"
    fi
    ;;
  cpp)
    printf 'PKT-PAYLOAD\n' >"$RUSTYCORE_CAPTURE_DIR/cpp.pkt"
    if [[ "${QA_FAKE_CAPTURE_DRIFT:-}" == cpp ]]; then
      printf '# concurrent drift\n' >>"${QA_FAKE_CPP_CONFIG:?}"
    fi
    ;;
esac
if [[ "${QA_FAKE_CAPTURE_DRIFT:-}" == journal ]]; then
  rm -f "$RUSTYCORE_CAPTURE_DIR/.qa-capture-journal.json"
fi
printf '{"fake_capture_report":true}\n' >"${RUSTYCORE_CAPTURE_REPORT:?}"
exit "${QA_FAKE_CAPTURE_STATUS:-0}"
FAKE
chmod +x "$WORK/bin/capture-runner"

# A fake fixture guard. The real one is source-only and talks to MySQL; this
# records what the harness asked it to do so the restore path can be asserted
# rather than trusted (#373).
cat >"$WORK/fixture-guard.sh" <<'FAKE'
load_loot_fixture_database_credentials() { return 0; }
loot_fixture_world_mysql() {
  printf 'mysql %s\n' "$*" >>"${QA_FAKE_STATE:?}.fixture"
  printf '%s\n' "${QA_FAKE_CHEST_ROWS:-1}"
}
apply_creature_health_fixture_guard() {
  printf 'fixture-armed %s\n' "${LOOT_FIXTURE_ENTRY:?}" >>"${QA_FAKE_STATE:?}.fixture"
  LOOT_FIXTURE_SNAPSHOT_READY=1
}
restore_creature_health_fixture_guard() {
  printf 'fixture-restored %s\n' "${LOOT_FIXTURE_ENTRY:?}" >>"${QA_FAKE_STATE:?}.fixture"
  LOOT_FIXTURE_SNAPSHOT_READY=0
  [[ "${QA_FAKE_FIXTURE_RESTORE_FAILS:-0}" == 1 ]] && return 1
  return 0
}
loot_fixture_wait_until_all_characters_offline() {
  printf 'offline-checked\n' >>"${QA_FAKE_STATE:?}.fixture"
}
# The chest guard records the service state it ran under: both its install
# and its cleanup must happen while no world is running (#373).
fake_world_state() {
  if [[ -f "${QA_FAKE_STATE:?}.log" ]]; then tail -1 "${QA_FAKE_STATE}.log"; else printf 'running\n'; fi
}
apply_shared_chest_fixture_guard() {
  printf 'chest-armed %s %s world=%s\n' "${LOOT_FIXTURE_CHEST_GUID:?}" \
    "${LOOT_FIXTURE_CHEST_TEMPLATE_ENTRY:?}" "$(fake_world_state)" >>"${QA_FAKE_STATE:?}.fixture"
  [[ "${QA_FAKE_CHEST_APPLY_FAILS:-0}" == 1 ]] && return 1
  return 0
}
restore_shared_chest_fixture_guard() {
  printf 'chest-restored %s world=%s\n' "${LOOT_FIXTURE_CHEST_GUID:?}" \
    "$(fake_world_state)" >>"${QA_FAKE_STATE:?}.fixture"
  [[ "${QA_FAKE_CHEST_RESTORE_FAILS:-0}" == 1 ]] && return 1
  return 0
}
FAKE
touch "$WORK/fixture-conf"

assert_fake() {
  local name="$1" value="$2"
  [[ "$value" == "$WORK"/* ]] || {
    printf 'FAIL: self-test would drive the real %s (%s)\n' "$name" "$value" >&2
    exit 1
  }
}

run_qa() {
  assert_fake systemctl "$WORK/bin/systemctl"
  assert_fake smoke "$WORK/bin/bot"
  assert_fake journals "$WORK/journals"
  assert_fake "fixture guard" "$WORK/fixture-guard.sh"
  assert_fake "fixture conf" "$WORK/fixture-conf"
  assert_fake "capture runner" "$WORK/bin/capture-runner"
  assert_fake "privileged file operation adapter" "$WORK/bin/privileged"
  env \
    QA_SYSTEMCTL="$WORK/bin/systemctl" \
    QA_SERVICE="fake-world" \
    QA_LIVE_DIR="$WORK/live" \
    QA_LIVE_NAME="world-server" \
    QA_BOT="$WORK/bin/bot" \
    QA_LOCK="$WORK/lock" \
    QA_EXE_RESOLVER="$WORK/bin/resolver" \
    QA_PROCESS_ENV_READER="$WORK/bin/process-env" \
    QA_PROC_STARTTIME_READER="$WORK/bin/starttime" \
    QA_PROCESS_ALIVE="$WORK/bin/alive" \
    QA_PRIVILEGED="$WORK/bin/privileged" \
    QA_SKIP_PORT_GUARD=1 \
    QA_READY_TIMEOUT_SECONDS=5 \
    QA_FAKE_STATE="$WORK/state" \
    QA_FAKE_LIVE="$WORK/live/world-server" \
    QA_GIT_DIR="$WORK/repo" \
    QA_ENV_FILE="$WORK/env.local" \
    QA_BOT_DIR="$WORK/botdir" \
    QA_SMOKE="$WORK/bin/bot" \
    QA_JOURNAL_DIR="$WORK/journals" \
    QA_FIXTURE_GUARD="$WORK/fixture-guard.sh" \
    QA_LOOT_FIXTURE_DB_CONF="$WORK/fixture-conf" \
    QA_CAPTURE_FILEOPS="$REPO_ROOT/tools/qa/capture_fileops.py" \
    QA_CAPTURE_WORLD_CONFIG="$WORK/worldserver.conf" \
    QA_CAPTURE_CPP_CONFIG="$WORK/worldserver.conf" \
    QA_SYSTEMD_RUNTIME_DIR="$WORK/systemd" \
    QA_FAKE_DROPIN="$WORK/systemd/fake-world.service.d/43-qa-capture.conf" \
    QA_FAKE_CPP_CONFIG="$WORK/worldserver.conf" \
    "${EXTRA_ENV[@]}" \
    "$QA" "$@"
}

reset_state() {
  rm -f "$WORK"/state.* "$WORK/lock"
  rm -rf "$WORK/systemd/fake-world.service.d"
  printf 'WOW_BOT_PASSWORD_TESTBOT2_BOT_LOCAL=fixture-secret\n' >"$WORK/env.local"
  printf 'ORIGINAL-BUILD\n' >"$WORK/live/world-server"
  cat >"$WORK/worldserver.conf" <<'CONF'
DataDir = "./Data"
DBC.Locale = 0
RealmID = 1
WorldServerPort = 8085
InstanceServerPort = 8086
PlayerSaveInterval = 600000
DurabilityLoss.InPvP = 0
DurabilityLoss.OnDeath = 10
Corpse.Decay.Normal = 300
Rate.Damage.Fall = 1
Death.SicknessLevel = 11
Death.CorpseReclaimDelay.PvP = 1
Death.CorpseReclaimDelay.PvE = 0
Death.Bones.World = 1
Death.Bones.BattlegroundOrArena = 1
LogsDir = ""
PacketLogFile = ""
LoginDatabaseInfo = "host;3306;qa;fixture-secret;auth"
CONF
  chmod 600 "$WORK/worldserver.conf"
  EXTRA_ENV=()
}

live_is_original() { [[ "$(cat "$WORK/live/world-server")" == "ORIGINAL-BUILD" ]]; }
bot_saw_candidate() { [[ "$(cat "$WORK/state.bot-saw")" == "CANDIDATE-BUILD" ]]; }

CAPTURE_CANDIDATE_SHA="$(sha256sum "$WORK/candidate" | awk '{print $1}')"
run_capture_fake() {
  local backend="$1" output_dir="$2"
  shift 2
  local -a args=(
    --allow-runtime-qa
    --backend "$backend"
    --world-exec "$WORK/candidate"
    --output-dir "$output_dir"
    --candidate-sha256 "$CAPTURE_CANDIDATE_SHA"
    --scenario-runner "$WORK/bin/capture-runner"
  )
  if [[ "$backend" == cpp ]]; then
    args+=(
      --source-revision a5f8da2ebf5424bf0450ca4e08843ecbf72577bd
      --derivation 'derivedR6; not clean upstream'
      --source-path "$WORK/cpp-source"
    )
  fi
  run_qa "${args[@]}" "$@" capture
}

# 1. The destructive command refuses without each mandatory flag.
reset_state
output="$(run_qa --world-exec "$WORK/candidate" loot-race 2>&1 || true)"
check "refuses without --allow-runtime-qa" grep -q -- "--allow-runtime-qa" <<<"$output"
check "nothing was swapped" live_is_original

output="$(run_qa --allow-runtime-qa --world-exec "$WORK/candidate" loot-race 2>&1 || true)"
check "refuses without the disposable-fixture acknowledgement" \
  grep -q -- "--ack-disposable-overworld-loot-race" <<<"$output"
check "still nothing swapped" live_is_original

# 2. A missing candidate is refused before anything stops.
output="$(run_qa --allow-runtime-qa --ack-disposable-overworld-loot-race \
  --world-exec "$WORK/absent" loot-race 2>&1 || true)"
check "refuses a candidate that is not executable" grep -q "not executable" <<<"$output"
check "nothing stopped" bash -c '[[ ! -f "'"$WORK"'/state.log" ]]'

# 3. A dry run plans without touching anything.
output="$(run_qa --dry-run --allow-runtime-qa --ack-disposable-overworld-loot-race \
  --world-exec "$WORK/candidate" loot-race 2>&1)"
check "dry run explains the plan" grep -q "would restore" <<<"$output"
check "dry run swapped nothing" live_is_original
check "dry run started nothing" bash -c '[[ ! -f "'"$WORK"'/state.log" ]]'

# 4. The happy path: the bot sees the candidate, the original comes back.
reset_state
run_qa --allow-runtime-qa --ack-disposable-overworld-loot-race \
  --world-exec "$WORK/candidate" --report "$WORK/report.json" loot-race >/dev/null
check "the bot ran against the candidate build" bot_saw_candidate
check "the original build was restored" live_is_original
check "the report records a pass after the restore" grep -q '"outcome":"passed-restored"' "$WORK/report.json"
check "the smoke received its credentials" test -f "$WORK/state.bot-env"
check "the smoke was asked for loot-race mode" \
  bash -c '[[ "$(cat "'"$WORK"'/state.bot-mode")" == "1" ]]'
check "the smoke received a fresh absolute journal" \
  bash -c 'j="$(cat "'"$WORK"'/state.bot-journal")"; [[ "$j" == /* && "$j" == *journals/fixture-* ]]'
check "a completed run leaves no journal behind" \
  bash -c '! compgen -G "'"$WORK"'/journals/*.journal" >/dev/null'
check "the smoke ran from the bot's own directory" \
  bash -c '[[ "$(cat "'"$WORK"'/state.bot-cwd")" == "'"$WORK"'/botdir" ]]'
check "no credential value reached the report" bash -c '! grep -q fixture-secret "'"$WORK"'/report.json"'

# 4b. Without any credential source the run refuses before touching the service.
reset_state
rm -f "$WORK/env.local"
status=0
EXTRA_ENV=(WOW_BOT_PASSWORD= WOW_BOT_PASSWORD_TESTBOT2_BOT_LOCAL=)
output="$(run_qa --allow-runtime-qa \
  --ack-disposable-overworld-loot-race --world-exec "$WORK/candidate" loot-race 2>&1)" || status=$?
check "refuses without credentials" grep -q "no bot credentials" <<<"$output"
check "the credential refusal stopped nothing" bash -c '[[ ! -f "'"$WORK"'/state.log" ]]'
check "the credential refusal swapped nothing" live_is_original

# 5. A failing smoke still restores, and the failure is the run's status.
reset_state
EXTRA_ENV=(QA_FAKE_BOT_STATUS=3)
status=0
run_qa --allow-runtime-qa --ack-disposable-overworld-loot-race \
  --world-exec "$WORK/candidate" --report "$WORK/report.json" loot-race >/dev/null 2>&1 || status=$?
check "a failing smoke fails the run" test "$status" -eq 3
check "a failing smoke still restores" live_is_original
check "the report records the failure" grep -q '"outcome":"failed-restored"' "$WORK/report.json"

# 5b. A run that leaves its journal behind is a pending fixture recovery.
reset_state
EXTRA_ENV=(QA_FAKE_LEAVE_JOURNAL=1)
status=0
output="$(run_qa --allow-runtime-qa --ack-disposable-overworld-loot-race \
  --world-exec "$WORK/candidate" loot-race 2>&1)" || status=$?
check "a surviving journal fails the run" test "$status" -eq 75
check "a surviving journal is explained" grep -q "still mutated" <<<"$output"
check "a surviving journal names the recovery command" grep -q -- "--recover-loot-fixture" <<<"$output"
check "a surviving journal still restored the build" live_is_original
rm -f "$WORK"/journals/*.journal

# 6. A restore that cannot start the service outranks everything else.
reset_state
EXTRA_ENV=(QA_FAKE_FAIL_START=1)
touch "$WORK/state.swapped"
status=0
output="$(run_qa --allow-runtime-qa --ack-disposable-overworld-loot-race \
  --world-exec "$WORK/candidate" loot-race 2>&1)" || status=$?
check "a failed restore exits 70" test "$status" -eq 70
check "a failed restore says so loudly" grep -q "NOT RESTORED CLEANLY" <<<"$output"
check "a failed restore keeps the original copy" grep -q "Original kept at" <<<"$output"

# 7. A configured packet dump is refused.
reset_state
EXTRA_ENV=(QA_FAKE_ENVIRONMENT=RUSTYCORE_PACKET_DUMP_DIR=/tmp/dump)
output="$(run_qa --allow-runtime-qa --ack-disposable-overworld-loot-race \
  --world-exec "$WORK/candidate" loot-race 2>&1 || true)"
check "refuses while a packet dump is configured" grep -q "packet dump" <<<"$output"
check "packet-dump refusal swapped nothing" live_is_original

# 8. A dirty worktree is refused.
reset_state
printf 'uncommitted\n' >"$WORK/repo/scratch"
output="$(run_qa --allow-runtime-qa --ack-disposable-overworld-loot-race \
  --world-exec "$WORK/candidate" loot-race 2>&1 || true)"
rm -f "$WORK/repo/scratch"
check "refuses a dirty worktree" grep -q "clean worktree" <<<"$output"
check "the dirty-worktree refusal swapped nothing" live_is_original

# 9. Two runs cannot overlap.
reset_state
exec 8>"$WORK/lock"
flock -n 8
status=0
output="$(run_qa --allow-runtime-qa --ack-disposable-overworld-loot-race \
  --world-exec "$WORK/candidate" loot-race 2>&1)" || status=$?
exec 8>&-
check "a second run is refused while the lock is held" grep -q "another runtime QA run" <<<"$output"
check "the refused run swapped nothing" live_is_original


# 10. The chest scenario installs its fixture with the world stopped, before
# the candidate starts, and removes it with the world stopped before the
# original build starts again (#373).
reset_state
rm -f "$WORK/state.fixture"
run_qa --allow-runtime-qa --ack-disposable-overworld-loot-race \
  --world-exec "$WORK/candidate" loot-race >/dev/null 2>&1
check "the chest scenario checks that nobody is online first" \
  grep -q "offline-checked" "$WORK/state.fixture"
check "the chest is installed while the world is stopped" \
  grep -q "chest-armed 9106001 2846 world=stopped" "$WORK/state.fixture"
check "the chest is removed while the world is stopped" \
  grep -q "chest-restored 9106001 world=stopped" "$WORK/state.fixture"
check "the chest is armed before the candidate starts" bash -c '
  [[ "$(head -1 "'"$WORK"'/state.log")" == stopped && "$(sed -n 2p "'"$WORK"'/state.log")" == started ]]'
check "the chest scenario restored the build" live_is_original

# 10b. A chest install that fails still runs the cleanup and restores the build.
reset_state
rm -f "$WORK/state.fixture"
status=0
output="$(QA_FAKE_CHEST_APPLY_FAILS=1 run_qa --allow-runtime-qa \
  --ack-disposable-overworld-loot-race --world-exec "$WORK/candidate" loot-race 2>&1)" || status=$?
check "a failed chest install fails the run" bash -c "(( $status != 0 ))"
check "a failed chest install never ran the bot" test ! -f "$WORK/state.bot-saw"
check "a failed chest install still runs the cleanup" \
  grep -q "chest-restored 9106001 world=stopped" "$WORK/state.fixture"
check "a failed chest install restored the build" live_is_original

# 10c. A chest that cannot be removed is the run's outcome, not a warning.
reset_state
rm -f "$WORK/state.fixture"
status=0
output="$(QA_FAKE_CHEST_RESTORE_FAILS=1 run_qa --allow-runtime-qa \
  --ack-disposable-overworld-loot-race --world-exec "$WORK/candidate" loot-race 2>&1)" || status=$?
check "a failed chest cleanup is reported" grep -q "chest fixture was NOT restored" <<<"$output"
check "a failed chest cleanup fails the run" bash -c "(( $status == 70 ))"

# 10d. A restore whose stop fails, or leaves the unit active, must not clean
# the chest under a possibly running candidate: the chest stays armed, the
# build is not copied, the run exits 70 and the report cannot say passed.
for mode in QA_FAKE_FAIL_STOP_AT QA_FAKE_STILL_ACTIVE_AT; do
  reset_state
  rm -f "$WORK/state.fixture" "$WORK/report.json"
  status=0
  EXTRA_ENV=("$mode=2")
  output="$(run_qa --allow-runtime-qa --ack-disposable-overworld-loot-race \
    --world-exec "$WORK/candidate" --report "$WORK/report.json" loot-race 2>&1)" || status=$?
  check "$mode: the bot ran and passed before the restore" bot_saw_candidate
  check "$mode: an unverified stop exits 70" test "$status" -eq 70
  check "$mode: an unverified stop skips the chest cleanup" \
    bash -c '! grep -q "chest-restored" "'"$WORK"'/state.fixture"'
  check "$mode: an unverified stop says the chest is still installed" \
    grep -q "chest fixture was NOT restored" <<<"$output"
  check "$mode: an unverified stop does not copy over the candidate" \
    bash -c '[[ "$(cat "'"$WORK"'/live/world-server")" == CANDIDATE-BUILD ]]'
  check "$mode: an unverified stop keeps the original copy" grep -q "Original kept at" <<<"$output"
  check "$mode: the report records the failed restore" \
    grep -q '"outcome":"restore-failed"' "$WORK/report.json"
  check "$mode: the report keeps the bot status" grep -q '"bot_status":0' "$WORK/report.json"
done
rm -f "$WORK/state.still-active"

# 10e. On the run path no database write happens until the world is proven
# stopped: a failed stop, a unit still active, or a surviving process.
for mode in QA_FAKE_FAIL_STOP_AT=1 QA_FAKE_STILL_ACTIVE_AT=1 QA_FAKE_PROCESS_ALIVE=1; do
  reset_state
  rm -f "$WORK/state.fixture"
  EXTRA_ENV=("$mode")
  status=0
  output="$(run_qa --allow-runtime-qa --ack-disposable-overworld-loot-race \
    --world-exec "$WORK/candidate" loot-race 2>&1)" || status=$?
  check "$mode: an unproven stop fails the run" bash -c "(( $status != 0 ))"
  check "$mode: an unproven stop writes no fixture" \
    bash -c '[[ ! -f "'"$WORK"'/state.fixture" ]] || ! grep -q "chest-armed" "'"$WORK"'/state.fixture"'
  check "$mode: an unproven stop never ran the bot" test ! -f "$WORK/state.bot-saw"
  check "$mode: an unproven stop installs no candidate" live_is_original
  rm -f "$WORK/state.still-active"
done

# 10f. The journal directory is never chmod-ed through a symlink.
reset_state
mkdir -p "$WORK/elsewhere"
chmod 755 "$WORK/elsewhere"
ln -s "$WORK/elsewhere" "$WORK/journal-link"
EXTRA_ENV=(QA_JOURNAL_DIR="$WORK/journal-link")
status=0
output="$(run_qa --allow-runtime-qa \
  --ack-disposable-overworld-loot-race --world-exec "$WORK/candidate" loot-race 2>&1)" || status=$?
check "a symlinked journal directory is refused" grep -q "must be a real directory" <<<"$output"
check "a symlinked journal directory's target keeps its mode" \
  bash -c '[[ "$(stat -c %a "'"$WORK"'/elsewhere")" == 755 ]]'
check "a symlinked journal directory never ran the bot" test ! -f "$WORK/state.bot-saw"
rm -f "$WORK/journal-link"

# 11. The creature scenario arms the fixture before the candidate starts and
# restores it afterwards, under the same trap as the build.
reset_state
rm -f "$WORK/state.fixture"
output="$(run_qa --allow-runtime-qa --ack-disposable-overworld-loot-race \
  --world-exec "$WORK/candidate" loot-item 2>&1 || true)"
check "the creature scenario arms the fixture" \
  grep -q "fixture-armed 21779" "$WORK/state.fixture"
check "the creature scenario restores the fixture" \
  grep -q "fixture-restored 21779" "$WORK/state.fixture"
check "the fixture is armed before the candidate starts" bash -c '
  armed="$(grep -n "fixture-armed" "'"$WORK"'/state.fixture" | head -1 | cut -d: -f1)"
  [[ -n "$armed" ]]'
check "the creature scenario restored the build" live_is_original

# 12. A fixture that cannot be restored is the run's outcome, not a warning.
reset_state
rm -f "$WORK/state.fixture"
status=0
output="$(QA_FAKE_FIXTURE_RESTORE_FAILS=1 run_qa --allow-runtime-qa \
  --ack-disposable-overworld-loot-race --world-exec "$WORK/candidate" loot-item 2>&1)" || status=$?
check "a failed fixture restore is reported" grep -q "was NOT restored" <<<"$output"
check "a failed fixture restore fails the run" bash -c "(( $status == 70 ))"

# 13. The dry run touches nothing, including the database.
reset_state
rm -f "$WORK/state.fixture"
output="$(run_qa --dry-run --allow-runtime-qa --ack-disposable-overworld-loot-race \
  --world-exec "$WORK/candidate" loot-item 2>&1)"
check "the creature dry run names the fixture" grep -q "would guard     creature 21779" <<<"$output"
check "the creature dry run queried no database" bash -c '[[ ! -f "'"$WORK"'/state.fixture" ]]'
check "the creature dry run stopped nothing" bash -c '[[ ! -f "'"$WORK"'/state.log" ]]'

# 14. Login uses the same restore path, pins its mode, and verifies the report.
reset_state
output="$(run_qa --world-exec "$WORK/candidate" login 2>&1 || true)"
check "login needs runtime authorization" grep -q -- "--allow-runtime-qa" <<<"$output"
check "unapproved login stopped nothing" test ! -f "$WORK/state.log"
output="$(run_qa --dry-run --allow-runtime-qa --world-exec "$WORK/candidate" login 2>&1)"
check "login dry run promises restoration" grep -q 'would restore' <<<"$output"
check "login dry run stopped nothing" test ! -f "$WORK/state.log"
EXTRA_ENV=(WOW_BOT_LOOT_RACE_SMOKE=1 WOW_BOT_STAND_STATE=1)
run_qa --allow-runtime-qa --world-exec "$WORK/candidate" --report "$WORK/report.json" login >/dev/null
check "login bot saw candidate" bot_saw_candidate
check "login restored the original" live_is_original
check "login report includes successful restoration" grep -q '"outcome":"passed-restored"' "$WORK/report.json"
check "login disabled inherited loot mode" grep -qx 0 "$WORK/state.bot-mode"
check "login removes the numeric stand-state override entirely" grep -qx '' "$WORK/state.bot-stand-state"
check "login disabled account provisioning" grep -qx 0 "$WORK/state.bot-provisioning"
check "login pinned the bot binary hash" grep -Eq '^[a-f0-9]{64}$' "$WORK/state.bot-sha"
check "login did not touch loot fixtures" test ! -f "$WORK/state.fixture"
reset_state
EXTRA_ENV=(QA_FAKE_BOT_STATUS=3)
status=0
run_qa --allow-runtime-qa --world-exec "$WORK/candidate" --report "$WORK/report.json" login >/dev/null 2>&1 || status=$?
check "login propagates bot failure" test "$status" -eq 3
check "failed login restores" live_is_original
check "failed login reports restoration" grep -q '"outcome":"failed-restored"' "$WORK/report.json"
reset_state
EXTRA_ENV=(QA_FAKE_NO_LOGIN_REPORT=1)
status=0
run_qa --allow-runtime-qa --world-exec "$WORK/candidate" login >/dev/null 2>&1 || status=$?
check "login refuses missing evidence despite zero exit" test "$status" -eq 65
check "missing login evidence still restores" live_is_original
reset_state
EXTRA_ENV=(QA_FAKE_FAIL_START=1)
touch "$WORK/state.swapped"
status=0
run_qa --allow-runtime-qa --world-exec "$WORK/candidate" --report "$WORK/report.json" login >/dev/null 2>&1 || status=$?
check "login restore failure takes precedence" test "$status" -eq 70
check "login report does not hide restore failure" grep -q '"outcome":"restore-failed"' "$WORK/report.json"

# 15. The trade scenario is guarded, swaps under the same restore trap, pins
# its mode and journal, never touches the world fixture guard, and verifies the
# bot's trade evidence (#38).
reset_state
rm -f "$WORK/state.fixture"
output="$(run_qa --world-exec "$WORK/candidate" trade 2>&1 || true)"
check "trade needs runtime authorization" grep -q -- "--allow-runtime-qa" <<<"$output"
output="$(run_qa --allow-runtime-qa --world-exec "$WORK/candidate" trade 2>&1 || true)"
check "trade needs its disposable-fixture acknowledgement" grep -q -- "--ack-disposable-trade" <<<"$output"
output="$(run_qa --allow-runtime-qa --ack-disposable-overworld-loot-race \
  --world-exec "$WORK/candidate" trade 2>&1 || true)"
check "the loot acknowledgement does not authorize trade" grep -q -- "--ack-disposable-trade" <<<"$output"
check "unapproved trade stopped nothing" test ! -f "$WORK/state.log"
output="$(run_qa --dry-run --allow-runtime-qa --ack-disposable-trade \
  --world-exec "$WORK/candidate" trade 2>&1)"
check "trade dry run names its mode" grep -q "WOW_BOT_TRADE_SMOKE=1" <<<"$output"
check "trade dry run stopped nothing" test ! -f "$WORK/state.log"
run_qa --allow-runtime-qa --ack-disposable-trade --world-exec "$WORK/candidate" \
  --report "$WORK/report.json" trade >/dev/null
check "trade bot saw the candidate" bot_saw_candidate
check "trade restored the original" live_is_original
check "trade reports successful restoration" grep -q '"command":"trade","outcome":"passed-restored"' "$WORK/report.json"
check "trade pinned its mode and acknowledgement" grep -qx '1 1' "$WORK/state.bot-trade-mode"
check "trade did not run a loot mode" grep -qx 0 "$WORK/state.bot-mode"
check "trade used a fresh journal under the journal directory" \
  grep -q "^$WORK/journals/trade-" "$WORK/state.bot-journal"
check "trade never touched the world fixture guard" test ! -f "$WORK/state.fixture"
reset_state
status=0
output="$(QA_FAKE_LEAVE_JOURNAL=1 run_qa --allow-runtime-qa --ack-disposable-trade \
  --world-exec "$WORK/candidate" trade 2>&1)" || status=$?
check "a surviving trade journal fails the run" test "$status" -eq 75
check "a surviving trade journal names its recovery" grep -q -- "--recover-trade-fixture" <<<"$output"
check "a surviving trade journal still restores the build" live_is_original
reset_state
status=0
QA_FAKE_BOT_STATUS=4 run_qa --allow-runtime-qa --ack-disposable-trade \
  --world-exec "$WORK/candidate" trade >/dev/null 2>&1 || status=$?
check "trade propagates bot failure" test "$status" -eq 4
check "failed trade restores" live_is_original
reset_state
status=0
QA_FAKE_NO_LOGIN_REPORT=1 run_qa --allow-runtime-qa --ack-disposable-trade \
  --world-exec "$WORK/candidate" trade >/dev/null 2>&1 || status=$?
check "trade refuses missing evidence despite zero exit" test "$status" -eq 65
check "missing trade evidence still restores" live_is_original

# 16. Native capture guards run before the live service is stopped.
reset_state
output="$(run_capture_fake rust "$WORK/capture-guard" 2>&1 || true)"
check "capture requires runtime authorization" grep -q -- "--allow-runtime-qa" <<<"$output"
check "capture authorization guard stopped nothing" test ! -f "$WORK/state.log"
check "capture authorization guard created no output directory" test ! -e "$WORK/capture-guard"

reset_state
status=0
output="$(run_qa --allow-runtime-qa --backend rust --world-exec "$WORK/candidate" \
  --output-dir "$WORK/capture-pin" --candidate-sha256 "$(printf '0%.0s' {1..64})" \
  --scenario-runner "$WORK/bin/capture-runner" capture 2>&1)" || status=$?
check "capture refuses a mismatched candidate pin" grep -q "pinned SHA-256" <<<"$output"
check "candidate pin guard stopped nothing" test ! -f "$WORK/state.log"
check "candidate pin guard created no output directory" test ! -e "$WORK/capture-pin"

reset_state
mkdir -p "$WORK/systemd/fake-world.service.d"
printf 'operator-owned\n' >"$WORK/systemd/fake-world.service.d/43-qa-capture.conf"
output="$(run_capture_fake rust "$WORK/capture-preexisting" 2>&1 || true)"
check "capture refuses a pre-existing systemd drop-in" grep -q "not available" <<<"$output"
check "pre-existing drop-in guard stopped nothing" test ! -f "$WORK/state.log"
check "pre-existing drop-in is preserved" \
  grep -q '^operator-owned$' "$WORK/systemd/fake-world.service.d/43-qa-capture.conf"

reset_state
ln -s "$WORK/systemd" "$WORK/systemd-link"
status=0
EXTRA_ENV=(QA_SYSTEMD_RUNTIME_DIR="$WORK/systemd-link")
output="$(run_capture_fake rust "$WORK/capture-symlink-root" 2>&1)" || status=$?
check "capture rejects a symlinked systemd runtime root" grep -q "unsafe" <<<"$output"
check "symlinked runtime root guard stopped nothing" test ! -f "$WORK/state.log"
rm -f "$WORK/systemd-link"

reset_state
ln -s "$WORK" "$WORK/config-link"
status=0
EXTRA_ENV=(QA_CAPTURE_WORLD_CONFIG="$WORK/config-link/worldserver.conf")
output="$(run_capture_fake cpp "$WORK/capture-symlink-config" 2>&1)" || status=$?
check "capture rejects config with a symlinked ancestor" grep -q "unsafe" <<<"$output"
check "symlinked config guard stopped nothing" test ! -f "$WORK/state.log"
rm -f "$WORK/config-link"

reset_state
printf 'LogsDir = "duplicate"\n' >>"$WORK/worldserver.conf"
status=0
output="$(run_capture_fake cpp "$WORK/capture-config-duplicate" 2>&1)" || status=$?
check "C++ capture requires one LogsDir assignment" grep -q "exactly one" <<<"$output"
check "duplicate config guard stopped nothing" test ! -f "$WORK/state.log"

# If preparation was armed and its recovery journal disappears, restoration
# must hold the service stopped rather than infer that config stayed original.
reset_state
EXTRA_ENV=(QA_FAKE_CAPTURE_DRIFT=journal)
status=0
run_capture_fake rust "$WORK/capture-journal-missing" >/dev/null 2>&1 || status=$?
check "missing armed capture journal exits restore-failed" test "$status" -eq 70
check "missing armed capture journal keeps the unit stopped" \
  bash -c 'test "$(grep -c "^started$" "$1")" -eq 1' _ "$WORK/state.log"
check "missing armed capture journal preserves candidate and drop-in" \
  bash -c 'test "$(cat "$1")" = CANDIDATE-BUILD && test -f "$2"' _ \
    "$WORK/live/world-server" "$WORK/systemd/fake-world.service.d/43-qa-capture.conf"
check "missing armed capture journal is reported for recovery" \
  jq -e '.outcome == "restore-failed" and .original_config_restored == false and .recovery_paths.journal != null' \
    "$WORK/capture-journal-missing/capture-report.json" >/dev/null

# 17. A failed first stop never installs the candidate or capture config.
reset_state
EXTRA_ENV=(QA_FAKE_FAIL_STOP_AT=1)
status=0
run_capture_fake rust "$WORK/capture-stop-fail" >/dev/null 2>&1 || status=$?
check "failed candidate stop fails capture" test "$status" -ne 0
check "failed candidate stop leaves original executable" live_is_original
check "failed candidate stop never ran scenario" test ! -f "$WORK/state.capture-runner"
check "failed candidate stop installed no drop-in" test ! -e "$WORK/systemd/fake-world.service.d/43-qa-capture.conf"
check "failed candidate stop records failed-restored" \
  grep -q '"outcome":"failed-restored"' "$WORK/capture-stop-fail/capture-report.json"

# 18. Rust collection retains private native files and is not gameplay acceptance.
reset_state
run_capture_fake rust "$WORK/capture-rust" >/dev/null 2>&1
check "Rust capture restores original executable" live_is_original
check "Rust capture removes its run-owned drop-in" test ! -e "$WORK/systemd/fake-world.service.d/43-qa-capture.conf"
check "Rust capture retains .bin and .meta artifacts" \
  bash -c 'test -s "$1/client.bin" && test -s "$1/client.meta"' _ "$WORK/capture-rust"
check "Rust capture output directory is private 0700" \
  test "$(stat -c %a "$WORK/capture-rust")" = 700
check "Rust report is captured-restored collection only" \
  jq -e '.outcome == "captured-restored" and .collection_only == true and .gameplay_acceptance == false' \
    "$WORK/capture-rust/capture-report.json" >/dev/null
check "Rust report includes runner exit and native artifact hashes" \
  jq -e '.runner_exit == 0 and .raw_artifact_count == 2 and ([.raw_artifacts[].sha256] | all(test("^[a-f0-9]{64}$")))' \
    "$WORK/capture-rust/capture-report.json" >/dev/null
check "Rust report hashes its stable scenario report" \
  jq -e '.runner.scenario_report_sha256 | test("^[a-f0-9]{64}$")' \
    "$WORK/capture-rust/capture-report.json" >/dev/null
check "capture report omits config secrets and packet payloads" \
  bash -c '! grep -qE "fixture-secret|AUTH-PAYLOAD|PKT-PAYLOAD" "$1"' _ \
    "$WORK/capture-rust/capture-report.json"

# Runner failure is reported separately and still restores the original runtime.
reset_state
EXTRA_ENV=(QA_FAKE_CAPTURE_STATUS=3)
status=0
run_capture_fake rust "$WORK/capture-runner-failure" >/dev/null 2>&1 || status=$?
check "runner failure remains the command exit" test "$status" -eq 3
check "runner failure restores original executable" live_is_original
check "runner failure report keeps its exit" \
  jq -e '.outcome == "failed-restored" and .runner_exit == 3' \
    "$WORK/capture-runner-failure/capture-report.json" >/dev/null
check "runner failure retains native packet" test -s "$WORK/capture-runner-failure/client.bin"

# 19. CAS drift and a failed restoration stop retain the run-owned state and
# never start the original over unresolved candidate config.
reset_state
EXTRA_ENV=(QA_FAKE_CAPTURE_DRIFT=rust)
status=0
run_capture_fake rust "$WORK/capture-dropin-drift" >/dev/null 2>&1 || status=$?
check "drop-in drift holds restoration" test "$status" -eq 70
check "drop-in drift is retained for recovery" \
  grep -q 'concurrent drift' "$WORK/systemd/fake-world.service.d/43-qa-capture.conf"
check "drop-in drift keeps candidate executable installed" \
  bash -c 'test "$(cat "$1")" = CANDIDATE-BUILD' _ "$WORK/live/world-server"
check "drop-in drift report names its private journal" \
  jq -e '.outcome == "restore-failed" and .original_config_restored == false and .recovery_paths.journal != null' \
    "$WORK/capture-dropin-drift/capture-report.json" >/dev/null

reset_state
EXTRA_ENV=(QA_FAKE_FAIL_STOP_AT=2)
status=0
run_capture_fake rust "$WORK/capture-restore-stop-fail" >/dev/null 2>&1 || status=$?
check "restore stop failure exits 70" test "$status" -eq 70
check "restore stop failure keeps run-owned drop-in" test -f "$WORK/systemd/fake-world.service.d/43-qa-capture.conf"
check "restore stop failure does not start original again" \
  bash -c 'test "$(grep -c "^started$" "$1")" -eq 1' _ "$WORK/state.log"
check "restore stop failure retains private journal" \
  test -f "$WORK/capture-restore-stop-fail/.qa-capture-journal.json"

# 20. C++ collection restores the active config and retains its PacketLog PKT.
reset_state
CPP_CONFIG_SHA="$(sha256sum "$WORK/worldserver.conf" | awk '{print $1}')"
run_capture_fake cpp "$WORK/capture-cpp" >/dev/null 2>&1
check "C++ capture restores original executable" live_is_original
check "C++ capture restores active config hash" \
  bash -c 'test "$(sha256sum "$1" | cut -d" " -f1)" = "$2"' _ \
    "$WORK/worldserver.conf" "$CPP_CONFIG_SHA"
check "C++ capture retains native PKT" test -s "$WORK/capture-cpp/cpp.pkt"
check "C++ report records supplied source derivation and restored config" \
  jq -e '.outcome == "captured-restored" and .candidate.source_revision == "a5f8da2ebf5424bf0450ca4e08843ecbf72577bd" and .candidate.derivation == "derivedR6; not clean upstream" and .original_config_restored == true' \
    "$WORK/capture-cpp/capture-report.json" >/dev/null

reset_state
EXTRA_ENV=(QA_FAKE_CAPTURE_DRIFT=cpp)
status=0
run_capture_fake cpp "$WORK/capture-config-drift" >/dev/null 2>&1 || status=$?
check "active C++ config drift holds restoration" test "$status" -eq 70
check "active C++ config drift remains visible" grep -q 'concurrent drift' "$WORK/worldserver.conf"
check "C++ config drift retains journal and config backup" \
  test -f "$WORK/capture-config-drift/.qa-capture-journal.json" \
    -a -f "$WORK/capture-config-drift/.qa-original-worldserver.conf"
check "C++ config drift does not restart over unresolved config" \
  bash -c 'test "$(grep -c "^started$" "$1")" -eq 1' _ "$WORK/state.log"

# A journal armed before drop-in creation restores the known original absence.
mkdir -m 700 "$WORK/capture-armed-absent"
ABSENT_JOURNAL="$WORK/capture-armed-absent/.qa-capture-journal.json"
ABSENT_DROPIN="$WORK/systemd/fake-world.service.d/43-qa-capture.conf"
jq -n --arg journal "$ABSENT_JOURNAL" --arg dropin "$ABSENT_DROPIN" \
  --arg output "$WORK/capture-armed-absent" --argjson uid "$(id -u)" \
  --argjson gid "$(id -g)" \
  '{schema:1,backend:"rust",journal:$journal,dropin:$dropin,
    dropin_sha256:"0000000000000000000000000000000000000000000000000000000000000000",
    dropin_uid:$uid,dropin_gid:$gid,dropin_mode:420,
    dropin_original_absent:true,created_dirs:[],output_dir:$output,files_restored:false}' \
  >"$ABSENT_JOURNAL"
chmod 600 "$ABSENT_JOURNAL"
output="$("$WORK/bin/privileged" python3 "$REPO_ROOT/tools/qa/capture_fileops.py" \
  restore --journal "$ABSENT_JOURNAL")"
check "armed-but-absent Rust drop-in restores" test "$output" = restored-absent
check "armed-but-absent restore leaves systemd path absent" test ! -e "$ABSENT_DROPIN"
check "armed-but-absent journal records restoration" \
  jq -e '.files_restored == true' "$ABSENT_JOURNAL" >/dev/null

printf 'qa-runtime self-test: PASS (%d checks)\n' "$PASSED"
