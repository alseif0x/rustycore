use super::*;
use crate::aura_session::SelectedAuraFlagsError;
use wow_constants::{
    creature::CreatureStaticFlags4,
    unit::{UnitFlags, UnitFlags2, UnitPvpFlags},
};
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_data::progression_rewards::{
    FACTION_TEMPLATE_FLAG_CONTESTED_GUARD_LIKE_CPP, FactionEntry, FactionStore,
    FactionTemplateEntry, FactionTemplateStore, FriendshipRepReactionStore,
};
use wow_data::reputation::ReputationRankLikeCpp;
use wow_entities::{Creature, MapObjectRecord, Pet, PetType, Player, PlayerGroupState};
use wow_map::{MapKey, MapManager, PlayerHandle};

const MAP: MapKey = MapKey::new(571, 0);
const OTHER_MAP: MapKey = MapKey::new(530, 0);

fn manager_with_maps(keys: &[MapKey]) -> MapManager {
    let mut manager = MapManager::default();
    for key in keys {
        manager.create_world_map(key.map_id, key.instance_id);
    }
    manager
}

fn add_player(
    manager: &mut MapManager,
    counter: i64,
    map: MapKey,
    faction_template: u32,
    controlled: bool,
) -> (ObjectGuid, PlayerHandle) {
    let guid = ObjectGuid::create_player(1, counter);
    let mut player = Box::new(Player::new(Some(counter as u64), false));
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().set_level(44);
    player.unit_mut().set_faction(faction_template);
    player.unit_mut().set_unit_flags_like_cpp(if controlled {
        UnitFlags::PLAYER_CONTROLLED
    } else {
        UnitFlags::empty()
    });
    let handle = manager.install_detached_player_like_cpp(player).unwrap();
    manager
        .attach_player_like_cpp(handle, map, Position::ZERO)
        .unwrap();
    (guid, handle)
}

#[allow(clippy::too_many_arguments)]
fn add_creature(
    manager: &mut MapManager,
    map: MapKey,
    counter: i64,
    faction_template: u32,
    owner: Option<ObjectGuid>,
    charmer: Option<ObjectGuid>,
    controlled: bool,
    summon: bool,
    static_flags: [u32; 8],
) -> ObjectGuid {
    let guid =
        ObjectGuid::create_creature_like_cpp(1, u16::try_from(map.map_id).unwrap(), 91, counter);
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .set_map(map.map_id, map.instance_id)
        .unwrap();
    creature.unit_mut().set_level(44);
    creature.unit_mut().set_faction(faction_template);
    creature.unit_mut().set_unit_flags_like_cpp(if controlled {
        UnitFlags::PLAYER_CONTROLLED
    } else {
        UnitFlags::empty()
    });
    creature.unit_mut().subsystems_mut().control.owner_guid = owner;
    creature.unit_mut().subsystems_mut().control.charmer_guid = charmer;
    creature.set_summon_like_cpp(summon);
    creature.set_static_flags_runtime_like_cpp(static_flags);
    creature.unit_mut().world_mut().object_mut().add_to_world();
    manager
        .find_map_mut(map.map_id, map.instance_id)
        .unwrap()
        .map_mut()
        .insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    guid
}

fn add_pet(manager: &mut MapManager, map: MapKey, counter: i64, owner: ObjectGuid) -> ObjectGuid {
    let guid = ObjectGuid::create_world_object(
        HighGuid::Pet,
        0,
        1,
        u16::try_from(map.map_id).unwrap(),
        0,
        91,
        counter,
    );
    let mut pet = Pet::new(owner, PetType::Summon);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(guid);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .set_map(map.map_id, map.instance_id)
        .unwrap();
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    manager
        .find_map_mut(map.map_id, map.instance_id)
        .unwrap()
        .map_mut()
        .insert_map_object_record(MapObjectRecord::new_pet(pet).unwrap())
        .unwrap();
    guid
}

fn faction_template(id: u32, faction: u16, flags: u16) -> FactionTemplateEntry {
    FactionTemplateEntry {
        id,
        faction,
        flags,
        faction_group: 0,
        friend_group: 0,
        enemy_group: 0,
        enemies: [0; 8],
        friend: [0; 8],
    }
}

fn faction_templates() -> FactionTemplateStore {
    FactionTemplateStore::from_entries([faction_template(1, 100, 0), faction_template(2, 200, 0)])
}

fn reaction(
    manager: &MapManager,
    owner: PlayerHandle,
    caster: ObjectGuid,
    templates: Option<&FactionTemplateStore>,
    factions: Option<&FactionStore>,
    friendship: Option<&FriendshipRepReactionStore>,
) -> Result<bool, SelectedCasterLevelError> {
    selected_caster_is_friendly(manager, owner, caster, factions, templates, friendship)
}

#[test]
fn selected_creation_flags_resolve_uniform_and_self_masks_without_relation_data() {
    let polarity = crate::aura_session::SelectedSpellPolarity {
        available_effect_mask: 0b111,
        negative_effect_mask: 0b010,
        positive_effect_mask: 0b101,
        whole_spell_positive: false,
    };
    let caster = ObjectGuid::create_player(1, 92_001);
    let target = ObjectGuid::create_player(1, 92_002);
    let attributes = [0; 15];

    assert_eq!(
        crate::aura_session::build_selected_aura_application_flags(
            polarity,
            0b101,
            caster,
            target,
            target,
            &attributes,
            None,
        ),
        Ok(0x0102),
    );
    assert_eq!(
        crate::aura_session::build_selected_aura_application_flags(
            polarity,
            0b010,
            caster,
            target,
            target,
            &attributes,
            None,
        ),
        Ok(0x0010),
    );
    assert_eq!(
        crate::aura_session::build_selected_aura_application_flags(
            polarity,
            0b111,
            target,
            target,
            target,
            &attributes,
            None,
        ),
        Ok(0x0011),
    );
    assert_eq!(
        crate::aura_session::build_selected_aura_application_flags(
            polarity,
            0b111,
            caster,
            target,
            target,
            &attributes,
            None,
        ),
        Err(SelectedAuraFlagsError::UnknownForeignCasterRelation),
    );
}

#[test]
fn absent_out_of_world_and_other_map_players_are_known_unfriendly_sources() {
    let mut manager = manager_with_maps(&[MAP, OTHER_MAP]);
    let (_, owner) = add_player(&mut manager, 92_010, MAP, 1, false);
    let (other_guid, _) = add_player(&mut manager, 92_011, OTHER_MAP, 1, false);
    let (out_guid, out_handle) = add_player(&mut manager, 92_012, MAP, 1, false);
    manager
        .with_player_mut_like_cpp(out_handle, |player| {
            player
                .unit_mut()
                .world_mut()
                .object_mut()
                .remove_from_world();
            player.unit_mut().world_mut().reset_map().unwrap();
        })
        .unwrap();
    let missing_guid = ObjectGuid::create_player(1, 92_013);

    for caster in [other_guid, out_guid, missing_guid] {
        assert_eq!(
            reaction(&manager, owner, caster, None, None, None),
            Ok(false)
        );
    }
}

#[test]
fn in_world_player_map_inconsistency_is_an_error() {
    let mut manager = manager_with_maps(&[MAP, OTHER_MAP]);
    let (_, owner) = add_player(&mut manager, 92_020, MAP, 1, false);
    let (caster_guid, caster) = add_player(&mut manager, 92_021, MAP, 1, false);
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
        reaction(&manager, owner, caster_guid, None, None, None),
        Err(SelectedCasterLevelError::CasterUnavailable),
    );
}

#[test]
fn source_and_target_affecting_players_supply_their_own_forced_ranks() {
    let mut manager = manager_with_maps(&[MAP]);
    let (_, target) = add_player(&mut manager, 92_030, MAP, 2, false);
    let (_, source_owner) = add_player(&mut manager, 92_031, MAP, 1, false);
    manager
        .with_player_mut_like_cpp(source_owner, |player| {
            player
                .reputation_mut_like_cpp()
                .set_forced_reaction_like_cpp(200, Some(ReputationRankLikeCpp::Friendly));
        })
        .unwrap();
    manager
        .with_player_mut_like_cpp(target, |player| {
            player
                .reputation_mut_like_cpp()
                .set_forced_reaction_like_cpp(200, Some(ReputationRankLikeCpp::Hostile));
        })
        .unwrap();
    let caster = add_creature(
        &mut manager,
        MAP,
        92_032,
        1,
        Some(source_owner.guid()),
        None,
        false,
        false,
        [0; 8],
    );
    let templates = faction_templates();

    assert_eq!(
        reaction(&manager, target, caster, Some(&templates), None, None),
        Ok(true)
    );

    let unowned_caster = add_creature(
        &mut manager,
        MAP,
        92_033,
        1,
        None,
        None,
        false,
        false,
        [0; 8],
    );
    manager
        .with_player_mut_like_cpp(target, |player| {
            player
                .reputation_mut_like_cpp()
                .set_forced_reaction_like_cpp(100, Some(ReputationRankLikeCpp::Hostile));
        })
        .unwrap();
    assert_eq!(
        reaction(
            &manager,
            target,
            unowned_caster,
            Some(&templates),
            None,
            None
        ),
        Ok(false),
    );
}

#[test]
fn pet_owner_fallback_charmer_priority_and_conflicts_follow_canonical_pet_data() {
    let mut manager = manager_with_maps(&[MAP]);
    let (pet_owner_guid, pet_owner) = add_player(&mut manager, 92_040, MAP, 1, false);
    let (charmer_guid, charmer) = add_player(&mut manager, 92_041, MAP, 1, false);
    let (other_guid, _) = add_player(&mut manager, 92_042, MAP, 1, false);
    let pet_guid = add_pet(&mut manager, MAP, 92_043, pet_owner_guid);

    assert_eq!(
        reaction(&manager, pet_owner, pet_guid, None, None, None),
        Ok(true)
    );
    manager
        .find_map_mut(MAP.map_id, MAP.instance_id)
        .unwrap()
        .map_mut()
        .get_typed_pet_mut(pet_guid)
        .unwrap()
        .creature_mut()
        .unit_mut()
        .subsystems_mut()
        .control
        .charmer_guid = Some(charmer_guid);
    assert_eq!(
        reaction(&manager, charmer, pet_guid, None, None, None),
        Ok(true)
    );

    manager
        .find_map_mut(MAP.map_id, MAP.instance_id)
        .unwrap()
        .map_mut()
        .get_typed_pet_mut(pet_guid)
        .unwrap()
        .creature_mut()
        .unit_mut()
        .subsystems_mut()
        .control
        .owner_guid = Some(other_guid);
    assert_eq!(
        reaction(&manager, charmer, pet_guid, None, None, None),
        Ok(true)
    );
    manager
        .find_map_mut(MAP.map_id, MAP.instance_id)
        .unwrap()
        .map_mut()
        .get_typed_pet_mut(pet_guid)
        .unwrap()
        .creature_mut()
        .unit_mut()
        .subsystems_mut()
        .control
        .charmer_guid = None;
    assert_eq!(
        reaction(&manager, charmer, pet_guid, None, None, None),
        Err(SelectedCasterLevelError::CasterUnavailable),
    );
}

#[test]
fn affecting_player_resolution_stops_after_the_bounded_second_owner() {
    let mut manager = manager_with_maps(&[MAP]);
    let (_, target) = add_player(&mut manager, 92_050, MAP, 2, false);
    let (_, first_owner) = add_player(&mut manager, 92_051, MAP, 1, false);
    let (_, affecting_player) = add_player(&mut manager, 92_052, MAP, 1, false);
    let (_, recursive_owner) = add_player(&mut manager, 92_053, MAP, 1, false);
    manager
        .with_player_mut_like_cpp(first_owner, |player| {
            player.unit_mut().subsystems_mut().control.owner_guid = Some(affecting_player.guid());
        })
        .unwrap();
    manager
        .with_player_mut_like_cpp(affecting_player, |player| {
            player.unit_mut().subsystems_mut().control.owner_guid = Some(recursive_owner.guid());
            player
                .reputation_mut_like_cpp()
                .set_forced_reaction_like_cpp(200, Some(ReputationRankLikeCpp::Friendly));
        })
        .unwrap();
    manager
        .with_player_mut_like_cpp(recursive_owner, |player| {
            player
                .reputation_mut_like_cpp()
                .set_forced_reaction_like_cpp(200, Some(ReputationRankLikeCpp::Hostile));
        })
        .unwrap();
    let caster = add_creature(
        &mut manager,
        MAP,
        92_054,
        1,
        Some(first_owner.guid()),
        None,
        false,
        false,
        [0; 8],
    );
    let templates = faction_templates();

    assert_eq!(
        reaction(&manager, target, caster, Some(&templates), None, None),
        Ok(true)
    );
}

#[test]
fn controlled_duel_and_current_group_precede_faction_fallbacks() {
    let mut manager = manager_with_maps(&[MAP]);
    let (target_guid, target) = add_player(&mut manager, 92_060, MAP, 0, true);
    let (source_guid, source) = add_player(&mut manager, 92_061, MAP, 0, true);
    let known_empty_templates =
        FactionTemplateStore::from_entries(Vec::<FactionTemplateEntry>::new());
    manager
        .with_player_mut_like_cpp(source, |player| {
            player.set_duel_opponent_in_progress_like_cpp(target_guid);
        })
        .unwrap();

    assert_eq!(
        reaction(
            &manager,
            target,
            source_guid,
            Some(&known_empty_templates),
            None,
            None
        ),
        Ok(false),
    );
    let group_guid = ObjectGuid::create_group(92_062);
    for handle in [source, target] {
        manager
            .with_player_mut_like_cpp(handle, |player| {
                player.clear_duel_like_cpp();
                player.set_group_like_cpp(Some(PlayerGroupState {
                    group_guid,
                    leader_guid: target_guid,
                    role_mask: 0,
                    subgroup: 0,
                }));
            })
            .unwrap();
    }
    assert_eq!(
        reaction(
            &manager,
            target,
            source_guid,
            Some(&known_empty_templates),
            None,
            None
        ),
        Ok(true),
    );
}

#[test]
fn inherited_raid_wrapper_uses_raw_creature_faction_template_values() {
    let player = Player::new(Some(92_070), false);
    let mut raid_creature = Creature::new(false);
    let mut flags = [0; 8];
    flags[3] = CreatureStaticFlags4::TREAT_AS_RAID_UNIT_FOR_HELPFUL_SPELLS.bits();
    raid_creature.set_static_flags_runtime_like_cpp(flags);
    assert_eq!(
        inherited_raid_units(
            SelectedUnit::Player(&player),
            SelectedUnit::Creature {
                body: &raid_creature,
                pet_owner_guid: None,
            },
        ),
        Ok(true),
    );

    flags[3] = 0;
    raid_creature.set_static_flags_runtime_like_cpp(flags);
    let mut first = Creature::new(false);
    let mut second = Creature::new(false);
    first.unit_mut().set_faction(11);
    second.unit_mut().set_faction(11);
    assert_eq!(
        inherited_raid_units(
            SelectedUnit::Creature {
                body: &first,
                pet_owner_guid: None,
            },
            SelectedUnit::Creature {
                body: &second,
                pet_owner_guid: None,
            },
        ),
        Ok(true),
    );
    second.unit_mut().set_faction(12);
    let templates_same_underlying_faction = FactionTemplateStore::from_entries([
        faction_template(11, 100, 0),
        faction_template(12, 100, 0),
    ]);
    assert_eq!(
        templates_same_underlying_faction.get(11).unwrap().faction,
        templates_same_underlying_faction.get(12).unwrap().faction,
    );
    assert_eq!(
        inherited_raid_units(
            SelectedUnit::Creature {
                body: &first,
                pet_owner_guid: None,
            },
            SelectedUnit::Creature {
                body: &second,
                pet_owner_guid: None,
            },
        ),
        Ok(false),
    );

    let mut manager = manager_with_maps(&[MAP]);
    let (_, source_player) = add_player(&mut manager, 92_071, MAP, 1, false);
    let (_, target_player) = add_player(&mut manager, 92_072, MAP, 1, false);
    let shared_owner = add_creature(
        &mut manager,
        MAP,
        92_073,
        1,
        None,
        None,
        false,
        false,
        [0; 8],
    );
    for handle in [source_player, target_player] {
        manager
            .with_player_mut_like_cpp(handle, |player| {
                player.unit_mut().subsystems_mut().control.owner_guid = Some(shared_owner);
            })
            .unwrap();
    }
    let shared_owner_relation = manager
        .with_player_like_cpp(source_player, |source| {
            manager.with_player_like_cpp(target_player, |target| {
                inherited_raid_relation(&manager, MAP, source, target)
            })
        })
        .flatten()
        .expect("both affecting Players resolve under the map guard");
    assert_eq!(shared_owner_relation, Ok(true));
}

#[test]
fn source_and_target_contested_reputation_flags_remain_distinct() {
    let mut source_unit = Creature::new(false);
    let mut target_unit = Player::new(Some(92_080), false);
    let mut source_player = Player::new(Some(92_081), false);
    let mut target_player = Player::new(Some(92_082), false);
    source_unit
        .unit_mut()
        .set_pvp_flag_like_cpp(UnitPvpFlags::FFA_PVP);
    target_unit
        .unit_mut()
        .set_pvp_flag_like_cpp(UnitPvpFlags::FFA_PVP);
    source_player.set_player_flag(0x0000_0100);
    target_unit
        .unit_mut()
        .set_unit_flags2_like_cpp(UnitFlags2::IGNORE_REPUTATION);
    let source = SelectedUnit::Creature {
        body: &source_unit,
        pet_owner_guid: None,
    };
    let target = SelectedUnit::Player(&target_unit);
    let input = reaction_input(
        source,
        target,
        false,
        true,
        true,
        false,
        false,
        false,
        false,
        Some(&source_player),
        Some(&target_player),
    );
    assert!(input.self_player_contested_pvp);
    assert!(!input.target_player_contested_pvp);
    assert!(input.target_ignores_reputation);
    assert!(input.self_ffa_pvp);
    assert!(input.target_ffa_pvp);

    target_player.set_player_flag(0x0000_0100);
    let input = reaction_input(
        source,
        target,
        false,
        true,
        true,
        false,
        false,
        false,
        false,
        Some(&source_player),
        Some(&target_player),
    );
    assert!(input.self_player_contested_pvp);
    assert!(input.target_player_contested_pvp);

    let mut manager = manager_with_maps(&[MAP]);
    let (_, target) = add_player(&mut manager, 92_083, MAP, 2, true);
    let (source_owner_guid, source_owner) = add_player(&mut manager, 92_084, MAP, 1, false);
    manager
        .with_player_mut_like_cpp(source_owner, |player| {
            player.set_player_flag(0x0000_0100);
        })
        .unwrap();
    let source = add_creature(
        &mut manager,
        MAP,
        92_085,
        1,
        Some(source_owner_guid),
        None,
        true,
        false,
        [0; 8],
    );
    let source_templates = FactionTemplateStore::from_entries([
        faction_template(1, 100, 0),
        faction_template(2, 200, FACTION_TEMPLATE_FLAG_CONTESTED_GUARD_LIKE_CPP),
    ]);
    let factions = FactionStore::from_entries([FactionEntry::for_test_like_cpp(200, 2)]);
    assert_eq!(
        reaction(
            &manager,
            target,
            source,
            Some(&source_templates),
            Some(&factions),
            None,
        ),
        Ok(false),
        "the source affecting Player's contested flag controls this branch",
    );

    let (_, contested_target) = add_player(&mut manager, 92_086, MAP, 2, true);
    manager
        .with_player_mut_like_cpp(contested_target, |player| {
            player.set_player_flag(0x0000_0100);
        })
        .unwrap();
    let target_templates = FactionTemplateStore::from_entries([
        faction_template(1, 100, FACTION_TEMPLATE_FLAG_CONTESTED_GUARD_LIKE_CPP),
        faction_template(2, 200, 0),
    ]);
    let unowned_source = add_creature(
        &mut manager,
        MAP,
        92_087,
        1,
        None,
        None,
        false,
        false,
        [0; 8],
    );
    assert_eq!(
        reaction(
            &manager,
            contested_target,
            unowned_source,
            Some(&target_templates),
            None,
            None,
        ),
        Ok(false),
        "the target affecting Player's contested flag controls this branch",
    );
}

#[test]
fn required_catalog_and_unrepresented_summon_gaps_hold_but_known_rows_are_neutral() {
    let mut manager = manager_with_maps(&[MAP]);
    let (_, target) = add_player(&mut manager, 92_090, MAP, 2, false);
    let caster = add_creature(
        &mut manager,
        MAP,
        92_091,
        1,
        None,
        None,
        false,
        false,
        [0; 8],
    );
    assert_eq!(
        reaction(&manager, target, caster, None, None, None),
        Err(SelectedCasterLevelError::ReactionUnavailable),
    );
    let empty_templates = FactionTemplateStore::from_entries(Vec::<FactionTemplateEntry>::new());
    assert_eq!(
        reaction(&manager, target, caster, Some(&empty_templates), None, None),
        Ok(false)
    );

    let templates = faction_templates();
    assert_eq!(
        reaction(&manager, target, caster, Some(&templates), None, None),
        Err(SelectedCasterLevelError::ReactionUnavailable),
    );
    let mut faction = FactionEntry::for_test_like_cpp(100, 1);
    faction.friendship_rep_id = 7;
    let factions = FactionStore::from_entries([faction]);
    assert_eq!(
        reaction(
            &manager,
            target,
            caster,
            Some(&templates),
            Some(&factions),
            None
        ),
        Err(SelectedCasterLevelError::ReactionUnavailable),
    );
    let empty_friendships = FriendshipRepReactionStore::from_entries(Vec::new());
    assert_eq!(
        reaction(
            &manager,
            target,
            caster,
            Some(&templates),
            Some(&factions),
            Some(&empty_friendships),
        ),
        Ok(false),
    );

    let summon = add_creature(
        &mut manager,
        MAP,
        92_092,
        1,
        None,
        None,
        false,
        true,
        [0; 8],
    );
    assert_eq!(
        reaction(
            &manager,
            target,
            summon,
            Some(&templates),
            Some(&factions),
            None
        ),
        Err(SelectedCasterLevelError::ReactionUnavailable),
    );
}
