creature_spell_fixture_restore_guard() {
  local snapshot
  CREATURE_SPELL_FIXTURE_CLEANUP_MARKER="${CREATURE_SPELL_FIXTURE_JOURNAL}.cleanup-complete"
  if [ ! -e "$CREATURE_SPELL_FIXTURE_JOURNAL" ] \
      && [ ! -L "$CREATURE_SPELL_FIXTURE_JOURNAL" ]; then
    if [ -e "$CREATURE_SPELL_FIXTURE_CLEANUP_MARKER" ] \
        || [ -L "$CREATURE_SPELL_FIXTURE_CLEANUP_MARKER" ]; then
      creature_spell_fixture_validate_cleanup_marker
      return
    fi
    [ "$CREATURE_SPELL_FIXTURE_DB_APPLIED" -eq 0 ] && return 0
    echo "WARNING: creature spell fixture mutation lost its recovery journal" >&2
    return 1
  fi
  creature_spell_fixture_load_journal || {
    echo "WARNING: creature spell fixture journal is unsafe or invalid; refusing DB writes" >&2
    return 1
  }
  creature_spell_fixture_require_safe_db_window || return 1
  if [ "$CREATURE_SPELL_FIXTURE_PHASE" = applied ] \
      && ! creature_spell_fixture_verify_character_prelogin; then
    echo "WARNING: applied creature spell fixture no longer equals its deterministic full-row pre-login hash and has no durable post-login hash; recovery is read-only and both worlds must remain stopped" >&2
    return 1
  fi
  snapshot="$(creature_spell_fixture_static_snapshot_sha256)" || return 1
  [ "$snapshot" = "$CREATURE_SPELL_FIXTURE_DATABASE_SNAPSHOT_SHA256" ] || {
    echo "WARNING: creature spell fixture topology/SmartAI/difficulty/respawn state drifted; refusing runtime-state restoration" >&2
    return 1
  }

  case "$CREATURE_SPELL_FIXTURE_PHASE" in
    armed|applied)
      if creature_spell_fixture_verify_character_original \
          || creature_spell_fixture_verify_character_prelogin; then
        :
      else
        echo "WARNING: armed creature spell fixture character differs from both exact known states; refusing restoration" >&2
        return 1
      fi
      ;;
    captured)
      if creature_spell_fixture_verify_character_original \
          || creature_spell_fixture_verify_character_post_login; then
        :
      else
        echo "WARNING: creature spell fixture character differs from the exact durable post-login snapshot; refusing restoration" >&2
        return 1
      fi
      ;;
    restored)
      creature_spell_fixture_verify_character_original || {
        echo "WARNING: restored creature spell fixture journal no longer matches the exact original character state" >&2
        return 1
      }
      ;;
    *) return 1 ;;
  esac

  if creature_spell_fixture_verify_exact_state \
      "$CREATURE_SPELL_FIXTURE_ORIGINAL_AI"; then
    : # A prior exact CAS committed; finish its durable bookkeeping.
  elif creature_spell_fixture_verify_exact_state \
      "$CREATURE_SPELL_FIXTURE_TEMP_AI"; then
    creature_spell_fixture_cas_runtime_state \
      "$CREATURE_SPELL_FIXTURE_TEMP_AI" \
      "$CREATURE_SPELL_FIXTURE_ORIGINAL_AI" || return 1
  else
    echo "WARNING: creature spell fixture AIName/StaticFlags1 pair changed externally; refusing overwrite" >&2
    return 1
  fi
  creature_spell_fixture_restore_character || return 1
  creature_spell_fixture_verify_exact_state \
    "$CREATURE_SPELL_FIXTURE_ORIGINAL_AI" || return 1
  creature_spell_fixture_verify_character_original || return 1
  creature_spell_fixture_verify_no_persisted_ghost_state || {
    echo "WARNING: creature spell fixture restored its owned rows, but persisted ghost state prevents cleanup accreditation" >&2
    return 1
  }
  [ "$(creature_spell_fixture_static_snapshot_sha256)" \
    = "$CREATURE_SPELL_FIXTURE_DATABASE_SNAPSHOT_SHA256" ] || return 1
  if [ "$CREATURE_SPELL_FIXTURE_PHASE" != restored ]; then
    creature_spell_fixture_write_journal replace restored || return 1
  fi
  creature_spell_fixture_complete_journal || return 1
  echo "creature spell fixture: restored entry ${CREATURE_SPELL_FIXTURE_ENTRY} AIName ${CREATURE_SPELL_FIXTURE_ORIGINAL_AI}, difficulty ${CREATURE_SPELL_FIXTURE_DIFFICULTY_ID} StaticFlags1 ${CREATURE_SPELL_FIXTURE_ORIGINAL_STATIC_FLAGS_1}; cleanup marker verified"
}

creature_spell_fixture_remove_cleanup_marker() {
  local marker="$CREATURE_SPELL_FIXTURE_CLEANUP_MARKER"
  local parent
  creature_spell_fixture_validate_cleanup_marker || return 1
  parent="$(dirname -- "$marker")"
  rm -- "$marker" && sync -f "$parent" || return 1
  [ ! -e "$marker" ] && [ ! -L "$marker" ]
}
