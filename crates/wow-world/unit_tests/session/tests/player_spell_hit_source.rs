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

#[path = "player_spell_hit_source/identity_and_lifetime.rs"]
mod identity_and_lifetime;
#[path = "player_spell_hit_source/outdoor_pvp_and_area_ancestry.rs"]
mod outdoor_pvp_and_area_ancestry;
#[path = "player_spell_hit_source/pet_and_login_sources.rs"]
mod pet_and_login_sources;
#[path = "player_spell_hit_source/source_mutation_invalidation.rs"]
mod source_mutation_invalidation;
#[path = "player_spell_hit_source/spell_area_requirements.rs"]
mod spell_area_requirements;
#[path = "player_spell_hit_source/trait_glyph_and_zone_gates.rs"]
mod trait_glyph_and_zone_gates;
