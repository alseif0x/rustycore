//! Misc operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransmogOutfitDbRow {
    pub(crate) set_guid: u64,
    pub(crate) set_index: u32,
    pub(crate) name: String,
    pub(crate) icon_name: String,
    pub(crate) ignore_mask: u32,
    pub(crate) appearances: [i32; EQUIPMENT_SET_SLOTS_LIKE_CPP],
    pub(crate) main_hand_enchant: i32,
    pub(crate) off_hand_enchant: i32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VendorInventoryItemWire {
    pub(crate) muid: i32,
    pub(crate) item_id: i32,
    pub(crate) item_type: i32,
    pub(crate) price: u64,
    pub(crate) stack_count: i32,
    pub(crate) extended_cost: i32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RestedXpSmokePhase {
    OfflineWilderness,
    OfflineResting,
    ConsumeKill,
    VerifyRelog,
}
#[derive(Debug, Clone)]
pub(crate) struct RestedXpSmokeOptions {
    pub(crate) phase: RestedXpSmokePhase,
    pub(crate) target: ResolvedCreatureTarget,
    pub(crate) target_match_radius: f32,
    pub(crate) test_level: u8,
    pub(crate) next_level_xp: u32,
    pub(crate) seeded_rest_bonus: f32,
    pub(crate) expected_xp: Option<u32>,
    pub(crate) expected_rest_bonus: Option<f32>,
    pub(crate) timeout_secs: u64,
}
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RestedXpCharacterRestorePoint {
    pub(crate) level: u8,
    pub(crate) xp: u32,
    pub(crate) rest_state: u8,
    pub(crate) player_flags: u32,
    pub(crate) rest_bonus: f32,
    pub(crate) logout_time: u64,
    pub(crate) is_logout_resting: u8,
    pub(crate) map_id: u32,
    pub(crate) zone_id: u32,
    pub(crate) instance_id: u32,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) z: f64,
    pub(crate) orientation: f32,
    pub(crate) health: u32,
    pub(crate) powers: [u32; 10],
    pub(crate) total_kills: u32,
    pub(crate) today_kills: u16,
    pub(crate) yesterday_kills: u16,
    pub(crate) total_time: u32,
    pub(crate) level_time: u32,
    pub(crate) latency: u32,
    pub(crate) last_login_build: u32,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RestedXpDbState {
    pub(crate) level: u8,
    pub(crate) xp: u32,
    pub(crate) rest_state: u8,
    pub(crate) rest_bonus: f32,
    pub(crate) online: u8,
}
#[derive(Debug, Clone)]
pub(crate) struct RestedXpSmokeFixture {
    pub(crate) options: RestedXpSmokeOptions,
    pub(crate) original: RestedXpCharacterRestorePoint,
    pub(crate) original_achievements: Vec<(u32, i64)>,
    pub(crate) original_achievement_progress: Vec<(u32, u64, i64)>,
    pub(crate) original_trait_configs: Vec<RestedXpTraitConfigSnapshot>,
    pub(crate) original_trait_entries: Vec<RestedXpTraitEntrySnapshot>,
    pub(crate) original_homebind: Option<RestedXpHomebindSnapshot>,
    pub(crate) original_fishing_steps: Option<u8>,
    pub(crate) original_battleground_data: Option<RestedXpBattlegroundDataSnapshot>,
    pub(crate) original_last_played_characters: Vec<RestedXpLastPlayedCharacterSnapshot>,
    pub(crate) original_battle_pet_slots: Vec<RestedXpBattlePetSlotSnapshot>,
    pub(crate) battlenet_account_id: u32,
    pub(crate) target_respawn_secs: u32,
    pub(crate) test_level: u8,
    pub(crate) offline_secs: u64,
    pub(crate) wilderness_rate: f32,
    pub(crate) resting_rate: f32,
}
pub(crate) type RestedXpTraitConfigSnapshot = (
    i32,
    i32,
    Option<i32>,
    Option<i32>,
    Option<i32>,
    Option<i32>,
    Option<i32>,
    String,
);
pub(crate) type RestedXpTraitEntrySnapshot = (i32, i32, i32, i32, i32);
pub(crate) type RestedXpHomebindSnapshot = (u16, u16, f32, f32, f32, f32);
pub(crate) type RestedXpBattlegroundDataSnapshot = (
    u32,
    u16,
    f32,
    f32,
    f32,
    f32,
    u16,
    u32,
    u32,
    u32,
    Option<u64>,
);
pub(crate) type RestedXpLastPlayedCharacterSnapshot = (
    u8,
    u8,
    Option<u32>,
    Option<String>,
    Option<u64>,
    Option<u32>,
);
pub(crate) type RestedXpBattlePetSlotSnapshot = (i8, i64, i8);

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct RestedXpFixtureSafetyState {
    pub(crate) at_login: u16,
    pub(crate) game_account_online: u8,
    pub(crate) bnet_email_matches_configured_account: bool,
    pub(crate) characters_on_game_account: u64,
    pub(crate) game_accounts_on_bnet_account: u64,
    pub(crate) nonempty_side_state: Vec<(String, u64)>,
}
#[derive(Debug, Clone)]
pub(crate) struct ResolvedCreatureTarget {
    pub(crate) entry: u32,
    pub(crate) spawn_guid: u64,
    pub(crate) guid_counter: u64,
    pub(crate) map_id: u16,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) z: f64,
    pub(crate) orientation: f32,
    pub(crate) packed_guid: Vec<u8>,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DiscoveredCreatureGuid {
    pub(crate) low: u64,
    pub(crate) high: u64,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) z: f32,
}
pub(crate) fn test_dungeon_id(app_config: &config::AppConfig) -> u32 {
    // Allow override via env var (handy for ad-hoc testing); otherwise pick up
    // the value from config.json::test_config.dungeon_id.
    std::env::var("WOW_BOT_DUNGEON_ID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            if app_config.test_config.dungeon_id != 0 {
                app_config.test_config.dungeon_id
            } else {
                DEFAULT_DUNGEON_ID
            }
        })
}
pub(crate) fn next_arg(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String> {
    args.next().ok_or_else(|| anyhow!("{} needs a value", flag))
}
pub(crate) fn parse_ack_disposable_rested_xp_arg(arg: &str, acknowledged: &mut bool) -> bool {
    if arg != ACK_DISPOSABLE_RESTED_XP_FLAG {
        return false;
    }

    *acknowledged = true;
    true
}
pub(crate) fn parse_ack_disposable_detour_fixture_arg(arg: &str, acknowledged: &mut bool) -> bool {
    if arg != ACK_DISPOSABLE_DETOUR_FIXTURE_FLAG {
        return false;
    }

    *acknowledged = true;
    true
}
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DetourFixtureManifest {
    pub(crate) schema_version: u32,
    pub(crate) flow: String,
    pub(crate) generator: DetourFixtureGenerator,
    pub(crate) map: DetourFixtureMap,
    pub(crate) geometry: DetourFixtureGeometry,
    pub(crate) creature: DetourFixtureCreature,
    pub(crate) character: DetourFixtureCharacter,
    pub(crate) assets: Vec<DetourFixtureAsset>,
}
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DetourFixtureGenerator {
    #[serde(rename = "crate")]
    pub(crate) crate_name: String,
    pub(crate) example: String,
    pub(crate) feature: String,
}
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DetourFixtureMap {
    pub(crate) id: u16,
    pub(crate) grid_x: u32,
    pub(crate) grid_y: u32,
}
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DetourFixtureGeometry {
    pub(crate) centre: DetourFixturePosition,
    pub(crate) creature_start: DetourFixturePosition,
    pub(crate) player_start: DetourFixtureOrientedPosition,
    pub(crate) player_destination: DetourFixtureOrientedPosition,
    pub(crate) obstacle: DetourFixtureObstacle,
}
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DetourFixturePosition {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) z: f32,
}
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DetourFixtureOrientedPosition {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) z: f32,
    pub(crate) orientation: f32,
}
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DetourFixtureObstacle {
    pub(crate) min_x: f32,
    pub(crate) max_x: f32,
    pub(crate) min_y: f32,
    pub(crate) max_y: f32,
}
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DetourFixtureCreature {
    pub(crate) entry: u32,
    pub(crate) spawn_guid: u64,
}
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DetourFixtureCharacter {
    pub(crate) guid: u64,
}
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DetourFixtureAsset {
    pub(crate) path: String,
    pub(crate) bytes: u64,
    pub(crate) sha256: String,
}
#[derive(Debug, Clone)]
pub(crate) struct DetourChaseCaptureOptions {
    pub(crate) map_id: u16,
    pub(crate) target_entry: u32,
    pub(crate) target_spawn_guid: u64,
    pub(crate) target_x: f32,
    pub(crate) target_y: f32,
    pub(crate) target_z: f32,
    pub(crate) player_start_x: f32,
    pub(crate) player_start_y: f32,
    pub(crate) player_start_z: f32,
    pub(crate) player_start_orientation: f32,
    pub(crate) destination_x: f32,
    pub(crate) destination_y: f32,
    pub(crate) destination_z: f32,
    pub(crate) destination_orientation: f32,
    pub(crate) timeout_secs: u64,
}
#[derive(Debug, Clone)]
pub(crate) struct CreatureSpellCaptureOptions {
    pub(crate) fixture_manifest_sha256: String,
    pub(crate) timeout_secs: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreatureSpellCastSummary {
    pub(crate) caster: (u64, u64),
    pub(crate) caster_unit: (u64, u64),
    pub(crate) cast_id: (u64, u64),
    pub(crate) original_cast_id: (u64, u64),
    pub(crate) spell_id: u32,
    pub(crate) cast_flags: u32,
    pub(crate) cast_flags_ex: u32,
    pub(crate) target: (u64, u64),
    pub(crate) hit_targets: Vec<(u64, u64)>,
    pub(crate) miss_targets: Vec<(u64, u64)>,
    pub(crate) miss_status_count: u16,
    pub(crate) full_combat_log: Option<bool>,
    pub(crate) consumed: usize,
}
pub(crate) fn detour_chase_options_from_pinned_manifest(
    manifest: &DetourFixtureManifest,
    timeout_secs: u64,
) -> Result<DetourChaseCaptureOptions> {
    if manifest != &expected_detour_fixture_manifest() {
        bail!("detour chase runner requires the exact pinned issue #24 fixture manifest");
    }
    if timeout_secs == 0 {
        bail!("detour chase runner timeout must be greater than zero");
    }
    Ok(DetourChaseCaptureOptions {
        map_id: manifest.map.id,
        target_entry: manifest.creature.entry,
        target_spawn_guid: manifest.creature.spawn_guid,
        target_x: manifest.geometry.creature_start.x,
        target_y: manifest.geometry.creature_start.y,
        target_z: manifest.geometry.creature_start.z,
        player_start_x: manifest.geometry.player_start.x,
        player_start_y: manifest.geometry.player_start.y,
        player_start_z: manifest.geometry.player_start.z,
        player_start_orientation: manifest.geometry.player_start.orientation,
        destination_x: manifest.geometry.player_destination.x,
        destination_y: manifest.geometry.player_destination.y,
        destination_z: manifest.geometry.player_destination.z,
        destination_orientation: manifest.geometry.player_destination.orientation,
        timeout_secs,
    })
}
pub(crate) fn expected_detour_fixture_manifest() -> DetourFixtureManifest {
    DetourFixtureManifest {
        schema_version: 1,
        flow: DETOUR_CHASE_FIXTURE_FLOW.to_string(),
        generator: DetourFixtureGenerator {
            crate_name: "wow-recastdetour".to_string(),
            example: "generate_detour_chase_fixture".to_string(),
            feature: "test-fixtures".to_string(),
        },
        map: DetourFixtureMap {
            id: 1,
            grid_x: 50,
            grid_y: 26,
        },
        geometry: DetourFixtureGeometry {
            centre: DetourFixturePosition {
                x: -10_118.333,
                y: 2_681.667,
                z: 218.49,
            },
            creature_start: DetourFixturePosition {
                x: -10_118.333,
                y: 2_671.667,
                z: 218.49,
            },
            player_start: DetourFixtureOrientedPosition {
                x: -10_118.333,
                y: 2_670.667,
                z: 218.49,
                orientation: std::f32::consts::FRAC_PI_2,
            },
            player_destination: DetourFixtureOrientedPosition {
                x: -10_118.333,
                y: 2_691.667,
                z: 218.49,
                orientation: -std::f32::consts::FRAC_PI_2,
            },
            obstacle: DetourFixtureObstacle {
                min_x: -10_123.333,
                max_x: -10_113.333,
                min_y: 2_676.667,
                max_y: 2_686.667,
            },
        },
        creature: DetourFixtureCreature {
            entry: 15_271,
            spawn_guid: 9_102_401,
        },
        character: DetourFixtureCharacter { guid: 15 },
        assets: vec![
            DetourFixtureAsset {
                path: "mmaps/0001.mmap".to_string(),
                bytes: 28,
                sha256: DETOUR_CHASE_MAP_ASSET_SHA256.to_string(),
            },
            DetourFixtureAsset {
                path: "mmaps/00015026.mmtile".to_string(),
                bytes: 1_496,
                sha256: DETOUR_CHASE_TILE_ASSET_SHA256.to_string(),
            },
        ],
    }
}
pub(crate) fn validate_detour_chase_cli_values(
    enabled: bool,
    acknowledged_disposable: bool,
    single_account: Option<&str>,
    timeout_secs: u64,
    manifest: Option<&str>,
) -> Result<()> {
    if !enabled {
        if acknowledged_disposable {
            bail!("{ACK_DISPOSABLE_DETOUR_FIXTURE_FLAG} is only valid with --detour-chase-capture");
        }
        return Ok(());
    }
    if !acknowledged_disposable {
        bail!(
            "--detour-chase-capture requires {ACK_DISPOSABLE_DETOUR_FIXTURE_FLAG}; this acknowledges that the deterministic map, creature, and character are disposable QA fixtures"
        );
    }
    if !single_account
        .is_some_and(|account| account.eq_ignore_ascii_case(DETOUR_CHASE_FIXTURE_ACCOUNT))
    {
        bail!("--detour-chase-capture requires --single {DETOUR_CHASE_FIXTURE_ACCOUNT}");
    }
    if timeout_secs == 0 {
        bail!("--detour-chase-timeout must be greater than zero");
    }
    if !manifest.is_some_and(|path| !path.trim().is_empty()) {
        bail!("--detour-chase-capture requires --detour-fixture-manifest <path>");
    }
    Ok(())
}
pub(crate) fn validate_detour_fixture_identity(account: &str, character_guid: u64) -> Result<()> {
    if !account.eq_ignore_ascii_case(DETOUR_CHASE_FIXTURE_ACCOUNT) || character_guid != 15 {
        bail!(
            "detour chase fixture requires account {DETOUR_CHASE_FIXTURE_ACCOUNT} with character guid 15; selected account={account} guid={character_guid}"
        );
    }
    Ok(())
}
pub(crate) fn detour_fixture_float_matches(actual: f64, expected: f32) -> bool {
    (actual - f64::from(expected)).abs() <= 0.02
}
pub(crate) fn validate_detour_fixture_manifest(path: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path)
        .with_context(|| format!("Read detour fixture manifest metadata {}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!(
            "Detour fixture manifest must be a regular file and not a symlink: {}",
            path.display()
        );
    }
    let canonical_manifest = path
        .canonicalize()
        .with_context(|| format!("Canonicalize detour fixture manifest {}", path.display()))?;
    let bytes = std::fs::read(&canonical_manifest).with_context(|| {
        format!(
            "Read detour fixture manifest {}",
            canonical_manifest.display()
        )
    })?;
    let manifest: DetourFixtureManifest = serde_json::from_slice(&bytes).with_context(|| {
        format!(
            "Decode detour fixture manifest {}",
            canonical_manifest.display()
        )
    })?;
    if manifest != expected_detour_fixture_manifest() {
        bail!(
            "Detour fixture manifest does not match the pinned issue #24 contract: {}",
            canonical_manifest.display()
        );
    }

    let fixture_dir = canonical_manifest
        .parent()
        .context("Detour fixture manifest has no parent directory")?;
    for asset in &manifest.assets {
        let asset_path = fixture_dir.join(&asset.path);
        let metadata = std::fs::symlink_metadata(&asset_path).with_context(|| {
            format!(
                "Read detour fixture asset metadata {}",
                asset_path.display()
            )
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            bail!(
                "Detour fixture asset must be a regular file and not a symlink: {}",
                asset_path.display()
            );
        }
        let canonical_asset = asset_path.canonicalize().with_context(|| {
            format!("Canonicalize detour fixture asset {}", asset_path.display())
        })?;
        if canonical_asset != asset_path {
            bail!(
                "Detour fixture asset path must not traverse symlinks: {}",
                asset_path.display()
            );
        }
        if metadata.len() != asset.bytes {
            bail!(
                "Detour fixture asset {} has {} bytes, expected {}",
                asset.path,
                metadata.len(),
                asset.bytes
            );
        }
        let asset_bytes = std::fs::read(&canonical_asset)
            .with_context(|| format!("Read detour fixture asset {}", canonical_asset.display()))?;
        let digest = format!("{:x}", Sha256::digest(&asset_bytes));
        if digest != asset.sha256 {
            bail!(
                "Detour fixture asset {} SHA-256 mismatch: got {}, expected {}",
                asset.path,
                digest,
                asset.sha256
            );
        }
    }
    Ok(())
}
pub(crate) fn is_truthy(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "y" | "on"
    )
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoginKnownSpellsLikeCpp {
    pub(crate) initial_login: bool,
    pub(crate) known_spells: Vec<u32>,
    pub(crate) favorite_spells: Vec<u32>,
}
pub(crate) fn validate_provisioning_mode(
    guarded_mode: bool,
    ensure_test_accounts: bool,
) -> Result<()> {
    if guarded_mode && ensure_test_accounts {
        bail!(
            "guarded multi-client workflows forbid --ensure-test-accounts/WOW_BOT_ENSURE_TEST_ACCOUNTS; provision fixtures separately, then run the read-only identity preflight"
        );
    }
    Ok(())
}
pub(crate) fn validate_rested_xp_cli_values(
    enabled: bool,
    acknowledged_disposable: bool,
    bot_count: usize,
    creature_entry: u32,
    offline_secs: u64,
    timeout_secs: u64,
    now_secs: u64,
) -> Result<()> {
    if !enabled {
        if acknowledged_disposable {
            bail!("{ACK_DISPOSABLE_RESTED_XP_FLAG} is only valid with --rested-xp-smoke");
        }
        return Ok(());
    }
    if !acknowledged_disposable {
        bail!(
            "--rested-xp-smoke requires {ACK_DISPOSABLE_RESTED_XP_FLAG}; this acknowledges that the selected account, character, and Battle.net identity are disposable QA fixtures"
        );
    }
    if bot_count != 1 {
        bail!("--rested-xp-smoke requires exactly one bot; select it with --single");
    }
    if creature_entry == 0 {
        bail!("--rested-xp-creature-entry must be nonzero");
    }
    if offline_secs == 0 {
        bail!("--rested-xp-offline-secs must be greater than zero");
    }
    if u32::try_from(offline_secs).is_err() {
        bail!("--rested-xp-offline-secs must fit the legacy C++ uint32 interval");
    }
    if offline_secs >= now_secs {
        bail!(
            "--rested-xp-offline-secs ({offline_secs}) must be smaller than the current Unix timestamp ({now_secs})"
        );
    }
    if timeout_secs == 0 {
        bail!("--rested-xp-timeout must be greater than zero");
    }
    Ok(())
}
pub(crate) fn validate_rested_xp_fixture_safety_state(
    state: &RestedXpFixtureSafetyState,
) -> Result<()> {
    if state.at_login != 0 {
        bail!(
            "rested-XP fixture requires characters.at_login = 0, found 0x{:X}; login flags can reset or create non-restored character state",
            state.at_login
        );
    }
    if state.game_account_online != 0 {
        bail!(
            "rested-XP fixture game account is marked online; refusing concurrent or stale account state"
        );
    }
    if !state.bnet_email_matches_configured_account {
        bail!(
            "rested-XP fixture account ID does not belong to the configured @bot.local Battle.net email"
        );
    }
    if state.characters_on_game_account != 1 {
        bail!(
            "rested-XP fixture requires a dedicated game account with exactly one character, found {}",
            state.characters_on_game_account
        );
    }
    if state.game_accounts_on_bnet_account != 1 {
        bail!(
            "rested-XP fixture requires a dedicated Battle.net identity with exactly one game account, found {}",
            state.game_accounts_on_bnet_account
        );
    }
    if !state.nonempty_side_state.is_empty() {
        let summary = state
            .nonempty_side_state
            .iter()
            .map(|(table, rows)| format!("{table}={rows}"))
            .collect::<Vec<_>>()
            .join(", ");
        bail!(
            "rested-XP fixture has non-restored high-risk side state ({summary}); use a clean disposable QA character/account"
        );
    }
    Ok(())
}
pub(crate) fn print_help() {
    println!("wow-test-bot options:");
    println!("  --config <path>          Config JSON path (default: config.json)");
    println!("  --dungeon <id>           LFG dungeon id override");
    println!("  --timeout <secs>         LFG read window override");
    println!("  --single <account>       Run one configured account only");
    println!("  --parallel               Run enabled bots concurrently (default)");
    println!("  --sequential             Run enabled bots one after another");
    println!("  --auto-teleport          Send CMSG_DF_TELEPORT after group formation");
    println!(
        "  --cleanup-groups         Delete stale group rows for configured bot GUIDs before run"
    );
    println!("  --require-group          Treat missing party info/group formation as failure");
    println!("  --ensure-test-accounts   Create missing local TESTBOT auth rows; validate existing rows without rewriting them");
    println!("  --login-only             Stop after SMSG_LOGIN_VERIFY_WORLD; do not run LFG");
    println!("  --stand-state-smoke      After login, verify Sit then Stand state round-trips");
    println!(
        "  --stand-state <n>        Verify one state instead (0=Stand, 1=Sit, 3=Sleep, 8=Kneel)"
    );
    println!("  --stand-state-timeout <secs>  Per-state response timeout (default: 5)");
    println!(
        "                           Env: WOW_BOT_STAND_STATE_SMOKE, WOW_BOT_STAND_STATE, WOW_BOT_STAND_STATE_TIMEOUT_SECS"
    );
    println!(
        "  --bank-smoke             Deposit, logout/relogin, withdraw, logout, and verify DB persistence"
    );
    println!("  --bank-item-entry <id>   Controlled fixture item (default: 2589)");
    println!("  --bank-runtime-counter <n> Live ObjectGuid low counter for the banker");
    println!("  --bank-timeout <secs>    Per bank phase timeout (default: 8)");
    println!(
        "                           Env: WOW_BOT_BANK_SMOKE, WOW_BOT_BANK_ITEM_ENTRY, WOW_BOT_BANK_RUNTIME_COUNTER, WOW_BOT_BANK_TIMEOUT_SECS"
    );
    println!(
        "  --void-storage-smoke     Unlock, deposit, relog/swap, relog/withdraw, and verify CharacterDB"
    );
    println!(
        "  --void-storage-query-capture  Query one seeded void item for a narrow C++/Rust wire capture"
    );
    println!("  --void-storage-item-entry <id> Controlled fixture item (default: 2589)");
    println!(
        "  --void-storage-runtime-counter <n> Optional checked live ObjectGuid counter override"
    );
    println!("  --void-storage-timeout <secs> Per action/DB timeout (default: 8)");
    println!(
        "                           Env: WOW_BOT_VOID_STORAGE_SMOKE, WOW_BOT_VOID_STORAGE_QUERY_CAPTURE, WOW_BOT_VOID_STORAGE_ITEM_ENTRY, WOW_BOT_VOID_STORAGE_RUNTIME_COUNTER, WOW_BOT_VOID_STORAGE_RUNTIME_REALM_ID, WOW_BOT_VOID_STORAGE_TIMEOUT_SECS"
    );
    println!(
        "  --vendor-smoke           Buy one extended-cost vendor item, relog, verify DB persistence, and restore the fixture"
    );
    println!("  --vendor-entry <id>      Vendor creature entry (default: 18525 G'eras)");
    println!("  --vendor-spawn-guid <n>  Exact world.creature spawn (default: 96654)");
    println!(
        "  --vendor-runtime-counter <n> Optional checked live ObjectGuid low-counter override"
    );
    println!("  --vendor-item-entry <id> Item to buy (default: 30183)");
    println!("  --vendor-extended-cost <id> Required vendor extended cost (default: 1642)");
    println!("  --vendor-currency-id <id> Cost currency (default: 42)");
    println!("  --vendor-currency-cost <n> Cost for one purchase (default: 15)");
    println!("  --vendor-currency-quantity <n> Seeded quantity (default: 30)");
    println!("  --vendor-timeout <secs>  Vendor response timeout (default: 8)");
    println!(
        "                           Env: WOW_BOT_VENDOR_SMOKE, WOW_BOT_VENDOR_ENTRY, WOW_BOT_VENDOR_SPAWN_GUID, WOW_BOT_VENDOR_RUNTIME_COUNTER, WOW_BOT_VENDOR_ITEM_ENTRY, WOW_BOT_VENDOR_EXTENDED_COST, WOW_BOT_VENDOR_CURRENCY_ID, WOW_BOT_VENDOR_CURRENCY_COST, WOW_BOT_VENDOR_CURRENCY_QUANTITY, WOW_BOT_VENDOR_TIMEOUT_SECS"
    );
    println!(
        "  --equipment-set-race-smoke  Concurrently save equipment/transmog sets, logout, relog, and verify shared GUID persistence"
    );
    println!(
        "  --equipment-set-account-a <account>  Equipment-set bot (default TESTBOT2@bot.local)"
    );
    println!(
        "  --equipment-set-account-b <account>  Transmog-set bot (default TESTBOT3@bot.local)"
    );
    println!("  --equipment-set-timeout <secs>  Per response/barrier timeout (default: 10)");
    println!(
        "                           Env: WOW_BOT_EQUIPMENT_SET_RACE_SMOKE, WOW_BOT_EQUIPMENT_SET_ACCOUNT_A, WOW_BOT_EQUIPMENT_SET_ACCOUNT_B, WOW_BOT_EQUIPMENT_SET_TIMEOUT_SECS"
    );
    println!(
        "  --homebind-smoke         Bind at an innkeeper, relog, and verify response packets plus DB persistence"
    );
    println!(
        "  --homebind-runtime-counter <n> Optional ObjectGuid low-counter override for the innkeeper"
    );
    println!("  --homebind-timeout <secs> Bind response timeout (default: 8)");
    println!(
        "                           Env: WOW_BOT_HOMEBIND_SMOKE, WOW_BOT_HOMEBIND_RUNTIME_COUNTER, WOW_BOT_HOMEBIND_TIMEOUT_SECS"
    );
    println!(
        "  --rested-xp-smoke       Compare offline rest rates, kill one mob, and verify XP/rest persistence"
    );
    println!(
        "  {ACK_DISPOSABLE_RESTED_XP_FLAG} Acknowledge the rested-XP account/character/BNet fixture is disposable"
    );
    println!("  --rested-xp-creature-entry <id>  Low-level XP target (default: 15274 Mana Wyrm)");
    println!("  --rested-xp-creature-guid <guid> Optional exact world.creature spawn GUID");
    println!(
        "  --rested-xp-runtime-counter <n> Optional live counter; must match the target's discovered CREATE_OBJECT"
    );
    println!("  --rested-xp-offline-secs <n> Simulated offline interval (default: 86400)");
    println!("  --rested-xp-timeout <secs> Combat/DB response timeout (default: 120)");
    println!(
        "                           Env: WOW_BOT_RESTED_XP_SMOKE, WOW_BOT_RESTED_XP_CREATURE_ENTRY, WOW_BOT_RESTED_XP_CREATURE_GUID, WOW_BOT_RESTED_XP_RUNTIME_COUNTER, WOW_BOT_RESTED_XP_OFFLINE_SECS, WOW_BOT_RESTED_XP_TIMEOUT_SECS"
    );
    println!(
        "  --detour-chase-capture  Run the pinned issue #24 heartbeat→chase-spline→ping capture window"
    );
    println!(
        "  {ACK_DISPOSABLE_DETOUR_FIXTURE_FLAG}  Acknowledge the map/creature/character fixture is disposable"
    );
    println!(
        "  --detour-fixture-manifest <path>  Exact generated fixture.json beside the pinned mmaps"
    );
    println!("  --detour-chase-timeout <secs>  Capture deadline (default: 30)");
    println!(
        "                           Env: WOW_BOT_DETOUR_CHASE_CAPTURE, WOW_BOT_DETOUR_FIXTURE_MANIFEST, WOW_BOT_DETOUR_CHASE_TIMEOUT_SECS"
    );
    println!(
        "  --creature-spell-capture  Body-pull the guarded issue #26 Cabal fixture and require one adjacent hit START/GO"
    );
    println!(
        "  --creature-spell-fixture-manifest <path>  Exact committed creature spell fixture.json"
    );
    println!("  --creature-spell-timeout <secs>  Cast observation deadline (default: 30)");
    println!(
        "                           Env: WOW_BOT_CREATURE_SPELL_CAPTURE, WOW_BOT_CREATURE_SPELL_FIXTURE_MANIFEST, WOW_BOT_CREATURE_SPELL_TIMEOUT_SECS"
    );
    println!(
        "  --cast-lifecycle          Run a JSON Cast/Cancel/Wait player spell script and ordered observer"
    );
    println!("  --cast-lifecycle-plan <path>  3.4.3 cast plan (or WOW_BOT_CAST_LIFECYCLE_PLAN)");
    println!(
        "  --loot-race-smoke       Race ITEM and MONEY claims on one shared chest from two real sessions"
    );
    println!(
        "  --loot-item-capture     One real session kills/opens/claims only the item and emits a fixed capture fence"
    );
    println!(
        "  {}  REQUIRED acknowledgement: mutates disposable loot fixtures (capture also kills its creature)",
        loot_race::ACK_FLAG
    );
    println!(
        "  --loot-race-account-a <account>  First disposable contender/capture killer (default TESTBOT2@bot.local)"
    );
    println!(
        "  --loot-race-account-b <account>  Second disposable bot (default TESTBOT3@bot.local)"
    );
    println!("  --loot-race-gameobject-entry <id> Exact chest entry (default 2846 Tattered Chest)");
    println!(
        "  --loot-race-gameobject-spawn-guid <id> Exact wrapper-owned world.gameobject spawn (default 9106001)"
    );
    println!("                           Legacy aliases: --loot-race-creature-entry / --loot-race-creature-spawn-guid");
    println!("  --loot-race-runtime-counter <n>  Optional strict live counter override (default 0=auto-discover full GUID)");
    println!("  --loot-race-item-entry <id>      Exact shared-chest loot item (default 38)");
    println!("  --loot-race-timeout <secs>       Per coordination/loot timeout (capture also uses it for combat; default 30)");
    println!("  --loot-workflow-deadline <secs>  Hard end-to-end loot deadline before guarded cleanup (default 900)");
    println!("  --recover-loot-fixture           Restore one pending WOW_BOT_FIXTURE_JOURNAL; no login/service actions");
    println!(
        "                           Race uses a guarded temporary chest; capture keeps its separate Doctor creature fixture"
    );
    println!(
        "                           Env capture mode: WOW_BOT_LOOT_ITEM_CAPTURE=1 (uses account A; account B remains offline as a guarded fixture snapshot)"
    );
    println!(
        "  --group-capacity-race-smoke  Race two invite accepts for the fifth slot of a preloaded four-member party"
    );
    println!("  --group-capacity-leader <account>     Preloaded party leader (default TESTBOT1)");
    println!("  --group-capacity-candidate-a <account> First invitee (default TESTBOT2)");
    println!("  --group-capacity-candidate-b <account> Second invitee (default TESTBOT3)");
    println!("  --group-capacity-group-id <id>         Required preloaded CharacterDB group id");
    println!("  --group-capacity-timeout <secs>        Per barrier/packet timeout (default 30)");
    println!("  --quest-smoke            After login, right-click/query one questgiver NPC");
    println!("  --quest-creature-entry <id>  Creature entry to resolve from world.creature");
    println!("  --quest-creature-guid <guid> Optional world.creature spawn guid override");
    println!("  --quest-runtime-counter <n> Live ObjectGuid low counter for the target");
    println!("  --quest-guid-counter <n> Legacy alias for --quest-runtime-counter");
    println!("  --quest-map <id>         Optional map id override for GUID construction");
    println!("  --expect-quest <id>      Require this quest id in gossip/list/details");
    println!("  --forbid-quest <id>      Fail if this quest id is offered");
    println!("  --forbid-title <text>    Fail if an offered quest title contains this text");
    println!("  --no-quest-query-details Skip the QuestGiverQueryQuest details probe");
    println!("  --quest-accept           Accept the selected quest and verify DB persistence");
    println!(
        "  --quest-reset            Delete the selected quest from bot quest tables before run"
    );
    println!(
        "  --quest-relocate         Move the bot character near the target spawn before login"
    );
    println!(
        "  --quest-set-level <n>    Set bot character level before login for deterministic QA"
    );
    println!("  --quest-set-race <n>     Set bot character race before login");
    println!("  --quest-set-class <n>    Set bot character class before login");
    println!(
        "  --quest-objective-persist Seed expected quest objectives, logout, and verify DB rows"
    );
    println!("  --quest-objectives <rows> Objective rows as storage:data,storage:data");
    println!("  --quest-objective-status <n> Status for seeded quest row (default: 3)");
    println!("  --gossip-select-option-id <id> Select this gossip option after GossipMessage");
    println!("  --expect-trainer-list     Require SMSG_TRAINER_LIST after gossip select");
    println!("  --expect-trainer-id <id>  Require this TrainerID in SMSG_TRAINER_LIST");
    println!("  --quest-timeout <secs>   Quest smoke read window after sending hello");
    println!("  --report <path>          Write JSON report");
}
pub(crate) fn apply_password_overrides(bots: &mut [config::BotConfig]) {
    let shared_password = std::env::var("WOW_BOT_PASSWORD").ok();
    for bot in bots {
        if let Ok(password) = std::env::var(password_env_name(&bot.account)) {
            bot.password = password;
        } else if bot.password.is_empty() {
            if let Some(password) = &shared_password {
                bot.password = password.clone();
            }
        }
    }
}
pub(crate) fn password_env_name(account: &str) -> String {
    let suffix: String = account
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    format!("WOW_BOT_PASSWORD_{suffix}")
}
pub(crate) fn ensure_test_accounts(bots: &[config::BotConfig]) -> Result<()> {
    use mysql::prelude::Queryable;

    let auth_db = auth_db_url()?;
    let char_db = characters_db_url()?;
    let auth_opts = qa_mysql_opts(&auth_db, "auth")?;
    let char_opts = qa_mysql_opts(&char_db, "characters")?;
    let mut auth_conn =
        mysql::Conn::new(auth_opts).map_err(|e| anyhow!("Connect to auth DB failed: {e}"))?;
    let mut char_conn =
        mysql::Conn::new(char_opts).map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;

    // Validate every existing character before the first auth write. Account
    // provisioning is intentionally create-only: an ID collision must never
    // become authority to rewrite credentials or character ownership.
    for bot in bots {
        validate_local_bot_character_owner(&mut char_conn, bot)?;
    }
    for bot in bots {
        let character_count: u64 = char_conn
            .exec_first(
                "SELECT COUNT(*) FROM characters WHERE account = ?",
                (bot.account_id,),
            )
            .map_err(|e| anyhow!("Count characters for account {}: {e}", bot.account_id))?
            .unwrap_or(0);
        provision_local_bot_account_create_only(&mut auth_conn, bot, character_count)?;
    }

    Ok(())
}
