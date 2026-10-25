use super::selected_caster::SelectedCasterLevelError;
use super::*;
use wow_core::{ObjectGuid, Position};
use wow_data::{DifficultyEntry, DifficultyStore, SpellLevelsEntry, SpellLevelsStore};
use wow_entities::{Creature, MapObjectRecord, Player};
use wow_map::{MapKey, MapManager, PlayerHandle};

const MAP: MapKey = MapKey::new(571, 0);
const OTHER_MAP: MapKey = MapKey::new(530, 0);
const SPELL_ID: u32 = 20_584;

fn add_player(
    manager: &mut MapManager,
    guid: ObjectGuid,
    level: u8,
    map: Option<MapKey>,
) -> PlayerHandle {
    let mut player = Box::new(Player::new(Some(1), false));
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().set_level(level);
    let handle = manager.install_detached_player_like_cpp(player).unwrap();
    if let Some(map) = map {
        manager
            .attach_player_like_cpp(handle, map, Position::ZERO)
            .unwrap();
    }
    handle
}

fn manager_with_maps(keys: &[MapKey]) -> MapManager {
    let mut manager = MapManager::default();
    for key in keys {
        manager.create_world_map(key.map_id, key.instance_id);
    }
    manager
}

fn level_entry(id: u32, spell_id: u32, difficulty_id: u8, spell_level: i16) -> SpellLevelsEntry {
    SpellLevelsEntry {
        id,
        difficulty_id,
        base_level: 0,
        max_level: 0,
        spell_level,
        max_passive_aura_level: 0,
        spell_id,
    }
}

fn difficulty_entry(id: u32, fallback_difficulty_id: u8) -> DifficultyEntry {
    DifficultyEntry {
        id,
        instance_type: 0,
        flags: 0,
        fallback_difficulty_id,
        toggle_difficulty_id: 0,
    }
}

fn spell_levels(rows: impl IntoIterator<Item = SpellLevelsEntry>) -> SpellLevelsStore {
    SpellLevelsStore::from_entries(rows)
}

#[test]
fn detached_self_caster_uses_canonical_level_without_spell_levels() {
    let mut manager = MapManager::default();
    let owner_guid = ObjectGuid::create_player(1, 91_001);
    let owner = add_player(&mut manager, owner_guid, 73, None);

    assert_eq!(
        selected_caster_level(&manager, owner, owner_guid, SPELL_ID, 0, None, None),
        Ok(73),
    );
}

#[test]
fn assigned_owner_need_not_be_in_world_to_resolve_same_map_player() {
    let mut manager = manager_with_maps(&[MAP]);
    let owner_guid = ObjectGuid::create_player(1, 91_002);
    let caster_guid = ObjectGuid::create_player(1, 91_003);
    let owner = add_player(&mut manager, owner_guid, 70, Some(MAP));
    add_player(&mut manager, caster_guid, 52, Some(MAP));
    manager
        .with_player_mut_like_cpp(owner, |player| {
            player
                .unit_mut()
                .world_mut()
                .object_mut()
                .remove_from_world();
        })
        .unwrap();

    assert_eq!(
        selected_caster_level(&manager, owner, caster_guid, SPELL_ID, 0, None, None),
        Ok(52),
    );
}

#[test]
fn foreign_player_on_another_map_not_in_world_or_missing_uses_spell_level() {
    let mut manager = manager_with_maps(&[MAP, OTHER_MAP]);
    let owner_guid = ObjectGuid::create_player(1, 91_004);
    let other_map_guid = ObjectGuid::create_player(1, 91_005);
    let out_of_world_guid = ObjectGuid::create_player(1, 91_006);
    let missing_guid = ObjectGuid::create_player(1, 91_007);
    let owner = add_player(&mut manager, owner_guid, 70, Some(MAP));
    add_player(&mut manager, other_map_guid, 22, Some(OTHER_MAP));
    let out_of_world = add_player(&mut manager, out_of_world_guid, 23, Some(MAP));
    manager
        .with_player_mut_like_cpp(out_of_world, |player| {
            player
                .unit_mut()
                .world_mut()
                .object_mut()
                .remove_from_world();
        })
        .unwrap();
    let levels = spell_levels([level_entry(1, SPELL_ID, 0, 44)]);

    for caster_guid in [other_map_guid, out_of_world_guid, missing_guid] {
        assert_eq!(
            selected_caster_level(
                &manager,
                owner,
                caster_guid,
                SPELL_ID,
                0,
                Some(&levels),
                None,
            ),
            Ok(44),
        );
    }
}

#[test]
fn creature_membership_in_the_owner_map_supplies_its_actual_level() {
    let mut manager = manager_with_maps(&[MAP, OTHER_MAP]);
    let owner_guid = ObjectGuid::create_player(1, 91_008);
    let owner = add_player(&mut manager, owner_guid, 70, Some(MAP));
    let creature_guid =
        ObjectGuid::create_creature_like_cpp(1, u16::try_from(MAP.map_id).unwrap(), 91_009, 1);
    let mut creature = Creature::new(false);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(creature_guid);
    creature
        .unit_mut()
        .world_mut()
        .set_map(MAP.map_id, MAP.instance_id)
        .unwrap();
    creature.unit_mut().world_mut().relocate(Position::ZERO);
    creature.unit_mut().world_mut().object_mut().add_to_world();
    creature.unit_mut().set_level(39);
    manager
        .find_map_mut(MAP.map_id, MAP.instance_id)
        .unwrap()
        .map_mut()
        .insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    manager
        .find_map_mut(MAP.map_id, MAP.instance_id)
        .unwrap()
        .map_mut()
        .get_typed_creature_mut(creature_guid)
        .unwrap()
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();

    assert_eq!(
        selected_caster_level(&manager, owner, creature_guid, SPELL_ID, 0, None, None),
        Ok(39),
    );
    manager
        .find_map_mut(MAP.map_id, MAP.instance_id)
        .unwrap()
        .map_mut()
        .get_typed_creature_mut(creature_guid)
        .unwrap()
        .unit_mut()
        .world_mut()
        .reset_map()
        .unwrap();
    manager
        .find_map_mut(MAP.map_id, MAP.instance_id)
        .unwrap()
        .map_mut()
        .get_typed_creature_mut(creature_guid)
        .unwrap()
        .unit_mut()
        .world_mut()
        .set_map(OTHER_MAP.map_id, OTHER_MAP.instance_id)
        .unwrap();
    assert_eq!(
        selected_caster_level(&manager, owner, creature_guid, SPELL_ID, 0, None, None),
        Err(SelectedCasterLevelError::CasterUnavailable),
    );
}

#[test]
fn foreign_player_body_with_a_map_assignment_mismatched_to_its_container_is_unavailable() {
    let mut manager = manager_with_maps(&[MAP, OTHER_MAP]);
    let owner_guid = ObjectGuid::create_player(1, 91_020);
    let caster_guid = ObjectGuid::create_player(1, 91_021);
    let owner = add_player(&mut manager, owner_guid, 70, Some(MAP));
    let caster = add_player(&mut manager, caster_guid, 52, Some(MAP));
    manager
        .with_player_mut_like_cpp(caster, |player| {
            player
                .unit_mut()
                .world_mut()
                .object_mut()
                .remove_from_world();
            player.unit_mut().world_mut().reset_map().unwrap();
            player
                .unit_mut()
                .world_mut()
                .set_map(OTHER_MAP.map_id, OTHER_MAP.instance_id)
                .unwrap();
            player.unit_mut().world_mut().object_mut().add_to_world();
        })
        .unwrap();

    assert_eq!(
        selected_caster_level(&manager, owner, caster_guid, SPELL_ID, 0, None, None),
        Err(SelectedCasterLevelError::CasterUnavailable),
    );
}

#[test]
fn known_empty_spell_levels_default_to_zero_but_missing_store_is_unknown() {
    let mut manager = manager_with_maps(&[MAP]);
    let owner_guid = ObjectGuid::create_player(1, 91_010);
    let missing_guid = ObjectGuid::create_player(1, 91_011);
    let owner = add_player(&mut manager, owner_guid, 70, Some(MAP));
    let empty_levels = spell_levels(std::iter::empty());

    assert_eq!(
        selected_caster_level(
            &manager,
            owner,
            missing_guid,
            SPELL_ID,
            0,
            Some(&empty_levels),
            None,
        ),
        Ok(0),
    );
    assert_eq!(
        selected_caster_level(&manager, owner, missing_guid, SPELL_ID, 0, None, None),
        Err(SelectedCasterLevelError::SpellLevelsUnavailable),
    );
    assert_eq!(
        selected_caster_level(
            &manager,
            owner,
            ObjectGuid::EMPTY,
            SPELL_ID,
            0,
            Some(&empty_levels),
            None,
        ),
        Ok(0),
    );
}

#[test]
fn difficulty_fallback_uses_highest_record_id_at_each_exact_difficulty() {
    let mut manager = manager_with_maps(&[MAP]);
    let owner_guid = ObjectGuid::create_player(1, 91_012);
    let missing_guid = ObjectGuid::create_player(1, 91_013);
    let owner = add_player(&mut manager, owner_guid, 70, Some(MAP));
    let levels = spell_levels([
        level_entry(10, SPELL_ID, 1, 13),
        level_entry(20, SPELL_ID, 1, 37),
        level_entry(30, SPELL_ID, 0, 9),
    ]);
    let difficulties = DifficultyStore::from_entries([
        difficulty_entry(3, 2),
        difficulty_entry(2, 1),
        difficulty_entry(1, 0),
    ]);

    assert_eq!(
        selected_caster_level(
            &manager,
            owner,
            missing_guid,
            SPELL_ID,
            3,
            Some(&levels),
            Some(&difficulties),
        ),
        Ok(37),
    );
}

#[test]
fn missing_difficulty_row_stops_fallback_and_missing_store_is_unavailable() {
    let mut manager = manager_with_maps(&[MAP]);
    let owner_guid = ObjectGuid::create_player(1, 91_014);
    let missing_guid = ObjectGuid::create_player(1, 91_015);
    let owner = add_player(&mut manager, owner_guid, 70, Some(MAP));
    let levels = spell_levels([level_entry(1, SPELL_ID, 0, 88)]);
    let no_difficulty_row = DifficultyStore::from_entries([difficulty_entry(1, 0)]);

    assert_eq!(
        selected_caster_level(
            &manager,
            owner,
            missing_guid,
            SPELL_ID,
            3,
            Some(&levels),
            Some(&no_difficulty_row),
        ),
        Ok(0),
    );
    assert_eq!(
        selected_caster_level(
            &manager,
            owner,
            missing_guid,
            SPELL_ID,
            3,
            Some(&levels),
            None,
        ),
        Err(SelectedCasterLevelError::DifficultyUnavailable),
    );
}

#[test]
fn stale_owner_generation_is_rejected_before_self_caster_fast_path() {
    let mut manager = MapManager::default();
    let owner_guid = ObjectGuid::create_player(1, 91_016);
    let stale_owner = add_player(&mut manager, owner_guid, 70, None);
    add_player(&mut manager, owner_guid, 80, None);

    assert_eq!(
        selected_caster_level(&manager, stale_owner, owner_guid, SPELL_ID, 0, None, None),
        Err(SelectedCasterLevelError::OwnerUnavailable),
    );
}

#[test]
fn difficulty_cycle_is_reported_without_looping() {
    let mut manager = manager_with_maps(&[MAP]);
    let owner_guid = ObjectGuid::create_player(1, 91_017);
    let missing_guid = ObjectGuid::create_player(1, 91_018);
    let owner = add_player(&mut manager, owner_guid, 70, Some(MAP));
    let levels = spell_levels([level_entry(1, SPELL_ID, 7, 88)]);
    let difficulties =
        DifficultyStore::from_entries([difficulty_entry(0, 3), difficulty_entry(3, 0)]);

    assert_eq!(
        selected_caster_level(
            &manager,
            owner,
            missing_guid,
            SPELL_ID,
            0,
            Some(&levels),
            Some(&difficulties),
        ),
        Err(SelectedCasterLevelError::DifficultyCycle),
    );
}
