//! Creature spell, wire, and melee fixtures.
//!
//! These builders retain the original session-test behavior and are
//! visible only within the parent `session::tests` subtree.

use super::*;

pub(in crate::session::tests) fn legacy_aggro_candidate_like_cpp(
    player_guid: ObjectGuid,
    position: Position,
) -> LegacyCreatureAggroCandidateLikeCpp {
    LegacyCreatureAggroCandidateLikeCpp {
        player_guid,
        map_id: 0,
        instance_id: 0,
        map_difficulty_id: 0,
        position,
        player_visibility_represented: true,
        player_phase_shift: PhaseShift::default(),
        player_visibility_detection: UnitVisibilityDetectionStateLikeCpp::default(),
        player_combat_reach: 0.0,
        player_detected_range_aura_mod: 0.0,
        player_level: 80,
        player_gray_level: 70,
        player_liquid_status_like_cpp: 0,
        player_unit_flags: UnitFlags::PLAYER_CONTROLLED.bits(),
        player_unit_flags2: 0,
        player_unit_state: 0,
        player_is_game_master: false,
        player_is_contested_pvp: false,
        player_faction_template_id: 1,
        player_reputation_standings: Vec::new(),
        player_reputation_state_flags: Vec::new(),
        player_forced_reputation_ranks: Vec::new(),
        player_forced_reputation_faction_ids: Vec::new(),
        player_school_immunity_mask: 0,
        player_damage_immunity_mask: 0,
        player_has_confuse_aura: false,
        player_has_breakable_stun_aura: false,
    }
}

pub(in crate::session::tests) fn legacy_aggro_relation_config_like_cpp(
    creature_faction_template: wow_data::progression_rewards::FactionTemplateEntry,
    player_faction_template: wow_data::progression_rewards::FactionTemplateEntry,
    creature_faction: FactionEntry,
) -> LegacyCreatureAggroConfigLikeCpp {
    LegacyCreatureAggroConfigLikeCpp {
        faction_template_store: Some(Arc::new(
            wow_data::progression_rewards::FactionTemplateStore::from_entries([
                creature_faction_template,
                player_faction_template,
            ]),
        )),
        faction_store: Some(Arc::new(FactionStore::from_entries([creature_faction]))),
        ..Default::default()
    }
}

pub(in crate::session::tests) fn legacy_aggro_hostile_config_like_cpp()
-> LegacyCreatureAggroConfigLikeCpp {
    legacy_aggro_relation_config_like_cpp(
        faction_template_entry(14, 72, 0, 0, 930),
        faction_template_entry(1, 930, 0, 0, 0),
        FactionEntry::for_test_like_cpp(72, 1),
    )
}

pub(in crate::session::tests) fn legacy_aggro_hostile_config_with_rate_like_cpp(
    creature_aggro_rate: f32,
) -> LegacyCreatureAggroConfigLikeCpp {
    LegacyCreatureAggroConfigLikeCpp {
        creature_aggro_rate,
        ..legacy_aggro_hostile_config_like_cpp()
    }
}

pub(in crate::session::tests) fn represented_creature_spell_test_attributes_like_cpp(
    no_attack_miss: bool,
) -> [u32; 15] {
    let mut attributes = [0; 15];
    attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_IS_ABILITY;
    if no_attack_miss {
        attributes[7] = 0x0200_0000; // SPELL_ATTR7_NO_ATTACK_MISS
    }
    attributes
}

pub(in crate::session::tests) fn spell_misc_entry_like_cpp(
    id: u32,
    spell_id: u32,
    range_index: u16,
) -> wow_data::SpellMiscEntry {
    wow_data::SpellMiscEntry {
        id,
        attributes: represented_creature_spell_test_attributes_like_cpp(true)
            .map(|attribute| attribute as i32),
        difficulty_id: 0,
        casting_time_index: 0,
        duration_index: 0,
        range_index,
        school_mask: 0x01,
        speed: 0.0,
        launch_delay: 0.0,
        min_duration: 0.0,
        spell_icon_file_data_id: 0,
        active_icon_file_data_id: 0,
        content_tuning_id: 0,
        show_future_spell_player_condition_id: 0,
        spell_id,
    }
}

pub(in crate::session::tests) fn spell_range_entry_like_cpp(
    id: u32,
    range_min: f32,
    range_max: f32,
) -> wow_data::SpellRangeEntry {
    wow_data::SpellRangeEntry {
        id,
        display_name: String::new(),
        display_name_short: String::new(),
        flags: 0,
        range_min: [range_min, range_min],
        range_max: [range_max, range_max],
    }
}

pub(in crate::session::tests) fn creature_ai_test_spell_info_like_cpp(
    spell_id: i32,
    implicit_target_1: u32,
    implicit_target_2: u32,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        // SpellStore hydrates GetRecoveryTime=max(RecoveryTime,
        // CategoryRecoveryTime). CombatAI must still use raw 6s below.
        recovery_time_ms: 12_000,
        effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
        effect_base_points: 7,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![wow_data::SpellEffectInfo {
            effect_index: 0,
            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
            effect_base_points: 7,
            implicit_target_1,
            implicit_target_2,
            ..Default::default()
        }],
    }
}

pub(in crate::session::tests) fn creature_ai_spell_test_config_like_cpp(
    spell: wow_data::SpellInfo,
    passive: bool,
    range_max: f32,
) -> LegacyCreatureAggroConfigLikeCpp {
    let spell_id = u32::try_from(spell.spell_id).unwrap();
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(spell.spell_id, spell);
    let mut misc = spell_misc_entry_like_cpp(8_001, spell_id, 71);
    if passive {
        misc.attributes[0] |= wow_data::spell::attributes::SPELL_ATTR0_PASSIVE as i32;
    }
    spell_store.insert_spell_misc_attributes_like_cpp(
        i32::try_from(spell_id).unwrap(),
        misc.attributes.map(|attribute| attribute as u32),
    );
    spell_store.insert_spell_hit_metadata_for_difficulty_like_cpp(
        i32::try_from(spell_id).unwrap(),
        0,
        wow_data::SpellHitMetadataLikeCpp {
            category_id: 0,
            charge_category_id: 0,
            defense_type: 2,
            spell_mechanic: 0,
            school_mask: 0x01,
            effect_mechanics: BTreeMap::from([(0, 0)]),
        },
    );
    LegacyCreatureAggroConfigLikeCpp {
        spell_misc_store: Some(Arc::new(wow_data::SpellMiscStore::from_entries([misc]))),
        spell_range_store: Some(Arc::new(wow_data::SpellRangeStore::from_entries([
            spell_range_entry_like_cpp(71, 0.0, range_max),
        ]))),
        spell_cooldowns_store: Some(Arc::new(wow_data::SpellCooldownsStore::from_entries([
            wow_data::SpellCooldownsEntry {
                id: 8_002,
                difficulty_id: 0,
                category_recovery_time: 12_000,
                recovery_time: 6_000,
                start_recovery_time: 0,
                spell_id,
            },
        ]))),
        spell_category_store: Some(Arc::new(SpellCategoryStore::from_entries([]))),
        spell_x_spell_visual_store: Some(Arc::new(wow_data::SpellXSpellVisualStore::from_entries(
            [wow_data::SpellXSpellVisualEntry {
                id: 8_003,
                difficulty_id: 0,
                spell_visual_id: 99_999,
                probability: 1.0,
                flags: 0,
                priority: 0,
                spell_icon_file_id: 0,
                active_icon_file_id: 0,
                viewer_unit_condition_id: 0,
                viewer_player_condition_id: 0,
                caster_unit_condition_id: 0,
                caster_player_condition_id: 0,
                spell_id,
            }],
        ))),
        spell_target_restrictions_store: Some(Arc::new(
            wow_data::SpellTargetRestrictionsStore::from_entries([]),
        )),
        spell_casting_requirements_store: Some(Arc::new(
            wow_data::SpellCastingRequirementsStore::from_entries([]),
        )),
        spell_aura_restrictions_store: Some(Arc::new(
            wow_data::SpellAuraRestrictionsStore::from_entries([]),
        )),
        spell_store: Some(Arc::new(spell_store)),
        spell_chain_store: Some(Arc::new(SpellChainStoreLikeCpp::default())),
        spell_linked_store: Some(Arc::new(SpellLinkedStoreLikeCpp::default())),
        spell_condition_store: Some(Arc::new(ConditionEntriesByTypeStore::default())),
        spell_script_exact_spell_ids_like_cpp: Some(Arc::new(BTreeSet::new())),
        spell_script_all_rank_root_spell_ids_like_cpp: Some(Arc::new(BTreeSet::new())),
        legacy_spell_script_spell_ids_like_cpp: Some(Arc::new(BTreeSet::new())),
        spell_linked_rejected_trigger_spell_ids_like_cpp: Some(Arc::new(BTreeSet::new())),
        ..legacy_aggro_hostile_config_like_cpp()
    }
}

pub(in crate::session::tests) fn creature_spell_bound_hook_tick_fixture_like_cpp(
    ai_name: &str,
    passive: bool,
    spell_id: i32,
    creature_counter: i64,
    victim_counter: i64,
) -> (
    crate::map_manager::SharedMapManager,
    SharedCanonicalMapManager,
    LegacyCreatureAggroConfigLikeCpp,
    ObjectGuid,
) {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(creature_counter);
    let victim_guid = ObjectGuid::create_player(1, victim_counter);
    add_canonical_creature_spell_test_pair_like_cpp(&canonical, creature_guid, victim_guid);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_identity_names_runtime_like_cpp(ai_name, String::new());
            creature
                .creature
                .set_spell(0, u32::try_from(spell_id).unwrap());
            creature.enter_combat(victim_guid);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let mut config = creature_ai_spell_test_config_like_cpp(
        creature_ai_test_spell_info_like_cpp(spell_id, 6, 0),
        passive,
        30.0,
    );
    config.spell_script_exact_spell_ids_like_cpp =
        Some(Arc::new(BTreeSet::from([u32::try_from(spell_id).unwrap()])));
    (manager, canonical, config, creature_guid)
}

pub(in crate::session::tests) fn add_canonical_creature_spell_test_pair_like_cpp(
    canonical: &SharedCanonicalMapManager,
    creature_guid: ObjectGuid,
    victim_guid: ObjectGuid,
) {
    add_canonical_creature_spell_test_pair_with_difficulty_like_cpp(
        canonical,
        creature_guid,
        victim_guid,
        0,
    );
}

pub(in crate::session::tests) fn add_canonical_creature_spell_test_pair_with_difficulty_like_cpp(
    canonical: &SharedCanonicalMapManager,
    creature_guid: ObjectGuid,
    victim_guid: ObjectGuid,
    difficulty_id: u8,
) {
    add_canonical_test_player_on_map_with_difficulty(
        canonical,
        victim_guid,
        Position::new(11.0, 10.0, 0.0, 0.0),
        0,
        0,
        difficulty_id,
    );
    add_canonical_test_creature_on_map(
        canonical,
        creature_guid,
        9_001,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
        0,
    );
    let mut manager = canonical.lock().unwrap();
    let map = manager.find_map_mut(0, 0).unwrap().map_mut();
    let creature = map.get_typed_creature_mut(creature_guid).unwrap();
    creature.unit_mut().set_faction(14);
    creature
        .unit_mut()
        .subsystems_mut()
        .auras
        .set_spell_cast_log_aura_authority_inert_like_cpp(true);
    let player = map.get_typed_player_mut(victim_guid).unwrap();
    player.unit_mut().set_faction(1);
    player.unit_mut().set_level(80);
    player.unit_mut().set_max_health(100);
    player.unit_mut().set_health(100);
    player
        .unit_mut()
        .set_death_state(wow_constants::DeathState::Alive);
    player
        .unit_mut()
        .subsystems_mut()
        .auras
        .set_spell_hit_aura_authority_inert_like_cpp(true);
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::session::tests) struct CreatureSpellWireHeaderLikeCpp {
    pub(in crate::session::tests) opcode: u16,
    pub(in crate::session::tests) caster: ObjectGuid,
    pub(in crate::session::tests) caster_unit: ObjectGuid,
    pub(in crate::session::tests) cast_id: ObjectGuid,
    pub(in crate::session::tests) original_cast_id: ObjectGuid,
    pub(in crate::session::tests) spell_id: i32,
    pub(in crate::session::tests) spell_x_spell_visual_id: u32,
    pub(in crate::session::tests) cast_flags: u32,
    pub(in crate::session::tests) cast_flags_ex: u32,
    pub(in crate::session::tests) cast_time_ms: u32,
    pub(in crate::session::tests) target_flags: u32,
    pub(in crate::session::tests) target_unit: ObjectGuid,
    pub(in crate::session::tests) hit_targets: Vec<ObjectGuid>,
    pub(in crate::session::tests) miss_targets: Vec<(ObjectGuid, u8)>,
}

pub(in crate::session::tests) fn decode_creature_spell_wire_header_and_tail_like_cpp(
    bytes: &[u8],
) -> (CreatureSpellWireHeaderLikeCpp, WorldPacket) {
    let mut packet = WorldPacket::from_bytes(bytes);
    let opcode = packet.opcode_raw();
    packet.skip_opcode();
    let caster = packet.read_packed_guid().unwrap();
    let caster_unit = packet.read_packed_guid().unwrap();
    let cast_id = packet.read_packed_guid().unwrap();
    let original_cast_id = packet.read_packed_guid().unwrap();
    let spell_id = packet.read_int32().unwrap();
    let spell_x_spell_visual_id = packet.read_uint32().unwrap();
    let cast_flags = packet.read_uint32().unwrap();
    let cast_flags_ex = packet.read_uint32().unwrap();
    let cast_time_ms = packet.read_uint32().unwrap();
    assert_eq!(packet.read_int32().unwrap(), 0); // missile travel time
    assert_eq!(packet.read_float().unwrap(), 0.0); // missile pitch
    assert_eq!(packet.read_uint8().unwrap(), 0); // destination cast index
    assert_eq!(packet.read_uint32().unwrap(), 0); // immunity school
    assert_eq!(packet.read_uint32().unwrap(), 0); // immunity value
    assert_eq!(packet.read_uint32().unwrap(), 0); // heal prediction
    assert_eq!(packet.read_uint8().unwrap(), 0); // prediction type
    assert_eq!(packet.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    let hit_count = packet.read_bits(16).unwrap() as usize;
    let miss_count = packet.read_bits(16).unwrap() as usize;
    assert_eq!(packet.read_bits(16).unwrap() as usize, miss_count);
    assert_eq!(packet.read_bits(9).unwrap(), 0); // remaining power
    assert!(!packet.read_bit().unwrap()); // remaining runes
    assert_eq!(packet.read_bits(16).unwrap(), 0); // target points
    assert!(!packet.read_bit().unwrap()); // ammo display
    assert!(!packet.read_bit().unwrap()); // ammo inventory type
    let target = wow_packet::packets::spell::SpellTargetData::read(&mut packet).unwrap();
    let hit_targets = (0..hit_count)
        .map(|_| packet.read_packed_guid().unwrap())
        .collect();
    let miss_guids: Vec<_> = (0..miss_count)
        .map(|_| packet.read_packed_guid().unwrap())
        .collect();
    let miss_targets = miss_guids
        .into_iter()
        .map(|guid| {
            let reason = packet.read_uint8().unwrap();
            if reason == wow_packet::packets::spell::SpellMissReason::Reflect as u8 {
                let _reflect_status = packet.read_uint8().unwrap();
            }
            (guid, reason)
        })
        .collect();
    let header = CreatureSpellWireHeaderLikeCpp {
        opcode,
        caster,
        caster_unit,
        cast_id,
        original_cast_id,
        spell_id,
        spell_x_spell_visual_id,
        cast_flags,
        cast_flags_ex,
        cast_time_ms,
        target_flags: target.flags,
        target_unit: target.unit,
        hit_targets,
        miss_targets,
    };
    (header, packet)
}

pub(in crate::session::tests) fn decode_creature_spell_wire_header_like_cpp(
    bytes: &[u8],
) -> CreatureSpellWireHeaderLikeCpp {
    decode_creature_spell_wire_header_and_tail_like_cpp(bytes).0
}

pub(in crate::session::tests) fn decode_creature_spell_full_log_like_cpp(
    bytes: &[u8],
) -> wow_packet::packets::spell::SpellCastLogData {
    use wow_packet::packets::spell::{SpellCastLogData, SpellLogPowerData};

    let (_, mut packet) = decode_creature_spell_wire_header_and_tail_like_cpp(bytes);
    assert!(packet.read_bit().expect("full combat-log presence bit"));
    let health = packet.read_int64().expect("combat-log health");
    let attack_power = packet.read_int32().expect("combat-log attack power");
    let spell_power = packet.read_int32().expect("combat-log spell power");
    let armor = packet.read_int32().expect("combat-log armor");
    let power_count = packet.read_bits(9).expect("combat-log power count") as usize;
    let power_data = (0..power_count)
        .map(|_| SpellLogPowerData {
            power_type: packet.read_int32().expect("combat-log power type"),
            amount: packet.read_int32().expect("combat-log power amount"),
            cost: packet.read_int32().expect("combat-log power cost"),
        })
        .collect();
    assert!(packet.is_empty());
    SpellCastLogData {
        health,
        attack_power,
        spell_power,
        armor,
        power_data,
    }
}

pub(in crate::session::tests) fn decode_atomic_creature_spell_wire_pair_like_cpp(
    event: &RuntimeEvent,
) -> (
    CreatureSpellWireHeaderLikeCpp,
    CreatureSpellWireHeaderLikeCpp,
) {
    let basic_go_packet_bytes = match &event.recipients {
        crate::map_manager::RecipientRule::NearbyVisibleDurableSpellCast {
            basic_go_packet_bytes,
            ..
        } => basic_go_packet_bytes,
        _ => panic!("creature spell START/GO must use the atomic cast recipient rule"),
    };
    (
        decode_creature_spell_wire_header_like_cpp(&event.packet_bytes),
        decode_creature_spell_wire_header_like_cpp(basic_go_packet_bytes),
    )
}
pub(in crate::session::tests) fn creature_melee_sync_state_for_test_like_cpp(
    victim: &crate::map_manager::WorldCreature,
    applied_damage: u32,
) -> CreatureMeleeVictimSyncStateLikeCpp {
    let mut canonical = victim.clone();
    let identity_authority = canonical.creature.loot_authority_like_cpp().clone();
    let identity_health_authority = canonical
        .creature
        .unit()
        .health_state_revision_authority_like_cpp();
    let health_before = canonical.creature.unit().data().health;
    let revision_before = canonical.creature.unit().health_state_revision_like_cpp();
    let lifecycle_before = canonical.creature.loot_lifecycle_revision_like_cpp();
    let death_before = canonical.creature.unit().death_state();
    let ai_before = canonical.creature.ai_ownership().state;
    let killed = canonical.take_damage_before_death_state_at_game_time_like_cpp(
        applied_damage,
        wow_entities::game_time_secs_like_cpp(),
    );
    if killed {
        canonical.complete_death_state_after_kill_hooks_like_cpp();
    }
    CreatureMeleeVictimSyncStateLikeCpp {
        applied_damage,
        threat: None,
        victim_health_before: health_before,
        victim_health_after: canonical.creature.unit().data().health,
        victim_health_state_revision_before: revision_before,
        victim_health_state_revision_after: canonical
            .creature
            .unit()
            .health_state_revision_like_cpp(),
        identity: CreatureMeleeVictimSyncIdentityLikeCpp {
            authority: identity_authority,
            health_state_revision_authority: identity_health_authority,
            spawn_id: canonical.creature.spawn_id(),
            loot_lifecycle_revision_before: lifecycle_before,
            loot_lifecycle_revision_after: canonical.creature.loot_lifecycle_revision_like_cpp(),
            death_state_before: death_before,
            death_state_after: canonical.creature.unit().death_state(),
            ai_state_before: ai_before,
            ai_state_after: canonical.creature.ai_ownership().state,
        },
    }
}
