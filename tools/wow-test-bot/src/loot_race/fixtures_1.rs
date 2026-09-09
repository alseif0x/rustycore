//! Loot-race fixtures operations.
//!
//! Moved out of loot_race.rs under #634. Behaviour is preserved.

use super::*;

pub(crate) fn configured_fixture_journal_path() -> Result<PathBuf> {
    let raw = std::env::var(FIXTURE_JOURNAL_ENV).map_err(|_| {
        anyhow!("loot workflows require {FIXTURE_JOURNAL_ENV}=<absolute recovery-journal path>")
    })?;
    let path = PathBuf::from(raw);
    if !path.is_absolute() || path.file_name().is_none() {
        bail!("{FIXTURE_JOURNAL_ENV} must name an absolute file path");
    }
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("fixture journal has no parent directory"))?;
    let metadata = fs::symlink_metadata(parent)
        .with_context(|| format!("Inspect fixture-journal directory {}", parent.display()))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        bail!(
            "fixture-journal parent {} must be a real directory",
            parent.display()
        );
    }
    Ok(path)
}
pub(crate) fn cleanup_marker_path(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.cleanup-complete", path.display()))
}
pub(crate) fn validate_cleanup_marker(path: &Path, expected_digest: Option<&str>) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("Inspect cleanup marker {}", path.display()))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        bail!("cleanup marker must be a regular non-symlink file");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.mode() & 0o777 != 0o600 {
            bail!("cleanup marker permissions must be exactly 0600");
        }
    }
    let marker: CleanupMarkerRecord = serde_json::from_slice(
        &fs::read(path).with_context(|| format!("Read cleanup marker {}", path.display()))?,
    )
    .context("Parse cleanup marker")?;
    if marker.version != FIXTURE_JOURNAL_VERSION
        || marker.journal_sha256.len() != 64
        || !marker
            .journal_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        || expected_digest.is_some_and(|expected| marker.journal_sha256 != expected)
    {
        bail!("cleanup marker does not match the durable journal contract");
    }
    Ok(())
}
pub(crate) async fn recover_pending_fixture() -> Result<()> {
    if !recover_pending_fixture_if_present().await? {
        let path = configured_fixture_journal_path()?;
        bail!("no pending fixture journal exists at {}", path.display());
    }
    Ok(())
}
pub(crate) async fn recover_pending_fixture_if_present() -> Result<bool> {
    let path = configured_fixture_journal_path()?;
    if fs::symlink_metadata(&path).is_err() {
        let marker = cleanup_marker_path(&path);
        if fs::symlink_metadata(&marker).is_ok() {
            validate_cleanup_marker(&marker, None)?;
            return Ok(true);
        }
        return Ok(false);
    }
    tokio::time::timeout(
        Duration::from_secs(LOOT_CLEANUP_TIMEOUT_SECS),
        tokio::task::spawn_blocking(move || {
            let fixture = FixtureJournal::load(path)?;
            cleanup_fixture(&fixture)
        }),
    )
    .await
    .map_err(|_| anyhow!("fixture recovery exceeded {LOOT_CLEANUP_TIMEOUT_SECS}s"))?
    .map_err(|error| anyhow!("fixture recovery DB worker join failed: {error}"))??;
    Ok(true)
}
pub(crate) fn generated_fixture_health_like_cpp(base_health: u32, health_modifier: f32) -> u32 {
    (base_health as f32 * health_modifier).ceil() as u32
}
pub(crate) fn validate_guarded_fixture_health(
    health_modifier: f32,
    generated_max_health: u32,
) -> Result<()> {
    if (health_modifier - GUARDED_FIXTURE_HEALTH_MODIFIER).abs() > 0.000_000_1
        || generated_max_health != 1
    {
        bail!(
            "loot fixture must be loaded through the pre-start health guard: expected HealthModifier {GUARDED_FIXTURE_HEALTH_MODIFIER} and generated base health 1, got modifier {health_modifier} and health {generated_max_health}; run this through ./tools/qa-runtime.sh loot-item, which arms the guard before the world starts and restores it afterwards (#373)"
        );
    }
    Ok(())
}
pub(crate) fn prepare_gameobject_race_fixture(
    bots: &[config::BotConfig],
    cli: &LootRaceCli,
    shutdown: &CancellationToken,
) -> Result<LootRaceFixture> {
    if shutdown.is_cancelled() {
        bail!("loot-race cancelled before GameObject fixture preflight");
    }
    if (cli.entry, cli.spawn_guid, cli.item_entry)
        != (
            DEFAULT_CREATURE_ENTRY,
            DEFAULT_CREATURE_SPAWN_GUID,
            DEFAULT_ITEM_ENTRY,
        )
    {
        bail!("GameObject race fixture did not match the pinned wrapper-owned contract");
    }
    let world_url = world_db_url()?;
    let world_opts = loot_db_opts(&world_url, "world")?;
    let mut world = mysql::Conn::new(world_opts)
        .map_err(|error| anyhow!("Connect to world DB failed: {error}"))?;
    let spawn: mysql::Row = world
        .exec_first(
            "SELECT guid, id, map, zoneId, areaId, spawnDifficulties, phaseUseFlags, \
                    PhaseId, PhaseGroup, terrainSwapMap, position_x, position_y, position_z, \
                    orientation, rotation0, rotation1, rotation2, rotation3, spawntimesecs, \
                    animprogress, state, ScriptName, StringId, VerifiedBuild \
             FROM gameobject WHERE guid = ?",
            (cli.spawn_guid,),
        )
        .map_err(|error| anyhow!("Load wrapper-owned GameObject race spawn: {error}"))?
        .ok_or_else(|| {
            anyhow!(
                "wrapper-owned world.gameobject spawn {} is absent; start the world through the loot-race fixture guard",
                cli.spawn_guid
            )
        })?;
    let spawn_guid: u64 = required_row_value(&spawn, "guid")?;
    let entry: u32 = required_row_value(&spawn, "id")?;
    let map_id: u16 = required_row_value(&spawn, "map")?;
    let zone_id: u16 = required_row_value(&spawn, "zoneId")?;
    let area_id: u16 = required_row_value(&spawn, "areaId")?;
    let difficulties: String = required_row_value(&spawn, "spawnDifficulties")?;
    let phase_flags: u8 = required_row_value(&spawn, "phaseUseFlags")?;
    let phase_id: i32 = required_row_value(&spawn, "PhaseId")?;
    let phase_group: i32 = required_row_value(&spawn, "PhaseGroup")?;
    let terrain_swap_map: i32 = required_row_value(&spawn, "terrainSwapMap")?;
    let x: f64 = required_row_value(&spawn, "position_x")?;
    let y: f64 = required_row_value(&spawn, "position_y")?;
    let z: f64 = required_row_value(&spawn, "position_z")?;
    let orientation: f32 = required_row_value(&spawn, "orientation")?;
    let rotations = [
        required_row_value::<f32>(&spawn, "rotation0")?,
        required_row_value::<f32>(&spawn, "rotation1")?,
        required_row_value::<f32>(&spawn, "rotation2")?,
        required_row_value::<f32>(&spawn, "rotation3")?,
    ];
    let spawntime: u32 = required_row_value(&spawn, "spawntimesecs")?;
    let anim_progress: u8 = required_row_value(&spawn, "animprogress")?;
    let state: u8 = required_row_value(&spawn, "state")?;
    let spawn_script: String = required_row_value(&spawn, "ScriptName")?;
    let string_id: Option<String> = required_row_value(&spawn, "StringId")?;
    let verified_build: i32 = required_row_value(&spawn, "VerifiedBuild")?;
    let position_matches = (x - RACE_GAMEOBJECT_X).abs() <= 0.01
        && (y - RACE_GAMEOBJECT_Y).abs() <= 0.01
        && (z - RACE_GAMEOBJECT_Z).abs() <= 0.01;
    if spawn_guid != DEFAULT_CREATURE_SPAWN_GUID
        || entry != DEFAULT_CREATURE_ENTRY
        || map_id != RACE_GAMEOBJECT_MAP_ID
        || zone_id != 0
        || area_id != 0
        || difficulties != "0"
        || phase_flags != 0
        || phase_id != 0
        || phase_group != 0
        || terrain_swap_map != -1
        || !position_matches
        || orientation.abs() > f32::EPSILON
        || rotations
            .iter()
            .any(|rotation| rotation.abs() > f32::EPSILON)
        || spawntime != RACE_GAMEOBJECT_RESPAWN_SECS
        || anim_progress != RACE_GAMEOBJECT_ANIM_PROGRESS
        || state != RACE_GAMEOBJECT_STATE
        || !spawn_script.is_empty()
        || string_id.is_some()
        || verified_build != 0
    {
        bail!(
            "wrapper-owned GameObject spawn drifted from the exact QA contract: guid={spawn_guid} entry={entry} map={map_id} zone={zone_id} area={area_id} difficulties={difficulties:?} phase={phase_flags}/{phase_id}/{phase_group} terrain={terrain_swap_map} pos=({x},{y},{z}) orientation={orientation} rotations={rotations:?} respawn={spawntime} anim={anim_progress} state={state} script={spawn_script:?} string_id={string_id:?} build={verified_build}"
        );
    }
    let same_entry_map_spawns: Vec<u64> = world
        .exec_map(
            "SELECT guid FROM gameobject WHERE id = ? AND map = ? ORDER BY guid",
            (entry, map_id),
            |guid: u64| guid,
        )
        .map_err(|error| anyhow!("Check GameObject race map/entry spawn uniqueness: {error}"))?;
    validate_unique_sql_spawn(&same_entry_map_spawns, spawn_guid, entry, map_id)?;

    let template: mysql::Row = world
        .exec_first(
            "SELECT type, displayId, name, size, \
                    Data0, Data1, Data2, Data3, Data4, Data5, Data6, Data7, Data8, Data9, \
                    Data10, Data11, Data12, Data13, Data14, Data15, Data16, Data17, Data18, Data19, \
                    Data20, Data21, Data22, Data23, Data24, Data25, Data26, Data27, Data28, Data29, \
                    Data30, Data31, Data32, Data33, Data34, ContentTuningId, AIName, ScriptName, \
                    StringId, VerifiedBuild \
             FROM gameobject_template WHERE entry = ?",
            (entry,),
        )
        .map_err(|error| anyhow!("Load Tattered Chest template: {error}"))?
        .ok_or_else(|| anyhow!("Tattered Chest template {entry} is absent"))?;
    let go_type: u8 = required_row_value(&template, "type")?;
    let display_id: u32 = required_row_value(&template, "displayId")?;
    let name: String = required_row_value(&template, "name")?;
    let size: f32 = required_row_value(&template, "size")?;
    let mut template_data = [0_i32; 35];
    for (index, value) in template_data.iter_mut().enumerate() {
        *value = required_row_value(&template, &format!("Data{index}"))?;
    }
    let content_tuning_id: u32 = required_row_value(&template, "ContentTuningId")?;
    let ai_name: String = required_row_value(&template, "AIName")?;
    let template_script: String = required_row_value(&template, "ScriptName")?;
    let template_string_id: Option<String> = required_row_value(&template, "StringId")?;
    let template_build: i32 = required_row_value(&template, "VerifiedBuild")?;
    if go_type != 3
        || display_id != 259
        || name != "Tattered Chest"
        || (size - 1.0).abs() > f32::EPSILON
        || template_data != RACE_GAMEOBJECT_TEMPLATE_DATA
        || content_tuning_id != 0
        || !ai_name.is_empty()
        || !template_script.is_empty()
        || template_string_id.is_some()
        || template_build != 11_723
    {
        bail!(
            "Tattered Chest template/data drifted from the exact shared C++ fixture contract: type={go_type} display={display_id} name={name:?} size={size} data={template_data:?} content_tuning={content_tuning_id} ai={ai_name:?} script={template_script:?} string_id={template_string_id:?} build={template_build}"
        );
    }

    let loot_rows: Vec<(u32, u32, f32, u8, u16, u8, u8, u8)> = world
        .exec(
            "SELECT Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount \
             FROM gameobject_loot_template WHERE Entry = ? ORDER BY Item, Reference",
            (RACE_GAMEOBJECT_LOOT_ID,),
        )
        .map_err(|error| anyhow!("Load Tattered Chest loot template: {error}"))?;
    if loot_rows.len() != 1
        || loot_rows[0].0 != DEFAULT_ITEM_ENTRY
        || loot_rows[0].1 != 0
        || (loot_rows[0].2 - 100.0).abs() > f32::EPSILON
        || loot_rows[0].3 != 0
        || loot_rows[0].4 != 1
        || loot_rows[0].5 != 0
        || loot_rows[0].6 != 1
        || loot_rows[0].7 != 1
    {
        bail!(
            "GameObject loot id {RACE_GAMEOBJECT_LOOT_ID} was not exactly one unconditional item-{DEFAULT_ITEM_ENTRY} grant: {loot_rows:?}"
        );
    }
    let loot_conditions: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM conditions \
             WHERE SourceTypeOrReferenceId = 4 AND SourceGroup = ?",
            (RACE_GAMEOBJECT_LOOT_ID,),
        )
        .map_err(|error| anyhow!("Check Tattered Chest loot conditions: {error}"))?
        .unwrap_or(0);
    let addon_rows: Vec<(u16, u32, u32, u32, i32, i32, i32, i32, i32, u32, u32)> = world
        .exec(
            "SELECT faction, flags, Mingold, Maxgold, artkit0, artkit1, artkit2, artkit3, artkit4, \
                    WorldEffectID, AIAnimKitID \
             FROM gameobject_template_addon WHERE entry = ?",
            (entry,),
        )
        .map_err(|error| anyhow!("Load guarded Tattered Chest addon: {error}"))?;
    if addon_rows.as_slice()
        != [(
            RACE_GAMEOBJECT_ADDON_FACTION,
            0,
            RACE_GAMEOBJECT_MONEY,
            RACE_GAMEOBJECT_MONEY,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        )]
    {
        bail!(
            "wrapper-owned GameObject addon must match exact faction/flags/artkits/effects and deterministic money {0}/{0}, got {addon_rows:?}",
            RACE_GAMEOBJECT_MONEY
        );
    }
    let event_rows: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM game_event_gameobject WHERE guid = ?",
            (spawn_guid,),
        )
        .map_err(|error| anyhow!("Check GameObject event ownership: {error}"))?
        .unwrap_or(0);
    let pool_rows: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM pool_members WHERE type = 1 AND spawnId = ?",
            (spawn_guid,),
        )
        .map_err(|error| anyhow!("Check GameObject pool ownership: {error}"))?
        .unwrap_or(0);
    let linked_rows: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM linked_respawn WHERE guid = ? OR linkedGuid = ?",
            (spawn_guid, spawn_guid),
        )
        .map_err(|error| anyhow!("Check GameObject linked respawn: {error}"))?
        .unwrap_or(0);
    let spawn_addon_rows: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM gameobject_addon WHERE guid = ?",
            (spawn_guid,),
        )
        .map_err(|error| anyhow!("Check GameObject spawn addon ownership: {error}"))?
        .unwrap_or(0);
    let override_rows: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM gameobject_overrides WHERE spawnId = ?",
            (spawn_guid,),
        )
        .map_err(|error| anyhow!("Check GameObject override ownership: {error}"))?
        .unwrap_or(0);
    let spawn_group_rows: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM spawn_group WHERE spawnType = 1 AND spawnId = ?",
            (spawn_guid,),
        )
        .map_err(|error| anyhow!("Check GameObject spawn-group ownership: {error}"))?
        .unwrap_or(0);
    if loot_conditions != 0
        || event_rows != 0
        || pool_rows != 0
        || linked_rows != 0
        || spawn_addon_rows != 0
        || override_rows != 0
        || spawn_group_rows != 0
    {
        bail!(
            "GameObject fixture has conditional/spawn metadata: conditions={loot_conditions} event={event_rows} pool={pool_rows} linked={linked_rows} addon={spawn_addon_rows} override={override_rows} spawn_group={spawn_group_rows}"
        );
    }

    let character_url = characters_db_url()?;
    let character_opts = loot_db_opts(&character_url, "characters")?;
    let mut character_db = mysql::Conn::new(character_opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    ensure_no_online_characters(&mut character_db, "GameObject loot-race fixture setup")?;
    let respawn_rows: Vec<(i64, u16, u32)> = character_db
        .exec(
            "SELECT respawnTime, mapId, instanceId FROM respawn \
             WHERE type = 1 AND spawnId = ? ORDER BY mapId, instanceId",
            (spawn_guid,),
        )
        .map_err(|error| anyhow!("Load GameObject respawn snapshot: {error}"))?;
    if !respawn_rows.is_empty() {
        bail!(
            "GameObject spawn {spawn_guid} already has {} type=1 respawn row(s); restart through a clean fixture guard",
            respawn_rows.len()
        );
    }

    let (characters, progress) = snapshot_loot_fixture_characters(
        &mut character_db,
        bots,
        DEFAULT_ITEM_ENTRY,
        0,
        RACE_GAMEOBJECT_MONEY,
        None,
    )?;
    info!(
        "Loot-race shared GameObject fixture: `{name}` entry={entry} spawn={spawn_guid} map={map_id} runtime=wire-discovered loot={RACE_GAMEOBJECT_LOOT_ID} item={DEFAULT_ITEM_ENTRY} money={RACE_GAMEOBJECT_MONEY} respawn={spawntime}s state={state}; C++ GameObject.cpp shared group-rules path"
    );
    let journal = FixtureJournal::configured()?;
    let fixture = LootRaceFixture {
        characters,
        target: LootRaceTarget {
            kind: LootRaceTargetKind::GameObject,
            entry,
            spawn_guid,
            runtime_counter_override: cli.runtime_counter,
            map_id,
            x,
            y,
            z,
            item_entry: DEFAULT_ITEM_ENTRY,
        },
        respawn: None,
        respawn_type: 1,
        gameobject_state: Some(state),
        progress,
        journal,
    };
    if shutdown.is_cancelled() {
        bail!("loot-race cancelled before durable fixture snapshot");
    }
    fixture.journal.persist(&fixture)?;
    if shutdown.is_cancelled() {
        bail!("loot-race cancelled after durable snapshot and before relocation");
    }
    relocate_loot_fixture_characters(&mut character_db, &fixture.characters, map_id, x, y, z)?;
    Ok(fixture)
}
pub(crate) fn prepare_fixture(
    bots: &[config::BotConfig],
    cli: &LootRaceCli,
    purpose: LootFixturePurpose,
    shutdown: &CancellationToken,
) -> Result<LootRaceFixture> {
    if shutdown.is_cancelled() {
        bail!("loot workflow cancelled before fixture preflight");
    }
    if bots.len() != 2 {
        bail!("internal loot-race setup requires exactly two bots");
    }
    for bot in bots {
        if !bot.account.to_ascii_uppercase().ends_with("@BOT.LOCAL") {
            bail!(
                "refusing destructive loot-race setup for non-local account {}",
                bot.account
            );
        }
    }

    if purpose == LootFixturePurpose::Race {
        return prepare_gameobject_race_fixture(bots, cli, shutdown);
    }

    let world_url = world_db_url()?;
    let world_opts = loot_db_opts(&world_url, "world")?;
    let mut world = mysql::Conn::new(world_opts)
        .map_err(|error| anyhow!("Connect to world DB failed: {error}"))?;
    let row: mysql::Row = world
        .exec_first(
            "SELECT c.guid, c.id, c.map, c.spawnDifficulties, c.spawntimesecs, \
                    c.position_x, c.position_y, c.position_z, c.orientation, \
                    c.phaseUseFlags, c.PhaseId, c.PhaseGroup, c.wander_distance, c.MovementType, \
                    ct.name, ct.type, ct.unit_class, ct.unit_flags, ct.flags_extra, \
                    d.MinLevel, d.MaxLevel, d.HealthScalingExpansion, d.HealthModifier, \
                    d.LootID, d.GoldMin, d.GoldMax, \
                    d.StaticFlags1 \
             FROM creature c \
             JOIN creature_template ct ON ct.entry = c.id \
             JOIN creature_template_difficulty d ON d.Entry = c.id AND d.DifficultyID = 0 \
             WHERE c.guid = ?",
            (cli.spawn_guid,),
        )
        .map_err(|error| anyhow!("Load loot-race creature fixture: {error}"))?
        .ok_or_else(|| anyhow!("No world.creature row for spawn {}", cli.spawn_guid))?;
    let spawn_guid: u64 = required_row_value(&row, "guid")?;
    let entry: u32 = required_row_value(&row, "id")?;
    let map_id: u16 = required_row_value(&row, "map")?;
    let difficulties: String = required_row_value(&row, "spawnDifficulties")?;
    let spawntime: u32 = required_row_value(&row, "spawntimesecs")?;
    let x: f64 = required_row_value(&row, "position_x")?;
    let y: f64 = required_row_value(&row, "position_y")?;
    let z: f64 = required_row_value(&row, "position_z")?;
    let _orientation: f32 = required_row_value(&row, "orientation")?;
    let phase_flags: u8 = required_row_value(&row, "phaseUseFlags")?;
    let phase_id: i32 = required_row_value(&row, "PhaseId")?;
    let phase_group: i32 = required_row_value(&row, "PhaseGroup")?;
    let wander_distance: f32 = required_row_value(&row, "wander_distance")?;
    let movement_type: u8 = required_row_value(&row, "MovementType")?;
    let name: String = required_row_value(&row, "name")?;
    let creature_type: u8 = required_row_value(&row, "type")?;
    let unit_class: u8 = required_row_value(&row, "unit_class")?;
    let unit_flags: u32 = required_row_value(&row, "unit_flags")?;
    let flags_extra: u32 = required_row_value(&row, "flags_extra")?;
    let min_level: u8 = required_row_value(&row, "MinLevel")?;
    let max_level: u8 = required_row_value(&row, "MaxLevel")?;
    let health_scaling_expansion: i32 = required_row_value(&row, "HealthScalingExpansion")?;
    let health_modifier: f32 = required_row_value(&row, "HealthModifier")?;
    let loot_id: u32 = required_row_value(&row, "LootID")?;
    let min_gold: u32 = required_row_value(&row, "GoldMin")?;
    let max_gold: u32 = required_row_value(&row, "GoldMax")?;
    let static_flags1: u32 = required_row_value(&row, "StaticFlags1")?;
    if spawn_guid != cli.spawn_guid || entry != cli.entry || loot_id == 0 {
        bail!("creature fixture does not match the exact spawn/entry/loot-id contract");
    }
    let same_entry_map_spawns: Vec<u64> = world
        .exec_map(
            "SELECT guid FROM creature WHERE id = ? AND map = ? ORDER BY guid",
            (entry, map_id),
            |guid: u64| guid,
        )
        .map_err(|error| anyhow!("Check loot-race map/entry spawn uniqueness: {error}"))?;
    validate_unique_sql_spawn(&same_entry_map_spawns, cli.spawn_guid, entry, map_id)?;
    if !matches!(map_id, 0 | 1 | 530 | 571) {
        bail!("loot-race creature must be on a known overworld map, got map {map_id}");
    }
    if !difficulties
        .split(',')
        .any(|difficulty| difficulty.trim() == "0")
    {
        bail!("creature fixture is not available on overworld difficulty 0");
    }
    if phase_flags != 0 || phase_id != 0 || phase_group != 0 {
        bail!("creature fixture has phase behavior outside this focused smoke");
    }
    if wander_distance != 0.0 || movement_type != 0 {
        bail!("creature fixture must be stationary for deterministic two-client engagement");
    }
    if spawntime == 0 || spawntime > 3_600 {
        bail!("creature respawn interval {spawntime} is outside the acknowledged QA boundary");
    }
    if purpose != LootFixturePurpose::CaptureItem {
        bail!("internal error: creature fixture is reserved for loot-item capture");
    }
    if min_gold != 0 || max_gold != 0 {
        bail!("loot-item capture creature must have no random money pool");
    }
    if min_level == 0 || max_level < min_level || max_level > 70 {
        bail!("loot-item capture creature exceeds the bounded combat target level");
    }
    let health_expansion_index = match health_scaling_expansion {
        -1 | 2 => 2,
        0 => 0,
        1 => 1,
        value => bail!(
            "loot fixture has unsupported HealthScalingExpansion {value}; expected the C++ -1..=2 domain"
        ),
    };
    let base_health_rows: Vec<(u8, u32, u32, u32)> = world
        .exec(
            "SELECT level, basehp0, basehp1, basehp2 \
             FROM creature_classlevelstats \
             WHERE class = ? AND level BETWEEN ? AND ? ORDER BY level",
            (unit_class, min_level, max_level),
        )
        .map_err(|error| anyhow!("Load loot fixture class-level health: {error}"))?;
    let expected_level_rows = usize::from(max_level - min_level) + 1;
    if base_health_rows.len() != expected_level_rows {
        bail!(
            "loot fixture class {} level range {}..{} resolved {} class-level health row(s), expected {expected_level_rows}",
            unit_class,
            min_level,
            max_level,
            base_health_rows.len()
        );
    }
    let guarded_max_health = base_health_rows
        .iter()
        .map(|(_, hp0, hp1, hp2)| {
            let base_health = [*hp0, *hp1, *hp2][health_expansion_index].max(1);
            generated_fixture_health_like_cpp(base_health, health_modifier)
        })
        .max()
        .unwrap_or(0);
    validate_guarded_fixture_health(health_modifier, guarded_max_health)?;
    const UNATTACKABLE_UNIT_FLAGS: u32 = 0x0000_0002 | 0x0000_0100 | 0x0001_0000 | 0x0200_0000;
    const UNSUITABLE_STATIC_FLAGS1: u32 = 0x0000_0004 | 0x0000_0020 | 0x0000_0200;
    if unit_flags & UNATTACKABLE_UNIT_FLAGS != 0
        || static_flags1 & UNSUITABLE_STATIC_FLAGS1 != 0
        || creature_type == CREATURE_TYPE_CRITTER
    {
        bail!("creature fixture is not a normal attackable shared-loot target");
    }
    const UNSUITABLE_FLAGS_EXTRA: u32 = 0x0000_0080 | 0x0000_0400 | 0x0000_2000 | 0x0000_4000;
    if flags_extra & UNSUITABLE_FLAGS_EXTRA != 0 {
        bail!("creature fixture has trigger/ghost/no-combat/world-event flags");
    }
    let loot_rows: Vec<(u32, u32, f32, u8, u16, u8, u8, u8)> = world
        .exec(
            "SELECT Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount \
             FROM creature_loot_template WHERE Entry = ? ORDER BY Item, Reference",
            (loot_id,),
        )
        .map_err(|error| anyhow!("Load loot-race creature loot rows: {error}"))?;
    if purpose == LootFixturePurpose::CaptureItem && loot_rows.len() != 1 {
        bail!(
            "creature loot id {loot_id} contains {} rows; strict capture requires exactly one logical item pool",
            loot_rows.len()
        );
    }
    let matching_item_rows = loot_rows
        .iter()
        .filter(|&&(item, _, _, _, _, _, _, _)| item == cli.item_entry)
        .collect::<Vec<_>>();
    if matching_item_rows.len() != 1 {
        bail!(
            "creature loot id {loot_id} contains {} rows for expected item {}; expected exactly one",
            matching_item_rows.len(),
            cli.item_entry
        );
    }
    let &(_item, reference, chance, quest_required, loot_mode, group_id, min_count, max_count) =
        matching_item_rows[0];
    if reference != 0
        || (chance - 100.0).abs() > f32::EPSILON
        || quest_required != 0
        || loot_mode != 1
        || group_id != 0
        || min_count != 1
        || max_count != 1
    {
        bail!("expected creature item row is not one unconditional normal single-item grant");
    }
    let condition_rows: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM conditions \
             WHERE SourceTypeOrReferenceId = 1 AND SourceGroup = ?",
            (loot_id,),
        )
        .map_err(|error| anyhow!("Check loot-race creature loot conditions: {error}"))?
        .unwrap_or(0);
    if condition_rows != 0 {
        bail!("creature loot id {loot_id} has conditions outside this focused smoke");
    }
    let event_rows: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM game_event_creature WHERE guid = ?",
            (cli.spawn_guid,),
        )
        .map_err(|error| anyhow!("Check loot-race game-event ownership: {error}"))?
        .unwrap_or(0);
    let pool_rows: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM pool_members WHERE type = 0 AND spawnId = ?",
            (cli.spawn_guid,),
        )
        .map_err(|error| anyhow!("Check loot-race pool ownership: {error}"))?
        .unwrap_or(0);
    let linked_rows: u64 = world
        .exec_first(
            "SELECT COUNT(*) FROM linked_respawn WHERE guid = ? OR linkedGuid = ?",
            (cli.spawn_guid, cli.spawn_guid),
        )
        .map_err(|error| anyhow!("Check loot-race linked respawn: {error}"))?
        .unwrap_or(0);
    if event_rows != 0 || pool_rows != 0 || linked_rows != 0 {
        bail!("creature fixture is event/pool/linked managed and cannot be consumed safely");
    }

    let character_url = characters_db_url()?;
    let character_opts = loot_db_opts(&character_url, "characters")?;
    let mut character_db = mysql::Conn::new(character_opts)
        .map_err(|error| anyhow!("Connect to characters DB failed: {error}"))?;
    ensure_no_online_characters(&mut character_db, "loot fixture setup")?;
    let respawn_rows: Vec<(i64, u16, u32)> = character_db
        .exec(
            "SELECT respawnTime, mapId, instanceId FROM respawn \
             WHERE type = 0 AND spawnId = ? ORDER BY mapId, instanceId",
            (cli.spawn_guid,),
        )
        .map_err(|error| anyhow!("Load loot-race respawn snapshot: {error}"))?;
    if !respawn_rows.is_empty() {
        bail!(
            "creature spawn {} already has {} persisted respawn timer row(s); use a fresh runtime/fixture",
            cli.spawn_guid,
            respawn_rows.len()
        );
    }
    let respawn = respawn_rows.into_iter().next();

    let (fixture_characters, progress) = snapshot_loot_fixture_characters(
        &mut character_db,
        bots,
        cli.item_entry,
        max_level,
        max_gold,
        Some((bots[0].character_guid, LOOT_ITEM_CAPTURE_KEYRING_SLOT)),
    )?;

    info!(
        "Loot-race disposable fixture: `{}` entry={} spawn={} map={} runtime_counter={} level={}..{} guarded_base_health={} loot={} item={} gold={}..{} respawn={}s; live creature cannot be restored without restart",
        name,
        entry,
        cli.spawn_guid,
        map_id,
        if cli.runtime_counter == 0 {
            "auto".to_string()
        } else {
            cli.runtime_counter.to_string()
        },
        min_level,
        max_level,
        guarded_max_health,
        loot_id,
        cli.item_entry,
        min_gold,
        max_gold,
        spawntime,
    );
    let journal = FixtureJournal::configured()?;
    let fixture = LootRaceFixture {
        characters: fixture_characters,
        target: LootRaceTarget {
            kind: LootRaceTargetKind::Creature,
            entry,
            spawn_guid: cli.spawn_guid,
            runtime_counter_override: cli.runtime_counter,
            map_id,
            x,
            y,
            z,
            item_entry: cli.item_entry,
        },
        respawn: respawn.map(|(respawn_time, map_id, instance_id)| RespawnSnapshot {
            respawn_time,
            map_id,
            instance_id,
        }),
        respawn_type: 0,
        gameobject_state: None,
        progress,
        journal,
    };
    if shutdown.is_cancelled() {
        bail!("loot-item capture cancelled before durable fixture snapshot");
    }
    fixture.journal.persist(&fixture)?;
    if shutdown.is_cancelled() {
        bail!("loot-item capture cancelled after durable snapshot and before relocation");
    }
    relocate_loot_fixture_characters(&mut character_db, &fixture.characters, map_id, x, y, z)?;
    Ok(fixture)
}
