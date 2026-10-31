use super::selected_visual::selected_aura_spell_visual_id;
use wow_core::{ObjectGuid, Position};
use wow_data::{
    ConditionPresenceCatalog, DifficultyEntry, DifficultyStore, SpellInfo, SpellStore,
    spell::{SpellVisualCasterKind, SpellVisualSelectionError},
    spell_db2::{SpellXSpellVisualEntry, SpellXSpellVisualStore},
};
use wow_entities::{Creature, MapObjectRecord, Player};
use wow_map::{MapKey, MapManager, PlayerHandle};

const MAP: MapKey = MapKey::new(571, 0);
const SPELL_ID: u32 = 20_584;

fn manager() -> MapManager {
    let mut manager = MapManager::default();
    manager.create_world_map(MAP.map_id, MAP.instance_id);
    manager
}

fn add_player(manager: &mut MapManager, guid: ObjectGuid, level: u8, active: bool) -> PlayerHandle {
    let mut player = Box::new(Player::new(Some(guid.counter() as u64), false));
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().set_level(level);
    let handle = manager.install_detached_player_like_cpp(player).unwrap();
    if active {
        manager
            .attach_player_like_cpp(handle, MAP, Position::ZERO)
            .unwrap();
    }
    handle
}

fn add_creature(
    manager: &mut MapManager,
    guid: ObjectGuid,
    owner_guid: ObjectGuid,
    charmer_guid: ObjectGuid,
) {
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .set_map(MAP.map_id, MAP.instance_id)
        .unwrap();
    creature.unit_mut().world_mut().relocate(Position::ZERO);
    creature.unit_mut().world_mut().object_mut().add_to_world();
    creature.unit_mut().subsystems_mut().control.owner_guid = Some(owner_guid);
    creature.unit_mut().subsystems_mut().control.charmer_guid = Some(charmer_guid);
    manager
        .find_map_mut(MAP.map_id, MAP.instance_id)
        .unwrap()
        .map_mut()
        .insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}

fn visual_row(
    id: u32,
    difficulty_id: u8,
    caster_player_condition_id: u32,
    caster_unit_condition_id: u16,
) -> SpellXSpellVisualEntry {
    SpellXSpellVisualEntry {
        id,
        difficulty_id,
        spell_visual_id: id.wrapping_add(100),
        probability: 1.0,
        flags: 0,
        priority: 0,
        spell_icon_file_id: 0,
        active_icon_file_id: 0,
        viewer_unit_condition_id: 0,
        viewer_player_condition_id: 0,
        caster_unit_condition_id,
        caster_player_condition_id,
        spell_id: SPELL_ID,
    }
}

fn select(
    manager: &MapManager,
    owner: PlayerHandle,
    caster_guid: ObjectGuid,
    regular_spells: Option<&SpellStore>,
    serverside_spells: Option<&wow_data::ServersideSpellStoreLikeCpp>,
    visuals: Option<&SpellXSpellVisualStore>,
    presence: Option<&ConditionPresenceCatalog>,
    difficulties: Option<&DifficultyStore>,
) -> Result<u32, &'static str> {
    selected_aura_spell_visual_id(
        manager,
        owner,
        caster_guid,
        SPELL_ID,
        0,
        regular_spells,
        serverside_spells,
        visuals,
        presence,
        difficulties,
    )
}

#[test]
fn selected_visual_uses_physical_caster_kind_for_player_and_controlled_creature() {
    let mut manager = manager();
    let owner_guid = ObjectGuid::create_player(1, 91_100);
    let owner = add_player(&mut manager, owner_guid, 70, true);
    let foreign_guid = ObjectGuid::create_player(1, 91_101);
    add_player(&mut manager, foreign_guid, 51, true);
    let presence = ConditionPresenceCatalog::from_effective_ids([9], []);
    let visuals =
        SpellXSpellVisualStore::from_entries([visual_row(101, 0, 9, 0), visual_row(100, 0, 0, 0)]);
    let regular = SpellStore::new();
    let serverside = wow_data::ServersideSpellStoreLikeCpp::default();

    assert_eq!(
        select(
            &manager,
            owner,
            foreign_guid,
            Some(&regular),
            Some(&serverside),
            Some(&visuals),
            Some(&presence),
            None,
        ),
        Err("Selected aura player condition evaluation is unavailable"),
        "a physical Player caster keeps Player condition semantics",
    );

    let creature_guid = ObjectGuid::create_creature_like_cpp(1, MAP.map_id as u16, 91_102, 1);
    add_creature(&mut manager, creature_guid, owner_guid, owner_guid);
    assert_eq!(
        select(
            &manager,
            owner,
            creature_guid,
            Some(&regular),
            Some(&serverside),
            Some(&visuals),
            Some(&presence),
            None,
        ),
        Ok(100),
        "ownership and charm do not turn the physical Creature into a Player",
    );
}

#[test]
fn selected_visual_accepts_known_absent_nonzero_condition_ids_and_requires_present_zero_ids() {
    let mut manager = manager();
    let owner_guid = ObjectGuid::create_player(1, 91_110);
    let owner = add_player(&mut manager, owner_guid, 70, true);
    let regular = SpellStore::new();
    let serverside = wow_data::ServersideSpellStoreLikeCpp::default();
    let visuals = SpellXSpellVisualStore::from_entries([visual_row(77, 0, 12, 0)]);
    let known_absent = ConditionPresenceCatalog::from_effective_ids([], []);

    assert_eq!(
        select(
            &manager,
            owner,
            owner_guid,
            Some(&regular),
            Some(&serverside),
            Some(&visuals),
            Some(&known_absent),
            None,
        ),
        Ok(77),
        "a nonzero ID absent from the complete effective catalog is not an active condition",
    );

    let zero_is_present = ConditionPresenceCatalog::from_effective_ids([0], []);
    let zero_visuals = SpellXSpellVisualStore::from_entries([visual_row(78, 0, 0, 0)]);
    assert_eq!(
        wow_data::spell::select_spell_visual_id(
            &zero_visuals.entries_like_cpp().collect::<Vec<_>>(),
            Some(&zero_is_present),
            SpellVisualCasterKind::Player,
        ),
        Err(SpellVisualSelectionError::PlayerConditionEvaluationUnavailable { condition_id: 0 }),
    );
    assert_eq!(
        select(
            &manager,
            owner,
            owner_guid,
            Some(&regular),
            Some(&serverside),
            Some(&zero_visuals),
            Some(&zero_is_present),
            None,
        ),
        Err("Selected aura player condition evaluation is unavailable"),
    );
}

#[test]
fn selected_visual_no_caster_skips_override_catalogs_but_keeps_known_condition_selection() {
    let mut manager = manager();
    let owner_guid = ObjectGuid::create_player(1, 91_120);
    let owner = add_player(&mut manager, owner_guid, 70, false);
    let presence = ConditionPresenceCatalog::from_effective_ids([4], [6]);
    let visuals =
        SpellXSpellVisualStore::from_entries([visual_row(90, 0, 4, 6), visual_row(89, 0, 0, 0)]);

    assert_eq!(
        select(
            &manager,
            owner,
            ObjectGuid::EMPTY,
            None,
            None,
            Some(&visuals),
            Some(&presence),
            None,
        ),
        Ok(89),
        "NoCaster skips known conditions and does not require override-query catalogs",
    );
}

#[test]
fn selected_visual_requires_both_override_catalogs_for_a_resolved_unit() {
    let mut manager = manager();
    let owner_guid = ObjectGuid::create_player(1, 91_125);
    let owner = add_player(&mut manager, owner_guid, 70, true);
    let visuals = SpellXSpellVisualStore::from_entries([]);
    let presence = ConditionPresenceCatalog::from_effective_ids([], []);
    let regular = SpellStore::new();

    assert_eq!(
        select(
            &manager,
            owner,
            owner_guid,
            Some(&regular),
            None,
            Some(&visuals),
            Some(&presence),
            None,
        ),
        Err("Missing serverside spell catalog for selected aura visual override check"),
    );
}

#[test]
fn selected_visual_distinguishes_known_empty_rows_from_unavailable_store() {
    let mut manager = manager();
    let owner_guid = ObjectGuid::create_player(1, 91_130);
    let owner = add_player(&mut manager, owner_guid, 70, false);
    let empty = SpellXSpellVisualStore::from_entries([]);

    assert_eq!(
        select(
            &manager,
            owner,
            ObjectGuid::EMPTY,
            None,
            None,
            Some(&empty),
            None,
            None,
        ),
        Ok(0),
        "a loaded empty Wisp visual store represents a known no-row result",
    );
    assert_eq!(
        select(
            &manager,
            owner,
            ObjectGuid::EMPTY,
            None,
            None,
            None,
            None,
            None,
        ),
        Err("Missing SpellXSpellVisual store for selected aura creation"),
    );
}

#[test]
fn selected_visual_rejects_regular_aura_403_potential_without_guessing_runtime_order() {
    let mut manager = manager();
    let owner_guid = ObjectGuid::create_player(1, 91_140);
    let owner = add_player(&mut manager, owner_guid, 70, true);
    let mut regular = SpellStore::new();
    regular.insert(
        91_141,
        SpellInfo {
            spell_id: 91_141,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(403),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: 403,
                effect_misc_value_1: SPELL_ID as i32,
                ..Default::default()
            }],
        },
    );
    let serverside = wow_data::ServersideSpellStoreLikeCpp::default();
    let visuals = SpellXSpellVisualStore::from_entries([]);
    let presence = ConditionPresenceCatalog::from_effective_ids([], []);

    assert_eq!(
        select(
            &manager,
            owner,
            owner_guid,
            Some(&regular),
            Some(&serverside),
            Some(&visuals),
            Some(&presence),
            None,
        ),
        Err("Selected aura visual override source/order is unavailable"),
    );
}

#[test]
fn selected_visual_resolves_same_map_foreign_player_from_detached_assigned_owner() {
    let mut manager = manager();
    let owner_guid = ObjectGuid::create_player(1, 91_150);
    let owner = add_player(&mut manager, owner_guid, 70, false);
    manager
        .with_player_mut_like_cpp(owner, |player| {
            player
                .unit_mut()
                .world_mut()
                .set_map(MAP.map_id, MAP.instance_id)
                .unwrap();
        })
        .unwrap();
    let caster_guid = ObjectGuid::create_player(1, 91_151);
    add_player(&mut manager, caster_guid, 51, true);
    let regular = SpellStore::new();
    let serverside = wow_data::ServersideSpellStoreLikeCpp::default();
    let visuals = SpellXSpellVisualStore::from_entries([visual_row(91_152, 0, 7, 0)]);
    let presence = ConditionPresenceCatalog::from_effective_ids([7], []);

    assert_eq!(
        select(
            &manager,
            owner,
            caster_guid,
            Some(&regular),
            Some(&serverside),
            Some(&visuals),
            Some(&presence),
            None,
        ),
        Err("Selected aura player condition evaluation is unavailable"),
        "the assigned target Map resolves its physically Player caster",
    );
}

#[test]
fn selected_visual_uses_saved_difficulty_vector_priority() {
    let mut manager = manager();
    let owner_guid = ObjectGuid::create_player(1, 91_160);
    let owner = add_player(&mut manager, owner_guid, 70, true);
    let regular = SpellStore::new();
    let serverside = wow_data::ServersideSpellStoreLikeCpp::default();
    let visuals =
        SpellXSpellVisualStore::from_entries([visual_row(31, 3, 12, 0), visual_row(30, 3, 0, 0)]);
    let presence = ConditionPresenceCatalog::from_effective_ids([], []);
    let difficulties = DifficultyStore::from_entries([
        DifficultyEntry {
            id: 0,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 3,
            toggle_difficulty_id: 0,
        },
        DifficultyEntry {
            id: 3,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
    ]);

    assert_eq!(
        selected_aura_spell_visual_id(
            &manager,
            owner,
            owner_guid,
            SPELL_ID,
            0,
            Some(&regular),
            Some(&serverside),
            Some(&visuals),
            Some(&presence),
            Some(&difficulties),
        ),
        Ok(31),
        "the raw difficulty-zero request follows its fallback and uses sorted row priority",
    );
}
