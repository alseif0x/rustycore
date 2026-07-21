#!/usr/bin/env bash
# Source-only PM2/PID/listener accreditation shared by capture wrapper tests.
# Callers define distinct CAPTURE_WORLD_PORT and CAPTURE_INSTANCE_PORT.

CAPTURE_WORLD_STOP_TIMEOUT_SECONDS="${CAPTURE_WORLD_STOP_TIMEOUT_SECONDS:-30}"
CAPTURE_WORLD_READY_TIMEOUT_SECONDS="${CAPTURE_WORLD_READY_TIMEOUT_SECONDS:-180}"
CAPTURE_WORLD_TIMEOUT_MAX_SECONDS=3600

capture_validate_world_timeouts() {
  local name value minimum
  for name in \
    CAPTURE_WORLD_STOP_TIMEOUT_SECONDS \
    CAPTURE_WORLD_READY_TIMEOUT_SECONDS; do
    value="${!name:-}"
    minimum=1
    [ "$name" != CAPTURE_WORLD_READY_TIMEOUT_SECONDS ] || minimum=3
    if [[ ! "$value" =~ ^[1-9][0-9]*$ ]] \
        || ((${#value} > 4)) \
        || ((10#$value < minimum)) \
        || ((10#$value > CAPTURE_WORLD_TIMEOUT_MAX_SECONDS)); then
      echo "error: ${name} must be an integer from ${minimum} through ${CAPTURE_WORLD_TIMEOUT_MAX_SECONDS}" >&2
      return 1
    fi
  done
}

capture_fixture_cleanup_verified_for_publication() {
  local guard_enabled="$1"
  local cleanup_verified="$2"

  case "$guard_enabled:$cleanup_verified" in
    0:0|0:1|1:1) return 0 ;;
    *) return 1 ;;
  esac
}

capture_pm2_online_pid() {
  local process_name="$1"
  pm2 jlist | jq -er --arg name "$process_name" '
    [.[] | select(.name == $name)] as $entries
    | if ($entries | length) == 1
        and $entries[0].pm2_env.status == "online"
        and ($entries[0].pid | type) == "number"
        and $entries[0].pid > 0
      then $entries[0].pid
      else empty
      end
  '
}

capture_pm2_runtime_metadata() {
  local process_name="$1"
  local expected_pm2_pid="$2"
  pm2 jlist | jq -er --arg name "$process_name" --argjson pid "$expected_pm2_pid" '
    [.[] | select(.name == $name)] as $entries
    | if ($entries | length) == 1
        and $entries[0].pm2_env.status == "online"
        and $entries[0].pid == $pid
        and (($entries[0].pm2_env.restart_time // 0) | type) == "number"
        and ($entries[0].pm2_env.restart_time // 0) >= 0
        and ($entries[0].pm2_env.pm_exec_path | type) == "string"
        and ($entries[0].pm2_env.pm_exec_path | length) > 0
      then [$entries[0].pid, ($entries[0].pm2_env.restart_time // 0), $entries[0].pm2_env.pm_exec_path] | @tsv
      else empty
      end
  '
}

capture_parent_pid() {
  local runtime_pid="$1"
  local parent_pid
  [[ "$runtime_pid" =~ ^[1-9][0-9]*$ ]] || return 1
  parent_pid="$(awk '$1 == "PPid:" { print $2 }' \
    "${CAPTURE_PROC_ROOT:-/proc}/${runtime_pid}/status" 2>/dev/null)" \
    || return 1
  [[ "$parent_pid" =~ ^[0-9]+$ ]] && [ "$parent_pid" != "$runtime_pid" ] \
    || return 1
  printf '%s\n' "$parent_pid"
}

# PM2's reported PID is the configured entry process. For a direct binary it
# also owns the listeners; for shell wrappers which do not `exec` (the legacy
# C++ profile), the listener is a descendant. Walk the live process tree rather
# than assuming either topology.
capture_pid_is_self_or_descendant() {
  local candidate_pid="$1"
  local ancestor_pid="$2"
  local current_pid="$candidate_pid"
  local parent_pid attempt

  [[ "$candidate_pid" =~ ^[1-9][0-9]*$ \
    && "$ancestor_pid" =~ ^[1-9][0-9]*$ ]] || return 1
  for ((attempt = 0; attempt < 64; attempt++)); do
    [ "$current_pid" = "$ancestor_pid" ] && return 0
    parent_pid="$(capture_parent_pid "$current_pid")" || return 1
    [ "$parent_pid" != 0 ] || return 1
    current_pid="$parent_pid"
  done
  return 1
}

capture_pm2_process_stopped() {
  local process_name="$1"
  pm2 jlist | jq -e --arg name "$process_name" '
    [.[] | select(.name == $name)] as $entries
    | ($entries | length) == 1
      and $entries[0].pm2_env.status == "stopped"
      and (($entries[0].pid // 0) == 0)
  ' >/dev/null
}

capture_world_ports_owned_by_pid() {
  local pid="$1"
  [[ "$pid" =~ ^[1-9][0-9]*$ ]] || return 1
  capture_port_owned_exclusively_by_pid "$CAPTURE_WORLD_PORT" "$pid" \
    && capture_port_owned_exclusively_by_pid "$CAPTURE_INSTANCE_PORT" "$pid"
}

capture_unique_listener_pid_for_port() {
  local port="$1"
  local sockets socket remaining seen_pid matched_pid listener_pid=""
  sockets="$(ss -H -ltnp "sport = :${port}" 2>/dev/null)" || return 1
  [ -n "$sockets" ] || return 1

  while IFS= read -r socket; do
    remaining="$socket"
    seen_pid=0
    while [[ "$remaining" =~ pid=([0-9]+), ]]; do
      matched_pid="${BASH_REMATCH[1]}"
      [[ "$matched_pid" =~ ^[1-9][0-9]*$ ]] || return 1
      if [ -z "$listener_pid" ]; then
        listener_pid="$matched_pid"
      else
        [ "$matched_pid" = "$listener_pid" ] || return 1
      fi
      seen_pid=1
      remaining="${remaining#*"${BASH_REMATCH[0]}"}"
    done
    [ "$seen_pid" -eq 1 ] || return 1
  done <<<"$sockets"
  [ -n "$listener_pid" ] || return 1
  printf '%s\n' "$listener_pid"
}

capture_world_listener_pid() {
  local world_pid instance_pid
  world_pid="$(capture_unique_listener_pid_for_port "$CAPTURE_WORLD_PORT")" \
    || return 1
  instance_pid="$(capture_unique_listener_pid_for_port "$CAPTURE_INSTANCE_PORT")" \
    || return 1
  [ "$world_pid" = "$instance_pid" ] || return 1
  printf '%s\n' "$world_pid"
}

capture_port_owned_exclusively_by_pid() {
  local port="$1"
  local pid="$2"
  local sockets socket remaining seen_pid matched_pid
  sockets="$(ss -H -ltnp "sport = :${port}" 2>/dev/null)" || return 1
  [ -n "$sockets" ] || return 1

  # SO_REUSEPORT can expose more than one listener for the same local port.
  # Every row must therefore identify the accredited PID; finding merely one
  # matching row would let an additional process share the capture endpoint.
  while IFS= read -r socket; do
    remaining="$socket"
    seen_pid=0
    while [[ "$remaining" =~ pid=([0-9]+), ]]; do
      matched_pid="${BASH_REMATCH[1]}"
      [ "$matched_pid" = "$pid" ] || return 1
      seen_pid=1
      remaining="${remaining#*"${BASH_REMATCH[0]}"}"
    done
    [ "$seen_pid" -eq 1 ] || return 1
  done <<<"$sockets"
}

capture_sha256_of_file() {
  local output digest
  output="$(sha256sum <"$1")" || return 1
  digest="${output%% *}"
  [[ "$digest" =~ ^[0-9a-f]{64}$ ]] || return 1
  printf '%s\n' "$digest"
}

# Bind PM2's entry PID to the exact configured entrypoint file. For an
# interpreted wrapper this is the wrapper script from pm_exec_path (not
# /proc/<pid>/exe, which is the shell interpreter); the listener executable is
# accredited separately through /proc below.
capture_pm2_entrypoint_identity() {
  local process_name="$1"
  local expected_pm2_pid="$2"
  local metadata pm2_pid restart_count configured_path canonical_path digest

  metadata="$(capture_pm2_runtime_metadata "$process_name" "$expected_pm2_pid")" \
    || return 1
  IFS=$'\t' read -r pm2_pid restart_count configured_path <<<"$metadata"
  [ "$pm2_pid" = "$expected_pm2_pid" ] \
    && [[ "$restart_count" =~ ^[0-9]+$ ]] || return 1
  canonical_path="$(realpath -e -- "$configured_path" 2>/dev/null)" || return 1
  [ "$canonical_path" = "$configured_path" ] \
    && [ -f "$canonical_path" ] \
    && [ ! -L "$canonical_path" ] || return 1
  digest="$(capture_sha256_of_file "$canonical_path")" || return 1
  printf '%s\t%s\t%s\t%s\n' \
    "$pm2_pid" "$restart_count" "$canonical_path" "$digest"
}

# Hash the stable PM2 launch profile without serializing environment values.
# PID/status/restart counters are runtime state and intentionally excluded; the
# executable, cwd, interpreter, argv, and environment key names define how PM2
# will start the process. Secret values can therefore never enter the digest
# preimage emitted by this helper.
capture_pm2_profile_redacted_sha256() {
  local process_name="$1"
  local output digest
  output="$(pm2 jlist | jq -cSe --arg name "$process_name" '
    [.[] | select(.name == $name)] as $entries
    | if ($entries | length) != 1 then empty else $entries[0] end
    | {
        name: .name,
        pm_exec_path: .pm2_env.pm_exec_path,
        pm_cwd: (.pm2_env.pm_cwd // ""),
        exec_interpreter: (.pm2_env.exec_interpreter // ""),
        args: (.pm2_env.args // []),
        env_keys: ((.pm2_env.env // {}) | keys | sort)
      }
  ' | sha256sum)" || return 1
  digest="${output%% *}"
  [[ "$digest" =~ ^[0-9a-f]{64}$ ]] || return 1
  printf '%s\n' "$digest"
}

# Resolve the config actually selected by PM2. Prefer an exact -c/--config
# argument. For the legacy shell entrypoint, accept exactly one absolute
# literal config argument in the wrapper body; variable expansion or multiple
# candidates are ambiguous and fail closed.
_capture_pm2_effective_config_path_untraced() {
  local process_name="$1"
  local entry_json configured_path configured_cwd configured_exec canonical

  entry_json="$(pm2 jlist | jq -cSe --arg name "$process_name" '
    [.[] | select(.name == $name)] as $entries
    | if ($entries | length) == 1 then $entries[0] else empty end
  ')" || return 1
  configured_exec="$(jq -er '.pm2_env.pm_exec_path | select(type == "string" and length > 0)' <<<"$entry_json")" \
    || return 1
  configured_cwd="$(jq -er '.pm2_env.pm_cwd // "" | select(type == "string")' <<<"$entry_json")" \
    || return 1
  configured_path="$(jq -er '
    (.pm2_env.args // []) as $args
    | if ($args | type) != "array"
        or any($args[]; type != "string")
      then error("PM2 args must be an array of strings")
      else
        ([range(0; $args | length) as $i
          | if ($args[$i] == "-c" or $args[$i] == "--config")
              then if ($i + 1) < ($args | length)
                     then $args[$i + 1]
                     else error("config switch has no value")
                   end
            elif ($args[$i] | startswith("--config="))
              then ($args[$i] | sub("^--config="; ""))
            else empty end]
         | if length == 0 then "__CAPTURE_NO_CONFIG_ARG__"
           elif length == 1 and (.[0] | length) > 0 then .[0]
           else error("PM2 config selection is empty or ambiguous") end)
      end
  ' <<<"$entry_json" 2>/dev/null)" || return 1

  if [ "$configured_path" != "__CAPTURE_NO_CONFIG_ARG__" ]; then
    if [[ "$configured_path" != /* ]]; then
      [ -n "$configured_cwd" ] || return 1
      configured_path="$configured_cwd/$configured_path"
    fi
  else
    canonical="$(realpath -e -- "$configured_exec" 2>/dev/null)" || return 1
    [ "$canonical" = "$configured_exec" ] \
      && [ -f "$canonical" ] && [ ! -L "$canonical" ] || return 1
    configured_path="$(awk '
      /^[[:space:]]*#/ { next }
      {
        for (i = 1; i <= NF; i++) {
          if ($i == "-c" || $i == "--config") {
            if (i == NF) exit 2
            candidate = $(i + 1)
            gsub(/^["\047]|["\047]$/, "", candidate)
            found[++count] = candidate
          } else if ($i ~ /^--config=/) {
            candidate = $i
            sub(/^--config=/, "", candidate)
            gsub(/^["\047]|["\047]$/, "", candidate)
            found[++count] = candidate
          }
        }
      }
      END {
        if (count != 1 || found[1] !~ /^\/[A-Za-z0-9._\/-]+$/) exit 3
        print found[1]
      }
    ' "$canonical")" || return 1
  fi

  canonical="$(realpath -e -- "$configured_path" 2>/dev/null)" || return 1
  [ "$canonical" = "$configured_path" ] \
    && [ -f "$canonical" ] && [ ! -L "$canonical" ] || return 1
  printf '%s\n' "$canonical"
}

capture_pm2_effective_config_path() {
  local restore_xtrace=0 result status
  if [[ "$-" == *x* ]]; then
    restore_xtrace=1
    set +x
  fi
  result="$(_capture_pm2_effective_config_path_untraced "$@")"
  status=$?
  if [ "$restore_xtrace" -eq 1 ]; then
    set -x
  fi
  [ "$status" -eq 0 ] || return "$status"
  printf '%s\n' "$result"
}

capture_git_repo_clean_at_head() {
  local repository="$1"
  local expected_head="$2"
  local current_head status

  current_head="$(git -C "$repository" rev-parse HEAD 2>/dev/null)" || return 1
  [ "$current_head" = "$expected_head" ] || return 1
  status="$(git -C "$repository" status --porcelain=v1 \
    --untracked-files=all --ignore-submodules=none 2>/dev/null)" || return 1
  [ -z "$status" ]
}

capture_git_repo_is_dirty() {
  local repository="$1"
  local status
  status="$(git -C "$repository" status --porcelain=v1 \
    --untracked-files=all --ignore-submodules=none 2>/dev/null)" || return 1
  [ -n "$status" ]
}

# Fingerprint the complete non-ignored worktree state without putting file
# contents into a manifest. The canonical stream contains HEAD plus each
# changed/untracked path, file type/mode, and a SHA-256 of its current bytes.
# Ignored runtime configs/credentials are deliberately outside Git's reported
# state. Special files and submodules fail closed rather than receiving an
# ambiguous identity.
capture_git_worktree_state_sha256() {
  local repository="$1"
  local head output digest path absolute kind mode content_sha

  head="$(git -C "$repository" rev-parse HEAD 2>/dev/null)" || return 1
  output="$({
    printf 'HEAD\0%s\0' "$head"
    {
      git -C "$repository" diff --name-only -z HEAD -- || exit 1
      git -C "$repository" ls-files --others --exclude-standard -z || exit 1
    } | LC_ALL=C sort -zu | while IFS= read -r -d '' path; do
      absolute="$repository/$path"
      if [ -L "$absolute" ]; then
        kind=L
        mode="$(stat -c '%a' -- "$absolute")" || exit 1
        content_sha="$(readlink -- "$absolute" | sha256sum)" || exit 1
        content_sha="${content_sha%% *}"
      elif [ -f "$absolute" ]; then
        kind=F
        mode="$(stat -c '%a' -- "$absolute")" || exit 1
        content_sha="$(capture_sha256_of_file "$absolute")" || exit 1
      elif [ ! -e "$absolute" ]; then
        kind=D
        mode=-
        content_sha=-
      else
        exit 1
      fi
      printf '%s\0%s\0%s\0%s\0' "$kind" "$path" "$mode" "$content_sha"
    done
  } | sha256sum)" || return 1
  digest="${output%% *}"
  [[ "$digest" =~ ^[0-9a-f]{64}$ ]] || return 1
  printf '%s\n' "$digest"
}

# Hash only an explicit ordered set of capture-relevant effective config keys.
# The last assignment wins, matching the server config parser. Values for any
# credential-like key are replaced with the literal <redacted-present>; missing
# keys become <unset>. `extra_canonical` contains only caller-owned non-secret
# facts such as listener ports and whether packet dumping is enabled. The
# canonical plaintext is piped directly into sha256sum and is never printed or
# stored, so the digest cannot be used to dictionary-attack DB credentials.
capture_effective_config_redacted_sha256() {
  local config_file="$1"
  local extra_canonical="$2"
  shift 2
  local wanted="$*"
  local output digest

  [ -f "$config_file" ] && [ ! -L "$config_file" ] || return 1
  output="$({
    awk -v wanted="$wanted" '
      function trim(value) {
        sub(/^[[:space:]]+/, "", value)
        sub(/[[:space:]]+$/, "", value)
        return value
      }
      /^[[:space:]]*[#;]/ { next }
      {
        separator = index($0, "=")
        if (separator == 0) next
        key = trim(substr($0, 1, separator - 1))
        value = trim(substr($0, separator + 1))
        values[key] = value
      }
      END {
        count = split(wanted, keys, " ")
        for (i = 1; i <= count; i++) {
          key = keys[i]
          if (key == "") continue
          if (!(key in values)) value = "<unset>"
          else if (key ~ /(DatabaseInfo|Password|Secret|Token|PrivateKey|SessionKey)/)
            value = "<redacted-present>"
          else value = values[key]
          printf "%s=%s\n", key, value
        }
      }
    ' "$config_file" || exit 1
    printf '%s\n' "$extra_canonical"
  } | sha256sum)" || return 1
  digest="${output%% *}"
  [[ "$digest" =~ ^[0-9a-f]{64}$ ]] || return 1
  printf '%s\n' "$digest"
}

capture_exec_source_matches() {
  local expected_exec="$1"
  local expected_sha="$2"
  local canonical digest

  canonical="$(realpath -e -- "$expected_exec" 2>/dev/null)" || return 1
  [ "$canonical" = "$expected_exec" ] || return 1
  [ -f "$expected_exec" ] && [ -x "$expected_exec" ] \
    && [ ! -L "$expected_exec" ] || return 1
  digest="$(capture_sha256_of_file "$expected_exec")" || return 1
  [ "$digest" = "$expected_sha" ]
}

capture_live_exec_matches() {
  local pid="$1"
  local expected_exec="$2"
  local expected_sha="$3"
  local proc_exe live_exec source_sha live_sha

  [[ "$pid" =~ ^[1-9][0-9]*$ ]] || return 1
  proc_exe="/proc/${pid}/exe"
  [ -L "$proc_exe" ] || return 1
  live_exec="$(realpath -e -- "$proc_exe" 2>/dev/null)" || return 1
  [ "$live_exec" = "$expected_exec" ] || return 1
  source_sha="$(capture_sha256_of_file "$expected_exec")" || return 1
  live_sha="$(capture_sha256_of_file "$proc_exe")" || return 1
  [ "$source_sha" = "$expected_sha" ] && [ "$live_sha" = "$expected_sha" ]
}

capture_acquire_orchestration_lock() {
  local lock_dir="$1"
  local parent canonical_parent canonical_dir owner mode before after fd_target
  local path_after fd_target_after

  [[ "$lock_dir" = /* && "$lock_dir" != *$'\n'* ]] || return 1
  parent="$(dirname -- "$lock_dir")"
  canonical_parent="$(realpath -e -- "$parent" 2>/dev/null)" || return 1
  [ "$canonical_parent" = "$parent" ] || return 1
  if [ ! -e "$lock_dir" ] && [ ! -L "$lock_dir" ]; then
    mkdir -m 700 -- "$lock_dir" || return 1
  fi
  [ -d "$lock_dir" ] && [ ! -L "$lock_dir" ] || return 1
  canonical_dir="$(realpath -e -- "$lock_dir" 2>/dev/null)" || return 1
  [ "$canonical_dir" = "$lock_dir" ] || return 1
  owner="$(stat -c '%u' -- "$lock_dir")" || return 1
  mode="$(stat -c '%a' -- "$lock_dir")" || return 1
  [ "$owner" = "$(id -u)" ] && [ "$mode" = 700 ] || return 1
  before="$(stat -c '%d:%i' -- "$lock_dir")" || return 1
  exec {CAPTURE_ORCHESTRATION_LOCK_FD}<"$lock_dir" || return 1
  fd_target="$(realpath -e -- "/proc/${BASHPID:-$$}/fd/${CAPTURE_ORCHESTRATION_LOCK_FD}" 2>/dev/null)" \
    || return 1
  after="$(stat -Lc '%d:%i' -- "/proc/${BASHPID:-$$}/fd/${CAPTURE_ORCHESTRATION_LOCK_FD}")" \
    || return 1
  [ "$fd_target" = "$lock_dir" ] && [ "$before" = "$after" ] || {
    exec {CAPTURE_ORCHESTRATION_LOCK_FD}>&-
    CAPTURE_ORCHESTRATION_LOCK_FD=""
    return 1
  }
  flock -n "$CAPTURE_ORCHESTRATION_LOCK_FD" || {
    exec {CAPTURE_ORCHESTRATION_LOCK_FD}>&-
    CAPTURE_ORCHESTRATION_LOCK_FD=""
    return 1
  }
  path_after="$(stat -c '%d:%i' -- "$lock_dir" 2>/dev/null)" || path_after=""
  fd_target_after="$(realpath -e -- "/proc/${BASHPID:-$$}/fd/${CAPTURE_ORCHESTRATION_LOCK_FD}" 2>/dev/null)" \
    || fd_target_after=""
  [ "$path_after" = "$before" ] && [ "$fd_target_after" = "$lock_dir" ] || {
    flock -u "$CAPTURE_ORCHESTRATION_LOCK_FD" 2>/dev/null || true
    exec {CAPTURE_ORCHESTRATION_LOCK_FD}>&-
    CAPTURE_ORCHESTRATION_LOCK_FD=""
    return 1
  }
}

capture_require_canonical_directory() {
  local directory="$1"
  local canonical
  [[ "$directory" = /* && "$directory" != *$'\n'* ]] || return 1
  mkdir -p -- "$directory" || return 1
  [ -d "$directory" ] && [ ! -L "$directory" ] || return 1
  canonical="$(realpath -e -- "$directory" 2>/dev/null)" || return 1
  [ "$canonical" = "$directory" ]
}

capture_publish_noreplace() {
  local source="$1"
  local target="$2"
  local source_identity target_identity
  [ -e "$source" ] && [ ! -L "$source" ] || return 1
  [ ! -e "$target" ] && [ ! -L "$target" ] || return 1
  source_identity="$(stat -c '%d:%i' -- "$source")" || return 1
  mv --no-clobber --no-target-directory -- "$source" "$target" || return 1
  [ ! -e "$source" ] && [ ! -L "$source" ] || return 1
  [ -e "$target" ] && [ ! -L "$target" ] || return 1
  target_identity="$(stat -c '%d:%i' -- "$target")" || return 1
  [ "$target_identity" = "$source_identity" ]
}

capture_vendor_report_proves_exact_success() {
  local report_path="$1"

  jq -e '
    .vendor_smoke == true
    and .loot_item_capture == false
    and .loot_race_smoke == false
    and (.results | type == "array" and length == 1)
    and (.results[0]
      | .account == "TESTBOT2@bot.local"
      and .account_id == 9
      and .character_guid == 15
      and .world_auth == true
      and .enum_characters == true
      and .player_login_verified == true
      and .vendor_smoke == true
      and .vendor_smoke_passed == true
      and .vendor_entry == 18525
      and .vendor_spawn_guid == 96654
      and (.vendor_runtime_counter | type == "number" and . > 0)
      and .vendor_item_entry == 30183
      and .vendor_extended_cost == 1642
      and .vendor_currency_id == 42
      and .vendor_currency_before == 30
      and .vendor_currency_after == 15
      and .vendor_item_total_after == 1
      and .vendor_inventory_seen == true
      and .vendor_buy_succeeded_seen == true
      and .vendor_set_currency_seen == true
      and .vendor_item_push_seen == true
      and .vendor_relogin_verified == true
      and .vendor_failure == null)
  ' "$report_path" >/dev/null
}

capture_vendor_bot_evidence() {
  local report_path="$1"
  local bot_exec="$2"
  local expected_bot_sha="$3"
  local canonical_report canonical_exec report_sha bot_sha

  [[ "$report_path" = /* && "$bot_exec" = /* \
    && "$expected_bot_sha" =~ ^[0-9a-f]{64}$ ]] || return 1
  canonical_report="$(realpath -e -- "$report_path" 2>/dev/null)" || return 1
  canonical_exec="$(realpath -e -- "$bot_exec" 2>/dev/null)" || return 1
  [ "$canonical_report" = "$report_path" ] \
    && [ -f "$report_path" ] && [ ! -L "$report_path" ] \
    && [ "$canonical_exec" = "$bot_exec" ] \
    && [ -f "$bot_exec" ] && [ -x "$bot_exec" ] && [ ! -L "$bot_exec" ] \
    || return 1
  bot_sha="$(capture_sha256_of_file "$bot_exec")" || return 1
  [ "$bot_sha" = "$expected_bot_sha" ] || return 1
  capture_vendor_report_proves_exact_success "$report_path" || return 1
  report_sha="$(capture_sha256_of_file "$report_path")" || return 1
  printf '%s\t%s\t%s\t%s\n' \
    "$canonical_exec" "$bot_sha" "$canonical_report" "$report_sha"
}

capture_validate_fresh_bot_inputs() {
  local bot_exec="$1"
  local expected_bot_sha="$2"
  local report_path="$3"
  local report_parent

  [ -n "$bot_exec" ] && [ -n "$expected_bot_sha" ] \
    && [ -n "$report_path" ] || return 1
  [[ "$expected_bot_sha" =~ ^[0-9a-f]{64}$ ]] || return 1
  [[ "$report_path" = /* && "$report_path" != *$'\n'* ]] \
    && [ -d "$(dirname -- "$report_path")" ] \
    && [ ! -e "$report_path" ] && [ ! -L "$report_path" ] || return 1
  report_parent="$(dirname -- "$report_path")"
  [ "$(realpath -e -- "$report_parent" 2>/dev/null)" = "$report_parent" ] \
    && [ ! -L "$report_parent" ] || return 1
  capture_exec_source_matches "$bot_exec" "$expected_bot_sha"
}

capture_loot_item_bot_evidence() {
  local report_path="$1"
  local bot_exec="$2"
  local expected_bot_sha="$3"
  local canonical_report canonical_exec report_sha bot_sha account_id

  [[ "$report_path" = /* && "$bot_exec" = /* \
    && "$expected_bot_sha" =~ ^[0-9a-f]{64}$ ]] || return 1
  canonical_report="$(realpath -e -- "$report_path" 2>/dev/null)" || return 1
  canonical_exec="$(realpath -e -- "$bot_exec" 2>/dev/null)" || return 1
  [ "$canonical_report" = "$report_path" ] \
    && [ -f "$report_path" ] && [ ! -L "$report_path" ] \
    && [ "$canonical_exec" = "$bot_exec" ] \
    && [ -f "$bot_exec" ] && [ -x "$bot_exec" ] && [ ! -L "$bot_exec" ] \
    || return 1
  bot_sha="$(capture_sha256_of_file "$bot_exec")" || return 1
  [ "$bot_sha" = "$expected_bot_sha" ] || return 1
  account_id="$(jq -er '
    if .loot_item_capture != true
        or .loot_race_smoke != false
        or (.results | type) != "array"
        or (.results | length) != 1
      then empty
      else .results[0]
      | select(
          .account == "TESTBOT2@bot.local"
          and .account_id == 9
          and .character_guid == 15
          and .world_auth == true
          and .enum_characters == true
          and .player_login_verified == true
          and .loot_race_smoke == true
          and .loot_race_smoke_passed == true
          and .loot_race_target_entry == 21779
          and .loot_race_target_spawn_guid == 1117
          and .loot_race_target_discovered == true
          and .loot_race_loot_opened == true
          and .loot_race_item_push_seen == true
          and .loot_race_loot_removed_seen == true
          and .loot_race_loot_coins == 0
          and .loot_race_coin_removed_seen == false
          and .loot_race_db_item_total == 1
          and .loot_race_db_money_delta == 0
          and .loot_race_relog_verified == true
          and .loot_race_failure == null)
      | .account_id
    end
  ' "$report_path")" || return 1
  [ "$account_id" = 9 ] || return 1
  report_sha="$(capture_sha256_of_file "$report_path")" || return 1
  printf '%s\t%s\t%s\t%s\n' \
    "$canonical_exec" "$bot_sha" "$canonical_report" "$report_sha"
}

capture_loot_race_report_proves_exact_success() {
  local report_path="$1"

  jq -e '
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
  ' "$report_path" >/dev/null
}

capture_loot_race_bot_evidence() {
  local report_path="$1"
  local bot_exec="$2"
  local expected_bot_sha="$3"
  local canonical_report canonical_exec report_sha bot_sha

  [[ "$report_path" = /* && "$bot_exec" = /* \
    && "$expected_bot_sha" =~ ^[0-9a-f]{64}$ ]] || return 1
  canonical_report="$(realpath -e -- "$report_path" 2>/dev/null)" || return 1
  canonical_exec="$(realpath -e -- "$bot_exec" 2>/dev/null)" || return 1
  [ "$canonical_report" = "$report_path" ] \
    && [ -f "$report_path" ] && [ ! -L "$report_path" ] \
    && [ "$canonical_exec" = "$bot_exec" ] \
    && [ -f "$bot_exec" ] && [ -x "$bot_exec" ] && [ ! -L "$bot_exec" ] \
    || return 1
  bot_sha="$(capture_sha256_of_file "$bot_exec")" || return 1
  [ "$bot_sha" = "$expected_bot_sha" ] || return 1
  capture_loot_race_report_proves_exact_success "$report_path" || return 1
  report_sha="$(capture_sha256_of_file "$report_path")" || return 1
  printf '%s\t%s\t%s\t%s\n' \
    "$canonical_exec" "$bot_sha" "$canonical_report" "$report_sha"
}

capture_bot_manifest_evidence() {
  local flow="$1"
  local bot_exec="$2"
  local bot_exec_sha256="$3"
  local bot_report="$4"
  local bot_report_sha256="$5"

  case "$flow" in
    loot-single-item-claim)
      jq -n \
        --arg exec_path "$bot_exec" \
        --arg exec_sha256 "$bot_exec_sha256" \
        --arg report_path "$bot_report" \
        --arg report_sha256 "$bot_report_sha256" '
          {
            fixture_guard: {
              enabled: true,
              contract: "loot-single-item-claim-fixture-v1",
              account: "TESTBOT2@bot.local",
              account_id: 9,
              character_guid: 15,
              peer_account: "TESTBOT3@bot.local",
              peer_account_id: 10,
              peer_character_guid: 16,
              creature_entry: 21779,
              creature_spawn_guid: 1117,
              item_entry: 30712,
              cleanup_verified: true
            },
            bot_report: {
              contract: "wow-test-bot-loot-item-capture-report-v1",
              exec_path: $exec_path,
              exec_sha256: $exec_sha256,
              report_path: $report_path,
              report_sha256: $report_sha256,
              account: "TESTBOT2@bot.local",
              account_id: 9,
              character_guid: 15,
              report_validated: true
            }
          }
        '
      ;;
    loot-two-session-atomic-race)
      jq -n \
        --arg exec_path "$bot_exec" \
        --arg exec_sha256 "$bot_exec_sha256" \
        --arg report_path "$bot_report" \
        --arg report_sha256 "$bot_report_sha256" '
          {
            fixture_guard: {
              enabled: true,
              contract: "loot-two-session-atomic-race-fixture-v1",
              account: "TESTBOT2@bot.local",
              account_id: 9,
              character_guid: 15,
              peer_account: "TESTBOT3@bot.local",
              peer_account_id: 10,
              peer_character_guid: 16,
              gameobject_entry: 2846,
              gameobject_spawn_guid: 9106001,
              item_entry: 38,
              cleanup_verified: true
            },
            bot_report: {
              contract: "wow-test-bot-loot-two-session-atomic-race-report-v1",
              exec_path: $exec_path,
              exec_sha256: $exec_sha256,
              report_path: $report_path,
              report_sha256: $report_sha256,
              account: "TESTBOT2@bot.local",
              account_id: 9,
              character_guid: 15,
              report_validated: true
            }
          }
        '
      ;;
    vendor-extended-cost-purchase)
      jq -n \
        --arg exec_path "$bot_exec" \
        --arg exec_sha256 "$bot_exec_sha256" \
        --arg report_path "$bot_report" \
        --arg report_sha256 "$bot_report_sha256" '
          {
            fixture_guard: null,
            bot_report: {
              contract: "wow-test-bot-vendor-extended-cost-purchase-report-v1",
              exec_path: $exec_path,
              exec_sha256: $exec_sha256,
              report_path: $report_path,
              report_sha256: $report_sha256,
              account: "TESTBOT2@bot.local",
              account_id: 9,
              character_guid: 15,
              report_validated: true
            }
          }
        '
      ;;
    *)
      printf '%s\n' '{"fixture_guard":null,"bot_report":null}'
      ;;
  esac
}

capture_release_orchestration_lock() {
  if [[ "${CAPTURE_ORCHESTRATION_LOCK_FD:-}" =~ ^[0-9]+$ ]]; then
    flock -u "$CAPTURE_ORCHESTRATION_LOCK_FD" 2>/dev/null || true
    exec {CAPTURE_ORCHESTRATION_LOCK_FD}>&- || true
  fi
  CAPTURE_ORCHESTRATION_LOCK_FD=""
}

capture_world_ports_absent() {
  local sockets
  sockets="$(ss -H -ltn)" || return 1
  ! rg -q ":(${CAPTURE_WORLD_PORT}|${CAPTURE_INSTANCE_PORT})\\b" <<<"$sockets"
}

capture_pid_starttime() {
  local pid="$1"
  local stat_line remainder
  local -a fields
  [[ "$pid" =~ ^[1-9][0-9]*$ ]] || return 1
  stat_line="$(<"${CAPTURE_PROC_ROOT:-/proc}/${pid}/stat")" || return 1
  [[ "$stat_line" == *") "* ]] || return 1
  remainder="${stat_line##*) }"
  read -r -a fields <<<"$remainder"
  # remainder starts at proc field 3; array index 19 is field 22/starttime.
  [ "${#fields[@]}" -ge 20 ] || return 1
  [[ "${fields[19]}" =~ ^[1-9][0-9]*$ ]] || return 1
  printf '%s\n' "${fields[19]}"
}

capture_pid_identity() {
  local pid="$1"
  local starttime
  starttime="$(capture_pid_starttime "$pid")" || return 1
  printf '%s:%s\n' "$pid" "$starttime"
}

capture_pid_identity_is_live() {
  local identity="$1"
  local pid expected actual
  IFS=: read -r pid expected <<<"$identity"
  [[ "$pid" =~ ^[1-9][0-9]*$ && "$expected" =~ ^[1-9][0-9]*$ ]] || return 1
  actual="$(capture_pid_starttime "$pid" 2>/dev/null)" || return 1
  [ "$actual" = "$expected" ] && kill -0 -- "$pid" 2>/dev/null
}

capture_pid_identity_absent() {
  ! capture_pid_identity_is_live "$1"
}

capture_process_tree_identity() {
  local root_pid="$1"
  local proc_dir pid parent identity
  local -a descendants=("$root_pid")
  local changed=1 candidate known
  [[ "$root_pid" =~ ^[1-9][0-9]*$ ]] || return 1

  while ((changed)); do
    changed=0
    for proc_dir in "${CAPTURE_PROC_ROOT:-/proc}"/[0-9]*; do
      [ -d "$proc_dir" ] || continue
      pid="${proc_dir##*/}"
      parent="$(capture_parent_pid "$pid" 2>/dev/null)" || continue
      candidate=0
      for known in "${descendants[@]}"; do
        if [ "$parent" = "$known" ]; then
          candidate=1
          break
        fi
      done
      [ "$candidate" -eq 1 ] || continue
      candidate=1
      for known in "${descendants[@]}"; do
        if [ "$pid" = "$known" ]; then
          candidate=0
          break
        fi
      done
      if [ "$candidate" -eq 1 ]; then
        descendants+=("$pid")
        changed=1
      fi
    done
  done

  for pid in "${descendants[@]}"; do
    identity="$(capture_pid_identity "$pid")" || return 1
    printf '%s\n' "$identity"
  done | sort -t: -k1,1n
}

capture_process_tree_absent() {
  local identities="$1"
  local identity
  while IFS= read -r identity; do
    [ -z "$identity" ] && continue
    capture_pid_identity_absent "$identity" || return 1
  done <<<"$identities"
}

capture_terminate_process_tree() {
  local identities="$1"
  local signal identity pid attempt
  for signal in TERM KILL; do
    while IFS= read -r identity; do
      [ -z "$identity" ] && continue
      if capture_pid_identity_is_live "$identity"; then
        pid="${identity%%:*}"
        kill -s "$signal" -- "$pid" 2>/dev/null || true
      fi
    done < <(sort -t: -k1,1nr <<<"$identities")
    for ((attempt = 0; attempt < 20; attempt++)); do
      capture_process_tree_absent "$identities" && return 0
      sleep 0.1
    done
  done
  capture_process_tree_absent "$identities"
}

capture_saved_pid_absent() {
  local pid="$1"
  [ -z "$pid" ] || ! kill -0 -- "$pid" 2>/dev/null
}

capture_saved_identity_absent() {
  local identity="$1"
  local pm2_pid listener_pid
  if [[ "$identity" == *:* ]]; then
    capture_process_tree_absent "$identity"
    return
  fi
  if [[ "$identity" == *$'\t'* ]]; then
    IFS=$'\t' read -r pm2_pid listener_pid <<<"$identity"
  else
    pm2_pid="$identity"
    listener_pid="$identity"
  fi
  capture_saved_pid_absent "$pm2_pid" \
    && { [ "$listener_pid" = "$pm2_pid" ] \
      || capture_saved_pid_absent "$listener_pid"; }
}

capture_world_stopped_once() {
  local process_name="$1"
  local saved_identity="$2"
  capture_saved_identity_absent "$saved_identity" \
    && capture_pm2_process_stopped "$process_name" \
    && capture_world_ports_absent
}

capture_world_ready_once() {
  local process_name="$1"
  local pm2_pid listener_pid
  pm2_pid="$(capture_pm2_online_pid "$process_name")" || return 1
  listener_pid="$(capture_world_listener_pid)" || return 1
  capture_pid_is_self_or_descendant "$listener_pid" "$pm2_pid" || return 1
  capture_world_ports_owned_by_pid "$listener_pid" || return 1
  printf '%s\t%s\n' "$pm2_pid" "$listener_pid"
}

capture_wait_for_world_stopped() {
  local process_name="$1"
  local saved_pid="$2"
  local deadline=$((SECONDS + CAPTURE_WORLD_STOP_TIMEOUT_SECONDS))
  while ((SECONDS < deadline)); do
    capture_world_stopped_once "$process_name" "$saved_pid" && return 0
    sleep 0.5
  done
  return 1
}

capture_wait_for_world_ready() {
  local process_name="$1"
  local identity="" last_identity="" stable_samples=0
  local deadline=$((SECONDS + CAPTURE_WORLD_READY_TIMEOUT_SECONDS))
  while ((SECONDS < deadline)); do
    if identity="$(capture_world_ready_once "$process_name")"; then
      if [ "$identity" = "$last_identity" ]; then
        stable_samples=$((stable_samples + 1))
      else
        last_identity="$identity"
        stable_samples=1
      fi
      if [ "$stable_samples" -ge 4 ]; then
        printf '%s\n' "$identity"
        return 0
      fi
    else
      last_identity=""
      stable_samples=0
    fi
    sleep 0.5
  done
  return 1
}
