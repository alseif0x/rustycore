//! Player spell-hit source authority: the complete-identity, tombstone,
//! fail-closed and area/quest/aura requirement scenarios for the login and
//! zone aura producers.
//!
//! Extracted from `session_tests.rs` under #592 so this responsibility owns
//! its own bounded file. The scenarios and their registrations are unchanged.

use super::*;

fn player_target_aura_spell_info_fixture_like_cpp(spell_id: i32, aura_types: &[i32]) -> SpellInfo {
    SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: aura_types.first().copied(),
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: aura_types
            .iter()
            .copied()
            .enumerate()
            .map(|(effect_index, effect_aura)| wow_data::SpellEffectInfo {
                effect_index: effect_index as u32,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura,
                ..Default::default()
            })
            .collect(),
    }
}

fn complete_empty_player_dynamic_spell_hit_aura_sources_like_cpp(session: &mut WorldSession) {
    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    session.set_chr_specialization_store(Arc::new(ChrSpecializationStore::from_entries([
        ChrSpecializationEntry {
            id: 71,
            class_id: 1,
            order_index: 0,
            role: 2,
        },
        ChrSpecializationEntry {
            id: 72,
            class_id: 1,
            order_index: 1,
            role: 2,
        },
        ChrSpecializationEntry {
            id: 73,
            class_id: 1,
            order_index: 2,
            role: 0,
        },
    ])));
    assert!(
        session.complete_represented_trait_config_authority_load_like_cpp(
            [(1, 1, 71, 1), (2, 1, 72, 1), (3, 1, 73, 1)],
            true,
        )
    );
    assert!(session.set_complete_player_skill_records_like_cpp(HashMap::new(), 0));
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::default()));
    session.begin_player_quest_status_authority_load_like_cpp();
    session.complete_player_quest_status_authority_load_like_cpp();
    session.set_represented_guild_id_like_cpp(0);
    session.reset_represented_glyphs_like_cpp();
    session.mark_represented_glyphs_loaded_like_cpp();
    session.set_spell_area_store(Arc::new(SpellAreaStoreLikeCpp::default()));
    let mut spell_store = SpellStore::new();
    spell_store.insert(
        33_795,
        player_target_aura_spell_info_fixture_like_cpp(
            33_795,
            &[wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE],
        ),
    );
    spell_store.insert(
        33_377,
        player_target_aura_spell_info_fixture_like_cpp(
            33_377,
            &[
                wow_data::spell::aura_types::SPELL_AURA_MOD_XP_PCT,
                wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
            ],
        ),
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_chain_store(Arc::new(wow_data::SpellChainStoreLikeCpp::default()));
    session.set_spell_linked_store(Arc::new(wow_data::SpellLinkedStoreLikeCpp::default()));
    session.set_spell_runtime_script_authority_like_cpp(
        Arc::new(BTreeSet::new()),
        Arc::new(BTreeSet::new()),
        Arc::new(BTreeSet::new()),
        Arc::new(BTreeSet::new()),
    );
    assert!(
        session.complete_represented_battle_pet_slot_authority_load_like_cpp([
            (0, None, true),
            (1, None, true),
            (2, None, true),
        ])
    );
    session.begin_represented_character_pet_authority_load_like_cpp();
    assert_eq!(
        session.load_represented_pet_stable_rows_like_cpp(
            0,
            std::iter::empty::<CharacterPetStableRowLikeCpp>(),
        ),
        0,
    );
    session.set_map_store(Arc::new(MapStore::from_entries([0, 530, 571].map(|id| {
        wow_data::MapEntry {
            id,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 1,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        }
    }))));
    session.set_area_table_store(Arc::new(AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 12,
            continent_id: 530,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
        wow_data::AreaTableEntry {
            id: 3_518,
            continent_id: 530,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
        wow_data::AreaTableEntry {
            id: 3_697,
            continent_id: 530,
            parent_area_id: 3_518,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
    ])));
    session.set_loaded_player_flags_like_cpp(0);
    session.set_player_zone_area_like_cpp(1, 12);
    session.set_player_zone_area_authority_complete_like_cpp(true);
}

fn complete_empty_player_spell_hit_authority_fixture_like_cpp() -> WorldSession {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 70_020);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AuthorityFixture".into(),
        Position::ZERO,
        530,
        1,
        1,
        80,
        0,
    ));
    session.set_player_aura_authority_complete_like_cpp(true);
    session.complete_player_equipment_inventory_authority_load_like_cpp();
    assert!(session.set_complete_represented_player_spell_rows_like_cpp([]));
    complete_empty_player_dynamic_spell_hit_aura_sources_like_cpp(&mut session);
    session
}

fn configure_outdoor_pvp_tf_authority_fixture_like_cpp(session: &mut WorldSession) {
    session.set_area_table_store(Arc::new(AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 3_519,
            continent_id: 530,
            parent_area_id: 0,
            area_bit: 1_143,
            exploration_level: 0,
            mount_flags: 2,
            flags: 0,
        },
        wow_data::AreaTableEntry {
            id: 3_697,
            continent_id: 530,
            parent_area_id: 3_519,
            area_bit: 1_321,
            exploration_level: 64,
            mount_flags: 2,
            flags: 0,
        },
    ])));
    session.set_player_zone_area_like_cpp(3_519, 3_697);
    session.set_player_zone_area_authority_complete_like_cpp(true);
}

fn spell_area_store_for_authority_like_cpp(
    rows: impl IntoIterator<Item = wow_data::SpellAreaRowLikeCpp>,
) -> SpellAreaStoreLikeCpp {
    SpellAreaStoreLikeCpp::from_rows_like_cpp(rows, |_| true, |_| true, |_| true).store
}

#[test]
fn rejected_spell_linked_trigger_blocks_hookless_inert_candidate_like_cpp() {
    const CANDIDATE_SPELL_ID: u32 = 33_795;

    let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    assert!(session.spell_has_no_unrepresented_runtime_hooks_like_cpp(CANDIDATE_SPELL_ID));
    assert!(session.player_target_spell_is_hit_inert_like_cpp(CANDIDATE_SPELL_ID, 0));

    session.spell_linked_rejected_trigger_spell_ids_like_cpp = None;
    assert!(
        !session.spell_has_no_unrepresented_runtime_hooks_like_cpp(CANDIDATE_SPELL_ID),
        "missing rejected-row authority must fail closed"
    );

    session.set_spell_runtime_script_authority_like_cpp(
        Arc::new(BTreeSet::new()),
        Arc::new(BTreeSet::new()),
        Arc::new(BTreeSet::new()),
        Arc::new(BTreeSet::from([CANDIDATE_SPELL_ID])),
    );
    assert!(
        !session.spell_has_no_unrepresented_runtime_hooks_like_cpp(CANDIDATE_SPELL_ID),
        "a rejected spell_linked_spell row still represents a possible C++ hook"
    );
    assert!(!session.player_target_spell_is_hit_inert_like_cpp(CANDIDATE_SPELL_ID, 0));
}

#[test]
fn player_spell_hit_source_authority_requires_one_complete_identity_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 70_010);
    let next_player_guid = ObjectGuid::create_player(1, 70_011);

    session.set_player_guid(Some(player_guid));
    assert!(
        session.set_complete_represented_player_spell_rows_like_cpp(std::iter::empty::<
            RepresentedPlayerSpellLikeCpp,
        >(),)
    );
    session.set_player_aura_authority_complete_like_cpp(true);
    session.complete_player_equipment_inventory_authority_load_like_cpp();
    complete_empty_player_dynamic_spell_hit_aura_sources_like_cpp(&mut session);
    assert!(session.player_aura_authority_complete_like_cpp());
    assert!(session.player_equipment_inventory_authority_complete_like_cpp());
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "a GUID without its matching Player controller is incomplete identity proof"
    );

    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AuthorityOwner".into(),
        Position::ZERO,
        571,
        1,
        1,
        1,
        0,
    ));
    assert!(session.player_aura_authority_complete_like_cpp());
    assert!(session.player_equipment_inventory_authority_complete_like_cpp());
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());

    assert!(!session.ensure_login_player_controller_like_cpp(
        player_guid,
        "AuthorityOwner".into(),
        Position::ZERO,
        571,
        1,
        1,
        1,
        0,
    ));
    assert!(
        session.player_aura_authority_complete_like_cpp(),
        "reattaching the same C++ Player identity must retain its aura-source proof"
    );
    assert!(session.player_equipment_inventory_authority_complete_like_cpp());

    session.set_player_guid(Some(next_player_guid));
    assert!(!session.player_aura_authority_complete_like_cpp());
    assert!(!session.player_equipment_inventory_authority_complete_like_cpp());
    assert!(!session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());
}

#[test]
fn player_spell_hit_source_tombstone_does_not_cross_player_lifetimes_like_cpp() {
    let (mut session, _, _) = make_session();
    let first = ObjectGuid::create_player(1, 70_030);
    let second = ObjectGuid::create_player(1, 70_031);

    session.set_player_guid(Some(first));
    session.tombstone_player_spell_hit_aura_authority_like_cpp();
    assert!(session.player_spell_hit_aura_authority_tombstoned_like_cpp);

    session.set_player_guid(Some(second));
    assert!(
        !session.player_spell_hit_aura_authority_tombstoned_like_cpp,
        "FIRST-login loss belongs to the old C++ Player, not the surviving WorldSession"
    );
}

#[test]
fn player_spell_hit_source_authority_is_fail_closed_for_sources_and_passives_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 70_012);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AuthoritySources".into(),
        Position::ZERO,
        571,
        1,
        1,
        1,
        0,
    ));
    session.set_player_aura_authority_complete_like_cpp(true);

    assert!(!session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());
    session.complete_player_equipment_inventory_authority_load_like_cpp();
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "empty known_spells is not evidence until its represented DB rows are complete"
    );
    assert!(
        session.set_complete_represented_player_spell_rows_like_cpp(std::iter::empty::<
            RepresentedPlayerSpellLikeCpp,
        >(),)
    );
    complete_empty_player_dynamic_spell_hit_aura_sources_like_cpp(&mut session);
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());

    const KNOWN_SPELL: i32 = 90_001;
    session.set_known_spells_like_cpp(vec![KNOWN_SPELL]);
    assert!(
        session.set_complete_represented_player_spell_rows_like_cpp([
            RepresentedPlayerSpellLikeCpp {
                spell_id: KNOWN_SPELL,
                active: true,
                disabled: false,
                dependent: false,
                favorite: false,
                state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
        ])
    );
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_for_difficulty_like_cpp(2),
        "missing SpellMisc metadata must not classify a known spell as non-passive"
    );

    let mut passive_store = SpellStore::new();
    let mut passive_attributes = [0; 15];
    passive_attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
    passive_store.insert_spell_misc_attributes_for_difficulty_like_cpp(
        KNOWN_SPELL,
        2,
        passive_attributes,
    );
    session.set_spell_store(Arc::new(passive_store));
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_for_difficulty_like_cpp(2),
        "the selected difficulty's passive bit must fail closed"
    );

    let mut non_passive_store = SpellStore::new();
    non_passive_store.insert_spell_misc_attributes_for_difficulty_like_cpp(KNOWN_SPELL, 2, [0; 15]);
    session.set_spell_store(Arc::new(non_passive_store));
    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_for_difficulty_like_cpp(2));

    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: ObjectGuid::create_item(1, 90_002),
            entry_id: 25,
            db_guid: 90_002,
            inventory_type: None,
        },
    );
    assert!(!session.can_authorize_empty_player_spell_hit_aura_source_for_difficulty_like_cpp(2));
}

#[test]
fn player_spell_hit_source_authority_requires_empty_traits_and_active_glyphs_like_cpp() {
    let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());

    session.begin_represented_trait_config_authority_load_like_cpp();
    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "trait config and entry queries must both be authoritative"
    );

    assert!(
        session.complete_represented_trait_config_authority_load_like_cpp(
            [(1, 1, 71, 1), (2, 1, 72, 1)],
            true,
        )
    );
    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "a missing C++ specialization config can be created with granted entries"
    );

    assert!(
        session.complete_represented_trait_config_authority_load_like_cpp(
            [(1, 1, 71, 1), (2, 1, 72, 1), (3, 1, 73, 1)],
            false,
        )
    );
    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "a nonempty persisted entry query is not the narrow empty-config proof"
    );

    assert!(
        session.complete_represented_trait_config_authority_load_like_cpp(
            [(1, 1, 71, 1), (2, 1, 72, 1), (3, 1, 73, 1)],
            true,
        )
    );
    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());

    assert!(
        session.set_complete_represented_spell_trait_definition_ids_like_cpp([(90_010, 70_010,)])
    );
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "C++ active traits can cast an unrepresented trait spell"
    );

    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    assert!(session.load_represented_glyph_row_like_cpp(&glyph_catalog::catalog(123), 0, 0, 123));
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "C++ _LoadGlyphAuras casts every nonzero active-group glyph"
    );

    session.set_represented_active_talent_group_like_cpp(1);
    assert!(
        session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "an inactive-group glyph is not cast by C++ _LoadGlyphAuras"
    );
    session.set_represented_active_talent_group_like_cpp(0);
    assert!(!session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());
}

#[test]
fn player_spell_hit_source_authority_gates_update_zone_aura_producers_like_cpp() {
    let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    assert_eq!(
        session.player_world_local_state_like_cpp(),
        Some(
            wow_entities::PlayerWorldLocalState::from_represented_parts_like_cpp(
                1, 12, true, false, None, 0, None,
            )
        )
    );
    assert_eq!(session.represented_player_flags_value_like_cpp(), Some(0));
    assert!(!session.represented_player_has_flag_like_cpp(PLAYER_FLAGS_WAR_MODE_DESIRED_LIKE_CPP));
    assert_eq!(
        (
            session.represented_spell_area_autocast_source_is_empty_like_cpp(),
            session.represented_war_mode_update_zone_aura_source_is_empty_like_cpp(),
            session.represented_update_area_pvp_rule_aura_source_is_empty_like_cpp(),
            session.represented_update_zone_script_aura_source_is_hit_inert_like_cpp(),
        ),
        (true, true, true, true),
    );
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());

    session.set_loaded_player_flags_like_cpp(PLAYER_FLAGS_WAR_MODE_DESIRED_LIKE_CPP);
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "C++ UpdateWarModeAuras casts 282559 or 269083 when War Mode is desired"
    );

    session.set_loaded_player_flags_like_cpp(0);
    session.set_player_zone_area_like_cpp(4_197, 4_197);
    session.set_player_zone_area_authority_complete_like_cpp(true);
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "BattlefieldWG adds control phase-shift auras on zone entry"
    );

    session.set_player_zone_area_like_cpp(3_518, 3_697);
    session.set_player_zone_area_authority_complete_like_cpp(true);
    assert!(
        session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "OutdoorPvPNA spell 33795 is audited as incoming SpellHitResult-inert aura type 79"
    );

    session.set_player_zone_area_authority_complete_like_cpp(false);
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "an unverified zone cannot prove OutdoorPvP/Battlefield sources hit-inert"
    );
}

#[test]
fn player_spell_hit_source_authority_audits_outdoor_pvp_tf_terokkar_buff_like_cpp() {
    let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    configure_outdoor_pvp_tf_authority_fixture_like_cpp(&mut session);
    assert!(
        session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "OutdoorPvPTF zone 3519 must admit spell 33377 only after both XP and outgoing-damage auras are proven hit-inert"
    );
}

#[test]
fn player_spell_hit_source_authority_keeps_outdoor_pvp_tf_dungeon_ids_fail_closed_like_cpp() {
    for zone_id in [3_791, 3_789, 3_792, 3_790] {
        let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
        configure_outdoor_pvp_tf_authority_fixture_like_cpp(&mut session);
        session.set_player_zone_area_like_cpp(zone_id, 3_697);
        assert!(
            !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
            "OutdoorPvPTF dungeon zone {zone_id} must remain fail-closed without C++ (Map*, zone) registration authority"
        );
    }
}

#[test]
fn player_spell_hit_source_authority_fails_closed_for_unproven_outdoor_pvp_tf_buff_like_cpp() {
    let mut missing_metadata = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    configure_outdoor_pvp_tf_authority_fixture_like_cpp(&mut missing_metadata);
    missing_metadata.set_spell_store(Arc::new(SpellStore::new()));
    assert!(
        !missing_metadata.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "missing effective spell 33377 metadata must fail closed"
    );

    let mut scripted = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    configure_outdoor_pvp_tf_authority_fixture_like_cpp(&mut scripted);
    scripted.set_spell_runtime_script_authority_like_cpp(
        Arc::new(BTreeSet::from([33_377])),
        Arc::new(BTreeSet::new()),
        Arc::new(BTreeSet::new()),
        Arc::new(BTreeSet::new()),
    );
    assert!(
        !scripted.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "an exact runtime hook on spell 33377 must fail closed"
    );

    let mut hit_relevant = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    configure_outdoor_pvp_tf_authority_fixture_like_cpp(&mut hit_relevant);
    let mut spell_store = SpellStore::new();
    spell_store.insert(
        33_377,
        player_target_aura_spell_info_fixture_like_cpp(
            33_377,
            &[
                wow_data::spell::aura_types::SPELL_AURA_MOD_XP_PCT,
                wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            ],
        ),
    );
    hit_relevant.set_spell_store(Arc::new(spell_store));
    assert!(
        !hit_relevant.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "a hit-relevant effective effect on spell 33377 must fail closed"
    );
}

#[test]
fn player_spell_hit_source_authority_gates_free_for_all_area_ancestry_like_cpp() {
    let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());

    let area = |id, parent_area_id, flags| wow_data::AreaTableEntry {
        id,
        continent_id: 530,
        parent_area_id,
        area_bit: -1,
        exploration_level: 0,
        mount_flags: 0,
        flags,
    };
    session.set_area_table_store(Arc::new(AreaTableStore::from_entries([
        area(12, 11, 0),
        area(11, 0, AREA_FLAG_FREE_FOR_ALL_PVP_LIKE_CPP),
    ])));
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "an FFA ancestor makes C++ UpdateArea cast 208682 and 134735"
    );

    session.set_area_table_store(Arc::new(AreaTableStore::from_entries([area(12, 11, 0)])));
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "a missing AreaTable ancestor must fail closed"
    );

    session.set_area_table_store(Arc::new(AreaTableStore::from_entries([
        area(12, 11, 0),
        area(11, 12, 0),
    ])));
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "a cyclic AreaTable hierarchy cannot prove coherent termination"
    );
}

#[test]
fn player_spell_hit_source_authority_requires_locked_complete_battle_pet_slots_like_cpp() {
    let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());

    assert!(
        !session.complete_represented_battle_pet_slot_authority_load_like_cpp([
            (0, None, true),
            (1, None, true),
        ])
    );
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "an incomplete battle_pet_slots query cannot exclude C++ login spell 125610"
    );

    assert!(
        session.complete_represented_battle_pet_slot_authority_load_like_cpp([
            (0, None, false),
            (1, None, true),
            (2, None, true),
        ])
    );
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "C++ learns spell 125610 when battle-pet slot zero is unlocked"
    );

    assert!(
        session.complete_represented_battle_pet_slot_authority_load_like_cpp([
            (0, None, true),
            (1, None, true),
            (2, None, true),
        ])
    );
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());
}

#[test]
fn player_spell_hit_source_authority_requires_complete_empty_character_pets_like_cpp() {
    let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());

    session.begin_represented_character_pet_authority_load_like_cpp();
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "an incomplete character_pet query cannot exclude pet-to-owner aura casts"
    );

    assert_eq!(
        session.load_represented_pet_stable_rows_like_cpp(
            0,
            [character_pet_stable_row_like_cpp(
                42,
                PetSaveMode::active_slot(0),
                1,
            )],
        ),
        1,
    );
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "any persisted character_pet row must fail closed"
    );

    assert_eq!(
        session.load_represented_pet_stable_rows_like_cpp(
            0,
            std::iter::empty::<CharacterPetStableRowLikeCpp>(),
        ),
        0,
    );
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());

    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 530, 0, 500, 42);
    session.set_represented_pet_mode_state_like_cpp(
        Some(pet_guid),
        wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP,
        wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP,
    );
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "an active represented pet revokes the empty lifetime proof"
    );
}

#[test]
fn player_spell_hit_source_authority_rejects_instance_map_login_hooks_like_cpp() {
    let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());

    session.set_map_store(Arc::new(MapStore::from_entries([wow_data::MapEntry {
        id: 530,
        instance_type: wow_data::map::MAP_INSTANCE,
        expansion_id: 1,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }])));
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "unrepresented InstanceScript::OnPlayerEnter can cast auras during AddPlayerToMap"
    );
}

#[test]
fn player_spell_hit_source_first_login_tombstone_survives_area_change_like_cpp() {
    let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());

    session.tombstone_player_spell_hit_aura_authority_like_cpp();
    assert!(!session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());
    assert!(session.update_area_represented_like_cpp(2));
    session.set_area_table_store(Arc::new(AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 2,
            continent_id: 530,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
    ])));
    session.set_player_zone_area_authority_complete_like_cpp(true);
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "an unknown FIRST-login cast can leave a permanent aura across later area changes"
    );
}

#[test]
fn player_spell_hit_source_authority_requires_login_skill_guild_and_quest_sources_like_cpp() {
    let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());

    session.set_represented_guild_id_like_cpp(7);
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "C++ Guild::SendLoginInfo can learn guild perk spells"
    );
    session.set_represented_guild_id_like_cpp(0);

    session.set_player_skill_records_like_cpp(HashMap::new());
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "trait and quest conditions require a complete player skill source"
    );
    assert!(session.set_complete_player_skill_records_like_cpp(HashMap::new(), 0));

    let mut auto_push = test_quest_template(10_044);
    auto_push.flags_ex |= 0x0400_0000;
    auto_push.source_spell_id = 33_795;
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [auto_push],
    )));
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "C++ PushQuests can AddQuest and cast an AUTO_PUSH template's SourceSpellID"
    );

    let mut rewarded = test_quest_template(10_046);
    rewarded.reward_spell = 90_046;
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [rewarded],
    )));
    session.begin_player_quest_status_authority_load_like_cpp();
    session.record_represented_rewarded_quest_row_like_cpp(10_046);
    session.complete_player_quest_status_authority_load_like_cpp();
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "C++ _LoadQuestStatusRewarded learns each row's reward spell"
    );

    let mut recast = test_quest_template(10_047);
    recast.flags |= QUEST_FLAGS_PLAYER_CAST_ACCEPT_LIKE_CPP;
    recast.flags_ex |= QUEST_FLAGS_EX_RECAST_ACCEPT_SPELL_ON_LOGIN_LIKE_CPP;
    recast.source_spell_id = 33_795;
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [recast.clone()],
    )));
    session.begin_player_quest_status_authority_load_like_cpp();
    session.player_quests.clear();
    session.player_quests.insert(
        recast.id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id: recast.id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );
    session.complete_player_quest_status_authority_load_like_cpp();
    assert!(
        session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "an accept-spell recast is admissible only after its exact spell is proven hit-inert"
    );

    recast.source_spell_id = 90_047;
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [recast],
    )));
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "an unrepresented active-quest login cast must fail closed"
    );
}

#[test]
fn player_spell_hit_source_authority_evaluates_spell_area_quest_requirements_like_cpp() {
    let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [test_quest_template(10_045)],
    )));
    session.set_player_zone_area_like_cpp(3_518, 3_697);
    session.set_player_zone_area_authority_complete_like_cpp(true);
    session.complete_player_quest_status_authority_load_like_cpp();
    session.set_spell_area_store(Arc::new(spell_area_store_for_authority_like_cpp([
        wow_data::SpellAreaRowLikeCpp {
            spell_id: 32_649,
            area_id: 3_518,
            quest_start: 10_045,
            quest_start_status: 1 << crate::conditions::QUEST_STATUS_REWARDED_LIKE_CPP,
            quest_end_status: 0,
            quest_end: 0,
            aura_spell: 0,
            race_mask: 0,
            gender: wow_data::GENDER_NONE_LIKE_CPP,
            flags: SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP,
        },
    ])));
    assert!(
        session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "the issue #26 zone aura cannot fit while quest 10045 is exactly not rewarded"
    );

    session.rewarded_quests.insert(10_045);
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "the same C++ spell_area row can autocast after its rewarded requirement fits"
    );

    session.rewarded_quests.clear();
    session.invalidate_player_quest_status_authority_like_cpp();
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "missing quest-source authority must fail closed"
    );

    session.complete_player_quest_status_authority_load_like_cpp();
    session.set_player_zone_area_authority_complete_like_cpp(false);
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "a DB-seeded or zero terrain location must not prove the row out of scope"
    );
}

#[test]
fn player_spell_hit_source_authority_evaluates_spell_area_aura_requirement_like_cpp() {
    let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    let row = wow_data::SpellAreaRowLikeCpp {
        spell_id: 32_649,
        area_id: 0,
        quest_start: 0,
        quest_start_status: 0,
        quest_end_status: 0,
        quest_end: 0,
        aura_spell: 33_795,
        race_mask: 0,
        gender: wow_data::GENDER_NONE_LIKE_CPP,
        flags: SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP,
    };
    session.set_spell_area_store(Arc::new(spell_area_store_for_authority_like_cpp([row])));
    assert!(
        session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "a missing positive aura prerequisite proves the SpellArea row cannot fit"
    );

    session
        .visible_auras
        .insert(0, test_visible_aura(0, 33_795));
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "a present positive prerequisite can make the AUTOCAST row fit"
    );

    session.set_spell_area_store(Arc::new(spell_area_store_for_authority_like_cpp([
        wow_data::SpellAreaRowLikeCpp {
            aura_spell: -33_795,
            ..row
        },
    ])));
    assert!(
        session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "a present aura disproves a negative C++ SpellArea prerequisite"
    );

    session.visible_auras.clear();
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "an absent aura makes the negative prerequisite fit"
    );
}

#[test]
fn player_spell_hit_source_sync_sets_and_mutations_invalidate_canonical_auras_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 70_013);
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AuthorityCanonical".into(),
        Position::ZERO,
        571,
        1,
        1,
        1,
        0,
    ));
    assert!(session.install_detached_canonical_player_for_test_like_cpp());
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 571, 0);
    session.set_player_aura_authority_complete_like_cpp(true);
    session.complete_player_equipment_inventory_authority_load_like_cpp();
    assert!(
        session.set_complete_represented_player_spell_rows_like_cpp(std::iter::empty::<
            RepresentedPlayerSpellLikeCpp,
        >(),)
    );
    complete_empty_player_dynamic_spell_hit_aura_sources_like_cpp(&mut session);

    assert_eq!(
        session.sync_player_spell_hit_aura_authority_to_canonical_like_cpp(),
        Some(true)
    );
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| player
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_hit_inert_aura_authority_like_cpp()),
        Some(true)
    );

    assert!(session.battle_pet_unlock_slot_like_cpp(0));
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| player
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_hit_inert_aura_authority_like_cpp()),
        Some(false),
        "battle-pet slot mutations must revoke the canonical proof immediately"
    );
    assert!(!session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());
    assert!(
        session.complete_represented_battle_pet_slot_authority_load_like_cpp([
            (0, None, true),
            (1, None, true),
            (2, None, true),
        ])
    );
    assert_eq!(
        session.sync_player_spell_hit_aura_authority_to_canonical_like_cpp(),
        Some(true)
    );

    session.learn_known_spell_like_cpp(90_003);
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| player
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_hit_inert_aura_authority_like_cpp()),
        Some(false),
        "known-spell mutation must revoke the canonical proof immediately"
    );

    session.set_known_spells_like_cpp(Vec::new());
    assert!(
        session.set_complete_represented_player_spell_rows_like_cpp(std::iter::empty::<
            RepresentedPlayerSpellLikeCpp,
        >(),)
    );
    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    assert_eq!(
        session.sync_player_spell_hit_aura_authority_to_canonical_like_cpp(),
        Some(true)
    );
    session.begin_player_equipment_inventory_authority_load_like_cpp();
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| player
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_hit_inert_aura_authority_like_cpp()),
        Some(false),
        "starting a new equipment query must revoke the canonical proof"
    );
}
