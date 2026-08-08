#!/usr/bin/env bash
# Shared, source-only guard for the issue-#26 creature spell-casting fixture.
#
# Callers own the service lifecycle and provide:
#   CREATURE_SPELL_FIXTURE_JOURNAL
#   CREATURE_SPELL_FIXTURE_DB_CONF
#   CREATURE_SPELL_FIXTURE_SIDE (cpp or rust)
#   CREATURE_SPELL_FIXTURE_PM2_RUST_WORLD / _PM2_CPP_WORLD
#   CREATURE_SPELL_FIXTURE_WORLD_PORT / _INSTANCE_PORT
#   CREATURE_SPELL_FIXTURE_ORCHESTRATION_LOCK
#
# loot-fixture-common.sh supplies the credential-safe MySQL helpers and
# capture-service-common.sh supplies PM2/port inspection. This file never
# starts or stops a service.

CREATURE_SPELL_FIXTURE_FLOW=creature-spell-casting
CREATURE_SPELL_FIXTURE_CONTRACT=creature-spell-casting-shell-fixture-v1
CREATURE_SPELL_FIXTURE_ENTRY=22378
CREATURE_SPELL_FIXTURE_SPAWN_GUID=78686
CREATURE_SPELL_FIXTURE_SPELL_SLOT=0
CREATURE_SPELL_FIXTURE_SPELL_ID=15691
CREATURE_SPELL_FIXTURE_GHOST_SPELL_ID=8326
CREATURE_SPELL_FIXTURE_ORIGINAL_AI=SmartAI
CREATURE_SPELL_FIXTURE_TEMP_AI=CombatAI
CREATURE_SPELL_FIXTURE_SOURCE_DERIVATION_CONTRACT=creature-spell-casting-cpp-source-patch-v1
CREATURE_SPELL_FIXTURE_ACCOUNT=TESTBOT2@bot.local
CREATURE_SPELL_FIXTURE_ACCOUNT_ID=9
CREATURE_SPELL_FIXTURE_CHARACTER_GUID=15
CREATURE_SPELL_FIXTURE_CHARACTER_NAME_HEX=4C66676865616C
CREATURE_SPELL_FIXTURE_CHARACTER_RACE=1
CREATURE_SPELL_FIXTURE_CHARACTER_CLASS=2
CREATURE_SPELL_FIXTURE_CHARACTER_LEVEL=80
# Login starts outside the grey-level minimum aggro radius. The bot then sends
# one heartbeat to the pull position, facing east while the creature is west
# of it. That puts the melee attacker behind the player and removes
# dodge/parry/block from the C++ roll without changing combat stats.
CREATURE_SPELL_FIXTURE_CHARACTER_START_X=-2749.52
CREATURE_SPELL_FIXTURE_CHARACTER_START_Y=5431.19
CREATURE_SPELL_FIXTURE_CHARACTER_START_Z=-34.4548
CREATURE_SPELL_FIXTURE_CHARACTER_PULL_X=-2760.52
CREATURE_SPELL_FIXTURE_CHARACTER_PULL_Y=5431.19
CREATURE_SPELL_FIXTURE_CHARACTER_PULL_Z=-34.4548
CREATURE_SPELL_FIXTURE_CHARACTER_ORIENTATION=0
CREATURE_SPELL_FIXTURE_CHARACTER_TEMP_HEALTH=50000
# Pin the exact 87-column local characters schema before constructing the
# full-row SHA-256 expressions used as compare-and-swap predicates. Digest is
# over ordinal COLUMN_NAME/COLUMN_TYPE/IS_NULLABLE/DEFAULT/EXTRA metadata.
CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_COLUMN_COUNT=87
CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_METADATA_BYTES=2888
CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_SHA256=1c8ef9a9367734daced44acf567cc5453357498c04d57c37f2cce3e5108aa24c

: "${CREATURE_SPELL_FIXTURE_JOURNAL:=}"
: "${CREATURE_SPELL_FIXTURE_CLEANUP_MARKER:=}"
: "${CREATURE_SPELL_FIXTURE_DB_CONF:=}"
: "${CREATURE_SPELL_FIXTURE_DB_CONF_SHA256:=}"
: "${CREATURE_SPELL_FIXTURE_DB_CONF_IDENTITY:=}"
: "${CREATURE_SPELL_FIXTURE_SIDE:=}"
: "${CREATURE_SPELL_FIXTURE_PM2_RUST_WORLD:=}"
: "${CREATURE_SPELL_FIXTURE_PM2_CPP_WORLD:=}"
: "${CREATURE_SPELL_FIXTURE_WORLD_PORT:=}"
: "${CREATURE_SPELL_FIXTURE_INSTANCE_PORT:=}"
: "${CREATURE_SPELL_FIXTURE_ORCHESTRATION_LOCK:=}"
: "${CREATURE_SPELL_FIXTURE_MANIFEST:=}"
: "${CREATURE_SPELL_FIXTURE_MANIFEST_SHA256:=}"
: "${CREATURE_SPELL_FIXTURE_CPP_PATCH:=}"
: "${CREATURE_SPELL_FIXTURE_SOURCE_DERIVATION_JSON:=}"
: "${CREATURE_SPELL_FIXTURE_DATABASE_SNAPSHOT_SHA256:=}"
: "${CREATURE_SPELL_FIXTURE_JOURNAL_SHA256:=}"
: "${CREATURE_SPELL_FIXTURE_CURRENT_JOURNAL_SHA256:=}"
: "${CREATURE_SPELL_FIXTURE_CURRENT_JOURNAL_IDENTITY:=}"
: "${CREATURE_SPELL_FIXTURE_CREATED_AT:=}"
: "${CREATURE_SPELL_FIXTURE_PHASE:=}"
: "${CREATURE_SPELL_FIXTURE_DB_APPLIED:=0}"
: "${CREATURE_SPELL_FIXTURE_CLEANUP_VERIFIED:=0}"
: "${CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV:=}"
: "${CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_SHA256:=}"
: "${CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_ROW_SHA256:=}"
: "${CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256:=}"
: "${CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256:=}"
: "${CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256:=}"
: "${CREATURE_SPELL_FIXTURE_GHOST_PREFLIGHT_VERIFIED:=0}"
: "${CREATURE_SPELL_FIXTURE_GHOST_POST_CAPTURE_VERIFIED:=0}"

creature_spell_fixture_sha256_of_file() {
  local output digest
  output="$(sha256sum <"$1")" || return 1
  digest="${output%% *}"
  [[ "$digest" =~ ^[0-9a-f]{64}$ ]] || return 1
  printf '%s\n' "$digest"
}

creature_spell_fixture_sha256_of_text() {
  local output digest
  output="$(printf '%s' "$1" | sha256sum)" || return 1
  digest="${output%% *}"
  [[ "$digest" =~ ^[0-9a-f]{64}$ ]] || return 1
  printf '%s\n' "$digest"
}

creature_spell_fixture_validate_private_path() {
  local path="$1"
  local label="$2"
  local parent canonical owner mode

  [[ "$path" = /* && "$path" != *$'\n'* ]] || {
    echo "error: ${label} must be an absolute single-line path" >&2
    return 1
  }
  parent="$(dirname -- "$path")"
  [ -d "$parent" ] && [ ! -L "$parent" ] || {
    echo "error: ${label} parent directory does not exist or is a symlink" >&2
    return 1
  }
  canonical="$(realpath -e -- "$parent" 2>/dev/null)" || return 1
  owner="$(stat -c '%u' -- "$parent" 2>/dev/null)" || return 1
  mode="$(stat -c '%a' -- "$parent" 2>/dev/null)" || return 1
  [ "$canonical" = "$parent" ] \
    && [ "$owner" = "$(id -u)" ] \
    && [ "$mode" = 700 ] || {
      echo "error: ${label} parent must be canonical, non-symlink, owned by this uid, and mode 0700" >&2
      return 1
    }
}

creature_spell_fixture_validate_fresh_journal() {
  [ -n "$CREATURE_SPELL_FIXTURE_JOURNAL" ] || {
    echo "error: creature-spell-casting requires CREATURE_SPELL_FIXTURE_JOURNAL" >&2
    return 1
  }
  creature_spell_fixture_validate_private_path \
    "$CREATURE_SPELL_FIXTURE_JOURNAL" CREATURE_SPELL_FIXTURE_JOURNAL \
    || return 1
  CREATURE_SPELL_FIXTURE_CLEANUP_MARKER="${CREATURE_SPELL_FIXTURE_JOURNAL}.cleanup-complete"
  [ ! -e "$CREATURE_SPELL_FIXTURE_JOURNAL" ] \
    && [ ! -L "$CREATURE_SPELL_FIXTURE_JOURNAL" ] \
    && [ ! -e "$CREATURE_SPELL_FIXTURE_CLEANUP_MARKER" ] \
    && [ ! -L "$CREATURE_SPELL_FIXTURE_CLEANUP_MARKER" ] || {
      echo "error: creature spell fixture journal/cleanup marker already exists; recover it explicitly" >&2
      return 1
    }
}

creature_spell_fixture_validate_manifest_file() {
  local manifest="$1"
  local canonical

  [[ "$manifest" = /* && "$manifest" != *$'\n'* ]] \
    && [ -f "$manifest" ] && [ ! -L "$manifest" ] || return 1
  canonical="$(realpath -e -- "$manifest" 2>/dev/null)" || return 1
  [ "$canonical" = "$manifest" ] \
    && [[ "$manifest" == */crates/capture-diff/flows/creature-spell-casting/fixture/fixture.json ]] \
    && jq -e '
      .schema_version == 1
      and .flow == "creature-spell-casting"
      and .contract == "creature-spell-casting-shell-fixture-v1"
      and .source_derivation.contract == "creature-spell-casting-cpp-source-patch-v1"
      and .source_derivation.remote_url == "https://github.com/alseif0x/TrinityCoreLegacyTest.git"
      and .source_derivation.remote_ref == "refs/remotes/origin/3.4.3"
      and .source_derivation.base_head == "a5f8da2ebf5424bf0450ca4e08843ecbf72577bd"
      and .source_derivation.base_tree == "bb5c4746be7f9944b1a3f7a1eec5ea88d62fff67"
      and .source_derivation.patched_head == "8cfed90bf1720dbf8b9dc109113c8d7d9173ff6c"
      and .source_derivation.patched_tree == "228e91ed36886593c85fb601d00a9f8eb0702137"
      and .source_derivation.patch_path == "crates/capture-diff/flows/creature-spell-casting/fixture/cpp-reference.patch"
      and .source_derivation.patch_sha256 == "ef8b3c29f46fe537e1ae4e826b5610afcd534999f900ec9554ee0534e7847262"
      and .source_derivation.changed_paths == ["src/server/game/DataStores/DB2Stores.cpp"]
      and .creature_template.entry == 22378
      and .creature_template.name == "Cabal Interrogator"
      and .creature_template.original_ai_name == "SmartAI"
      and .creature_template.temporary_ai_name == "CombatAI"
      and .creature_template.script_name == ""
      and .creature_template.verified_build == 52237
      and .spawn.guid == 78686
      and .spawn.map == 530
      and .spawn.zone_id == 0
      and .spawn.area_id == 0
      and .spawn.spawn_difficulties == "0"
      and .spawn.phase_use_flags == 0
      and .spawn.phase_id == 0
      and .spawn.phase_group == 0
      and .spawn.terrain_swap_map == -1
      and .spawn.position_x == -2764.52
      and .spawn.position_y == 5431.19
      and .spawn.position_z == -34.4548
      and .spawn.orientation == 3.735
      and .spawn.spawn_time_seconds == 300
      and .spawn.wander_distance == 0
      and .spawn.movement_type == 0
      and .spawn.verified_build == 0
      and .template_spell.index == 0
      and .template_spell.spell_id == 15691
      and .template_spell.name == "Eviscerate"
      and .template_spell.verified_build == 41031
      and .spell_shape.defense_type == 2
      and .spell_shape.attributes_0 == 851984
      and .spell_shape.attributes_3 == 0
      and .spell_shape.cast_time_ms == 0
      and .spell_shape.speed == 0
      and .spell_shape.launch_delay == 0
      and .spell_shape.start_recovery_time_ms == 1000
      and .spell_shape.effect == 2
      and .spell_shape.implicit_target_a == 6
      and .spell_shape.implicit_target_b == 0
      and .spell_shape.base_points == 64
      and .spell_shape.die_sides == 1
      and .spell_shape.spell_x_spell_visual_id == 244493
      and .spell_shape.spell_visual_id == 671
      and .spell_shape.expected_start_cast_flags == 2
      and .spell_shape.expected_go_cast_flags == 256
      and .spell_shape.expected_cast_flags_ex == 0
      and .spell_shape.expected_full_combat_log == false
      and .spell_shape.required_go_hit_targets == 1
      and .spell_shape.required_go_miss_targets == 0
    ' "$manifest" >/dev/null
}

creature_spell_fixture_validate_committed_fixture() {
  local repo_root="$1"
  CREATURE_SPELL_FIXTURE_MANIFEST="${repo_root}/crates/capture-diff/flows/creature-spell-casting/fixture/fixture.json"
  creature_spell_fixture_validate_manifest_file \
    "$CREATURE_SPELL_FIXTURE_MANIFEST" || {
      echo "error: committed creature spell fixture manifest is missing or differs from its v1 contract" >&2
      return 1
    }
  CREATURE_SPELL_FIXTURE_MANIFEST_SHA256="$(
    creature_spell_fixture_sha256_of_file "$CREATURE_SPELL_FIXTURE_MANIFEST"
  )" || return 1
}

# Bind the clean C++ checkout and embedded executable revision to one reviewed
# one-parent derivation of the canonical remote base. The committed full-index
# patch is compared byte-for-byte with Git's diff, not merely by filename or
# caller-supplied digest, and the changed path set is independently pinned.
creature_spell_fixture_require_unredirected_git_environment() {
  local variable
  for variable in \
    GIT_DIR GIT_WORK_TREE GIT_COMMON_DIR GIT_INDEX_FILE \
    GIT_OBJECT_DIRECTORY GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_QUARANTINE_PATH \
    GIT_NAMESPACE GIT_REPLACE_REF_BASE GIT_GRAFT_FILE GIT_SHALLOW_FILE \
    GIT_CONFIG GIT_CONFIG_PARAMETERS GIT_CONFIG_COUNT GIT_CONFIG_SYSTEM \
    GIT_CONFIG_GLOBAL GIT_CONFIG_NOSYSTEM GIT_ATTR_SOURCE GIT_ATTR_SYSTEM \
    GIT_ATTR_GLOBAL GIT_ATTR_NOSYSTEM; do
    if [[ -v "$variable" ]]; then
      echo "error: ${variable} must be unset while validating C++ source derivation" >&2
      return 1
    fi
  done
}

creature_spell_fixture_validate_cpp_source_derivation() {
  local repo_root="$1"
  local source_repo="$2"
  local manifest="${3:-$CREATURE_SPELL_FIXTURE_MANIFEST}"
  local derivation contract remote_url remote_ref base_head base_tree
  local patched_head patched_tree patch_path patch_sha256 patch_file
  local repo_canonical patch_canonical actual_patch_sha current_head
  local parent_line actual_base_tree actual_patched_tree actual_remote_url
  local actual_remote_head expected_paths actual_paths
  local -a commit_and_parents

  CREATURE_SPELL_FIXTURE_CPP_PATCH=""
  CREATURE_SPELL_FIXTURE_SOURCE_DERIVATION_JSON=""
  creature_spell_fixture_require_unredirected_git_environment || return 1
  [ -n "$manifest" ] && [ -f "$manifest" ] && [ ! -L "$manifest" ] \
    || return 1
  derivation="$(jq -cSe '
    .source_derivation
    | select(
        type == "object"
        and (keys | sort) == [
          "base_head", "base_tree", "changed_paths", "contract",
          "patch_path", "patch_sha256", "patched_head", "patched_tree",
          "remote_ref", "remote_url"
        ]
        and (.contract | type == "string" and length > 0 and test("^[A-Za-z0-9._-]+$"))
        and (.remote_url | type == "string" and length > 0 and (contains("\\n") | not))
        and (.remote_ref | type == "string" and test("^refs/remotes/[A-Za-z0-9._/-]+$"))
        and (.base_head | type == "string" and test("^([0-9a-f]{40}|[0-9a-f]{64})$"))
        and (.base_tree | type == "string" and test("^([0-9a-f]{40}|[0-9a-f]{64})$"))
        and (.patched_head | type == "string" and test("^([0-9a-f]{40}|[0-9a-f]{64})$"))
        and (.patched_tree | type == "string" and test("^([0-9a-f]{40}|[0-9a-f]{64})$"))
        and .base_head != .patched_head
        and (.patch_path | type == "string" and test("^[A-Za-z0-9._/-]+$"))
        and (.patch_sha256 | type == "string" and test("^[0-9a-f]{64}$"))
        and (.changed_paths | type == "array" and length > 0)
        and all(.changed_paths[];
          type == "string" and test("^[A-Za-z0-9._/-]+$")
          and startswith("/") == false
          and contains("//") == false
          and contains("/../") == false
          and startswith("../") == false
          and endswith("/..") == false)
        and (.changed_paths == (.changed_paths | sort | unique))
      )
  ' "$manifest")" || {
    echo "error: creature spell C++ source derivation metadata is malformed" >&2
    return 1
  }
  contract="$(jq -r '.contract' <<<"$derivation")" || return 1
  remote_url="$(jq -r '.remote_url' <<<"$derivation")" || return 1
  remote_ref="$(jq -r '.remote_ref' <<<"$derivation")" || return 1
  base_head="$(jq -r '.base_head' <<<"$derivation")" || return 1
  base_tree="$(jq -r '.base_tree' <<<"$derivation")" || return 1
  patched_head="$(jq -r '.patched_head' <<<"$derivation")" || return 1
  patched_tree="$(jq -r '.patched_tree' <<<"$derivation")" || return 1
  patch_path="$(jq -r '.patch_path' <<<"$derivation")" || return 1
  patch_sha256="$(jq -r '.patch_sha256' <<<"$derivation")" || return 1
  [ "$contract" = "$CREATURE_SPELL_FIXTURE_SOURCE_DERIVATION_CONTRACT" ] || {
    echo "error: unexpected creature spell C++ source derivation contract" >&2
    return 1
  }
  case "/${patch_path}/" in
    */../*|*/./*|*//*)
      echo "error: creature spell C++ patch path is not canonical" >&2
      return 1
      ;;
  esac

  repo_canonical="$(realpath -e -- "$repo_root" 2>/dev/null)" || return 1
  [ "$repo_canonical" = "$repo_root" ] || return 1
  patch_file="${repo_root}/${patch_path}"
  patch_canonical="$(realpath -e -- "$patch_file" 2>/dev/null)" || {
    echo "error: reviewed creature spell C++ patch does not resolve" >&2
    return 1
  }
  [ "$patch_canonical" = "$patch_file" ] \
    && [ -f "$patch_file" ] && [ ! -L "$patch_file" ] || {
      echo "error: reviewed creature spell C++ patch is not a canonical regular file" >&2
      return 1
    }
  actual_patch_sha="$(creature_spell_fixture_sha256_of_file "$patch_file")" \
    || return 1
  [ "$actual_patch_sha" = "$patch_sha256" ] || {
    echo "error: reviewed creature spell C++ patch SHA-256 differs from fixture metadata" >&2
    return 1
  }

  current_head="$(git --no-replace-objects -C "$source_repo" rev-parse HEAD 2>/dev/null)" \
    || return 1
  [ "$current_head" = "$patched_head" ] || {
    echo "error: creature spell C++ source HEAD is not the reviewed patched commit" >&2
    return 1
  }
  parent_line="$(git --no-replace-objects -C "$source_repo" \
    rev-list --parents -n 1 "$patched_head" 2>/dev/null)" \
    || return 1
  read -r -a commit_and_parents <<<"$parent_line"
  [ "${#commit_and_parents[@]}" -eq 2 ] \
    && [ "${commit_and_parents[0]}" = "$patched_head" ] \
    && [ "${commit_and_parents[1]}" = "$base_head" ] || {
      echo "error: creature spell C++ patched commit must have exactly the reviewed canonical base as its sole parent" >&2
      return 1
    }
  actual_base_tree="$(git --no-replace-objects -C "$source_repo" \
    rev-parse "${base_head}^{tree}" 2>/dev/null)" \
    || return 1
  [ "$actual_base_tree" = "$base_tree" ] || {
    echo "error: creature spell C++ canonical base tree differs from fixture metadata" >&2
    return 1
  }
  actual_patched_tree="$(git --no-replace-objects -C "$source_repo" \
    rev-parse "${patched_head}^{tree}" 2>/dev/null)" \
    || return 1
  [ "$actual_patched_tree" = "$patched_tree" ] || {
    echo "error: creature spell C++ patched tree differs from fixture metadata" >&2
    return 1
  }
  actual_remote_url="$(git -C "$source_repo" config --get remote.origin.url 2>/dev/null)" \
    || return 1
  [ "$actual_remote_url" = "$remote_url" ] || {
    echo "error: creature spell C++ origin URL differs from fixture metadata" >&2
    return 1
  }
  actual_remote_head="$(git --no-replace-objects -C "$source_repo" \
    rev-parse "${remote_ref}^{commit}" 2>/dev/null)" \
    || {
      echo "error: creature spell C++ reviewed remote ref is missing" >&2
      return 1
    }
  [ "$actual_remote_head" = "$base_head" ] || {
    echo "error: creature spell C++ reviewed remote ref does not resolve to the canonical base" >&2
    return 1
  }
  if ! LC_ALL=C git --no-replace-objects -C "$source_repo" diff \
      --binary --full-index --no-ext-diff --no-textconv --no-renames --no-color \
      --src-prefix=a/ --dst-prefix=b/ --diff-algorithm=myers \
      "$base_head" "$patched_head" -- | cmp -s - "$patch_file"; then
    echo "error: creature spell C++ source diff bytes differ from the reviewed patch" >&2
    return 1
  fi
  expected_paths="$(jq -r '.changed_paths[]' <<<"$derivation")" || return 1
  actual_paths="$(LC_ALL=C git --no-replace-objects -C "$source_repo" diff \
    --name-only --no-ext-diff --no-textconv --no-renames --no-color \
    "$base_head" "$patched_head" --)" || return 1
  [ "$actual_paths" = "$expected_paths" ] || {
    echo "error: creature spell C++ changed paths differ from fixture metadata" >&2
    return 1
  }

  CREATURE_SPELL_FIXTURE_CPP_PATCH="$patch_file"
  CREATURE_SPELL_FIXTURE_SOURCE_DERIVATION_JSON="$derivation"
}

creature_spell_fixture_validate_db_config() {
  local canonical digest identity
  [[ "$CREATURE_SPELL_FIXTURE_DB_CONF" = /* \
    && "$CREATURE_SPELL_FIXTURE_DB_CONF" != *$'\n'* ]] \
    && [ -f "$CREATURE_SPELL_FIXTURE_DB_CONF" ] \
    && [ ! -L "$CREATURE_SPELL_FIXTURE_DB_CONF" ] || return 1
  canonical="$(realpath -e -- "$CREATURE_SPELL_FIXTURE_DB_CONF" 2>/dev/null)" \
    || return 1
  [ "$canonical" = "$CREATURE_SPELL_FIXTURE_DB_CONF" ] || return 1
  digest="$(creature_spell_fixture_sha256_of_file \
    "$CREATURE_SPELL_FIXTURE_DB_CONF")" || return 1
  identity="$(stat -c '%d:%i' -- "$CREATURE_SPELL_FIXTURE_DB_CONF")" \
    || return 1
  if [ -n "$CREATURE_SPELL_FIXTURE_DB_CONF_SHA256" ] \
      && [ "$digest" != "$CREATURE_SPELL_FIXTURE_DB_CONF_SHA256" ]; then
    return 1
  fi
  if [ -n "$CREATURE_SPELL_FIXTURE_DB_CONF_IDENTITY" ] \
      && [ "$identity" != "$CREATURE_SPELL_FIXTURE_DB_CONF_IDENTITY" ]; then
    return 1
  fi
  CREATURE_SPELL_FIXTURE_DB_CONF_SHA256="$digest"
  CREATURE_SPELL_FIXTURE_DB_CONF_IDENTITY="$identity"
}

# Recovery needs the journal-pinned DB config before the full journal loader can
# validate the live characters schema. Read only those provenance fields from a
# private journal first, and bind the result to stable journal bytes/inode so a
# caller can reject any replacement before proceeding to the full loader.
creature_spell_fixture_preload_recovery_db_config() {
  local journal="$CREATURE_SPELL_FIXTURE_JOURNAL"
  local before_sha before_identity after_sha after_identity
  local db_conf db_conf_sha256 db_conf_identity

  [ -f "$journal" ] && [ ! -L "$journal" ] \
    && [ "$(stat -c '%a' -- "$journal" 2>/dev/null)" = 600 ] \
    && [ "$(stat -c '%u' -- "$journal" 2>/dev/null)" = "$(id -u)" ] \
    || return 1
  before_sha="$(creature_spell_fixture_sha256_of_file "$journal")" \
    || return 1
  before_identity="$(stat -c '%d:%i' -- "$journal")" || return 1
  jq -e '
    (.recovery | type == "object")
    and (.recovery.db_conf
      | type == "string" and test("^/[^\\r\\n]*$"))
    and (.recovery.db_conf_sha256 | test("^[0-9a-f]{64}$"))
    and (.recovery.db_conf_identity | test("^[0-9]+:[0-9]+$"))
  ' "$journal" >/dev/null || return 1
  db_conf="$(jq -r '.recovery.db_conf' "$journal")" || return 1
  db_conf_sha256="$(jq -r '.recovery.db_conf_sha256' "$journal")" \
    || return 1
  db_conf_identity="$(jq -r '.recovery.db_conf_identity' "$journal")" \
    || return 1
  after_sha="$(creature_spell_fixture_sha256_of_file "$journal")" \
    || return 1
  after_identity="$(stat -c '%d:%i' -- "$journal")" || return 1
  [ "$after_sha" = "$before_sha" ] \
    && [ "$after_identity" = "$before_identity" ] || return 1

  CREATURE_SPELL_FIXTURE_DB_CONF="$db_conf"
  CREATURE_SPELL_FIXTURE_DB_CONF_SHA256="$db_conf_sha256"
  CREATURE_SPELL_FIXTURE_DB_CONF_IDENTITY="$db_conf_identity"
  creature_spell_fixture_validate_db_config || return 1
  CREATURE_SPELL_FIXTURE_CURRENT_JOURNAL_SHA256="$after_sha"
  CREATURE_SPELL_FIXTURE_CURRENT_JOURNAL_IDENTITY="$after_identity"
}

# CHAR_UPD_CHARACTER in the 3.4.3 C++ reference and Rust statement layer owns
# these 73 fields. Text is represented as H<uppercase-hex>, nullable text as N
# or H<hex>; every other token is numeric. This projection supplies the SET
# values for rollback, while an independent hash over all 87 columns is the
# atomic CAS predicate and detects drift even in columns outside this list.
creature_spell_fixture_validate_character_original_tsv() {
  local value="$1"
  jq -Rn -e --arg value "$value" '
    def number: test("^-?[0-9]+([.][0-9]+)?([eE][+-]?[0-9]+)?$");
    def hex: test("^H[0-9A-F]*$");
    def nullable_hex: test("^(N|H[0-9A-F]*)$");
    ($value | split("\t")) as $f
    | ($f | length) == 73
      and ($f[0] | hex)
      and all($f[1:26][]; number)
      and ($f[26] | hex)
      and all($f[27:43][]; number)
      and ($f[43] | nullable_hex)
      and all($f[44:63][]; number)
      and all($f[63:66][]; nullable_hex)
      and all($f[66:73][]; number)
  ' >/dev/null
}

creature_spell_fixture_validate_character_schema() {
  local schema
  schema="$(loot_fixture_character_mysql -e "
    SET SESSION group_concat_max_len = 1048576;
    SELECT COUNT(*),
           LENGTH(GROUP_CONCAT(CONCAT_WS(CHAR(31),
             COLUMN_NAME, COLUMN_TYPE, IS_NULLABLE,
             IFNULL(COLUMN_DEFAULT, '<NULL>'), EXTRA)
             ORDER BY ORDINAL_POSITION SEPARATOR '|')),
           LOWER(SHA2(GROUP_CONCAT(CONCAT_WS(CHAR(31),
             COLUMN_NAME, COLUMN_TYPE, IS_NULLABLE,
             IFNULL(COLUMN_DEFAULT, '<NULL>'), EXTRA)
             ORDER BY ORDINAL_POSITION SEPARATOR '|'), 256))
      FROM information_schema.COLUMNS
     WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'characters';
  ")" || return 1
  [ "$schema" = "${CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_COLUMN_COUNT}"$'\t'"${CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_METADATA_BYTES}"$'\t'"${CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_SHA256}" ] || {
    echo "error: characters table schema differs from the pinned 87-column creature spell CAS contract (${schema:-query-failed})" >&2
    return 1
  }
}

creature_spell_fixture_character_row_hash_expression() {
  local projection="${1:-current}"
  local column expression separator="" joined=""
  local -a columns=(
    guid account name slot race class gender level xp money inventorySlots
    bankSlots restState playerFlags playerFlagsEx position_x position_y
    position_z map instance_id dungeonDifficulty raidDifficulty
    legacyRaidDifficulty orientation taximask online createTime createMode
    cinematic totaltime leveltime logout_time is_logout_resting rest_bonus
    resettalents_cost resettalents_time numRespecs activeTalentGroup
    bonusTalentGroups trans_x trans_y trans_z trans_o transguid extra_flags
    summonedPetNumber at_login zone death_expire_time taxi_path totalKills
    todayKills yesterdayKills chosenTitle watchedFaction drunk health power1
    power2 power3 power4 power5 power6 power7 power8 power9 power10 ammoId
    latency lootSpecId exploredZones equipmentCache knownTitles actionBars
    deleteInfos_Account deleteInfos_Name deleteDate honor honorLevel
    honorRestState honorRestBonus lastLoginBuild personalTabardEmblemStyle
    personalTabardEmblemColor personalTabardBorderStyle
    personalTabardBorderColor personalTabardBackgroundColor
  )
  case "$projection" in current|prelogin) ;; *) return 1 ;; esac
  for column in "${columns[@]}"; do
    expression="\`${column}\`"
    if [ "$projection" = prelogin ]; then
      case "$column" in
        map) expression=530 ;;
        zone|instance_id) expression=0 ;;
        position_x) expression="CAST(${CREATURE_SPELL_FIXTURE_CHARACTER_START_X} AS FLOAT)" ;;
        position_y) expression="CAST(${CREATURE_SPELL_FIXTURE_CHARACTER_START_Y} AS FLOAT)" ;;
        position_z) expression="CAST(${CREATURE_SPELL_FIXTURE_CHARACTER_START_Z} AS FLOAT)" ;;
        orientation) expression="CAST(${CREATURE_SPELL_FIXTURE_CHARACTER_ORIENTATION} AS FLOAT)" ;;
        health) expression="$CREATURE_SPELL_FIXTURE_CHARACTER_TEMP_HEALTH" ;;
      esac
    fi
    joined+="${separator}${expression}"
    separator=,
  done
  printf 'LOWER(SHA2(JSON_COMPACT(JSON_ARRAY(%s)), 256))' "$joined"
}

# These are the 14 characters columns not written by CHAR_UPD_CHARACTER. They
# must remain byte-for-byte equal to the original snapshot before rollback;
# otherwise the 73-field SET could not reproduce the original 87-column hash.
creature_spell_fixture_character_immutable_hash_expression() {
  printf '%s' "LOWER(SHA2(JSON_COMPACT(JSON_ARRAY(
    \`guid\`, \`account\`, \`slot\`, \`createTime\`, \`createMode\`,
    \`ammoId\`, \`deleteInfos_Account\`, \`deleteInfos_Name\`, \`deleteDate\`,
    \`personalTabardEmblemStyle\`, \`personalTabardEmblemColor\`,
    \`personalTabardBorderStyle\`, \`personalTabardBorderColor\`,
    \`personalTabardBackgroundColor\`)), 256))"
}

creature_spell_fixture_character_structural_count() {
  loot_fixture_character_mysql -e "
    SELECT COUNT(*) FROM characters
     WHERE guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID}
       AND account = ${CREATURE_SPELL_FIXTURE_ACCOUNT_ID}
       AND HEX(name) = '${CREATURE_SPELL_FIXTURE_CHARACTER_NAME_HEX}'
       AND race = ${CREATURE_SPELL_FIXTURE_CHARACTER_RACE}
       AND class = ${CREATURE_SPELL_FIXTURE_CHARACTER_CLASS}
       AND level = ${CREATURE_SPELL_FIXTURE_CHARACTER_LEVEL}
       AND online = 0
       AND trans_x = 0 AND trans_y = 0 AND trans_z = 0 AND trans_o = 0
       AND transguid = 0
       AND COALESCE(taxi_path, '') = ''
       AND death_expire_time = 0
       AND NOT EXISTS (
         SELECT 1 FROM corpse
          WHERE corpse.guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID});
  "
}

creature_spell_fixture_snapshot_character() {
  local structural row remainder current_hash prelogin_hash immutable_hash
  local current_expression prelogin_expression immutable_expression
  creature_spell_fixture_validate_character_schema || return 1
  structural="$(creature_spell_fixture_character_structural_count)" || return 1
  [ "$structural" = 1 ] || {
    echo "error: creature spell fixture character ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID} is missing, online, transported/taxied/dead, or has unexpected identity" >&2
    return 1
  }
  current_expression="$(
    creature_spell_fixture_character_row_hash_expression current
  )" || return 1
  prelogin_expression="$(
    creature_spell_fixture_character_row_hash_expression prelogin
  )" || return 1
  immutable_expression="$(
    creature_spell_fixture_character_immutable_hash_expression
  )" || return 1
  row="$(loot_fixture_character_mysql -e "
    SELECT ${current_expression}, ${prelogin_expression}, ${immutable_expression},
           CONCAT('H', HEX(name)), race, class, gender, level, xp, money,
           inventorySlots, bankSlots, restState, playerFlags, playerFlagsEx,
           map, instance_id, dungeonDifficulty, raidDifficulty,
           legacyRaidDifficulty, position_x, position_y, position_z,
           orientation, trans_x, trans_y, trans_z, trans_o, transguid,
           CONCAT('H', HEX(taximask)), cinematic, totaltime, leveltime,
           rest_bonus, logout_time, is_logout_resting, resettalents_cost,
           resettalents_time, numRespecs, activeTalentGroup,
           bonusTalentGroups, extra_flags, summonedPetNumber, at_login, zone,
           death_expire_time,
           IF(taxi_path IS NULL, 'N', CONCAT('H', HEX(taxi_path))),
           totalKills, todayKills, yesterdayKills, chosenTitle,
           watchedFaction, drunk, health, power1, power2, power3, power4,
           power5, power6, power7, power8, power9, power10, latency,
           lootSpecId,
           IF(exploredZones IS NULL, 'N', CONCAT('H', HEX(exploredZones))),
           IF(equipmentCache IS NULL, 'N', CONCAT('H', HEX(equipmentCache))),
           IF(knownTitles IS NULL, 'N', CONCAT('H', HEX(knownTitles))),
           actionBars, online, honor, honorLevel, honorRestState,
           honorRestBonus, lastLoginBuild
      FROM characters
     WHERE guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID}
       AND account = ${CREATURE_SPELL_FIXTURE_ACCOUNT_ID}
       AND online = 0;
  ")" || return 1
  current_hash="${row%%$'\t'*}"
  remainder="${row#*$'\t'}"
  prelogin_hash="${remainder%%$'\t'*}"
  remainder="${remainder#*$'\t'}"
  immutable_hash="${remainder%%$'\t'*}"
  remainder="${remainder#*$'\t'}"
  [[ "$current_hash" =~ ^[0-9a-f]{64}$ \
    && "$prelogin_hash" =~ ^[0-9a-f]{64}$ \
    && "$immutable_hash" =~ ^[0-9a-f]{64}$ ]] \
    && creature_spell_fixture_validate_character_original_tsv "$remainder" || {
    echo "error: creature spell fixture character snapshot has an unsafe shape" >&2
    return 1
  }
  CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV="$remainder"
  CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_SHA256="$(
    creature_spell_fixture_sha256_of_text "$remainder"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_ROW_SHA256="$current_hash"
  CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256="$prelogin_hash"
  CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256="$immutable_hash"
}

creature_spell_fixture_verify_character_state() {
  local expected_row_sha256="$1"
  local structural current_expression current_hash
  [[ "$expected_row_sha256" =~ ^[0-9a-f]{64}$ ]] || return 1
  creature_spell_fixture_validate_character_schema || return 1
  structural="$(creature_spell_fixture_character_structural_count)" || return 1
  [ "$structural" = 1 ] || return 1
  current_expression="$(
    creature_spell_fixture_character_row_hash_expression current
  )" || return 1
  current_hash="$(loot_fixture_character_mysql -e "
    SELECT ${current_expression} FROM characters
     WHERE guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID}
       AND account = ${CREATURE_SPELL_FIXTURE_ACCOUNT_ID}
       AND online = 0;
  ")" || return 1
  [ "$current_hash" = "$expected_row_sha256" ]
}

creature_spell_fixture_verify_character_original() {
  creature_spell_fixture_validate_character_original_tsv \
    "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV" \
    && [ "$(creature_spell_fixture_sha256_of_text \
      "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV")" \
      = "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_SHA256" ] \
    && creature_spell_fixture_verify_character_state \
      "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_ROW_SHA256"
}

creature_spell_fixture_verify_character_prelogin() {
  creature_spell_fixture_verify_character_state \
    "$CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256"
}

creature_spell_fixture_verify_character_immutable() {
  local expression current_hash
  [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256" \
    =~ ^[0-9a-f]{64}$ ]] || return 1
  expression="$(
    creature_spell_fixture_character_immutable_hash_expression
  )" || return 1
  current_hash="$(loot_fixture_character_mysql -e "
    SELECT ${expression} FROM characters
     WHERE guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID}
       AND account = ${CREATURE_SPELL_FIXTURE_ACCOUNT_ID}
       AND online = 0;
  ")" || return 1
  [ "$current_hash" = "$CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256" ]
}

# Snapshot every field that restoration owns only after both worlds are stopped.
# The bounded WHERE proves this is the fixture's completed live-session state;
# the durable hash then becomes the sole CAS authority for restoration.
creature_spell_fixture_snapshot_character_post_login() {
  local row_hash current_expression
  creature_spell_fixture_validate_character_schema || return 1
  current_expression="$(
    creature_spell_fixture_character_row_hash_expression current
  )" || return 1
  row_hash="$(loot_fixture_character_mysql -e "
    SELECT ${current_expression}
      FROM characters
     WHERE guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID}
       AND account = ${CREATURE_SPELL_FIXTURE_ACCOUNT_ID}
       AND HEX(name) = '${CREATURE_SPELL_FIXTURE_CHARACTER_NAME_HEX}'
       AND race = ${CREATURE_SPELL_FIXTURE_CHARACTER_RACE}
       AND class = ${CREATURE_SPELL_FIXTURE_CHARACTER_CLASS}
       AND level = ${CREATURE_SPELL_FIXTURE_CHARACTER_LEVEL}
       AND online = 0
       AND map = 530 AND instance_id = 0
       AND health > 0
       AND SQRT(
         POW(position_x - CAST(-2764.52 AS FLOAT), 2)
         + POW(position_y - CAST(5431.19 AS FLOAT), 2)
         + POW(position_z - CAST(-34.4548 AS FLOAT), 2)) <= 30
       AND trans_x = 0 AND trans_y = 0 AND trans_z = 0 AND trans_o = 0
       AND transguid = 0 AND COALESCE(taxi_path, '') = ''
       AND death_expire_time = 0
       AND NOT EXISTS (
         SELECT 1 FROM corpse
          WHERE corpse.guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID});
  ")" || return 1
  [[ "$row_hash" =~ ^[0-9a-f]{64}$ ]] || {
    echo "error: creature spell post-login character snapshot is absent, ambiguous, or outside the owned live-session envelope" >&2
    return 1
  }
  CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256="$row_hash"
  creature_spell_fixture_verify_character_immutable || {
    echo "error: creature spell live session changed a column outside CHAR_UPD_CHARACTER; refusing an incomplete rollback contract" >&2
    return 1
  }
}

creature_spell_fixture_verify_character_post_login() {
  creature_spell_fixture_verify_character_state \
    "$CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256"
}

creature_spell_fixture_cas_character_to_prelogin() {
  local current_expression updated
  current_expression="$(
    creature_spell_fixture_character_row_hash_expression current
  )" || return 1
  updated="$(loot_fixture_character_mysql -e "
    UPDATE characters
       SET map = 530, zone = 0, instance_id = 0,
           position_x = CAST(${CREATURE_SPELL_FIXTURE_CHARACTER_START_X} AS FLOAT),
           position_y = CAST(${CREATURE_SPELL_FIXTURE_CHARACTER_START_Y} AS FLOAT),
           position_z = CAST(${CREATURE_SPELL_FIXTURE_CHARACTER_START_Z} AS FLOAT),
           orientation = CAST(${CREATURE_SPELL_FIXTURE_CHARACTER_ORIENTATION} AS FLOAT),
           health = ${CREATURE_SPELL_FIXTURE_CHARACTER_TEMP_HEALTH}
     WHERE guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID}
       AND account = ${CREATURE_SPELL_FIXTURE_ACCOUNT_ID}
       AND online = 0
       AND ${current_expression} = '${CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_ROW_SHA256}'
       AND NOT EXISTS (
         SELECT 1 FROM corpse
          WHERE corpse.guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID});
    SELECT ROW_COUNT();
  ")" || return 1
  [ "$updated" = 1 ] || {
    echo "error: creature spell fixture character relocation CAS changed ${updated:-unknown} row(s)" >&2
    return 1
  }
}

creature_spell_fixture_nullable_hex_sql() {
  local token="$1"
  case "$token" in
    N) printf 'NULL' ;;
    H|H[0-9A-F]*) printf "UNHEX('%s')" "${token#H}" ;;
    *) return 1 ;;
  esac
}

creature_spell_fixture_restore_character() {
  local -a f
  local source_row_sha256 current_expression immutable_expression updated
  local taxi_path_sql explored_zones_sql equipment_cache_sql known_titles_sql
  if creature_spell_fixture_verify_character_original; then
    return 0
  fi
  case "$CREATURE_SPELL_FIXTURE_PHASE" in
    armed)
      source_row_sha256="$CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256"
      ;;
    applied)
      source_row_sha256="$CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256"
      ;;
    captured)
      source_row_sha256="$CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256"
      ;;
    *)
      echo "WARNING: creature spell fixture has no exact source state authorized for character restoration" >&2
      return 1
      ;;
  esac
  creature_spell_fixture_verify_character_state "$source_row_sha256" || {
    echo "WARNING: creature spell fixture character differs from the exact journaled CAS source; refusing overwrite" >&2
    return 1
  }
  creature_spell_fixture_verify_character_immutable || {
    echo "WARNING: a non-restored character column differs from the original immutable projection; refusing rollback" >&2
    return 1
  }
  mapfile -t f < <(jq -Rr 'split("\t")[]' \
    <<<"$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV")
  [ "${#f[@]}" = 73 ] || return 1
  taxi_path_sql="$(creature_spell_fixture_nullable_hex_sql "${f[43]}")" \
    || return 1
  explored_zones_sql="$(creature_spell_fixture_nullable_hex_sql "${f[63]}")" \
    || return 1
  equipment_cache_sql="$(creature_spell_fixture_nullable_hex_sql "${f[64]}")" \
    || return 1
  known_titles_sql="$(creature_spell_fixture_nullable_hex_sql "${f[65]}")" \
    || return 1
  current_expression="$(
    creature_spell_fixture_character_row_hash_expression current
  )" || return 1
  immutable_expression="$(
    creature_spell_fixture_character_immutable_hash_expression
  )" || return 1
  updated="$(loot_fixture_character_mysql -e "
    UPDATE characters
       SET name = UNHEX('${f[0]#H}'), race = ${f[1]}, class = ${f[2]},
           gender = ${f[3]}, level = ${f[4]}, xp = ${f[5]}, money = ${f[6]},
           inventorySlots = ${f[7]}, bankSlots = ${f[8]}, restState = ${f[9]},
           playerFlags = ${f[10]}, playerFlagsEx = ${f[11]}, map = ${f[12]},
           instance_id = ${f[13]}, dungeonDifficulty = ${f[14]},
           raidDifficulty = ${f[15]}, legacyRaidDifficulty = ${f[16]},
           position_x = CAST(${f[17]} AS FLOAT),
           position_y = CAST(${f[18]} AS FLOAT),
           position_z = CAST(${f[19]} AS FLOAT),
           orientation = CAST(${f[20]} AS FLOAT),
           trans_x = CAST(${f[21]} AS FLOAT),
           trans_y = CAST(${f[22]} AS FLOAT),
           trans_z = CAST(${f[23]} AS FLOAT),
           trans_o = CAST(${f[24]} AS FLOAT), transguid = ${f[25]},
           taximask = UNHEX('${f[26]#H}'), cinematic = ${f[27]},
           totaltime = ${f[28]}, leveltime = ${f[29]},
           rest_bonus = CAST(${f[30]} AS FLOAT), logout_time = ${f[31]},
           is_logout_resting = ${f[32]}, resettalents_cost = ${f[33]},
           resettalents_time = ${f[34]}, numRespecs = ${f[35]},
           activeTalentGroup = ${f[36]}, bonusTalentGroups = ${f[37]},
           extra_flags = ${f[38]}, summonedPetNumber = ${f[39]},
           at_login = ${f[40]}, zone = ${f[41]}, death_expire_time = ${f[42]},
           taxi_path = ${taxi_path_sql}, totalKills = ${f[44]},
           todayKills = ${f[45]}, yesterdayKills = ${f[46]},
           chosenTitle = ${f[47]}, watchedFaction = ${f[48]}, drunk = ${f[49]},
           health = ${f[50]}, power1 = ${f[51]}, power2 = ${f[52]},
           power3 = ${f[53]}, power4 = ${f[54]}, power5 = ${f[55]},
           power6 = ${f[56]}, power7 = ${f[57]}, power8 = ${f[58]},
           power9 = ${f[59]}, power10 = ${f[60]}, latency = ${f[61]},
           lootSpecId = ${f[62]}, exploredZones = ${explored_zones_sql},
           equipmentCache = ${equipment_cache_sql}, knownTitles = ${known_titles_sql},
           actionBars = ${f[66]}, online = ${f[67]}, honor = ${f[68]},
           honorLevel = ${f[69]}, honorRestState = ${f[70]},
           honorRestBonus = CAST(${f[71]} AS FLOAT), lastLoginBuild = ${f[72]}
     WHERE guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID}
       AND account = ${CREATURE_SPELL_FIXTURE_ACCOUNT_ID}
       AND online = 0
       AND ${current_expression} = '${source_row_sha256}'
       AND ${immutable_expression} = '${CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256}'
       AND NOT EXISTS (
         SELECT 1 FROM corpse
          WHERE corpse.guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID});
    SELECT ROW_COUNT();
  ")" || return 1
  [ "$updated" = 1 ] || {
    echo "WARNING: creature spell fixture character restoration CAS changed ${updated:-unknown} row(s)" >&2
    return 1
  }
  creature_spell_fixture_verify_character_original
}

# Hash every fixture-relevant row except the one intentionally changed AIName.
# Full spawn/spell/SmartAI rows make the digest useful across both capture sides,
# while the explicit state checks below produce actionable errors.
creature_spell_fixture_static_snapshot_sha256() {
  local world_snapshot character_snapshot snapshot
  creature_spell_fixture_validate_character_original_tsv \
    "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV" || return 1
  [ "$(creature_spell_fixture_sha256_of_text \
      "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV")" \
    = "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_SHA256" ] || return 1
  [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_ROW_SHA256" \
    =~ ^[0-9a-f]{64}$ ]] \
    && [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256" \
      =~ ^[0-9a-f]{64}$ ]] \
    && [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256" \
      =~ ^[0-9a-f]{64}$ ]] \
    && creature_spell_fixture_validate_character_schema || return 1
  world_snapshot="$(loot_fixture_world_mysql -e "
    SELECT entry, HEX(name), HEX(ScriptName), VerifiedBuild
      FROM creature_template WHERE entry = ${CREATURE_SPELL_FIXTURE_ENTRY};
    SELECT * FROM creature
      WHERE guid = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}
      ORDER BY guid;
    SELECT * FROM creature_template_spell
      WHERE CreatureID = ${CREATURE_SPELL_FIXTURE_ENTRY}
      ORDER BY \`Index\`;
    SELECT * FROM smart_scripts
      WHERE entryorguid = ${CREATURE_SPELL_FIXTURE_ENTRY}
        AND source_type = 0
      ORDER BY id, link;
    SELECT
      (SELECT COUNT(*) FROM pool_members
        WHERE type = 0 AND spawnId = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}),
      (SELECT COUNT(*) FROM game_event_creature
        WHERE guid = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}),
      (SELECT COUNT(*) FROM linked_respawn
        WHERE guid = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}
           OR linkedGuid = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}),
      (SELECT COUNT(*) FROM creature_addon
        WHERE guid = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}),
      (SELECT COUNT(*) FROM spawn_group
        WHERE spawnType = 0 AND spawnId = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID});
  ")" || return 1
  character_snapshot="$(loot_fixture_character_mysql -e "
    SELECT COUNT(*) FROM respawn
      WHERE type = 0 AND spawnId = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID};
    SELECT * FROM respawn
      WHERE type = 0 AND spawnId = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}
      ORDER BY instanceId, mapId;
  ")" || return 1
  snapshot="${world_snapshot}"$'\n--characters--\n'"${character_snapshot}"$'\n--fixture-character-schema--\n'"${CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_COLUMN_COUNT}"$'\t'"${CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_METADATA_BYTES}"$'\t'"${CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_SHA256}"$'\n--fixture-character-original-row--\n'"${CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_ROW_SHA256}"$'\n--fixture-character-prelogin-row--\n'"${CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256}"$'\n--fixture-character-immutable--\n'"${CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256}"$'\n--fixture-character-restore-projection--\n'"${CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV}"
  creature_spell_fixture_sha256_of_text "$snapshot"
}

# Spell 8326 applies C++ ghost server-side visibility. A stale persisted aura
# makes the living Cabal fixture invisible even when the core character row,
# corpse table, and creature respawn state are all clean. Keep this check
# intentionally narrow: reject the aura and any orphaned associated effects
# for the one pinned fixture character without claiming ownership of other
# character auras.
creature_spell_fixture_verify_no_persisted_ghost_state() {
  local counts
  counts="$(loot_fixture_character_mysql -e "
    SELECT
      (SELECT COUNT(*) FROM character_aura
        WHERE guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID}
          AND spell = ${CREATURE_SPELL_FIXTURE_GHOST_SPELL_ID}),
      (SELECT COUNT(*) FROM character_aura_effect
        WHERE guid = ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID}
          AND spell = ${CREATURE_SPELL_FIXTURE_GHOST_SPELL_ID});
  ")" || return 1
  [ "$counts" = $'0\t0' ] || {
    echo "error: creature spell fixture character ${CREATURE_SPELL_FIXTURE_CHARACTER_GUID} has persisted ghost spell ${CREATURE_SPELL_FIXTURE_GHOST_SPELL_ID} aura/effect state (${counts:-query-failed})" >&2
    return 1
  }
}

creature_spell_fixture_verify_exact_state() {
  local expected_ai="$1"
  local counts respawns expected
  case "$expected_ai" in
    "$CREATURE_SPELL_FIXTURE_ORIGINAL_AI"|"$CREATURE_SPELL_FIXTURE_TEMP_AI") ;;
    *) return 1 ;;
  esac

  counts="$(loot_fixture_world_mysql -e "
    SELECT
      (SELECT COUNT(*) FROM creature_template
        WHERE entry = ${CREATURE_SPELL_FIXTURE_ENTRY}),
      (SELECT COUNT(*) FROM creature_template
        WHERE entry = ${CREATURE_SPELL_FIXTURE_ENTRY}
          AND name = 'Cabal Interrogator'
          AND AIName = '${expected_ai}'
          AND ScriptName = ''
          AND VerifiedBuild = 52237),
      (SELECT COUNT(*) FROM creature
        WHERE id = ${CREATURE_SPELL_FIXTURE_ENTRY}),
      (SELECT COUNT(*) FROM creature
        WHERE guid = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}
          AND id = ${CREATURE_SPELL_FIXTURE_ENTRY}
          AND map = 530
          AND zoneId = 0
          AND areaId = 0
          AND spawnDifficulties = '0'
          AND phaseUseFlags = 0
          AND COALESCE(PhaseId, 0) = 0
          AND COALESCE(PhaseGroup, 0) = 0
          AND terrainSwapMap = -1
          AND position_x = CAST(-2764.52 AS FLOAT)
          AND position_y = CAST(5431.19 AS FLOAT)
          AND position_z = CAST(-34.4548 AS FLOAT)
          AND orientation = CAST(3.735 AS FLOAT)
          AND spawntimesecs = 300
          AND wander_distance = 0
          AND MovementType = 0
          AND VerifiedBuild = 0),
      (SELECT COUNT(*) FROM creature_template_spell
        WHERE CreatureID = ${CREATURE_SPELL_FIXTURE_ENTRY}),
      (SELECT COUNT(*) FROM creature_template_spell
        WHERE CreatureID = ${CREATURE_SPELL_FIXTURE_ENTRY}
          AND \`Index\` = ${CREATURE_SPELL_FIXTURE_SPELL_SLOT}
          AND Spell = ${CREATURE_SPELL_FIXTURE_SPELL_ID}
          AND VerifiedBuild = 41031),
      (SELECT COUNT(*) FROM smart_scripts
        WHERE entryorguid = ${CREATURE_SPELL_FIXTURE_ENTRY}
          AND source_type = 0),
      (SELECT COUNT(*) FROM smart_scripts
        WHERE entryorguid = ${CREATURE_SPELL_FIXTURE_ENTRY}
          AND source_type = 0
          AND id = 1 AND link = 0
          AND event_type = 0
          AND event_param1 = 2000 AND event_param2 = 5000
          AND event_param3 = 6000 AND event_param4 = 9000
          AND action_type = 11 AND action_param1 = ${CREATURE_SPELL_FIXTURE_SPELL_ID}
          AND target_type = 2),
      (SELECT COUNT(*) FROM pool_members
        WHERE type = 0 AND spawnId = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}),
      (SELECT COUNT(*) FROM game_event_creature
        WHERE guid = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}),
      (SELECT COUNT(*) FROM linked_respawn
        WHERE guid = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}
           OR linkedGuid = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}),
      (SELECT COUNT(*) FROM creature_addon
        WHERE guid = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}),
      (SELECT COUNT(*) FROM spawn_group
        WHERE spawnType = 0 AND spawnId = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID});
  ")" || return 1
  expected=$'1\t1\t1\t1\t1\t1\t3\t1\t0\t0\t0\t0\t0'
  [ "$counts" = "$expected" ] || {
    echo "error: creature spell fixture DB topology/state differs from the pinned Cabal 22378/78686/15691 contract (${counts:-query-failed})" >&2
    return 1
  }
  respawns="$(loot_fixture_character_mysql -e "
    SELECT COUNT(*) FROM respawn
      WHERE type = 0 AND spawnId = ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}
  ")" || return 1
  [ "$respawns" = 0 ] || {
    echo "error: creature spell fixture spawn ${CREATURE_SPELL_FIXTURE_SPAWN_GUID} has ${respawns:-unknown} persisted respawn row(s)" >&2
    return 1
  }
}

creature_spell_fixture_pm2_inactive() {
  local process_name="$1"
  pm2 jlist | jq -e --arg name "$process_name" '
    [.[] | select(.name == $name)] as $entries
    | ($entries | length) == 0
      or (($entries | length) == 1
        and $entries[0].pm2_env.status == "stopped"
        and (($entries[0].pid // 0) == 0))
  ' >/dev/null
}

creature_spell_fixture_require_safe_db_window() {
  [ -n "$CREATURE_SPELL_FIXTURE_PM2_RUST_WORLD" ] \
    && [ -n "$CREATURE_SPELL_FIXTURE_PM2_CPP_WORLD" ] \
    && creature_spell_fixture_pm2_inactive \
      "$CREATURE_SPELL_FIXTURE_PM2_RUST_WORLD" \
    && creature_spell_fixture_pm2_inactive \
      "$CREATURE_SPELL_FIXTURE_PM2_CPP_WORLD" \
    && capture_world_ports_absent || {
      echo "error: both world PM2 entries must be stopped/absent and both listeners absent before creature spell fixture DB access" >&2
      return 1
    }
  loot_fixture_wait_until_all_characters_offline || {
    echo "error: all characters must be offline before creature spell fixture DB access" >&2
    return 1
  }
}

creature_spell_fixture_cas_ai_name() {
  local from="$1"
  local to="$2"
  local updated
  case "${from}:${to}" in
    "${CREATURE_SPELL_FIXTURE_ORIGINAL_AI}:${CREATURE_SPELL_FIXTURE_TEMP_AI}"|\
    "${CREATURE_SPELL_FIXTURE_TEMP_AI}:${CREATURE_SPELL_FIXTURE_ORIGINAL_AI}") ;;
    *) return 1 ;;
  esac
  updated="$(loot_fixture_world_mysql -e "
    UPDATE creature_template
       SET AIName = '${to}'
     WHERE entry = ${CREATURE_SPELL_FIXTURE_ENTRY}
       AND AIName = '${from}'
       AND ScriptName = '';
    SELECT ROW_COUNT();
  ")" || return 1
  [ "$updated" = 1 ] || {
    echo "error: creature spell fixture AIName CAS ${from}->${to} changed ${updated:-unknown} row(s)" >&2
    return 1
  }
}

creature_spell_fixture_write_journal() {
  local mode="$1"
  local phase="$2"
  local journal="$CREATURE_SPELL_FIXTURE_JOURNAL"
  local parent stage old_sha old_identity
  case "$mode" in create|replace) ;; *) return 1 ;; esac
  case "$phase" in armed|applied|captured|restored) ;; *) return 1 ;; esac
  case "$CREATURE_SPELL_FIXTURE_SIDE" in cpp|rust) ;; *) return 1 ;; esac
  [[ "$CREATURE_SPELL_FIXTURE_WORLD_PORT" =~ ^[1-9][0-9]*$ ]] \
    && [[ "$CREATURE_SPELL_FIXTURE_INSTANCE_PORT" =~ ^[1-9][0-9]*$ ]] \
    && [ "$CREATURE_SPELL_FIXTURE_WORLD_PORT" \
      != "$CREATURE_SPELL_FIXTURE_INSTANCE_PORT" ] || return 1
  creature_spell_fixture_validate_character_original_tsv \
    "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV" \
    && [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_SHA256" \
      =~ ^[0-9a-f]{64}$ ]] \
    && [ "$(creature_spell_fixture_sha256_of_text \
      "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV")" \
      = "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_SHA256" ] \
    && [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_ROW_SHA256" \
      =~ ^[0-9a-f]{64}$ ]] \
    && [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256" \
      =~ ^[0-9a-f]{64}$ ]] \
    && [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256" \
      =~ ^[0-9a-f]{64}$ ]] || return 1
  case "$phase" in
    armed|applied)
      [ -z "$CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256" ] || return 1
      ;;
    captured)
      [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256" \
        =~ ^[0-9a-f]{64}$ ]] || return 1
      ;;
    restored)
      if [ -n "$CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256" ]; then
        [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256" \
          =~ ^[0-9a-f]{64}$ ]] || return 1
      fi
      ;;
  esac
  parent="$(dirname -- "$journal")"
  stage="$(mktemp "${parent}/.creature-spell-journal.XXXXXX")" || return 1
  chmod 600 "$stage" || { rm -f -- "$stage"; return 1; }
  if ! jq -n \
      --arg contract "$CREATURE_SPELL_FIXTURE_CONTRACT" \
      --arg flow "$CREATURE_SPELL_FIXTURE_FLOW" \
      --arg side "$CREATURE_SPELL_FIXTURE_SIDE" \
      --arg phase "$phase" \
      --arg created_at "$CREATURE_SPELL_FIXTURE_CREATED_AT" \
      --arg manifest "$CREATURE_SPELL_FIXTURE_MANIFEST" \
      --arg manifest_sha256 "$CREATURE_SPELL_FIXTURE_MANIFEST_SHA256" \
      --arg snapshot_sha256 "$CREATURE_SPELL_FIXTURE_DATABASE_SNAPSHOT_SHA256" \
      --arg db_conf "$CREATURE_SPELL_FIXTURE_DB_CONF" \
      --arg db_conf_sha256 "$CREATURE_SPELL_FIXTURE_DB_CONF_SHA256" \
      --arg db_conf_identity "$CREATURE_SPELL_FIXTURE_DB_CONF_IDENTITY" \
      --arg lock "$CREATURE_SPELL_FIXTURE_ORCHESTRATION_LOCK" \
      --arg pm2_rust "$CREATURE_SPELL_FIXTURE_PM2_RUST_WORLD" \
      --arg pm2_cpp "$CREATURE_SPELL_FIXTURE_PM2_CPP_WORLD" \
      --arg original_ai "$CREATURE_SPELL_FIXTURE_ORIGINAL_AI" \
      --arg temporary_ai "$CREATURE_SPELL_FIXTURE_TEMP_AI" \
      --arg character_account "$CREATURE_SPELL_FIXTURE_ACCOUNT" \
      --arg character_name_hex "$CREATURE_SPELL_FIXTURE_CHARACTER_NAME_HEX" \
      --arg character_original "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV" \
      --arg character_original_sha256 "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_SHA256" \
      --arg character_original_row_sha256 "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_ROW_SHA256" \
      --arg character_prelogin_row_sha256 "$CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256" \
      --arg character_post_login_row_sha256 "$CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256" \
      --arg character_immutable_sha256 "$CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256" \
      --arg character_schema_sha256 "$CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_SHA256" \
      --argjson world_port "$CREATURE_SPELL_FIXTURE_WORLD_PORT" \
      --argjson instance_port "$CREATURE_SPELL_FIXTURE_INSTANCE_PORT" \
      --argjson entry "$CREATURE_SPELL_FIXTURE_ENTRY" \
      --argjson spawn_guid "$CREATURE_SPELL_FIXTURE_SPAWN_GUID" \
      --argjson spell_slot "$CREATURE_SPELL_FIXTURE_SPELL_SLOT" \
      --argjson spell_id "$CREATURE_SPELL_FIXTURE_SPELL_ID" '
        {
          version: 1,
          contract: $contract,
          flow: $flow,
          side: $side,
          phase: $phase,
          created_at: $created_at,
          fixture_manifest: $manifest,
          fixture_manifest_sha256: $manifest_sha256,
          database_snapshot_sha256: $snapshot_sha256,
          original: {
            creature_entry: $entry,
            creature_spawn_guid: $spawn_guid,
            ai_name: $original_ai,
            spawn_count: 1,
            spell_slot: $spell_slot,
            spell_id: $spell_id,
            spell_count: 1
          },
          temporary: {ai_name: $temporary_ai},
          character: {
            account: $character_account,
            account_id: 9,
            guid: 15,
            name_hex: $character_name_hex,
            race: 1,
            class: 2,
            level: 80,
            schema_column_count: 87,
            schema_metadata_bytes: 2888,
            schema_sha256: $character_schema_sha256,
            restore_projection: "CHAR_UPD_CHARACTER-73-fields-v1",
            original_projection_sha256: $character_original_sha256,
            original_fields: ($character_original | split("\t")),
            original_row_sha256: $character_original_row_sha256,
            prelogin_row_sha256: $character_prelogin_row_sha256,
            immutable_sha256: $character_immutable_sha256,
            post_login_row_sha256: (
              if $character_post_login_row_sha256 == ""
              then null else $character_post_login_row_sha256 end
            ),
            temporary: {
              map: 530,
              zone: 0,
              instance_id: 0,
              start_x: -2749.52,
              start_y: 5431.19,
              start_z: -34.4548,
              pull_x: -2760.52,
              pull_y: 5431.19,
              pull_z: -34.4548,
              orientation: 0,
              health: 50000
            }
          },
          recovery: {
            db_conf: $db_conf,
            db_conf_sha256: $db_conf_sha256,
            db_conf_identity: $db_conf_identity,
            orchestration_lock: $lock,
            pm2_rust_world: $pm2_rust,
            pm2_cpp_world: $pm2_cpp,
            world_port: $world_port,
            instance_port: $instance_port
          }
        }
      ' >"$stage"; then
    rm -f -- "$stage"
    return 1
  fi
  sync -f "$stage" || { rm -f -- "$stage"; return 1; }

  if [ "$mode" = create ]; then
    [ ! -e "$journal" ] && [ ! -L "$journal" ] \
      && ln -- "$stage" "$journal" || {
        rm -f -- "$stage"
        echo "error: refusing to replace an existing creature spell recovery journal" >&2
        return 1
      }
    rm -- "$stage" || return 1
  else
    [ -f "$journal" ] && [ ! -L "$journal" ] \
      && [ "$(stat -c '%a' -- "$journal" 2>/dev/null)" = 600 ] || {
        rm -f -- "$stage"
        return 1
      }
    old_sha="$(creature_spell_fixture_sha256_of_file "$journal")" || {
      rm -f -- "$stage"; return 1;
    }
    old_identity="$(stat -c '%d:%i' -- "$journal")" || {
      rm -f -- "$stage"; return 1;
    }
    [ "$old_sha" = "$CREATURE_SPELL_FIXTURE_CURRENT_JOURNAL_SHA256" ] \
      && [ "$old_identity" \
        = "$CREATURE_SPELL_FIXTURE_CURRENT_JOURNAL_IDENTITY" ] || {
        rm -f -- "$stage"
        echo "error: creature spell recovery journal changed before phase update" >&2
        return 1
      }
    mv -f -- "$stage" "$journal" || { rm -f -- "$stage"; return 1; }
  fi
  sync -f "$journal" && sync -f "$parent" || return 1
  [ -f "$journal" ] && [ ! -L "$journal" ] \
    && [ "$(stat -c '%a' -- "$journal")" = 600 ] || return 1
  CREATURE_SPELL_FIXTURE_PHASE="$phase"
  CREATURE_SPELL_FIXTURE_CURRENT_JOURNAL_SHA256="$(
    creature_spell_fixture_sha256_of_file "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CURRENT_JOURNAL_IDENTITY="$(
    stat -c '%d:%i' -- "$journal"
  )" || return 1
}

creature_spell_fixture_load_journal() {
  local journal="$CREATURE_SPELL_FIXTURE_JOURNAL"
  local expected_manifest_sha
  [ -f "$journal" ] && [ ! -L "$journal" ] \
    && [ "$(stat -c '%a' -- "$journal" 2>/dev/null)" = 600 ] \
    && [ "$(stat -c '%u' -- "$journal" 2>/dev/null)" = "$(id -u)" ] \
    || return 1
  jq -e '
    keys == [
      "character", "contract", "created_at", "database_snapshot_sha256", "fixture_manifest",
      "fixture_manifest_sha256", "flow", "original", "phase", "recovery",
      "side", "temporary", "version"
    ]
    and .version == 1
    and .contract == "creature-spell-casting-shell-fixture-v1"
    and .flow == "creature-spell-casting"
    and (.side == "cpp" or .side == "rust")
    and (.phase == "armed" or .phase == "applied" or .phase == "captured" or .phase == "restored")
    and (.created_at | type == "string" and length > 0)
    and (.fixture_manifest | type == "string" and startswith("/"))
    and (.fixture_manifest_sha256 | test("^[0-9a-f]{64}$"))
    and (.database_snapshot_sha256 | test("^[0-9a-f]{64}$"))
    and .original == {
      creature_entry: 22378,
      creature_spawn_guid: 78686,
      ai_name: "SmartAI",
      spawn_count: 1,
      spell_slot: 0,
      spell_id: 15691,
      spell_count: 1
    }
    and .temporary == {ai_name: "CombatAI"}
    and .character.account == "TESTBOT2@bot.local"
    and .character.account_id == 9
    and .character.guid == 15
    and .character.name_hex == "4C66676865616C"
    and .character.race == 1
    and .character.class == 2
    and .character.level == 80
    and (.character | keys) == [
      "account", "account_id", "class", "guid", "immutable_sha256", "level",
      "name_hex", "original_fields", "original_projection_sha256", "original_row_sha256",
      "post_login_row_sha256", "prelogin_row_sha256", "race",
      "restore_projection", "schema_column_count", "schema_metadata_bytes",
      "schema_sha256", "temporary"
    ]
    and .character.schema_column_count == 87
    and .character.schema_metadata_bytes == 2888
    and .character.schema_sha256 == "1c8ef9a9367734daced44acf567cc5453357498c04d57c37f2cce3e5108aa24c"
    and .character.restore_projection == "CHAR_UPD_CHARACTER-73-fields-v1"
    and (.character.original_projection_sha256 | test("^[0-9a-f]{64}$"))
    and (.character.original_row_sha256 | test("^[0-9a-f]{64}$"))
    and (.character.prelogin_row_sha256 | test("^[0-9a-f]{64}$"))
    and (.character.immutable_sha256 | test("^[0-9a-f]{64}$"))
    and (.character.original_fields | type == "array" and length == 73)
    and all(.character.original_fields[]; type == "string")
    and (
      if (.phase == "armed" or .phase == "applied") then
        .character.post_login_row_sha256 == null
      elif .phase == "captured" then
        (.character.post_login_row_sha256 | test("^[0-9a-f]{64}$"))
      else
        (.character.post_login_row_sha256 == null
          or (.character.post_login_row_sha256 | test("^[0-9a-f]{64}$")))
      end
    )
    and .character.temporary == {
      map: 530,
      zone: 0,
      instance_id: 0,
      start_x: -2749.52,
      start_y: 5431.19,
      start_z: -34.4548,
      pull_x: -2760.52,
      pull_y: 5431.19,
      pull_z: -34.4548,
      orientation: 0,
      health: 50000
    }
    and (.recovery.db_conf | type == "string" and startswith("/"))
    and (.recovery.db_conf_sha256 | test("^[0-9a-f]{64}$"))
    and (.recovery.db_conf_identity | test("^[0-9]+:[0-9]+$"))
    and (.recovery.orchestration_lock | type == "string" and startswith("/"))
    and (.recovery.pm2_rust_world | type == "string" and length > 0)
    and (.recovery.pm2_cpp_world | type == "string" and length > 0)
    and (.recovery.world_port | type == "number" and . >= 1 and . <= 65535)
    and (.recovery.instance_port | type == "number" and . >= 1 and . <= 65535)
    and .recovery.world_port != .recovery.instance_port
  ' "$journal" >/dev/null || return 1

  CREATURE_SPELL_FIXTURE_SIDE="$(jq -r '.side' "$journal")" || return 1
  CREATURE_SPELL_FIXTURE_PHASE="$(jq -r '.phase' "$journal")" || return 1
  CREATURE_SPELL_FIXTURE_CREATED_AT="$(jq -r '.created_at' "$journal")" || return 1
  CREATURE_SPELL_FIXTURE_MANIFEST="$(jq -r '.fixture_manifest' "$journal")" || return 1
  CREATURE_SPELL_FIXTURE_MANIFEST_SHA256="$(
    jq -r '.fixture_manifest_sha256' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_DATABASE_SNAPSHOT_SHA256="$(
    jq -r '.database_snapshot_sha256' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV="$(
    jq -r '.character.original_fields | @tsv' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_SHA256="$(
    jq -r '.character.original_projection_sha256' "$journal"
  )" || return 1
  creature_spell_fixture_validate_character_original_tsv \
    "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV" || return 1
  [ "$(creature_spell_fixture_sha256_of_text \
      "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_TSV")" \
    = "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_SHA256" ] || return 1
  CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_ROW_SHA256="$(
    jq -r '.character.original_row_sha256' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256="$(
    jq -r '.character.prelogin_row_sha256' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256="$(
    jq -r '.character.post_login_row_sha256 // ""' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256="$(
    jq -r '.character.immutable_sha256' "$journal"
  )" || return 1
  [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_ROW_SHA256" \
    =~ ^[0-9a-f]{64}$ ]] \
    && [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256" \
      =~ ^[0-9a-f]{64}$ ]] \
    && [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256" \
      =~ ^[0-9a-f]{64}$ ]] \
    && { [ -z "$CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256" ] \
      || [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256" \
        =~ ^[0-9a-f]{64}$ ]]; } || return 1
  CREATURE_SPELL_FIXTURE_DB_CONF="$(jq -r '.recovery.db_conf' "$journal")" || return 1
  CREATURE_SPELL_FIXTURE_DB_CONF_SHA256="$(
    jq -r '.recovery.db_conf_sha256' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_DB_CONF_IDENTITY="$(
    jq -r '.recovery.db_conf_identity' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_ORCHESTRATION_LOCK="$(
    jq -r '.recovery.orchestration_lock' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_PM2_RUST_WORLD="$(
    jq -r '.recovery.pm2_rust_world' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_PM2_CPP_WORLD="$(
    jq -r '.recovery.pm2_cpp_world' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_WORLD_PORT="$(
    jq -r '.recovery.world_port' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_INSTANCE_PORT="$(
    jq -r '.recovery.instance_port' "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CURRENT_JOURNAL_SHA256="$(
    creature_spell_fixture_sha256_of_file "$journal"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CURRENT_JOURNAL_IDENTITY="$(
    stat -c '%d:%i' -- "$journal"
  )" || return 1

  creature_spell_fixture_validate_manifest_file \
    "$CREATURE_SPELL_FIXTURE_MANIFEST" || return 1
  expected_manifest_sha="$(creature_spell_fixture_sha256_of_file \
    "$CREATURE_SPELL_FIXTURE_MANIFEST")" || return 1
  [ "$expected_manifest_sha" = "$CREATURE_SPELL_FIXTURE_MANIFEST_SHA256" ] \
    && creature_spell_fixture_validate_db_config \
    && creature_spell_fixture_validate_character_schema
}

creature_spell_fixture_apply_guard() {
  local snapshot after_snapshot
  CREATURE_SPELL_FIXTURE_GHOST_PREFLIGHT_VERIFIED=0
  CREATURE_SPELL_FIXTURE_GHOST_POST_CAPTURE_VERIFIED=0
  creature_spell_fixture_validate_fresh_journal || return 1
  creature_spell_fixture_validate_manifest_file \
    "$CREATURE_SPELL_FIXTURE_MANIFEST" || return 1
  [ "$(creature_spell_fixture_sha256_of_file \
      "$CREATURE_SPELL_FIXTURE_MANIFEST")" \
    = "$CREATURE_SPELL_FIXTURE_MANIFEST_SHA256" ] || return 1
  creature_spell_fixture_validate_db_config || return 1
  creature_spell_fixture_require_safe_db_window || return 1
  creature_spell_fixture_verify_exact_state \
    "$CREATURE_SPELL_FIXTURE_ORIGINAL_AI" || return 1
  creature_spell_fixture_verify_no_persisted_ghost_state || return 1
  CREATURE_SPELL_FIXTURE_GHOST_PREFLIGHT_VERIFIED=1
  creature_spell_fixture_snapshot_character || return 1
  creature_spell_fixture_verify_character_original || return 1
  snapshot="$(creature_spell_fixture_static_snapshot_sha256)" || return 1
  CREATURE_SPELL_FIXTURE_DATABASE_SNAPSHOT_SHA256="$snapshot"
  CREATURE_SPELL_FIXTURE_CREATED_AT="$(date -u +'%Y-%m-%dT%H:%M:%SZ')" \
    || return 1
  creature_spell_fixture_write_journal create armed || return 1

  # The durable journal exists before the only owned DB mutation. A kill after
  # COMMIT but before the phase update is recoverable from either exact AIName.
  creature_spell_fixture_cas_ai_name \
    "$CREATURE_SPELL_FIXTURE_ORIGINAL_AI" \
    "$CREATURE_SPELL_FIXTURE_TEMP_AI" || return 1
  CREATURE_SPELL_FIXTURE_DB_APPLIED=1
  creature_spell_fixture_cas_character_to_prelogin || return 1
  creature_spell_fixture_verify_exact_state \
    "$CREATURE_SPELL_FIXTURE_TEMP_AI" || return 1
  creature_spell_fixture_verify_character_prelogin || return 1
  after_snapshot="$(creature_spell_fixture_static_snapshot_sha256)" || return 1
  [ "$after_snapshot" = "$snapshot" ] || {
    echo "error: creature spell fixture topology drifted during activation; journal retained" >&2
    return 1
  }
  creature_spell_fixture_write_journal replace applied || return 1
  echo "creature spell fixture: entry ${CREATURE_SPELL_FIXTURE_ENTRY} AIName ${CREATURE_SPELL_FIXTURE_ORIGINAL_AI} -> ${CREATURE_SPELL_FIXTURE_TEMP_AI}; spawn ${CREATURE_SPELL_FIXTURE_SPAWN_GUID}, spell ${CREATURE_SPELL_FIXTURE_SPELL_ID} (restore journal armed)"
}

# Normal capture wrappers call this after they have stopped and accredited the
# capture world. Recovery deliberately does not synthesize a post-login hash:
# an applied journal is recoverable only while the complete row still equals
# the deterministic pre-login hash; any other row remains fail-closed.
creature_spell_fixture_record_post_login_snapshot() {
  CREATURE_SPELL_FIXTURE_GHOST_POST_CAPTURE_VERIFIED=0
  creature_spell_fixture_load_journal || {
    echo "WARNING: creature spell fixture journal is unsafe; refusing to snapshot post-login state" >&2
    return 1
  }
  creature_spell_fixture_require_safe_db_window || return 1
  case "$CREATURE_SPELL_FIXTURE_PHASE" in
    armed)
      # Activation did not reach its durable applied phase. No world could have
      # been started by a successful caller, and restore uses the exact known
      # original/pre-login states instead.
      return 0
      ;;
    applied)
      creature_spell_fixture_snapshot_character_post_login || return 1
      # Re-read by exact full-row hash before the journal transition. A later
      # change is still caught atomically by the restoration UPDATE CAS.
      creature_spell_fixture_verify_character_post_login || return 1
      creature_spell_fixture_write_journal replace captured || return 1
      creature_spell_fixture_verify_no_persisted_ghost_state || return 1
      CREATURE_SPELL_FIXTURE_GHOST_POST_CAPTURE_VERIFIED=1
      ;;
    captured)
      if creature_spell_fixture_verify_character_post_login \
          || creature_spell_fixture_verify_character_original; then
        creature_spell_fixture_verify_no_persisted_ghost_state || return 1
        CREATURE_SPELL_FIXTURE_GHOST_POST_CAPTURE_VERIFIED=1
        return 0
      fi
      echo "WARNING: journaled creature spell post-login state changed before restoration" >&2
      return 1
      ;;
    restored)
      creature_spell_fixture_verify_character_original
      ;;
    *) return 1 ;;
  esac
}

creature_spell_fixture_validate_cleanup_marker() {
  local marker="$CREATURE_SPELL_FIXTURE_CLEANUP_MARKER"
  local parent
  [ -n "$marker" ] || marker="${CREATURE_SPELL_FIXTURE_JOURNAL}.cleanup-complete"
  CREATURE_SPELL_FIXTURE_CLEANUP_MARKER="$marker"
  creature_spell_fixture_validate_private_path \
    "$marker" CREATURE_SPELL_FIXTURE_CLEANUP_MARKER || return 1
  [ -f "$marker" ] && [ ! -L "$marker" ] \
    && [ "$(stat -c '%a' -- "$marker" 2>/dev/null)" = 600 ] \
    && [ "$(stat -c '%u' -- "$marker" 2>/dev/null)" = "$(id -u)" ] \
    || return 1
  jq -e '
    keys == [
      "character_account", "character_account_id", "character_guid",
      "character_immutable_sha256",
      "character_original_projection_sha256", "character_original_row_sha256",
      "character_post_login_row_sha256", "character_prelogin_row_sha256",
      "character_schema_sha256", "cleanup_pid", "contract", "creature_entry", "creature_spawn_guid",
      "database_snapshot_sha256", "fixture_manifest_sha256", "flow",
      "journal_sha256", "original_ai_name", "side", "spell_id",
      "temporary_ai_name", "version"
    ]
    and .version == 1
    and .contract == "creature-spell-casting-shell-fixture-v1"
    and .flow == "creature-spell-casting"
    and (.side == "cpp" or .side == "rust")
    and .creature_entry == 22378
    and .creature_spawn_guid == 78686
    and .spell_id == 15691
    and .character_account == "TESTBOT2@bot.local"
    and .character_account_id == 9
    and .character_guid == 15
    and (.character_original_projection_sha256 | test("^[0-9a-f]{64}$"))
    and (.character_original_row_sha256 | test("^[0-9a-f]{64}$"))
    and (.character_prelogin_row_sha256 | test("^[0-9a-f]{64}$"))
    and (.character_immutable_sha256 | test("^[0-9a-f]{64}$"))
    and (.character_post_login_row_sha256 == null
      or (.character_post_login_row_sha256 | test("^[0-9a-f]{64}$")))
    and .character_schema_sha256 == "1c8ef9a9367734daced44acf567cc5453357498c04d57c37f2cce3e5108aa24c"
    and .original_ai_name == "SmartAI"
    and .temporary_ai_name == "CombatAI"
    and (.cleanup_pid | type == "number" and . > 0)
    and (.journal_sha256 | test("^[0-9a-f]{64}$"))
    and (.database_snapshot_sha256 | test("^[0-9a-f]{64}$"))
    and (.fixture_manifest_sha256 | test("^[0-9a-f]{64}$"))
  ' "$marker" >/dev/null || return 1
  CREATURE_SPELL_FIXTURE_SIDE="$(jq -r '.side' "$marker")" || return 1
  CREATURE_SPELL_FIXTURE_JOURNAL_SHA256="$(
    jq -r '.journal_sha256' "$marker"
  )" || return 1
  CREATURE_SPELL_FIXTURE_DATABASE_SNAPSHOT_SHA256="$(
    jq -r '.database_snapshot_sha256' "$marker"
  )" || return 1
  CREATURE_SPELL_FIXTURE_MANIFEST_SHA256="$(
    jq -r '.fixture_manifest_sha256' "$marker"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_SHA256="$(
    jq -r '.character_original_projection_sha256' "$marker"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_ROW_SHA256="$(
    jq -r '.character_original_row_sha256' "$marker"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256="$(
    jq -r '.character_prelogin_row_sha256' "$marker"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256="$(
    jq -r '.character_post_login_row_sha256 // ""' "$marker"
  )" || return 1
  CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256="$(
    jq -r '.character_immutable_sha256' "$marker"
  )" || return 1
  parent="$(dirname -- "$marker")"
  [ "$(realpath -e -- "$parent")" = "$parent" ] || return 1
  CREATURE_SPELL_FIXTURE_CLEANUP_VERIFIED=1
}

creature_spell_fixture_complete_journal() {
  local journal="$CREATURE_SPELL_FIXTURE_JOURNAL"
  local marker="${journal}.cleanup-complete"
  local parent stage journal_sha marker_sha
  CREATURE_SPELL_FIXTURE_CLEANUP_MARKER="$marker"
  [ -f "$journal" ] && [ ! -L "$journal" ] \
    && [ "$CREATURE_SPELL_FIXTURE_PHASE" = restored ] || return 1
  journal_sha="$(creature_spell_fixture_sha256_of_file "$journal")" || return 1
  [ "$journal_sha" = "$CREATURE_SPELL_FIXTURE_CURRENT_JOURNAL_SHA256" ] \
    || return 1
  parent="$(dirname -- "$journal")"

  if [ -e "$marker" ] || [ -L "$marker" ]; then
    creature_spell_fixture_validate_cleanup_marker || return 1
    marker_sha="$CREATURE_SPELL_FIXTURE_JOURNAL_SHA256"
    [ "$marker_sha" = "$journal_sha" ] || return 1
  else
    stage="$(mktemp "${parent}/.creature-spell-cleanup.XXXXXX")" || return 1
    chmod 600 "$stage" || { rm -f -- "$stage"; return 1; }
    if ! jq -n \
        --arg contract "$CREATURE_SPELL_FIXTURE_CONTRACT" \
        --arg flow "$CREATURE_SPELL_FIXTURE_FLOW" \
        --arg side "$CREATURE_SPELL_FIXTURE_SIDE" \
        --arg journal_sha256 "$journal_sha" \
        --arg snapshot_sha256 "$CREATURE_SPELL_FIXTURE_DATABASE_SNAPSHOT_SHA256" \
        --arg fixture_manifest_sha256 "$CREATURE_SPELL_FIXTURE_MANIFEST_SHA256" \
        --arg original_ai "$CREATURE_SPELL_FIXTURE_ORIGINAL_AI" \
        --arg temporary_ai "$CREATURE_SPELL_FIXTURE_TEMP_AI" \
        --arg character_account "$CREATURE_SPELL_FIXTURE_ACCOUNT" \
        --arg character_original_sha256 "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_SHA256" \
        --arg character_original_row_sha256 "$CREATURE_SPELL_FIXTURE_CHARACTER_ORIGINAL_ROW_SHA256" \
        --arg character_prelogin_row_sha256 "$CREATURE_SPELL_FIXTURE_CHARACTER_PRELOGIN_ROW_SHA256" \
        --arg character_post_login_row_sha256 "$CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256" \
        --arg character_immutable_sha256 "$CREATURE_SPELL_FIXTURE_CHARACTER_IMMUTABLE_SHA256" \
        --arg character_schema_sha256 "$CREATURE_SPELL_FIXTURE_CHARACTER_SCHEMA_SHA256" \
        --argjson cleanup_pid "$$" \
        --argjson creature_entry "$CREATURE_SPELL_FIXTURE_ENTRY" \
        --argjson creature_spawn_guid "$CREATURE_SPELL_FIXTURE_SPAWN_GUID" \
        --argjson spell_id "$CREATURE_SPELL_FIXTURE_SPELL_ID" '
          {
            version: 1,
            contract: $contract,
            flow: $flow,
            side: $side,
            creature_entry: $creature_entry,
            creature_spawn_guid: $creature_spawn_guid,
            spell_id: $spell_id,
            character_account: $character_account,
            character_account_id: 9,
            character_guid: 15,
            character_schema_sha256: $character_schema_sha256,
            character_immutable_sha256: $character_immutable_sha256,
            character_original_projection_sha256: $character_original_sha256,
            character_original_row_sha256: $character_original_row_sha256,
            character_prelogin_row_sha256: $character_prelogin_row_sha256,
            character_post_login_row_sha256: (
              if $character_post_login_row_sha256 == ""
              then null else $character_post_login_row_sha256 end
            ),
            original_ai_name: $original_ai,
            temporary_ai_name: $temporary_ai,
            fixture_manifest_sha256: $fixture_manifest_sha256,
            database_snapshot_sha256: $snapshot_sha256,
            journal_sha256: $journal_sha256,
            cleanup_pid: $cleanup_pid
          }
        ' >"$stage"; then
      rm -f -- "$stage"
      return 1
    fi
    sync -f "$stage" \
      && ln -- "$stage" "$marker" \
      && rm -- "$stage" \
      && sync -f "$marker" \
      && sync -f "$parent" || {
        rm -f -- "$stage"
        return 1
      }
  fi
  rm -- "$journal" && sync -f "$parent" || return 1
  CREATURE_SPELL_FIXTURE_JOURNAL_SHA256="$journal_sha"
  CREATURE_SPELL_FIXTURE_CURRENT_JOURNAL_SHA256=""
  CREATURE_SPELL_FIXTURE_CURRENT_JOURNAL_IDENTITY=""
  CREATURE_SPELL_FIXTURE_DB_APPLIED=0
  creature_spell_fixture_validate_cleanup_marker
}

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
    echo "WARNING: creature spell fixture topology/SmartAI/respawn state drifted; refusing AIName restoration" >&2
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
    creature_spell_fixture_cas_ai_name \
      "$CREATURE_SPELL_FIXTURE_TEMP_AI" \
      "$CREATURE_SPELL_FIXTURE_ORIGINAL_AI" || return 1
  else
    echo "WARNING: creature spell fixture AIName changed externally; refusing overwrite" >&2
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
  echo "creature spell fixture: restored entry ${CREATURE_SPELL_FIXTURE_ENTRY} AIName ${CREATURE_SPELL_FIXTURE_ORIGINAL_AI}; cleanup marker verified"
}

creature_spell_fixture_remove_cleanup_marker() {
  local marker="$CREATURE_SPELL_FIXTURE_CLEANUP_MARKER"
  local parent
  creature_spell_fixture_validate_cleanup_marker || return 1
  parent="$(dirname -- "$marker")"
  rm -- "$marker" && sync -f "$parent" || return 1
  [ ! -e "$marker" ] && [ ! -L "$marker" ]
}

creature_spell_fixture_report_proves_exact_success() {
  local report_path="$1"
  jq -e --arg manifest_sha "$CREATURE_SPELL_FIXTURE_MANIFEST_SHA256" '
    .creature_spell_capture == true
    and .detour_chase_capture == false
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
      and .creature_spell_capture == true
      and .creature_spell_capture_passed == true
      and .creature_spell_fixture_manifest_sha256 == $manifest_sha
      and .creature_spell_target_entry == 22378
      and .creature_spell_target_spawn_guid == 78686
      and (.creature_spell_target_runtime_counter | type == "number" and . > 0)
      and .creature_spell_target_discovered == true
      and .creature_spell_heartbeat_sent == true
      and (.creature_spell_heartbeat_sha256 | test("^[0-9a-f]{64}$"))
      and .creature_spell_start_opcode == 11319
      and (.creature_spell_start_body_sha256 | test("^[0-9a-f]{64}$"))
      and (.creature_spell_start_body_bytes | type == "number" and . > 0)
      and .creature_spell_go_opcode == 11318
      and (.creature_spell_go_body_sha256 | test("^[0-9a-f]{64}$"))
      and (.creature_spell_go_body_bytes | type == "number" and . > 0)
      and (.creature_spell_cast_id_low | type == "number" and . >= 0)
      and (.creature_spell_cast_id_high | type == "number" and . >= 0)
      and ((.creature_spell_cast_id_low + .creature_spell_cast_id_high) > 0)
      and .creature_spell_caster_guid_low == .creature_spell_target_runtime_counter
      and (.creature_spell_caster_guid_high | type == "number" and . > 0)
      and .creature_spell_victim_guid_low == 15
      and (.creature_spell_victim_guid_high | type == "number" and . > 0)
      and .creature_spell_spell_id == 15691
      and .creature_spell_start_cast_flags == 2
      and .creature_spell_go_cast_flags == 256
      and .creature_spell_cast_flags_ex == 0
      and .creature_spell_go_hit_target_count == 1
      and .creature_spell_go_miss_target_count == 0
      and .creature_spell_full_combat_log == false
      and .creature_spell_advanced_logging_sent == false
      and .creature_spell_adjacent_start_go == true
      and .creature_spell_disconnect_confirmed == true
      and .creature_spell_logout_confirmed == false
      and .creature_spell_failure == null)
  ' "$report_path" >/dev/null
}

creature_spell_fixture_bot_evidence() {
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
  bot_sha="$(creature_spell_fixture_sha256_of_file "$bot_exec")" || return 1
  [ "$bot_sha" = "$expected_bot_sha" ] || return 1
  creature_spell_fixture_report_proves_exact_success "$report_path" || return 1
  report_sha="$(creature_spell_fixture_sha256_of_file "$report_path")" || return 1
  printf '%s\t%s\t%s\t%s\n' \
    "$canonical_exec" "$bot_sha" "$canonical_report" "$report_sha"
}

creature_spell_fixture_capture_evidence() {
  local bot_exec="$1"
  local bot_exec_sha256="$2"
  local bot_report="$3"
  local bot_report_sha256="$4"
  [ "$CREATURE_SPELL_FIXTURE_CLEANUP_VERIFIED" -eq 1 ] \
    && [ "$CREATURE_SPELL_FIXTURE_GHOST_PREFLIGHT_VERIFIED" -eq 1 ] \
    && [ "$CREATURE_SPELL_FIXTURE_GHOST_POST_CAPTURE_VERIFIED" -eq 1 ] \
    && [[ "$CREATURE_SPELL_FIXTURE_JOURNAL_SHA256" =~ ^[0-9a-f]{64}$ ]] \
    && [[ "$CREATURE_SPELL_FIXTURE_DATABASE_SNAPSHOT_SHA256" =~ ^[0-9a-f]{64}$ ]] \
    && [[ "$CREATURE_SPELL_FIXTURE_MANIFEST_SHA256" =~ ^[0-9a-f]{64}$ ]] \
    && [[ "$CREATURE_SPELL_FIXTURE_CHARACTER_POST_LOGIN_ROW_SHA256" =~ ^[0-9a-f]{64}$ ]] \
    && [[ "$bot_exec" = /* && "$bot_exec" != *$'\t'* \
      && "$bot_exec" != *$'\n'* ]] \
    && [[ "$bot_report" = /* && "$bot_report" != *$'\t'* \
      && "$bot_report" != *$'\n'* ]] \
    && [[ "$bot_exec_sha256" =~ ^[0-9a-f]{64}$ ]] \
    && [[ "$bot_report_sha256" =~ ^[0-9a-f]{64}$ ]] \
    && creature_spell_fixture_validate_manifest_file \
      "$CREATURE_SPELL_FIXTURE_MANIFEST" \
    && [ "$(creature_spell_fixture_sha256_of_file \
      "$CREATURE_SPELL_FIXTURE_MANIFEST")" \
      = "$CREATURE_SPELL_FIXTURE_MANIFEST_SHA256" ] || return 1
  jq -n \
    --arg contract "$CREATURE_SPELL_FIXTURE_CONTRACT" \
    --arg manifest "$CREATURE_SPELL_FIXTURE_MANIFEST" \
    --arg manifest_sha256 "$CREATURE_SPELL_FIXTURE_MANIFEST_SHA256" \
    --arg journal_sha256 "$CREATURE_SPELL_FIXTURE_JOURNAL_SHA256" \
    --arg snapshot_sha256 "$CREATURE_SPELL_FIXTURE_DATABASE_SNAPSHOT_SHA256" \
    --arg bot_exec "$bot_exec" \
    --arg bot_exec_sha256 "$bot_exec_sha256" \
    --arg bot_report "$bot_report" \
    --arg bot_report_sha256 "$bot_report_sha256" \
    --argjson entry "$CREATURE_SPELL_FIXTURE_ENTRY" \
    --argjson spawn_guid "$CREATURE_SPELL_FIXTURE_SPAWN_GUID" '
      {
        fixture_guard: {
          enabled: true,
          contract: $contract,
          account: "TESTBOT2@bot.local",
          account_id: 9,
          character_guid: 15,
          peer_account: "",
          peer_account_id: 0,
          peer_character_guid: 0,
          creature_entry: $entry,
          creature_spawn_guid: $spawn_guid,
          item_entry: 0,
          fixture_manifest_path: $manifest,
          fixture_manifest_sha256: $manifest_sha256,
          journal_sha256: $journal_sha256,
          database_snapshot_sha256: $snapshot_sha256,
          cleanup_verified: true
        },
        bot_report: {
          contract: "wow-test-bot-creature-spell-casting-report-v1",
          exec_path: $bot_exec,
          exec_sha256: $bot_exec_sha256,
          report_path: $bot_report,
          report_sha256: $bot_report_sha256,
          account: "TESTBOT2@bot.local",
          account_id: 9,
          character_guid: 15,
          report_validated: true
        }
      }
    '
}
