#!/usr/bin/env bash
# Guarded death-fixture native capture. Scenario behavior belongs to the runner.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
QA_BOT="${QA_BOT:-$REPO_ROOT/target/debug/wow-test-bot}"
QA_GIT_DIR="${QA_GIT_DIR:-$REPO_ROOT}"
DEATH_FIXTURE_SCRIPT="${QA_DEATH_FIXTURE_SCRIPT:-$REPO_ROOT/tools/qa/death_fixture.py}"
# shellcheck source=tools/qa-runtime.sh
source "$REPO_ROOT/tools/qa-runtime.sh"

ALLOW_DEATH_QA=0
ACK_DEATH_FIXTURE=0
DEATH_BACKEND=""
DEATH_CANDIDATE_SHA=""
DEATH_OUTPUT=""
DEATH_BOT_CONFIG="${QA_DEATH_BOT_CONFIG:-$REPO_ROOT/tools/wow-test-bot/config.json}"
DEATH_RUNNERS=()
DEATH_OFFLINE_TIMEOUT=60
DEATH_JOURNAL=""
DEATH_ORIGINAL_SHA=""
DEATH_CONFIG_SHA=""
DEATH_STOPPED=0
DEATH_PREPARED=0
DEATH_RESTORED=0
DEATH_HOLD=0
DEATH_CAPTURE_IN_PROGRESS=0
DEATH_CLEANUP_DONE=0
DEATH_CLEANUP_STATUS=pending
DEATH_SCENARIO_STATUS=not_run
DEATH_COLLECTION_STATUS=not_run
DEATH_EPOCHS='[]'
DEATH_PIDS=()

usage_death_runtime() {
  cat <<'USAGE'
Guarded disposable death-fixture native capture

Usage:
  ./tools/qa-death-runtime.sh --allow-runtime-qa --ack-disposable-death-fixture \
    --backend rust|cpp --world-exec PATH --candidate-sha256 SHA256 \
    --output-dir ABSOLUTE_PATH --scenario-runner ABSOLUTE_EXECUTABLE [OPTIONS]

Options:
  --scenario-runner PATH       Add one capture epoch; repeat at most twice.
  --source-path PATH           Required for cpp; canonical source checkout.
  --source-revision REV        Required for cpp provenance.
  --derivation TEXT            Required for cpp provenance.
  --world-config PATH          Active config used by capture and fixture helper.
  --bot-config PATH            Existing private config; never rewritten.
  --offline-timeout-seconds N  Bounded wait for fixture actors to go offline (default 60).
  -h, --help                   Show this help.

The output directory must be fresh, canonical, mode 0700 and outside this checkout.
The runner receives RUSTYCORE_DEATH_JOURNAL and the usual RUSTYCORE_CAPTURE_* values.
Scenario observation, native collection, cleanup and parity are reported separately.
This wrapper does not establish full issue acceptance or parity.
USAGE
}

while (($#)); do
  case "$1" in
    --allow-runtime-qa) ALLOW_DEATH_QA=1; shift ;;
    --ack-disposable-death-fixture) ACK_DEATH_FIXTURE=1; shift ;;
    --backend) (($# >= 2)) || die "--backend needs rust or cpp"; DEATH_BACKEND="$2"; shift 2 ;;
    --world-exec) (($# >= 2)) || die "--world-exec needs a path"; WORLD_EXEC="$2"; shift 2 ;;
    --candidate-sha256) (($# >= 2)) || die "--candidate-sha256 needs a hash"; DEATH_CANDIDATE_SHA="$2"; shift 2 ;;
    --output-dir) (($# >= 2)) || die "--output-dir needs a path"; DEATH_OUTPUT="$2"; shift 2 ;;
    --scenario-runner) (($# >= 2)) || die "--scenario-runner needs a path"; DEATH_RUNNERS+=("$2"); shift 2 ;;
    --source-path) (($# >= 2)) || die "--source-path needs a path"; CAPTURE_SOURCE_PATH="$2"; shift 2 ;;
    --source-revision) (($# >= 2)) || die "--source-revision needs a revision"; CAPTURE_SOURCE_REVISION="$2"; shift 2 ;;
    --derivation) (($# >= 2)) || die "--derivation needs a description"; CAPTURE_DERIVATION="$2"; shift 2 ;;
    --world-config)
      (($# >= 2)) || die "--world-config needs a path"
      QA_CAPTURE_WORLD_CONFIG="$2"; QA_CAPTURE_CPP_CONFIG="$2"; shift 2 ;;
    --bot-config) (($# >= 2)) || die "--bot-config needs a path"; DEATH_BOT_CONFIG="$2"; shift 2 ;;
    --offline-timeout-seconds)
      (($# >= 2)) || die "--offline-timeout-seconds needs a positive integer"
      DEATH_OFFLINE_TIMEOUT="$2"; shift 2 ;;
    -h|--help) usage_death_runtime; exit 0 ;;
    *) die "unknown option: $1" ;;
  esac
done

((ALLOW_DEATH_QA == 1)) || die "pass --allow-runtime-qa to stop and start $QA_SERVICE"
((ACK_DEATH_FIXTURE == 1)) || die "pass --ack-disposable-death-fixture"
[[ "$DEATH_BACKEND" == rust || "$DEATH_BACKEND" == cpp ]] || die "--backend must be rust or cpp"
[[ "$WORLD_EXEC" == /* && -x "$WORLD_EXEC" && ! -L "$WORLD_EXEC" \
  && "$(realpath -e -- "$WORLD_EXEC")" == "$WORLD_EXEC" ]] \
  || die "--world-exec must be an absolute canonical executable"
[[ "$DEATH_CANDIDATE_SHA" =~ ^[[:xdigit:]]{64}$ ]] || die "--candidate-sha256 must be 64 hex digits"
DEATH_CANDIDATE_SHA="${DEATH_CANDIDATE_SHA,,}"
[[ "$(sha256_of "$WORLD_EXEC")" == "$DEATH_CANDIDATE_SHA" ]] \
  || die "candidate executable does not match its SHA-256 pin"
[[ "${#DEATH_RUNNERS[@]}" -ge 1 && "${#DEATH_RUNNERS[@]}" -le 2 ]] \
  || die "supply one or two --scenario-runner paths"
[[ "$DEATH_OFFLINE_TIMEOUT" =~ ^[1-9][0-9]*$ && "$DEATH_OFFLINE_TIMEOUT" -le 300 ]] \
  || die "offline timeout must be between 1 and 300 seconds"

if [[ "$DEATH_BACKEND" == cpp ]]; then
  [[ -n "$CAPTURE_SOURCE_PATH" && -n "$CAPTURE_SOURCE_REVISION" && -n "$CAPTURE_DERIVATION" ]] \
    || die "cpp requires --source-path, --source-revision and --derivation"
  [[ -d "$CAPTURE_SOURCE_PATH" && ! -L "$CAPTURE_SOURCE_PATH" \
    && "$(realpath -e -- "$CAPTURE_SOURCE_PATH")" == "$CAPTURE_SOURCE_PATH" ]] \
    || die "cpp --source-path must be a canonical existing directory"
elif [[ -n "$CAPTURE_SOURCE_PATH" || -n "$CAPTURE_SOURCE_REVISION" || -n "$CAPTURE_DERIVATION" ]]; then
  die "C++ provenance options require --backend cpp"
fi
[[ "$QA_CAPTURE_WORLD_CONFIG" == /* && -f "$QA_CAPTURE_WORLD_CONFIG" \
  && ! -L "$QA_CAPTURE_WORLD_CONFIG" \
  && "$(realpath -e -- "$QA_CAPTURE_WORLD_CONFIG")" == "$QA_CAPTURE_WORLD_CONFIG" ]] \
  || die "world config must be an absolute canonical file"
[[ "$DEATH_BOT_CONFIG" == /* && -f "$DEATH_BOT_CONFIG" && ! -L "$DEATH_BOT_CONFIG" \
  && "$(realpath -e -- "$DEATH_BOT_CONFIG")" == "$DEATH_BOT_CONFIG" ]] \
  || die "bot config must be an absolute canonical file"
[[ -x "$QA_BOT" && ! -L "$QA_BOT" && "$(realpath -e -- "$QA_BOT")" == "$QA_BOT" ]] \
  || die "checkout QA bot is missing or not canonical: $QA_BOT"
[[ -r "$REPO_ROOT/tools/qa/capture_fileops.py" && -f "$REPO_ROOT/tools/qa/capture_fileops.py" ]] \
  || die "capture file-operations helper is missing"
[[ "$DEATH_FIXTURE_SCRIPT" == /* && -r "$DEATH_FIXTURE_SCRIPT" \
  && -f "$DEATH_FIXTURE_SCRIPT" && ! -L "$DEATH_FIXTURE_SCRIPT" \
  && "$(realpath -e -- "$DEATH_FIXTURE_SCRIPT")" == "$DEATH_FIXTURE_SCRIPT" ]] \
  || die "death fixture helper is missing"
for DEATH_RUNNER in "${DEATH_RUNNERS[@]}"; do
  [[ "$DEATH_RUNNER" == /* && -x "$DEATH_RUNNER" && ! -L "$DEATH_RUNNER" \
    && "$(realpath -e -- "$DEATH_RUNNER")" == "$DEATH_RUNNER" ]] \
    || die "scenario runners must be absolute canonical executables"
done
[[ -n "$DEATH_OUTPUT" ]] || die "--output-dir is required"
CAPTURE_OUTPUT_DIR="$DEATH_OUTPUT"
validate_capture_output_path
[[ "$DEATH_OUTPUT" != "$REPO_ROOT" && "$DEATH_OUTPUT" != "$REPO_ROOT"/* ]] \
  || die "output directory must be outside this checkout"
for DEATH_COMMAND in ss timeout jq python3 realpath stat flock sha256sum; do
  require_command "$DEATH_COMMAND"
done
require_clean_worktree
have_bot_credentials || die "no bot credentials before swapping a build"

umask 077
mkdir -m 700 -- "$DEATH_OUTPUT"
[[ "$(stat -c %a -- "$DEATH_OUTPUT")" == 700 && -d "$DEATH_OUTPUT" && ! -L "$DEATH_OUTPUT" ]] \
  || die "campaign directory was not created with mode 0700"
mkdir -m 700 -- "$DEATH_OUTPUT/epochs"
DEATH_JOURNAL="$DEATH_OUTPUT/death-fixture.journal"
[[ ! -e "$DEATH_JOURNAL" && ! -L "$DEATH_JOURNAL" \
  && ! -e "$DEATH_JOURNAL.cleanup-complete" && ! -L "$DEATH_JOURNAL.cleanup-complete" ]] \
  || die "fixture journal path is not fresh"

exec 9>"$QA_LOCK"
flock -n 9 || die "another runtime QA run holds $QA_LOCK"

death_runtime_write_report() {
  local report="$DEATH_OUTPUT/death-runtime-report.json" temp="$DEATH_OUTPUT/.death-runtime-report.tmp"
  jq -n --arg backend "$DEATH_BACKEND" --arg original "$DEATH_ORIGINAL_SHA" \
    --arg candidate "$DEATH_CANDIDATE_SHA" --arg observation "$DEATH_SCENARIO_STATUS" \
    --arg collection "$DEATH_COLLECTION_STATUS" --arg cleanup "$DEATH_CLEANUP_STATUS" \
    --arg journal "$DEATH_JOURNAL" --arg snapshot "$DEATH_OUTPUT/post-run-state-snapshot.json" \
    --argjson epochs "$DEATH_EPOCHS" \
    '{schema:"rustycore-death-runtime-v1", backend:$backend,
      original_runtime_sha256:(if $original == "" then null else $original end),
      candidate_sha256:$candidate,
      scenario_observation:{status:$observation,epochs:$epochs},
      native_collection:{status:$collection,
        epochs:[$epochs[] | {epoch,runner_exit,native_collection}]},
      cleanup:{status:$cleanup,fixture_journal:$journal,state_snapshot:$snapshot},
      parity:{status:"not_assessed",full_issue_claim:false}}' >"$temp" || return 1
  chmod 600 -- "$temp"
  mv -f -- "$temp" "$report"
}

death_runtime_identity_ok() {
  local identity pid
  identity="$(live_identity)" || return 1
  pid="$(cut -f1 <<<"$identity")"
  [[ "$(cut -f3 <<<"$identity")" == "$DEATH_ORIGINAL_SHA" \
    && "$(sha256_of "$QA_CAPTURE_WORLD_CONFIG")" == "$DEATH_CONFIG_SHA" ]] || return 1
  ports_ready "$pid" || return 1
  capture_unit_env_absent || return 1
  packet_dump_absent || return 1
  WORLD_PID="$pid"
}

death_runtime_start_original() {
  [[ "$(sha256_of "$LIVE_PATH")" == "$DEATH_ORIGINAL_SHA" \
    && "$(sha256_of "$QA_CAPTURE_WORLD_CONFIG")" == "$DEATH_CONFIG_SHA" ]] || return 1
  capture_unit_env_absent || return 1
  # shellcheck disable=SC2086
  $QA_SYSTEMCTL start "$QA_SERVICE" || return 1
  local pid identity
  pid="$(wait_until_serving)" || return 1
  identity="$(live_identity)" || return 1
  [[ "$(cut -f3 <<<"$identity")" == "$DEATH_ORIGINAL_SHA" ]] || return 1
  ports_ready "$pid" && packet_dump_absent && capture_unit_env_absent \
    && [[ "$(sha256_of "$QA_CAPTURE_WORLD_CONFIG")" == "$DEATH_CONFIG_SHA" ]] || return 1
  WORLD_PID="$pid"
}

death_runtime_snapshot_offline() {
  local deadline=$((SECONDS + DEATH_OFFLINE_TIMEOUT)) snapshot="$DEATH_OUTPUT/post-run-state-snapshot.json"
  while ((SECONDS < deadline)); do
    local status=0
    set +e
    timeout --foreground --signal=TERM --kill-after=5 20s \
      python3 "$DEATH_FIXTURE_SCRIPT" snapshot \
      --world-config "$QA_CAPTURE_WORLD_CONFIG" --bot-config "$DEATH_BOT_CONFIG" \
      --journal "$DEATH_JOURNAL" >"$snapshot" 2>"$DEATH_OUTPUT/snapshot.stderr"
    status=$?
    set -e
    if ((status == 0)) && jq -e --slurpfile fixture "$DEATH_JOURNAL" '
      . as $s
      | ($s.owned_state.characters.columns | index("guid")) as $g
      | ($s.owned_state.characters.columns | index("online")) as $o
      | [$fixture[0].actors[].guid | tostring] as $ids
      | [$s.owned_state.characters.rows[] as $row
          | select($ids | index($row[$g])) | $row[$o]] as $states
      | $s.mode == "read-only-snapshot"
        and ($states | length) == ($ids | length) and all($states[]; . == "0")
    ' "$snapshot" >/dev/null 2>&1; then return 0; fi
    sleep 2
  done
  return 1
}

death_runtime_finalize() {
  if ((DEATH_HOLD == 1)); then DEATH_CLEANUP_STATUS=held; return 1; fi
  if ((DEATH_PREPARED == 1 && DEATH_RESTORED == 0)); then
    death_runtime_snapshot_offline || {
      DEATH_HOLD=1; DEATH_CLEANUP_STATUS=held_actor_state; return 1;
    }
    if service_active; then
      death_runtime_identity_ok || {
        DEATH_HOLD=1; DEATH_CLEANUP_STATUS=held_runtime_identity; return 1;
      }
      stop_world_verified || {
        DEATH_HOLD=1; DEATH_CLEANUP_STATUS=held_stop; return 1;
      }
      DEATH_STOPPED=1
    elif ! world_stopped "$WORLD_PID"; then
      DEATH_HOLD=1; DEATH_CLEANUP_STATUS=held_stop; return 1
    fi
    if ! python3 "$DEATH_FIXTURE_SCRIPT" restore \
      --world-config "$QA_CAPTURE_WORLD_CONFIG" --bot-config "$DEATH_BOT_CONFIG" \
      --journal "$DEATH_JOURNAL" --ack-disposable-death-fixture --world-stopped \
      >"$DEATH_OUTPUT/fixture-restore.json" 2>"$DEATH_OUTPUT/fixture-restore.stderr"; then
      DEATH_HOLD=1; DEATH_CLEANUP_STATUS=held_fixture_restore; return 1
    fi
    [[ -e "$DEATH_JOURNAL.cleanup-complete" && ! -L "$DEATH_JOURNAL.cleanup-complete" \
      && "$(jq -r '.state' "$DEATH_JOURNAL")" == restored ]] || {
        DEATH_HOLD=1; DEATH_CLEANUP_STATUS=held_fixture_marker; return 1;
      }
    DEATH_RESTORED=1
  elif [[ -e "$DEATH_JOURNAL" || -L "$DEATH_JOURNAL" ]]; then
    DEATH_HOLD=1; DEATH_CLEANUP_STATUS=held_fixture_journal; return 1
  fi
  if ((DEATH_STOPPED == 1)); then
    death_runtime_start_original || {
      DEATH_HOLD=1; DEATH_CLEANUP_STATUS=held_start; return 1;
    }
    DEATH_STOPPED=0
  else
    death_runtime_identity_ok || {
      DEATH_HOLD=1; DEATH_CLEANUP_STATUS=held_runtime_identity; return 1;
    }
  fi
  DEATH_CLEANUP_STATUS=restored
  DEATH_CLEANUP_DONE=1
}

death_runtime_capture_epoch() {
  local index="$1" runner="$2" name dir report scenario_report rc pid scenario_sha scenario_status capture
  printf -v name 'epoch-%03d' "$index"
  dir="$DEATH_OUTPUT/epochs/$name"
  report="$dir/capture-report.json"
  scenario_report="$dir/scenario-report.json"
  CAPTURE_BACKEND="$DEATH_BACKEND"
  CAPTURE_CANDIDATE_SHA="$DEATH_CANDIDATE_SHA"
  CAPTURE_RUNNER="$runner"
  CAPTURE_OUTPUT_DIR="$dir"
  REPORT=""
  DEATH_CAPTURE_IN_PROGRESS=1
  set +e
  (
    set -e
    trap - EXIT
    export RUSTYCORE_DEATH_JOURNAL="$DEATH_JOURNAL"
    export RUSTYCORE_DEATH_EPOCH="$name" RUSTYCORE_DEATH_EPOCH_INDEX="$index"
    export RUSTYCORE_DEATH_BOT_CONFIG="$DEATH_BOT_CONFIG"
    export RUSTYCORE_DEATH_WORLD_CONFIG="$QA_CAPTURE_WORLD_CONFIG"
    ALLOW_RUNTIME_QA=1
    COMMAND=capture
    run_capture
  )
  rc=$?
  set -e
  DEATH_CAPTURE_IN_PROGRESS=0
  if [[ ! -f "$report" || -L "$report" ]] || ! jq -e \
    --arg original "$DEATH_ORIGINAL_SHA" --arg candidate "$DEATH_CANDIDATE_SHA" '
      .schema == "rustycore-native-capture-v1" and .collection_only == true
      and .gameplay_acceptance == false and .original.sha256 == $original
      and .candidate.sha256 == $candidate and .original_config_restored == true
      and .original_runtime_restored == true and .packet_dump_absent_after_restore == true
      and (.outcome == "captured-restored" or .outcome == "failed-restored")
      and (.runner_exit | type) == "number" and (.collection_exit | type) == "number"
      and (.raw_artifact_count | type) == "number"
      and (.listener_before_runner.pid | test("^[1-9][0-9]*$"))
      and .listener_before_runner.pid == .listener_after_runner.pid
      and .listener_before_runner.proc_starttime == .listener_after_runner.proc_starttime
      and .listener_before_runner.restart_count == .listener_after_runner.restart_count
    ' "$report" >/dev/null 2>&1; then
    DEATH_HOLD=1; DEATH_CLEANUP_STATUS=held_capture_restore
    die "capture restore report is missing or unresolved; fixture cleanup is held"
  fi
  if [[ -e "$dir/.qa-capture-journal.json" || -L "$dir/.qa-capture-journal.json" ]]; then
    DEATH_HOLD=1; DEATH_CLEANUP_STATUS=held_capture_evidence
    die "capture journal remains; fixture cleanup is held"
  fi
  death_runtime_identity_ok || {
    DEATH_HOLD=1; DEATH_CLEANUP_STATUS=held_runtime_identity
    die "original runtime/config is unresolved; fixture cleanup is held"
  }
  pid="$(jq -r '.listener_before_runner.pid' "$report")"
  scenario_sha=""
  scenario_status=missing_report
  if [[ -f "$scenario_report" && ! -L "$scenario_report" ]]; then
    scenario_status=present
    scenario_sha="$(sha256_of "$scenario_report")"
  else
    DEATH_SCENARIO_STATUS=failed
  fi
  if ((rc != 0)) && [[ "$scenario_status" != missing_report ]]; then
    scenario_status=runner_failed
    DEATH_SCENARIO_STATUS=failed
  fi
  if [[ "$DEATH_SCENARIO_STATUS" != failed ]]; then DEATH_SCENARIO_STATUS=observed; fi
  for prior in "${DEATH_PIDS[@]}"; do
    [[ "$prior" != "$pid" ]] || DEATH_SCENARIO_STATUS=failed
  done
  DEATH_PIDS+=("$pid")
  capture="$(jq -c '{outcome,backend,runner_exit,collection_exit,raw_artifact_count,
    original_config_restored,original_runtime_restored}' "$report")"
  if jq -e '.collection_exit == 0 and .raw_artifact_count > 0' "$report" >/dev/null; then
    [[ "$DEATH_COLLECTION_STATUS" == failed ]] || DEATH_COLLECTION_STATUS=collected
  else
    DEATH_COLLECTION_STATUS=failed
  fi
  DEATH_EPOCHS="$(jq -cn --argjson old "$DEATH_EPOCHS" --arg epoch "$name" \
    --arg runner "$runner" --arg scenario_report "$scenario_report" \
    --arg scenario_sha "$scenario_sha" --arg scenario_status "$scenario_status" \
    --argjson runner_exit "$rc" --argjson capture "$capture" --argjson pid "$pid" \
    '$old + [{epoch:$epoch,runner:$runner,runner_exit:$runner_exit,candidate_pid:$pid,
      scenario_observation:{status:$scenario_status,report:$scenario_report,sha256:$scenario_sha},
      native_collection:$capture}]')"
}

death_runtime_exit() {
  local status=$? cleanup_status=0
  trap - EXIT HUP INT TERM
  if ((DEATH_CAPTURE_IN_PROGRESS == 1)); then
    DEATH_HOLD=1; DEATH_CLEANUP_STATUS=held_capture_in_progress
  fi
  if ((DEATH_HOLD == 0 && DEATH_CLEANUP_DONE == 0)); then
    set +e
    death_runtime_finalize
    cleanup_status=$?
    set -e
    ((cleanup_status == 0)) || status=70
  elif ((DEATH_HOLD == 1)); then
    DEATH_CLEANUP_STATUS=held
    ((status != 0)) || status=70
  fi
  if ! death_runtime_write_report; then
    warn "could not write the private death-runtime report"
    ((status != 0)) || status=70
  fi
  exit "$status"
}

# Runtime mutation begins only after every guard and cleanup function exists.
LIVE_PATH="$QA_LIVE_DIR/$QA_LIVE_NAME"
[[ -f "$LIVE_PATH" && ! -L "$LIVE_PATH" ]] || die "live executable is missing or unsafe"
DEATH_IDENTITY="$(live_identity)"
WORLD_PID="$(cut -f1 <<<"$DEATH_IDENTITY")"
DEATH_ORIGINAL_SHA="$(cut -f3 <<<"$DEATH_IDENTITY")"
DEATH_CONFIG_SHA="$(sha256_of "$QA_CAPTURE_WORLD_CONFIG")"
ports_ready "$WORLD_PID" || die "live listeners are not owned by PID $WORLD_PID"
packet_dump_absent || die "a packet dump directory is already configured"
trap death_runtime_exit EXIT
trap 'exit 130' HUP INT TERM

stop_world_verified || { DEATH_HOLD=1; die "$QA_SERVICE did not stop cleanly"; }
DEATH_STOPPED=1
if python3 "$REPO_ROOT/tools/qa/death_fixture.py" prepare \
  --world-config "$QA_CAPTURE_WORLD_CONFIG" --bot-config "$DEATH_BOT_CONFIG" \
  --journal "$DEATH_JOURNAL" --ack-disposable-death-fixture --world-stopped \
  >"$DEATH_OUTPUT/fixture-prepare.json" 2>"$DEATH_OUTPUT/fixture-prepare.stderr"; then
  DEATH_PREPARED=1
else
  if [[ -e "$DEATH_JOURNAL" || -L "$DEATH_JOURNAL" ]]; then
    DEATH_HOLD=1; DEATH_CLEANUP_STATUS=held_fixture_prepare
  fi
  die "fixture preparation failed; inspect the private campaign journal and logs"
fi

death_runtime_start_original || die "original service did not restart after fixture preparation"
DEATH_STOPPED=0
DEATH_SCENARIO_STATUS=observed
DEATH_COLLECTION_STATUS=collected
for DEATH_INDEX in "${!DEATH_RUNNERS[@]}"; do
  death_runtime_capture_epoch "$((DEATH_INDEX + 1))" "${DEATH_RUNNERS[$DEATH_INDEX]}"
done
if [[ "$DEATH_SCENARIO_STATUS" == failed || "$DEATH_COLLECTION_STATUS" == failed ]]; then exit 1; fi
exit 0
