//! The `player.login` hook through the whole production login (#1445 review
//! L3): wow-world without cfg(test), a canonical MapManager, every load
//! answering empty so the login runs to its end, and a probe module.
use super::*;
use std::sync::Mutex;
use wow_module_api::{
    ModuleDescriptor, ModuleHooks, ModuleRegistry, ModuleVersion, PlayerLoginModule,
    PlayerLoginSnapshot, ScopedEffects,
};

pub(super) fn empty_auxiliary_load(
    request: PlayerLoginAuxiliaryLoadRequestLikeCpp,
) -> PersistenceFutureLikeCpp<'static, PlayerLoginAuxiliaryLoadOutcomeLikeCpp> {
    use PlayerLoginAuxiliaryLoadRequestLikeCpp as Request;
    use PlayerLoginAuxiliaryLoadedLikeCpp as Loaded;
    let loaded = match request {
        Request::Mail { .. } => Loaded::Mail(vec![]),
        Request::Customizations { .. } => Loaded::Customizations(vec![]),
        Request::CompletedAchievements { .. } => Loaded::CompletedAchievements(vec![]),
        Request::InstanceTimeRestrictions { .. } => Loaded::InstanceTimeRestrictions(vec![]),
        Request::SpellCooldowns { .. } => Loaded::SpellCooldowns(vec![]),
        Request::SpellCharges { .. } => Loaded::SpellCharges(vec![]),
        Request::TraitEntries { .. } => Loaded::TraitEntries(vec![]),
        Request::TraitConfigs { .. } => Loaded::TraitConfigs(vec![]),
        Request::PetStable { .. } => Loaded::PetStable(vec![]),
        Request::PetAuras { .. } => Loaded::PetAuras(vec![]),
        Request::PetAuraEffects { .. } => Loaded::PetAuraEffects(vec![]),
        Request::PetSpells { .. } => Loaded::PetSpells(vec![]),
        Request::PetSpellCooldowns { .. } => Loaded::PetSpellCooldowns(vec![]),
        Request::PetSpellCharges { .. } => Loaded::PetSpellCharges(vec![]),
        Request::PetDeclinedNames { .. } => Loaded::PetDeclinedNames(vec![]),
        Request::GroupMembership { .. } => Loaded::GroupMembership(vec![]),
        Request::EquipmentSets { .. } => Loaded::EquipmentSets(vec![]),
        Request::TransmogOutfits { .. } => Loaded::TransmogOutfits(vec![]),
        Request::CufProfiles { .. } => Loaded::CufProfiles(vec![]),
        Request::Currencies { .. } => Loaded::Currencies(vec![]),
        Request::Spells { .. } => Loaded::Spells(vec![]),
        Request::SpellFavorites { .. } => Loaded::SpellFavorites(vec![]),
        Request::Skills { .. } => Loaded::Skills(vec![]),
        Request::Talents { .. } => Loaded::Talents(vec![]),
        Request::Glyphs { .. } => Loaded::Glyphs(vec![]),
        Request::ActionButtons { .. } => Loaded::ActionButtons(vec![]),
        Request::Reputation { .. } => Loaded::Reputation(vec![]),
        Request::CharacterAuras { .. } => Loaded::CharacterAuras(vec![]),
        Request::CharacterAuraEffects { .. } => Loaded::CharacterAuraEffects(vec![]),
        Request::EquipmentInventory { .. } => Loaded::EquipmentInventory(vec![]),
        Request::BagInventory { .. } => Loaded::BagInventory(vec![]),
        Request::VoidStorage { .. } => Loaded::VoidStorage(vec![]),
    };
    Box::pin(async move { PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(loaded) })
}

/// Records the call, what the port had persisted by then, whether the
/// canonical MapManager lock was free, and greets.
struct Probe {
    port: Arc<LoginPort>,
    manager: Arc<std::sync::Mutex<wow_map::MapManager>>,
    calls: Mutex<Vec<(PlayerLoginSnapshot, Vec<&'static str>, bool)>>,
}

impl PlayerLoginModule for Probe {
    fn on_player_login(&self, snapshot: &PlayerLoginSnapshot, effects: &mut ScopedEffects<'_>) {
        let persisted = self.port.events.lock().unwrap().clone();
        let map_free = self.manager.try_lock().is_ok();
        self.calls
            .lock()
            .unwrap()
            .push((snapshot.clone(), persisted, map_free));
        effects.send_system_message_self("Welcome from the probe module.");
    }
}

struct Fixture {
    session: WorldSession,
    port: Arc<LoginPort>,
    send_rx: flume::Receiver<Vec<u8>>,
    manager: Arc<std::sync::Mutex<wow_map::MapManager>>,
    maps: Arc<MapStore>,
}

/// A session ready to log character 42 in through the production login.
fn fixture() -> Fixture {
    let (_, packet_rx) = flume::bounded(8);
    let (send_tx, send_rx) = flume::unbounded();
    let mut session = WorldSession::new(
        1,
        "fixture".into(),
        0,
        2,
        2,
        54261,
        vec![],
        "enUS".into(),
        packet_rx,
        send_tx,
        wow_world::session::registry::build_dispatch_table(),
    );
    let guid = ObjectGuid::create_player(1, 42);
    let manager = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(300_000, 10)));
    let port = Arc::new(LoginPort {
        retain_after_save: AtomicBool::new(false),
        finalization_probe: std::sync::Mutex::new(None),
        save_probe: std::sync::Mutex::new(None),
        reached_pet_load: AtomicBool::new(false),
        reached_inventory_load: AtomicBool::new(false),
        stop_at_pet_load: false,
        manager: Arc::clone(&manager),
        full_login: AtomicBool::new(true),
        events: std::sync::Mutex::new(Vec::new()),
    });
    let maps = Arc::new(MapStore::from_entries([MapEntry {
        id: 0,
        instance_type: 0,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]));
    session.set_map_store(Arc::clone(&maps));
    session.set_canonical_map_manager(Arc::clone(&manager));
    session.set_player_lifecycle_port_like_cpp(port.clone());
    session.set_player_loading(Some(guid));
    Fixture {
        session,
        port,
        send_rx,
        manager,
        maps,
    }
}

/// Run the production login to its end.
async fn run_login(session: &mut WorldSession, maps: &Arc<MapStore>) {
    let maps = Arc::clone(maps);
    let bootstrap = PlayerBootstrapCatalogsLikeCpp {
        create_info: Arc::new(PlayerCreateInfoStoreLikeCpp::from_rows_like_cpp(
            [PlayerCreateInfoRowLikeCpp {
                race: 1,
                class: 1,
                create_position: PlayerCreatePositionLikeCpp {
                    map_id: 0,
                    position: Position::new(1.0, 2.0, 3.0, 0.0),
                    transport_guid: None,
                },
                create_position_npe: None,
                npe_transport_template_valid: true,
            }],
            &maps,
            |_| true,
            |_| true,
            |_| true,
        )),
        cast_spells: Arc::new(PlayerCreateInfoCastSpellStoreLikeCpp::default()),
        glyph_properties: Arc::new(wow_data::GlyphPropertiesStore::from_entries([])),
        talent_tabs: Arc::new(wow_data::TalentTabStore::from_entries([])),
        trait_node_entries: Arc::new(wow_data::trait_tree::TraitNodeEntryStore::from_entries([])),
        custom_spells: Arc::new(PlayerCreateInfoCustomSpellStoreLikeCpp::default()),
        start_all_spells: false,
        start_all_explored: false,
        start_all_reputation: false,
    };
    let creatures = CreatureSpawnCatalogsLikeCpp {
        difficulty: Arc::new(CreatureDifficultyStoreLikeCpp::default()),
        base_stats: Arc::new(CreatureBaseStatsStoreLikeCpp::default()),
        health_rates: CreatureClassificationHealthRatesLikeCpp::default(),
        addons: Arc::new(CreatureAddonStoreLikeCpp::default()),
        equipment: Arc::new(CreatureEquipmentStoreLikeCpp::default()),
        power_types: Arc::new(PowerTypeStore::from_entries([])),
    };
    let progression = ProgressionCatalogsLikeCpp {
        no_reset_talent_cost: false,
        player_xp: Arc::new(vec![0, 400]),
        exploration_base_xp: Arc::new(ExplorationBaseXpStoreLikeCpp::default()),
        exploration_xp_rate: 1.0,
        min_discovered_scaled_xp_ratio: 0,
    };
    let grid: PlayerGridLoadResolverLikeCpp =
        Arc::new(|_, _, _| PlayerGridLoadOutcomeLikeCpp::default());
    let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
    let rest = PlayerRestRatePolicyLikeCpp::default();
    let features = SupportFeaturePolicyLikeCpp::default();
    session
        .handle_continue_player_login_with_catalogs_like_cpp(
            &generator,
            &creatures,
            &bootstrap,
            &rest,
            &progression,
            &features,
            &grid,
        )
        .await;
}

fn probe_registry(probe: Arc<Probe>) -> ModuleRegistry {
    let mut registry = ModuleRegistry::new();
    registry
        .register_module(
            ModuleDescriptor::new(
                "alpha.probe",
                ModuleVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                "Probe",
            )
            .unwrap(),
            ModuleHooks::new().player_login(probe),
        )
        .unwrap();
    registry
}

const GREETING: &str = "Welcome from the probe module.";

fn carries(bytes: &[u8], text: &str) -> bool {
    bytes
        .windows(text.len())
        .any(|window| window == text.as_bytes())
}

#[tokio::test]
async fn production_login_greets_once_after_the_online_mark_with_no_map_guard() {
    let mut fixture = fixture();
    let probe = Arc::new(Probe {
        port: Arc::clone(&fixture.port),
        manager: Arc::clone(&fixture.manager),
        calls: Mutex::new(Vec::new()),
    });
    fixture
        .session
        .install_module_hooks_like_cpp(Arc::new(probe_registry(Arc::clone(&probe))));

    run_login(&mut fixture.session, &fixture.maps).await;

    let calls = probe.calls.lock().unwrap();
    assert_eq!(calls.len(), 1, "the login hook runs exactly once");
    let (snapshot, persisted, map_free) = &calls[0];
    assert_eq!(snapshot.guid, ObjectGuid::create_player(1, 42));
    assert_eq!(snapshot.name, "Bootstrap");
    assert_eq!(
        persisted,
        &vec!["online"],
        "the module runs after the online mark persisted"
    );
    assert!(
        *map_free,
        "no canonical map guard is held during the callback"
    );

    let packets: Vec<Vec<u8>> = fixture.send_rx.try_iter().collect();
    assert!(packets.len() > 1, "the login sequence was published");
    assert!(
        carries(packets.last().unwrap(), GREETING),
        "the greeting is the last packet of the login"
    );
    assert_eq!(
        packets
            .iter()
            .filter(|bytes| carries(bytes, GREETING))
            .count(),
        1
    );
}

#[tokio::test]
async fn production_login_with_zero_modules_sends_no_greeting() {
    let mut fixture = fixture();
    run_login(&mut fixture.session, &fixture.maps).await;
    assert_eq!(*fixture.port.events.lock().unwrap(), vec!["online"]);
    let packets: Vec<Vec<u8>> = fixture.send_rx.try_iter().collect();
    assert!(!packets.is_empty());
    assert!(!packets.iter().any(|bytes| carries(bytes, GREETING)));
}
