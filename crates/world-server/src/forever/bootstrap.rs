//! Target-specific mandatory resources, never the legacy startup catalogs.
use super::character_capture::CharacterCapture;
use super::navigation::NavigationRuntime;
use anyhow::{Context, Result, bail, ensure};
use std::{env, fs, path::Path, sync::Arc};
use wow_data::forever_birth::item_records::{
    ITEM_EFFECT_HASH, ITEM_HASH, ITEM_RELATION_HASH, ITEM_SPARSE_HASH, ItemRecords,
};
use wow_data::forever_birth::item_specs::{
    GEM_PROPERTIES_HASH, ITEM_SPEC_HASH, ITEM_SPEC_OVERRIDE_HASH, ItemSpecRecords,
};
use wow_data::forever_birth::{
    ABILITY_HASH, AbilityBaseline, BirthRecords, LOADOUT_HASH, LOADOUT_ITEM_HASH, RACE_CLASS_HASH,
    SKILL_LINE_HASH,
};
use wow_data::forever_character_ids::ForeverAchievementIds;
use wow_data::forever_hotfix::{
    ForeverHotfixCatalog, ForeverTactKeys, ITEM_TABLE_HASHES, TACT_KEY_TABLE_HASH,
};
use wow_data::forever_initialization::{
    CLASS_POWER_HASH, InitializationRecords, MAP_HASH, MOVIE_HASH, MapBaseline, POWER_HASH,
    SPECIALIZATION_HASH,
};
use wow_data::forever_spells::{SPELL_TABLE_HASHES, SpellBaseline, SpellCatalog, SpellRecords};
use wow_data::{HotfixBlobCache, hotfix_cache::HotfixRecordStatus};
use wow_database::{
    CharacterDatabase, HotfixDatabase, LoginDatabase,
    MariaDbHotfixDeliveryMetadataPersistenceAdapterLikeCpp, WorldDatabase,
    build_connection_string_with_ssl_like_cpp,
    forever::{
        ForeverAvailabilityRepository, ForeverCreationWorldRepository, ForeverSessionRepository,
        ForeverSpellWorldRepository,
    },
    forever_hotfix::ForeverHotfixRepository,
};
use wow_persistence::forever::creation::CreationWorldRepository;
use wow_persistence::forever::{AvailabilityRepository, SessionRepository};
use wow_persistence::{
    HotfixDeliveryMetadataLoadOutcomeLikeCpp, HotfixDeliveryMetadataPersistencePortLikeCpp,
};
use wow_world::forever::creation::{CharacterTemplates, NumericItemTemplates, StartingPolicy};
use wow_world::forever::name_rules::{NamePolicy, NameRules};
use wow_world::forever::permissions::DefaultAccountPermissions;
use wow_world::forever::spells::{SpellDefinitionSeeds, SpellLoadPlan};
use wow_world::forever::{CharacterCatalog, InitializationPolicy};

#[cfg(test)]
#[path = "../../unit_tests/forever/bootstrap.rs"]
mod tests;

fn navigation_arguments<'a>(
    args: &'a [std::ffi::OsString],
    offset: &mut usize,
) -> Result<Option<[&'a std::ffi::OsStr; 3]>> {
    let remaining = args.get(*offset..).context("invalid argument offset")?;
    if !remaining.first().is_some_and(|arg| {
        arg == "--ack-available-navigation56" || arg == "--ack-available-navigation57"
    }) {
        return Ok(None);
    }
    let [_, package, plan, map, ..] = remaining else {
        bail!("navigation package, plan and map are required");
    };
    *offset += 4;
    Ok(Some([
        package.as_os_str(),
        plan.as_os_str(),
        map.as_os_str(),
    ]))
}

pub(super) struct Runtime {
    pub build: u32,
    pub dbc_locale: u8,
    pub auth: Arc<LoginDatabase>,
    pub session_repository: Arc<dyn SessionRepository>,
    pub catalog: CharacterCatalog,
    pub hotfixes: Arc<ForeverHotfixCatalog>,
    pub build_key: [u8; 16],
    pub policy: InitializationPolicy,
    pub region_group: i32,
    pub character_idle_timeout: std::time::Duration,
    pub character_capture: Option<CharacterCapture>,
    pub appearance: Arc<wow_data::forever_appearance::AppearanceCatalog>,
    pub creation_sources: wow_world::forever::creation::WorldSources,
    pub initialization: Arc<wow_data::forever_initialization::InitializationCatalog>,
    pub birth: Arc<wow_data::forever_birth::BirthCatalog>,
    pub spells: Arc<SpellCatalog>,
    pub spell_definitions: Arc<SpellDefinitionSeeds>,
    pub items: Arc<NumericItemTemplates>,
    pub game_tables: wow_data::forever_game_tables::InitialGameTables,
    pub name_rules: Arc<NameRules>,
    pub name_policy: NamePolicy,
    pub permissions: Arc<DefaultAccountPermissions>,
    pub starting_policy: StartingPolicy,
    pub super_district: i32,
    pub navigation: Option<NavigationRuntime>,
}

pub(super) async fn load() -> Result<Runtime> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() < 4 || args[0] != "--ack-isolated-forever" {
        bail!(
            "usage: --ack-isolated-forever <bnet-config> <private-build-key-file> <private-target-db2-directory> [--ack-available-initial-map] [--ack-available-birth-abilities] --ack-available-item-tables [--ack-available-spell-info-tables] [--build 70170|70450] [--locale enUS|esES] [--ack-available-navigation56|--ack-available-navigation57 <package-root> <plan> <map-id>] [--ack-private-character-create-capture|--ack-private-name-availability-capture <new-private-file>]"
        );
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/forever-login")
        .canonicalize()?;
    for path in &args[1..4] {
        ensure!(
            Path::new(path).canonicalize()?.starts_with(&root),
            "outside isolated runtime"
        );
    }
    let mut optional = 4;
    let map_baseline = if args
        .get(optional)
        .is_some_and(|arg| arg == "--ack-available-initial-map")
    {
        optional += 1;
        MapBaseline::AvailablePrefix
    } else {
        MapBaseline::Complete
    };
    let ability_baseline = if args
        .get(optional)
        .is_some_and(|arg| arg == "--ack-available-birth-abilities")
    {
        optional += 1;
        AbilityBaseline::AvailablePrefix
    } else {
        AbilityBaseline::Complete
    };
    ensure!(
        args.get(optional)
            .is_some_and(|arg| arg == "--ack-available-item-tables"),
        "target numeric items currently require explicit frozen-prefix acknowledgement"
    );
    optional += 1;
    let spell_baseline = if args
        .get(optional)
        .is_some_and(|arg| arg == "--ack-available-spell-info-tables")
    {
        optional += 1;
        SpellBaseline::AvailablePrefixes
    } else {
        SpellBaseline::Complete
    };
    let build = if args.get(optional).is_some_and(|arg| arg == "--build") {
        let build = args
            .get(optional + 1)
            .and_then(|value| value.to_str())
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|build| matches!(build, 70170 | 70450))
            .context("unsupported target build")?;
        optional += 2;
        build
    } else {
        70170
    };
    let dbc_locale = if args.get(optional).is_some_and(|arg| arg == "--locale") {
        let locale = args
            .get(optional + 1)
            .and_then(|value| value.to_str())
            .context("missing target locale")?;
        ensure!(
            matches!(locale, "enUS" | "esES"),
            "unsupported target locale"
        );
        optional += 2;
        if locale == "enUS" { 0 } else { 6 }
    } else {
        6
    };
    let locale = if dbc_locale == 0 { "enUS" } else { "esES" };
    let navigation = navigation_arguments(&args, &mut optional)?
        .map(|[package, plan, map]| super::navigation::load(package, plan, map))
        .transpose()?;
    let character_capture = if args.len() != optional {
        ensure!(
            args.len() == optional + 2,
            "invalid target opt-in arguments"
        );
        Some(
            if args[optional] == "--ack-private-character-create-capture" {
                CharacterCapture::prepare_for_build(&root, Path::new(&args[optional + 1]), build)?
            } else if args[optional] == "--ack-private-name-availability-capture" {
                CharacterCapture::prepare_name_for_build(
                    &root,
                    Path::new(&args[optional + 1]),
                    build,
                )?
            } else {
                bail!("unknown opt-in");
            },
        )
    } else {
        None
    };
    let key_path = Path::new(&args[2]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        ensure!(
            fs::metadata(key_path)?.permissions().mode() & 0o077 == 0,
            "key is not private"
        );
    }
    let build_key = fs::read(key_path)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("build key length"))?;
    wow_config::load_config(args[1].to_str().context("config path")?)?;
    let info = wow_config::parse_database_info(
        "LoginDatabaseInfo",
        &wow_config::get_string_default("LoginDatabaseInfo", ""),
    )?;
    ensure!(
        info.host == "127.0.0.1"
            && info.port_or_socket == "13316"
            && info.database == "auth_forever_70170"
            && wow_config::get_string_default("BindIP", "") == "127.0.0.1",
        "not isolated databases"
    );
    let url = |database| {
        build_connection_string_with_ssl_like_cpp(
            &info.host,
            &info.port_or_socket,
            &info.username,
            &info.password,
            database,
            info.ssl,
        )
    };
    let auth = Arc::new(LoginDatabase::open_with_pool_size(&url(&info.database), 1).await?);
    let characters = Arc::new(
        CharacterDatabase::open_with_pool_size(&url("characters_forever_70170"), 1).await?,
    );
    let session_repository = Arc::new(ForeverSessionRepository::new(auth.clone(), characters));
    let reserved_names = session_repository
        .load_reserved_names()
        .await
        .map_err(|_| anyhow::anyhow!("reserved name query failed"))?;
    let permissions = Arc::new(DefaultAccountPermissions::load(
        session_repository
            .load_default_permissions(1)
            .await
            .map_err(|_| anyhow::anyhow!("default permissions query failed"))?,
    ));
    let world =
        Arc::new(WorldDatabase::open_with_pool_size(&url("world_forever_70170_02245"), 1).await?);
    let hotfix =
        Arc::new(HotfixDatabase::open_with_pool_size(&url("hotfixes_forever_70170"), 1).await?);
    eprintln!("bootstrap data phase: achievements");
    let achievements = ForeverAchievementIds::load(Path::new(&args[3]))?;
    eprintln!("bootstrap data phase: initialization");
    let initialization_rows = InitializationRecords::load(Path::new(&args[3]), map_baseline)?;
    eprintln!("bootstrap data phase: birth");
    let birth_rows = BirthRecords::load(Path::new(&args[3]), ability_baseline)?;
    eprintln!("bootstrap data phase: items");
    let item_rows = ItemRecords::load_available(Path::new(&args[3]))?;
    eprintln!("bootstrap data phase: item specs");
    let item_spec_rows = ItemSpecRecords::load(Path::new(&args[3]))?;
    eprintln!("bootstrap data phase: spells");
    let spell_rows = SpellRecords::load(Path::new(&args[3]), spell_baseline)?;
    eprintln!("bootstrap data phase: game tables");
    let game_tables = wow_data::forever_game_tables::InitialGameTables::load(Path::new(&args[3]))?;
    eprintln!("bootstrap data phase: spell value game tables");
    let spell_value_tables = Arc::new(wow_data::forever_game_tables::SpellValueGameTables::load(
        Path::new(&args[3]),
    )?);
    eprintln!("bootstrap data phase: hotfix overlays");
    let initialization_overlays = ForeverHotfixRepository::new(hotfix.clone())
        .load_initialization_overlays()
        .await
        .map_err(|_| anyhow::anyhow!("initialization overlay query failed"))?;
    eprintln!("bootstrap data phase: birth overlays");
    let birth_overlays = ForeverHotfixRepository::new(hotfix.clone())
        .load_birth_overlays()
        .await
        .map_err(|_| anyhow::anyhow!("birth overlay query failed"))?;
    eprintln!("bootstrap data phase: item overlays");
    let item_overlays = ForeverHotfixRepository::new(hotfix.clone())
        .load_item_overlays()
        .await
        .map_err(|_| anyhow::anyhow!("item overlay query failed"))?;
    eprintln!("bootstrap data phase: item spec overlays");
    let item_spec_overlays = ForeverHotfixRepository::new(hotfix.clone())
        .load_item_spec_overlays()
        .await
        .map_err(|_| anyhow::anyhow!("item specialization overlay query failed"))?;
    eprintln!("bootstrap data phase: item locale overlays");
    let sparse_locale_overlays = ForeverHotfixRepository::new(hotfix.clone())
        .load_sparse_es_es_overlays()
        .await
        .map_err(|_| anyhow::anyhow!("item locale overlay query failed"))?;
    eprintln!("bootstrap data phase: spell overlays");
    let spell_overlays = ForeverHotfixRepository::new(hotfix.clone())
        .load_spell_overlays()
        .await
        .map_err(|_| anyhow::anyhow!("spell overlay query failed"))?;
    eprintln!("bootstrap data phase: spell locale overlays");
    let spell_locale_overlays = ForeverHotfixRepository::new(hotfix.clone())
        .load_spell_es_es_overlays()
        .await
        .map_err(|_| anyhow::anyhow!("spell locale overlay query failed"))?;
    let spell_repository = ForeverSpellWorldRepository::new(world.clone());
    eprintln!("bootstrap data phase: server spells");
    let server_spell_rows = spell_repository
        .load_server_spells()
        .await
        .map_err(|_| anyhow::anyhow!("server spell source query failed"))?;
    eprintln!("bootstrap data phase: spell custom attributes");
    let spell_custom_attributes = spell_repository
        .load_spell_custom_attributes()
        .await
        .map_err(|_| anyhow::anyhow!("spell SQL custom attribute source query failed"))?;
    eprintln!("bootstrap data phase: creature immunities");
    let creature_immunities = spell_repository
        .load_creature_immunities()
        .await
        .map_err(|_| anyhow::anyhow!("creature immunity source query failed"))?;
    eprintln!("bootstrap data phase: spell required");
    let spell_required = spell_repository
        .load_spell_required()
        .await
        .map_err(|_| anyhow::anyhow!("spell required source query failed"))?;
    eprintln!("bootstrap data phase: spell learning");
    let spell_learn = spell_repository
        .load_spell_learn_spells()
        .await
        .map_err(|_| anyhow::anyhow!("spell learn relationship source query failed"))?;
    let creation_repository = ForeverCreationWorldRepository::new(world.clone());
    eprintln!("bootstrap data phase: item addons");
    let item_addons = creation_repository
        .load_item_addons()
        .await
        .map_err(|_| anyhow::anyhow!("target item addon source query failed"))?;
    eprintln!("bootstrap data phase: creation world");
    let creation_rows = creation_repository
        .load_creation_world()
        .await
        .map_err(|_| anyhow::anyhow!("target creation world source query failed"))?;
    eprintln!("bootstrap data phase: character templates");
    let template_rows = creation_repository
        .load_character_templates()
        .await
        .map_err(|_| anyhow::anyhow!("target character template query failed"))?;
    // Pinned C++ World.cpp:752 / SharedDefines.h:107-137 default is 90,
    // range 1..123. Retained as reference configuration, NOT proof of the
    // Forever client's playable level cap; Create remains disabled.
    // 02245dcd World.cpp numeric defaults/limits, NOT native Forever playable
    // cap proof. No DB2 StartingLevel or legacy DK=55 policy is substituted.
    let (max_level, starting_config) = super::start_config::load();
    eprintln!("bootstrap data phase: availability");
    let availability_rows = ForeverAvailabilityRepository::new(world)
        .load_availability()
        .await
        .map_err(|_| anyhow::anyhow!("availability query failed"))?;
    eprintln!("bootstrap data phase: TACT keys");
    let tact_keys = ForeverTactKeys::load(Path::new(&args[3]))?;
    eprintln!("bootstrap data phase: TACT key overlays");
    let overlays = ForeverHotfixRepository::new(hotfix.clone())
        .load_tact_key_overlays()
        .await
        .map_err(|_| anyhow::anyhow!("TactKey overlay query failed"))?;
    let tact_keys = tact_keys.with_overlays(
        overlays.official.into_iter().map(|row| (row.id, row.key)),
        overlays.custom.into_iter().map(|row| (row.id, row.key)),
    )?;
    eprintln!("bootstrap data phase: appearance");
    let appearance_rows =
        wow_data::forever_appearance::AppearanceRecords::load(Path::new(&args[3]))?;
    eprintln!("bootstrap data phase: appearance overlays");
    let appearance_overlays = ForeverHotfixRepository::new(hotfix.clone())
        .load_appearance_overlays()
        .await
        .map_err(|_| anyhow::anyhow!("appearance overlay query failed"))?;
    eprintln!("bootstrap data phase: names");
    let name_rows = wow_data::forever_names::NameRecords::load(Path::new(&args[3]))?;
    eprintln!("bootstrap data phase: name overlays");
    let name_overlays = ForeverHotfixRepository::new(hotfix.clone())
        .load_name_overlays()
        .await
        .map_err(|_| anyhow::anyhow!("name overlay query failed"))?;
    eprintln!("bootstrap data phase: hotfix metadata");
    let hotfix_adapter = MariaDbHotfixDeliveryMetadataPersistenceAdapterLikeCpp::new(hotfix);
    let mut hotfixes = HotfixBlobCache::new();
    hotfixes.register_typed_table(TACT_KEY_TABLE_HASH);
    // Register effective stores as known. Tact, seven item stores and all 42
    // spell stores have full serializers; the Valid-status guard retains the
    // other stores' explicit unported boundary instead of claiming absence.
    for hash in [
        0x49349C6E,
        0x2FB7905B,
        0x681D0F3D,
        0x61431A65,
        0xA7E150FE,
        0x9B1BEE48,
        0xDA82D96C,
        0x25C1CB13,
        0x3ACAE305,
        0xC7ED797D,
        MAP_HASH,
        POWER_HASH,
        SPECIALIZATION_HASH,
        CLASS_POWER_HASH,
        MOVIE_HASH,
        SKILL_LINE_HASH,
        RACE_CLASS_HASH,
        ABILITY_HASH,
        LOADOUT_HASH,
        LOADOUT_ITEM_HASH,
        ITEM_HASH,
        ITEM_SPARSE_HASH,
        ITEM_EFFECT_HASH,
        ITEM_RELATION_HASH,
        ITEM_SPEC_HASH,
        ITEM_SPEC_OVERRIDE_HASH,
        GEM_PROPERTIES_HASH,
    ] {
        hotfixes.register_typed_table(hash);
    }
    for hash in SPELL_TABLE_HASHES {
        hotfixes.register_typed_table(hash);
    }
    for name in ["ChrClasses.db2", "ChrRaces.db2"] {
        hotfixes.load_db2(Path::new(&args[3]).join(name))?;
    }
    eprintln!("bootstrap data phase: hotfix blobs");
    let blobs = match hotfix_adapter.load_hotfix_blob_rows_like_cpp().await {
        HotfixDeliveryMetadataLoadOutcomeLikeCpp::Loaded(rows) => rows,
        _ => bail!("hotfix blob query failed"),
    };
    hotfixes.apply_hotfix_blob_rows_like_cpp(
        blobs
            .into_iter()
            .map(|row| (row.table_hash, row.record_id, row.locale, row.blob)),
        locale,
    );
    eprintln!("bootstrap data phase: hotfix data rows");
    let rows = match hotfix_adapter.load_hotfix_data_rows_like_cpp().await {
        HotfixDeliveryMetadataLoadOutcomeLikeCpp::Loaded(rows) => rows,
        _ => bail!("hotfix data query failed"),
    };
    for row in &rows {
        ensure!(
            !(row.status == HotfixRecordStatus::Valid as u8
                && hotfixes.has_table(row.table_hash)
                && row.table_hash != TACT_KEY_TABLE_HASH
                && !ITEM_TABLE_HASHES.contains(&row.table_hash)
                && !SPELL_TABLE_HASHES.contains(&row.table_hash)),
            "target typed hotfix serializer required"
        );
        ensure!(row.status <= 4, "invalid hotfix status");
    }
    // The appearance projection consumes every status for its loaded stores,
    // before delivery filters unknown stores without a blob fallback.
    let removals = wow_data::Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp(
        rows.iter()
            .map(|row| (row.table_hash, row.record_id, row.status)),
    );
    eprintln!("bootstrap data phase: finish appearance");
    let appearance = appearance_rows.finish(
        super::appearance::records(appearance_overlays.official),
        super::appearance::records(appearance_overlays.custom),
        &removals,
    )?;
    eprintln!("bootstrap data phase: finish initialization");
    let initialization = initialization_rows.finish(
        super::initialization::records(initialization_overlays.official),
        super::initialization::records(initialization_overlays.custom),
        &removals,
    )?;
    let character_templates = Arc::new(
        CharacterTemplates::load(template_rows, &initialization)
            .map_err(|error| anyhow::anyhow!("target character templates rejected: {error:?}"))?,
    );
    eprintln!("bootstrap data phase: finish birth");
    let birth = birth_rows
        .finish(
            super::birth::records(birth_overlays.official),
            super::birth::records(birth_overlays.custom),
            &removals,
        )
        .map_err(|error| {
            eprintln!("birth composition rejected: {error}");
            error
        })?;
    let birth = Arc::new(birth);
    // Null-caster spell values consume the effective sparse item catalog.
    // Finish this canonical input before the ordered spell custom phase;
    // numeric templates later share it rather than owning a second copy.
    eprintln!("bootstrap data phase: finish items");
    let items = item_rows
        .finish(
            super::items::records(item_overlays.official),
            super::items::records(item_overlays.custom),
            &removals,
        )
        .map_err(|error| {
            eprintln!("item composition rejected: {error}");
            error
        })?
        .with_sparse_locale(
            6,
            sparse_locale_overlays
                .official
                .into_iter()
                .map(|r| (r.id, r.strings)),
            sparse_locale_overlays
                .custom
                .into_iter()
                .map(|r| (r.id, r.strings)),
        );
    let item_data = Arc::new(items);
    eprintln!("bootstrap data phase: finish spells");
    let spell_catalog = Arc::new(
        spell_rows
            .finish(
                super::spells::records(spell_overlays.official)?,
                super::spells::records(spell_overlays.custom)?,
                6,
                super::spells::locale_records(spell_locale_overlays.official),
                super::spells::locale_records(spell_locale_overlays.custom),
                &removals,
            )
            .map_err(|error| {
                eprintln!("spell composition rejected: {error}");
                error
            })?,
    );
    eprintln!("bootstrap data phase: spell load plan");
    let spell_load_plan = SpellLoadPlan::build(spell_catalog).map_err(|error| {
        eprintln!("spell load plan rejected: {error:?}");
        anyhow::anyhow!("target spell input join rejected: {error:?}")
    })?;
    let spell_definitions = super::spell_traversal::correct(
        spell_load_plan
            .with_server_spells(server_spell_rows)
            .map_err(|error| anyhow::anyhow!("target server spell inputs rejected: {error:?}"))?
            .with_id_corrections()
            .map_err(|error| anyhow::anyhow!("target spell ID corrections rejected: {error:?}"))?,
    )?
    .with_skill_line_abilities(birth.clone())
    .map_err(|error| anyhow::anyhow!("target skill-line ability map rejected: {error:?}"))?
    .with_sql_custom_attributes(spell_custom_attributes)
    .map_err(|error| anyhow::anyhow!("target spell SQL custom attributes rejected: {error:?}"))?;
    let spell_definitions = if matches!(spell_baseline, SpellBaseline::AvailablePrefixes) {
        spell_definitions
    } else {
        spell_definitions
            .with_value_game_tables(spell_value_tables)
            .map_err(|error| anyhow::anyhow!("target spell value tables rejected: {error:?}"))?
            .with_custom_attributes(&item_data, &mut super::spell_random::draw)
            .map_err(|error| {
                anyhow::anyhow!("target spell derived custom attributes rejected: {error:?}")
            })?
            .with_diminishing_info(&mut super::spell_random::select)
            .map_err(|error| anyhow::anyhow!("target spell diminishing info rejected: {error:?}"))?
            .with_immunity_info(creature_immunities)
            .map_err(|error| anyhow::anyhow!("target spell immunity info rejected: {error:?}"))?
            .with_target_caps(&item_data)
            .map_err(|error| anyhow::anyhow!("target spell target caps rejected: {error:?}"))?
            .with_spell_ranks()
            .map_err(|error| anyhow::anyhow!("target spell ranks rejected: {error:?}"))?
            .with_spell_required(spell_required)
            .map_err(|error| {
                anyhow::anyhow!("target spell required relationships rejected: {error:?}")
            })?
            .with_learn_skills(&item_data, &mut super::spell_random::draw)
            .map_err(|error| anyhow::anyhow!("target spell learn skills rejected: {error:?}"))?
            .with_specific_and_aura_state()
            .map_err(|error| {
                anyhow::anyhow!("target spell specific/aura state rejected: {error:?}")
            })?
            .with_learn_spells(spell_learn)
            .map_err(|error| {
                anyhow::anyhow!("target spell learn relationships rejected: {error:?}")
            })?
    };
    let spell_definitions = Arc::new(spell_definitions);
    // ID/global corrections, custom, diminishing, immunity, caps and rank/
    // required/skill/specific/aura/learning phases finish before publishing
    // either immutable reader. This is not full executable SpellInfo readiness.
    let spells = spell_definitions.raw_catalog();
    let constructor_effect_slots: usize = spell_definitions
        .records()
        .map(|definition| definition.effect_count())
        .sum();
    println!(
        "Forever effective spell inputs loaded: tables={:?}; joins={:?}; server={:?}; id-corrections={:?}; global-corrections={:?}; skill-abilities={:?}; sql-custom-attributes={:?}; value-gt={:?}; derived-custom-attributes={:?}; diminishing={:?}; immunities={:?}; target-caps={:?}; ranks={:?}; required={:?}; learned-skills={:?}; specific-aura={:?}; learned-spells={:?}; effect-slots={}; remaining executable SpellInfo/Player construction still required.",
        spells.counts(),
        spell_definitions.client_counts(),
        spell_definitions.server_counts(),
        spell_definitions.id_correction_counts(),
        spell_definitions.global_correction_counts(),
        spell_definitions.skill_line_ability_counts(),
        spell_definitions.sql_custom_attribute_counts(),
        spell_definitions
            .value_game_tables()
            .map(|tables| tables.counts()),
        spell_definitions.custom_attribute_counts(),
        spell_definitions.diminishing_counts(),
        spell_definitions.immunity_counts(),
        spell_definitions.target_cap_counts(),
        spell_definitions.spell_rank_counts(),
        spell_definitions.required_spell_counts(),
        spell_definitions.learn_skill_counts(),
        spell_definitions.specific_counts(),
        spell_definitions.learn_spell_counts(),
        constructor_effect_slots
    );
    let item_specs = item_spec_rows.finish(
        super::item_specs::records(item_spec_overlays.official),
        super::item_specs::records(item_spec_overlays.custom),
        &removals,
    )?;
    let item_spec_counts = item_specs.counts();
    let item_specs = Arc::new(item_specs);
    let items =
        NumericItemTemplates::load(item_data.clone(), &item_specs, &initialization, item_addons)
            .map_err(|error| {
                anyhow::anyhow!("target numeric item templates rejected: {error:?}")
            })?;
    println!(
        "Forever effective numeric item sources loaded: counts={:?}; excluded baseline direct rows={:?}; full templates/equip/instances still required.",
        items.counts(),
        items.unknown_baseline_records()
    );
    println!(
        "Forever numeric item template metadata derived: templates={}; effective spec/override/gem counts={:?}; full template/bonus/use/equip/script operations and native item wire acceptance still required.",
        items.template_count(),
        item_spec_counts
    );
    let starting_policy =
        StartingPolicy::from_config(starting_config, max_level, character_templates.clone())
            .map_err(|error| anyhow::anyhow!("target numeric start policy rejected: {error:?}"))?;
    let catalog = CharacterCatalog::load(&initialization, &achievements, availability_rows)
        .map_err(anyhow::Error::msg)?;
    let birth_skill_order = super::birth_skill_lookup::order(&birth)?;
    let creation_sources = wow_world::forever::creation::WorldSources::load(
        creation_rows,
        &initialization,
        &game_tables,
        max_level,
    )
    .and_then(|sources| sources.with_birth_skills(birth.clone()))
    .and_then(|sources| sources.with_birth_skill_lookup(birth_skill_order))
    .map_err(|error| anyhow::anyhow!("target creation world source rejected: {error:?}"))?;
    println!(
        "Forever source skill race/class lookup loaded: counts={:?}; actual Player SetSkill/learning/child effects still required.",
        creation_sources.birth_skill_lookup_counts()
    );
    println!(
        "Forever creation world sources loaded: counts={:?}; XP row/cap metadata={:?}; effective class/race counts={:?}; initial skill source counts={:?}; full Player validation/save still required.",
        creation_sources.counts(),
        creation_sources.progression_counts(),
        initialization.identity_counts(),
        creation_sources.birth_skill_source_counts()
    );
    println!(
        "Forever effective initialization data loaded: counts={:?}; GT rows including unused zero={:?}; full Player initialization/persistence still required.",
        initialization.counts(),
        game_tables.counts()
    );
    let names = name_rows.finish(
        super::names::records(name_overlays.official),
        super::names::records(name_overlays.custom),
        &removals,
    )?;
    println!(
        "Forever name data loaded: {} profanity and {} locale-reserved patterns for esES, {} global reserved patterns; compiling target expressions.",
        names.profanity(6).context("name locale")?.len(),
        names.locale_reserved(6).context("name locale")?.len(),
        names.reserved().len(),
    );
    let name_rules = Arc::new(super::names::compile(&names, reserved_names)?);
    drop(names); // Compiled immutable rules are now the sole runtime policy.
    let name_policy = NamePolicy {
        minimum_units: wow_config::get_value_default("MinPlayerName", 2_u32).clamp(1, 12),
        strict_mask: wow_config::get_value_default("StrictPlayerNames", 0_u32),
        creation_charset: 2, // Replaced using the admitted real realm timezone.
    };
    println!(
        "Target name expressions compiled; database-backed availability registered, no character creation admitted."
    );
    hotfixes.apply_hotfix_data_rows_like_cpp(
        rows.into_iter().map(|row| {
            (
                row.push_id,
                row.unique_id,
                row.table_hash,
                row.record_id,
                row.status,
            )
        }),
        locale,
    );
    let rows = match hotfix_adapter
        .load_hotfix_optional_data_rows_like_cpp()
        .await
    {
        HotfixDeliveryMetadataLoadOutcomeLikeCpp::Loaded(rows) => rows,
        _ => bail!("hotfix optional query failed"),
    };
    hotfixes.apply_hotfix_optional_data_rows_like_cpp(
        // DB2Stores.cpp:1866 only allows optional TactKey data on BroadcastText,
        // NOT on TactKey or any item/spell store. Skip those rows.
        rows.into_iter()
            .filter(|row| {
                row.table_hash != TACT_KEY_TABLE_HASH
                    && !ITEM_TABLE_HASHES.contains(&row.table_hash)
                    && !SPELL_TABLE_HASHES.contains(&row.table_hash)
            })
            .map(|row| (row.table_hash, row.record_id, row.locale, row.key, row.data)),
        locale,
    );
    // Names and expansion are taken from the admitted auth/realm rows later;
    // this policy controls only this isolated target's unimplemented services.
    let (content_set, super_district) = super::ruleset::isolated_binding(
        &wow_config::get_string_default("Forever.RealmBindings", "[]"),
    )?;
    let policy = InitializationPolicy {
        realm_name: "RustyCore Forever".into(),
        normalized_realm_name: "RustyCoreForever".into(),
        timezone: "Etc/UTC".into(),
        cache_version: 0,
        content_set,
        max_characters: 200,
        character_templates,
        classic_friends_subscriptions: build >= 70291,
    };
    // World.cpp:726,1061 and WorldSession::ResetTimeOutTime(false): inactive
    // character selection uses SocketTimeOutTime, not the 30-second probe read.
    // Player-active timeout and queue exemptions belong to the later player port.
    let idle_ms = wow_config::get_value_default("SocketTimeOutTime", 900000_u32);
    ensure!(idle_ms >= 1000, "invalid character idle timeout");
    let character_idle_timeout = std::time::Duration::from_secs(u64::from(idle_ms / 1000));
    let hotfixes = ForeverHotfixCatalog::new(tact_keys, hotfixes)?
        .with_item_stores(item_data, item_specs)?
        .with_spell_stores(spells.clone())?;
    println!(
        "Forever appearance prerequisites loaded: {} race/gender option indexes; no creation admitted.",
        appearance.indexed_race_gender_count()
    );
    println!(
        "Forever prerequisites loaded: {} availability races, {} readable achievements, {} hotfix records, {} effective TactKey records; no session admitted yet.",
        catalog.races().len(),
        achievements.available_count(),
        hotfixes.metadata().hotfix_count(),
        hotfixes.tact_key_count()
    );
    Ok(Runtime {
        build,
        dbc_locale,
        auth: auth.clone(),
        session_repository,
        name_rules,
        name_policy,
        permissions,
        starting_policy,
        super_district,
        catalog,
        appearance: Arc::new(appearance),
        creation_sources,
        initialization: Arc::new(initialization),
        birth,
        spells,
        spell_definitions,
        items: Arc::new(items),
        game_tables,
        hotfixes: Arc::new(hotfixes),
        build_key,
        policy,
        character_idle_timeout,
        character_capture,
        navigation,
        region_group: wow_config::get_value_default("Network.EnterEncryptedModeRegionGroup", 0_i32),
    })
}
