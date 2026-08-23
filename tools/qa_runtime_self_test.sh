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

mkdir -p "$WORK/live" "$WORK/bin" "$WORK/repo"
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
case "${1:-}" in
  show)
    case "$*" in
      *ActiveState*) printf 'active\n' ;;
      *MainPID*) printf '4242\n' ;;
      *NRestarts*) printf '7\n' ;;
      *Environment*) printf '%s\n' "${QA_FAKE_ENVIRONMENT:-}" ;;
      *) printf '\n' ;;
    esac
    ;;
  stop) printf 'stopped\n' >>"${state}.log" ;;
  start)
    if [[ "${QA_FAKE_FAIL_START:-0}" == "1" && -f "${state}.swapped" ]]; then
      printf 'refusing to start\n' >&2
      exit 1
    fi
    printf 'started\n' >>"${state}.log"
    ;;
esac
FAKE
chmod +x "$WORK/bin/systemctl"

# A fake bot that records which build was live when it ran.
cat >"$WORK/bin/bot" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
cat "${QA_FAKE_LIVE:?}" >"${QA_FAKE_STATE:?}.bot-saw"
exit "${QA_FAKE_BOT_STATUS:-0}"
FAKE
chmod +x "$WORK/bin/bot"

cat >"$WORK/bin/resolver" <<'FAKE'
#!/usr/bin/env bash
printf '%s\n' "${QA_FAKE_LIVE:?}"
FAKE
chmod +x "$WORK/bin/resolver"

run_qa() {
  env \
    QA_SYSTEMCTL="$WORK/bin/systemctl" \
    QA_SERVICE="fake-world" \
    QA_LIVE_DIR="$WORK/live" \
    QA_LIVE_NAME="world-server" \
    QA_BOT="$WORK/bin/bot" \
    QA_LOCK="$WORK/lock" \
    QA_EXE_RESOLVER="$WORK/bin/resolver" \
    QA_SKIP_PORT_GUARD=1 \
    QA_READY_TIMEOUT_SECONDS=5 \
    QA_FAKE_STATE="$WORK/state" \
    QA_FAKE_LIVE="$WORK/live/world-server" \
    QA_GIT_DIR="$WORK/repo" \
    "${EXTRA_ENV[@]}" \
    "$QA" "$@"
}

reset_state() {
  rm -f "$WORK"/state.* "$WORK/lock"
  printf 'ORIGINAL-BUILD\n' >"$WORK/live/world-server"
  EXTRA_ENV=()
}

live_is_original() { [[ "$(cat "$WORK/live/world-server")" == "ORIGINAL-BUILD" ]]; }
bot_saw_candidate() { [[ "$(cat "$WORK/state.bot-saw")" == "CANDIDATE-BUILD" ]]; }

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
check "the report records a pass" grep -q '"outcome":"passed"' "$WORK/report.json"

# 5. A failing smoke still restores, and the failure is the run's status.
reset_state
EXTRA_ENV=(QA_FAKE_BOT_STATUS=3)
status=0
run_qa --allow-runtime-qa --ack-disposable-overworld-loot-race \
  --world-exec "$WORK/candidate" --report "$WORK/report.json" loot-race >/dev/null 2>&1 || status=$?
check "a failing smoke fails the run" test "$status" -eq 3
check "a failing smoke still restores" live_is_original
check "the report records the failure" grep -q '"outcome":"failed"' "$WORK/report.json"

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

printf 'qa-runtime self-test: PASS (%d checks)\n' "$PASSED"
