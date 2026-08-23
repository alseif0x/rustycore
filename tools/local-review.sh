#!/usr/bin/env bash
set -euo pipefail

# Local Codex review, extracted verbatim from tools/pr-preflight.sh when #331
# retired that wrapper. Validation lives in tools/validation-v2; this reviews.

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
POLICY_FILE="$REPO_ROOT/tools/codex-review-policy.md"
SCHEMA_FILE="$REPO_ROOT/tools/codex-review-schema.json"
DEFAULT_BASE="origin/3.4.3"
CODEX_REVIEW_TIMEOUT_SECONDS="${CODEX_REVIEW_TIMEOUT_SECONDS:-1800}"
DRY_RUN=0

usage() {
  cat <<'USAGE'
RustyCore local Codex review

Usage:
  ./tools/local-review.sh [--dry-run] <COMMAND> [BASE]

Commands:
  review [BASE]       Review the clean committed diff against BASE.
  review-uncommitted  Review staged, unstaged, and untracked changes.

BASE defaults to origin/3.4.3. This is optional local advice: the required
remote gate for external authors is the Codex Review Gate workflow, and
validation is ./tools/validation-v2.
USAGE
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


while (($# > 0)); do
  case "$1" in
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    -h|--help)
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

case "$COMMAND" in
  review)
    (($# <= 1)) || die "review accepts at most one BASE"
    run_review "${1:-$DEFAULT_BASE}"
    ;;
  review-uncommitted)
    (($# == 0)) || die "review-uncommitted does not accept arguments"
    run_review_uncommitted
    ;;
  help)
    usage
    ;;
  *)
    usage >&2
    die "unknown command: $COMMAND"
    ;;
esac
