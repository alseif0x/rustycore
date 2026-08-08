#!/usr/bin/env bash
# Explicit DB recovery for the creature-spell-casting shell fixture.
#
# Usage:
#   CREATURE_SPELL_FIXTURE_JOURNAL=/absolute/private/fixture.journal \
#     recover-creature-spell-casting-fixture.sh [--consume-marker]
#
# This command never stops or starts a service. Both PM2 worlds must already be
# stopped/absent, both world ports must be absent, and every character must be
# offline. The optional flag removes an already-validated cleanup marker after
# recovery; without it, the marker is retained for operator review.
set -euo pipefail

MODE="${1:-}"
case "$MODE" in
  ""|--consume-marker) ;;
  *)
    echo "usage: CREATURE_SPELL_FIXTURE_JOURNAL=/absolute/private/fixture.journal $0 [--consume-marker]" >&2
    exit 2
    ;;
esac

SCRIPT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CREATURE_SPELL_FIXTURE_JOURNAL="${CREATURE_SPELL_FIXTURE_JOURNAL:-}"
CREATURE_SPELL_FIXTURE_CLEANUP_MARKER="${CREATURE_SPELL_FIXTURE_JOURNAL}.cleanup-complete"
CREATURE_SPELL_FIXTURE_DB_CONF=""
CREATURE_SPELL_FIXTURE_DB_CONF_SHA256=""
CREATURE_SPELL_FIXTURE_DB_CONF_IDENTITY=""
CREATURE_SPELL_FIXTURE_SIDE=""
CREATURE_SPELL_FIXTURE_PM2_RUST_WORLD=""
CREATURE_SPELL_FIXTURE_PM2_CPP_WORLD=""
CREATURE_SPELL_FIXTURE_WORLD_PORT=""
CREATURE_SPELL_FIXTURE_INSTANCE_PORT=""
CREATURE_SPELL_FIXTURE_ORCHESTRATION_LOCK=""
CREATURE_SPELL_FIXTURE_DB_APPLIED=1
CREATURE_SPELL_FIXTURE_CLEANUP_VERIFIED=0
LOOT_FIXTURE_DB_CONF=""
LOOT_FIXTURE_WORLD_HOST=""
LOOT_FIXTURE_WORLD_PORT=""
LOOT_FIXTURE_WORLD_USER=""
LOOT_FIXTURE_WORLD_PASSWORD=""
LOOT_FIXTURE_WORLD_DATABASE=""
LOOT_FIXTURE_CHARACTER_HOST=""
LOOT_FIXTURE_CHARACTER_PORT=""
LOOT_FIXTURE_CHARACTER_USER=""
LOOT_FIXTURE_CHARACTER_PASSWORD=""
LOOT_FIXTURE_CHARACTER_DATABASE=""
CAPTURE_ORCHESTRATION_LOCK_FD=""

# shellcheck source=loot-fixture-common.sh
source "$SCRIPT_ROOT/loot-fixture-common.sh"
# shellcheck source=capture-service-common.sh
source "$SCRIPT_ROOT/capture-service-common.sh"
# shellcheck source=creature-spell-casting-fixture-common.sh
source "$SCRIPT_ROOT/creature-spell-casting-fixture-common.sh"

for dependency in dirname flock id jq mysql pm2 realpath rg sha256sum ss stat sync; do
  command -v "$dependency" >/dev/null 2>&1 || {
    echo "error: required command not found: $dependency" >&2
    exit 2
  }
done
creature_spell_fixture_validate_private_path \
  "$CREATURE_SPELL_FIXTURE_JOURNAL" CREATURE_SPELL_FIXTURE_JOURNAL || exit 2

if [ ! -e "$CREATURE_SPELL_FIXTURE_JOURNAL" ] \
    && [ ! -L "$CREATURE_SPELL_FIXTURE_JOURNAL" ]; then
  creature_spell_fixture_validate_cleanup_marker || {
    echo "error: no safe creature spell journal or cleanup marker exists" >&2
    exit 2
  }
  if [ "$MODE" = --consume-marker ]; then
    creature_spell_fixture_remove_cleanup_marker
    echo "creature spell recovery: consumed the already-verified cleanup marker"
  else
    echo "creature spell recovery: DB restoration is already complete; cleanup marker retained at ${CREATURE_SPELL_FIXTURE_CLEANUP_MARKER}"
  fi
  exit 0
fi

creature_spell_fixture_load_journal || {
  echo "error: creature spell recovery journal failed schema/provenance validation" >&2
  exit 2
}
PRELOCK_JOURNAL_SHA256="$CREATURE_SPELL_FIXTURE_CURRENT_JOURNAL_SHA256"
CAPTURE_WORLD_PORT="$CREATURE_SPELL_FIXTURE_WORLD_PORT"
CAPTURE_INSTANCE_PORT="$CREATURE_SPELL_FIXTURE_INSTANCE_PORT"
CAPTURE_ORCHESTRATION_LOCK="$CREATURE_SPELL_FIXTURE_ORCHESTRATION_LOCK"
capture_validate_world_timeouts || exit 2
capture_acquire_orchestration_lock "$CAPTURE_ORCHESTRATION_LOCK" || {
  echo "error: another capture/QA process holds ${CAPTURE_ORCHESTRATION_LOCK}" >&2
  exit 1
}
trap capture_release_orchestration_lock EXIT

creature_spell_fixture_load_journal \
  && [ "$CREATURE_SPELL_FIXTURE_CURRENT_JOURNAL_SHA256" \
    = "$PRELOCK_JOURNAL_SHA256" ] || {
    echo "error: creature spell journal changed while acquiring the lock" >&2
    exit 1
  }
LOOT_FIXTURE_DB_CONF="$CREATURE_SPELL_FIXTURE_DB_CONF"
load_loot_fixture_database_credentials || exit 1
creature_spell_fixture_restore_guard || {
  echo "error: creature spell recovery stopped fail-closed; journal retained" >&2
  exit 1
}
if [ "$MODE" = --consume-marker ]; then
  creature_spell_fixture_remove_cleanup_marker || {
    echo "error: DB is restored but cleanup marker consumption failed" >&2
    exit 1
  }
  echo "creature spell recovery: DB restored, journal and cleanup marker consumed; services remain stopped"
else
  echo "creature spell recovery: DB restored and journal consumed; services remain stopped; cleanup marker retained at ${CREATURE_SPELL_FIXTURE_CLEANUP_MARKER}"
fi
