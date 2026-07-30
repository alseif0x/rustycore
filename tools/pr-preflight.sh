#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
POLICY_FILE="$REPO_ROOT/tools/codex-review-policy.md"
SCHEMA_FILE="$REPO_ROOT/tools/codex-review-schema.json"
ARCHITECTURE_CHECKER="$REPO_ROOT/tools/architecture/check_architecture.py"
HANDLER_CONTRACT_CHECK_MANIFEST="$REPO_ROOT/tools/architecture/handler-contract-check/Cargo.toml"
PROTOC_VERSION_FILE="$REPO_ROOT/.protoc-version"
DEFAULT_BASE="origin/3.4.3"
DEFAULT_RUST_MIN_STACK=268435456
CODEX_REVIEW_TIMEOUT_SECONDS="${CODEX_REVIEW_TIMEOUT_SECONDS:-1800}"
DRY_RUN=0
ALLOW_RUNTIME_QA=0
ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=0
RUST_MIN_STACK="${RUST_MIN_STACK:-$DEFAULT_RUST_MIN_STACK}"
export RUST_MIN_STACK
QA_LOOT_RACE_CAPTURE_SCRIPT="$REPO_ROOT/crates/capture-diff/scripts/capture-rust.sh"
QA_LOOT_RACE_CAPTURE_PID=""
QA_LOOT_RACE_CAPTURE_FD=""
QA_LOOT_RACE_CAPTURE_DIR=""
QA_LOOT_RACE_CAPTURE_LOG=""
QA_LOOT_RACE_FIXTURE_JOURNAL=""
QA_LOOT_RACE_CAPTURE_WAIT_STATUS=0
QA_LOOT_RACE_BOT_PID=""
QA_LOOT_RACE_PRE_READY_MARGIN_SECONDS=120

usage() {
  cat <<'EOF'
RustyCore local PR preflight

Usage:
  ./tools/pr-preflight.sh [OPTIONS] <COMMAND> [BASE]

Options:
  --dry-run                              Print commands without running them.
  --allow-runtime-qa                     Allow live QA commands to modify local QA data.
  --ack-disposable-overworld-loot-race  Acknowledge qa-loot-race mutates its exact shared-chest fixture.
  -h, --help                             Show this help.

Commands:
  self-test           Test harness parsing and pinned-version invariants.
  architecture        Check dependency boundaries and report source hotspots.
  format              Run the three formatting checks used by GitHub Actions.
  check               Run the locked core checks and server builds used by CI.
  test                Run focused suites, loot-race tests, and required capture gate used by CI.
  ci                  Run architecture, format, check, and test.
  diff [BASE]         Check committed, staged, and unstaged diffs for whitespace errors.
  quick [BASE]        Run diff, architecture, format, and check.
  capture             Run capture-diff regression tests (protoc not required).
  review [BASE]       Review the clean committed diff with local Codex.
  review-uncommitted  Review staged, unstaged, and untracked changes with local Codex.
  full [BASE]         Run diff, CI (including architecture/capture), and review on a clean HEAD.
  stable              Check/build the server binaries with latest stable Rust.
  qa-login            Run the existing live login bot; requires --allow-runtime-qa.
  qa-loot-race        Run destructive live two-session loot QA; requires both QA flags.

BASE defaults to origin/3.4.3. The GitHub Codex reviewer verdict remains required.
EOF
}

log() {
  printf '\n==> %s\n' "$*"
}

warn() {
  printf 'warning: %s\n' "$*" >&2
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 64
}

validate_rust_min_stack() {
  [[ "$RUST_MIN_STACK" =~ ^[1-9][0-9]*$ ]] || die \
    "RUST_MIN_STACK must be a positive integer"
  ((RUST_MIN_STACK >= DEFAULT_RUST_MIN_STACK)) || die \
    "RUST_MIN_STACK must be at least $DEFAULT_RUST_MIN_STACK bytes for Rust 1.88"
}

print_command() {
  printf '+'
  printf ' %q' "$@"
  printf '\n'
}

run_cmd() {
  print_command "$@"
  if ((DRY_RUN == 0)); then
    "$@"
  fi
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || die "required command not found: $1"
}

sha256_of_file() {
  local output digest
  output="$(sha256sum <"$1")" || return 1
  digest="${output%% *}"
  [[ "$digest" =~ ^[0-9a-f]{64}$ ]] || return 1
  printf '%s' "$digest"
}

qa_world_identity() {
  local process_name="$1"
  local expected_exec="$2"
  pm2 jlist | jq -er --arg name "$process_name" --arg expected_exec "$expected_exec" '
    [.[] | select(.name == $name)] as $matches
    | if ($matches | length) != 1 then
        error("PM2 world process must exist exactly once")
      else $matches[0] end
    | if .pm2_env.status != "online"
        or (.pid // 0) <= 0
        or .pm2_env.pm_exec_path != $expected_exec
      then error("PM2 world process is not the pinned online executable")
      else [.pid, (.pm2_env.restart_time // 0)] | @tsv end
  '
}

qa_world_snapshot() {
  local process_name="$1"
  pm2 jlist | jq -er --arg name "$process_name" '
    [.[] | select(.name == $name)] as $matches
    | if ($matches | length) != 1 then
        error("PM2 world process must exist exactly once")
      else $matches[0] end
    | if .pm2_env.status != "online"
        or (.pid // 0) <= 0
        or (.pm2_env.pm_exec_path | type) != "string"
        or .pm2_env.pm_exec_path == ""
      then error("PM2 world process is not an online executable")
      else [.pid, (.pm2_env.restart_time // 0), .pm2_env.pm_exec_path] | @tsv end
  '
}

qa_world_process_matches() {
  local identity="$1"
  local expected_exec="$2"
  local expected_sha="$3"
  local pid proc_exe live_exec source_sha live_sha

  [[ "$identity" == *$'\t'* ]] || return 1
  pid="${identity%%$'\t'*}"
  [[ "$pid" =~ ^[1-9][0-9]*$ ]] || return 1
  proc_exe="/proc/${pid}/exe"
  [[ -L "$proc_exe" ]] || return 1
  live_exec="$(realpath -e -- "$proc_exe" 2>/dev/null)" || return 1
  [[ "$live_exec" == "$expected_exec" ]] || return 1
  source_sha="$(sha256_of_file "$expected_exec")" || return 1
  live_sha="$(sha256_of_file "$proc_exe")" || return 1
  [[ "$source_sha" == "$expected_sha" && "$live_sha" == "$expected_sha" ]]
}

qa_world_ports_ready() {
  local identity="$1"
  local world_port="$2"
  local instance_port="$3"
  local pid

  [[ "$world_port" != "$instance_port" ]] || return 1
  pid="${identity%%$'\t'*}"
  qa_port_owned_exclusively_by_pid "$world_port" "$pid" \
    && qa_port_owned_exclusively_by_pid "$instance_port" "$pid"
}

qa_port_owned_exclusively_by_pid() {
  local port="$1"
  local pid="$2"
  local sockets socket remaining seen_pid matched_pid

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

qa_world_packet_dump_absent() {
  local process_name="$1"
  pm2 jlist | jq -e --arg name "$process_name" '
    [.[] | select(.name == $name)] as $matches
    | ($matches | length) == 1
      and (($matches[0].pm2_env | has("RUSTYCORE_PACKET_DUMP_DIR")) | not)
      and (((($matches[0].pm2_env.env // {})
        | has("RUSTYCORE_PACKET_DUMP_DIR"))) | not)
  ' >/dev/null
}

qa_loot_race_capture_cleanup() {
  local status=$?
  local bot_wait_status=0
  local wrapper_status="$QA_LOOT_RACE_CAPTURE_WAIT_STATUS"
  local recovery_pending=0

  trap - EXIT
  trap '' HUP INT TERM
  if [[ "$QA_LOOT_RACE_BOT_PID" =~ ^[1-9][0-9]*$ ]]; then
    if kill -0 "$QA_LOOT_RACE_BOT_PID" 2>/dev/null; then
      kill -TERM "$QA_LOOT_RACE_BOT_PID" 2>/dev/null || true
    fi
    wait "$QA_LOOT_RACE_BOT_PID" || bot_wait_status=$?
  fi
  QA_LOOT_RACE_BOT_PID=""
  if [[ "$QA_LOOT_RACE_CAPTURE_PID" =~ ^[1-9][0-9]*$ ]]; then
    if kill -0 "$QA_LOOT_RACE_CAPTURE_PID" 2>/dev/null; then
      kill -TERM "$QA_LOOT_RACE_CAPTURE_PID" 2>/dev/null || true
    fi
    wait "$QA_LOOT_RACE_CAPTURE_PID" || wrapper_status=$?
  fi
  QA_LOOT_RACE_CAPTURE_PID=""
  if [[ "$QA_LOOT_RACE_CAPTURE_FD" =~ ^[0-9]+$ ]]; then
    exec {QA_LOOT_RACE_CAPTURE_FD}>&- || true
  fi
  QA_LOOT_RACE_CAPTURE_FD=""
  # A wrapper cleanup/restoration failure is more important than the original
  # bot/signal status because it means the normal world may be unsafe to start.
  if ((wrapper_status != 0)); then
    status=$wrapper_status
  elif ((status == 0 && bot_wait_status != 0)); then
    status=$bot_wait_status
  fi
  if [[ -n "$QA_LOOT_RACE_FIXTURE_JOURNAL" ]] \
      && { [[ -e "$QA_LOOT_RACE_FIXTURE_JOURNAL" \
          || -L "$QA_LOOT_RACE_FIXTURE_JOURNAL" ]] \
        || [[ -e "${QA_LOOT_RACE_FIXTURE_JOURNAL}.cleanup-complete" \
          || -L "${QA_LOOT_RACE_FIXTURE_JOURNAL}.cleanup-complete" ]]; }; then
    recovery_pending=1
  fi
  if ((status != 0)) && [[ -s "$QA_LOOT_RACE_CAPTURE_LOG" ]]; then
    echo "qa-loot-race guarded capture log:" >&2
    sed -n '1,240p' "$QA_LOOT_RACE_CAPTURE_LOG" >&2 || true
  fi
  if [[ -n "$QA_LOOT_RACE_CAPTURE_DIR" && -d "$QA_LOOT_RACE_CAPTURE_DIR" ]]; then
    rm -f -- "$QA_LOOT_RACE_CAPTURE_DIR/control.fifo"
    if ((status == 0 && recovery_pending == 0)); then
      rm -f -- "$QA_LOOT_RACE_CAPTURE_DIR/capture.log"
      rmdir -- "$QA_LOOT_RACE_CAPTURE_DIR" 2>/dev/null || true
    else
      echo "qa-loot-race recovery artifacts retained at ${QA_LOOT_RACE_CAPTURE_DIR}" >&2
    fi
  fi
  QA_LOOT_RACE_CAPTURE_DIR=""
  QA_LOOT_RACE_CAPTURE_LOG=""
  QA_LOOT_RACE_FIXTURE_JOURNAL=""
  QA_LOOT_RACE_CAPTURE_WAIT_STATUS=0
  exit "$status"
}

qa_loot_capture_ready_timeout_seconds() {
  local world_ready_timeout="${1:-${CAPTURE_WORLD_READY_TIMEOUT_SECONDS:-180}}"
  local world_stop_timeout="${2:-${CAPTURE_WORLD_STOP_TIMEOUT_SECONDS:-30}}"

  [[ "$world_ready_timeout" =~ ^[1-9][0-9]*$ ]] \
    && ((${#world_ready_timeout} <= 4)) \
    && ((10#$world_ready_timeout >= 3)) \
    && ((10#$world_ready_timeout <= 3600)) || {
    echo "CAPTURE_WORLD_READY_TIMEOUT_SECONDS must be an integer from 3 through 3600" >&2
    return 1
  }
  [[ "$world_stop_timeout" =~ ^[1-9][0-9]*$ ]] \
    && ((${#world_stop_timeout} <= 4)) \
    && ((10#$world_stop_timeout <= 3600)) || {
    echo "CAPTURE_WORLD_STOP_TIMEOUT_SECONDS must be an integer from 1 through 3600" >&2
    return 1
  }
  printf '%s\n' "$((world_ready_timeout + world_stop_timeout + QA_LOOT_RACE_PRE_READY_MARGIN_SECONDS))"
}

qa_wait_for_loot_capture_ready() {
  local capture_pid="$1"
  local capture_log="$2"
  local marker=">>> Perform the 'loot-two-session-atomic-race' flow with the client now."
  local wrapper_status=0
  local timeout_seconds deadline

  timeout_seconds="$(qa_loot_capture_ready_timeout_seconds)" || return 1
  deadline=$((SECONDS + timeout_seconds))
  while ((SECONDS < deadline)); do
    if [[ -f "$capture_log" ]] && rg -Fq -- "$marker" "$capture_log"; then
      return 0
    fi
    if ! kill -0 "$capture_pid" 2>/dev/null; then
      wait "$capture_pid" || wrapper_status=$?
      QA_LOOT_RACE_CAPTURE_WAIT_STATUS=$wrapper_status
      QA_LOOT_RACE_CAPTURE_PID=""
      echo "guarded capture wrapper exited before its ready marker (status $wrapper_status)" >&2
      return 1
    fi
    sleep 0.25
  done
  echo "timed out waiting ${timeout_seconds} seconds for the guarded capture ready marker" >&2
  return 1
}

require_exact_occurrences() {
  local text="$1"
  local needle="$2"
  local expected="$3"
  local label="$4"
  local count=0

  [[ -n "$needle" ]] || die "cannot count an empty self-test pattern for $label"
  while [[ "$text" == *"$needle"* ]]; do
    text="${text#*"$needle"}"
    ((count += 1))
  done
  ((count == expected)) || die \
    "$label appeared $count time(s); expected exactly $expected"
}

self_test_cleanup() {
  local world_pid="${1:-}"
  local artifacts="${2:-}"

  if [[ "$world_pid" =~ ^[1-9][0-9]*$ ]] && kill -0 "$world_pid" 2>/dev/null; then
    kill "$world_pid" 2>/dev/null || true
    wait "$world_pid" 2>/dev/null || true
  fi
  if [[ -n "$artifacts" && -d "$artifacts" ]]; then
    rm -rf -- "$artifacts"
  fi
}

valid_tcp_port() {
  local value="$1"
  [[ "$value" =~ ^[0-9]+$ ]] || return 1
  ((10#$value >= 1 && 10#$value <= 65535))
}

toolchain_channel() {
  local channel
  channel="$(sed -n 's/^channel = "\([^"]*\)"/\1/p' "$REPO_ROOT/rust-toolchain.toml" | head -n 1)"
  [[ -n "$channel" ]] || die "cannot read toolchain channel from rust-toolchain.toml"
  printf '%s' "$channel"
}

cargo_cmd() {
  local channel
  ((DRY_RUN)) || require_command cargo
  channel="$(toolchain_channel)"
  # Rust 1.88 can ICE while reloading a stale incremental dep graph. Keep the
  # compile/test gate deterministic without leaking this setting into live QA.
  run_cmd env CARGO_INCREMENTAL=0 cargo "+$channel" "$@"
}

project_protoc_version() {
  local version
  [[ -r "$PROTOC_VERSION_FILE" ]] || die "missing protoc version file: $PROTOC_VERSION_FILE"
  version="$(tr -d '[:space:]' <"$PROTOC_VERSION_FILE")"
  [[ "$version" =~ ^[0-9]+([.][0-9]+)+$ ]] || die \
    "invalid protoc version in $PROTOC_VERSION_FILE: $version"
  printf '%s' "$version"
}

resolve_protoc() {
  local explicit="${PROTOC:-}"
  local candidate=""
  local version_output=""
  local actual_version=""
  local found_versions=""
  local expected_version
  local -a candidates=()

  expected_version="$(project_protoc_version)"

  if [[ -n "$explicit" ]]; then
    if [[ "$explicit" != */* ]] && candidate="$(command -v "$explicit" 2>/dev/null)"; then
      explicit="$candidate"
    fi
    candidates+=("$explicit")
  else
    candidates+=("$HOME/.local/protoc/bin/protoc")
    if command -v protoc >/dev/null 2>&1; then
      candidates+=("$(command -v protoc)")
    fi
    candidates+=(/home/cdmonio/.local/protoc/bin/protoc)
  fi

  if ((DRY_RUN)); then
    candidate="${candidates[0]:-protoc}"
    export PROTOC="$candidate"
    printf 'Using protoc %s at %s\n' "$expected_version" "$PROTOC"
    return
  fi

  for candidate in "${candidates[@]}"; do
    [[ -x "$candidate" ]] || continue
    if ! version_output="$("$candidate" --version 2>/dev/null)"; then
      found_versions+=" $candidate=unusable"
      [[ -z "$explicit" ]] || break
      continue
    fi
    actual_version="$(awk '{print $2}' <<<"$version_output")"
    if [[ "$actual_version" == "$expected_version" ]]; then
      export PROTOC="$candidate"
      printf 'Using protoc %s at %s\n' "$expected_version" "$PROTOC"
      return
    fi
    found_versions+=" $candidate=${actual_version:-unknown}"
    [[ -z "$explicit" ]] || break
  done

  if [[ -n "$explicit" ]]; then
    die "PROTOC must resolve to protoc $expected_version; checked:${found_versions:- none}"
  fi

  die "protoc $expected_version is required to match CI; checked:${found_versions:- none}"
}

require_ref() {
  local ref="$1"
  git rev-parse --verify --quiet "$ref^{commit}" >/dev/null || die \
    "base ref $ref is unavailable; run: git fetch origin"
}

merge_base_for() {
  local base="$1"
  local merge_base=""

  require_ref "$base"
  if merge_base="$(git merge-base "$base" HEAD)" && [[ -n "$merge_base" ]]; then
    printf '%s' "$merge_base"
    return
  fi

  if [[ "$(git rev-parse --is-shallow-repository)" == "true" ]]; then
    die "no merge base for $base and HEAD; run: git fetch --unshallow origin"
  fi
  die "no merge base for $base and HEAD; run: git fetch origin"
}

require_clean_worktree() {
  local status
  ((DRY_RUN)) && return
  status="$(git status --porcelain --untracked-files=normal)"
  [[ -z "$status" ]] || {
    printf '%s\n' "$status" >&2
    die "review/full requires a clean committed HEAD; use review-uncommitted while iterating"
  }
}

run_format() {
  run_self_test
  log "Format (same commands as GitHub Actions)"
  cargo_cmd fmt --all --check
  cargo_cmd fmt --manifest-path tools/wow-test-bot/Cargo.toml -- --check
  cargo_cmd fmt --manifest-path "$HANDLER_CONTRACT_CHECK_MANIFEST" -- --check
}

run_architecture() {
  log "Architecture dependency boundaries and source hotspots"
  ((DRY_RUN)) || require_command python3
  run_cmd python3 "$ARCHITECTURE_CHECKER" check
  cargo_cmd test --locked --manifest-path "$HANDLER_CONTRACT_CHECK_MANIFEST"
  cargo_cmd run --locked --manifest-path "$HANDLER_CONTRACT_CHECK_MANIFEST" -- check
}

run_check() {
  log "Core checks and linked server builds (same commands as GitHub Actions)"
  resolve_protoc
  cargo_cmd check --locked \
    -p wow-data \
    -p wow-database \
    -p wow-network \
    -p wow-world \
    -p bnet-server \
    -p world-server
  cargo_cmd check --locked --manifest-path tools/wow-test-bot/Cargo.toml
  cargo_cmd build --locked -p bnet-server -p world-server
  cargo_cmd clippy --locked --no-deps --message-format short \
    -p wow-loot \
    -p wow-entities \
    -p wow-map \
    -p wow-network \
    --lib
  cargo_cmd clippy --locked --no-deps --message-format short \
    -p wow-world \
    --lib \
    -- \
    --cap-lints warn
}

run_test() {
  log "Focused library tests (same commands as GitHub Actions)"
  resolve_protoc
  cargo_cmd test --locked -p wow-data --lib
  cargo_cmd test --locked -p wow-handler --test inventory_registry
  cargo_cmd test --locked -p wow-packet --lib
  cargo_cmd test --locked -p wow-loot --lib
  cargo_cmd test --locked -p wow-entities --lib
  cargo_cmd test --locked -p wow-map --lib
  cargo_cmd test --locked -p wow-network --lib
  cargo_cmd test --locked -p wow-world --lib
  cargo_cmd test --locked -p wow-world --test production_handler_registry_contract
  cargo_cmd test --locked --manifest-path tools/wow-test-bot/Cargo.toml loot_race::tests
  run_capture
}

run_ci() {
  run_architecture
  run_format
  run_check
  run_test
}

run_diff() {
  local base="$1"
  local merge_base

  log "Whitespace checks against $base"
  merge_base="$(merge_base_for "$base")"
  run_cmd git diff --check "$merge_base"..HEAD
  run_cmd git diff --cached --check
  run_cmd git diff --check
}

run_quick() {
  local base="$1"
  run_diff "$base"
  run_architecture
  run_format
  run_check
}

run_capture() {
  log "Committed capture-diff regression gate"
  cargo_cmd test --locked -p capture-diff
  cargo_cmd run --locked -p capture-diff -- \
    verify-required loot-single-item-claim
}

review_result() {
  local result_file="$1"

  python3 - "$result_file" "$SCHEMA_FILE" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
schema_path = pathlib.Path(sys.argv[2])
try:
    raw = path.read_text(encoding="utf-8").strip()
except OSError as exc:
    print(f"error: cannot read Codex review result: {exc}", file=sys.stderr)
    raise SystemExit(65)

if raw.startswith("```"):
    lines = raw.splitlines()
    if len(lines) >= 3 and lines[-1].strip() == "```":
        raw = "\n".join(lines[1:-1])

try:
    result = json.loads(raw)
except json.JSONDecodeError as exc:
    print(f"error: Codex returned an invalid structured review: {exc}", file=sys.stderr)
    print(raw, file=sys.stderr)
    raise SystemExit(65)

try:
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError) as exc:
    print(f"error: cannot load Codex review schema: {exc}", file=sys.stderr)
    raise SystemExit(65)
if not isinstance(schema, dict):
    print("error: Codex review schema must be a JSON object", file=sys.stderr)
    raise SystemExit(65)


def type_matches(value, expected):
    if expected == "object":
        return isinstance(value, dict)
    if expected == "array":
        return isinstance(value, list)
    if expected == "string":
        return isinstance(value, str)
    if expected == "number":
        return isinstance(value, (int, float)) and not isinstance(value, bool)
    if expected == "integer":
        return isinstance(value, int) and not isinstance(value, bool)
    return True


def validate(value, definition, location="$"):
    errors = []
    expected_type = definition.get("type")
    if expected_type and not type_matches(value, expected_type):
        return [f"{location}: expected {expected_type}"]

    if "enum" in definition and value not in definition["enum"]:
        errors.append(f"{location}: value is not in enum")

    if isinstance(value, (int, float)) and not isinstance(value, bool):
        if "minimum" in definition and value < definition["minimum"]:
            errors.append(f"{location}: value is below minimum")
        if "maximum" in definition and value > definition["maximum"]:
            errors.append(f"{location}: value is above maximum")

    if isinstance(value, dict):
        properties = definition.get("properties", {})
        required = definition.get("required", [])
        for key in required:
            if key not in value:
                errors.append(f"{location}: missing required property {key}")
        if definition.get("additionalProperties") is False:
            for key in value.keys() - properties.keys():
                errors.append(f"{location}: unexpected property {key}")
        for key, child in value.items():
            if key in properties:
                errors.extend(validate(child, properties[key], f"{location}.{key}"))

    if isinstance(value, list) and "items" in definition:
        for index, child in enumerate(value):
            errors.extend(validate(child, definition["items"], f"{location}[{index}]"))

    return errors


schema_errors = validate(result, schema)
if schema_errors:
    print("error: Codex review result does not match the required schema", file=sys.stderr)
    for error in schema_errors[:20]:
        print(f"- {error}", file=sys.stderr)
    print(json.dumps(result, indent=2), file=sys.stderr)
    raise SystemExit(65)

findings = result.get("findings")
correctness = result.get("overall_correctness")

print(json.dumps(result, indent=2))
if findings or correctness != "patch is correct":
    print(f"LOCAL CODEX REVIEW: FINDINGS ({len(findings)})", file=sys.stderr)
    raise SystemExit(10)

print("LOCAL CODEX REVIEW: CLEAN")
PY
}

review_inspection_result() {
  local events_file="$1"
  local selector="$2"
  local expected_merge_base="${3:-}"

  python3 - "$events_file" "$selector" "$expected_merge_base" <<'PY'
import json
import pathlib
import shlex
import sys

path = pathlib.Path(sys.argv[1])
selector = sys.argv[2]
expected_merge_base = sys.argv[3]
try:
    lines = path.read_text(encoding="utf-8").splitlines()
except OSError as exc:
    print(f"error: cannot read Codex review event log: {exc}", file=sys.stderr)
    raise SystemExit(65)

completed_commands = []
for line_number, line in enumerate(lines, start=1):
    if not line.strip():
        continue
    try:
        event = json.loads(line)
    except json.JSONDecodeError as exc:
        print(
            f"error: invalid Codex review event at line {line_number}: {exc}",
            file=sys.stderr,
        )
        raise SystemExit(65)

    item = event.get("item")
    if not isinstance(item, dict):
        continue
    command = item.get("command")
    if (
        event.get("type") == "item.completed"
        and item.get("type") == "command_execution"
        and item.get("status") == "completed"
        and item.get("exit_code") == 0
        and isinstance(command, str)
        and command.strip()
    ):
        completed_commands.append(command)

if not completed_commands:
    print("error: Codex review completed without a successful command", file=sys.stderr)
    raise SystemExit(65)


def shell_payload(command):
    try:
        outer = shlex.split(command, posix=True)
    except ValueError:
        return None
    if not outer:
        return None
    executable = pathlib.Path(outer[0]).name
    if executable in {"bash", "sh"} and len(outer) >= 3 and outer[1] in {
        "-c",
        "-lc",
        "-cl",
    }:
        return outer[2]
    return command


def executed_invocation(command):
    payload = shell_payload(command)
    if payload is None:
        return None
    try:
        tokens = shlex.split(payload, posix=True)
    except ValueError:
        return None
    return tokens or None


invocations = [
    invocation
    for command in completed_commands
    if (invocation := executed_invocation(command)) is not None
]
git_invocations = [
    invocation[1:]
    for invocation in invocations
    if pathlib.Path(invocation[0]).name == "git"
]
missing = []
if selector == "--base":
    if not expected_merge_base:
        missing.append("expected merge base")
    elif ["diff", f"{expected_merge_base}..HEAD"] not in git_invocations:
        missing.append(f"exact content diff: git diff {expected_merge_base}..HEAD")
elif selector == "--uncommitted":
    if ["diff"] not in git_invocations:
        missing.append("exact unstaged content diff: git diff")
    if ["diff", "--cached"] not in git_invocations:
        missing.append("exact staged content diff: git diff --cached")
    if ["ls-files", "--others", "--exclude-standard"] not in git_invocations:
        missing.append("exact untracked-file listing")
else:
    missing.append(f"known review selector (got {selector})")

if missing:
    print(
        "error: Codex review did not complete required inspection: " + ", ".join(missing),
        file=sys.stderr,
    )
    raise SystemExit(65)

print(f"Codex review inspection commands: {len(completed_commands)} ({selector})")
PY
}

run_self_test() {
  local artifacts
  local capture_output
  local clean_result
  local cpp_capture_text
  local cpp_missing_pin_output
  local ci_dry_run_output
  local combined_inspection_result
  local committed_inspection_result
  local dependency
  local expected_protoc_version
  local echo_inspection_result
  local findings_result
  local full_dry_run_output
  local github_workflow_text
  local incomplete_inspection_result
  local inspection_result
  local invalid_result
  local loot_race_fake_bot
  local loot_race_fake_bot_sha
  local loot_race_malicious_env
  local loot_race_wrapper_args
  local loot_race_wrapper_args_file
  local loot_race_wrapper_missing_ack_output
  local loot_race_wrapper_xtrace_output
  local loot_race_wrapper_output
  local loot_race_secret_sentinel="rc106-secret-must-not-appear"
  local no_inspection_result
  local path_limited_inspection_result
  local protoc_output
  local qa_fake_bin
  local qa_fake_bot
  local qa_fake_bot_marker
  local qa_fake_bot_sha
  local qa_fake_capture
  local qa_fake_capture_publish_marker
  local qa_fake_world
  local qa_fake_world_other
  local qa_identity
  local qa_instance_port=45124
  local qa_loot_race_dry_run_output
  local qa_loot_race_missing_ack_output
  local qa_loot_race_missing_bot_pin_output
  local qa_loot_race_missing_world_pin_output
  local qa_pin_fixture
  local qa_pin_fixture_sha
  local qa_pm2_after_file
  local qa_pm2_before_file
  local qa_pm2_duplicate
  local qa_pm2_json_file
  local qa_pm2_offline
  local qa_pm2_online
  local qa_pm2_pid_changed
  local qa_pm2_restart_changed
  local qa_pm2_state_file
  local qa_pm2_wrong_path
  local qa_positive_output
  local qa_cleanup_output
  local qa_drift_output
  local qa_early_output
  local qa_early_status=0
  local qa_kill_output
  local qa_restart_output
  local qa_pid_output
  local qa_term_output
  local qa_world_pid=""
  local qa_world_port=45123
  local qa_world_sha
  local qa_target_sha
  local range_summary_inspection_result
  local rust_capture_text
  local rust_missing_pin_output
  local rust_race_missing_bot_output
  local review_dry_run_output
  local staged_alias_inspection_result
  local -a qa_common_env=()
  local rc=0

  if ((DRY_RUN)); then
    log "Preflight self-test (dry-run)"
    print_command python3 -m json.tool "$SCHEMA_FILE"
    print_command python3 "$ARCHITECTURE_CHECKER" self-test
    printf '+ validate Codex review parser exit codes 0, 10, and 65\n'
    return
  fi

  require_command python3
  [[ "$(qa_loot_capture_ready_timeout_seconds 180 30)" == 330 ]] || die \
    "loot-race pre-ready timeout does not dominate explicit wrapper budgets"
  if qa_loot_capture_ready_timeout_seconds 2 30 >/dev/null 2>&1 \
      || qa_loot_capture_ready_timeout_seconds 180 3601 >/dev/null 2>&1; then
    die "loot-race pre-ready timeout accepted an invalid inner timeout"
  fi
  project_protoc_version >/dev/null
  python3 -m json.tool "$SCHEMA_FILE" >/dev/null || die "invalid Codex review JSON schema"
  python3 "$ARCHITECTURE_CHECKER" self-test >/dev/null || die \
    "architecture policy self-test failed"
  git -C "$REPO_ROOT" check-ignore --quiet \
    tools/architecture/handler-contract-check/target/preflight-ignore-probe || die \
    "standalone handler-contract checker target directory is not gitignored"

  artifacts="$(mktemp -d "${TMPDIR:-/tmp}/rustycore-preflight-self-test.XXXXXX")"
  trap 'self_test_cleanup "${qa_world_pid:-}" "${artifacts:-}"' EXIT
  clean_result="$artifacts/clean.json"
  findings_result="$artifacts/findings.json"
  invalid_result="$artifacts/invalid.json"
  inspection_result="$artifacts/inspection.jsonl"
  combined_inspection_result="$artifacts/combined-inspection.jsonl"
  committed_inspection_result="$artifacts/committed-inspection.jsonl"
  echo_inspection_result="$artifacts/echo-inspection.jsonl"
  incomplete_inspection_result="$artifacts/incomplete-inspection.jsonl"
  no_inspection_result="$artifacts/no-inspection.jsonl"
  path_limited_inspection_result="$artifacts/path-limited-inspection.jsonl"
  range_summary_inspection_result="$artifacts/range-summary-inspection.jsonl"
  staged_alias_inspection_result="$artifacts/staged-alias-inspection.jsonl"

  printf '%s\n' \
    '{"findings":[],"overall_correctness":"patch is correct","overall_explanation":"clean","overall_confidence_score":1}' >"$clean_result"
  printf '%s\n' \
    '{"findings":[{"title":"[P2] test","body":"test","confidence_score":1,"code_location":{"absolute_file_path":"/tmp/test","line_range":{"start":1,"end":1}}}],"overall_correctness":"patch is incorrect","overall_explanation":"finding","overall_confidence_score":1}' >"$findings_result"
  printf '%s\n' '[]' >"$invalid_result"
  printf '%s\n' \
    '{"type":"item.completed","item":{"type":"command_execution","command":"git diff","status":"completed","exit_code":0}}' \
    '{"type":"item.completed","item":{"type":"command_execution","command":"git diff --cached","status":"completed","exit_code":0}}' \
    '{"type":"item.completed","item":{"type":"command_execution","command":"git ls-files --others --exclude-standard","status":"completed","exit_code":0}}' >"$inspection_result"
  printf '%s\n' \
    '{"type":"item.completed","item":{"type":"command_execution","command":"git diff && git diff --cached && git ls-files --others --exclude-standard","status":"completed","exit_code":0}}' >"$combined_inspection_result"
  printf '%s\n' \
    '{"type":"item.completed","item":{"type":"command_execution","command":"git diff deadbeef..HEAD","status":"completed","exit_code":0}}' >"$committed_inspection_result"
  printf '%s\n' \
    '{"type":"item.completed","item":{"type":"command_execution","command":"git diff --stat && git diff --cached","status":"completed","exit_code":0}}' >"$incomplete_inspection_result"
  printf '%s\n' \
    '{"type":"item.completed","item":{"type":"command_execution","command":"pwd","status":"completed","exit_code":0}}' >"$no_inspection_result"
  printf '%s\n' \
    '{"type":"item.completed","item":{"type":"command_execution","command":"echo git diff && echo git diff --cached && echo git ls-files --others --exclude-standard","status":"completed","exit_code":0}}' >"$echo_inspection_result"
  printf '%s\n' \
    '{"type":"item.completed","item":{"type":"command_execution","command":"git diff -- tools/pr-preflight.sh && git diff --cached -- tools/pr-preflight.sh && git ls-files --others --exclude-standard","status":"completed","exit_code":0}}' >"$path_limited_inspection_result"
  printf '%s\n' \
    '{"type":"item.completed","item":{"type":"command_execution","command":"git diff HEAD~1..HEAD && git diff --cached --stat && git ls-files --others --exclude-standard","status":"completed","exit_code":0}}' >"$range_summary_inspection_result"
  printf '%s\n' \
    '{"type":"item.completed","item":{"type":"command_execution","command":"git diff","status":"completed","exit_code":0}}' \
    '{"type":"item.completed","item":{"type":"command_execution","command":"git diff --staged","status":"completed","exit_code":0}}' \
    '{"type":"item.completed","item":{"type":"command_execution","command":"git ls-files --others --exclude-standard","status":"completed","exit_code":0}}' >"$staged_alias_inspection_result"

  review_result "$clean_result" >/dev/null
  review_result "$findings_result" >/dev/null 2>&1 || rc=$?
  [[ "$rc" == "10" ]] || die "review findings self-test returned $rc instead of 10"

  rc=0
  review_result "$invalid_result" >/dev/null 2>&1 || rc=$?
  [[ "$rc" == "65" ]] || die "invalid review self-test returned $rc instead of 65"

  review_inspection_result "$inspection_result" --uncommitted >/dev/null
  review_inspection_result "$committed_inspection_result" --base deadbeef >/dev/null
  rc=0
  review_inspection_result "$no_inspection_result" --uncommitted >/dev/null 2>&1 || rc=$?
  [[ "$rc" == "65" ]] || die "missing inspection self-test returned $rc instead of 65"

  rc=0
  review_inspection_result "$combined_inspection_result" --uncommitted \
    >/dev/null 2>&1 || rc=$?
  [[ "$rc" == "65" ]] || die "combined inspection self-test returned $rc instead of 65"

  rc=0
  review_inspection_result "$echo_inspection_result" --uncommitted \
    >/dev/null 2>&1 || rc=$?
  [[ "$rc" == "65" ]] || die "echo inspection self-test returned $rc instead of 65"

  rc=0
  review_inspection_result "$incomplete_inspection_result" --uncommitted \
    >/dev/null 2>&1 || rc=$?
  [[ "$rc" == "65" ]] || die "incomplete inspection self-test returned $rc instead of 65"

  rc=0
  review_inspection_result "$path_limited_inspection_result" --uncommitted \
    >/dev/null 2>&1 || rc=$?
  [[ "$rc" == "65" ]] || die "path-limited inspection self-test returned $rc instead of 65"

  rc=0
  review_inspection_result "$range_summary_inspection_result" --uncommitted \
    >/dev/null 2>&1 || rc=$?
  [[ "$rc" == "65" ]] || die "range/summary inspection self-test returned $rc instead of 65"

  rc=0
  review_inspection_result "$staged_alias_inspection_result" --uncommitted \
    >/dev/null 2>&1 || rc=$?
  [[ "$rc" == "65" ]] || die "staged alias inspection self-test returned $rc instead of 65"

  mkdir -p "$artifacts/bin"
  for dependency in awk dirname env git head realpath sed sha256sum tr; do
    ln -s "$(command -v "$dependency")" "$artifacts/bin/$dependency"
  done
  printf '#!/bin/sh\n[ "${RUST_MIN_STACK:-0}" -ge %s ] || exit 70\n[ "${CARGO_INCREMENTAL:-}" = 0 ] || exit 71\nexit 0\n' \
    "$DEFAULT_RUST_MIN_STACK" >"$artifacts/bin/cargo"
  expected_protoc_version="$(project_protoc_version)"
  printf '#!/bin/sh\nprintf "libprotoc %s\\n"\n' \
    "$expected_protoc_version" >"$artifacts/bin/protoc"
  chmod +x "$artifacts/bin/cargo" "$artifacts/bin/protoc"

  protoc_output="$(PATH="$artifacts/bin" PROTOC=protoc \
    "$BASH" "$REPO_ROOT/tools/pr-preflight.sh" check 2>&1)" || die \
    "bare PROTOC command name did not resolve through PATH"
  [[ "$protoc_output" == *"Using protoc $expected_protoc_version at $artifacts/bin/protoc"* ]] || die \
    "bare PROTOC command resolved to the wrong executable"

  capture_output="$(PATH="$artifacts/bin" PROTOC="$artifacts/missing-protoc" \
    "$BASH" "$REPO_ROOT/tools/pr-preflight.sh" capture 2>&1)" || die \
    "capture profile unexpectedly requires protoc"
  require_exact_occurrences "$capture_output" \
    "test --locked -p capture-diff" 1 \
    "capture profile capture-diff test command"
  require_exact_occurrences "$capture_output" \
    "verify-required loot-single-item-claim" 1 \
    "capture profile required-flow command"

  ci_dry_run_output="$(PATH="$artifacts/bin" \
    "$BASH" "$REPO_ROOT/tools/pr-preflight.sh" --dry-run ci 2>&1)" || die \
    "CI dry-run unexpectedly requires optional execution tools"
  require_exact_occurrences "$ci_dry_run_output" \
    "tools/architecture/check_architecture.py check" 1 \
    "local CI architecture check"
  require_exact_occurrences "$ci_dry_run_output" \
    "test --locked --manifest-path $HANDLER_CONTRACT_CHECK_MANIFEST" 1 \
    "local CI handler-contract checker tests"
  require_exact_occurrences "$ci_dry_run_output" \
    "run --locked --manifest-path $HANDLER_CONTRACT_CHECK_MANIFEST -- check" 1 \
    "local CI handler-contract repository check"
  require_exact_occurrences "$ci_dry_run_output" \
    "fmt --manifest-path $HANDLER_CONTRACT_CHECK_MANIFEST -- --check" 1 \
    "local CI handler-contract checker formatting"
  [[ "$ci_dry_run_output" == *"clippy --locked --no-deps --message-format short -p wow-loot"* ]] || die \
    "CI profile did not print the loot-authority clippy command"
  [[ "$ci_dry_run_output" == *"clippy --locked --no-deps --message-format short -p wow-world --lib -- --cap-lints warn"* ]] || die \
    "CI profile did not print the capped wow-world clippy command"
  [[ "$ci_dry_run_output" == *"test --locked -p wow-loot --lib"* ]] || die \
    "CI profile did not print the wow-loot tests"
  [[ "$ci_dry_run_output" == *"test --locked -p wow-entities --lib"* ]] || die \
    "CI profile did not print the wow-entities tests"
  [[ "$ci_dry_run_output" == *"test --locked -p wow-network --lib"* ]] || die \
    "CI profile did not print the wow-network tests"
  [[ "$ci_dry_run_output" == *"test --locked -p wow-handler --test inventory_registry"* ]] || die \
    "CI profile did not print the wow-handler inventory registry integration tests"
  [[ "$ci_dry_run_output" == *"test --locked -p wow-world --test production_handler_registry_contract"* ]] || die \
    "CI profile did not print the production-linked handler registry contract"
  [[ "$ci_dry_run_output" == *"--manifest-path tools/wow-test-bot/Cargo.toml loot_race::tests"* ]] || die \
    "CI profile did not print the focused loot-race harness tests"
  [[ "$ci_dry_run_output" == *"+ env CARGO_INCREMENTAL=0 cargo +1.88.0 test --locked -p wow-world --lib"* ]] || die \
    "local CI profile did not disable Rust 1.88 incremental compilation"
  require_exact_occurrences "$ci_dry_run_output" \
    "test --locked -p capture-diff" 1 \
    "local CI capture-diff test command"
  require_exact_occurrences "$ci_dry_run_output" \
    "verify-required loot-single-item-claim" 1 \
    "local CI required-flow command"
  [[ "$ci_dry_run_output" != *"WOW_BOT_LOOT_RACE_SMOKE=1"* ]] || die \
    "normal CI profile must never activate destructive live loot-race QA"

  github_workflow_text="$(<"$REPO_ROOT/.github/workflows/rust-ci.yml")"
  require_exact_occurrences "$github_workflow_text" \
    "python3 tools/architecture/check_architecture.py check" 1 \
    "GitHub workflow architecture check"
  require_exact_occurrences "$github_workflow_text" \
    "cargo +1.88.0 test --locked --manifest-path tools/architecture/handler-contract-check/Cargo.toml" 1 \
    "GitHub workflow handler-contract checker tests"
  require_exact_occurrences "$github_workflow_text" \
    "cargo +1.88.0 run --locked --manifest-path tools/architecture/handler-contract-check/Cargo.toml -- check" 1 \
    "GitHub workflow handler-contract repository check"
  require_exact_occurrences "$github_workflow_text" \
    "cargo +1.88.0 fmt --manifest-path tools/architecture/handler-contract-check/Cargo.toml -- --check" 1 \
    "GitHub workflow handler-contract checker formatting"
  require_exact_occurrences "$github_workflow_text" \
    'CARGO_INCREMENTAL: "0"' 2 \
    "GitHub workflow non-incremental Rust 1.88 contract"
  require_exact_occurrences "$github_workflow_text" \
    "cargo +1.88.0 test --locked -p capture-diff" 1 \
    "GitHub workflow capture-diff test command"
  require_exact_occurrences "$github_workflow_text" \
    "cargo +1.88.0 test --locked -p wow-world --test production_handler_registry_contract" 1 \
    "GitHub workflow production-linked handler registry contract"
  require_exact_occurrences "$github_workflow_text" \
    "cargo +1.88.0 test --locked -p wow-handler --test inventory_registry" 1 \
    "GitHub workflow wow-handler inventory registry integration tests"
  require_exact_occurrences "$github_workflow_text" \
    "cargo +1.88.0 run --locked -p capture-diff -- verify-required loot-single-item-claim" 1 \
    "GitHub workflow required-flow command"

  if qa_loot_race_missing_ack_output="$(PATH="$artifacts/bin" \
    "$BASH" "$REPO_ROOT/tools/pr-preflight.sh" --dry-run --allow-runtime-qa \
      qa-loot-race 2>&1)"; then
    die "qa-loot-race accepted live mutation without its destructive acknowledgement"
  fi
  [[ "$qa_loot_race_missing_ack_output" == *"--ack-disposable-overworld-loot-race"* ]] || die \
    "qa-loot-race missing-ack error did not name the required acknowledgement"

  qa_loot_race_dry_run_output="$(PATH="$artifacts/bin" \
    "$BASH" "$REPO_ROOT/tools/pr-preflight.sh" --dry-run --allow-runtime-qa \
      --ack-disposable-overworld-loot-race qa-loot-race 2>&1)" || die \
    "acknowledged qa-loot-race dry-run failed"
  [[ "$qa_loot_race_dry_run_output" == *"WOW_BOT_LOOT_RACE_SMOKE=1"* ]] || die \
    "qa-loot-race did not select the loot-race bot mode"
  [[ "$qa_loot_race_dry_run_output" != *"CARGO_INCREMENTAL=0"* ]] || die \
    "qa-loot-race must not inherit compile-only incremental settings"
  [[ "$qa_loot_race_dry_run_output" == *"WOW_BOT_EXEC="* \
    && "$qa_loot_race_dry_run_output" == *"WOW_BOT_EXEC_SHA256="* ]] || die \
    "qa-loot-race dry-run did not disclose the pinned bot provenance"
  [[ "$qa_loot_race_dry_run_output" == *"WOW_BOT_ENSURE_TEST_ACCOUNTS=0"* \
    && "$qa_loot_race_dry_run_output" == *"WOW_BOT_FIXTURE_JOURNAL="* ]] || die \
    "qa-loot-race did not disable identity bootstrap and disclose its recovery journal"
  [[ "$qa_loot_race_dry_run_output" == *"WOW_BOT_ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=1"* ]] || die \
    "qa-loot-race did not forward the destructive acknowledgement"
  [[ "$qa_loot_race_dry_run_output" == *"run_rustycore_login_smoke.sh"* ]] || die \
    "qa-loot-race did not invoke the live QA wrapper"
  [[ "$qa_loot_race_dry_run_output" == *"capture-rust.sh loot-two-session-atomic-race --yes"* \
    && "$qa_loot_race_dry_run_output" == *"RUST_CAPTURE_LOOT_FIXTURE_GUARD=1"* \
    && "$qa_loot_race_dry_run_output" == *"RUST_CAPTURE_ACK_LOOT_FIXTURE_MUTATION=1"* \
    && "$qa_loot_race_dry_run_output" == *"WOW_BOT_REPORT=/tmp/rustycore-loot-race-qa/bot-report.json"* \
    && "$qa_loot_race_dry_run_output" == *"RUST_CAPTURE_DB_CONF=/home/server/trinity-legacy-install/bin/worldserver.conf"* \
    && "$qa_loot_race_dry_run_output" == *"RUST_CAPTURE_EFFECTIVE_CONFIG=/home/server/trinity-legacy-install/etc/worldserver.conf"* \
    && "$qa_loot_race_dry_run_output" == *"wait for guarded capture READY marker"* \
    && "$qa_loot_race_dry_run_output" == *"wait for exact PM2/fixture restoration"* ]] || die \
    "qa-loot-race dry-run did not disclose its guarded capture lifecycle"
  [[ "$qa_loot_race_dry_run_output" == *"WORLD_HOST=127.0.0.1"* \
    && "$qa_loot_race_dry_run_output" == *"WORLD_PORT=8085"* \
    && "$qa_loot_race_dry_run_output" == *"INSTANCE_HOST=127.0.0.1"* \
    && "$qa_loot_race_dry_run_output" == *"INSTANCE_PORT=8086"* ]] || die \
    "qa-loot-race dry-run did not pin the bot to the accredited local listeners"
  [[ "$qa_loot_race_dry_run_output" == *"BNET_HOST=127.0.0.1"* \
    && "$qa_loot_race_dry_run_output" == *"BNET_PORT=8081"* \
    && "$qa_loot_race_dry_run_output" == *"WOW_BOT_LOOT_RACE_ACCOUNT_A=TESTBOT2@bot.local"* \
    && "$qa_loot_race_dry_run_output" == *"WOW_BOT_LOOT_RACE_ACCOUNT_B=TESTBOT3@bot.local"* \
    && "$qa_loot_race_dry_run_output" == *"WOW_BOT_LOOT_RACE_GAMEOBJECT_ENTRY=2846"* \
    && "$qa_loot_race_dry_run_output" == *"WOW_BOT_LOOT_RACE_GAMEOBJECT_SPAWN_GUID=9106001"* \
    && "$qa_loot_race_dry_run_output" == *"WOW_BOT_LOOT_RACE_ITEM_ENTRY=38"* ]] || die \
    "qa-loot-race dry-run did not pin BNet and the exact disposable fixture"

  if qa_loot_race_missing_world_pin_output="$(PATH="$artifacts/bin" \
    "$BASH" "$REPO_ROOT/tools/pr-preflight.sh" --allow-runtime-qa \
      --ack-disposable-overworld-loot-race qa-loot-race 2>&1)"; then
    die "qa-loot-race accepted an unpinned running world server"
  fi
  [[ "$qa_loot_race_missing_world_pin_output" == *"WOW_BOT_WORLD_EXEC"* ]] || die \
    "qa-loot-race missing-world-pin error did not name WOW_BOT_WORLD_EXEC"

  qa_pin_fixture="$artifacts/bin/cargo"
  qa_pin_fixture_sha="$(sha256sum "$qa_pin_fixture" | awk '{print $1}')"
  if qa_loot_race_missing_bot_pin_output="$(PATH="$artifacts/bin" \
    WOW_BOT_WORLD_EXEC="$qa_pin_fixture" \
    WOW_BOT_WORLD_EXEC_SHA256="$qa_pin_fixture_sha" \
    "$BASH" "$REPO_ROOT/tools/pr-preflight.sh" --allow-runtime-qa \
      --ack-disposable-overworld-loot-race qa-loot-race 2>&1)"; then
    die "qa-loot-race accepted an unpinned bot executable"
  fi
  [[ "$qa_loot_race_missing_bot_pin_output" == *"WOW_BOT_EXEC"* ]] || die \
    "qa-loot-race missing-bot-pin error did not name WOW_BOT_EXEC"

  if loot_race_wrapper_missing_ack_output="$(PATH="$artifacts/bin" \
    WOW_BOT_PASSWORD="$loot_race_secret_sentinel" WOW_BOT_GENERATE_LOCAL_PASSWORD=0 \
    WOW_BOT_LOOT_RACE_SMOKE=1 \
    WOW_BOT_ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=0 \
    WOW_BOT_ENV_FILE=/dev/null \
    "$BASH" "$REPO_ROOT/tools/wow-test-bot/run_rustycore_login_smoke.sh" 2>&1)"; then
    die "loot-race wrapper accepted destructive mode without acknowledgement"
  fi
  [[ "$loot_race_wrapper_missing_ack_output" == *"WOW_BOT_ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=1"* ]] || die \
    "loot-race wrapper missing-ack error did not name the required acknowledgement"
  [[ "$loot_race_wrapper_missing_ack_output" != *"$loot_race_secret_sentinel"* ]] || die \
    "loot-race wrapper exposed a caller secret while loading defaults"
  loot_race_wrapper_xtrace_output="$(PATH="$artifacts/bin" \
    WOW_BOT_PASSWORD="$loot_race_secret_sentinel" WOW_BOT_GENERATE_LOCAL_PASSWORD=0 \
    WOW_BOT_LOOT_RACE_SMOKE=1 \
    WOW_BOT_ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=0 \
    WOW_BOT_ENV_FILE=/dev/null \
    "$BASH" -x "$REPO_ROOT/tools/wow-test-bot/run_rustycore_login_smoke.sh" 2>&1 || true)"
  [[ "$loot_race_wrapper_xtrace_output" != *"$loot_race_secret_sentinel"* ]] || die \
    "loot-race wrapper exposed a caller secret when invoked with bash -x"

  loot_race_fake_bot="$artifacts/fake-loot-race-bot"
  loot_race_malicious_env="$artifacts/malicious.env.local"
  loot_race_wrapper_args_file="$artifacts/loot-race-wrapper-args"
  printf '#!/bin/sh\nprintf "%%s\\n" "$@" >"$WOW_BOT_SELF_TEST_ARGS"\n' >"$loot_race_fake_bot"
  printf '%s\n' \
    'set -x' \
    'WOW_BOT_LOOT_RACE_SMOKE=0' \
    'WOW_BOT_ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=0' \
    'WOW_BOT_EXEC=/tmp/not-the-pinned-bot' \
    'WOW_BOT_EXEC_SHA256=0000000000000000000000000000000000000000000000000000000000000000' \
    >"$loot_race_malicious_env"
  printf 'WOW_BOT_SELF_TEST_ARGS=%q\nWOW_BOT_REPORT=%q\nWOW_BOT_LOG=%q\n' \
    "$artifacts/not-the-pinned-args" \
    "$artifacts/not-the-pinned-report" \
    "$artifacts/not-the-pinned-log" >>"$loot_race_malicious_env"
  chmod +x "$loot_race_fake_bot"
  loot_race_fake_bot_sha="$(sha256sum "$loot_race_fake_bot" | awk '{print $1}')"
  loot_race_wrapper_output="$(PATH="$artifacts/bin" \
    WOW_BOT_PASSWORD="$loot_race_secret_sentinel" WOW_BOT_GENERATE_LOCAL_PASSWORD=0 \
    WOW_BOT_ENSURE_TEST_ACCOUNTS=1 WOW_BOT_LOOT_RACE_SMOKE=1 \
    WOW_BOT_ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=1 \
    WOW_BOT_LOOT_RACE_ACCOUNT_A=WRONG1@bot.local \
    WOW_BOT_LOOT_RACE_ACCOUNT_B=WRONG2@bot.local \
    WOW_BOT_LOOT_RACE_GAMEOBJECT_ENTRY=9999 \
    WOW_BOT_LOOT_RACE_GAMEOBJECT_SPAWN_GUID=9998 \
    WOW_BOT_LOOT_RACE_RUNTIME_COUNTER=77 \
    WOW_BOT_LOOT_RACE_ITEM_ENTRY=9997 \
    WOW_BOT_FIXTURE_JOURNAL="$artifacts/standalone-loot-race.journal" \
    WOW_BOT_ENV_FILE="$loot_race_malicious_env" \
    WOW_BOT_EXEC="$loot_race_fake_bot" WOW_BOT_EXEC_SHA256="$loot_race_fake_bot_sha" \
    WOW_BOT_REPORT="$artifacts/loot-race-report.json" \
    WOW_BOT_LOG="$artifacts/loot-race.log" \
    WOW_BOT_SELF_TEST_ARGS="$loot_race_wrapper_args_file" \
    "$BASH" "$REPO_ROOT/tools/wow-test-bot/run_rustycore_login_smoke.sh" 2>&1)" || die \
    "acknowledged loot-race wrapper self-test failed"
  [[ "$loot_race_wrapper_output" != *"$loot_race_secret_sentinel"* ]] || die \
    "loot-race wrapper exposed a caller secret while execing the bot"
  loot_race_wrapper_args="$(<"$loot_race_wrapper_args_file")"
  [[ "$loot_race_wrapper_args" == *"--loot-race-smoke"* ]] || die \
    "loot-race wrapper did not pass its bot mode"
  [[ "$loot_race_wrapper_args" == *"--ack-disposable-overworld-loot-race"* ]] || die \
    "loot-race wrapper did not translate its acknowledgement to the CLI guard"
  [[ "$loot_race_wrapper_args" == *$'--loot-race-gameobject-entry\n2846\n'* \
    && "$loot_race_wrapper_args" == *$'--loot-race-gameobject-spawn-guid\n9106001\n'* \
    && "$loot_race_wrapper_args" == *$'--loot-race-item-entry\n38\n'* ]] || die \
    "loot-race wrapper did not pin the Tattered Chest 2846/9106001/item-38 defaults"
  [[ "$loot_race_wrapper_args" == *$'--loot-race-account-a\nTESTBOT2@bot.local\n'* \
    && "$loot_race_wrapper_args" == *$'--loot-race-account-b\nTESTBOT3@bot.local\n'* \
    && "$loot_race_wrapper_args" == *$'--loot-race-runtime-counter\n0\n'* ]] || die \
    "loot-race wrapper allowed hostile environment overrides of its disposable identities/runtime counter"
  [[ "$loot_race_wrapper_args" != *"--single"* ]] || die \
    "loot-race wrapper incorrectly reduced the two-session flow to one account"
  [[ "$loot_race_wrapper_args" != *"--ensure-test-accounts"* ]] || die \
    "loot-race wrapper did not force the destructive identity bootstrap off"
  [[ -f "$artifacts/loot-race.log" && ! -e "$artifacts/not-the-pinned-args" ]] || die \
    "loot-race wrapper allowed .env.local to replace caller-pinned QA inputs"
  [[ "$loot_race_wrapper_output" != *"$loot_race_secret_sentinel"* ]] || die \
    "loot-race wrapper exposed a caller secret through env-file xtrace"

  require_command cp
  require_command jq
  require_command rg
  require_command sleep
  (
    # Exercise the SQL/journal guard shared by capture-cpp.sh and
    # capture-rust.sh without touching a real database or service.
    # shellcheck source=crates/capture-diff/scripts/loot-fixture-common.sh
    source "$REPO_ROOT/crates/capture-diff/scripts/loot-fixture-common.sh"
    fixture_guard_dir="$artifacts/common-fixture-guard"
    fixture_health_state="$fixture_guard_dir/health"
    mkdir -m 700 "$fixture_guard_dir"
    WOW_BOT_FIXTURE_JOURNAL="$fixture_guard_dir/fixture.journal"
    LOOT_FIXTURE_CLEANUP_MARKER=""
    LOOT_FIXTURE_GUARD_ENABLED=1
    LOOT_FIXTURE_ENTRY=21779
    LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER=1
    LOOT_FIXTURE_TEMP_HEALTH_MODIFIER=0.0001
    LOOT_FIXTURE_SNAPSHOT_READY=0

    validate_fresh_loot_fixture_journal
    : >"$WOW_BOT_FIXTURE_JOURNAL"
    if validate_fresh_loot_fixture_journal >/dev/null 2>&1; then
      exit 80
    fi
    rm -f "$WOW_BOT_FIXTURE_JOURNAL"
    chmod 755 "$fixture_guard_dir"
    if validate_fresh_loot_fixture_journal >/dev/null 2>&1; then
      exit 146
    fi
    chmod 700 "$fixture_guard_dir"
    validate_fresh_loot_fixture_journal
    loot_fixture_bot_cleanup_safe_for_capture_state 0
    if loot_fixture_bot_cleanup_safe_for_capture_state 2 >/dev/null 2>&1; then
      exit 147
    fi
    ln -s "$fixture_guard_dir/missing-journal" "$WOW_BOT_FIXTURE_JOURNAL"
    if loot_fixture_bot_cleanup_safe_for_capture_state 0 >/dev/null 2>&1; then
      exit 148
    fi
    rm -f "$WOW_BOT_FIXTURE_JOURNAL"
    : >"$WOW_BOT_FIXTURE_JOURNAL"
    if loot_fixture_bot_cleanup_safe_for_capture_state 0 >/dev/null 2>&1; then
      exit 152
    fi
    rm -f "$WOW_BOT_FIXTURE_JOURNAL"
    ln -s "$fixture_guard_dir/missing-marker" "$LOOT_FIXTURE_CLEANUP_MARKER"
    if loot_fixture_bot_cleanup_safe_for_capture_state 0 >/dev/null 2>&1; then
      exit 149
    fi
    rm -f "$LOOT_FIXTURE_CLEANUP_MARKER"

    printf '%s\n' \
      '{"version":1,"journal_sha256":"0000000000000000000000000000000000000000000000000000000000000000","cleanup_pid":123}' \
      >"$LOOT_FIXTURE_CLEANUP_MARKER"
    chmod 600 "$LOOT_FIXTURE_CLEANUP_MARKER"
    if loot_fixture_bot_cleanup_safe_for_capture_state 0 >/dev/null 2>&1; then
      exit 150
    fi
    loot_fixture_bot_cleanup_safe_for_capture_state 1
    loot_fixture_bot_cleanup_complete
    chmod 644 "$LOOT_FIXTURE_CLEANUP_MARKER"
    if loot_fixture_bot_cleanup_complete >/dev/null 2>&1; then
      exit 81
    fi
    chmod 600 "$LOOT_FIXTURE_CLEANUP_MARKER"
    : >"$WOW_BOT_FIXTURE_JOURNAL"
    if loot_fixture_bot_cleanup_complete >/dev/null 2>&1; then
      exit 82
    fi
    rm -f "$WOW_BOT_FIXTURE_JOURNAL" "$LOOT_FIXTURE_CLEANUP_MARKER"
    if loot_fixture_bot_cleanup_safe_for_capture_state 1 >/dev/null 2>&1; then
      exit 151
    fi
    LOOT_FIXTURE_GUARD_ENABLED=0
    loot_fixture_bot_cleanup_safe_for_capture_state invalid
    LOOT_FIXTURE_GUARD_ENABLED=1

    printf '%s\n' 1 >"$fixture_health_state"
    loot_fixture_character_mysql() {
      printf '%s\n' 0
    }
    loot_fixture_world_mysql() {
      local query="${2:-}"
      local state
      state="$(<"$fixture_health_state")"
      if [[ "$query" == *"UPDATE creature_template_difficulty"* \
          && "$query" == *"SET HealthModifier = 0.0001"* ]]; then
        if [[ "$state" == 1 ]]; then
          printf '%s\n' 0.0001 >"$fixture_health_state"
          printf '%s\n' 1
        else
          printf '%s\n' 0
        fi
      elif [[ "$query" == *"UPDATE creature_template_difficulty"* \
          && "$query" == *"SET HealthModifier = 1"* ]]; then
        if [[ "$state" == 0.0001 ]]; then
          printf '%s\n' 1 >"$fixture_health_state"
          printf '%s\n' 1
        else
          printf '%s\n' 0
        fi
      elif [[ "$query" == *"SELECT COUNT(*)"* \
          && "$query" == *"HealthModifier - 0.0001"* ]]; then
        [[ "$state" == 0.0001 ]] && printf '%s\n' 1 || printf '%s\n' 0
      elif [[ "$query" == *"SELECT COUNT(*)"* \
          && "$query" == *"HealthModifier - 1"* ]]; then
        [[ "$state" == 1 ]] && printf '%s\n' 1 || printf '%s\n' 0
      else
        return 83
      fi
    }
    loot_fixture_wait_until_all_characters_offline
    apply_creature_health_fixture_guard >/dev/null
    [[ "$(<"$fixture_health_state")" == 0.0001 \
      && "$LOOT_FIXTURE_SNAPSHOT_READY" == 1 ]] || exit 84
    restore_creature_health_fixture_guard >/dev/null
    [[ "$(<"$fixture_health_state")" == 1 \
      && "$LOOT_FIXTURE_SNAPSHOT_READY" == 0 ]] || exit 85

    printf '%s\n' 2 >"$fixture_health_state"
    LOOT_FIXTURE_SNAPSHOT_READY=1
    if restore_creature_health_fixture_guard >/dev/null 2>&1; then
      exit 86
    fi
    [[ "$(<"$fixture_health_state")" == 2 ]] || exit 87
  ) || die "shared C++/Rust loot-fixture guard self-test failed"

  (
    # Exercise the exact shared-chest cleanup implementation without sourcing
    # capture-rust.sh's executable entrypoint or touching a real database.
    capture_rust_script="$REPO_ROOT/crates/capture-diff/scripts/capture-rust.sh"
    shared_chest_functions="$artifacts/shared-chest-restore-functions.sh"
    shared_chest_state_dir="$artifacts/shared-chest-restore-state"
    mkdir -m 700 "$shared_chest_state_dir"

    extract_capture_function() {
      local function_name="$1"
      awk -v signature="${function_name}() {" '
        $0 == signature { copying = 1 }
        copying { print }
        copying && $0 == "}" { found = 1; exit }
        END { if (!found) exit 1 }
      ' "$capture_rust_script"
    }

    : >"$shared_chest_functions"
    for function_name in \
      restore_loot_fixture_guard \
      shared_chest_spawn_exact_count \
      shared_chest_spawn_cleanup_counts \
      shared_chest_addon_exact_count \
      shared_chest_spawn_metadata_counts \
      restore_shared_chest_fixture_guard; do
      extract_capture_function "$function_name" \
        >>"$shared_chest_functions" || exit 157
    done
    # shellcheck disable=SC1090
    source "$shared_chest_functions"

    LOOT_FIXTURE_KIND=shared-chest
    LOOT_FIXTURE_CHEST_GUID=9106001
    LOOT_FIXTURE_CHEST_TEMPLATE_ENTRY=2846
    LOOT_FIXTURE_CHEST_ADDON_FACTION=101
    shared_chest_spawn_state="$shared_chest_state_dir/spawn"
    shared_chest_addon_state="$shared_chest_state_dir/addon"
    shared_chest_respawn_state="$shared_chest_state_dir/respawn"
    shared_chest_metadata_state="$shared_chest_state_dir/metadata"
    shared_chest_write_log="$shared_chest_state_dir/writes"
    shared_chest_failure_file="$shared_chest_state_dir/failure"

    reset_shared_chest_restore_state() {
      local spawn="$1"
      local addon="$2"
      local respawn="$3"
      local metadata="$4"
      printf '%s\n' "$spawn" >"$shared_chest_spawn_state"
      printf '%s\n' "$addon" >"$shared_chest_addon_state"
      printf '%s\n' "$respawn" >"$shared_chest_respawn_state"
      printf '%s\n' "$metadata" >"$shared_chest_metadata_state"
      : >"$shared_chest_write_log"
      : >"$shared_chest_failure_file"
      LOOT_FIXTURE_CHEST_RESPAWN_DELETE_READY=1
      LOOT_FIXTURE_CHEST_SPAWN_DELETE_READY=1
      LOOT_FIXTURE_CHEST_ADDON_RESTORE_READY=1
    }

    loot_fixture_world_mysql() {
      local query="${2:-}"
      local expected state
      if [[ "$query" == *"DELETE FROM gameobject"* ]]; then
        printf '%s\n' delete-spawn >>"$shared_chest_write_log"
        for expected in \
          "WHERE guid = 9106001" \
          "AND id = 2846" \
          "AND map = 0" \
          "AND zoneId = 0" \
          "AND areaId = 0" \
          "AND spawnDifficulties = '0'" \
          "AND phaseUseFlags = 0" \
          "AND PhaseId = 0" \
          "AND PhaseGroup = 0" \
          "AND terrainSwapMap = -1" \
          "AND position_x = CAST(-8946.95 AS FLOAT)" \
          "AND position_y = CAST(-132.493 AS FLOAT)" \
          "AND position_z = CAST(83.5312 AS FLOAT)" \
          "AND orientation = 0" \
          "AND rotation0 = 0" \
          "AND rotation1 = 0" \
          "AND rotation2 = 0" \
          "AND rotation3 = 0" \
          "AND spawntimesecs = 300" \
          "AND animprogress = 255" \
          "AND state = 1" \
          "AND ScriptName = ''" \
          "AND StringId IS NULL" \
          "AND VerifiedBuild = 0"; do
          [[ "$query" == *"$expected"* ]] || return 169
        done
        state="$(<"$shared_chest_spawn_state")"
        if [[ "$state" == exact || "$state" == exact-verify-fails ]]; then
          if [[ "$state" == exact-verify-fails ]]; then
            printf '%s\n' absent-fail-once >"$shared_chest_spawn_state"
          else
            printf '%s\n' absent >"$shared_chest_spawn_state"
          fi
          printf '%s\n' 1
        else
          printf '%s\n' 0
        fi
      elif [[ "$query" == *"UPDATE gameobject_template_addon"* ]]; then
        printf '%s\n' restore-addon >>"$shared_chest_write_log"
        for expected in \
          "SET mingold = 0, maxgold = 0" \
          "WHERE entry = 2846" \
          "AND faction = 101" \
          "AND flags = 0" \
          "AND mingold = 10" \
          "AND maxgold = 10" \
          "AND artkit0 = 0" \
          "AND artkit1 = 0" \
          "AND artkit2 = 0" \
          "AND artkit3 = 0" \
          "AND artkit4 = 0" \
          "AND WorldEffectID = 0" \
          "AND AIAnimKitID = 0"; do
          [[ "$query" == *"$expected"* ]] || return 170
        done
        state="$(<"$shared_chest_addon_state")"
        if [[ "$state" == 10 ]]; then
          printf '%s\n' 0 >"$shared_chest_addon_state"
          printf '%s\n' 1
        else
          printf '%s\n' 0
        fi
      elif [[ "$query" == *"COALESCE(SUM("* \
          && "$query" == *"FROM gameobject"* ]]; then
        for expected in \
          "id = 2846" \
          "AND map = 0" \
          "AND zoneId = 0" \
          "AND areaId = 0" \
          "AND spawnDifficulties = '0'" \
          "AND phaseUseFlags = 0" \
          "AND PhaseId = 0" \
          "AND PhaseGroup = 0" \
          "AND terrainSwapMap = -1" \
          "AND position_x = CAST(-8946.95 AS FLOAT)" \
          "AND position_y = CAST(-132.493 AS FLOAT)" \
          "AND position_z = CAST(83.5312 AS FLOAT)" \
          "AND orientation = 0" \
          "AND rotation0 = 0" \
          "AND rotation1 = 0" \
          "AND rotation2 = 0" \
          "AND rotation3 = 0" \
          "AND spawntimesecs = 300" \
          "AND animprogress = 255" \
          "AND state = 1" \
          "AND ScriptName = ''" \
          "AND StringId IS NULL" \
          "AND VerifiedBuild = 0" \
          "FROM pool_members
         WHERE type = 1 AND spawnId = 9106001" \
          "FROM game_event_gameobject
         WHERE guid = 9106001" \
          "FROM linked_respawn
         WHERE guid = 9106001
            OR linkedGuid = 9106001" \
          "FROM gameobject_addon
         WHERE guid = 9106001" \
          "FROM gameobject_overrides
         WHERE spawnId = 9106001" \
          "FROM spawn_group
         WHERE spawnType = 1 AND spawnId = 9106001" \
          "FROM gameobject
     WHERE guid = 9106001"; do
          [[ "$query" == *"$expected"* ]] || return 179
        done
        state="$(<"$shared_chest_spawn_state")"
        case "$state" in
          exact|exact-verify-fails)
            printf '1\t1\t%s\n' "$(<"$shared_chest_metadata_state")"
            ;;
          absent)
            printf '0\t0\t%s\n' "$(<"$shared_chest_metadata_state")"
            ;;
          absent-fail-once)
            printf '%s\n' absent >"$shared_chest_spawn_state"
            return 171
            ;;
          *)
            printf '1\t0\t%s\n' "$(<"$shared_chest_metadata_state")"
            ;;
        esac
      elif [[ "$query" == *"FROM pool_members"* ]]; then
        for expected in \
          "FROM pool_members
         WHERE type = 1 AND spawnId = 9106001" \
          "FROM game_event_gameobject
         WHERE guid = 9106001" \
          "FROM linked_respawn
         WHERE guid = 9106001
            OR linkedGuid = 9106001" \
          "FROM gameobject_addon
         WHERE guid = 9106001" \
          "FROM gameobject_overrides
         WHERE spawnId = 9106001" \
          "FROM spawn_group
         WHERE spawnType = 1 AND spawnId = 9106001"; do
          [[ "$query" == *"$expected"* ]] || return 179
        done
        cat "$shared_chest_metadata_state"
      elif [[ "$query" == *"FROM gameobject_template_addon"* ]]; then
        for expected in \
          "WHERE entry = 2846" \
          "AND faction = 101" \
          "AND flags = 0" \
          "AND artkit0 = 0" \
          "AND artkit1 = 0" \
          "AND artkit2 = 0" \
          "AND artkit3 = 0" \
          "AND artkit4 = 0" \
          "AND WorldEffectID = 0" \
          "AND AIAnimKitID = 0"; do
          [[ "$query" == *"$expected"* ]] || return 180
        done
        state="$(<"$shared_chest_addon_state")"
        if [[ "$state" == 0 \
            && "$query" == *"mingold = 0"* \
            && "$query" == *"maxgold = 0"* ]] \
            || [[ "$state" == 10 \
              && "$query" == *"mingold = 10"* \
              && "$query" == *"maxgold = 10"* ]]; then
          printf '%s\n' 1
        else
          printf '%s\n' 0
        fi
      elif [[ "$query" == *"SELECT COUNT(*) FROM gameobject"* \
          && "$query" == *"AND id ="* ]]; then
        state="$(<"$shared_chest_spawn_state")"
        [[ "$state" == exact || "$state" == exact-verify-fails ]] \
          && printf '%s\n' 1 || printf '%s\n' 0
      elif [[ "$query" == *"SELECT COUNT(*) FROM gameobject"* ]]; then
        state="$(<"$shared_chest_spawn_state")"
        case "$state" in
          absent) printf '%s\n' 0 ;;
          absent-fail-once)
            printf '%s\n' absent >"$shared_chest_spawn_state"
            return 171
            ;;
          *) printf '%s\n' 1 ;;
        esac
      else
        return 158
      fi
    }

    loot_fixture_character_mysql() {
      local query="${2:-}"
      local state
      state="$(<"$shared_chest_respawn_state")"
      if [[ "$query" == *"DELETE FROM respawn"* ]]; then
        printf '%s\n' delete-respawn >>"$shared_chest_write_log"
        if [[ "$state" == generated \
            && "$query" == *"WHERE type = 1"* \
            && "$query" == *"spawnId = 9106001"* \
            && "$query" == *"respawnTime = 123456"* \
            && "$query" == *"mapId = 0"* \
            && "$query" == *"instanceId = 0"* ]]; then
          printf '%s\n' none >"$shared_chest_respawn_state"
          printf '%s\n' 1
        else
          printf '%s\n' 0
        fi
      elif [[ "$query" == *"SELECT respawnTime FROM respawn"* ]]; then
        [[ "$state" == generated \
          && "$query" == *"WHERE type = 1"* \
          && "$query" == *"spawnId = 9106001"* \
          && "$query" == *"mapId = 0"* \
          && "$query" == *"instanceId = 0"* ]] \
          && printf '%s\n' 123456
      elif [[ "$query" == *"SELECT COUNT(*) FROM respawn"* ]]; then
        [[ "$query" == *"WHERE type = 1"* \
          && "$query" == *"spawnId = 9106001"* ]] || return 172
        case "$state" in
          none) printf '%s\n' 0 ;;
          generated) printf '%s\n' 1 ;;
          drift) printf '%s\n' 2 ;;
          *) return 159 ;;
        esac
      else
        return 160
      fi
    }

    reset_shared_chest_restore_state exact 10 none $'0\t0\t0\t0\t0\t0'
    restore_loot_fixture_guard >/dev/null
    [[ "$(<"$shared_chest_spawn_state")" == absent \
      && "$(<"$shared_chest_addon_state")" == 0 \
      && "$(<"$shared_chest_respawn_state")" == none \
      && "$(<"$shared_chest_write_log")" == $'delete-spawn\nrestore-addon' \
      && "$LOOT_FIXTURE_CHEST_RESPAWN_DELETE_READY" == 0 \
      && "$LOOT_FIXTURE_CHEST_SPAWN_DELETE_READY" == 0 \
      && "$LOOT_FIXTURE_CHEST_ADDON_RESTORE_READY" == 0 ]] || exit 161

    reset_shared_chest_restore_state exact 10 generated $'0\t0\t0\t0\t0\t0'
    restore_loot_fixture_guard >/dev/null
    [[ "$(<"$shared_chest_spawn_state")" == absent \
      && "$(<"$shared_chest_addon_state")" == 0 \
      && "$(<"$shared_chest_respawn_state")" == none \
      && "$(<"$shared_chest_write_log")" == $'delete-respawn\ndelete-spawn\nrestore-addon' \
      && "$LOOT_FIXTURE_CHEST_RESPAWN_DELETE_READY" == 0 \
      && "$LOOT_FIXTURE_CHEST_SPAWN_DELETE_READY" == 0 \
      && "$LOOT_FIXTURE_CHEST_ADDON_RESTORE_READY" == 0 ]] || exit 162

    # The spawn flag is armed before INSERT. Absence is already the exact
    # postcondition and must not block restoration of the addon mutation.
    reset_shared_chest_restore_state absent 10 none $'0\t0\t0\t0\t0\t0'
    restore_loot_fixture_guard >/dev/null
    [[ "$(<"$shared_chest_spawn_state")" == absent \
      && "$(<"$shared_chest_addon_state")" == 0 \
      && "$(<"$shared_chest_write_log")" == restore-addon \
      && "$LOOT_FIXTURE_CHEST_RESPAWN_DELETE_READY" == 0 \
      && "$LOOT_FIXTURE_CHEST_SPAWN_DELETE_READY" == 0 \
      && "$LOOT_FIXTURE_CHEST_ADDON_RESTORE_READY" == 0 ]] || exit 173

    # Simulate DELETE committing while its immediate COUNT verification fails.
    # A fresh final reconciliation must observe absence and disarm cleanup.
    reset_shared_chest_restore_state exact-verify-fails 10 none $'0\t0\t0\t0\t0\t0'
    restore_loot_fixture_guard >/dev/null 2>&1
    [[ "$(<"$shared_chest_spawn_state")" == absent \
      && "$(<"$shared_chest_addon_state")" == 0 \
      && "$(<"$shared_chest_write_log")" == $'delete-spawn\nrestore-addon' \
      && "$LOOT_FIXTURE_CHEST_RESPAWN_DELETE_READY" == 0 \
      && "$LOOT_FIXTURE_CHEST_SPAWN_DELETE_READY" == 0 \
      && "$LOOT_FIXTURE_CHEST_ADDON_RESTORE_READY" == 0 ]] || exit 174

    reset_shared_chest_restore_state exact 10 none $'1\t0\t0\t0\t0\t0'
    if restore_loot_fixture_guard > /dev/null 2>"$shared_chest_failure_file"; then
      exit 163
    fi
    shared_chest_failure="$(<"$shared_chest_failure_file")"
    [[ ! -s "$shared_chest_write_log" \
      && "$(<"$shared_chest_spawn_state")" == exact \
      && "$shared_chest_failure" == *"spawn/state/metadata drifted"* ]] || exit 164

    reset_shared_chest_restore_state exact drift none $'0\t0\t0\t0\t0\t0'
    LOOT_FIXTURE_CHEST_RESPAWN_DELETE_READY=0
    LOOT_FIXTURE_CHEST_SPAWN_DELETE_READY=0
    if restore_loot_fixture_guard > /dev/null 2>"$shared_chest_failure_file"; then
      exit 165
    fi
    shared_chest_failure="$(<"$shared_chest_failure_file")"
    [[ ! -s "$shared_chest_write_log" \
      && "$(<"$shared_chest_addon_state")" == drift \
      && "$shared_chest_failure" == *"template addon drifted"* ]] || exit 166

    reset_shared_chest_restore_state absent 0 drift $'0\t0\t0\t0\t0\t0'
    LOOT_FIXTURE_CHEST_SPAWN_DELETE_READY=0
    LOOT_FIXTURE_CHEST_ADDON_RESTORE_READY=0
    if restore_loot_fixture_guard > /dev/null 2>"$shared_chest_failure_file"; then
      exit 167
    fi
    shared_chest_failure="$(<"$shared_chest_failure_file")"
    [[ ! -s "$shared_chest_write_log" \
      && "$(<"$shared_chest_respawn_state")" == drift \
      && "$LOOT_FIXTURE_CHEST_RESPAWN_DELETE_READY" == 1 \
      && "$shared_chest_failure" == *"unexpected respawn ownership"* ]] || exit 168
  ) || die "shared-chest exact restore self-test failed"

  (
    # Prove both supported topologies: a direct Rust PM2 entry/listener and a
    # non-exec C++ shell wrapper with one descendant owning both listeners.
    # Reject changed ancestry, changed/foreign listeners, duplicate PM2
    # identity, and incomplete stopped state before DB restoration.
    # shellcheck source=crates/capture-diff/scripts/capture-service-common.sh
    unset CAPTURE_WORLD_STOP_TIMEOUT_SECONDS CAPTURE_WORLD_READY_TIMEOUT_SECONDS
    source "$REPO_ROOT/crates/capture-diff/scripts/capture-service-common.sh"
    [[ "$CAPTURE_WORLD_STOP_TIMEOUT_SECONDS" == 30 \
      && "$CAPTURE_WORLD_READY_TIMEOUT_SECONDS" == 180 ]] || exit 153
    capture_validate_world_timeouts
    CAPTURE_WORLD_STOP_TIMEOUT_SECONDS=0
    if capture_validate_world_timeouts >/dev/null 2>&1; then
      exit 154
    fi
    CAPTURE_WORLD_STOP_TIMEOUT_SECONDS=30
    CAPTURE_WORLD_READY_TIMEOUT_SECONDS=2
    if capture_validate_world_timeouts >/dev/null 2>&1; then
      exit 156
    fi
    CAPTURE_WORLD_READY_TIMEOUT_SECONDS=3601
    if capture_validate_world_timeouts >/dev/null 2>&1; then
      exit 155
    fi
    CAPTURE_WORLD_READY_TIMEOUT_SECONDS=180
    capture_validate_world_timeouts
    capture_fixture_cleanup_verified_for_publication 0 0 || exit 175
    capture_fixture_cleanup_verified_for_publication 0 1 || exit 176
    capture_fixture_cleanup_verified_for_publication 1 1 || exit 177
    if capture_fixture_cleanup_verified_for_publication 1 0 \
        || capture_fixture_cleanup_verified_for_publication invalid 1 \
        || capture_fixture_cleanup_verified_for_publication 1 invalid; then
      exit 178
    fi
    CAPTURE_WORLD_PORT=45123
    CAPTURE_INSTANCE_PORT=45124
    CAPTURE_PROC_ROOT="$artifacts/fake-capture-proc"
    mkdir -p "$CAPTURE_PROC_ROOT/42" "$CAPTURE_PROC_ROOT/43" "$CAPTURE_PROC_ROOT/44"
    printf 'PPid:\t1\n' >"$CAPTURE_PROC_ROOT/42/status"
    printf 'PPid:\t42\n' >"$CAPTURE_PROC_ROOT/43/status"
    printf 'PPid:\t43\n' >"$CAPTURE_PROC_ROOT/44/status"
    for pid_and_start in 42:42000 43:43000 44:44000; do
      fake_pid="${pid_and_start%%:*}"
      fake_start="${pid_and_start#*:}"
      {
        printf '%s (fake world process) S' "$fake_pid"
        for _ in $(seq 4 21); do printf ' 0'; done
        printf ' %s 0\n' "$fake_start"
      } >"$CAPTURE_PROC_ROOT/$fake_pid/stat"
    done
    [ "$(capture_pid_starttime 42)" = 42000 ] \
      && [ "$(capture_process_tree_identity 42)" \
        = $'42:42000\n43:43000\n44:44000' ] || exit 143
    service_state="$artifacts/cpp-service-state.json"
    service_world_listener=1
    service_instance_listener=1
    service_foreign_listener=0
    service_foreign_same_row=0
    service_listener_pid=44
    service_parent_alive=1
    service_listener_alive=1
    pm2() {
      [ "${1:-}" = jlist ] || return 90
      cat "$service_state"
    }
    ss() {
      case "$*" in
        *"sport = :$CAPTURE_WORLD_PORT"*)
          if ((service_world_listener && service_foreign_same_row)); then
            printf 'LISTEN 0 128 127.0.0.1:%s 0.0.0.0:* users:((world,pid=%s,fd=3),(other,pid=99,fd=5))\n' "$CAPTURE_WORLD_PORT" "$service_listener_pid"
          elif ((service_world_listener)); then
            printf 'LISTEN 0 128 127.0.0.1:%s 0.0.0.0:* users:((world,pid=%s,fd=3))\n' "$CAPTURE_WORLD_PORT" "$service_listener_pid"
          fi
          ((service_foreign_listener)) \
            && printf 'LISTEN 0 128 127.0.0.1:%s 0.0.0.0:* users:((other,pid=99,fd=5))\n' "$CAPTURE_WORLD_PORT"
          ;;
        *"sport = :$CAPTURE_INSTANCE_PORT"*)
          ((service_instance_listener)) \
            && printf 'LISTEN 0 128 127.0.0.1:%s 0.0.0.0:* users:((world,pid=%s,fd=4))\n' "$CAPTURE_INSTANCE_PORT" "$service_listener_pid"
          ;;
        *)
          ((service_world_listener)) \
            && printf 'LISTEN 0 128 127.0.0.1:%s 0.0.0.0:*\n' "$CAPTURE_WORLD_PORT"
          ((service_instance_listener)) \
            && printf 'LISTEN 0 128 127.0.0.1:%s 0.0.0.0:*\n' "$CAPTURE_INSTANCE_PORT"
          ;;
      esac
      return 0
    }
    kill() {
      [ "${1:-}" = -0 ] || return 1
      case "${3:-${2:-}}" in
        42) ((service_parent_alive)) ;;
        44) ((service_listener_alive)) ;;
        *) return 1 ;;
      esac
    }

    printf '%s\n' \
      '[{"name":"cpp-world","pid":42,"pm2_env":{"status":"online"}}]' \
      >"$service_state"
    [[ "$(capture_world_ready_once cpp-world)" == $'42\t44' ]] || exit 91
    # A direct binary is also valid: PM2 entry PID and listener PID coincide.
    service_listener_pid=42
    [[ "$(capture_world_ready_once cpp-world)" == $'42\t42' ]] || exit 110
    service_listener_pid=44
    # Breaking the wrapper -> listener ancestry must invalidate the identity.
    printf 'PPid:\t99\n' >"$CAPTURE_PROC_ROOT/43/status"
    if capture_world_ready_once cpp-world >/dev/null 2>&1; then
      exit 111
    fi
    printf 'PPid:\t42\n' >"$CAPTURE_PROC_ROOT/43/status"
    printf '%s\n' \
      '[{"name":"cpp-world","pid":45,"pm2_env":{"status":"online"}}]' \
      >"$service_state"
    if capture_world_ready_once cpp-world >/dev/null 2>&1; then
      exit 112
    fi
    printf '%s\n' \
      '[{"name":"cpp-world","pid":42,"pm2_env":{"status":"online"}}]' \
      >"$service_state"
    service_foreign_listener=1
    if capture_world_ready_once cpp-world >/dev/null 2>&1; then
      exit 97
    fi
    service_foreign_listener=0
    service_foreign_same_row=1
    if capture_world_ready_once cpp-world >/dev/null 2>&1; then
      exit 98
    fi
    service_foreign_same_row=0
    service_instance_listener=0
    if capture_world_ready_once cpp-world >/dev/null 2>&1; then
      exit 92
    fi
    service_instance_listener=1
    printf '%s\n' \
      '[{"name":"cpp-world","pid":42,"pm2_env":{"status":"online"}},{"name":"cpp-world","pid":43,"pm2_env":{"status":"online"}}]' \
      >"$service_state"
    if capture_world_ready_once cpp-world >/dev/null 2>&1; then
      exit 93
    fi

    printf '%s\n' \
      '[{"name":"cpp-world","pid":0,"pm2_env":{"status":"launching"}}]' \
      >"$service_state"
    service_world_listener=0
    service_instance_listener=0
    service_parent_alive=0
    service_listener_alive=0
    if capture_world_stopped_once cpp-world $'42\t44'; then
      exit 99
    fi

    printf '%s\n' \
      '[{"name":"cpp-world","pid":0,"pm2_env":{"status":"stopped"}}]' \
      >"$service_state"
    service_world_listener=0
    service_instance_listener=0
    service_parent_alive=0
    service_listener_alive=0
    capture_world_stopped_once cpp-world $'42\t44' || exit 94
    service_parent_alive=1
    if capture_world_stopped_once cpp-world $'42\t44'; then
      exit 95
    fi
    service_parent_alive=0
    service_listener_alive=1
    if capture_world_stopped_once cpp-world $'42\t44'; then
      exit 113
    fi
    service_listener_alive=0
    service_world_listener=1
    if capture_world_stopped_once cpp-world $'42\t44'; then
      exit 96
    fi
  ) || die "C++ PM2/PID/listener fail-closed self-test failed"

  (
    # PM2 entrypoint bytes and Git worktree state are provenance, not comments.
    # A changed wrapper, dirty harness, or newly dirty source must change/fail
    # the recorded identity before any service mutation.
    # shellcheck source=crates/capture-diff/scripts/capture-service-common.sh
    source "$REPO_ROOT/crates/capture-diff/scripts/capture-service-common.sh"
    entrypoint="$artifacts/non-exec-world-wrapper.sh"
    entry_state="$artifacts/entrypoint-pm2.json"
    printf '#!/bin/sh\n./world-server\n' >"$entrypoint"
    chmod 700 "$entrypoint"
    pm2() {
      [ "${1:-}" = jlist ] || return 114
      cat "$entry_state"
    }
    jq -n --arg path "$entrypoint" \
      '[{name:"world",pid:42,pm2_env:{status:"online",restart_time:7,pm_exec_path:$path}}]' \
      >"$entry_state"
    entry_before="$(capture_pm2_entrypoint_identity world 42)" || exit 115
    printf '#!/bin/sh\n./different-world-server\n' >"$entrypoint"
    entry_after="$(capture_pm2_entrypoint_identity world 42)" || exit 116
    [ "$entry_before" != "$entry_after" ] || exit 117

    harness_repo="$artifacts/clean-harness-repo"
    source_repo="$artifacts/dirty-source-repo"
    for repository in "$harness_repo" "$source_repo"; do
      git init -q "$repository"
      git -C "$repository" config user.name preflight
      git -C "$repository" config user.email preflight@example.invalid
      printf 'committed\n' >"$repository/tracked.txt"
      git -C "$repository" add tracked.txt
      git -C "$repository" commit -qm initial
    done
    harness_head="$(git -C "$harness_repo" rev-parse HEAD)"
    source_head="$(git -C "$source_repo" rev-parse HEAD)"
    capture_git_repo_clean_at_head "$harness_repo" "$harness_head" || exit 118
    capture_git_repo_clean_at_head "$source_repo" "$source_head" || exit 124
    harness_clean_digest="$(capture_git_worktree_state_sha256 "$harness_repo")" \
      || exit 119
    source_clean_digest="$(capture_git_worktree_state_sha256 "$source_repo")" \
      || exit 125
    printf 'dirty\n' >>"$harness_repo/tracked.txt"
    if capture_git_repo_clean_at_head "$harness_repo" "$harness_head"; then
      exit 120
    fi
    [ "$(capture_git_worktree_state_sha256 "$harness_repo")" \
      != "$harness_clean_digest" ] || exit 121
    printf 'untracked source state\n' >"$source_repo/local.patch"
    capture_git_repo_is_dirty "$source_repo" || exit 122
    [ "$(capture_git_worktree_state_sha256 "$source_repo")" \
      != "$source_clean_digest" ] || exit 123
  ) || die "capture entrypoint/worktree provenance self-test failed"

  (
    # Credential changes must not become dictionary-testable config hashes,
    # while a capture-relevant non-secret change must still alter the digest.
    # shellcheck source=crates/capture-diff/scripts/capture-service-common.sh
    source "$REPO_ROOT/crates/capture-diff/scripts/capture-service-common.sh"
    conf_a="$artifacts/redacted-config-a.conf"
    conf_b="$artifacts/redacted-config-b.conf"
    conf_c="$artifacts/redacted-config-c.conf"
    printf '%s\n' \
      'WorldServerPort = 8085' \
      'WorldDatabaseInfo = "mysql://user:first-secret@localhost/world"' >"$conf_a"
    printf '%s\n' \
      'WorldServerPort = 8085' \
      'WorldDatabaseInfo = "mysql://user:second-secret@localhost/world"' >"$conf_b"
    printf '%s\n' \
      'WorldServerPort = 9085' \
      'WorldDatabaseInfo = "mysql://user:first-secret@localhost/world"' >"$conf_c"
    hash_a="$(capture_effective_config_redacted_sha256 \
      "$conf_a" 'capture.packet_dump=enabled' WorldServerPort WorldDatabaseInfo)" || exit 105
    hash_b="$(capture_effective_config_redacted_sha256 \
      "$conf_b" 'capture.packet_dump=enabled' WorldServerPort WorldDatabaseInfo)" || exit 106
    hash_c="$(capture_effective_config_redacted_sha256 \
      "$conf_c" 'capture.packet_dump=enabled' WorldServerPort WorldDatabaseInfo)" || exit 107
    [ "$hash_a" = "$hash_b" ] || exit 108
    [ "$hash_a" != "$hash_c" ] || exit 109
  ) || die "redacted effective-config hash self-test failed"

  (
    # A race dump may publish only when a regular report from the exact pinned
    # bot proves the complete two-session result. This is intentionally
    # independent of the outer preflight's own report check.
    # shellcheck source=crates/capture-diff/scripts/capture-service-common.sh
    source "$REPO_ROOT/crates/capture-diff/scripts/capture-service-common.sh"
    race_bot="$artifacts/race-evidence-bot"
    race_report="$artifacts/race-evidence-valid.json"
    race_invalid="$artifacts/race-evidence-invalid.json"
    race_split_counter="$artifacts/race-evidence-split-counter.json"
    race_report_link="$artifacts/race-evidence-link.json"
    printf '#!/bin/sh\nexit 0\n' >"$race_bot"
    chmod 700 "$race_bot"
    jq -n '
      def result($account; $account_id; $character_guid; $item_push; $money): {
        account: $account,
        account_id: $account_id,
        character_guid: $character_guid,
        world_auth: true,
        enum_characters: true,
        player_login_verified: true,
        loot_race_smoke: true,
        loot_race_smoke_passed: true,
        loot_race_failure: null,
        loot_race_target_entry: 2846,
        loot_race_target_spawn_guid: 9106001,
        loot_race_target_runtime_counter: 40,
        loot_race_party_confirmed: true,
        loot_race_target_discovered: true,
        loot_race_loot_opened: true,
        loot_race_loot_list_id: 0,
        loot_race_loot_coins: 10,
        loot_race_item_push_seen: $item_push,
        loot_race_loot_removed_seen: true,
        loot_race_money_notify_amount: $money,
        loot_race_coin_removed_seen: true,
        loot_race_db_item_total: 1,
        loot_race_db_money_delta: 10,
        loot_race_relog_verified: true
      };
      {
        loot_race_smoke: true,
        loot_item_capture: false,
        results: [
          result("TESTBOT2@bot.local"; 9; 15; true; 10),
          result("TESTBOT3@bot.local"; 10; 16; false; 0)
        ]
      }
    ' >"$race_report"
    race_bot_sha="$(capture_sha256_of_file "$race_bot")" || exit 179
    race_report_sha="$(capture_sha256_of_file "$race_report")" || exit 180
    race_evidence="$(capture_loot_race_bot_evidence \
      "$race_report" "$race_bot" "$race_bot_sha")" || exit 181
    [ "$race_evidence" \
      = "$race_bot"$'\t'"$race_bot_sha"$'\t'"$race_report"$'\t'"$race_report_sha" ] \
      || exit 182
    race_manifest_evidence="$(capture_bot_manifest_evidence \
      loot-two-session-atomic-race "$race_bot" "$race_bot_sha" \
      /target/captures/loot-two-session-atomic-race/rust/race.bot-report.json \
      "$race_report_sha")" || exit 186
    jq -e \
      --arg bot "$race_bot" \
      --arg bot_sha "$race_bot_sha" \
      --arg report_sha "$race_report_sha" '
        .fixture_guard != null
        and .fixture_guard.enabled == true
        and .fixture_guard.contract == "loot-two-session-atomic-race-fixture-v1"
        and .fixture_guard.account == "TESTBOT2@bot.local"
        and .fixture_guard.peer_account == "TESTBOT3@bot.local"
        and .fixture_guard.gameobject_entry == 2846
        and .fixture_guard.gameobject_spawn_guid == 9106001
        and .fixture_guard.item_entry == 38
        and .fixture_guard.cleanup_verified == true
        and .bot_report != null
        and .bot_report.contract
          == "wow-test-bot-loot-two-session-atomic-race-report-v1"
        and .bot_report.exec_path == $bot
        and .bot_report.exec_sha256 == $bot_sha
        and .bot_report.report_path
          == "/target/captures/loot-two-session-atomic-race/rust/race.bot-report.json"
        and .bot_report.report_sha256 == $report_sha
        and .bot_report.report_validated == true
      ' <<<"$race_manifest_evidence" >/dev/null || exit 187
    jq -e '.fixture_guard == null and .bot_report == null' \
      <<<"$(capture_bot_manifest_evidence login '' '' '' '')" >/dev/null \
      || exit 188

    jq '.results[0].loot_race_smoke_passed = false
      | .results[0].loot_race_failure = "failure=17"' \
      "$race_report" >"$race_invalid"
    if capture_loot_race_bot_evidence \
        "$race_invalid" "$race_bot" "$race_bot_sha" >/dev/null; then
      exit 183
    fi
    jq '.results[1].loot_race_target_runtime_counter = 41' \
      "$race_report" >"$race_split_counter"
    if capture_loot_race_bot_evidence \
        "$race_split_counter" "$race_bot" "$race_bot_sha" >/dev/null; then
      exit 184
    fi
    ln -s "$race_report" "$race_report_link"
    if capture_loot_race_bot_evidence \
        "$race_report_link" "$race_bot" "$race_bot_sha" >/dev/null \
        || capture_loot_race_bot_evidence \
          "$race_report" "$race_bot" "$(printf '0%.0s' {1..64})" >/dev/null; then
      exit 185
    fi
  ) || die "two-session race publication evidence self-test failed"

  (
    # C++ provenance must be derived from the PM2 profile/entrypoint, and a
    # profile selecting config B must never accredit caller-declared config A.
    # Environment values are secret and intentionally do not affect the
    # stable profile hash; argv/config selection does.
    # shellcheck source=crates/capture-diff/scripts/capture-service-common.sh
    source "$REPO_ROOT/crates/capture-diff/scripts/capture-service-common.sh"
    pm2_state="$artifacts/effective-config-pm2.json"
    pm2_wrapper="$artifacts/effective-config-wrapper.sh"
    config_a="$artifacts/effective-config-a.conf"
    config_b="$artifacts/effective-config-b.conf"
    printf 'WorldServerPort = 45123\n' >"$config_a"
    printf 'WorldServerPort = 45125\n' >"$config_b"
    printf '#!/bin/sh\nexec /opt/fake/worldserver -c %q\n' "$config_a" \
      >"$pm2_wrapper"
    chmod 700 "$pm2_wrapper"
    pm2() {
      [ "${1:-}" = jlist ] || return 126
      cat "$pm2_state"
    }
    jq -n --arg wrapper "$pm2_wrapper" --arg cwd "$artifacts" \
      '[{name:"cpp-world",pid:0,pm2_env:{status:"stopped",pm_exec_path:$wrapper,pm_cwd:$cwd,args:[],env:{DB_PASSWORD:"first-secret",VISIBLE:"one"}}}]' \
      >"$pm2_state"
    [ "$(capture_pm2_effective_config_path cpp-world)" = "$config_a" ] \
      || exit 127
    profile_a="$(capture_pm2_profile_redacted_sha256 cpp-world)" || exit 128
    jq '.[0].pm2_env.env.DB_PASSWORD = "second-secret"' \
      "$pm2_state" >"$pm2_state.next"
    mv -- "$pm2_state.next" "$pm2_state"
    profile_secret_changed="$(capture_pm2_profile_redacted_sha256 cpp-world)" \
      || exit 129
    [ "$profile_a" = "$profile_secret_changed" ] || exit 130
    jq --arg config "$config_b" \
      '.[0].pm2_env.args = ["-c", $config]' \
      "$pm2_state" >"$pm2_state.next"
    mv -- "$pm2_state.next" "$pm2_state"
    [ "$(capture_pm2_effective_config_path cpp-world)" = "$config_b" ] \
      || exit 131
    [ "$(capture_pm2_effective_config_path cpp-world)" != "$config_a" ] \
      || exit 132
    profile_b="$(capture_pm2_profile_redacted_sha256 cpp-world)" || exit 133
    [ "$profile_a" != "$profile_b" ] || exit 134
    jq '.[0].pm2_env.args = ["-c"]' \
      "$pm2_state" >"$pm2_state.next"
    mv -- "$pm2_state.next" "$pm2_state"
    if capture_pm2_effective_config_path cpp-world >/dev/null 2>&1; then
      exit 144
    fi
    jq --arg a "$config_a" --arg b "$config_b" \
      '.[0].pm2_env.args = ["-c", $a, "--config", $b]' \
      "$pm2_state" >"$pm2_state.next"
    mv -- "$pm2_state.next" "$pm2_state"
    if capture_pm2_effective_config_path cpp-world >/dev/null 2>&1; then
      exit 145
    fi
  ) || die "PM2 effective-config/profile A/B self-test failed"

  (
    # `bash -x` must never reveal DatabaseInfo credentials or MYSQL_PWD.
    # Use a shell-local fake mysql command, so this test cannot reach a DB.
    # shellcheck source=crates/capture-diff/scripts/loot-fixture-common.sh
    source "$REPO_ROOT/crates/capture-diff/scripts/loot-fixture-common.sh"
    xtrace_conf="$artifacts/xtrace-worldserver.conf"
    xtrace_log="$artifacts/xtrace-credentials.log"
    world_secret="rc106-world-secret-must-not-appear"
    character_secret="rc106-character-secret-must-not-appear"
    printf '%s\n' \
      "WorldDatabaseInfo = \"127.0.0.1;3306;world-user;${world_secret};world\"" \
      "CharacterDatabaseInfo = \"127.0.0.1;3306;character-user;${character_secret};characters\"" \
      >"$xtrace_conf"
    LOOT_FIXTURE_DB_CONF="$xtrace_conf"
    mysql() {
      case " $* " in
        *" world "*) [ "${MYSQL_PWD:-}" = "$world_secret" ] || return 135 ;;
        *" characters "*) [ "${MYSQL_PWD:-}" = "$character_secret" ] || return 136 ;;
        *) return 137 ;;
      esac
      printf '0\n'
    }
    {
      set -x
      load_loot_fixture_database_credentials
      loot_fixture_world_mysql -e 'SELECT 0' >/dev/null
      loot_fixture_character_mysql -e 'SELECT 0' >/dev/null
      set +x
    } 2>"$xtrace_log" || exit 138
    if rg -q "${world_secret}|${character_secret}|MYSQL_PWD=.*secret" \
        "$xtrace_log"; then
      exit 139
    fi
  ) || die "credential/MYSQL_PWD xtrace redaction self-test failed"

  (
    # Two wrappers that target the same host ports must never pass their
    # service/SQL preconditions concurrently.
    # shellcheck source=crates/capture-diff/scripts/capture-service-common.sh
    source "$REPO_ROOT/crates/capture-diff/scripts/capture-service-common.sh"
    lock_file="$artifacts/capture-orchestration.lock"
    ready_file="$artifacts/capture-orchestration.ready"
    (
      capture_acquire_orchestration_lock "$lock_file" || exit 100
      : >"$ready_file"
      sleep 1
      capture_release_orchestration_lock
    ) &
    holder_pid=$!
    for _ in $(seq 1 40); do
      [ -e "$ready_file" ] && break
      sleep 0.025
    done
    [ -e "$ready_file" ] || exit 101
    if capture_acquire_orchestration_lock "$lock_file"; then
      capture_release_orchestration_lock
      exit 102
    fi
    wait "$holder_pid" || exit 103
    capture_acquire_orchestration_lock "$lock_file" || exit 104
    capture_release_orchestration_lock
    [ -d "$lock_file" ] && [ ! -L "$lock_file" ] \
      && [ "$(stat -c '%a' -- "$lock_file")" = 700 ] \
      && [ "$(stat -c '%u' -- "$lock_file")" = "$(id -u)" ] || exit 140
    chmod 755 "$lock_file"
    if capture_acquire_orchestration_lock "$lock_file"; then
      capture_release_orchestration_lock
      exit 141
    fi
    chmod 700 "$lock_file"
    lock_symlink="$artifacts/capture-orchestration-symlink"
    ln -s "$lock_file" "$lock_symlink"
    if capture_acquire_orchestration_lock "$lock_symlink"; then
      capture_release_orchestration_lock
      exit 142
    fi
  ) || die "capture orchestration lock self-test failed"

  cpp_capture_text="$(<"$REPO_ROOT/crates/capture-diff/scripts/capture-cpp.sh")"
  [[ "$cpp_capture_text" == *"loot-fixture-common.sh"* \
    && "$cpp_capture_text" == *"capture-service-common.sh"* \
    && "$cpp_capture_text" == *"capture_wait_for_world_stopped"* \
    && "$cpp_capture_text" == *"apply_creature_health_fixture_guard"* \
    && "$cpp_capture_text" == *"capture_validate_world_timeouts"* \
    && "$cpp_capture_text" == *"CPP_CAPTURE_BOT_READY=1"* \
    && "$cpp_capture_text" == *"loot_fixture_bot_cleanup_safe_for_capture_state"* ]] || die \
    "C++ capture wrapper is not wired to the shared fail-closed loot guard"
  [[ "$cpp_capture_text" == *"CPP_CAPTURE_EXEC_SHA256"* \
    && "$cpp_capture_text" == *"cpp.capture-manifest.json"* \
    && "$cpp_capture_text" == *"OUT_PKT_STAGE"* \
    && "$cpp_capture_text" == *"capture_acquire_orchestration_lock"* \
    && "$cpp_capture_text" == *"cpp_capture_executable_unchanged"* \
    && "$cpp_capture_text" == *"source_repo_head"* \
    && "$cpp_capture_text" == *"effective_config_redacted_sha256"* \
    && "$cpp_capture_text" == *"pm2_entry_pid"* \
    && "$cpp_capture_text" == *"pm2_exec_sha256"* \
    && "$cpp_capture_text" == *"source_worktree_state_sha256"* ]] || die \
    "C++ capture wrapper lacks executable provenance, lock, or atomic manifest publication"
  if cpp_missing_pin_output="$(
    CPP_CAPTURE_LOOT_FIXTURE_GUARD=1 \
      CPP_CAPTURE_ACK_LOOT_FIXTURE_MUTATION=1 \
      "$BASH" "$REPO_ROOT/crates/capture-diff/scripts/capture-cpp.sh" \
        loot-single-item-claim --yes 2>&1
  )"; then
    die "required C++ loot capture accepted missing executable path/SHA pin"
  fi
  [[ "$cpp_missing_pin_output" == *"requires CPP_CAPTURE_EXEC and CPP_CAPTURE_EXEC_SHA256"* ]] || die \
    "required C++ loot capture missing-pin error was not explicit"

  rust_capture_text="$(<"$REPO_ROOT/crates/capture-diff/scripts/capture-rust.sh")"
  [[ "$rust_capture_text" == *"guarded Rust evidence requires RUST_CAPTURE_EXEC"* \
    && "$rust_capture_text" == *"rust.capture-manifest.json"* \
    && "$rust_capture_text" == *"DUMP_STAGE_DIR"* \
    && "$rust_capture_text" == *"capture_process_tree_identity"* \
    && "$rust_capture_text" == *"capture_terminate_process_tree"* \
    && "$rust_capture_text" == *"capture_process_tree_absent"* \
    && "$rust_capture_text" == *"capture_publish_noreplace"* \
    && "$rust_capture_text" == *"pm2_entry_starttime"* \
    && "$rust_capture_text" == *"pm2_profile_redacted_sha256"* \
    && "$rust_capture_text" == *"capture_pm2_process_stopped"* \
    && "$rust_capture_text" == *"capture_validate_world_timeouts"* \
    && "$rust_capture_text" == *'CAPTURE_STABLE_SAMPLES" -ge 4'* \
    && "$rust_capture_text" == *"CAPTURE_BOT_READY=1"* \
    && "$rust_capture_text" == *"capture_loot_race_bot_evidence"* \
    && "$rust_capture_text" == *"loot_fixture_bot_cleanup_safe_for_capture_state"* \
    && "$rust_capture_text" == *"source_repo_head"* \
    && "$rust_capture_text" == *"effective_config_redacted_sha256"* \
    && "$rust_capture_text" == *"pm2_entry_pid"* \
    && "$rust_capture_text" == *"pm2_exec_sha256"* \
    && "$rust_capture_text" == *"harness_worktree_state_sha256"* \
    && "$rust_capture_text" != *'rm -rf -- "$DUMP_DIR"'* ]] || die \
    "Rust capture wrapper lacks guarded provenance, PID death, C++ stop, or atomic manifest checks"
  if rust_missing_pin_output="$(
    RUST_CAPTURE_LOOT_FIXTURE_GUARD=1 \
      RUST_CAPTURE_ACK_LOOT_FIXTURE_MUTATION=1 \
      "$BASH" "$REPO_ROOT/crates/capture-diff/scripts/capture-rust.sh" \
        loot-single-item-claim --yes 2>&1
  )"; then
    die "required Rust loot capture accepted missing executable path/SHA pin"
  fi
  [[ "$rust_missing_pin_output" == *"requires RUST_CAPTURE_EXEC and RUST_CAPTURE_EXEC_SHA256"* ]] || die \
    "required Rust loot capture missing-pin error was not explicit"
  if rust_race_missing_bot_output="$(
    RUST_CAPTURE_LOOT_FIXTURE_GUARD=1 \
      RUST_CAPTURE_ACK_LOOT_FIXTURE_MUTATION=1 \
      RUST_CAPTURE_EXEC=/not-used-before-bot-validation \
      RUST_CAPTURE_EXEC_SHA256="$(printf '0%.0s' {1..64})" \
      RUST_CAPTURE_EFFECTIVE_CONFIG=/dev/null \
      "$BASH" "$REPO_ROOT/crates/capture-diff/scripts/capture-rust.sh" \
        loot-two-session-atomic-race --yes 2>&1
  )"; then
    die "guarded Rust race capture accepted missing pinned bot report evidence"
  fi
  [[ "$rust_race_missing_bot_output" \
    == *"requires WOW_BOT_EXEC, WOW_BOT_EXEC_SHA256, and WOW_BOT_REPORT"* ]] || die \
    "guarded Rust race capture missing-bot-report error was not explicit"
  qa_fake_bin="$artifacts/qa-bin"
  qa_fake_world="$artifacts/fake-world-server"
  qa_fake_world_other="$artifacts/not-the-live-world-server"
  qa_fake_bot="$artifacts/fake-loot-race-bot-only"
  qa_fake_capture="$artifacts/fake-capture-rust.sh"
  qa_fake_bot_marker="$artifacts/fake-loot-race-bot-ran"
  qa_pm2_json_file="$artifacts/fake-pm2.json"
  qa_pm2_before_file="$artifacts/fake-pm2-before.json"
  qa_pm2_after_file="$artifacts/fake-pm2-after.json"
  qa_pm2_state_file="$artifacts/fake-pm2-state"
  mkdir -p "$qa_fake_bin"
  for dependency in awk bash chmod dirname env git jq mkfifo mktemp mv realpath rg rm rmdir sed seq sha256sum sleep stat; do
    ln -s "$(command -v "$dependency")" "$qa_fake_bin/$dependency"
  done
  printf '#!/bin/sh\nexit 0\n' >"$qa_fake_bin/mysql"
  printf '%s\n' \
    '#!/bin/sh' \
    'set -eu' \
    '[ "${1:-}" = jlist ] || exit 64' \
    'json_file="${QA_FAKE_PM2_JSON_FILE:?}"' \
    'if [ -n "${QA_FAKE_PM2_STATE_FILE:-}" ]; then' \
    '  if [ -e "$QA_FAKE_PM2_STATE_FILE" ]; then' \
    '    json_file="${QA_FAKE_PM2_AFTER_FILE:?}"' \
    '  else' \
    '    : >"$QA_FAKE_PM2_STATE_FILE"' \
    '    json_file="${QA_FAKE_PM2_BEFORE_FILE:?}"' \
    '  fi' \
    'fi' \
    'IFS= read -r payload <"$json_file"' \
    'printf "%s\n" "$payload"' \
    >"$qa_fake_bin/pm2"
  printf '%s\n' \
    '#!/bin/sh' \
    'set -eu' \
    'listener_pid=""' \
    'if [ "${QA_FAKE_SS_DYNAMIC:-0}" = 1 ]; then' \
    '  listener_pid="$(jq -r ".[0].pid // empty" "${QA_FAKE_PM2_JSON_FILE:?}")"' \
    'else' \
    '  case "$*" in' \
    '    *":${QA_FAKE_WORLD_PORT:?}"*) listener_pid="${QA_FAKE_SS_WORLD_PID:-}" ;;' \
    '    *":${QA_FAKE_INSTANCE_PORT:?}"*) listener_pid="${QA_FAKE_SS_INSTANCE_PID:-}" ;;' \
    '  esac' \
    'fi' \
    'if [ -n "$listener_pid" ] && [ -n "${QA_FAKE_SS_EXTRA_PID:-}" ]; then' \
    '  printf "LISTEN 0 128 127.0.0.1:* 0.0.0.0:* users:((fake-world,pid=%s,fd=3),(foreign,pid=%s,fd=4))\n" "$listener_pid" "$QA_FAKE_SS_EXTRA_PID"' \
    'elif [ -n "$listener_pid" ]; then' \
    '  printf "LISTEN 0 128 127.0.0.1:* 0.0.0.0:* users:((fake-world,pid=%s,fd=3))\n" "$listener_pid"' \
    'fi' \
    >"$qa_fake_bin/ss"
  printf '%s\n' \
    '#!/bin/sh' \
    'set -eu' \
    '[ "${WOW_BOT_ENSURE_TEST_ACCOUNTS:-}" = 0 ]' \
    'journal="${WOW_BOT_FIXTURE_JOURNAL:?}"' \
    'umask 077' \
    'write_marker() {' \
    '  printf "%s\n" "{\"version\":1,\"journal_sha256\":\"0000000000000000000000000000000000000000000000000000000000000000\",\"cleanup_pid\":123}" >"${journal}.cleanup-complete"' \
    '  chmod 600 "${journal}.cleanup-complete"' \
    '}' \
    ': >"$journal"' \
    'report=""' \
    'while [ "$#" -gt 0 ]; do' \
    '  if [ "$1" = --report ]; then' \
    '    shift' \
    '    report="${1:?missing report path}"' \
    '  fi' \
    '  shift' \
    'done' \
    '[ -n "$report" ] || exit 65' \
    'case "${QA_FAKE_BOT_MUTATION:-}" in' \
    '  restart) jq -c ".[0].pm2_env.restart_time += 1" "${QA_FAKE_PM2_JSON_FILE:?}" >"${QA_FAKE_PM2_JSON_FILE}.tmp" ;;' \
    '  pid) jq -c --argjson pid "${QA_FAKE_ORIGINAL_PID:?}" ".[0].pid = \$pid" "${QA_FAKE_PM2_JSON_FILE:?}" >"${QA_FAKE_PM2_JSON_FILE}.tmp" ;;' \
    '  term) kill -TERM "$$" ;;' \
    '  kill) kill -KILL "$$" ;;' \
    '  pending) exit 72 ;;' \
    '  drift) write_marker ;;' \
    'esac' \
    'if [ -e "${QA_FAKE_PM2_JSON_FILE:-}.tmp" ]; then mv "${QA_FAKE_PM2_JSON_FILE}.tmp" "$QA_FAKE_PM2_JSON_FILE"; fi' \
    'if [ "${QA_FAKE_BOT_REPORT_MODE:-full}" = summary ]; then' \
    '  printf "%s\n" '\''{"loot_race_smoke":true,"loot_item_capture":false,"results":[{"account":"TESTBOT2@bot.local","account_id":9,"character_guid":15,"world_auth":true,"enum_characters":true,"player_login_verified":true,"loot_race_smoke":true,"loot_race_smoke_passed":true,"loot_race_failure":null,"loot_race_party_confirmed":true,"loot_race_target_discovered":true,"loot_race_loot_opened":true,"loot_race_relog_verified":true},{"account":"TESTBOT3@bot.local","account_id":10,"character_guid":16,"world_auth":true,"enum_characters":true,"player_login_verified":true,"loot_race_smoke":true,"loot_race_smoke_passed":true,"loot_race_failure":null,"loot_race_party_confirmed":true,"loot_race_target_discovered":true,"loot_race_loot_opened":true,"loot_race_relog_verified":true}]}'\'' >"$report"' \
    'else' \
    '  printf "%s\n" '\''{"loot_race_smoke":true,"loot_item_capture":false,"results":[{"account":"TESTBOT2@bot.local","account_id":9,"character_guid":15,"world_auth":true,"enum_characters":true,"player_login_verified":true,"loot_race_smoke":true,"loot_race_smoke_passed":true,"loot_race_failure":null,"loot_race_target_entry":2846,"loot_race_target_spawn_guid":9106001,"loot_race_target_runtime_counter":12345,"loot_race_party_confirmed":true,"loot_race_target_discovered":true,"loot_race_loot_opened":true,"loot_race_loot_list_id":0,"loot_race_loot_coins":10,"loot_race_item_push_seen":true,"loot_race_loot_removed_seen":true,"loot_race_money_notify_amount":10,"loot_race_coin_removed_seen":true,"loot_race_db_item_total":1,"loot_race_db_money_delta":10,"loot_race_relog_verified":true},{"account":"TESTBOT3@bot.local","account_id":10,"character_guid":16,"world_auth":true,"enum_characters":true,"player_login_verified":true,"loot_race_smoke":true,"loot_race_smoke_passed":true,"loot_race_failure":null,"loot_race_target_entry":2846,"loot_race_target_spawn_guid":9106001,"loot_race_target_runtime_counter":12345,"loot_race_party_confirmed":true,"loot_race_target_discovered":true,"loot_race_loot_opened":true,"loot_race_loot_list_id":0,"loot_race_loot_coins":10,"loot_race_item_push_seen":false,"loot_race_loot_removed_seen":true,"loot_race_money_notify_amount":0,"loot_race_coin_removed_seen":true,"loot_race_db_item_total":1,"loot_race_db_money_delta":10,"loot_race_relog_verified":true}]}'\'' >"$report"' \
    'fi' \
    'printf "%s\n" fake-bot-only >"${QA_FAKE_BOT_MARKER:?}"' \
    'if [ "${QA_FAKE_BOT_MUTATION:-}" != drift ]; then' \
    '  rm -f -- "$journal"' \
    '  write_marker' \
    'fi' \
    >"$qa_fake_bot"
  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'set -euo pipefail' \
    '[ "${1:-}" = loot-two-session-atomic-race ] && [ "${2:-}" = --yes ]' \
    '[ "${RUST_CAPTURE_LOOT_FIXTURE_GUARD:-}" = 1 ]' \
    '[ "${RUST_CAPTURE_ACK_LOOT_FIXTURE_MUTATION:-}" = 1 ]' \
    '[ "${RUST_CAPTURE_EXEC:?}" = "${QA_FAKE_TARGET_EXEC:?}" ]' \
    '[ "${RUST_CAPTURE_EXEC_SHA256:?}" = "${QA_FAKE_TARGET_SHA:?}" ]' \
    '[ -n "${RUST_CAPTURE_DB_CONF:?}" ]' \
    '[ "${PM2_RUST_WORLD:?}" = "${QA_FAKE_PROCESS_NAME:?}" ]' \
    '[ "${RUST_WORLD_PORT:?}" = "${QA_FAKE_WORLD_PORT:?}" ]' \
    '[ "${RUST_INSTANCE_PORT:?}" = "${QA_FAKE_INSTANCE_PORT:?}" ]' \
    'report="${WOW_BOT_REPORT:?}"' \
    '[[ "$report" = /* && "$report" != *$'\''\n'\''* ]]' \
    '[ -d "$(dirname -- "$report")" ] && [ ! -L "$(dirname -- "$report")" ]' \
    '[ ! -e "$report" ] && [ ! -L "$report" ]' \
    'source "${QA_FAKE_CAPTURE_COMMON:?}"' \
    'journal="${WOW_BOT_FIXTURE_JOURNAL:?}"' \
    'capture_pid=""' \
    'capture_bot_ready=0' \
    'restore() {' \
    '  status=$?' \
    '  trap - EXIT HUP INT TERM' \
    '  set +e' \
    '  if [ -n "$capture_pid" ]; then kill "$capture_pid" 2>/dev/null; wait "$capture_pid" 2>/dev/null; fi' \
    '  marker="${journal}.cleanup-complete"' \
    '  marker_mode="$(stat -c %a -- "$marker" 2>/dev/null || true)"' \
    '  if [ "$capture_bot_ready" = 0 ]; then' \
    '    cleanup_safe=$([ ! -e "$journal" ] && [ ! -L "$journal" ] && [ ! -e "$marker" ] && [ ! -L "$marker" ]; echo $?)' \
    '  else' \
    '    cleanup_safe=$([ ! -e "$journal" ] && [ ! -L "$journal" ] && [ -f "$marker" ] && [ ! -L "$marker" ] && [ "$marker_mode" = 600 ] && jq -e ".version == 1 and (.journal_sha256 | type == \"string\" and length == 64) and (.cleanup_pid | type == \"number\" and . > 0)" "$marker" >/dev/null 2>&1; echo $?)' \
    '  fi' \
    '  if [ "$cleanup_safe" != 0 ]; then' \
    '    printf "%s\n" "fake guarded capture refused normal PM2 restore: pending journal or missing cleanup-complete" >&2' \
    '    exit 74' \
    '  fi' \
    '  jq -cn --arg name "$QA_FAKE_PROCESS_NAME" --arg exec "$QA_FAKE_ORIGINAL_EXEC" --argjson pid "$QA_FAKE_ORIGINAL_PID" '\''[{name:$name,pid:$pid,pm2_env:{status:"online",pm_exec_path:$exec,restart_time:9,env:{}}}]'\'' >"$QA_FAKE_PM2_JSON_FILE"' \
    '  rm -f -- "${journal}.cleanup-complete"' \
    '  printf "%s\n" "fake guarded capture restored normal PM2 profile"' \
    '  exit "$status"' \
    '}' \
    'trap restore EXIT' \
    'trap '\''exit 129'\'' HUP' \
    'trap '\''exit 130'\'' INT' \
    'trap '\''exit 143'\'' TERM' \
    'if [ -n "${QA_FAKE_CAPTURE_BEFORE_READY_STATUS:-}" ]; then exit "$QA_FAKE_CAPTURE_BEFORE_READY_STATUS"; fi' \
    '"$QA_FAKE_TARGET_EXEC" 300 &' \
    'capture_pid=$!' \
    'jq -cn --arg name "$QA_FAKE_PROCESS_NAME" --arg exec "$QA_FAKE_TARGET_EXEC" --argjson pid "$capture_pid" '\''[{name:$name,pid:$pid,pm2_env:{status:"online",pm_exec_path:$exec,restart_time:8,RUSTYCORE_PACKET_DUMP_DIR:"/tmp/fake-dump",env:{RUSTYCORE_PACKET_DUMP_DIR:"/tmp/fake-dump"}}}]'\'' >"$QA_FAKE_PM2_JSON_FILE"' \
    'capture_bot_ready=1' \
    'printf "%s\n" ">>> Perform the '\''loot-two-session-atomic-race'\'' flow with the client now."' \
    'IFS= read -r _' \
    'if ! capture_loot_race_bot_evidence "$report" "$WOW_BOT_EXEC" "$WOW_BOT_EXEC_SHA256" >/dev/null; then' \
    '  printf "%s\n" "fake guarded capture refused publication without exact pinned race report" >&2' \
    '  exit 75' \
    'fi' \
    'printf "%s\n" published >"${QA_FAKE_CAPTURE_PUBLISH_MARKER:?}"' \
    'exit "${QA_FAKE_CAPTURE_EXIT_STATUS:-0}"' \
    >"$qa_fake_capture"
  chmod +x "$qa_fake_bin/mysql" "$qa_fake_bin/pm2" "$qa_fake_bin/ss" \
    "$qa_fake_bot" "$qa_fake_capture"

  cp -- "$(command -v sleep)" "$qa_fake_world"
  cp -- "$qa_fake_world" "$qa_fake_world_other"
  chmod +x "$qa_fake_world" "$qa_fake_world_other"
  "$qa_fake_world" 300 &
  qa_world_pid=$!
  kill -0 "$qa_world_pid" 2>/dev/null || die \
    "temporary world-process self-test fixture did not start"
  qa_world_sha="$(sha256_of_file "$qa_fake_world")" || die \
    "cannot hash temporary world-process self-test fixture"
  qa_target_sha="$(sha256_of_file "$qa_fake_world_other")" || die \
    "cannot hash temporary target-world self-test fixture"
  qa_fake_bot_sha="$(sha256_of_file "$qa_fake_bot")" || die \
    "cannot hash fake loot-race bot"
  qa_fake_capture_publish_marker="$artifacts/fake-loot-race-capture-published"

  qa_pm2_online="$(jq -cn \
    --arg name self-test-world \
    --arg exec "$qa_fake_world" \
    --argjson pid "$qa_world_pid" \
    '[{name:$name,pid:$pid,pm2_env:{status:"online",pm_exec_path:$exec,restart_time:7}}]')"
  qa_pm2_duplicate="$(jq -cn --argjson rows "$qa_pm2_online" '$rows + $rows')"
  qa_pm2_offline="$(jq -cn \
    --arg name self-test-world \
    --arg exec "$qa_fake_world" \
    --argjson pid "$qa_world_pid" \
    '[{name:$name,pid:$pid,pm2_env:{status:"stopped",pm_exec_path:$exec,restart_time:7}}]')"
  qa_pm2_wrong_path="$(jq -cn \
    --arg name self-test-world \
    --arg exec "$qa_fake_world_other" \
    --argjson pid "$qa_world_pid" \
    '[{name:$name,pid:$pid,pm2_env:{status:"online",pm_exec_path:$exec,restart_time:7}}]')"
  qa_pm2_restart_changed="$(jq -cn \
    --arg name self-test-world \
    --arg exec "$qa_fake_world" \
    --argjson pid "$qa_world_pid" \
    '[{name:$name,pid:$pid,pm2_env:{status:"online",pm_exec_path:$exec,restart_time:8}}]')"
  qa_pm2_pid_changed="$(jq -cn \
    --arg name self-test-world \
    --arg exec "$qa_fake_world" \
    --argjson pid "$((qa_world_pid + 1))" \
    '[{name:$name,pid:$pid,pm2_env:{status:"online",pm_exec_path:$exec,restart_time:7}}]')"

  printf '%s\n' "$qa_pm2_duplicate" >"$qa_pm2_json_file"
  if (
    export PATH="$qa_fake_bin" QA_FAKE_PM2_JSON_FILE="$qa_pm2_json_file"
    qa_world_identity self-test-world "$qa_fake_world" >/dev/null 2>&1
  ); then
    die "world identity accepted duplicate PM2 entries"
  fi
  printf '%s\n' "$qa_pm2_offline" >"$qa_pm2_json_file"
  if (
    export PATH="$qa_fake_bin" QA_FAKE_PM2_JSON_FILE="$qa_pm2_json_file"
    qa_world_identity self-test-world "$qa_fake_world" >/dev/null 2>&1
  ); then
    die "world identity accepted an offline PM2 entry"
  fi
  printf '%s\n' "$qa_pm2_wrong_path" >"$qa_pm2_json_file"
  if (
    export PATH="$qa_fake_bin" QA_FAKE_PM2_JSON_FILE="$qa_pm2_json_file"
    qa_world_identity self-test-world "$qa_fake_world" >/dev/null 2>&1
  ); then
    die "world identity accepted the wrong PM2 executable path"
  fi

  printf '%s\n' "$qa_pm2_online" >"$qa_pm2_json_file"
  qa_identity="$(
    export PATH="$qa_fake_bin" QA_FAKE_PM2_JSON_FILE="$qa_pm2_json_file"
    qa_world_identity self-test-world "$qa_fake_world"
  )" || die "world identity rejected the valid fake PM2 entry"
  [[ "$qa_identity" == "$qa_world_pid"$'\t''7' ]] || die \
    "world identity returned an unexpected PID/restart tuple"
  qa_world_process_matches "$qa_identity" "$qa_fake_world" "$qa_world_sha" || die \
    "world process pin rejected the valid live fake executable"
  if qa_world_process_matches \
    "$qa_identity" "$qa_fake_world_other" "$qa_world_sha"; then
    die "world process pin accepted the wrong executable path"
  fi
  if qa_world_process_matches \
    "$qa_identity" "$qa_fake_world" \
    0000000000000000000000000000000000000000000000000000000000000000; then
    die "world process pin accepted the wrong SHA-256"
  fi
  if (
    export PATH="$qa_fake_bin" \
      QA_FAKE_WORLD_PORT="$qa_world_port" \
      QA_FAKE_INSTANCE_PORT="$qa_instance_port" \
      QA_FAKE_SS_WORLD_PID="$qa_world_pid" \
      QA_FAKE_SS_INSTANCE_PID=
    qa_world_ports_ready "$qa_identity" "$qa_world_port" "$qa_instance_port"
  ); then
    die "world listener gate accepted a missing instance listener"
  fi
  (
    export PATH="$qa_fake_bin" \
      QA_FAKE_WORLD_PORT="$qa_world_port" \
      QA_FAKE_INSTANCE_PORT="$qa_instance_port" \
      QA_FAKE_SS_WORLD_PID="$qa_world_pid" \
      QA_FAKE_SS_INSTANCE_PID="$qa_world_pid"
    qa_world_ports_ready "$qa_identity" "$qa_world_port" "$qa_instance_port"
  ) || die "world listener gate rejected the valid fake listeners"
  if (
    export PATH="$qa_fake_bin" \
      QA_FAKE_WORLD_PORT="$qa_world_port" \
      QA_FAKE_INSTANCE_PORT="$qa_instance_port" \
      QA_FAKE_SS_WORLD_PID="$qa_world_pid" \
      QA_FAKE_SS_INSTANCE_PID="$qa_world_pid" \
      QA_FAKE_SS_EXTRA_PID="$((qa_world_pid + 1))"
    qa_world_ports_ready "$qa_identity" "$qa_world_port" "$qa_instance_port"
  ); then
    die "world listener gate accepted a foreign PID on the same SO_REUSEPORT row"
  fi

  printf '%s\n' "$qa_pm2_online" >"$qa_pm2_json_file"
  printf '%s\n' "$qa_pm2_online" >"$qa_pm2_before_file"
  printf '%s\n' "$qa_pm2_restart_changed" >"$qa_pm2_after_file"
  qa_common_env=(
    "PATH=$qa_fake_bin"
    "TMPDIR=$artifacts"
    "RUST_MIN_STACK=$DEFAULT_RUST_MIN_STACK"
    "PM2_RUST_WORLD=self-test-world"
    "WOW_BOT_WORLD_EXEC=$qa_fake_world_other"
    "WOW_BOT_WORLD_EXEC_SHA256=$qa_target_sha"
    "RUST_WORLD_PORT=$qa_world_port"
    "RUST_INSTANCE_PORT=$qa_instance_port"
    "QA_FAKE_PM2_JSON_FILE=$qa_pm2_json_file"
    "QA_FAKE_PM2_BEFORE_FILE=$qa_pm2_before_file"
    "QA_FAKE_PM2_AFTER_FILE=$qa_pm2_after_file"
    "QA_FAKE_PM2_STATE_FILE="
    "QA_FAKE_WORLD_PORT=$qa_world_port"
    "QA_FAKE_INSTANCE_PORT=$qa_instance_port"
    "QA_FAKE_SS_WORLD_PID=$qa_world_pid"
    "QA_FAKE_SS_INSTANCE_PID=$qa_world_pid"
    "QA_FAKE_SS_DYNAMIC=1"
    "QA_FAKE_ORIGINAL_EXEC=$qa_fake_world"
    "QA_FAKE_TARGET_EXEC=$qa_fake_world_other"
    "QA_FAKE_TARGET_SHA=$qa_target_sha"
    "QA_FAKE_PROCESS_NAME=self-test-world"
    "QA_FAKE_ORIGINAL_PID=$qa_world_pid"
    "RUST_CAPTURE_DB_CONF=/dev/null"
    "WOW_BOT_ENV_FILE=/dev/null"
    "WOW_BOT_PASSWORD=self-test"
    "WOW_BOT_GENERATE_LOCAL_PASSWORD=0"
    "WOW_BOT_ENSURE_TEST_ACCOUNTS=1"
    "WOW_BOT_EXEC=$qa_fake_bot"
    "WOW_BOT_EXEC_SHA256=$qa_fake_bot_sha"
    "WOW_BOT_REPORT=$artifacts/fake-loot-race-report.json"
    "WOW_BOT_LOG=$artifacts/fake-loot-race.log"
    "QA_FAKE_BOT_MARKER=$qa_fake_bot_marker"
    "QA_FAKE_CAPTURE_COMMON=$REPO_ROOT/crates/capture-diff/scripts/capture-service-common.sh"
    "QA_FAKE_CAPTURE_PUBLISH_MARKER=$qa_fake_capture_publish_marker"
  )

  rm -f "$qa_fake_bot_marker" "$qa_fake_capture_publish_marker" "$qa_pm2_state_file"
  printf '%s\n' "$qa_pm2_online" >"$qa_pm2_json_file"
  qa_early_output="$(
    (
      export "${qa_common_env[@]}" QA_FAKE_CAPTURE_BEFORE_READY_STATUS=73
      ALLOW_RUNTIME_QA=1
      ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=1
      QA_LOOT_RACE_CAPTURE_SCRIPT="$qa_fake_capture"
      run_qa_loot_race
    ) 2>&1
  )" || qa_early_status=$?
  [[ "$qa_early_status" == 73 \
    && "$qa_early_output" == *"exited before its ready marker (status 73)"* \
    && "$qa_early_output" == *"fake guarded capture restored normal PM2 profile"* ]] || die \
    "pre-ready wrapper failure did not restore safely and preserve its status"

  rm -f "$qa_fake_bot_marker" "$qa_fake_capture_publish_marker" "$qa_pm2_state_file"
  printf '%s\n' "$qa_pm2_online" >"$qa_pm2_json_file"
  qa_positive_output="$(
    (
      export "${qa_common_env[@]}"
      ALLOW_RUNTIME_QA=1
      ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=1
      QA_LOOT_RACE_CAPTURE_SCRIPT="$qa_fake_capture"
      run_qa_loot_race
    ) 2>&1
  )" || {
    printf '%s\n' "$qa_positive_output" >&2
    die "guarded capture QA self-test rejected valid fake infrastructure"
  }
  [[ "$(<"$qa_fake_bot_marker")" == fake-bot-only ]] || die \
    "guarded capture QA self-test did not execute only the fake bot"
  [[ "$(<"$qa_fake_capture_publish_marker")" == published \
    && "$qa_positive_output" == *"fake guarded capture restored normal PM2 profile"* ]] || die \
    "guarded capture QA self-test did not gate publication on the exact report and restore"

  rm -f "$qa_fake_bot_marker" "$qa_fake_capture_publish_marker" "$qa_pm2_state_file"
  printf '%s\n' "$qa_pm2_online" >"$qa_pm2_json_file"
  if qa_summary_output="$(
    (
      export "${qa_common_env[@]}" QA_FAKE_BOT_REPORT_MODE=summary
      ALLOW_RUNTIME_QA=1
      ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=1
      QA_LOOT_RACE_CAPTURE_SCRIPT="$qa_fake_capture"
      run_qa_loot_race
    ) 2>&1
  )"; then
    die "guarded capture QA accepted a summary-only bot report"
  fi
  [[ -f "$qa_fake_bot_marker" && ! -e "$qa_fake_capture_publish_marker" \
    && "$qa_summary_output" == *"report did not prove the exact successful two-session contract"* \
    && "$qa_summary_output" == *"refused publication without exact pinned race report"* \
    && "$qa_summary_output" == *"fake guarded capture restored normal PM2 profile"* ]] || die \
    "summary-only bot report self-test did not block publication and restore PM2"

  rm -f "$qa_fake_bot_marker" "$qa_pm2_state_file"
  printf '%s\n' "$qa_pm2_online" >"$qa_pm2_json_file"
  if qa_restart_output="$(
    (
      export "${qa_common_env[@]}" QA_FAKE_BOT_MUTATION=restart
      ALLOW_RUNTIME_QA=1
      ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=1
      QA_LOOT_RACE_CAPTURE_SCRIPT="$qa_fake_capture"
      run_qa_loot_race
    ) 2>&1
  )"; then
    die "guarded capture QA accepted a changed PM2 restart count"
  fi
  if [[ ! -f "$qa_fake_bot_marker" \
      || "$qa_restart_output" != *"guarded capture world restarted during loot-race QA"* \
      || "$qa_restart_output" != *"fake guarded capture restored normal PM2 profile"* ]]; then
    printf '%s\n' "$qa_restart_output" >&2
    die "restart-count self-test did not fail closed and restore after the fake bot"
  fi

  rm -f "$qa_fake_bot_marker" "$qa_pm2_state_file"
  printf '%s\n' "$qa_pm2_online" >"$qa_pm2_json_file"
  if qa_pid_output="$(
    (
      export "${qa_common_env[@]}" QA_FAKE_BOT_MUTATION=pid
      ALLOW_RUNTIME_QA=1
      ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=1
      QA_LOOT_RACE_CAPTURE_SCRIPT="$qa_fake_capture"
      run_qa_loot_race
    ) 2>&1
  )"; then
    die "guarded capture QA accepted a changed PM2 PID"
  fi
  [[ -f "$qa_fake_bot_marker" \
    && "$qa_pid_output" == *"guarded capture world restarted during loot-race QA"* \
    && "$qa_pid_output" == *"fake guarded capture restored normal PM2 profile"* ]] || die \
    "PID-change self-test did not fail closed and restore after the fake bot"

  rm -f "$qa_fake_bot_marker" "$qa_pm2_state_file"
  printf '%s\n' "$qa_pm2_online" >"$qa_pm2_json_file"
  if qa_cleanup_output="$(
    (
      export "${qa_common_env[@]}" QA_FAKE_CAPTURE_EXIT_STATUS=73
      ALLOW_RUNTIME_QA=1
      ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=1
      QA_LOOT_RACE_CAPTURE_SCRIPT="$qa_fake_capture"
      run_qa_loot_race
    ) 2>&1
  )"; then
    die "guarded capture QA ignored fixture-cleanup wrapper failure"
  fi
  [[ "$qa_cleanup_output" == *"guarded capture/fixture restoration failed with status 73"* \
    && "$qa_cleanup_output" == *"fake guarded capture restored normal PM2 profile"* ]] || die \
    "fixture-cleanup failure self-test did not propagate the wrapper status"

  rm -f "$qa_fake_bot_marker" "$qa_pm2_state_file"
  printf '%s\n' "$qa_pm2_online" >"$qa_pm2_json_file"
  if qa_term_output="$(
    (
      export "${qa_common_env[@]}" QA_FAKE_BOT_MUTATION=term
      ALLOW_RUNTIME_QA=1
      ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=1
      QA_LOOT_RACE_CAPTURE_SCRIPT="$qa_fake_capture"
      run_qa_loot_race
    ) 2>&1
  )"; then
    die "guarded capture QA restored normal PM2 after a TERM-interrupted fixture"
  fi
  [[ "$qa_term_output" == *"guarded capture/fixture restoration failed with status 74"* \
    && "$qa_term_output" == *"pending journal or missing cleanup-complete"* \
    && "$qa_term_output" != *"restored normal PM2 profile"* ]] || die \
    "TERM self-test did not retain the pending journal and prioritize cleanup failure"

  rm -f "$qa_fake_bot_marker" "$qa_pm2_state_file"
  printf '%s\n' "$qa_pm2_online" >"$qa_pm2_json_file"
  if qa_kill_output="$(
    (
      export "${qa_common_env[@]}" QA_FAKE_BOT_MUTATION=kill
      ALLOW_RUNTIME_QA=1
      ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=1
      QA_LOOT_RACE_CAPTURE_SCRIPT="$qa_fake_capture"
      run_qa_loot_race
    ) 2>&1
  )"; then
    die "guarded capture QA restored normal PM2 after a KILL-interrupted fixture"
  fi
  [[ "$qa_kill_output" == *"guarded capture/fixture restoration failed with status 74"* \
    && "$qa_kill_output" == *"pending journal or missing cleanup-complete"* \
    && "$qa_kill_output" != *"restored normal PM2 profile"* ]] || die \
    "KILL self-test did not retain the pending journal and prioritize cleanup failure"

  rm -f "$qa_fake_bot_marker" "$qa_pm2_state_file"
  printf '%s\n' "$qa_pm2_online" >"$qa_pm2_json_file"
  if qa_drift_output="$(
    (
      export "${qa_common_env[@]}" QA_FAKE_BOT_MUTATION=drift
      ALLOW_RUNTIME_QA=1
      ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=1
      QA_LOOT_RACE_CAPTURE_SCRIPT="$qa_fake_capture"
      run_qa_loot_race
    ) 2>&1
  )"; then
    die "guarded capture QA accepted cleanup-complete while its recovery journal remained pending"
  fi
  [[ "$qa_drift_output" == *"guarded capture/fixture restoration failed with status 74"* \
    && "$qa_drift_output" == *"pending journal or missing cleanup-complete"* \
    && "$qa_drift_output" != *"restored normal PM2 profile"* ]] || die \
    "journal-drift self-test did not fail closed"

  review_dry_run_output="$(PATH="$artifacts/bin" \
    "$BASH" "$REPO_ROOT/tools/pr-preflight.sh" --dry-run review HEAD 2>&1)" || die \
    "review dry-run unexpectedly requires optional execution tools"
  [[ "$review_dry_run_output" == *"+ codex"* ]] || die \
    "review dry-run did not print the Codex command"

  full_dry_run_output="$(PATH="$artifacts/bin" \
    "$BASH" "$REPO_ROOT/tools/pr-preflight.sh" --dry-run full HEAD 2>&1)" || die \
    "full dry-run unexpectedly requires optional execution tools"
  [[ "$full_dry_run_output" == *"+ codex"* ]] || die \
    "full dry-run did not print the Codex command"
  require_exact_occurrences "$full_dry_run_output" \
    "test --locked -p capture-diff" 1 \
    "local full capture-diff test command"
  require_exact_occurrences "$full_dry_run_output" \
    "verify-required loot-single-item-claim" 1 \
    "local full required-flow command"

  self_test_cleanup "$qa_world_pid" "$artifacts"
  qa_world_pid=""
  artifacts=""
  trap - EXIT
  log "Preflight self-test passed"
}

run_codex_review() {
  local selector="$1"
  local value="${2:-}"
  local policy_toml
  local artifacts
  local result_file
  local events_file
  local codex_rc=0
  local inspection_rc=0
  local review_rc=0
  local merge_base=""
  local prompt=""

  [[ -f "$POLICY_FILE" ]] || die "missing Codex review policy: $POLICY_FILE"
  [[ -f "$SCHEMA_FILE" ]] || die "missing Codex review schema: $SCHEMA_FILE"
  [[ "$CODEX_REVIEW_TIMEOUT_SECONDS" =~ ^[1-9][0-9]*$ ]] || die \
    "CODEX_REVIEW_TIMEOUT_SECONDS must be a positive integer"

  if [[ "$selector" == "--base" ]]; then
    require_clean_worktree
    merge_base="$(merge_base_for "$value")"
    prompt="Before other inspection, run the exact unmodified command git diff ${merge_base}..HEAD in its own shell tool call. Do not combine it with another command, pipe, redirection, or fallback. Review only that committed patch. Use additional read-only commands to inspect the repository and relevant C++ reference sources. Do not review uncommitted files. Return the structured review required by the output schema."
  else
    prompt="Before other inspection, run each of these exact unmodified commands in its own separate shell tool call: git diff; git diff --cached; git ls-files --others --exclude-standard. Do not combine them with another command, pipe, redirection, or fallback. Review all staged, unstaged, and untracked changes in this repository. Inspect the contents of any untracked files plus relevant C++ reference sources with additional read-only commands. Return the structured review required by the output schema."
  fi

  if ((DRY_RUN)); then
    print_command codex -a never -c "developer_instructions=<tools/codex-review-policy.md>" \
      exec --ephemeral --ignore-user-config --ignore-rules --disable hooks --disable plugins \
      --disable apps --json --color never --sandbox read-only -C "$REPO_ROOT" \
      --output-schema "$SCHEMA_FILE" -o '<result.json>' '<review prompt>'
    return
  fi

  require_command codex
  require_command python3
  require_command timeout
  policy_toml="$(python3 -c 'import json, pathlib, sys; print(json.dumps(pathlib.Path(sys.argv[1]).read_text()))' "$POLICY_FILE")"

  artifacts="$(mktemp -d "${TMPDIR:-/tmp}/rustycore-codex-review.XXXXXX")"
  result_file="$artifacts/result.json"
  events_file="$artifacts/events.jsonl"

  log "Local Codex review ($selector${value:+ $value})"
  printf 'Codex review event log: %s\n' "$events_file"
  timeout --foreground --signal=TERM --kill-after=30 \
    "${CODEX_REVIEW_TIMEOUT_SECONDS}s" \
    codex -a never -c "developer_instructions=$policy_toml" \
    exec --ephemeral --ignore-user-config --ignore-rules --disable hooks --disable plugins \
    --disable apps --json --color never --sandbox read-only -C "$REPO_ROOT" \
    --output-schema "$SCHEMA_FILE" -o "$result_file" "$prompt" \
    >"$events_file" || codex_rc=$?

  if ((codex_rc != 0)); then
    warn "Codex failed with exit code $codex_rc; event log: $events_file"
    return "$codex_rc"
  fi

  review_inspection_result "$events_file" "$selector" "$merge_base" || inspection_rc=$?
  if ((inspection_rc != 0)); then
    warn "Codex review artifacts: $artifacts"
    return "$inspection_rc"
  fi

  review_result "$result_file" || review_rc=$?
  if ((review_rc != 0)); then
    warn "Codex review artifacts: $artifacts"
    return "$review_rc"
  fi

  if [[ "${CODEX_REVIEW_KEEP_ARTIFACTS:-0}" == "1" ]]; then
    printf 'Codex review artifacts: %s\n' "$artifacts"
  else
    rm -rf "$artifacts"
  fi
}

run_review() {
  local base="$1"
  run_codex_review --base "$base"
}

run_review_uncommitted() {
  log "Reviewing uncommitted work; this does not correspond to a PR HEAD"
  run_codex_review --uncommitted
}

run_full() {
  local base="$1"
  require_clean_worktree
  run_diff "$base"
  run_ci
  run_review "$base"
}

run_stable() {
  log "Latest stable compatibility (scheduled/dispatch CI profile)"
  resolve_protoc
  run_cmd cargo +stable check --locked -p bnet-server -p world-server
  run_cmd cargo +stable build --locked -p bnet-server -p world-server
}

run_qa_login() {
  ((ALLOW_RUNTIME_QA == 1)) || die \
    "qa-login can modify local QA account/session data; rerun with --allow-runtime-qa"
  log "Live login QA bot"
  run_cmd "$REPO_ROOT/tools/wow-test-bot/run_rustycore_login_smoke.sh"
}

run_qa_loot_race() {
  local capture_script="$QA_LOOT_RACE_CAPTURE_SCRIPT"
  local capture_status=0
  local bot_exec="${WOW_BOT_EXEC:-}"
  local bot_expected_sha="${WOW_BOT_EXEC_SHA256:-}"
  local bot_log
  local bot_report
  local control_fifo
  local cpp_process_name="${PM2_CPP_WORLD:-cpp-world}"
  local db_conf="${RUST_CAPTURE_DB_CONF:-${WOW_BOT_DB_CONF:-/home/server/trinity-legacy-install/bin/worldserver.conf}}"
  local effective_config="${RUST_CAPTURE_EFFECTIVE_CONFIG:-/home/server/trinity-legacy-install/etc/worldserver.conf}"
  local expected_sha
  local fixture_cleanup_marker
  local fixture_journal
  local identity_before
  local identity_capture
  local identity_after
  local identity_restored
  local original_canonical_exec
  local original_exec
  local original_pid
  local original_restart
  local original_sha
  local original_snapshot
  local process_name="${PM2_RUST_WORLD:-rustycore-world}"
  local world_exec="${WOW_BOT_WORLD_EXEC:-}"
  local world_port="${RUST_WORLD_PORT:-8085}"
  local instance_port="${RUST_INSTANCE_PORT:-8086}"
  local bot_status=0
  local bnet_port="${BNET_PORT:-8081}"
  local qa_status=0

  ((ALLOW_RUNTIME_QA == 1)) || die \
    "qa-loot-race modifies disposable characters and world state; rerun with --allow-runtime-qa"
  ((ACK_DISPOSABLE_OVERWORLD_LOOT_RACE == 1)) || die \
    "qa-loot-race mutates an exact shared-chest fixture; rerun with --ack-disposable-overworld-loot-race"
  log "Live two-session atomic loot-claim QA bot"

  valid_tcp_port "$world_port" || die "RUST_WORLD_PORT must be an integer from 1 through 65535"
  valid_tcp_port "$instance_port" || die \
    "RUST_INSTANCE_PORT must be an integer from 1 through 65535"
  [[ "$world_port" != "$instance_port" ]] || die \
    "RUST_WORLD_PORT and RUST_INSTANCE_PORT must be distinct"
  valid_tcp_port "$bnet_port" || die "BNET_PORT must be an integer from 1 through 65535"

  if ((DRY_RUN)); then
    fixture_journal="/tmp/rustycore-loot-race-qa/fixture.journal"
    bot_report="/tmp/rustycore-loot-race-qa/bot-report.json"
    printf '+ snapshot the pinned PM2 world, then start the guarded capture world via FIFO/background\n'
    run_cmd env \
      PM2_RUST_WORLD="$process_name" \
      PM2_CPP_WORLD="$cpp_process_name" \
      RUST_WORLD_PORT="$world_port" \
      RUST_INSTANCE_PORT="$instance_port" \
      RUST_CAPTURE_EXEC="$world_exec" \
      RUST_CAPTURE_EXEC_SHA256="${WOW_BOT_WORLD_EXEC_SHA256:-}" \
      RUST_CAPTURE_LOOT_FIXTURE_GUARD=1 \
      RUST_CAPTURE_ACK_LOOT_FIXTURE_MUTATION=1 \
      RUST_CAPTURE_DB_CONF="$db_conf" \
      RUST_CAPTURE_EFFECTIVE_CONFIG="$effective_config" \
      WOW_BOT_EXEC="$bot_exec" \
      WOW_BOT_EXEC_SHA256="$bot_expected_sha" \
      WOW_BOT_REPORT="$bot_report" \
      WOW_BOT_FIXTURE_JOURNAL="$fixture_journal" \
      "$capture_script" loot-two-session-atomic-race --yes
    printf '+ wait for guarded capture READY marker and accredit its exact PID/executable/listeners\n'
    run_cmd env \
      BNET_HOST=127.0.0.1 \
      BNET_PORT="$bnet_port" \
      WORLD_HOST=127.0.0.1 \
      WORLD_PORT="$world_port" \
      INSTANCE_HOST=127.0.0.1 \
      INSTANCE_PORT="$instance_port" \
      WOW_BOT_DB_CONF="$db_conf" \
      WOW_BOT_ENSURE_TEST_ACCOUNTS=0 \
      WOW_BOT_EXEC="$bot_exec" \
      WOW_BOT_EXEC_SHA256="$bot_expected_sha" \
      WOW_BOT_FIXTURE_JOURNAL="$fixture_journal" \
      WOW_BOT_LOOT_RACE_SMOKE=1 \
      WOW_BOT_ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=1 \
      WOW_BOT_LOOT_RACE_ACCOUNT_A=TESTBOT2@bot.local \
      WOW_BOT_LOOT_RACE_ACCOUNT_B=TESTBOT3@bot.local \
      WOW_BOT_LOOT_RACE_GAMEOBJECT_ENTRY=2846 \
      WOW_BOT_LOOT_RACE_GAMEOBJECT_SPAWN_GUID=9106001 \
      WOW_BOT_LOOT_RACE_RUNTIME_COUNTER=0 \
      WOW_BOT_LOOT_RACE_ITEM_ENTRY=38 \
      "$REPO_ROOT/tools/wow-test-bot/run_rustycore_login_smoke.sh"
    printf '+ verify capture PID/restart/executable/listeners unchanged, send ENTER, and wait for exact PM2/fixture restoration\n'
    return
  fi

  [[ -n "$world_exec" ]] || die \
    "qa-loot-race requires WOW_BOT_WORLD_EXEC pinned to the target capture world-server binary"
  [[ "$world_exec" == /* ]] || die "WOW_BOT_WORLD_EXEC must be an absolute canonical path"
  [[ ! -L "$world_exec" && -f "$world_exec" && -x "$world_exec" ]] || die \
    "WOW_BOT_WORLD_EXEC must be a regular executable file (not a symlink)"
  [[ "$(realpath -e -- "$world_exec" 2>/dev/null)" == "$world_exec" ]] || die \
    "WOW_BOT_WORLD_EXEC must already be canonical"
  expected_sha="${WOW_BOT_WORLD_EXEC_SHA256:-}"
  [[ "$expected_sha" =~ ^[[:xdigit:]]{64}$ ]] || die \
    "WOW_BOT_WORLD_EXEC_SHA256 must contain the pinned 64-digit SHA-256"
  expected_sha="${expected_sha,,}"
  [[ -n "$bot_exec" ]] || die \
    "qa-loot-race requires WOW_BOT_EXEC pinned to the exact wow-test-bot binary"
  [[ "$bot_exec" == /* ]] || die "WOW_BOT_EXEC must be an absolute canonical path"
  [[ ! -L "$bot_exec" && -f "$bot_exec" && -x "$bot_exec" ]] || die \
    "WOW_BOT_EXEC must be a regular executable file (not a symlink)"
  [[ "$(realpath -e -- "$bot_exec" 2>/dev/null)" == "$bot_exec" ]] || die \
    "WOW_BOT_EXEC must already be canonical"
  [[ "$bot_expected_sha" =~ ^[[:xdigit:]]{64}$ ]] || die \
    "WOW_BOT_EXEC_SHA256 must contain the pinned 64-digit SHA-256"
  bot_expected_sha="${bot_expected_sha,,}"
  require_command jq
  require_command mkfifo
  require_command mysql
  require_command pm2
  require_command realpath
  require_command rg
  require_command seq
  require_command sha256sum
  require_command ss
  [[ -x "$capture_script" && ! -L "$capture_script" ]] || die \
    "qa-loot-race capture wrapper is missing, non-executable, or a symlink: $capture_script"
  [[ "$(sha256_of_file "$world_exec")" == "$expected_sha" ]] || die \
    "WOW_BOT_WORLD_EXEC does not match WOW_BOT_WORLD_EXEC_SHA256"
  [[ "$(sha256_of_file "$bot_exec")" == "$bot_expected_sha" ]] || die \
    "WOW_BOT_EXEC does not match WOW_BOT_EXEC_SHA256"

  original_snapshot="$(qa_world_snapshot "$process_name")" || die \
    "PM2 process $process_name is not one exact online world executable"
  IFS=$'\t' read -r original_pid original_restart original_exec <<<"$original_snapshot"
  [[ "$original_pid" =~ ^[1-9][0-9]*$ && "$original_restart" =~ ^[0-9]+$ \
    && -n "$original_exec" ]] || die "PM2 normal-world snapshot was malformed"
  original_canonical_exec="$(realpath -e -- "$original_exec" 2>/dev/null)" || die \
    "normal PM2 world executable does not resolve: $original_exec"
  [[ -f "$original_canonical_exec" && -x "$original_canonical_exec" ]] || die \
    "normal PM2 world executable is not a regular executable file"
  original_sha="$(sha256_of_file "$original_canonical_exec")" || die \
    "cannot hash the normal PM2 world executable"
  identity_before="${original_pid}"$'\t'"${original_restart}"
  qa_world_process_matches \
    "$identity_before" "$original_canonical_exec" "$original_sha" || die \
    "live normal PM2 world bytes do not match its snapshotted executable"
  qa_world_ports_ready "$identity_before" "$world_port" "$instance_port" || die \
    "pinned PM2 world process does not own listeners $world_port and $instance_port"
  qa_world_packet_dump_absent "$process_name" || die \
    "normal PM2 world already carries RUSTYCORE_PACKET_DUMP_DIR; refusing nested capture QA"

  QA_LOOT_RACE_CAPTURE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/rustycore-loot-race-qa.XXXXXX")"
  QA_LOOT_RACE_CAPTURE_WAIT_STATUS=0
  QA_LOOT_RACE_CAPTURE_LOG="$QA_LOOT_RACE_CAPTURE_DIR/capture.log"
  QA_LOOT_RACE_FIXTURE_JOURNAL="$QA_LOOT_RACE_CAPTURE_DIR/fixture.journal"
  bot_report="$QA_LOOT_RACE_CAPTURE_DIR/bot-report.json"
  bot_log="$QA_LOOT_RACE_CAPTURE_DIR/bot.log"
  fixture_journal="$QA_LOOT_RACE_FIXTURE_JOURNAL"
  fixture_cleanup_marker="${fixture_journal}.cleanup-complete"
  [[ ! -e "$fixture_journal" && ! -L "$fixture_journal" \
    && ! -e "$fixture_cleanup_marker" && ! -L "$fixture_cleanup_marker" \
    && ! -e "$bot_report" && ! -L "$bot_report" \
    && ! -e "$bot_log" && ! -L "$bot_log" ]] || die \
    "fresh QA fixture/report path is unexpectedly occupied"
  control_fifo="$QA_LOOT_RACE_CAPTURE_DIR/control.fifo"
  mkfifo -- "$control_fifo"
  exec {QA_LOOT_RACE_CAPTURE_FD}<>"$control_fifo"
  trap qa_loot_race_capture_cleanup EXIT
  trap 'exit 129' HUP
  trap 'exit 130' INT
  trap 'exit 143' TERM

  env \
    PM2_RUST_WORLD="$process_name" \
    PM2_CPP_WORLD="$cpp_process_name" \
    RUST_WORLD_PORT="$world_port" \
    RUST_INSTANCE_PORT="$instance_port" \
    RUST_CAPTURE_EXEC="$world_exec" \
    RUST_CAPTURE_EXEC_SHA256="$expected_sha" \
    RUST_CAPTURE_LOOT_FIXTURE_GUARD=1 \
    RUST_CAPTURE_ACK_LOOT_FIXTURE_MUTATION=1 \
    RUST_CAPTURE_DB_CONF="$db_conf" \
    RUST_CAPTURE_EFFECTIVE_CONFIG="$effective_config" \
    WOW_BOT_EXEC="$bot_exec" \
    WOW_BOT_EXEC_SHA256="$bot_expected_sha" \
    WOW_BOT_REPORT="$bot_report" \
    WOW_BOT_FIXTURE_JOURNAL="$fixture_journal" \
    "$capture_script" loot-two-session-atomic-race --yes \
    <&"$QA_LOOT_RACE_CAPTURE_FD" >"$QA_LOOT_RACE_CAPTURE_LOG" 2>&1 &
  QA_LOOT_RACE_CAPTURE_PID=$!

  qa_wait_for_loot_capture_ready \
    "$QA_LOOT_RACE_CAPTURE_PID" "$QA_LOOT_RACE_CAPTURE_LOG" || die \
    "guarded capture wrapper did not become ready"
  identity_capture="$(qa_world_identity "$process_name" "$world_exec")" || die \
    "guarded capture world is not the pinned online executable"
  [[ "$identity_capture" != "$identity_before" ]] || die \
    "guarded capture wrapper did not replace the normal PM2 world process"
  qa_world_process_matches "$identity_capture" "$world_exec" "$expected_sha" || die \
    "guarded capture world bytes do not match the pinned executable"
  qa_world_ports_ready "$identity_capture" "$world_port" "$instance_port" || die \
    "guarded capture world does not own listeners $world_port and $instance_port"

  env \
    BNET_HOST=127.0.0.1 \
    BNET_PORT="$bnet_port" \
    WORLD_HOST=127.0.0.1 \
    WORLD_PORT="$world_port" \
    INSTANCE_HOST=127.0.0.1 \
    INSTANCE_PORT="$instance_port" \
    WOW_BOT_DB_CONF="$db_conf" \
    WOW_BOT_ENSURE_TEST_ACCOUNTS=0 \
    WOW_BOT_EXEC="$bot_exec" \
    WOW_BOT_EXEC_SHA256="$bot_expected_sha" \
    WOW_BOT_REPORT="$bot_report" \
    WOW_BOT_LOG="$bot_log" \
    WOW_BOT_FIXTURE_JOURNAL="$fixture_journal" \
    WOW_BOT_LOOT_RACE_SMOKE=1 \
    WOW_BOT_ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=1 \
    WOW_BOT_LOOT_RACE_ACCOUNT_A=TESTBOT2@bot.local \
    WOW_BOT_LOOT_RACE_ACCOUNT_B=TESTBOT3@bot.local \
    WOW_BOT_LOOT_RACE_GAMEOBJECT_ENTRY=2846 \
    WOW_BOT_LOOT_RACE_GAMEOBJECT_SPAWN_GUID=9106001 \
    WOW_BOT_LOOT_RACE_RUNTIME_COUNTER=0 \
    WOW_BOT_LOOT_RACE_ITEM_ENTRY=38 \
    "$REPO_ROOT/tools/wow-test-bot/run_rustycore_login_smoke.sh" &
  QA_LOOT_RACE_BOT_PID=$!
  wait "$QA_LOOT_RACE_BOT_PID" || bot_status=$?
  QA_LOOT_RACE_BOT_PID=""

  if ((bot_status == 0)); then
    if [[ ! -f "$bot_report" || -L "$bot_report" ]]; then
      echo "loot-race bot exited successfully without a fresh regular report" >&2
      bot_status=1
    elif ! jq -e '
      .loot_race_smoke == true
      and .loot_item_capture == false
      and (.results | type == "array" and length == 2)
      and ([.results[] | [.account, .account_id, .character_guid]] | sort
        == [["TESTBOT2@bot.local", 9, 15], ["TESTBOT3@bot.local", 10, 16]])
      and all(.results[];
        .world_auth == true
        and .enum_characters == true
        and .player_login_verified == true
        and .loot_race_smoke == true
        and .loot_race_smoke_passed == true
        and .loot_race_failure == null
        and .loot_race_target_entry == 2846
        and .loot_race_target_spawn_guid == 9106001
        and (.loot_race_target_runtime_counter | type == "number" and . > 0)
        and .loot_race_party_confirmed == true
        and .loot_race_target_discovered == true
        and .loot_race_loot_opened == true
        and (.loot_race_loot_list_id | type == "number" and . >= 0 and . <= 255)
        and .loot_race_loot_coins == 10
        and .loot_race_loot_removed_seen == true
        and .loot_race_coin_removed_seen == true
        and .loot_race_db_item_total == 1
        and .loot_race_db_money_delta == 10
        and .loot_race_relog_verified == true)
      and ([.results[].loot_race_target_runtime_counter] | unique | length == 1)
      and ([.results[].loot_race_loot_list_id] | unique | length == 1)
      and ([.results[] | select(.loot_race_item_push_seen == true)] | length == 1)
      and ([.results[] | select(.loot_race_item_push_seen == false)] | length == 1)
      and ([.results[].loot_race_money_notify_amount] | sort == [0, 10])
    ' "$bot_report" >/dev/null; then
      echo "loot-race bot report did not prove the exact successful two-session contract" >&2
      bot_status=1
    fi
  fi

  if ! identity_after="$(qa_world_identity "$process_name" "$world_exec")"; then
    echo "guarded capture world changed or stopped during loot-race QA" >&2
    qa_status=1
  elif [[ "$identity_after" != "$identity_capture" ]]; then
    echo "guarded capture world restarted during loot-race QA" >&2
    qa_status=1
  elif ! qa_world_process_matches "$identity_after" "$world_exec" "$expected_sha"; then
    echo "guarded capture world bytes changed during loot-race QA" >&2
    qa_status=1
  elif ! qa_world_ports_ready "$identity_after" "$world_port" "$instance_port"; then
    echo "guarded capture world listeners disappeared during loot-race QA" >&2
    qa_status=1
  fi
  if ((bot_status != 0)); then
    qa_status=$bot_status
  fi

  if ! printf '\n' >&"$QA_LOOT_RACE_CAPTURE_FD"; then
    echo "failed to signal guarded capture completion" >&2
    ((qa_status != 0)) || qa_status=1
  fi
  exec {QA_LOOT_RACE_CAPTURE_FD}>&-
  QA_LOOT_RACE_CAPTURE_FD=""
  wait "$QA_LOOT_RACE_CAPTURE_PID" || capture_status=$?
  QA_LOOT_RACE_CAPTURE_WAIT_STATUS=$capture_status
  QA_LOOT_RACE_CAPTURE_PID=""
  sed -n '1,240p' "$QA_LOOT_RACE_CAPTURE_LOG"
  if ((capture_status != 0)); then
    echo "guarded capture/fixture restoration failed with status $capture_status" >&2
    qa_status=$capture_status
  else
    if ! identity_restored="$(qa_world_identity "$process_name" "$original_exec")"; then
      echo "normal PM2 world was not restored after loot-race QA" >&2
      ((qa_status != 0)) || qa_status=1
    elif [[ "$identity_restored" == "$identity_capture" ]]; then
      echo "capture PM2 identity remained live instead of restoring the normal world" >&2
      ((qa_status != 0)) || qa_status=1
    elif ! qa_world_process_matches \
        "$identity_restored" "$original_canonical_exec" "$original_sha"; then
      echo "restored PM2 world does not match the snapshotted normal executable" >&2
      ((qa_status != 0)) || qa_status=1
    elif ! qa_world_ports_ready "$identity_restored" "$world_port" "$instance_port"; then
      echo "restored PM2 world does not own the accredited listeners" >&2
      ((qa_status != 0)) || qa_status=1
    elif ! qa_world_packet_dump_absent "$process_name"; then
      echo "restored PM2 world still carries RUSTYCORE_PACKET_DUMP_DIR" >&2
      ((qa_status != 0)) || qa_status=1
    fi
  fi

  trap - EXIT
  trap '' HUP INT TERM
  rm -f -- "$control_fifo"
  if ((qa_status == 0)) \
      && [[ ! -e "$fixture_journal" && ! -L "$fixture_journal" \
        && ! -e "$fixture_cleanup_marker" && ! -L "$fixture_cleanup_marker" ]]; then
    rm -f -- "$QA_LOOT_RACE_CAPTURE_LOG" "$bot_report" "$bot_log"
    rmdir -- "$QA_LOOT_RACE_CAPTURE_DIR" 2>/dev/null || true
  else
    echo "qa-loot-race recovery artifacts retained at ${QA_LOOT_RACE_CAPTURE_DIR}" >&2
  fi
  QA_LOOT_RACE_CAPTURE_DIR=""
  QA_LOOT_RACE_CAPTURE_LOG=""
  QA_LOOT_RACE_FIXTURE_JOURNAL=""
  QA_LOOT_RACE_CAPTURE_WAIT_STATUS=0
  return "$qa_status"
}

while (($# > 0)); do
  case "$1" in
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --allow-runtime-qa)
      ALLOW_RUNTIME_QA=1
      shift
      ;;
    --ack-disposable-overworld-loot-race)
      ACK_DISPOSABLE_OVERWORLD_LOOT_RACE=1
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    --)
      shift
      break
      ;;
    *)
      break
      ;;
  esac
done

COMMAND="${1:-help}"
if (($# > 0)); then
  shift
fi

cd "$REPO_ROOT"
require_command git
validate_rust_min_stack

case "$COMMAND" in
  self-test)
    (($# == 0)) || die "self-test does not accept arguments"
    run_self_test
    ;;
  architecture)
    (($# == 0)) || die "architecture does not accept arguments"
    run_architecture
    ;;
  format)
    (($# == 0)) || die "format does not accept arguments"
    run_format
    ;;
  check)
    (($# == 0)) || die "check does not accept arguments"
    run_check
    ;;
  test)
    (($# == 0)) || die "test does not accept arguments"
    run_test
    ;;
  ci)
    (($# == 0)) || die "ci does not accept arguments"
    run_ci
    ;;
  diff)
    (($# <= 1)) || die "diff accepts at most one BASE"
    run_diff "${1:-$DEFAULT_BASE}"
    ;;
  quick)
    (($# <= 1)) || die "quick accepts at most one BASE"
    run_quick "${1:-$DEFAULT_BASE}"
    ;;
  capture)
    (($# == 0)) || die "capture does not accept arguments"
    run_capture
    ;;
  review)
    (($# <= 1)) || die "review accepts at most one BASE"
    run_review "${1:-$DEFAULT_BASE}"
    ;;
  review-uncommitted)
    (($# == 0)) || die "review-uncommitted does not accept arguments"
    run_review_uncommitted
    ;;
  full)
    (($# <= 1)) || die "full accepts at most one BASE"
    run_full "${1:-$DEFAULT_BASE}"
    ;;
  stable)
    (($# == 0)) || die "stable does not accept arguments"
    run_stable
    ;;
  qa-login)
    (($# == 0)) || die "qa-login does not accept arguments"
    run_qa_login
    ;;
  qa-loot-race)
    (($# == 0)) || die "qa-loot-race does not accept arguments"
    run_qa_loot_race
    ;;
  help)
    usage
    ;;
  *)
    usage >&2
    die "unknown command: $COMMAND"
    ;;
esac
