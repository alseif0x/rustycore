#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
POLICY_FILE="$REPO_ROOT/tools/codex-review-policy.md"
SCHEMA_FILE="$REPO_ROOT/tools/codex-review-schema.json"
PROTOC_VERSION_FILE="$REPO_ROOT/.protoc-version"
DEFAULT_BASE="origin/3.4.3"
CODEX_REVIEW_TIMEOUT_SECONDS="${CODEX_REVIEW_TIMEOUT_SECONDS:-1800}"
DRY_RUN=0
ALLOW_RUNTIME_QA=0

usage() {
  cat <<'EOF'
RustyCore local PR preflight

Usage:
  ./tools/pr-preflight.sh [OPTIONS] <COMMAND> [BASE]

Options:
  --dry-run           Print commands without running them.
  --allow-runtime-qa Allow qa-login to touch local QA account/session data.
  -h, --help          Show this help.

Commands:
  self-test           Test harness parsing and pinned-version invariants.
  format              Run the two formatting checks used by GitHub Actions.
  check               Run the locked core checks and server builds used by CI.
  test                Run the four focused library suites used by CI.
  ci                  Run format, check, and test (the required Rust CI jobs).
  diff [BASE]         Check committed, staged, and unstaged diffs for whitespace errors.
  quick [BASE]        Run diff, format, and check.
  capture             Run capture-diff regression tests (protoc not required).
  review [BASE]       Review the clean committed diff with local Codex.
  review-uncommitted  Review staged, unstaged, and untracked changes with local Codex.
  full [BASE]         Run diff, ci, capture, and review on a clean committed HEAD.
  stable              Check/build the server binaries with latest stable Rust.
  qa-login            Run the existing live login bot; requires --allow-runtime-qa.

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
  run_cmd cargo "+$channel" "$@"
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
    die "PROTOC must point to protoc $expected_version; checked:${found_versions:- none}"
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
}

run_test() {
  log "Focused library tests (same commands as GitHub Actions)"
  resolve_protoc
  cargo_cmd test --locked -p wow-data --lib
  cargo_cmd test --locked -p wow-packet --lib
  cargo_cmd test --locked -p wow-map --lib
  cargo_cmd test --locked -p wow-world --lib
}

run_ci() {
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
  run_format
  run_check
}

run_capture() {
  log "Committed capture-diff regression gate"
  cargo_cmd test --locked -p capture-diff
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

run_self_test() {
  local artifacts
  local capture_output
  local clean_result
  local dependency
  local findings_result
  local full_dry_run_output
  local invalid_result
  local review_dry_run_output
  local rc=0

  if ((DRY_RUN)); then
    log "Preflight self-test (dry-run)"
    print_command python3 -m json.tool "$SCHEMA_FILE"
    printf '+ validate Codex review parser exit codes 0, 10, and 65\n'
    return
  fi

  require_command python3
  project_protoc_version >/dev/null
  python3 -m json.tool "$SCHEMA_FILE" >/dev/null || die "invalid Codex review JSON schema"

  artifacts="$(mktemp -d "${TMPDIR:-/tmp}/rustycore-preflight-self-test.XXXXXX")"
  clean_result="$artifacts/clean.json"
  findings_result="$artifacts/findings.json"
  invalid_result="$artifacts/invalid.json"

  printf '%s\n' \
    '{"findings":[],"overall_correctness":"patch is correct","overall_explanation":"clean","overall_confidence_score":1}' >"$clean_result"
  printf '%s\n' \
    '{"findings":[{"title":"[P2] test","body":"test","confidence_score":1,"code_location":{"absolute_file_path":"/tmp/test","line_range":{"start":1,"end":1}}}],"overall_correctness":"patch is incorrect","overall_explanation":"finding","overall_confidence_score":1}' >"$findings_result"
  printf '%s\n' '[]' >"$invalid_result"

  review_result "$clean_result" >/dev/null
  review_result "$findings_result" >/dev/null 2>&1 || rc=$?
  [[ "$rc" == "10" ]] || die "review findings self-test returned $rc instead of 10"

  rc=0
  review_result "$invalid_result" >/dev/null 2>&1 || rc=$?
  [[ "$rc" == "65" ]] || die "invalid review self-test returned $rc instead of 65"

  mkdir -p "$artifacts/bin"
  for dependency in dirname git head sed tr; do
    ln -s "$(command -v "$dependency")" "$artifacts/bin/$dependency"
  done
  printf '#!/bin/sh\nexit 0\n' >"$artifacts/bin/cargo"
  chmod +x "$artifacts/bin/cargo"

  capture_output="$(PATH="$artifacts/bin" PROTOC="$artifacts/missing-protoc" \
    "$BASH" "$REPO_ROOT/tools/pr-preflight.sh" capture 2>&1)" || die \
    "capture profile unexpectedly requires protoc"
  [[ "$capture_output" == *"test --locked -p capture-diff"* ]] || die \
    "capture profile did not run the capture-diff tests"

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
  [[ "$full_dry_run_output" == *"test --locked -p capture-diff"* ]] || die \
    "full dry-run did not print the capture-diff command"

  rm -rf "$artifacts"
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
    prompt="Review only the committed patch in git diff ${merge_base}..HEAD. Use read-only commands to inspect the repository and relevant C++ reference sources. Do not review uncommitted files. Return the structured review required by the output schema."
  else
    prompt="Review all staged, unstaged, and untracked changes in this repository. Use read-only commands to inspect git diff, git diff --cached, and untracked files plus relevant C++ reference sources. Return the structured review required by the output schema."
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
  run_capture
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

case "$COMMAND" in
  self-test)
    (($# == 0)) || die "self-test does not accept arguments"
    run_self_test
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
  help)
    usage
    ;;
  *)
    usage >&2
    die "unknown command: $COMMAND"
    ;;
esac
