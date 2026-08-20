#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEFAULT_BASE="origin/3.4.3"
DEFAULT_RUST_MIN_STACK=1073741824
MODE="${1:-}"
BASE="${2:-$DEFAULT_BASE}"
DRY_RUN="${LOCAL_HARNESS_DRY_RUN:-0}"

usage() {
  cat <<'EOF'
RustyCore lightweight local development harness

Non-interactive and agent-agnostic: humans, Kimi, Codex, Grok, Claude, and other
agents use the same commands and receive the same exit status.

Usage:
  ./tools/local-harness.sh quick [BASE]
  ./tools/local-harness.sh final [BASE]
  ./tools/local-harness.sh self-test
  ./tools/local-harness.sh --help

quick
  Whitespace, changed JSON/shell files, Rust formatting, and cargo check only
  for directly affected workspace crates.

final
  Everything in quick plus library tests for directly affected crates. Slow
  workspace-wide inventories, live databases, capture QA, and Codex review are
  intentionally excluded from the daily development path.

BASE defaults to origin/3.4.3.
Set LOCAL_HARNESS_DRY_RUN=1 to print routed commands without executing them.
EOF
}

log() {
  printf '\n==> %s\n' "$*"
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

run() {
  print_command "$@"
  if [[ "$DRY_RUN" != "1" ]]; then
    "$@"
  fi
}

assert_eq() {
  local actual="$1"
  local expected="$2"
  local label="$3"
  [[ "$actual" == "$expected" ]] || die \
    "self-test failed for $label: expected '$expected', got '$actual'"
}

path_kind() {
  local path="$1"
  case "$path" in
    crates/*) printf 'workspace-rust' ;;
    tools/architecture/handler-contract-check/*) printf 'architecture-checker' ;;
    tools/wow-test-bot/*) printf 'wow-test-bot' ;;
    *.rs|Cargo.toml|Cargo.lock|rust-toolchain.toml|.protoc-version|proto/*|*/build.rs)
      printf 'workspace-rust'
      ;;
    *.sh) printf 'shell' ;;
    *.json) printf 'json' ;;
    .github/workflows/*.yml|.github/workflows/*.yaml) printf 'workflow' ;;
    *) printf 'other' ;;
  esac
}

self_test() {
  assert_eq "$(path_kind docs/migration/STATE.md)" other "documentation routing"
  assert_eq "$(path_kind crates/wow-world/src/session.rs)" workspace-rust \
    "workspace crate routing"
  assert_eq "$(path_kind tools/architecture/handler-contract-check/src/lib.rs)" \
    architecture-checker "standalone architecture checker routing"
  assert_eq "$(path_kind tools/wow-test-bot/src/main.rs)" wow-test-bot \
    "standalone bot routing"
  assert_eq "$(path_kind tools/pr-preflight.sh)" shell "shell routing"
  assert_eq "$(path_kind tools/architecture/session-ownership-policy.json)" json \
    "JSON routing"
  assert_eq "$(path_kind .github/workflows/rust-ci.yml)" workflow \
    "workflow routing"
  log "Local harness self-test passed"
}

if [[ "$MODE" == "--help" || "$MODE" == "-h" ]]; then
  usage
  exit 0
fi

if [[ "$MODE" == "self-test" ]]; then
  (($# == 1)) || die "self-test does not accept a base"
  self_test
  exit 0
fi

[[ "$MODE" == "quick" || "$MODE" == "final" ]] || {
  usage >&2
  exit 64
}
(($# <= 2)) || die "too many arguments"

RUST_MIN_STACK="${RUST_MIN_STACK:-$DEFAULT_RUST_MIN_STACK}"
[[ "$RUST_MIN_STACK" =~ ^[1-9][0-9]*$ ]] || die \
  "RUST_MIN_STACK must be a positive integer"
((RUST_MIN_STACK >= DEFAULT_RUST_MIN_STACK)) || die \
  "RUST_MIN_STACK must be at least $DEFAULT_RUST_MIN_STACK bytes for Rust 1.88"
export RUST_MIN_STACK

cd "$REPO_ROOT"
git rev-parse --is-inside-work-tree >/dev/null 2>&1 || die "not inside a git worktree"
git rev-parse --verify "$BASE^{commit}" >/dev/null 2>&1 || die \
  "base '$BASE' is unavailable; fetch it or pass another base"

merge_base="$(git merge-base "$BASE" HEAD)"
mapfile -t changed_files < <(
  {
    git diff --name-only "$merge_base"..HEAD
    git diff --name-only --cached
    git diff --name-only
    git ls-files --others --exclude-standard
  } | awk 'NF' | sort -u
)

if ((${#changed_files[@]} == 0)); then
  log "No changes relative to $BASE"
  exit 0
fi

log "Changed paths (${#changed_files[@]})"
printf '  %s\n' "${changed_files[@]}"

log "Whitespace checks"
run git diff --check "$merge_base"..HEAD
run git diff --check --cached
run git diff --check

declare -A crate_dirs=()
declare -a shell_files=()
declare -a json_files=()
workspace_rust=0
root_rust_scope=0
architecture_checker=0
wow_test_bot=0
workflow_changed=0
local_harness_changed=0

for path in "${changed_files[@]}"; do
  [[ "$path" == "tools/local-harness.sh" ]] && local_harness_changed=1
  case "$(path_kind "$path")" in
    workspace-rust)
      workspace_rust=1
      if [[ "$path" == crates/*/* ]]; then
        remainder="${path#crates/}"
        crate_dirs["${remainder%%/*}"]=1
      else
        root_rust_scope=1
      fi
      ;;
    architecture-checker) architecture_checker=1 ;;
    wow-test-bot) wow_test_bot=1 ;;
    shell)
      [[ -f "$path" ]] && shell_files+=("$path")
      ;;
    json)
      [[ -f "$path" ]] && json_files+=("$path")
      ;;
    workflow) workflow_changed=1 ;;
  esac
done

if ((local_harness_changed == 1)); then
  self_test
fi

if ((${#shell_files[@]} > 0)); then
  log "Changed shell syntax"
  for path in "${shell_files[@]}"; do
    run bash -n "$path"
  done
fi

if ((${#json_files[@]} > 0)); then
  command -v jq >/dev/null 2>&1 || die "jq is required to validate changed JSON"
  log "Changed JSON syntax"
  for path in "${json_files[@]}"; do
    run jq empty "$path"
  done
fi

if ((workflow_changed == 1)); then
  log "Changed workflow syntax"
  if command -v actionlint >/dev/null 2>&1; then
    run actionlint
  else
    printf 'warning: actionlint is not installed; workflow execution remains disabled for trusted PRs\n' >&2
  fi
fi

if ((workspace_rust == 1 || architecture_checker == 1 || wow_test_bot == 1)); then
  if [[ -x /home/cdmonio/.local/protoc/bin/protoc ]]; then
    export PROTOC=/home/cdmonio/.local/protoc/bin/protoc
  elif command -v protoc >/dev/null 2>&1; then
    export PROTOC="$(command -v protoc)"
  fi
fi

if ((workspace_rust == 1)); then
  log "Workspace Rust formatting"
  run cargo +1.88.0 fmt --all --check
fi

if ((architecture_checker == 1)); then
  log "Architecture checker formatting and fast tests"
  run cargo +1.88.0 fmt \
    --manifest-path tools/architecture/handler-contract-check/Cargo.toml -- --check
  run cargo +1.88.0 test --locked \
    --manifest-path tools/architecture/handler-contract-check/Cargo.toml --lib -- \
    --skip repository_surface_can_be_collected
fi

if ((wow_test_bot == 1)); then
  log "QA bot formatting and check"
  run cargo +1.88.0 fmt --manifest-path tools/wow-test-bot/Cargo.toml -- --check
  run cargo +1.88.0 check --locked --manifest-path tools/wow-test-bot/Cargo.toml
fi

declare -A packages=()
metadata=''
if ((workspace_rust == 1)); then
  metadata="$(cargo metadata --no-deps --format-version 1)"
  for crate_dir in "${!crate_dirs[@]}"; do
    manifest="$REPO_ROOT/crates/$crate_dir/Cargo.toml"
    [[ -f "$manifest" ]] || continue
    package="$(jq -r --arg manifest "$manifest" \
      '.packages[] | select(.manifest_path == $manifest) | .name' <<<"$metadata")"
    [[ -n "$package" ]] || die "cannot map crates/$crate_dir to a Cargo package"
    packages["$package"]=1
  done
  if ((root_rust_scope == 1)); then
    packages[bnet-server]=1
    packages[world-server]=1
  fi
fi

if ((${#packages[@]} > 0)); then
  mapfile -t package_names < <(printf '%s\n' "${!packages[@]}" | sort)
  cargo_package_args=()
  for package in "${package_names[@]}"; do
    cargo_package_args+=(--package "$package")
  done
  log "Cargo check for affected packages: ${package_names[*]}"
  run cargo +1.88.0 check --locked "${cargo_package_args[@]}"

  if [[ "$MODE" == "final" ]]; then
    for package in "${package_names[@]}"; do
      has_lib="$(jq -r --arg package "$package" '
        [.packages[] | select(.name == $package) | .targets[].kind[]]
        | any(. == "lib" or . == "proc-macro")
      ' <<<"$metadata")"
      if [[ "$has_lib" == "true" ]]; then
        log "Library tests for affected package: $package"
        run cargo +1.88.0 test --locked --package "$package" --lib
      else
        log "No library target for $package; cargo check is the final local gate"
      fi
    done
  fi
fi

if [[ "$MODE" == "final" ]]; then
  log "Final local harness passed"
else
  log "Quick local harness passed"
fi
