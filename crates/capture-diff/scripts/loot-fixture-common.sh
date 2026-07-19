#!/usr/bin/env bash
# Shared, source-only guard for the deterministic loot capture fixtures.
#
# Callers own service lifecycle and define these globals before using it:
#   LOOT_FIXTURE_DB_CONF
#   WOW_BOT_FIXTURE_JOURNAL
#   LOOT_FIXTURE_CLEANUP_MARKER
#   LOOT_FIXTURE_ENTRY
#   LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER
#   LOOT_FIXTURE_TEMP_HEALTH_MODIFIER
#   LOOT_FIXTURE_SNAPSHOT_READY

validate_fresh_loot_fixture_journal() {
  local journal_parent canonical_parent owner mode
  [ -n "$WOW_BOT_FIXTURE_JOURNAL" ] || {
    echo "error: guarded loot capture requires WOW_BOT_FIXTURE_JOURNAL" >&2
    return 1
  }
  [[ "$WOW_BOT_FIXTURE_JOURNAL" = /* \
    && "$WOW_BOT_FIXTURE_JOURNAL" != *$'\n'* ]] || {
    echo "error: WOW_BOT_FIXTURE_JOURNAL must be an absolute single-line path" >&2
    return 1
  }
  journal_parent="$(dirname -- "$WOW_BOT_FIXTURE_JOURNAL")"
  [ -d "$journal_parent" ] && [ ! -L "$journal_parent" ] || {
    echo "error: WOW_BOT_FIXTURE_JOURNAL parent directory does not exist" >&2
    return 1
  }
  canonical_parent="$(realpath -e -- "$journal_parent" 2>/dev/null)" \
    || return 1
  owner="$(stat -c '%u' -- "$journal_parent" 2>/dev/null)" || return 1
  mode="$(stat -c '%a' -- "$journal_parent" 2>/dev/null)" || return 1
  [ "$canonical_parent" = "$journal_parent" ] \
    && [ "$owner" = "$(id -u)" ] && [ "$mode" = 700 ] || {
    echo "error: WOW_BOT_FIXTURE_JOURNAL parent must be canonical, owned by this uid, mode 0700, and non-symlink" >&2
    return 1
  }
  LOOT_FIXTURE_CLEANUP_MARKER="${WOW_BOT_FIXTURE_JOURNAL}.cleanup-complete"
  [ ! -e "$WOW_BOT_FIXTURE_JOURNAL" ] \
    && [ ! -L "$WOW_BOT_FIXTURE_JOURNAL" ] \
    && [ ! -e "$LOOT_FIXTURE_CLEANUP_MARKER" ] \
    && [ ! -L "$LOOT_FIXTURE_CLEANUP_MARKER" ] || {
      echo "error: fixture journal/cleanup marker already exists; recover or remove it explicitly before capture" >&2
      return 1
    }
}

read_database_info() {
  local key="$1"
  local value
  value="$({
    awk -F '"' -v key="$key" '
      $0 ~ "^[[:space:]]*" key "[[:space:]]*=" { print $2; exit }
    ' "$LOOT_FIXTURE_DB_CONF"
  })"
  [ -n "$value" ] || return 1
  printf '%s\n' "$value"
}

_load_loot_fixture_database_credentials_untraced() {
  local world_info character_info extra
  [ -f "$LOOT_FIXTURE_DB_CONF" ] || {
    echo "error: loot fixture DB conf not found: ${LOOT_FIXTURE_DB_CONF}" >&2
    return 1
  }
  world_info="$(read_database_info WorldDatabaseInfo)" || {
    echo "error: WorldDatabaseInfo not found in ${LOOT_FIXTURE_DB_CONF}" >&2
    return 1
  }
  character_info="$(read_database_info CharacterDatabaseInfo)" || {
    echo "error: CharacterDatabaseInfo not found in ${LOOT_FIXTURE_DB_CONF}" >&2
    return 1
  }
  IFS=';' read -r \
    LOOT_FIXTURE_WORLD_HOST \
    LOOT_FIXTURE_WORLD_PORT \
    LOOT_FIXTURE_WORLD_USER \
    LOOT_FIXTURE_WORLD_PASSWORD \
    LOOT_FIXTURE_WORLD_DATABASE \
    extra <<<"$world_info"
  [ -z "${extra:-}" ] || {
    echo "error: WorldDatabaseInfo has unsupported extra fields" >&2
    return 1
  }
  IFS=';' read -r \
    LOOT_FIXTURE_CHARACTER_HOST \
    LOOT_FIXTURE_CHARACTER_PORT \
    LOOT_FIXTURE_CHARACTER_USER \
    LOOT_FIXTURE_CHARACTER_PASSWORD \
    LOOT_FIXTURE_CHARACTER_DATABASE \
    extra <<<"$character_info"
  [ -z "${extra:-}" ] || {
    echo "error: CharacterDatabaseInfo has unsupported extra fields" >&2
    return 1
  }
  [ -n "$LOOT_FIXTURE_WORLD_HOST" ] \
    && [[ "$LOOT_FIXTURE_WORLD_PORT" =~ ^[1-9][0-9]*$ ]] \
    && [ -n "$LOOT_FIXTURE_WORLD_USER" ] \
    && [ -n "$LOOT_FIXTURE_WORLD_DATABASE" ] \
    && [ -n "$LOOT_FIXTURE_CHARACTER_HOST" ] \
    && [[ "$LOOT_FIXTURE_CHARACTER_PORT" =~ ^[1-9][0-9]*$ ]] \
    && [ -n "$LOOT_FIXTURE_CHARACTER_USER" ] \
    && [ -n "$LOOT_FIXTURE_CHARACTER_DATABASE" ] || {
      echo "error: loot fixture DatabaseInfo is incomplete" >&2
      return 1
    }
}

load_loot_fixture_database_credentials() {
  local restore_xtrace=0 status
  if [[ "$-" == *x* ]]; then
    restore_xtrace=1
    set +x
  fi
  if _load_loot_fixture_database_credentials_untraced; then
    status=0
  else
    status=$?
  fi
  if [ "$restore_xtrace" -eq 1 ]; then
    set -x
  fi
  return "$status"
}

loot_fixture_world_mysql() {
  local restore_xtrace=0 status
  if [[ "$-" == *x* ]]; then
    restore_xtrace=1
    set +x
  fi
  if MYSQL_PWD="$LOOT_FIXTURE_WORLD_PASSWORD" mysql \
      --protocol=TCP \
      -h "$LOOT_FIXTURE_WORLD_HOST" \
      -P "$LOOT_FIXTURE_WORLD_PORT" \
      -u "$LOOT_FIXTURE_WORLD_USER" \
      --batch --raw --skip-column-names \
      "$LOOT_FIXTURE_WORLD_DATABASE" "$@"; then
    status=0
  else
    status=$?
  fi
  if [ "$restore_xtrace" -eq 1 ]; then
    set -x
  fi
  return "$status"
}

loot_fixture_character_mysql() {
  local restore_xtrace=0 status
  if [[ "$-" == *x* ]]; then
    restore_xtrace=1
    set +x
  fi
  if MYSQL_PWD="$LOOT_FIXTURE_CHARACTER_PASSWORD" mysql \
      --protocol=TCP \
      -h "$LOOT_FIXTURE_CHARACTER_HOST" \
      -P "$LOOT_FIXTURE_CHARACTER_PORT" \
      -u "$LOOT_FIXTURE_CHARACTER_USER" \
      --batch --raw --skip-column-names \
      "$LOOT_FIXTURE_CHARACTER_DATABASE" "$@"; then
    status=0
  else
    status=$?
  fi
  if [ "$restore_xtrace" -eq 1 ]; then
    set -x
  fi
  return "$status"
}

loot_fixture_wait_until_all_characters_offline() {
  local attempt online=""
  for ((attempt = 0; attempt < 300; attempt++)); do
    online="$(loot_fixture_character_mysql \
      -e 'SELECT COUNT(*) FROM characters WHERE online <> 0')" || return 1
    [ "$online" = "0" ] && return 0
    sleep 0.1
  done
  echo "error: refusing loot fixture mutation while ${online:-unknown} character(s) remain online" >&2
  return 1
}

apply_creature_health_fixture_guard() {
  local matching updated
  matching="$(loot_fixture_world_mysql -e \
    "SELECT COUNT(*) FROM creature_template_difficulty
       WHERE Entry = ${LOOT_FIXTURE_ENTRY}
         AND DifficultyID = 0
         AND ABS(HealthModifier - ${LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER}) < 0.0000001")" \
    || return 1
  [ "$matching" = "1" ] || {
    echo "error: loot fixture ${LOOT_FIXTURE_ENTRY} is missing or its original HealthModifier is not ${LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER}" >&2
    return 1
  }

  # Arm cleanup before the CAS. If the caller exits after the UPDATE but before
  # verification, restoration still inspects the exact temporary value.
  LOOT_FIXTURE_SNAPSHOT_READY=1
  updated="$(loot_fixture_world_mysql -e \
    "UPDATE creature_template_difficulty
        SET HealthModifier = ${LOOT_FIXTURE_TEMP_HEALTH_MODIFIER}
      WHERE Entry = ${LOOT_FIXTURE_ENTRY}
        AND DifficultyID = 0
        AND ABS(HealthModifier - ${LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER}) < 0.0000001;
     SELECT ROW_COUNT();")" || return 1
  [ "$updated" = "1" ] || {
    echo "error: loot fixture ${LOOT_FIXTURE_ENTRY} changed during activation (ROW_COUNT=${updated:-unknown})" >&2
    return 1
  }
  matching="$(loot_fixture_world_mysql -e \
    "SELECT COUNT(*) FROM creature_template_difficulty
       WHERE Entry = ${LOOT_FIXTURE_ENTRY}
         AND DifficultyID = 0
         AND ABS(HealthModifier - ${LOOT_FIXTURE_TEMP_HEALTH_MODIFIER}) < 0.0000001")" \
    || return 1
  [ "$matching" = "1" ] || {
    echo "error: failed to activate loot fixture ${LOOT_FIXTURE_ENTRY}" >&2
    return 1
  }
  echo "loot fixture: entry ${LOOT_FIXTURE_ENTRY} HealthModifier ${LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER} -> ${LOOT_FIXTURE_TEMP_HEALTH_MODIFIER} (restore armed)"
}

restore_creature_health_fixture_guard() {
  [ "$LOOT_FIXTURE_SNAPSHOT_READY" -eq 1 ] || return 0

  local original_matches temporary_matches updated
  original_matches="$(loot_fixture_world_mysql -e \
    "SELECT COUNT(*) FROM creature_template_difficulty
       WHERE Entry = ${LOOT_FIXTURE_ENTRY}
         AND DifficultyID = 0
         AND ABS(HealthModifier - ${LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER}) < 0.0000001")" \
    || return 1
  if [ "$original_matches" = "1" ]; then
    LOOT_FIXTURE_SNAPSHOT_READY=0
    return 0
  fi
  temporary_matches="$(loot_fixture_world_mysql -e \
    "SELECT COUNT(*) FROM creature_template_difficulty
       WHERE Entry = ${LOOT_FIXTURE_ENTRY}
         AND DifficultyID = 0
         AND ABS(HealthModifier - ${LOOT_FIXTURE_TEMP_HEALTH_MODIFIER}) < 0.0000001")" \
    || return 1
  [ "$temporary_matches" = "1" ] || {
    echo "WARNING: loot fixture ${LOOT_FIXTURE_ENTRY} changed externally; refusing to overwrite it" >&2
    return 1
  }
  updated="$(loot_fixture_world_mysql -e \
    "UPDATE creature_template_difficulty
        SET HealthModifier = ${LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER}
      WHERE Entry = ${LOOT_FIXTURE_ENTRY}
        AND DifficultyID = 0
        AND ABS(HealthModifier - ${LOOT_FIXTURE_TEMP_HEALTH_MODIFIER}) < 0.0000001;
     SELECT ROW_COUNT();")" || return 1
  [ "$updated" = "1" ] || {
    echo "WARNING: loot fixture ${LOOT_FIXTURE_ENTRY} changed during restoration (ROW_COUNT=${updated:-unknown})" >&2
    return 1
  }
  original_matches="$(loot_fixture_world_mysql -e \
    "SELECT COUNT(*) FROM creature_template_difficulty
       WHERE Entry = ${LOOT_FIXTURE_ENTRY}
         AND DifficultyID = 0
         AND ABS(HealthModifier - ${LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER}) < 0.0000001")" \
    || return 1
  [ "$original_matches" = "1" ] || {
    echo "WARNING: failed to verify restoration of loot fixture ${LOOT_FIXTURE_ENTRY}" >&2
    return 1
  }
  LOOT_FIXTURE_SNAPSHOT_READY=0
  echo "loot fixture: restored entry ${LOOT_FIXTURE_ENTRY} HealthModifier ${LOOT_FIXTURE_EXPECTED_HEALTH_MODIFIER}"
}

loot_fixture_bot_cleanup_complete() {
  [ "${LOOT_FIXTURE_GUARD_ENABLED:-0}" = "1" ] || return 0

  if [ -e "$WOW_BOT_FIXTURE_JOURNAL" ] \
      || [ -L "$WOW_BOT_FIXTURE_JOURNAL" ]; then
    echo "WARNING: bot fixture recovery journal is still pending at ${WOW_BOT_FIXTURE_JOURNAL}; refusing to start the normal PM2 world" >&2
    return 1
  fi
  if [ ! -f "$LOOT_FIXTURE_CLEANUP_MARKER" ] \
      || [ -L "$LOOT_FIXTURE_CLEANUP_MARKER" ]; then
    echo "WARNING: bot cleanup-complete marker is missing or unsafe at ${LOOT_FIXTURE_CLEANUP_MARKER}; refusing to start the normal PM2 world" >&2
    return 1
  fi
  if [ "$(stat -c '%a' -- "$LOOT_FIXTURE_CLEANUP_MARKER" 2>/dev/null)" != "600" ] \
      || ! jq -e '
        .version == 1
        and (.cleanup_pid | type == "number" and . > 0)
        and (.journal_sha256 | type == "string" and test("^[0-9a-f]{64}$"))
      ' "$LOOT_FIXTURE_CLEANUP_MARKER" >/dev/null 2>&1; then
    echo "WARNING: bot cleanup-complete marker failed its mode-0600/schema contract; refusing to start the normal PM2 world" >&2
    return 1
  fi
}

# Before the flow is exposed to the client/bot, absence of both recovery files
# proves that no bot-side mutation needs recovery. Once exposed, require the
# durable cleanup marker contract above. Ambiguous state always fails closed.
loot_fixture_bot_cleanup_safe_for_capture_state() {
  local capture_bot_ready="${1:-}"

  [ "${LOOT_FIXTURE_GUARD_ENABLED:-0}" = "1" ] || return 0

  [ -n "${WOW_BOT_FIXTURE_JOURNAL:-}" ] \
    && [ -n "${LOOT_FIXTURE_CLEANUP_MARKER:-}" ] \
    && [ "$LOOT_FIXTURE_CLEANUP_MARKER" \
      = "${WOW_BOT_FIXTURE_JOURNAL}.cleanup-complete" ] || {
    echo "WARNING: bot fixture cleanup paths are missing or inconsistent; refusing to start the normal PM2 world" >&2
    return 1
  }

  case "$capture_bot_ready" in
    0)
      if [ -e "$WOW_BOT_FIXTURE_JOURNAL" ] \
          || [ -L "$WOW_BOT_FIXTURE_JOURNAL" ] \
          || [ -e "$LOOT_FIXTURE_CLEANUP_MARKER" ] \
          || [ -L "$LOOT_FIXTURE_CLEANUP_MARKER" ]; then
        echo "WARNING: bot fixture recovery state appeared before capture readiness; refusing to start the normal PM2 world" >&2
        return 1
      fi
      ;;
    1)
      loot_fixture_bot_cleanup_complete
      ;;
    *)
      echo "WARNING: invalid capture bot-readiness state; refusing to start the normal PM2 world" >&2
      return 1
      ;;
  esac
}
