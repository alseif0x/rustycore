use super::*;
use wow_entities::{AccessorObjectKind, Creature, MapObjectRecord, Player, WorldObject};
use wow_core::Position;
use wow_constants::{TypeId, TypeMask};

fn creature_record() -> MapObjectRecord {
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(
        ObjectGuid::create_creature_like_cpp(1, 571, 42, 81),
    );
    creature.unit_mut().world_mut().set_map(571, 9).unwrap();
    creature.unit_mut().world_mut().relocate(Position::xyz(1.0, 2.0, 3.0));
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(75);
    creature.set_spawn_id(81);
    creature.set_spawn_string_id_runtime_like_cpp(Some("owned-provenance-buffer".to_string()));
    MapObjectRecord::new_creature(creature).unwrap()
}

fn apply_addon(record: &mut MapObjectRecord, spells: &[u32]) {
    let addon = wow_entities::CreatureAddonLifecycleRecordLikeCpp {
        aura_applications: spells.iter().map(|spell_id| {
            wow_entities::CreatureAddonAuraApplicationLikeCpp {
                spell_id: *spell_id, spell_visual_id: 1234, effect_mask: 1,
                flags: 0, effects: Vec::new(),
            }
        }).collect(),
        ..wow_entities::CreatureAddonLifecycleRecordLikeCpp::default()
    };
    assert!(record.creature_mut().unwrap().apply_creatures_addon_lifecycle_like_cpp(Some(&addon)));
}

#[test]
fn settlement_moves_the_same_primary_and_installs_cast_ids_in_pending_order() {
    let mut map = wow_map::Map::new(571, 9, 0, 60_000);
    let mut record = creature_record();
    apply_addon(&mut record, &[81001, 81002]);
    let creature = record.creature().unwrap();
    let guid = creature.guid();
    let pointer = creature as *const Creature;
    let string = creature.lifecycle_metadata().string_id.as_ref().unwrap().as_ptr();
    let timeline = creature.unit().health_state_revision_authority_like_cpp();
    let loot = creature.loot_authority_like_cpp().clone();
    let records = settle_creature_record(&mut map, Ok(Some(record)), 81, 42, guid)
        .unwrap().unwrap();
    assert!(records.pre_add_records.is_empty());
    let moved = records.primary_record.creature().unwrap();
    assert_eq!(moved as *const Creature, pointer);
    assert_eq!(moved.lifecycle_metadata().string_id.as_ref().unwrap().as_ptr(), string);
    assert!(moved.unit().shares_health_state_revision_authority_like_cpp(&timeline));
    assert!(moved.loot_authority_like_cpp().shares_storage_like_cpp(&loot));
    assert_eq!((moved.current_health(), moved.max_health()), (75, 100));
    let auras = &moved.unit().subsystems().auras;
    for (spell_id, counter) in [(81001, 1), (81002, 2)] {
        let slot = auras.visible_auras.iter()
            .find_map(|(slot, aura)| (aura.spell_id == spell_id).then_some(*slot)).unwrap();
        let provenance = auras.aura_cast_provenance_like_cpp(slot);
        assert_eq!(provenance.cast_id.entry(), spell_id);
        assert_eq!(provenance.cast_id.counter(), counter);
        assert_eq!(provenance.spell_visual_id, 1234);
    }
    assert_eq!(map.get_max_low_guid_like_cpp(HighGuid::Cast), Ok(3));
    assert_eq!(map.map_object_count(), 0);
}

#[test]
fn returned_error_projection_keeps_already_installed_provenance_and_authorities() {
    let mut map = wow_map::Map::new(571, 9, 0, 60_000);
    let mut record = creature_record();
    apply_addon(&mut record, &[81003]);
    let creature = record.creature_mut().unwrap();
    assert_eq!(map.settle_creature_addon_aura_provenance_like_cpp(creature), Ok(1));
    let pointer = creature as *const Creature;
    let string = creature.lifecycle_metadata().string_id.as_ref().unwrap().as_ptr();
    let timeline = creature.unit().health_state_revision_authority_like_cpp();
    let revision = creature.unit().health_state_revision_like_cpp();
    let loot = creature.loot_authority_like_cpp().clone();
    let slot = *creature.unit().subsystems().auras.visible_auras.keys().next().unwrap();
    let provenance = creature.unit().subsystems().auras.aura_cast_provenance_like_cpp(slot);
    // Transport-only input: HighGuid::Cast does not actually return this error.
    // This neither injects an allocator nor models overflow panic as Result.
    let error = wow_map::MapGuidSequenceErrorLikeCpp::UnsupportedSequenceSource {
        high: HighGuid::Player,
    };
    let rejection = project_settlement(record, Err(error)).unwrap_err();
    let LoadedGridCreaturePreparationError::AddonProvenance { error: returned, records } = rejection else {
        panic!("expected owned provenance rejection");
    };
    assert_eq!(returned, error);
    assert!(records.pre_add_records.is_empty());
    let moved = records.primary_record.creature().unwrap();
    assert_eq!(moved as *const Creature, pointer);
    assert_eq!(moved.lifecycle_metadata().string_id.as_ref().unwrap().as_ptr(), string);
    assert!(moved.unit().shares_health_state_revision_authority_like_cpp(&timeline));
    assert_eq!(moved.unit().health_state_revision_like_cpp(), revision);
    assert!(moved.loot_authority_like_cpp().shares_storage_like_cpp(&loot));
    assert_eq!(moved.unit().subsystems().auras.aura_cast_provenance_like_cpp(slot), provenance);
    assert_eq!(map.get_max_low_guid_like_cpp(HighGuid::Cast), Ok(2));
    assert_eq!(map.map_object_count(), 0);
}

#[test]
fn rejected_player_primary_is_retained_before_any_cast_allocation() {
    let mut map = wow_map::Map::new(571, 9, 0, 60_000);
    let mut player = Player::new(None, false);
    let guid = ObjectGuid::create_player(1, 82);
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().world_mut().set_map(571, 9).unwrap();
    let record = MapObjectRecord::new_player(player).unwrap();
    let pointer = record.player().unwrap() as *const Player;
    let rejection = settle_creature_record(&mut map, Ok(Some(record)), 81, 42, guid).unwrap_err();
    let LoadedGridCreaturePreparationError::NotCreature(records) = rejection else {
        panic!("expected owned typed-body rejection");
    };
    assert_eq!(records.primary_record.player().unwrap() as *const Player, pointer);
    assert_eq!(map.get_max_low_guid_like_cpp(HighGuid::Cast), Ok(1));
    assert_eq!(map.map_object_count(), 0);
}

#[test]
fn generic_creature_kind_does_not_bypass_typed_body_gate() {
    let mut map = wow_map::Map::new(571, 9, 0, 60_000);
    let mut object = WorldObject::new(false, TypeId::Unit, TypeMask::OBJECT | TypeMask::UNIT);
    let guid = ObjectGuid::create_creature_like_cpp(1, 571, 42, 83);
    object.object_mut().create(guid);
    object.set_map(571, 9).unwrap();
    let record = MapObjectRecord::new(AccessorObjectKind::Creature, object).unwrap();
    let rejection = settle_creature_record(&mut map, Ok(Some(record)), 81, 42, guid).unwrap_err();
    let LoadedGridCreaturePreparationError::NotCreature(records) = rejection else {
        panic!("expected generic body rejection");
    };
    assert_eq!(records.primary_record.object().guid(), guid);
    assert!(records.primary_record.creature().is_none());
    assert_eq!(map.get_max_low_guid_like_cpp(HighGuid::Cast), Ok(1));
}

#[test]
fn resolver_rejection_and_no_requested_record_remain_none_without_dummy_payload() {
    let mut map = wow_map::Map::new(571, 9, 0, 60_000);
    let guid = ObjectGuid::create_creature_like_cpp(1, 571, 42, 84);
    let rejection = creature_loaded_grid::CreatureLoadedGridResolveErrorLikeCpp::MissingSpawnData {
        spawn_id: 81,
    };
    assert!(settle_creature_record(&mut map, Err(rejection), 81, 42, guid).unwrap().is_none());
    assert!(settle_creature_record(&mut map, Ok(None), 81, 42, guid).unwrap().is_none());
    assert_eq!(map.get_max_low_guid_like_cpp(HighGuid::Cast), Ok(1));
    assert_eq!(map.map_object_count(), 0);
}

#[test]
fn raw_preparer_error_projects_all_original_preadds_into_typed_error() {
    use crate::runtime::game_events::spawn_helpers::prepare_loaded_grid_creature;
    let mut object = WorldObject::new(false, TypeId::Unit, TypeMask::OBJECT | TypeMask::UNIT);
    let guid = ObjectGuid::create_creature_like_cpp(1, 571, 42, 85);
    object.object_mut().create(guid);
    object.set_map(571, 9).unwrap();
    let primary = MapObjectRecord::new(AccessorObjectKind::Creature, object).unwrap();
    let first = creature_record();
    let second = creature_record();
    let first_pointer = first.creature().unwrap() as *const Creature;
    let second_pointer = second.creature().unwrap() as *const Creature;
    let result = prepare_loaded_grid_creature(
        wow_map::map::LoadedGridRespawnRecordsLikeCpp {
            pre_add_records: vec![first, second], primary_record: primary,
        },
        &crate::spawn_store_loader::WaypointPathStoreLikeCpp::default(),
    ).map_err(LoadedGridCreaturePreparationError::NotCreature);
    let LoadedGridCreaturePreparationError::NotCreature(records) = result.unwrap_err() else {
        panic!("expected full raw extractor payload");
    };
    assert_eq!(records.primary_record.object().guid(), guid);
    assert_eq!(records.pre_add_records.len(), 2);
    assert_eq!(records.pre_add_records[0].creature().unwrap() as *const Creature, first_pointer);
    assert_eq!(records.pre_add_records[1].creature().unwrap() as *const Creature, second_pointer);
}

#[test]
fn legacy_option_projection_discards_rejection_without_new_guid_allocation() {
    let mut map = wow_map::Map::new(571, 9, 0, 60_000);
    let record = creature_record();
    // A supplied error VALUE tests projection, not a reachable Cast allocator failure.
    let error = wow_map::MapGuidSequenceErrorLikeCpp::UnsupportedSequenceSource {
        high: HighGuid::Player,
    };
    assert!(project_settlement(record, Err(error)).ok().flatten().is_none());
    assert_eq!(map.get_max_low_guid_like_cpp(HighGuid::Cast), Ok(1));
    assert_eq!(map.map_object_count(), 0);
}
