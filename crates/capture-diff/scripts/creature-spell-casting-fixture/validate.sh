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
      .schema_version == 2
      and .flow == "creature-spell-casting"
      and .contract == "creature-spell-casting-shell-fixture-v2"
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
      and .creature_template_difficulty == {
        entry: 22378,
        difficulty_id: 0,
        min_level: 64,
        max_level: 65,
        health_scaling_expansion: 0,
        health_modifier: 1,
        mana_modifier: 1,
        armor_modifier: 1,
        damage_modifier: 1,
        creature_difficulty_id: 18203,
        type_flags: 0,
        type_flags_2: 0,
        loot_id: 22378,
        pickpocket_loot_id: 22378,
        skin_loot_id: 0,
        gold_min: 153,
        gold_max: 205,
        original_static_flags_1: 0,
        temporary_static_flags_1: 1048576,
        static_flags_2: 0,
        static_flags_3: 0,
        static_flags_4: 0,
        static_flags_5: 0,
        static_flags_6: 0,
        static_flags_7: 0,
        static_flags_8: 0
      }
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
      echo "error: committed creature spell fixture manifest is missing or differs from its v2 contract" >&2
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
