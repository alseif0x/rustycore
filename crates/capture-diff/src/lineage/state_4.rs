//! Capture lineage tracking state definitions, part 4 of 4.
//!
//! Separated from the lineage.rs root under #658. Behaviour is preserved.

use super::*;

impl AtomicFlowImport {
    /// Prepare a private complete-tree staging directory, copying only the
    /// flow's hand-reviewed metadata from the previous generation.
    pub fn prepare(root: &Path, flow: &str) -> Result<Self> {
        fs::create_dir_all(root)
            .with_context(|| format!("creating flow root {}", root.display()))?;
        let target = root.join(flow);
        let target_existed_at_prepare = match fs::symlink_metadata(&target) {
            Ok(metadata) => {
                ensure!(
                    metadata.file_type().is_dir() && !metadata.file_type().is_symlink(),
                    "published flow {} is not a non-symlink directory",
                    target.display()
                );
                true
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(error).context("inspecting published flow target"),
        };
        let staging = allocate_staging_directory(root, flow)?;
        let mut transaction = Self {
            root: root.to_path_buf(),
            target,
            staging,
            target_existed_at_prepare,
            published: false,
        };
        if transaction.target_existed_at_prepare
            && let Err(error) = copy_reviewed_metadata(&transaction.target, &transaction.staging)
        {
            let _ = fs::remove_dir_all(&transaction.staging);
            transaction.published = true;
            return Err(error);
        }
        Ok(transaction)
    }

    /// Directory into which all derived artifacts and the lineage marker must
    /// be written and validated before publication.
    #[must_use]
    pub fn staging_dir(&self) -> &Path {
        &self.staging
    }

    /// Atomically publish the entire complete flow. Existing flows use Linux
    /// `RENAME_EXCHANGE`, so a process death exposes either the old complete
    /// tree or the new complete tree, never a mixture of their files.
    pub fn publish(mut self) -> Result<()> {
        sync_tree(&self.staging)?;
        if self.target_existed_at_prepare {
            atomic_exchange_directories(&self.staging, &self.target)?;
            self.published = true;
            sync_directory(&self.root)?;
            if let Err(error) = fs::remove_dir_all(&self.staging) {
                eprintln!(
                    "capture-diff: warning: imported flow is complete, but old staging tree {} could not be removed: {error}",
                    self.staging.display()
                );
            } else {
                sync_directory(&self.root)?;
            }
        } else {
            atomic_publish_new_directory(&self.staging, &self.target)?;
            self.published = true;
            sync_directory(&self.root)?;
        }
        Ok(())
    }
}

impl Drop for AtomicFlowImport {
    fn drop(&mut self) {
        if !self.published {
            let _ = fs::remove_dir_all(&self.staging);
        }
    }
}

pub(super) fn allocate_staging_directory(root: &Path, flow: &str) -> Result<PathBuf> {
    for _ in 0..100_u64 {
        let candidate = root.join(format!(
            ".{flow}.import-partial.{}.{}",
            std::process::id(),
            STAGING_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("creating staging flow {}", candidate.display()));
            }
        }
    }
    bail!("could not allocate a staging directory for flow {flow:?}")
}

pub(super) fn is_generated_entry(name: &str) -> bool {
    matches!(
        name,
        "cpp.pkt" | "rust" | "expected-divergences.json" | LINEAGE_FILE | RAW_PROVENANCE_DIR
    )
}

pub(super) fn copy_reviewed_metadata(source: &Path, destination: &Path) -> Result<()> {
    let flow = source
        .file_name()
        .and_then(|name| name.to_str())
        .context("published flow name is not UTF-8")?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| anyhow::anyhow!("flow metadata filename is not UTF-8"))?;
        if is_generated_entry(&name) {
            continue;
        }
        if flow == "detour-chase-around-obstacle" && name == "fixture" {
            copy_detour_fixture_metadata(&entry.path(), &destination.join(&name))?;
            continue;
        }
        if flow == "creature-spell-casting" && name == "fixture" {
            copy_creature_spell_fixture_metadata(&entry.path(), &destination.join(&name))?;
            continue;
        }
        ensure!(
            matches!(
                name.as_str(),
                "README.md" | "flow.json" | "requirement.json"
            ),
            "flow contains unknown non-generated entry {name:?}; import copies only reviewed flow metadata"
        );
        copy_tree_entry(&entry.path(), &destination.join(name))?;
    }
    Ok(())
}

pub(super) fn copy_creature_spell_fixture_metadata(
    source: &Path,
    destination: &Path,
) -> Result<()> {
    validate_creature_spell_fixture_files(source)?;

    fs::create_dir(destination)?;
    for name in ["fixture.json", "cpp-reference.patch"] {
        copy_tree_entry(&source.join(name), &destination.join(name))?;
    }
    Ok(())
}

pub(super) fn validate_creature_spell_fixture_files(source: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    ensure!(
        metadata.file_type().is_dir() && !metadata.file_type().is_symlink(),
        "creature-spell fixture is not a non-symlink directory"
    );
    let mut root_names = fs::read_dir(source)?
        .map(|entry| {
            entry?
                .file_name()
                .into_string()
                .map_err(|_| std::io::Error::other("creature-spell fixture filename is not UTF-8"))
        })
        .collect::<std::io::Result<Vec<_>>>()?;
    root_names.sort();
    ensure!(
        root_names == ["cpp-reference.patch", "fixture.json"],
        "creature-spell fixture contains an unreviewed root entry"
    );
    let manifest_bytes = read_regular_file(&source.join("fixture.json"))?;
    let patch_bytes = read_regular_file(&source.join("cpp-reference.patch"))?;
    ensure!(
        sha256_bytes(&manifest_bytes) == CREATURE_SPELL_FIXTURE_MANIFEST_SHA256,
        "creature-spell fixture manifest differs from the reviewed bytes"
    );
    ensure!(
        sha256_bytes(&patch_bytes) == CREATURE_SPELL_CPP_PATCH_SHA256,
        "creature-spell C++ reference patch differs from the reviewed bytes"
    );
    let fixture: serde_json::Value = serde_json::from_slice(&manifest_bytes)
        .context("parsing reviewed creature-spell fixture manifest")?;
    ensure!(
        fixture
            .get("schema_version")
            .and_then(serde_json::Value::as_u64)
            == Some(2)
            && fixture.get("flow").and_then(serde_json::Value::as_str)
                == Some("creature-spell-casting")
            && fixture.get("contract").and_then(serde_json::Value::as_str)
                == Some(CREATURE_SPELL_FIXTURE_CONTRACT)
            && fixture.get("creature_template")
                == Some(&serde_json::json!({
                    "entry": 22_378,
                    "name": "Cabal Interrogator",
                    "original_ai_name": "SmartAI",
                    "temporary_ai_name": "CombatAI",
                    "script_name": "",
                    "verified_build": 52_237
                }))
            && fixture.get("creature_template_difficulty")
                == Some(&serde_json::json!({
                    "entry": 22_378,
                    "difficulty_id": 0,
                    "min_level": 64,
                    "max_level": 65,
                    "health_scaling_expansion": 0,
                    "health_modifier": 1.0,
                    "mana_modifier": 1.0,
                    "armor_modifier": 1.0,
                    "damage_modifier": 1.0,
                    "creature_difficulty_id": 18_203,
                    "type_flags": 0,
                    "type_flags_2": 0,
                    "loot_id": 22_378,
                    "pickpocket_loot_id": 22_378,
                    "skin_loot_id": 0,
                    "gold_min": 153,
                    "gold_max": 205,
                    "original_static_flags_1": 0,
                    "temporary_static_flags_1": 0x0010_0000,
                    "static_flags_2": 0,
                    "static_flags_3": 0,
                    "static_flags_4": 0,
                    "static_flags_5": 0,
                    "static_flags_6": 0,
                    "static_flags_7": 0,
                    "static_flags_8": 0
                })),
        "reviewed creature-spell fixture is not the exact shell guard v2 SmartAI/0 -> CombatAI/NO_MELEE contract"
    );
    let derivation = fixture
        .get("source_derivation")
        .context("reviewed creature-spell fixture source_derivation is missing")?;
    ensure!(
        derivation
            .get("contract")
            .and_then(serde_json::Value::as_str)
            == Some(CREATURE_SPELL_CPP_SOURCE_DERIVATION_CONTRACT)
            && derivation
                .get("remote_url")
                .and_then(serde_json::Value::as_str)
                == Some(CREATURE_SPELL_CPP_REMOTE_URL)
            && derivation
                .get("remote_ref")
                .and_then(serde_json::Value::as_str)
                == Some(CREATURE_SPELL_CPP_REMOTE_REF)
            && derivation
                .get("base_head")
                .and_then(serde_json::Value::as_str)
                == Some(CREATURE_SPELL_CPP_BASE_HEAD)
            && derivation
                .get("base_tree")
                .and_then(serde_json::Value::as_str)
                == Some(CREATURE_SPELL_CPP_BASE_TREE)
            && derivation
                .get("patched_head")
                .and_then(serde_json::Value::as_str)
                == Some(CREATURE_SPELL_CPP_PATCHED_HEAD)
            && derivation
                .get("patched_tree")
                .and_then(serde_json::Value::as_str)
                == Some(CREATURE_SPELL_CPP_PATCHED_TREE)
            && derivation
                .get("patch_path")
                .and_then(serde_json::Value::as_str)
                == Some(CREATURE_SPELL_CPP_PATCH_PATH)
            && derivation
                .get("patch_sha256")
                .and_then(serde_json::Value::as_str)
                == Some(CREATURE_SPELL_CPP_PATCH_SHA256)
            && derivation.get("changed_paths")
                == Some(&serde_json::json!([CREATURE_SPELL_CPP_CHANGED_PATH])),
        "reviewed creature-spell fixture source_derivation metadata differs from the pinned patch"
    );
    Ok(())
}

pub(super) fn copy_detour_fixture_metadata(source: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    ensure!(
        metadata.file_type().is_dir() && !metadata.file_type().is_symlink(),
        "detour fixture is not a non-symlink directory"
    );
    let mut root_names = fs::read_dir(source)?
        .map(|entry| {
            entry?
                .file_name()
                .into_string()
                .map_err(|_| std::io::Error::other("detour fixture filename is not UTF-8"))
        })
        .collect::<std::io::Result<Vec<_>>>()?;
    root_names.sort();
    ensure!(
        root_names == ["fixture.json", "mmaps"],
        "detour fixture contains an unreviewed root entry"
    );
    let mmaps = source.join("mmaps");
    let mmaps_metadata = fs::symlink_metadata(&mmaps)?;
    ensure!(
        mmaps_metadata.file_type().is_dir() && !mmaps_metadata.file_type().is_symlink(),
        "detour fixture mmaps is not a non-symlink directory"
    );
    let mut mmap_names = fs::read_dir(&mmaps)?
        .map(|entry| {
            entry?
                .file_name()
                .into_string()
                .map_err(|_| std::io::Error::other("detour MMap filename is not UTF-8"))
        })
        .collect::<std::io::Result<Vec<_>>>()?;
    mmap_names.sort();
    ensure!(
        mmap_names == ["0001.mmap", "00015026.mmtile"],
        "detour fixture contains an unreviewed MMap entry"
    );
    let manifest_bytes = read_regular_file(&source.join("fixture.json"))?;
    let map_bytes = read_regular_file(&mmaps.join("0001.mmap"))?;
    let tile_bytes = read_regular_file(&mmaps.join("00015026.mmtile"))?;
    ensure!(
        sha256_bytes(&manifest_bytes) == DETOUR_FIXTURE_MANIFEST_SHA256,
        "detour fixture manifest differs from the reviewed bytes"
    );
    ensure!(
        map_bytes.len() == 28 && sha256_bytes(&map_bytes) == DETOUR_FIXTURE_MAP_SHA256,
        "detour fixture map header differs from the reviewed bytes"
    );
    ensure!(
        tile_bytes.len() == 1_496 && sha256_bytes(&tile_bytes) == DETOUR_FIXTURE_TILE_SHA256,
        "detour fixture tile differs from the reviewed bytes"
    );

    fs::create_dir(destination)?;
    copy_tree_entry(
        &source.join("fixture.json"),
        &destination.join("fixture.json"),
    )?;
    fs::create_dir(destination.join("mmaps"))?;
    for name in mmap_names {
        copy_tree_entry(&mmaps.join(&name), &destination.join("mmaps").join(name))?;
    }
    Ok(())
}

pub(super) fn copy_tree_entry(source: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    ensure!(
        !metadata.file_type().is_symlink(),
        "flow metadata contains symlink {}",
        source.display()
    );
    if metadata.file_type().is_file() {
        fs::copy(source, destination).with_context(|| {
            format!(
                "copying flow metadata {} to {}",
                source.display(),
                destination.display()
            )
        })?;
        Ok(())
    } else {
        bail!(
            "reviewed flow metadata must be a regular file: {}",
            source.display()
        )
    }
}

pub(super) fn sync_tree(root: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(root)?;
    ensure!(
        metadata.file_type().is_dir(),
        "{} is not a directory",
        root.display()
    );
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        ensure!(
            !metadata.file_type().is_symlink(),
            "staging tree contains symlink {}",
            path.display()
        );
        if metadata.file_type().is_dir() {
            sync_tree(&path)?;
        } else if metadata.file_type().is_file() {
            File::open(&path)?.sync_all()?;
        } else {
            bail!("staging tree contains unsupported entry {}", path.display());
        }
    }
    sync_directory(root)
}

#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
pub(super) fn atomic_exchange_directories(left: &Path, right: &Path) -> Result<()> {
    use std::os::unix::ffi::OsStrExt as _;

    let left = CString::new(left.as_os_str().as_bytes()).context("staging path contains NUL")?;
    let right = CString::new(right.as_os_str().as_bytes()).context("target path contains NUL")?;
    // SAFETY: both C strings remain alive for the call, contain terminating
    // NUL bytes supplied by `CString`, and `renameat2` does not retain them.
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            left.as_ptr(),
            libc::AT_FDCWD,
            right.as_ptr(),
            libc::RENAME_EXCHANGE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error()).with_context(|| {
            format!(
                "atomically exchanging staged flow {} with {}",
                left.to_string_lossy(),
                right.to_string_lossy()
            )
        })
    }
}

#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
pub(super) fn atomic_publish_new_directory(source: &Path, target: &Path) -> Result<()> {
    use std::os::unix::ffi::OsStrExt as _;

    let source =
        CString::new(source.as_os_str().as_bytes()).context("staging path contains NUL")?;
    let target = CString::new(target.as_os_str().as_bytes()).context("target path contains NUL")?;
    // SAFETY: both C strings remain alive and NUL-terminated for renameat2;
    // RENAME_NOREPLACE makes a concurrent target creation fail atomically.
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            target.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error()).with_context(|| {
            format!(
                "atomically publishing staged flow {} as new target {} without replacement",
                source.to_string_lossy(),
                target.to_string_lossy()
            )
        })
    }
}

#[cfg(not(target_os = "linux"))]
pub(super) fn atomic_exchange_directories(_left: &Path, _right: &Path) -> Result<()> {
    bail!("replacing an existing flow atomically requires Linux renameat2(RENAME_EXCHANGE)")
}

#[cfg(not(target_os = "linux"))]
pub(super) fn atomic_publish_new_directory(_source: &Path, _target: &Path) -> Result<()> {
    bail!("publishing a new flow without replacement requires Linux renameat2(RENAME_NOREPLACE)")
}
