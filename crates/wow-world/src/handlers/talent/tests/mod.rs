//! Talent handlers regression scenarios.
//!
//! Separated from the talent.rs root under #662.

use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use super::*;
use crate::session::{
    RepresentedAtLoginFlagRemovalLikeCpp, RepresentedTalentResetScriptHookLikeCpp,
};
use wow_constants::BuyResult;
use wow_constants::unit::UnitState;
use wow_core::guid::HighGuid;
use wow_core::{ObjectGuid, Position};
use wow_entities::{PetStable, PetStableInfo, PetType};
use wow_packet::ServerPacket;
use wow_packet::packets::chat::PrintNotification;
use wow_packet::packets::misc::BuyFailed;
use wow_packet::packets::update::CreatureCreateData;

use crate::session::{
    AuraApplication, RepresentedAuraEffectLikeCpp, RepresentedTalentRespecCriteriaEventLikeCpp,
    SPELL_AURA_INTERRUPT_FLAG2_CHANGE_TALENT_LIKE_CPP, SessionPlayerController,
};

fn make_session_with_send_capacity(capacity: usize) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(capacity.max(1024));
    (
        WorldSession::new(
            1,
            "TestAccount".into(),
            0,
            2,
            9,
            54261,
            vec![0u8; 40],
            "esES".into(),
            pkt_rx,
            send_tx,
        ),
        send_rx,
    )
}

fn drain_sent_packets(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<Vec<u8>> {
    let mut packets = Vec::new();
    while let Ok(packet) = send_rx.try_recv() {
        packets.push(packet);
    }
    packets
}

fn confirm_respec_wipe_packet(respec_master: ObjectGuid, respec_type: u8) -> WorldPacket {
    let mut packet = WorldPacket::new_empty();
    packet.write_guid(&respec_master);
    packet.write_uint8(respec_type);
    packet
}

fn learn_talent_packet(talent_id: i32, requested_rank: u16) -> WorldPacket {
    let mut packet = WorldPacket::new_empty();
    packet.write_int32(talent_id);
    packet.write_uint16(requested_rank);
    packet
}

fn learn_talents_packet(talent_ids: &[u16]) -> WorldPacket {
    let mut packet = WorldPacket::new_empty();
    packet.write_bits(talent_ids.len() as u32, 6);
    packet.flush_bits();
    for talent_id in talent_ids {
        packet.write_uint16(*talent_id);
    }
    packet
}

fn test_talent_entry_like_cpp(id: u32, rank: u8, spell_id: i32) -> wow_data::TalentEntry {
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

fn test_spell_info_like_cpp(spell_id: i32) -> wow_data::SpellInfo {
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

fn test_learn_spell_info_like_cpp(spell_id: i32, trigger_spell: i32) -> wow_data::SpellInfo {
    let mut spell = test_spell_info_like_cpp(spell_id);
    spell.effects = vec![wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_SPELL,
        effect_trigger_spell: trigger_spell,
        ..wow_data::SpellEffectInfo::default()
    }];
    spell
}

fn test_quest_template_like_cpp(id: u32) -> wow_data::quest::QuestTemplate {
    wow_data::quest::QuestTemplate {
        id,
        quest_type: 0,
        quest_level: 1,
        quest_max_scaling_level: 0,
        quest_package_id: 0,
        min_level: 1,
        quest_sort_id: 0,
        quest_info_id: 0,
        suggested_group_num: 0,
        reward_next_quest: 0,
        reward_xp_difficulty: 0,
        reward_xp_multiplier: 1.0,
        reward_money_difficulty: 0,
        reward_money_multiplier: 1.0,
        reward_bonus_money: 0,
        reward_display_spell: [0; wow_data::quest::QUEST_REWARD_DISPLAY_SPELL_COUNT],
        reward_spell: 0,
        reward_honor: 0,
        reward_title_id: 0,
        reward_skill_line_id: 0,
        reward_skill_points: 0,
        reward_mail_template_id: 0,
        reward_mail_delay_secs: 0,
        reward_mail_sender_entry: 0,
        reward_faction_ids: [0; wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_values: [0; wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_overrides: [0; wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_cap_in: [0; wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT],
        reward_faction_flags: 0,
        source_item_id: 0,
        source_item_count: 0,
        source_spell_id: 0,
        limit_time_secs: 0,
        expansion: 0,
        flags: 0,
        flags_ex: 0,
        flags_ex2: 0,
        special_flags: 0,
        event_id_for_quest: 0,
        reward_items: [0; wow_data::quest::QUEST_REWARD_ITEM_COUNT],
        reward_amounts: [0; wow_data::quest::QUEST_REWARD_ITEM_COUNT],
        reward_currencies: [0; wow_data::quest::QUEST_REWARD_CURRENCY_COUNT],
        reward_currency_amounts: [0; wow_data::quest::QUEST_REWARD_CURRENCY_COUNT],
        item_drop: [0; wow_data::quest::QUEST_ITEM_DROP_COUNT],
        item_drop_quantity: [0; wow_data::quest::QUEST_ITEM_DROP_COUNT],
        log_title: String::new(),
        log_description: String::new(),
        quest_description: String::new(),
        area_description: String::new(),
        quest_completion_log: String::new(),
        objectives: Vec::new(),
        allowable_races: 0,
        allowable_classes: 0,
        max_level: 0,
        prev_quest_id: 0,
        next_quest_id: 0,
        exclusive_group: 0,
        breadcrumb_for_quest_id: 0,
        dependent_previous_quests: Vec::new(),
        dependent_breadcrumb_quests: Vec::new(),
        required_min_rep_faction: 0,
        required_min_rep_value: 0,
        required_max_rep_faction: 0,
        required_max_rep_value: 0,
        required_skill_id: 0,
        required_skill_points: 0,
        reward_choice_items: [(0, 0); wow_data::quest::QUEST_REWARD_CHOICES_COUNT],
        reward_choice_item_types: [0; wow_data::quest::QUEST_REWARD_CHOICES_COUNT],
    }
}

fn test_rewarded_skill_ability_like_cpp(
    spell_id: i32,
    acquire_method: i8,
) -> wow_data::skill::SkillLineAbilityRecord {
    wow_data::skill::SkillLineAbilityRecord {
        id: 1,
        race_mask: 0,
        skill_line: 777,
        spell: spell_id,
        min_skill_line_rank: 0,
        class_mask: 0,
        supercedes_spell: 0,
        acquire_method,
        trivial_rank_high: 0,
        trivial_rank_low: 0,
        flags: 0,
        num_skill_ups: 0,
        skillup_skill_line_id: 0,
    }
}

fn test_visible_aura_like_cpp(slot: u8, spell_id: i32) -> AuraApplication {
    AuraApplication {
        spell_id,
        difficulty_id: 0,
        caster_guid: ObjectGuid::EMPTY,
        slot,
        duration_total: 0,
        duration_remaining: 0,
        stack_count: 1,
        aura_flags: 0,
        effect_mask: 1,
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

fn install_test_talent_store(
    session: &mut WorldSession,
    talents: &[(u32, u8, i32)],
) -> wow_data::TalentTabStore {
    install_test_talent_store_with_tab_class_mask(session, talents, 1)
}

fn install_test_talent_store_with_tab_class_mask(
    session: &mut WorldSession,
    talents: &[(u32, u8, i32)],
    class_mask: i32,
) -> wow_data::TalentTabStore {
    install_test_talent_entries_with_tab_class_mask(
        session,
        talents
            .iter()
            .map(|(talent_id, rank, spell_id)| {
                test_talent_entry_like_cpp(*talent_id, *rank, *spell_id)
            })
            .collect::<Vec<_>>(),
        class_mask,
    )
}

fn install_test_talent_entries_with_tab_class_mask(
    session: &mut WorldSession,
    talents: Vec<wow_data::TalentEntry>,
    class_mask: i32,
) -> wow_data::TalentTabStore {
    let spell_ids = talents
        .iter()
        .flat_map(|talent| talent.spell_rank)
        .filter(|spell_id| *spell_id > 0)
        .collect::<Vec<_>>();
    session.set_talent_store(Arc::new(wow_data::TalentStore::from_entries(talents)));
    let talent_tabs = wow_data::TalentTabStore::from_entries([wow_data::TalentTabEntry {
        id: 0,
        name: String::new(),
        background_file: String::new(),
        order_index: 0,
        race_mask: 0,
        class_mask,
        pet_talent_mask: 0,
        spell_icon_id: 0,
    }]);
    session.set_player_class_like_cpp(1);
    session.set_player_level_like_cpp(80);
    session.set_num_talents_at_level_store(Arc::new(
        wow_data::progression_rewards::NumTalentsAtLevelStore::from_entries([
            wow_data::progression_rewards::NumTalentsAtLevelEntry {
                id: 80,
                num_talents: 71,
                num_talents_death_knight: 71,
                num_talents_demon_hunter: 71,
            },
        ]),
    ));

    let mut spell_store = wow_data::SpellStore::new();
    for spell_id in spell_ids {
        spell_store.insert(spell_id, test_spell_info_like_cpp(spell_id));
    }
    session.set_spell_store(Arc::new(spell_store));
    talent_tabs
}

fn install_test_talent_entries_with_spell_store_like_cpp(
    session: &mut WorldSession,
    talents: Vec<wow_data::TalentEntry>,
    spell_store: wow_data::SpellStore,
) -> wow_data::TalentTabStore {
    session.set_talent_store(Arc::new(wow_data::TalentStore::from_entries(talents)));
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
    session.set_player_level_like_cpp(80);
    session.set_num_talents_at_level_store(Arc::new(
        wow_data::progression_rewards::NumTalentsAtLevelStore::from_entries([
            wow_data::progression_rewards::NumTalentsAtLevelEntry {
                id: 80,
                num_talents: 71,
                num_talents_death_knight: 71,
                num_talents_demon_hunter: 71,
            },
        ]),
    ));
    session.set_spell_store(Arc::new(spell_store));
    talent_tabs
}

fn install_test_talent_store_without_tab(
    session: &mut WorldSession,
    talents: &[(u32, u8, i32)],
) -> wow_data::TalentTabStore {
    session.set_talent_store(Arc::new(wow_data::TalentStore::from_entries(
        talents.iter().map(|(talent_id, rank, spell_id)| {
            test_talent_entry_like_cpp(*talent_id, *rank, *spell_id)
        }),
    )));
    session.set_player_class_like_cpp(1);
    session.set_player_level_like_cpp(80);
    session.set_num_talents_at_level_store(Arc::new(
        wow_data::progression_rewards::NumTalentsAtLevelStore::from_entries([
            wow_data::progression_rewards::NumTalentsAtLevelEntry {
                id: 80,
                num_talents: 71,
                num_talents_death_knight: 71,
                num_talents_demon_hunter: 71,
            },
        ]),
    ));

    let mut spell_store = wow_data::SpellStore::new();
    for (_, _, spell_id) in talents {
        spell_store.insert(*spell_id, test_spell_info_like_cpp(*spell_id));
    }
    session.set_spell_store(Arc::new(spell_store));
    wow_data::TalentTabStore::from_entries([])
}

fn test_creature_guid(counter: u32) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, counter, 1)
}

fn visible_aura(slot: u8, flags2: u32) -> AuraApplication {
    AuraApplication {
        spell_id: 90_000 + i32::from(slot),
        difficulty_id: 0,
        caster_guid: ObjectGuid::EMPTY,
        slot,
        duration_total: 0,
        duration_remaining: 0,
        stack_count: 1,
        aura_flags: 0,
        effect_mask: 1,
        aura_interrupt_flags: 0,
        aura_interrupt_flags2: flags2,
        represented_effect: None,
        represented_amount: 0,
        represented_effect_amounts: Vec::new(),
        represented_misc_value: None,
        represented_multiplier: 1.0,
        applied_at: std::time::Instant::now(),
    }
}

fn test_creature_create_data(guid: ObjectGuid, npc_flags: u32) -> CreatureCreateData {
    CreatureCreateData {
        guid,
        entry: 123,
        display_id: 100,
        native_display_id: 100,
        display_scale: 1.0,
        native_x_display_scale: 1.0,
        bounding_radius: 0.389,
        combat_reach: 1.5,
        health: 100,
        max_health: 100,
        level: 60,
        faction_template: 35,
        npc_flags: u64::from(npc_flags),
        unit_flags: 0,
        unit_flags2: 0,
        unit_flags3: 0,
        aura_state: 0x00D0_0000, // full-HP creature, mirrors C++ ModifyAuraState
        damage_school: wow_constants::spell::SpellSchools::Normal as u8,
        scale: 1.0,
        unit_class: 1,
        display_power: 1,
        power: [0; 10],
        max_power: [0; 10],
        base_mana: 0,
        virtual_items: [(0, 0, 0); 3],
        base_attack_time: 2000,
        ranged_attack_time: 0,
        movement_flags: 0,
        vehicle_id: 0,
        play_hover_anim: false,
        hover_height: 1.0,
        mount_display_id: 0,
        stand_state: 0,
        vis_flags: 0,
        anim_tier: 0,
        emote_state: 0,
        sheathe_state: wow_constants::unit::SheathState::Melee as u8,
        pvp_flags: 0,
        current_area_id: 0,
        speed_walk_rate: 1.0,
        speed_run_rate: 1.14286,
        ai_anim_kit_id: 0,
        movement_anim_kit_id: 0,
        melee_anim_kit_id: 0,
    }
}

fn register_test_trainer(session: &mut WorldSession, guid: ObjectGuid, npc_flags: u32) {
    register_test_trainer_with_class(session, guid, npc_flags, 1);
}

fn register_test_trainer_with_class(
    session: &mut WorldSession,
    guid: ObjectGuid,
    npc_flags: u32,
    trainer_class: u8,
) {
    session.set_map_manager(Arc::new(RwLock::new(crate::map_manager::MapManager::new())));
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 80, 0);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_player_position_like_cpp(Position::new(0.0, 0.0, 0.0, 0.0));
    session.register_world_creature(
        0,
        Position::new(1.0, 0.0, 0.0, 0.0),
        test_creature_create_data(guid, npc_flags),
        1,
        2,
        5.0,
        0,
        0,
        0,
        0,
        None,
        0,
        0,
        0,
        0,
        0,
    );
    session
        .mutate_world_creature(guid, |creature| {
            creature
                .creature
                .set_trainer_class_runtime_like_cpp(trainer_class);
        })
        .expect("test trainer exists");
}

fn add_canonical_test_trainer_like_cpp(
    canonical: &Arc<Mutex<wow_map::MapManager>>,
    guid: ObjectGuid,
    position: Position,
    npc_flags: u32,
    trainer_class: u8,
) {
    let mut creature = wow_entities::Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().object_mut().set_entry(123);
    creature.unit_mut().world_mut().set_map(0, 0).unwrap();
    creature.unit_mut().world_mut().relocate(position);
    creature.unit_mut().world_mut().set_combat_reach(1.0);
    creature.unit_mut().set_level(80);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.unit_mut().set_faction(35);
    creature.set_ai_identity_runtime(1, 35, npc_flags, 0);
    creature.set_trainer_class_runtime_like_cpp(trainer_class);
    creature.unit_mut().world_mut().object_mut().add_to_world();

    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .expect("canonical map")
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}

fn add_canonical_test_pet_like_cpp(
    canonical: &Arc<Mutex<wow_map::MapManager>>,
    guid: ObjectGuid,
    owner_guid: ObjectGuid,
    position: Position,
) {
    let mut pet = wow_entities::Pet::new(owner_guid, PetType::Hunter);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(guid);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(500);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .set_map(0, 0)
        .unwrap();
    pet.creature_mut().unit_mut().world_mut().relocate(position);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();

    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .expect("canonical map")
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_pet(pet).unwrap())
        .unwrap();
}

fn represented_current_pet_stable_like_cpp(pet_number: u32) -> PetStable {
    PetStable {
        current_pet_index: Some(0),
        active_pets: vec![Some(PetStableInfo {
            pet_number,
            creature_id: 500,
            pet_type: PetType::Hunter,
            health: 100,
            level: 80,
            ..PetStableInfo::default()
        })],
        stabled_pets: Vec::new(),
        unslotted_pets: Vec::new(),
    }
}

mod scenarios_1;
mod scenarios_2;
