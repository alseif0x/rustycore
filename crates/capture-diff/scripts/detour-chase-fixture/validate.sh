detour_chase_validate_committed_fixture() {
  local repo_root="$1"
  local fixture_root manifest map_asset tile_asset
  local map_sha tile_sha map_size tile_size

  fixture_root="${repo_root}/crates/capture-diff/flows/${DETOUR_FIXTURE_FLOW}/fixture"
  manifest="${fixture_root}/fixture.json"
  map_asset="${fixture_root}/mmaps/0001.mmap"
  tile_asset="${fixture_root}/mmaps/00015026.mmtile"

  for dependency in base64 jq realpath sha256sum stat; do
    command -v "$dependency" >/dev/null 2>&1 || {
      echo "error: detour fixture validation requires ${dependency}" >&2
      return 1
    }
  done
  for path in "$manifest" "$map_asset" "$tile_asset"; do
    [ -f "$path" ] && [ ! -L "$path" ] \
      && [ "$(realpath -e -- "$path" 2>/dev/null)" = "$path" ] || {
        echo "error: detour fixture prerequisite is missing/non-canonical: ${path}" >&2
        return 1
      }
  done

  jq -e '
    (keys | sort) == ["assets","character","creature","flow","generator","geometry","map","schema_version"]
    and .schema_version == 1
    and .flow == "detour-chase-around-obstacle"
    and .generator == {
      "crate":"wow-recastdetour",
      "example":"generate_detour_chase_fixture",
      "feature":"test-fixtures"
    }
    and .map == {"id":1,"grid_x":50,"grid_y":26}
    and .geometry == {
      "centre":{"x":-10118.333,"y":2681.667,"z":218.49},
      "creature_start":{"x":-10118.333,"y":2671.667,"z":218.49},
      "player_start":{"x":-10118.333,"y":2670.667,"z":218.49,"orientation":1.5707964},
      "player_destination":{"x":-10118.333,"y":2691.667,"z":218.49,"orientation":-1.5707964},
      "obstacle":{"min_x":-10123.333,"max_x":-10113.333,"min_y":2676.667,"max_y":2686.667}
    }
    and .creature == {"entry":15271,"spawn_guid":9102401}
    and .character == {"guid":15}
    and .assets == [
      {
        "path":"mmaps/0001.mmap",
        "bytes":28,
        "sha256":"3ff3365bbd0aafb383f4c2984389d07df133dd86cdb0b9340c25361db32d8f5a"
      },
      {
        "path":"mmaps/00015026.mmtile",
        "bytes":1496,
        "sha256":"693b93ac3ac605fea8b846a0e1fcf6ca2d0b0dce2f8c5d9c34739febc3731f47"
      }
    ]
  ' "$manifest" >/dev/null || {
    echo "error: detour fixture manifest differs from the reviewed issue-#24 schema" >&2
    return 1
  }

  map_size="$(stat -c '%s' -- "$map_asset")" || return 1
  tile_size="$(stat -c '%s' -- "$tile_asset")" || return 1
  map_sha="$(detour_chase_sha256_of_file "$map_asset")" || return 1
  tile_sha="$(detour_chase_sha256_of_file "$tile_asset")" || return 1
  [ "$map_size" = "$DETOUR_FIXTURE_MAP_BYTES" ] \
    && [ "$tile_size" = "$DETOUR_FIXTURE_TILE_BYTES" ] \
    && [ "$map_sha" = "$DETOUR_FIXTURE_MAP_SHA256" ] \
    && [ "$tile_sha" = "$DETOUR_FIXTURE_TILE_SHA256" ] || {
      echo "error: committed detour fixture assets differ from fixture.json" >&2
      return 1
    }

  DETOUR_FIXTURE_SOURCE_ROOT="$fixture_root"
  DETOUR_FIXTURE_MANIFEST="$manifest"
  DETOUR_FIXTURE_MANIFEST_SHA256="$(detour_chase_sha256_of_file "$manifest")" \
    || return 1
  [ "$DETOUR_FIXTURE_MANIFEST_SHA256" \
      = "$DETOUR_FIXTURE_MANIFEST_EXPECTED_SHA256" ] || {
    echo "error: detour fixture manifest bytes differ from the reviewed digest" >&2
    return 1
  }
}

detour_chase_read_config_value() {
  local config_path="$1"
  local key="$2"
  local value
  value="$(
    awk -F '"' -v key="$key" '
      $0 ~ "^[[:space:]]*" key "[[:space:]]*=" { value=$2 }
      END { if (value != "") print value }
    ' "$config_path"
  )" || return 1
  [ -n "$value" ] || return 1
  printf '%s\n' "$value"
}

detour_chase_validate_normal_data_dir() {
  local config_path="$1"
  local configured canonical child

  [ -f "$config_path" ] && [ ! -L "$config_path" ] \
    && [ "$(realpath -e -- "$config_path" 2>/dev/null)" = "$config_path" ] \
    || {
      echo "error: detour capture config must be a canonical regular non-symlink file" >&2
      return 1
    }
  configured="$(detour_chase_read_config_value "$config_path" DataDir)" || {
    echo "error: detour capture config has no quoted DataDir" >&2
    return 1
  }
  [[ "$configured" = /* ]] || {
    echo "error: detour capture requires an absolute DataDir to avoid cwd-dependent evidence" >&2
    return 1
  }
  canonical="$(realpath -e -- "$configured" 2>/dev/null)" || {
    echo "error: configured DataDir does not resolve: ${configured}" >&2
    return 1
  }
  [ -d "$canonical" ] && [ ! -L "$canonical" ] || {
    echo "error: normal DataDir must resolve to a real directory" >&2
    return 1
  }
  for child in dbc gt maps vmaps cameras; do
    [ -d "$canonical/$child" ] && [ ! -L "$canonical/$child" ] \
      && [ "$(realpath -e -- "$canonical/$child" 2>/dev/null)" \
        = "$canonical/$child" ] || {
        echo "error: normal DataDir/${child} is missing, non-canonical, or a symlink" >&2
        return 1
      }
  done
  DETOUR_FIXTURE_NORMAL_DATA_DIR="$canonical"
}

detour_chase_validate_bot_inputs() {
  [ -n "${WOW_BOT_EXEC:-}" ] \
    && [ -n "${WOW_BOT_EXEC_SHA256:-}" ] \
    && [ -n "${WOW_BOT_REPORT:-}" ] || {
      echo "error: detour capture requires WOW_BOT_EXEC, WOW_BOT_EXEC_SHA256, and WOW_BOT_REPORT" >&2
      return 1
    }
  [[ "$WOW_BOT_EXEC" != *$'\t'* && "$WOW_BOT_EXEC" != *$'\n'* \
    && "$WOW_BOT_REPORT" != *$'\t'* && "$WOW_BOT_REPORT" != *$'\n'* ]] || {
    echo "error: detour bot executable/report paths cannot contain tabs or newlines" >&2
    return 1
  }
  [[ "$WOW_BOT_EXEC_SHA256" =~ ^[0-9A-Fa-f]{64}$ ]] || {
    echo "error: WOW_BOT_EXEC_SHA256 must contain exactly 64 hexadecimal characters" >&2
    return 1
  }
  WOW_BOT_EXEC_SHA256="${WOW_BOT_EXEC_SHA256,,}"
  capture_validate_fresh_bot_inputs \
    "$WOW_BOT_EXEC" "$WOW_BOT_EXEC_SHA256" "$WOW_BOT_REPORT" || {
      echo "error: detour bot executable/report inputs are not fresh, canonical, and pinned" >&2
      return 1
    }
  DETOUR_FIXTURE_BOT_EXEC="$(realpath -e -- "$WOW_BOT_EXEC")" || return 1
  DETOUR_FIXTURE_BOT_EXEC_SHA256="$WOW_BOT_EXEC_SHA256"
  DETOUR_FIXTURE_BOT_REPORT="$WOW_BOT_REPORT"
}

detour_chase_validate_recovery_metadata() {
  [[ "$DETOUR_FIXTURE_DB_CONF" = /* \
    && "$DETOUR_FIXTURE_ORCHESTRATION_LOCK" = /* ]] || return 1
  [ -d "$DETOUR_FIXTURE_PRIVATE_DATA_DIR" ] \
    && [ ! -L "$DETOUR_FIXTURE_PRIVATE_DATA_DIR" ] \
    && [ "$(stat -c '%a' -- "$DETOUR_FIXTURE_PRIVATE_DATA_DIR")" = 700 ] \
    && [ "$(stat -c '%d:%i' -- "$DETOUR_FIXTURE_PRIVATE_DATA_DIR")" \
      = "$DETOUR_FIXTURE_PRIVATE_DATA_DIR_IDENTITY" ] || return 1
  [ -f "$DETOUR_FIXTURE_DB_CONF" ] \
    && [ ! -L "$DETOUR_FIXTURE_DB_CONF" ] \
    && [ "$(realpath -e -- "$DETOUR_FIXTURE_DB_CONF" 2>/dev/null)" \
      = "$DETOUR_FIXTURE_DB_CONF" ] \
    && [ "$(stat -c '%d:%i' -- "$DETOUR_FIXTURE_DB_CONF")" \
      = "$DETOUR_FIXTURE_DB_CONF_IDENTITY" ] \
    && [ "$(detour_chase_sha256_of_file "$DETOUR_FIXTURE_DB_CONF")" \
      = "$DETOUR_FIXTURE_DB_CONF_SHA256" ] || return 1
  [[ "$DETOUR_FIXTURE_PM2_RUST_WORLD" =~ ^[A-Za-z0-9._-]+$ \
    && "$DETOUR_FIXTURE_PM2_CPP_WORLD" =~ ^[A-Za-z0-9._-]+$ \
    && "$DETOUR_FIXTURE_WORLD_PORT" =~ ^[1-9][0-9]*$ \
    && "$DETOUR_FIXTURE_INSTANCE_PORT" =~ ^[1-9][0-9]*$ ]] \
    && ((DETOUR_FIXTURE_WORLD_PORT <= 65535)) \
    && ((DETOUR_FIXTURE_INSTANCE_PORT <= 65535)) \
    && [ "$DETOUR_FIXTURE_WORLD_PORT" != "$DETOUR_FIXTURE_INSTANCE_PORT" ] \
    && [[ "$DETOUR_FIXTURE_NORMAL_RUST_PM2_PROFILE_SHA256" \
      =~ ^[0-9a-f]{64}$ ]] \
    && [[ "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG" = /* ]] \
    || return 1
  [ -f "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG" ] \
    && [ ! -L "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG" ] \
    && [ "$(realpath -e -- "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG")" \
      = "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG" ] \
    && [ "$(stat -c '%d:%i' -- "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG")" \
      = "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG_IDENTITY" ] \
    && [ "$(detour_chase_sha256_of_file \
      "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG")" \
      = "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG_SHA256" ] || return 1
  case "$DETOUR_FIXTURE_SIDE" in
    cpp)
      [ -f "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP" ] \
        && [ ! -L "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP" ] \
        && [[ "$DETOUR_FIXTURE_CPP_CONFIG" = /* ]] \
        && [ "$(stat -c '%d:%i' -- "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP")" \
          = "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP_IDENTITY" ] \
        && [ "$(detour_chase_sha256_of_file \
          "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP")" \
          = "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP_SHA256" ] || return 1
      ;;
    rust)
      [ -f "$DETOUR_FIXTURE_PM2_RESTORE_FILE" ] \
        && [ ! -L "$DETOUR_FIXTURE_PM2_RESTORE_FILE" ] \
        && [ "$(stat -c '%a' -- "$DETOUR_FIXTURE_PM2_RESTORE_FILE")" = 600 ] \
        && [ "$(detour_chase_sha256_of_file \
          "$DETOUR_FIXTURE_PM2_RESTORE_FILE")" \
          = "$DETOUR_FIXTURE_PM2_RESTORE_FILE_SHA256" ] \
        && [ "$(stat -c '%d:%i' -- "$DETOUR_FIXTURE_PM2_RESTORE_FILE")" \
          = "$DETOUR_FIXTURE_PM2_RESTORE_FILE_IDENTITY" ] \
        && [ -f "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE" ] \
        && [ ! -L "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE" ] \
        && [ "$(stat -c '%a' -- "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE")" \
          = 600 ] \
        && [ "$(stat -c '%d:%i' -- "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE")" \
          = "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE_IDENTITY" ] \
        && [ "$(detour_chase_sha256_of_file \
          "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE")" \
          = "$DETOUR_FIXTURE_CAPTURE_CONFIG_FILE_SHA256" ] || return 1
      if [ -n "$DETOUR_FIXTURE_RUST_CONFIG" ]; then
        [ -f "$DETOUR_FIXTURE_RUST_CONFIG" ] \
          && [ ! -L "$DETOUR_FIXTURE_RUST_CONFIG" ] \
          && [ "$(stat -c '%a' -- "$DETOUR_FIXTURE_RUST_CONFIG")" = 600 ] \
          && [ "$(stat -c '%d:%i' -- "$DETOUR_FIXTURE_RUST_CONFIG")" \
            = "$DETOUR_FIXTURE_RUST_CONFIG_IDENTITY" ] \
          && [ "$(detour_chase_sha256_of_file \
            "$DETOUR_FIXTURE_RUST_CONFIG")" \
            = "$DETOUR_FIXTURE_RUST_CONFIG_SHA256" ] || return 1
      else
        [ "$DETOUR_FIXTURE_DB_APPLIED" = 0 ] \
          && [ -z "$DETOUR_FIXTURE_RUST_CONFIG_SHA256" ] \
          && [ -z "$DETOUR_FIXTURE_RUST_CONFIG_IDENTITY" ] || return 1
      fi
      ;;
    *) return 1 ;;
  esac
}

detour_chase_pinned_file_matches() {
  local path="$1"
  local identity="$2"
  local digest="$3"
  [ -f "$path" ] && [ ! -L "$path" ] \
    && [ "$(realpath -e -- "$path" 2>/dev/null)" = "$path" ] \
    && [ "$(stat -c '%d:%i' -- "$path" 2>/dev/null)" = "$identity" ] \
    && [ "$(detour_chase_sha256_of_file "$path" 2>/dev/null)" = "$digest" ]
}

# These two files decide which databases recovery writes and how the normal
# service restarts. Re-accredit them under the orchestration lock before any
# recovery mutation. On the C++ side the exact backup inode is atomically moved
# back over the active config; accept that one journaled relocation only.
detour_chase_validate_recovery_anchor_files() {
  if ! detour_chase_pinned_file_matches \
      "$DETOUR_FIXTURE_DB_CONF" \
      "$DETOUR_FIXTURE_DB_CONF_IDENTITY" \
      "$DETOUR_FIXTURE_DB_CONF_SHA256"; then
    [ "$DETOUR_FIXTURE_SIDE" = cpp ] \
      && detour_chase_pinned_file_matches \
        "$DETOUR_FIXTURE_CPP_CONFIG" \
        "$DETOUR_FIXTURE_DB_CONF_IDENTITY" \
        "$DETOUR_FIXTURE_DB_CONF_SHA256" || return 1
    DETOUR_FIXTURE_DB_CONF="$DETOUR_FIXTURE_CPP_CONFIG"
  fi
  if [ "$DETOUR_FIXTURE_SIDE" = cpp ] \
      && [ "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG" \
        = "$DETOUR_FIXTURE_CPP_CONFIG" ]; then
    if [ -f "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG" ] \
        && [ ! -L "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG" ] \
        && [ "$(detour_chase_sha256_of_file \
          "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG" 2>/dev/null)" \
          = "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG_SHA256" ]; then
      return 0
    fi
    detour_chase_pinned_file_matches \
      "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP" \
      "$DETOUR_FIXTURE_CPP_CONFIG_BACKUP_IDENTITY" \
      "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG_SHA256"
    return
  fi
  detour_chase_pinned_file_matches \
    "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG" \
    "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG_IDENTITY" \
    "$DETOUR_FIXTURE_NORMAL_RUST_CONFIG_SHA256"
}

detour_chase_arm_filesystem_recovery_journal() {
  [ "$DETOUR_FIXTURE_ENABLED" = 1 ] \
    && [ "$DETOUR_FIXTURE_DB_APPLIED" = 0 ] || return 1
  detour_chase_validate_recovery_metadata || return 1
  DETOUR_FIXTURE_PRIOR_CREATURE_EXISTS=0
  DETOUR_FIXTURE_PRIOR_CREATURE_SHA256=""
  DETOUR_FIXTURE_CREATURE_RESTORE_SQL=""
  DETOUR_FIXTURE_FIXTURE_CREATURE_SHA256=""
  DETOUR_FIXTURE_CHARACTER_IDENTITY_SHA256=""
  DETOUR_FIXTURE_CHARACTER_STABLE_SHA256=""
  DETOUR_FIXTURE_PRIOR_CHARACTER_SHA256=""
  DETOUR_FIXTURE_CHARACTER_RESTORE_SQL=""
  DETOUR_FIXTURE_CHARACTER_AUX_SNAPSHOTS_JSON="[]"
  DETOUR_FIXTURE_RESPAWN_SNAPSHOT_JSON="{}"
  DETOUR_FIXTURE_WORLD_AUX_SHA256=""
  DETOUR_FIXTURE_ACCOUNT_SNAPSHOTS_JSON="[]"
  DETOUR_FIXTURE_DATABASE_SNAPSHOT_SHA256=""
  DETOUR_FIXTURE_POSTSTATE_CHECKPOINTED=0
  DETOUR_FIXTURE_POST_CHARACTER_SHA256=""
  DETOUR_FIXTURE_POST_CHARACTER_PREDICATE_SQL=""
  DETOUR_FIXTURE_POST_WORLD_AUX_SHA256=""
  DETOUR_FIXTURE_BNET_ACCOUNT_ID=0
  DETOUR_FIXTURE_DB_APPLIED=0
  DETOUR_FIXTURE_DB_RESTORED=0
  DETOUR_FIXTURE_FILESYSTEM_RESTORED=0
  DETOUR_FIXTURE_NORMAL_RUNTIME_RESTORED=0
  detour_chase_write_fixture_journal create
}

detour_chase_checkpoint_filesystem_recovery_metadata() {
  [ "$DETOUR_FIXTURE_DB_APPLIED" = 0 ] \
    && [ -n "$DETOUR_FIXTURE_RUST_CONFIG" ] \
    && detour_chase_validate_recovery_metadata \
    && detour_chase_write_fixture_journal replace
}

detour_chase_validate_capture_orchestration() {
  local repo_root="$1"
  local config_path="$2"
  local side="$3"

  [ "${DETOUR_CAPTURE_ACK_FIXTURE_MUTATION:-0}" = "1" ] || {
    echo "error: detour capture requires DETOUR_CAPTURE_ACK_FIXTURE_MUTATION=1" >&2
    return 1
  }
  case "$side" in
    cpp|rust) ;;
    *)
      echo "error: internal detour fixture side must be cpp or rust" >&2
      return 1
      ;;
  esac
  for dependency in awk cat chmod cp dirname id jq ln mkdir mktemp mv readlink \
    realpath rm sha256sum stat sync tr wc; do
    command -v "$dependency" >/dev/null 2>&1 || {
      echo "error: detour fixture orchestration requires ${dependency}" >&2
      return 1
    }
  done
  detour_chase_validate_committed_fixture "$repo_root" || return 1
  detour_chase_validate_normal_data_dir "$config_path" || return 1
  validate_fresh_loot_fixture_journal || return 1
  detour_chase_validate_bot_inputs || return 1

  DETOUR_FIXTURE_ENABLED=1
  DETOUR_FIXTURE_SIDE="$side"
  DETOUR_FIXTURE_REPO_ROOT="$repo_root"
  DETOUR_FIXTURE_CONFIG="$config_path"
}

detour_chase_allocate_private_data_dir() {
  local private_parent private_root

  [ "$DETOUR_FIXTURE_ENABLED" = "1" ] \
    && [ -n "$DETOUR_FIXTURE_NORMAL_DATA_DIR" ] \
    && [ -n "$DETOUR_FIXTURE_SOURCE_ROOT" ] || return 1
  [ -z "$DETOUR_FIXTURE_PRIVATE_DATA_DIR" ] || return 1

  private_parent="${DETOUR_CAPTURE_PRIVATE_PARENT:-${XDG_RUNTIME_DIR:-/tmp}}"
  [ -d "$private_parent" ] && [ ! -L "$private_parent" ] \
    && [ "$(realpath -e -- "$private_parent" 2>/dev/null)" = "$private_parent" ] \
    || {
      echo "error: detour private-DataDir parent must be a canonical real directory" >&2
      return 1
    }
  private_root="$(mktemp -d \
    "${private_parent}/rustycore-detour-${DETOUR_FIXTURE_SIDE}.XXXXXX")" \
    || return 1
  chmod 700 "$private_root" || {
    rm -rf -- "$private_root"
    return 1
  }
  [ "$(realpath -e -- "$private_root" 2>/dev/null)" = "$private_root" ] \
    && [ ! -L "$private_root" ] || {
      rm -rf -- "$private_root"
      return 1
    }
  DETOUR_FIXTURE_PRIVATE_DATA_DIR="$private_root"
  DETOUR_FIXTURE_PRIVATE_DATA_DIR_IDENTITY="$(stat -c '%d:%i' -- "$private_root")" \
    || return 1
}

detour_chase_populate_private_data_dir() {
  local private_root="$DETOUR_FIXTURE_PRIVATE_DATA_DIR"
  local child source target fixture_map fixture_tile

  [ "$DETOUR_FIXTURE_ENABLED" = "1" ] \
    && [ -n "$DETOUR_FIXTURE_NORMAL_DATA_DIR" ] \
    && [ -n "$DETOUR_FIXTURE_SOURCE_ROOT" ] \
    && [ -d "$private_root" ] \
    && [ ! -L "$private_root" ] \
    && [ "$(stat -c '%d:%i' -- "$private_root" 2>/dev/null)" \
      = "$DETOUR_FIXTURE_PRIVATE_DATA_DIR_IDENTITY" ] || return 1
  for child in dbc gt maps vmaps cameras; do
    source="$DETOUR_FIXTURE_NORMAL_DATA_DIR/$child"
    target="$private_root/$child"
    ln -s -- "$source" "$target" || return 1
    [ -L "$target" ] \
      && [ "$(readlink -- "$target")" = "$source" ] \
      && [ "$(realpath -e -- "$target" 2>/dev/null)" = "$source" ] || return 1
  done
  mkdir -- "$private_root/mmaps" || return 1
  chmod 700 "$private_root/mmaps" || return 1
  fixture_map="$DETOUR_FIXTURE_SOURCE_ROOT/mmaps/0001.mmap"
  fixture_tile="$DETOUR_FIXTURE_SOURCE_ROOT/mmaps/00015026.mmtile"
  cp --update=none -- "$fixture_map" "$private_root/mmaps/0001.mmap" \
    && cp --update=none -- "$fixture_tile" \
      "$private_root/mmaps/00015026.mmtile" || return 1
  chmod 400 \
    "$private_root/mmaps/0001.mmap" \
    "$private_root/mmaps/00015026.mmtile" || return 1
  [ ! -L "$private_root/mmaps/0001.mmap" ] \
    && [ ! -L "$private_root/mmaps/00015026.mmtile" ] \
    && [ "$(detour_chase_sha256_of_file \
      "$private_root/mmaps/0001.mmap")" = "$DETOUR_FIXTURE_MAP_SHA256" ] \
    && [ "$(detour_chase_sha256_of_file \
      "$private_root/mmaps/00015026.mmtile")" = "$DETOUR_FIXTURE_TILE_SHA256" ] \
    && [ "$(stat -c '%s' -- "$private_root/mmaps/0001.mmap")" \
      = "$DETOUR_FIXTURE_MAP_BYTES" ] \
    && [ "$(stat -c '%s' -- "$private_root/mmaps/00015026.mmtile")" \
      = "$DETOUR_FIXTURE_TILE_BYTES" ] || {
      echo "error: private DataDir synthetic MMap copy failed accreditation" >&2
      return 1
    }
  sync -f "$private_root/mmaps/0001.mmap" \
    && sync -f "$private_root/mmaps/00015026.mmtile" \
    && sync -f "$private_root/mmaps" \
    && sync -f "$private_root" || return 1
}

detour_chase_prepare_private_data_dir() {
  detour_chase_allocate_private_data_dir \
    && detour_chase_populate_private_data_dir
}

detour_chase_patch_config_data_dir() {
  local config_path="$1"
  local private_data_dir="$2"
  local stage

  [ -f "$config_path" ] && [ ! -L "$config_path" ] \
    && [ -d "$private_data_dir" ] && [ ! -L "$private_data_dir" ] || return 1
  stage="$(mktemp "${config_path}.detour-stage.XXXXXX")" || return 1
  if ! awk -v data_dir="${private_data_dir}/" '
      BEGIN { replaced=0 }
      /^[[:space:]]*DataDir[[:space:]]*=/ {
        if (!replaced) {
          print "DataDir = \"" data_dir "\""
          replaced=1
        }
        next
      }
      { print }
      END {
        if (!replaced)
          print "DataDir = \"" data_dir "\""
      }
    ' "$config_path" >"$stage"; then
    rm -f -- "$stage"
    return 1
  fi
  chmod --reference="$config_path" "$stage" || {
    rm -f -- "$stage"
    return 1
  }
  mv -f -- "$stage" "$config_path" || return 1
  [ "$(detour_chase_read_config_value "$config_path" DataDir)" \
    = "${private_data_dir}/" ]
}

detour_chase_create_rust_capture_config() {
  local original_config="$1"
  local private_data_dir="$2"
  local private_parent temporary_config temporary_sha temporary_identity

  # Keep the credential-bearing temporary config inside the journaled private
  # root. If SIGKILL lands before its exact file metadata is checkpointed, the
  # unarmed recovery path can still remove the whole root by pinned inode.
  private_parent="$private_data_dir"
  temporary_config="$(mktemp \
    "${private_parent}/rustycore-detour-worldserver.XXXXXX.conf")" \
    || return 1
  cp -- "$original_config" "$temporary_config" || {
    rm -f -- "$temporary_config"
    return 1
  }
  chmod 600 "$temporary_config" || {
    rm -f -- "$temporary_config"
    return 1
  }
  detour_chase_patch_config_data_dir \
    "$temporary_config" "$private_data_dir" || {
      rm -f -- "$temporary_config"
      return 1
    }
  temporary_sha="$(detour_chase_sha256_of_file "$temporary_config")" || {
    rm -f -- "$temporary_config"
    return 1
  }
  temporary_identity="$(stat -c '%d:%i' -- "$temporary_config")" || {
    rm -f -- "$temporary_config"
    return 1
  }
  DETOUR_FIXTURE_RUST_CONFIG="$temporary_config"
  DETOUR_FIXTURE_RUST_CONFIG_SHA256="$temporary_sha"
  DETOUR_FIXTURE_RUST_CONFIG_IDENTITY="$temporary_identity"
}

detour_chase_patch_rust_pm2_capture_config() {
  local pm2_capture_file="$1"
  local temporary_config="$2"
  local stage

  stage="$(mktemp "${pm2_capture_file}.detour-stage.XXXXXX")" || return 1
  if ! jq --arg config "$temporary_config" '
      .apps[0].args as $args
      | ([range(0; ($args | length)) as $index
          | if $index > 0
              and ($args[$index - 1] == "-c"
                or $args[$index - 1] == "--config")
            then $config
            elif ($args[$index] | startswith("--config="))
            then "--config=" + $config
            else $args[$index]
            end]) as $rewritten
      | if any($args[];
          . == "-c" or . == "--config" or startswith("--config="))
        then .apps[0].args = $rewritten
        else .apps[0].args = ($args + ["--config", $config])
        end
    ' "$pm2_capture_file" >"$stage"; then
    rm -f -- "$stage"
    return 1
  fi
  chmod 600 "$stage" || {
    rm -f -- "$stage"
    return 1
  }
  mv -f -- "$stage" "$pm2_capture_file" || return 1
  jq -e --arg config "$temporary_config" '
    .apps[0].args as $args
    | ([range(0; ($args | length)) as $index
        | select(
            ($args[$index] == "--config=" + $config)
            or ($index > 0 and $args[$index] == $config
              and ($args[$index - 1] == "-c"
                or $args[$index - 1] == "--config"))
          )] | length) == 1
  ' "$pm2_capture_file" >/dev/null
}
