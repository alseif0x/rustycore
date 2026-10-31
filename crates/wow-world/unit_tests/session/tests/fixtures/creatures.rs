//! Creature, map, and reputation fixtures.
//!
//! These builders retain the original session-test behavior and are
//! visible only within the parent `session::tests` subtree.

use super::*;

pub(in crate::session::tests) fn canonical_player_transfer_test_map_store_like_cpp()
-> Arc<wow_data::MapStore> {
    Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ]))
}

/// F6-7 R7a: a session that owns a canonical map manager also owns the canonical
/// map instance at its runtime map key, so a creature registration and an
/// owner mutation can be admitted against the canonical incarnation. Fixtures
/// that inject a canonical manager must not leave that map instance absent.
pub(in crate::session::tests) fn ensure_canonical_test_map_for_session_like_cpp(
    session: &mut WorldSession,
) {
    let Some(canonical) = session.core.canonical_map_manager.clone() else {
        return;
    };
    let (map_id, instance_id) = session.core.current_runtime_map_key_like_cpp();
    let mut guard = canonical
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if guard.find_map(u32::from(map_id), instance_id).is_none() {
        guard.create_world_map(u32::from(map_id), instance_id);
    }
}

/// Register one test creature through the session's canonical registration
/// path. `manager` becomes the session's canonical creature store unless the
/// session already has one.
pub(in crate::session::tests) fn register_test_creature(
    session: &mut WorldSession,
    manager: SharedCanonicalMapManager,
    guid: ObjectGuid,
    hp: u32,
) {
    if session.core.canonical_map_manager.is_none() {
        session.set_canonical_map_manager(manager);
    }
    session.core.current_map_id = 0;
    if session.player_position_like_cpp().is_none() {
        session.set_player_map_position_like_cpp(0, Position::new(10.0, 10.0, 0.0, 0.0));
    }
    ensure_canonical_test_map_for_session_like_cpp(session);
    session.register_world_creature(
        0,
        Position::new(10.0, 10.0, 0.0, 0.0),
        test_creature_create_data(guid, 9001, hp),
        3,
        5,
        20.0,
        0,
        0,
        0,
        0,
        None,
        0,
        0,
        0,
        0,
        -1,
    );
    session
        .mutate_world_creature(guid, |creature| {
            // This test fixture represents a fully hydrated DB-backed
            // creature with no addon/template-addon auras.
            creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .set_spell_hit_aura_authority_inert_like_cpp(true);
            creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .set_spell_cast_log_aura_authority_inert_like_cpp(true);
            crate::creature_projection::test_support::seed_runtime_rng_like_cpp(
                creature, 0x5E11_117,
            );
        })
        .expect("registered test creature must remain available");

    // Fixtures assert exact melee damage terms, so the represented attack table
    // is inert for them until a test opts back in.
    disable_represented_melee_attack_table_for_test_like_cpp(session, guid);
}

/// Register a creature the way production does: through a session that owns
/// the canonical map manager `canonical`.
pub(in crate::session::tests) fn register_test_creature_mirrored_like_cpp(
    session: &mut WorldSession,
    _manager: SharedCanonicalMapManager,
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    hp: u32,
) {
    session.set_canonical_map_manager(Arc::clone(canonical));
    register_test_creature(session, Arc::clone(canonical), guid, hp);
}

/// Keep the canonical incarnation a fixture attached at `to_key` as the
/// creature's one incarnation.
///
/// F6-7 R7a / #1263 F6-8E. A fixture that registers a creature through the
/// session (at `from_key` of the store `registered`) and then attaches a
/// separately constructed canonical creature for the same GUID at `to_key`
/// owns two incarnations for one GUID. Attaching at the same key on the same
/// store already replaced the registered record; otherwise the registered
/// record is removed, so the fixture exercises ONE incarnation — the attached
/// one — exactly as the former legacy adoption did.
pub(in crate::session::tests) fn adopt_canonical_test_incarnation_like_cpp(
    registered: &SharedCanonicalMapManager,
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    from_key: (u16, u32),
    to_key: (u32, u32),
) {
    let same_record = Arc::ptr_eq(registered, canonical)
        && u32::from(from_key.0) == to_key.0
        && from_key.1 == to_key.1;
    if !same_record {
        let _ = registered
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove_creature_any(from_key.0, from_key.1, guid);
    }
    assert!(
        canonical
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .find_map(to_key.0, to_key.1)
            .and_then(|map| map.map().get_typed_creature(guid).map(|_| ()))
            .is_some(),
        "the fixture attached the canonical incarnation"
    );
}

/// Shape one canonical incarnation in place.
pub(in crate::session::tests) fn shape_canonical_test_incarnation_like_cpp(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    map_id: u32,
    instance_id: u32,
    shape: impl FnOnce(&mut wow_entities::Creature),
) {
    let mut manager = canonical
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let map = manager
        .find_map_mut(map_id, instance_id)
        .expect("the fixture attached the canonical map instance");
    map.map_mut()
        .with_creature_mut_like_cpp(guid, shape)
        .expect("the fixture attached the canonical incarnation");
}

/// Make the represented attack table inert for a fixture that asserts an exact
/// melee damage term.
///
/// C++ `Player::UpdateMeleeHitChances` (`StatSystem.cpp:743-746`) gives a player
/// `7.5 + CR_HIT_MELEE`, and `MeleeSpellMissChance` adds `19` for dual wielding.
/// A fixture that has not run the stat projection would otherwise carry a 5%
/// (or 24%) miss band, and a directly constructed creature carries no seeded
/// avoidance, so this makes the represented table inert for a fixture that
/// asserts an exact damage term.
pub(in crate::session::tests) fn disable_represented_melee_attack_table_for_test_like_cpp(
    session: &mut WorldSession,
    creature_guid: ObjectGuid,
) {
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        let mut stats = *player.effective_combat_stats_like_cpp();
        // `7.5` is C++'s base, plus the `19` dual-wield penalty
        // `MeleeSpellMissChance` adds, so both hands stay table-inert.
        stats.melee_hit_chance_pct = 26.5;
        player.replace_effective_combat_stats_like_cpp(stats);
    });
}

pub(in crate::session::tests) fn creature_template_lifecycle_store_for_test(
    entries: impl IntoIterator<Item = u32>,
) -> wow_data::CreatureTemplateLifecycleStoreLikeCpp {
    wow_data::CreatureTemplateLifecycleStoreLikeCpp::from_templates(entries.into_iter().map(
        |entry| wow_data::CreatureTemplateLifecycleRecordLikeCpp {
            entry,
            name: format!("Creature {entry}"),
            ai_name: String::new(),
            script_name: String::new(),
            required_expansion: 0,
            faction: 14,
            npc_flags: 0,
            speed_walk: 1.0,
            speed_run: 1.0,
            scale: 1.0,
            classification: 0,
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            unit_flags: 0,
            unit_flags2: 0,
            unit_flags3: 0,
            creature_type: 0,
            family: 0,
            trainer_class: 0,
            unit_class: 1,
            vehicle_id: 0,
            movement_type: 0,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            rooted: false,
            chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
            random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
            interaction_pause_timer_ms:
                wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
            flags_extra: 0,
            string_id: String::new(),
            regen_health: true,
            base_variance: 0.0,
            spells: [0; wow_data::MAX_CREATURE_SPELLS_LIKE_CPP],
            models: Vec::new(),
        },
    ))
}

pub(in crate::session::tests) fn lfg_dungeon_entry_for_test(
    id: u32,
    map_id: i16,
    difficulty_id: u8,
    target_level: u8,
) -> wow_data::LfgDungeonsEntry {
    wow_data::LfgDungeonsEntry {
        id,
        name: String::new(),
        description: String::new(),
        min_level: 0,
        max_level: 0,
        type_id: 0,
        subtype: 0,
        faction: 0,
        icon_texture_file_id: 0,
        rewards_bg_texture_file_id: 0,
        popup_bg_texture_file_id: 0,
        expansion_level: 0,
        map_id,
        difficulty_id,
        min_gear: 0.0,
        group_id: 0,
        order_index: 0,
        required_player_condition_id: 0,
        target_level,
        target_level_min: 0,
        target_level_max: 0,
        random_id: 0,
        scenario_id: 0,
        final_encounter_id: 0,
        count_tank: 0,
        count_healer: 0,
        count_damage: 0,
        min_count_tank: 0,
        min_count_healer: 0,
        min_count_damage: 0,
        bonus_reputation_amount: 0,
        mentor_item_level: 0,
        mentor_char_level: 0,
        flags: [0; 2],
    }
}

pub(in crate::session::tests) fn configure_single_creature_kill_reputation_for_test(
    session: &mut WorldSession,
) {
    let mut faction = FactionEntry::for_test_like_cpp(7, 5);
    faction.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
    let faction_store = FactionStore::from_entries([faction]);
    let creature_template_store = creature_template_lifecycle_store_for_test([9001]);
    let (onkill_store, report) =
        wow_data::reputation::CreatureOnKillReputationStoreLikeCpp::from_rows_like_cpp(
            [wow_data::reputation::CreatureOnKillReputationRowLikeCpp {
                creature_id: 9001,
                entry: wow_data::reputation::CreatureOnKillReputationEntryLikeCpp {
                    rep_faction_1: 7,
                    rep_faction_2: 0,
                    reputation_max_cap_1: wow_data::reputation::ReputationRankLikeCpp::Exalted
                        .as_u8(),
                    rep_value_1: 250,
                    reputation_max_cap_2: 0,
                    rep_value_2: 0,
                    is_team_award_1: false,
                    is_team_award_2: false,
                    team_dependent: false,
                },
            }],
            &creature_template_store,
            &faction_store,
        );
    assert_eq!(report.loaded, 1);
    session.set_faction_store(Arc::new(faction_store));
    session.set_creature_onkill_reputation_store(Arc::new(onkill_store));
}

pub(in crate::session::tests) fn configure_two_player_group_for_reputation_test(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    other_guid: ObjectGuid,
) {
    let (other_tx, _other_rx) = flume::bounded(10);
    let player_registry = Arc::new(PlayerRegistry::default());
    let mut other_info = broadcast_info(other_guid, other_tx);
    other_info.placement.map_id = 0;
    other_info.placement.position = Position::new(10.0, 10.0, 0.0, 0.0);
    other_info.placement.level = 80;
    other_info.placement.is_alive = true;
    player_registry.register_or_replace(other_guid, other_info, Default::default());

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player_guid);
    group.add_member(other_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session
        .social
        .set_group_guid_for_test_like_cpp(Some(group_guid));
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
}

pub(in crate::session::tests) fn reputation_aura_for_test(
    slot: u8,
    effect: RepresentedAuraEffectLikeCpp,
    amount: i32,
    misc_value: Option<i32>,
) -> AuraApplication {
    AuraApplication {
        spell_id: 69_500 + i32::from(slot),
        difficulty_id: 0,
        caster_guid: ObjectGuid::EMPTY,
        slot,
        duration_total: 30_000,
        duration_remaining: 30_000,
        stack_count: 1,
        aura_flags: 0x0000_0001,
        effect_mask: 0x0000_0001,
        aura_interrupt_flags: 0,
        aura_interrupt_flags2: 0,
        represented_effect: Some(effect),
        represented_amount: amount,
        represented_effect_amounts: vec![RepresentedAuraEffectAmountLikeCpp {
            effect_index: 0,
            amount,
        }],
        represented_misc_value: misc_value,
        represented_multiplier: 1.0,
        applied_at: std::time::Instant::now(),
    }
}

/// Mutate the canonical incarnation at map 0/0 without a session.
pub(in crate::session::tests) fn mutate_canonical_creature_for_test_like_cpp<R>(
    manager: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    f: impl FnOnce(&mut CanonicalCreatureFixtureMutLikeCpp<'_>) -> R,
) -> Option<R> {
    let mut manager = manager
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    manager
        .find_creature_mut(0, 0, guid)
        .map(|mut creature| f(&mut creature))
}
