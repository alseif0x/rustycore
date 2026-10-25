use super::*;
use std::sync::Arc;
use std::time::Instant;
use wow_core::{ObjectGuid, Position};
use wow_entities::{AuraApplicationLikeCpp as AuraApplication, Player};

fn empty_respawn_caches() -> LoadedGridCreatureRespawnCachesLikeCpp {
    LoadedGridCreatureRespawnCachesLikeCpp {
        realm_id: 1,
        template_store: Default::default(),
        sparring_store: Default::default(),
        difficulty_store: Default::default(),
        base_stats_store: Default::default(),
        chr_classes_store: Arc::new(
            wow_data::character_progression::ChrClassesStore::from_entries([]),
        ),
        power_type_store: Arc::new(
            wow_data::character_progression::PowerTypeStore::from_entries([]),
        ),
        health_rates: Default::default(),
        display_store: Arc::new(wow_data::CreatureDisplayInfoStore::from_entries([])),
        model_store: Arc::new(wow_data::CreatureModelDataStore::from_entries([])),
        model_info_store: Arc::new(wow_data::CreatureModelInfoStoreLikeCpp::from_entries([])),
        creature_equipment_store: Default::default(),
        creature_addon_store: Default::default(),
        spell_x_spell_visual_store: Arc::new(wow_data::SpellXSpellVisualStore::from_entries([])),
        vehicle_store: Arc::new(wow_data::VehicleStore::from_entries([])),
        vehicle_seat_store: Arc::new(wow_data::VehicleSeatStore::from_entries([])),
        vehicle_accessory_store: Arc::new(wow_data::VehicleAccessoryStoreLikeCpp::from_parts(
            [],
            [],
        )),
        gameobject_template_store: Default::default(),
        gameobject_override_store: Default::default(),
    }
}

#[test]
fn map_tick_flushes_selected_aura_when_creature_owner_is_absent() {
    let key = wow_map::MapKey::new(571, 0);
    let guid = ObjectGuid::create_player(1, 8_420_001);
    let mut player = Box::new(Player::new(Some(1), false));
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().set_level(80);
    player
        .unit_mut()
        .subsystems_mut()
        .auras
        .insert_runtime_application_like_cpp(AuraApplication {
            spell_id: 20_584,
            difficulty_id: 0,
            caster_guid: guid,
            slot: 7,
            duration_total: 0,
            duration_remaining: 0,
            stack_count: 1,
            aura_flags: 1,
            effect_mask: 0b111,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: None,
            represented_amount: 0,
            represented_effect_amounts: Vec::new(),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: Instant::now(),
        });
    player
        .unit_mut()
        .subsystems_mut()
        .auras
        .visible_auras_to_update
        .insert(7);

    let mut manager = wow_map::MapManager::new(wow_map::MIN_GRID_DELAY_MS, 10);
    manager.create_world_map(key.map_id, key.instance_id);
    let handle = manager.install_detached_player_like_cpp(player).unwrap();
    manager
        .attach_player_like_cpp(handle, key, Position::ZERO)
        .unwrap();

    let plan = canonical_map_tick_begin_like_cpp(&mut manager, 100)
        .expect("canonical map tick begins");
    let metadata = crate::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::default();
    let conditions = wow_data::ConditionEntriesByTypeStore::default();
    let maps = wow_data::MapStore::from_entries(std::iter::empty());
    let caches = empty_respawn_caches();
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1_000);

    let summary = canonical_map_tick_resume_with_creature_owner_like_cpp(
        &mut manager,
        plan.plan,
        &mut scheduler,
        &metadata,
        &conditions,
        &maps,
        &caches,
        None,
    )
    .expect("same map tick resumed without a Creature callback");

    assert_eq!(summary.selected_aura_updates.len(), 1);
    let aura_opcode = (wow_constants::ServerOpcodes::AuraUpdate as u16).to_le_bytes();
    assert_eq!(
        summary.selected_aura_updates[0].packet_bytes().get(..2),
        Some(aura_opcode.as_slice())
    );
    assert!(manager
        .with_player_like_cpp(handle, |player| player
            .unit()
            .subsystems()
            .auras
            .visible_auras_to_update
            .is_empty())
        .unwrap());
}
