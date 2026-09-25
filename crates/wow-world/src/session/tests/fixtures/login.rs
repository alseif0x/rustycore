//! Login, talent, and progression fixtures.
//!
//! These builders retain the original session-test behavior and are
//! visible only within the parent `session::tests` subtree.

use super::*;

pub(in crate::session::tests) fn stuck_spell_info_like_cpp(spell_id: i32) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![wow_data::SpellEffectInfo {
            effect_index: 0,
            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_STUCK,
            ..Default::default()
        }],
    }
}

pub(in crate::session::tests) fn hearthstone_spell_info_like_cpp() -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id: 8690,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: Vec::new(),
    }
}

pub(in crate::session::tests) fn set_stuck_spell_store_like_cpp(
    session: &mut WorldSession,
    spell_id: i32,
) {
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(spell_id, stuck_spell_info_like_cpp(spell_id));
    spell_store.insert(8690, hearthstone_spell_info_like_cpp());
    session.set_spell_store(Arc::new(spell_store));
}

pub(in crate::session::tests) fn threat_spell_info_like_cpp(
    spell_id: i32,
    effect: u32,
    damage: i32,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![wow_data::SpellEffectInfo {
            effect_index: 0,
            effect,
            effect_base_points: damage,
            ..Default::default()
        }],
    }
}

pub(in crate::session::tests) fn cooldown_or_charges_spell_info_like_cpp(
    spell_id: i32,
    effect: u32,
    damage: i32,
    misc_value: i32,
    trigger_spell: i32,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![wow_data::SpellEffectInfo {
            effect_index: 0,
            effect,
            effect_base_points: damage,
            effect_misc_value_1: misc_value,
            effect_trigger_spell: trigger_spell,
            ..Default::default()
        }],
    }
}

pub(in crate::session::tests) fn power_spell_info_like_cpp(
    spell_id: i32,
    effect: u32,
    damage: i32,
    power_type: PowerType,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![wow_data::SpellEffectInfo {
            effect_index: 0,
            effect,
            effect_base_points: damage,
            effect_misc_value_1: power_type as i32,
            ..Default::default()
        }],
    }
}

pub(in crate::session::tests) fn represented_hunter_pet_stable_like_cpp(
    pet_number: u32,
    creature_id: u32,
) -> PetStable {
    PetStable {
        current_pet_index: Some(0),
        active_pets: vec![Some(PetStableInfo {
            name: "Misha".to_string(),
            pet_number,
            creature_id,
            display_id: 12_345,
            health: 345,
            mana: 67,
            created_by_spell_id: 9_001,
            specialization_id: 2,
            level: 80,
            react_state: wow_entities::ReactState::Defensive,
            pet_type: wow_entities::PetType::Hunter,
            ..PetStableInfo::default()
        })],
        stabled_pets: Vec::new(),
        unslotted_pets: Vec::new(),
    }
}

pub(in crate::session::tests) fn character_pet_stable_row_like_cpp(
    pet_number: u32,
    slot: i16,
    pet_type: u8,
) -> CharacterPetStableRowLikeCpp {
    CharacterPetStableRowLikeCpp {
        pet_number,
        creature_id: 500 + pet_number,
        display_id: 12_000 + pet_number,
        level: 70,
        experience: 123_456,
        react_state: 1,
        slot,
        name: format!("Pet{pet_number}"),
        was_renamed: true,
        health: 345,
        mana: 67,
        action_bar: "1 2 3".to_string(),
        last_save_time: 98_765,
        created_by_spell_id: 9_001,
        pet_type,
        specialization_id: 2,
    }
}

pub(in crate::session::tests) fn first_login_cast_spell_store_like_cpp(
    normal_spell: u32,
    npe_spell: u32,
) -> PlayerCreateInfoCastSpellStoreLikeCpp {
    PlayerCreateInfoCastSpellStoreLikeCpp::from_rows_like_cpp([
        wow_data::PlayerCreateInfoCastSpellRowLikeCpp {
            race_mask: 1,
            class_mask: 1,
            spell_id: normal_spell,
            create_mode: wow_data::PLAYER_CREATE_MODE_NORMAL_LIKE_CPP as i8,
        },
        wow_data::PlayerCreateInfoCastSpellRowLikeCpp {
            race_mask: 1,
            class_mask: 1,
            spell_id: npe_spell,
            create_mode: wow_data::PLAYER_CREATE_MODE_NPE_LIKE_CPP as i8,
        },
    ])
}

pub(in crate::session::tests) fn player_create_custom_spell_store_like_cpp()
-> PlayerCreateInfoCustomSpellStoreLikeCpp {
    PlayerCreateInfoCustomSpellStoreLikeCpp::from_rows_like_cpp([
        wow_data::PlayerCreateInfoCustomSpellRowLikeCpp {
            race_mask: 1,
            class_mask: 1,
            spell_id: 80_001,
        },
        wow_data::PlayerCreateInfoCustomSpellRowLikeCpp {
            race_mask: 1,
            class_mask: 1,
            spell_id: 80_002,
        },
        wow_data::PlayerCreateInfoCustomSpellRowLikeCpp {
            race_mask: 2,
            class_mask: 1,
            spell_id: 80_003,
        },
    ])
}

pub(in crate::session::tests) fn first_login_noop_spell_store_like_cpp(
    spell_ids: impl IntoIterator<Item = i32>,
) -> SpellStore {
    let mut store = SpellStore::new();
    for spell_id in spell_ids {
        store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: 0,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: None,
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: Vec::new(),
            },
        );
    }
    store
}

pub(in crate::session::tests) fn first_login_reputation_faction_store_like_cpp() -> FactionStore {
    let mut entries = Vec::new();
    for (rep_index, faction_id) in FIRST_LOGIN_START_REPUTATION_COMMON_FACTIONS_LIKE_CPP
        .iter()
        .chain(FIRST_LOGIN_START_REPUTATION_ALLIANCE_FACTIONS_LIKE_CPP.iter())
        .chain(FIRST_LOGIN_START_REPUTATION_HORDE_FACTIONS_LIKE_CPP.iter())
        .enumerate()
    {
        let mut entry = FactionEntry::for_test_like_cpp(*faction_id, rep_index as i16);
        entry.reputation_race_mask[0] = 1;
        entry.reputation_max[0] = wow_data::reputation::REPUTATION_CAP_LIKE_CPP;
        entries.push(entry);
    }
    FactionStore::from_entries(entries)
}

pub(in crate::session::tests) fn install_xp_victim_like_cpp(
    session: &mut WorldSession,
    creature_guid: ObjectGuid,
    tapped: bool,
) {
    let player_guid = session
        .player_guid()
        .unwrap_or_else(|| ObjectGuid::create_player(1, 0xE1C0));
    session.set_player_guid(Some(player_guid));
    let map_id = session.player_map_id_like_cpp();
    session.set_map_manager(shared_map_manager());
    if session.player_position_like_cpp().is_none() {
        session.set_player_map_position_like_cpp(map_id, Position::new(10.0, 10.0, 0.0, 0.0));
    }
    session.register_world_creature(
        map_id,
        Position::new(10.0, 10.0, 0.0, 0.0),
        test_creature_create_data(creature_guid, 9_001, 100),
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
    if tapped {
        session
            .mutate_world_creature(creature_guid, |creature| {
                creature.creature.set_tapped_by_player(player_guid, &[]);
            })
            .expect("XP victim is installed in the represented map");
    }
}

pub(in crate::session::tests) fn install_tapped_xp_victim_like_cpp(
    session: &mut WorldSession,
    creature_guid: ObjectGuid,
) {
    install_xp_victim_like_cpp(session, creature_guid, true);
}

pub(in crate::session::tests) fn test_db2_area_trigger_like_cpp(
    trigger_id: u32,
    map_id: u16,
    pos: Position,
) -> wow_data::AreaTriggerDb2Entry {
    wow_data::AreaTriggerDb2Entry {
        id: trigger_id,
        message: String::new(),
        pos: wow_data::Db2Position3 {
            x: pos.x,
            y: pos.y,
            z: pos.z,
        },
        continent_id: map_id as i16,
        phase_use_flags: 0,
        phase_id: 0,
        phase_group_id: 0,
        radius: 5.0,
        box_length: 0.0,
        box_width: 0.0,
        box_height: 0.0,
        box_yaw: 0.0,
        shape_type: 0,
        shape_id: 0,
        area_trigger_action_set_id: 0,
        flags: 0,
    }
}

pub(in crate::session::tests) fn test_talent_entry_like_cpp(
    id: u32,
    rank: u8,
    spell_id: i32,
) -> wow_data::TalentEntry {
    let mut spell_rank = [0; 9];
    spell_rank[usize::from(rank)] = spell_id;
    wow_data::TalentEntry {
        id,
        description: String::new(),
        tier_id: 0,
        flags: 0,
        column_index: 0,
        tab_id: 0,
        class_id: 0,
        spec_id: 0,
        spell_id,
        overrides_spell_id: 0,
        required_spell_id: 0,
        category_mask: [0; 2],
        spell_rank,
        prereq_talent: [0; 3],
        prereq_rank: [0; 3],
    }
}

pub(in crate::session::tests) fn test_spell_info_like_cpp(spell_id: i32) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: Vec::new(),
    }
}

pub(in crate::session::tests) fn test_visible_aura(slot: u8, spell_id: i32) -> AuraApplication {
    AuraApplication {
        spell_id,
        difficulty_id: 0,
        caster_guid: ObjectGuid::EMPTY,
        slot,
        duration_total: 30_000,
        duration_remaining: 30_000,
        stack_count: 1,
        aura_flags: 0x1,
        effect_mask: 0x1,
        aura_interrupt_flags: 0,
        aura_interrupt_flags2: 0,
        represented_effect: None,
        represented_amount: 0,
        represented_effect_amounts: Vec::new(),
        represented_misc_value: None,
        represented_multiplier: 1.0,
        applied_at: Instant::now(),
    }
}

pub(in crate::session::tests) fn install_test_talent_tab_store_like_cpp(
    session: &mut WorldSession,
) -> wow_data::TalentTabStore {
    let talent_tabs = wow_data::TalentTabStore::from_entries([wow_data::TalentTabEntry {
        id: 0,
        name: String::new(),
        background_file: String::new(),
        order_index: 0,
        race_mask: 0,
        class_mask: 1,
        pet_talent_mask: 0,
        spell_icon_id: 0,
    }]);
    session.set_player_class_like_cpp(1);
    talent_tabs
}

pub(in crate::session::tests) fn cuf_profile_for_save_test(
    name: &str,
    height: u16,
) -> wow_packet::packets::misc::CufProfile {
    wow_packet::packets::misc::CufProfile {
        profile_name: name.to_string(),
        frame_height: height,
        frame_width: 120,
        sort_by: 1,
        health_text: 2,
        top_point: 3,
        bottom_point: 4,
        left_point: 5,
        top_offset: 6,
        bottom_offset: 7,
        left_offset: 8,
        bool_options: 0b10101,
    }
}

pub(in crate::session::tests) fn currency_entry(id: u32) -> wow_data::CurrencyTypesEntry {
    wow_data::CurrencyTypesEntry {
        id,
        category_id: 0,
        inventory_icon_file_id: 0,
        spell_weight: 0,
        spell_category: 0,
        max_qty: 0,
        max_earnable_per_week: 0,
        quality: 0,
        faction_id: 0,
        award_condition_id: 0,
        flags: wow_constants::CurrencyTypesFlags::empty(),
        flags_b: wow_constants::CurrencyTypesFlagsB::empty(),
    }
}
