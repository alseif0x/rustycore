//! Behaviour tests for [`super`].
//!
//! Extracted from `player.rs`. Moving tests moves no invariant: the
//! production module boundary, its visibility and its owners are untouched.
//!
//! Dedenting by one level lets rustfmt collapse some argument lists onto a single
//! line, which drops their trailing commas; that is the only difference from the
//! original text.

#![cfg(test)]

use super::*;
use wow_constants::{
    BagFamilyMask, InventoryResult, InventoryType, ItemBondingType, ItemClass, ItemContext,
    ItemFieldFlags, ItemSubClassContainer, ItemSubclassProfession,
};

fn can_store_args<'a>(
    bag: u8,
    slot: u8,
    proto: Option<&'a ItemStorageTemplate>,
    count: u32,
) -> CanStoreItemArgs<'a> {
    CanStoreItemArgs {
        bag,
        slot,
        entry: proto.map_or(0, |proto| proto.entry),
        count,
        proto,
        source_item: None,
        source_is_not_empty_bag: false,
        source_bop_trade_allowed_for_player: false,
        swap: false,
        limit_category: None,
        slot_items: &[],
        stored_items: &[],
        bag_templates: &[],
    }
}

fn item_with_guid_entry(low: i64, entry: u32) -> Item {
    let mut item = Item::default();
    item.object_mut().create(ObjectGuid::create_item(1, low));
    item.object_mut().set_entry(entry);
    item
}

struct StubPowerResolver;

impl PlayerPowerIndexResolver for StubPowerResolver {
    fn power_index_by_class(&self, power: PowerType, class_id: u8) -> Option<usize> {
        if class_id != CLASS_PALADIN {
            return None;
        }
        match power {
            PowerType::Mana => Some(0),
            PowerType::Energy => Some(3),
            PowerType::ComboPoints => Some(9),
            PowerType::AlternateMount => Some(MAX_POWERS_PER_CLASS),
            _ => None,
        }
    }
}

#[test]
fn player_power_index_resolver_configures_runtime_mapping_without_update_masks() {
    let mut player = Player::new(None, false);
    player.set_race_class_gender(1, CLASS_PALADIN, Gender::Male);
    player.set_power_index(PowerType::Focus, Some(4));
    player.clear_data_changes();

    player.configure_power_indices_for_class(&StubPowerResolver);

    assert_eq!(player.get_power_index(PowerType::Mana), Some(0));
    assert_eq!(player.get_power_index(PowerType::Energy), Some(3));
    assert_eq!(player.get_power_index(PowerType::ComboPoints), Some(9));
    assert_eq!(player.get_power_index(PowerType::Focus), None);
    assert_eq!(player.get_power_index(PowerType::AlternateMount), None);
    assert!(!player.unit().unit_data_changes_mask().is_any_set());
    assert!(!player.player_data_changes_mask().is_any_set());
    assert!(!player.active_player_data_changes_mask().is_any_set());
}

fn lifecycle_create_record() -> PlayerCreateLifecycleRecord {
    PlayerCreateLifecycleRecord {
        guid: ObjectGuid::create_player(1, 42),
        name: "Lifecycle".to_string(),
        race: 1,
        class_id: CLASS_PALADIN,
        gender: Gender::Female,
        level: 12,
        xp: 345,
        money: 678,
        inventory_slot_count: INVENTORY_DEFAULT_SIZE,
        bank_bag_slot_count: 2,
        map_id: 571,
        position: Position::new(1.0, 2.0, 3.0, 4.0),
        max_health: 1000,
        health: 750,
        powers: vec![
            PlayerLifecyclePower::new(PowerType::Mana, 400, 900),
            PlayerLifecyclePower::new(PowerType::Energy, 40, 100),
            PlayerLifecyclePower::new(PowerType::Focus, 99, 100),
        ],
        display_power: PowerType::Mana,
        faction_template: Some(35),
        display_id: Some(1234),
        player_flags: 0x10,
        player_flags_ex: 0x20,
        extra_flags: 0x40,
        create_time: Some(1_700_000_000),
        create_mode: Some(0),
        played_time_total: 11,
        played_time_level: 7,
        active_talent_group: Some(0),
    }
}

fn assert_player_lifecycle_is_clean(player: &Player) {
    assert_eq!(player.unit().changed_object_type_mask(), 0);
    assert!(!player.unit().unit_data_changes_mask().is_any_set());
    assert!(!player.player_data_changes_mask().is_any_set());
    assert!(!player.active_player_data_changes_mask().is_any_set());
}

#[test]
fn player_lifecycle_create_initializes_representable_state_as_clean_baseline() {
    let record = lifecycle_create_record();
    let player = Player::create_from_lifecycle(Some(9), true, record.clone(), &StubPowerResolver);

    assert_eq!(player.guid(), record.guid);
    assert_eq!(player.session_id(), Some(9));
    assert_eq!(player.unit().world().name(), "Lifecycle");
    assert_eq!(player.unit().world().map_id(), record.map_id);
    assert_eq!(player.unit().world().position(), record.position);
    assert_eq!(player.unit().world().object().scale(), 1.0);
    assert_eq!(player.unit().data().race, record.race);
    assert_eq!(player.unit().data().class_id, record.class_id);
    assert_eq!(player.unit().data().player_class_id, record.class_id);
    assert_eq!(player.unit().data().sex, Gender::Female as u8);
    assert_eq!(player.unit().data().level, i32::from(record.level));
    assert_eq!(player.unit().data().faction_template, 35);
    assert_eq!(player.unit().data().display_id, 1234);
    assert_eq!(player.unit().data().native_display_id, 1234);
    assert_eq!(player.unit().data().display_power, PowerType::Mana as u8);
    assert_eq!(player.unit().data().max_health, record.max_health);
    assert_eq!(player.unit().data().health, record.health);
    assert_eq!(player.active_data().xp, record.xp);
    assert_eq!(player.active_data().coinage, record.money);
    assert_eq!(
        player.active_data().num_backpack_slots,
        record.inventory_slot_count
    );
    assert_eq!(player.data().num_bank_slots, record.bank_bag_slot_count);
    assert_eq!(player.data().player_flags, record.player_flags);
    assert_eq!(player.data().player_flags_ex, record.player_flags_ex);
    assert_eq!(player.extra_flags(), record.extra_flags);
    assert_eq!(player.get_power_index(PowerType::Mana), Some(0));
    assert_eq!(player.get_power(PowerType::Mana), 400);
    assert_eq!(player.get_max_power(PowerType::Mana), 900);
    assert_eq!(player.get_power_index(PowerType::Energy), Some(3));
    assert_eq!(player.get_power(PowerType::Energy), 40);
    assert_eq!(player.get_power_index(PowerType::Focus), None);
    assert_eq!(player.get_power(PowerType::Focus), 0);
    assert_eq!(
        player.lifecycle_metadata(),
        PlayerLifecycleMetadata {
            account_id: None,
            create_time: record.create_time,
            create_mode: record.create_mode,
            played_time_total: record.played_time_total,
            played_time_level: record.played_time_level,
            active_talent_group: record.active_talent_group,
            zone_id: None,
        }
    );
    assert_player_lifecycle_is_clean(&player);
}

#[test]
fn player_lifecycle_load_from_db_initializes_loaded_state_as_clean_baseline() {
    let create = lifecycle_create_record();
    let record = PlayerDbLoadLifecycleRecord {
        guid: create.guid,
        account_id: 77,
        name: create.name,
        race: create.race,
        class_id: create.class_id,
        gender: create.gender,
        level: create.level,
        xp: create.xp,
        money: create.money,
        inventory_slot_count: create.inventory_slot_count,
        bank_bag_slot_count: create.bank_bag_slot_count,
        map_id: create.map_id,
        position: create.position,
        max_health: create.max_health,
        health: create.health,
        powers: create.powers,
        display_power: create.display_power,
        faction_template: create.faction_template,
        display_id: create.display_id,
        player_flags: create.player_flags,
        player_flags_ex: create.player_flags_ex,
        extra_flags: create.extra_flags,
        create_time: create.create_time,
        create_mode: create.create_mode,
        played_time_total: 123,
        played_time_level: 45,
        active_talent_group: Some(1),
        zone_id: Some(67),
    };

    let player = Player::load_from_db_lifecycle(None, false, record, &StubPowerResolver);

    assert_eq!(player.lifecycle_metadata().account_id, Some(77));
    assert_eq!(player.lifecycle_metadata().zone_id, Some(67));
    assert_eq!(player.lifecycle_metadata().played_time_total, 123);
    assert_eq!(player.lifecycle_metadata().played_time_level, 45);
    assert_eq!(player.lifecycle_metadata().active_talent_group, Some(1));
    assert_eq!(player.get_power(PowerType::Mana), 400);
    assert_player_lifecycle_is_clean(&player);
}

#[test]
fn player_lifecycle_login_plan_keeps_trinity_phase_ordering() {
    let plan = PlayerLoginLifecyclePlan::trinity_handle_player_login();

    assert!(plan.occurs_before(
        PlayerLoginLifecycleStep::LoadFromDb,
        PlayerLoginLifecycleStep::SendInitialPacketsBeforeAddToMap,
    ));
    assert!(plan.occurs_before(
        PlayerLoginLifecycleStep::SendInitialPacketsBeforeAddToMap,
        PlayerLoginLifecycleStep::AddPlayerToMap,
    ));
    assert!(plan.occurs_before(
        PlayerLoginLifecycleStep::AddPlayerToMap,
        PlayerLoginLifecycleStep::SendInitialPacketsAfterAddToMap,
    ));
    assert!(plan.occurs_before(
        PlayerLoginLifecycleStep::SendInitialPacketsAfterAddToMap,
        PlayerLoginLifecycleStep::BootstrapVisibility,
    ));
    assert!(plan.occurs_before(
        PlayerLoginLifecycleStep::AddPlayerToMap,
        PlayerLoginLifecycleStep::SendZoneWorldStates,
    ));
    assert!(plan.occurs_before(
        PlayerLoginLifecycleStep::SendMovementCompoundState,
        PlayerLoginLifecycleStep::MarkOnline,
    ));
}

#[test]
fn player_lifecycle_world_insertion_state_marks_visibility_after_add() {
    let plan = PlayerLoginLifecyclePlan::trinity_handle_player_login();
    let add_index = plan
        .position_of(PlayerLoginLifecycleStep::AddPlayerToMap)
        .unwrap();
    let after_index = plan
        .position_of(PlayerLoginLifecycleStep::SendInitialPacketsAfterAddToMap)
        .unwrap();
    let add_only = PlayerWorldInsertionState::from_completed_steps(&plan.steps()[..=add_index]);
    let after_add =
        PlayerWorldInsertionState::from_completed_steps(&plan.steps()[..=after_index + 2]);

    assert!(add_only.added_to_map);
    assert!(!add_only.visibility_bootstrapped);
    assert!(!add_only.worldstates_sent);
    assert!(after_add.added_to_map);
    assert!(after_add.object_accessor_registered);
    assert!(after_add.visibility_bootstrapped);
    assert!(after_add.worldstates_sent);
}

fn player_gameplay_sample_state() -> PlayerGameplayState {
    PlayerGameplayState {
        quests: PlayerQuestGameplayState {
            statuses: BTreeMap::from([(
                100,
                PlayerQuestStatusRecord {
                    quest_id: 100,
                    status: 3,
                    explored: true,
                    accept_time_secs: 1_700_000_000,
                    end_time_secs: 1_700_000_100,
                    objective_counts: vec![4],
                    slot: 1,
                },
            )]),
            objective_progress: vec![PlayerQuestObjectiveProgress {
                quest_id: 100,
                objective_id: 7,
                counter: 4,
            }],
            rewarded_quest_ids: BTreeSet::from([90]),
            daily_quest_ids: BTreeSet::from([101]),
            weekly_quest_ids: BTreeSet::from([102]),
            monthly_quest_ids: BTreeSet::from([103]),
            seasonal_quests: BTreeMap::from([(1, BTreeMap::from([(104, 0)]))]),
            ..Default::default()
        },
        skills: vec![PlayerSkillRecord {
            skill_line_id: SKILL_PLATE_MAIL,
            current_value: 225,
            max_value: 300,
            step: 2,
            profession_slot: -1,
            state: PlayerSkillLoadState::Unchanged,
        }],
        spells: PlayerSpellRuntimeState {
            known_spells: vec![635],
            rows: std::collections::BTreeMap::from([(
                635,
                PlayerKnownSpellRecord {
                    spell_id: 635,
                    state: PlayerSpellLoadState::Unchanged,
                    active: true,
                    disabled: false,
                    favorite: false,
                    dependent: false,
                },
            )]),
            rows_loaded: true,
            rows_complete: true,
            ..Default::default()
        },
        talents: PlayerTalentRuntimeState {
            talent_groups: [
                std::collections::BTreeMap::from([(42, 1)]),
                Default::default(),
                Default::default(),
                Default::default(),
            ],
            talents_loaded: true,
            glyph_groups: [[0; PLAYER_MAX_GLYPH_SLOTS_LIKE_CPP];
                PLAYER_MAX_SPECIALIZATIONS_LIKE_CPP],
            glyphs_loaded: true,
            ..Default::default()
        },
        action_buttons: vec![PlayerActionButtonRecord {
            button: 1,
            action_id: 635,
            action_type: 0,
        }],
        taxi: PlayerTaxiState {
            known_node_mask: vec![0b0000_0011, 0b1000_0000],
            known_node_mask_text: Some("3 128".to_string()),
            source_node_id: Some(1),
            destination_node_id: Some(2),
            destinations: vec![1, 2, 3],
            ..Default::default()
        },
        social: PlayerSocialState {
            friend_guids: vec![ObjectGuid::create_player(1, 1001)],
            ignore_guids: vec![ObjectGuid::create_player(1, 1002)],
            ..Default::default()
        },
        mails: vec![PlayerMailRecord {
            mail_id: 55,
            message_type: 0,
            sender: 1003,
            receiver: 42,
            template_id: Some(9),
            deliver_time: 1_700_000_000,
            expire_time: 1_700_086_400,
            checked_flags: 0x2,
            stationery_id: 0,
        }],
        group: Some(PlayerGroupState {
            group_guid: ObjectGuid::new(1, 77),
            leader_guid: ObjectGuid::create_player(1, 1001),
            role_mask: 0x1,
            subgroup: 0,
        }),
        guild: PlayerGuildState {
            guild_id: Some(12),
            invited_guild_id: Some(13),
            rank_id: Some(4),
            authority_complete: true,
        },
        battleground: PlayerBattlegroundState {
            queues: vec![PlayerBattlegroundQueueRecord {
                queue_id: 30,
                bracket_id: 4,
                joined_at: 1_700_000_050,
                team_id: TEAM_ALLIANCE_ID,
            }],
            current_bg_instance_id: Some(7001),
            current_bg_team: Some(TEAM_ALLIANCE_ID),
            random: PlayerRandomBattlegroundState {
                reward_claimed_today: true,
                last_reward_time: Some(1_700_000_060),
            },
            ..Default::default()
        },
        reputations: vec![PlayerReputationRecord {
            faction_id: TEAM_ALLIANCE_ID,
            standing: 4_200,
            flags: 0x1,
        }],
        achievements: vec![PlayerAchievementRecord {
            achievement_id: 6,
            completed_at: Some(1_700_000_070),
        }],
        achievement_criteria: vec![PlayerAchievementCriteriaRecord {
            criteria_id: 10,
            counter: 99,
            completed_at: None,
        }],
        currencies: HashMap::from([(
            395,
            crate::PlayerCurrency {
                state: crate::PlayerCurrencyState::Unchanged,
                quantity: 12,
                weekly_quantity: 3,
                tracked_quantity: 20,
                increased_cap_quantity: 0,
                earned_quantity: 0,
                flags: 0,
            },
        )]),
        spell_cooldowns: vec![PlayerSpellCooldownRecord {
            spell_id: 642,
            item_id: None,
            category_id: Some(100),
            cooldown_expires_at: 1_700_000_200,
            category_cooldown_expires_at: Some(1_700_000_150),
        }],
        spell_charges: vec![PlayerSpellChargeRecord {
            category_id: 100,
            consumed_charges: 1,
            recharge_started_at: Some(1_700_000_120),
            recharge_ends_at: Some(1_700_000_180),
        }],
        rest: PlayerRestState {
            rest_xp: 1234,
            rest_bonus: 1.5,
            rest_honor_bonus: 0.25,
            rest_state: 2,
            logout_time: Some(1_699_999_999),
            logout_was_resting: true,
            is_resting_now: true,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[test]
fn action_buttons_are_sorted_player_owned_state_like_cpp() {
    let mut player = Player::new(None, false);

    assert!(player.set_action_button_like_cpp(7, 12_345, 0x80));
    assert!(player.set_action_button_like_cpp(2, 635, 0));
    assert_eq!(
        player.action_button_like_cpp(7),
        Some(12_345 | (0x80 << 24))
    );
    assert_eq!(
        player
            .gameplay_state()
            .action_buttons
            .iter()
            .map(|button| button.button)
            .collect::<Vec<_>>(),
        vec![2, 7]
    );

    assert!(player.set_action_button_like_cpp(7, 0, 0));
    assert_eq!(player.action_button_like_cpp(7), Some(0));
    assert_eq!(player.action_buttons_snapshot_like_cpp()[2], 635);
}

#[test]
fn action_button_load_authority_distinguishes_empty_from_unavailable_like_cpp() {
    let mut player = Player::new(None, false);

    assert!(!player.action_buttons_loaded_like_cpp());
    player.mark_action_buttons_loaded_like_cpp();
    assert!(player.action_buttons_loaded_like_cpp());
    assert_eq!(player.action_buttons_snapshot_like_cpp(), [0; 180]);

    assert!(player.set_action_button_like_cpp(1, 635, 0));
    player.reset_action_buttons_for_load_like_cpp();
    assert!(!player.action_buttons_loaded_like_cpp());
    assert_eq!(player.action_button_like_cpp(1), Some(0));
}

#[test]
fn player_owns_exact_skill_rows_and_persistence_authority_like_cpp() {
    let mut player = Player::new(None, false);
    player.replace_skill_records_like_cpp(
        vec![PlayerSkillRecord {
            skill_line_id: 333,
            current_value: 150,
            max_value: 225,
            step: 2,
            profession_slot: 0,
            state: PlayerSkillLoadState::Changed,
        }],
        true,
        true,
        Some(1),
        BTreeSet::from([755]),
    );

    assert!(player.skill_records_loaded_like_cpp());
    assert!(player.skill_records_complete_like_cpp());
    assert_eq!(player.occupied_skill_slots_like_cpp(), Some(1));
    assert_eq!(
        player.non_durable_skill_tombstones_like_cpp(),
        &BTreeSet::from([755])
    );
    assert_eq!(player.skill_records_like_cpp()[0].profession_slot, 0);
    assert_eq!(
        player.skill_records_like_cpp()[0].state,
        PlayerSkillLoadState::Changed
    );
}

#[test]
fn player_owns_create_form_and_specialization_state_like_cpp() {
    let mut player = Player::new(Some(7), false);

    player.set_create_mode_like_cpp(1);
    player.set_shapeshift_form_id_like_cpp(5);
    player.set_loot_specialization_id_like_cpp(65);
    player.set_primary_specialization(66);

    assert_eq!(player.create_mode_like_cpp(), 1);
    assert_eq!(player.shapeshift_form_id_like_cpp(), 5);
    assert_eq!(player.loot_specialization_id_like_cpp(), 65);
    assert_eq!(player.primary_specialization_id_like_cpp(), 66);
}

#[test]
fn player_gameplay_default_state_is_empty_and_attached_to_new_player() {
    let player = Player::new(None, false);

    assert!(player.gameplay_state().is_empty());
    assert!(player.gameplay_state().quests.statuses.is_empty());
    assert!(player.gameplay_state().rest.logout_time.is_none());
}

#[test]
fn player_gameplay_apply_load_record_stores_every_major_bucket() {
    let mut player = Player::new(None, false);
    let state = player_gameplay_sample_state();

    player.apply_gameplay_state_from_load(PlayerGameplayLoadRecord {
        state: state.clone(),
    });

    assert_eq!(
        player.gameplay_state().quests.statuses,
        state.quests.statuses
    );
    assert_eq!(
        player.gameplay_state().quests.objective_progress,
        state.quests.objective_progress
    );
    assert_eq!(player.gameplay_state().skills, state.skills);
    assert_eq!(player.gameplay_state().spells, state.spells);
    assert_eq!(player.gameplay_state().talents, state.talents);
    assert_eq!(player.gameplay_state().action_buttons, state.action_buttons);
    assert_eq!(player.gameplay_state().taxi, state.taxi);
    assert_eq!(player.gameplay_state().social, state.social);
    assert_eq!(player.gameplay_state().mails, state.mails);
    assert_eq!(player.gameplay_state().group, state.group);
    assert_eq!(player.gameplay_state().guild, state.guild);
    assert_eq!(player.gameplay_state().battleground, state.battleground);
    assert_eq!(player.gameplay_state().reputations, state.reputations);
    assert_eq!(player.gameplay_state().achievements, state.achievements);
    assert_eq!(
        player.gameplay_state().achievement_criteria,
        state.achievement_criteria
    );
    assert_eq!(player.gameplay_state().currencies, state.currencies);
    assert_eq!(
        player.gameplay_state().spell_cooldowns,
        state.spell_cooldowns
    );
    assert_eq!(player.gameplay_state().spell_charges, state.spell_charges);
    assert_eq!(player.gameplay_state().rest, state.rest);
}

#[test]
fn player_gameplay_load_plan_preserves_trinity_order() {
    let plan = PlayerGameplayLoadPlan::trinity_load_from_db();

    assert!(plan.occurs_before(
        PlayerGameplayLoadStep::LoadAchievementsAndQuestCriteria,
        PlayerGameplayLoadStep::LoadHomeBind,
    ));
    assert!(plan.occurs_before(
        PlayerGameplayLoadStep::InitializeSkillFields,
        PlayerGameplayLoadStep::LoadSpells,
    ));
    assert!(plan.occurs_before(
        PlayerGameplayLoadStep::LoadSkills,
        PlayerGameplayLoadStep::LoadSpells,
    ));
    assert!(plan.occurs_before(
        PlayerGameplayLoadStep::LoadSkills,
        PlayerGameplayLoadStep::LoadActionButtons,
    ));
    assert!(plan.occurs_before(
        PlayerGameplayLoadStep::LoadTaxiMaskAndDestinations,
        PlayerGameplayLoadStep::InitTaxiNodesForLevel,
    ));
    assert!(plan.occurs_before(
        PlayerGameplayLoadStep::InitStatsForLevel,
        PlayerGameplayLoadStep::ApplyRestBonus,
    ));
    assert!(plan.occurs_before(
        PlayerGameplayLoadStep::LoadQuestStatus,
        PlayerGameplayLoadStep::LoadReputation,
    ));
    assert!(plan.occurs_before(
        PlayerGameplayLoadStep::LoadQuestStatus,
        PlayerGameplayLoadStep::LoadInventory,
    ));
    assert!(plan.occurs_before(
        PlayerGameplayLoadStep::LoadQuestStatus,
        PlayerGameplayLoadStep::LoadActionButtons,
    ));
    assert!(plan.occurs_before(
        PlayerGameplayLoadStep::LoadQuestStatus,
        PlayerGameplayLoadStep::LoadMail,
    ));
    assert!(plan.occurs_before(
        PlayerGameplayLoadStep::LoadQuestStatus,
        PlayerGameplayLoadStep::LoadSocial,
    ));
    assert!(plan.occurs_before(
        PlayerGameplayLoadStep::FinalRelocate,
        PlayerGameplayLoadStep::LoadSpellCooldownsAndCharges,
    ));
}

#[test]
fn player_gameplay_rest_and_taxi_destination_round_trip() {
    let mut player = Player::new(None, false);
    let state = player_gameplay_sample_state();
    let expected_taxi = state.taxi.clone();
    let expected_rest = state.rest.clone();

    player.apply_gameplay_state_from_load(PlayerGameplayLoadRecord { state });

    assert_eq!(player.gameplay_state().taxi, expected_taxi);
    assert_eq!(player.gameplay_state().taxi.source_node_id, Some(1));
    assert_eq!(player.gameplay_state().taxi.destination_node_id, Some(2));
    assert_eq!(player.gameplay_state().taxi.destinations, vec![1, 2, 3]);
    assert_eq!(player.gameplay_state().rest, expected_rest);
    assert_eq!(player.gameplay_state().rest.rest_bonus, 1.5);
    assert!(player.gameplay_state().rest.logout_was_resting);
}

fn can_bank_args<'a>(
    bag: u8,
    slot: u8,
    proto: Option<&'a ItemStorageTemplate>,
    source_item: Option<&'a Item>,
) -> CanBankItemArgs<'a> {
    CanBankItemArgs {
        bag,
        slot,
        proto,
        source_item,
        source_is_not_empty_bag: false,
        source_is_bag: false,
        source_is_currency_token: false,
        source_bop_trade_allowed_for_player: false,
        swap: false,
        can_use_result: InventoryResult::Ok,
        limit_category: None,
        slot_items: &[],
        stored_items: &[],
        bag_templates: &[],
    }
}

fn find_equip_args<'a>(
    proto: &'a ItemStorageTemplate,
    slot: u8,
    swap: bool,
    equipped_items: &'a [ItemSlotRef<'a>],
) -> FindEquipSlotArgs<'a> {
    FindEquipSlotArgs {
        proto,
        slot,
        swap,
        can_dual_wield: false,
        can_titan_grip: false,
        is_two_hand_used: false,
        has_required_profession_skill: false,
        profession_slot: None,
        equipped_items,
    }
}

fn can_equip_args<'a>(
    slot: u8,
    proto: Option<&'a ItemStorageTemplate>,
    source_item: Option<&'a Item>,
) -> CanEquipItemArgs<'a> {
    CanEquipItemArgs {
        slot,
        proto,
        source_item,
        source_bop_trade_allowed_for_player: false,
        swap: false,
        not_loading: true,
        is_stunned: false,
        is_charmed: false,
        is_in_combat: false,
        is_in_progress_arena: false,
        weapon_change_timer_active: false,
        current_generic_spell_allows_equip: None,
        current_channeled_spell_allows_equip: None,
        heirloom_required_level_failed: false,
        can_use_result: InventoryResult::Ok,
        can_equip_unique_result: InventoryResult::Ok,
        can_dual_wield: false,
        can_titan_grip: false,
        is_two_hand_used: false,
        proto_always_allow_dual_wield: false,
        has_required_profession_skill: false,
        profession_slot: None,
        offhand_can_unequip_result: InventoryResult::Ok,
        offhand_can_store_result: InventoryResult::Ok,
        limit_category: None,
        equipped_items: &[],
        stored_items: &[],
    }
}

fn can_unequip_args<'a>(
    pos: u16,
    proto: Option<&'a ItemStorageTemplate>,
    source_item: Option<&'a Item>,
) -> CanUnequipItemArgs<'a> {
    CanUnequipItemArgs {
        pos,
        source_item,
        proto,
        swap: false,
        source_is_not_empty_bag: false,
        is_charmed: false,
        is_in_combat: false,
        is_in_progress_arena: false,
    }
}

fn can_use_template_args<'a>(proto: Option<&'a ItemStorageTemplate>) -> CanUseItemTemplateArgs<'a> {
    CanUseItemTemplateArgs {
        proto,
        skip_required_level_check: false,
        player_level: 70,
        team: TEAM_HORDE_ID,
        allowable_class_matches: true,
        allowable_race_matches: true,
        internal_item: false,
        faction_horde: false,
        faction_alliance: false,
        required_skill: 0,
        required_skill_rank: 0,
        required_skill_value: 0,
        required_spell: 0,
        has_required_spell: false,
        base_required_level: 0,
        holiday_id: 0,
        holiday_active: false,
        required_reputation_faction: 0,
        required_reputation_rank: 0,
        player_reputation_rank: 0,
        effect0_spell_id: None,
        effect1_spell_id: None,
        has_effect1_spell: false,
        artifact_specialization: None,
        primary_specialization: 0,
    }
}

fn can_use_args<'a>(
    proto: Option<&'a ItemStorageTemplate>,
    source_item: Option<&'a Item>,
) -> CanUseItemArgs<'a> {
    CanUseItemArgs {
        source_item,
        proto,
        not_loading: true,
        is_alive: true,
        player_level: 70,
        item_required_level: 0,
        source_bop_trade_allowed_for_player: false,
        template_args: can_use_template_args(proto),
        item_skill: 0,
        item_skill_value: 0,
        has_item_skill: false,
        player_class: CLASS_WARRIOR,
        proto_is_heirloom: false,
    }
}

fn can_equip_unique_template_args<'a>(
    proto: Option<&'a ItemStorageTemplate>,
) -> CanEquipUniqueItemTemplateArgs<'a> {
    CanEquipUniqueItemTemplateArgs {
        proto,
        except_slot: NULL_SLOT,
        limit_count: 1,
        unique_equippable: false,
        limit_category: None,
        equipped_items: &[],
        equipped_gems: &[],
    }
}

fn can_equip_unique_args<'a>(
    source_item: Option<&'a Item>,
    proto: Option<&'a ItemStorageTemplate>,
) -> CanEquipUniqueItemArgs<'a> {
    CanEquipUniqueItemArgs {
        source_item,
        proto,
        except_slot: NULL_SLOT,
        limit_count: 1,
        unique_equippable: false,
        limit_category: None,
        equipped_items: &[],
        equipped_gems: &[],
        socketed_gems: &[],
    }
}

#[test]
fn player_constructor_matches_cpp_base_state() {
    let player = Player::new(Some(42), false);

    assert_eq!(player.unit().world().object().type_id(), TypeId::Player);
    assert_eq!(
        player.unit().world().object().type_mask(),
        TypeMask::OBJECT | TypeMask::UNIT | TypeMask::PLAYER
    );
    assert_eq!(player.session_id(), Some(42));
    assert_eq!(player.hit_chances(), (7.5, 7.5, 15.0));
    assert_eq!(player.ingame_time(), 0);
    assert_eq!(player.shared_quest_id(), 0);
    assert_eq!(player.extra_flags(), 0);
    assert!(!player.is_game_master_like_cpp());
    assert_eq!(player.team(), TEAM_OTHER);
    assert!(player.is_active());
    assert!(player.controlled_by_player());
    assert!(player.accept_whispers());
    assert_eq!(
        player.data().visible_items,
        [VisibleItemValues::default(); EQUIPMENT_SLOT_END as usize]
    );
    assert!(!player.player_data_changes_mask().is_any_set());
    assert!(!player.active_player_data_changes_mask().is_any_set());
}

#[test]
fn game_master_flag_matches_cpp_extra_flag() {
    let mut player = Player::new(Some(42), false);

    player.set_game_master_like_cpp(true);
    assert!(player.is_game_master_like_cpp());
    assert_eq!(
        player.extra_flags() & PLAYER_EXTRA_GM_ON,
        PLAYER_EXTRA_GM_ON
    );

    player.set_game_master_like_cpp(false);
    assert!(!player.is_game_master_like_cpp());
    assert_eq!(player.extra_flags() & PLAYER_EXTRA_GM_ON, 0);
}

#[test]
fn can_filter_whispers_permission_keeps_constructor_accept_flag_false() {
    let player = Player::new(None, true);
    assert!(!player.accept_whispers());
}

#[test]
fn player_position_classifiers_match_cpp_static_helpers() {
    assert!(is_inventory_pos(INVENTORY_SLOT_BAG_0, NULL_SLOT));
    assert!(!is_inventory_pos(NULL_BAG, NULL_SLOT));
    assert!(is_inventory_pos(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START
    ));
    assert!(is_inventory_pos(INVENTORY_SLOT_BAG_START, 0));
    assert!(is_inventory_pos(INVENTORY_SLOT_BAG_0, KEYRING_SLOT_START));
    assert!(is_inventory_pos(
        INVENTORY_SLOT_BAG_0,
        CHILD_EQUIPMENT_SLOT_START
    ));
    assert!(!is_inventory_pos(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_BAG_START
    ));
    assert!(is_inventory_packed_pos(make_item_pos(
        INVENTORY_SLOT_BAG_START,
        5
    )));

    assert!(is_equipment_pos(INVENTORY_SLOT_BAG_0, 0));
    assert!(is_equipment_pos(
        INVENTORY_SLOT_BAG_0,
        PROFESSION_SLOT_START
    ));
    assert!(is_equipment_pos(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_BAG_START
    ));
    assert!(is_equipment_pos(
        INVENTORY_SLOT_BAG_0,
        REAGENT_BAG_SLOT_START
    ));
    assert!(!is_equipment_pos(INVENTORY_SLOT_BAG_START, 0));
    assert!(is_equipment_packed_pos(make_item_pos(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_BAG_START
    )));

    assert!(is_bank_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START));
    assert!(is_bank_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_BAG_START));
    assert!(is_bank_pos(BANK_SLOT_BAG_START, 0));
    assert!(!is_bank_pos(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START
    ));
    assert!(is_bank_packed_pos(make_item_pos(BANK_SLOT_BAG_START, 2)));

    assert!(is_bag_pos(make_item_pos(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_BAG_START
    )));
    assert!(is_bag_pos(make_item_pos(
        INVENTORY_SLOT_BAG_0,
        BANK_SLOT_BAG_START
    )));
    assert!(is_bag_pos(make_item_pos(
        INVENTORY_SLOT_BAG_0,
        REAGENT_BAG_SLOT_START
    )));
    assert!(!is_bag_pos(make_item_pos(INVENTORY_SLOT_BAG_START, 0)));

    assert!(is_child_equipment_pos(
        INVENTORY_SLOT_BAG_0,
        CHILD_EQUIPMENT_SLOT_START
    ));
    assert!(is_child_equipment_packed_pos(make_item_pos(
        INVENTORY_SLOT_BAG_0,
        CHILD_EQUIPMENT_SLOT_START
    )));
    assert!(!is_child_equipment_pos(
        INVENTORY_SLOT_BAG_START,
        CHILD_EQUIPMENT_SLOT_START
    ));
}

#[test]
fn player_is_valid_pos_matches_cpp_top_level_and_bag_rules() {
    let bag_guid = ObjectGuid::create_item(1, 300);
    let mut player = Player::new(None, false);
    player.set_inventory_slot_count(16);

    assert!(player.is_valid_pos(NULL_BAG, NULL_SLOT, false));
    assert!(!player.is_valid_pos(NULL_BAG, NULL_SLOT, true));
    assert!(player.is_valid_pos(INVENTORY_SLOT_BAG_0, NULL_SLOT, false));
    assert!(!player.is_valid_pos(INVENTORY_SLOT_BAG_0, NULL_SLOT, true));
    assert!(player.is_valid_pos(INVENTORY_SLOT_BAG_0, 0, true));
    assert!(player.is_valid_pos(INVENTORY_SLOT_BAG_0, PROFESSION_SLOT_START, true));
    assert!(player.is_valid_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START, true));
    assert!(player.is_valid_pos(INVENTORY_SLOT_BAG_0, REAGENT_BAG_SLOT_START, true));
    assert!(player.is_valid_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 15, true));
    assert!(!player.is_valid_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 16, true));
    assert!(player.is_valid_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, true));
    assert!(player.is_valid_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_BAG_START, true));
    assert!(player.is_valid_pos(INVENTORY_SLOT_BAG_0, KEYRING_SLOT_START, true));
    assert!(!player.is_valid_pos(INVENTORY_SLOT_BAG_0, CHILD_EQUIPMENT_SLOT_START, true));

    assert!(!player.is_valid_pos(INVENTORY_SLOT_BAG_START, 0, true));
    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 4)
        .unwrap();
    assert!(player.is_valid_pos(INVENTORY_SLOT_BAG_START, NULL_SLOT, false));
    assert!(!player.is_valid_pos(INVENTORY_SLOT_BAG_START, NULL_SLOT, true));
    assert!(player.is_valid_pos(INVENTORY_SLOT_BAG_START, 3, true));
    assert!(!player.is_valid_pos(INVENTORY_SLOT_BAG_START, 4, true));
    assert!(player.is_valid_packed_pos(make_item_pos(INVENTORY_SLOT_BAG_START, 3), true));
}

#[test]
fn find_equip_slot_maps_inventory_types_like_cpp() {
    let player = Player::new(None, false);
    let head = ItemStorageTemplate {
        inventory_type: InventoryType::Head,
        ..ItemStorageTemplate::regular_item(1, 1)
    };
    let robe = ItemStorageTemplate {
        inventory_type: InventoryType::Robe,
        ..ItemStorageTemplate::regular_item(2, 1)
    };
    let bag = ItemStorageTemplate {
        inventory_type: InventoryType::Bag,
        ..ItemStorageTemplate::regular_item(3, 1)
    };
    let weapon = ItemStorageTemplate {
        inventory_type: InventoryType::Weapon,
        ..ItemStorageTemplate::regular_item(4, 1)
    };
    let two_hand = ItemStorageTemplate {
        inventory_type: InventoryType::Weapon2Hand,
        ..ItemStorageTemplate::regular_item(5, 1)
    };

    assert_eq!(
        player.find_equip_slot(find_equip_args(&head, NULL_SLOT, false, &[])),
        EQUIPMENT_SLOT_HEAD
    );
    assert_eq!(
        player.find_equip_slot(find_equip_args(&robe, NULL_SLOT, false, &[])),
        EQUIPMENT_SLOT_CHEST
    );
    assert_eq!(
        player.find_equip_slot(find_equip_args(&bag, NULL_SLOT, false, &[])),
        INVENTORY_SLOT_BAG_START
    );
    assert_eq!(
        player.find_equip_slot(find_equip_args(&weapon, EQUIPMENT_SLOT_OFFHAND, false, &[])),
        NULL_SLOT
    );

    let mut dual_args = find_equip_args(&weapon, EQUIPMENT_SLOT_OFFHAND, false, &[]);
    dual_args.can_dual_wield = true;
    assert_eq!(player.find_equip_slot(dual_args), EQUIPMENT_SLOT_OFFHAND);

    let mut titan_args = find_equip_args(&two_hand, EQUIPMENT_SLOT_OFFHAND, false, &[]);
    titan_args.can_dual_wield = true;
    assert_eq!(player.find_equip_slot(titan_args), NULL_SLOT);
    titan_args.can_titan_grip = true;
    assert_eq!(player.find_equip_slot(titan_args), EQUIPMENT_SLOT_OFFHAND);
}

#[test]
fn find_equip_slot_requested_free_and_swap_paths_match_cpp() {
    let player = Player::new(None, false);
    let finger = ItemStorageTemplate {
        inventory_type: InventoryType::Finger,
        ..ItemStorageTemplate::regular_item(10, 1)
    };
    let mut ring1 = Item::default();
    ring1.set_debug_item_level(120);
    let mut ring2 = Item::default();
    ring2.set_debug_item_level(45);
    let equipped = [
        ItemSlotRef::new(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_FINGER1, &ring1),
        ItemSlotRef::new(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_FINGER2, &ring2),
    ];

    assert_eq!(
        player.find_equip_slot(find_equip_args(
            &finger,
            EQUIPMENT_SLOT_FINGER1,
            false,
            &equipped
        )),
        NULL_SLOT
    );
    assert_eq!(
        player.find_equip_slot(find_equip_args(
            &finger,
            EQUIPMENT_SLOT_FINGER1,
            true,
            &equipped
        )),
        EQUIPMENT_SLOT_FINGER1
    );
    assert_eq!(
        player.find_equip_slot(find_equip_args(&finger, NULL_SLOT, true, &equipped)),
        EQUIPMENT_SLOT_FINGER2
    );

    let equipped = [ItemSlotRef::new(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_FINGER1,
        &ring1,
    )];
    assert_eq!(
        player.find_equip_slot(find_equip_args(&finger, NULL_SLOT, false, &equipped)),
        EQUIPMENT_SLOT_FINGER2
    );
}

#[test]
fn find_equip_slot_twohand_offhand_and_professions_match_cpp_edges() {
    let player = Player::new(None, false);
    let weapon = ItemStorageTemplate {
        inventory_type: InventoryType::Weapon,
        ..ItemStorageTemplate::regular_item(20, 1)
    };
    let mut mainhand = Item::default();
    mainhand.set_debug_item_level(100);
    let equipped = [ItemSlotRef::new(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_MAINHAND,
        &mainhand,
    )];
    let mut args = find_equip_args(&weapon, NULL_SLOT, false, &equipped);
    args.can_dual_wield = true;
    args.is_two_hand_used = true;
    assert_eq!(player.find_equip_slot(args), NULL_SLOT);

    let cooking_gear = ItemStorageTemplate {
        class_id: ItemClass::Profession,
        subclass_id: ItemSubclassProfession::Cooking as u32,
        inventory_type: InventoryType::ProfessionGear,
        ..ItemStorageTemplate::regular_item(21, 1)
    };
    let fishing_gear = ItemStorageTemplate {
        class_id: ItemClass::Profession,
        subclass_id: ItemSubclassProfession::Fishing as u32,
        inventory_type: InventoryType::ProfessionGear,
        ..ItemStorageTemplate::regular_item(22, 1)
    };
    let blacksmithing_gear = ItemStorageTemplate {
        class_id: ItemClass::Profession,
        subclass_id: ItemSubclassProfession::Blacksmithing as u32,
        inventory_type: InventoryType::ProfessionGear,
        ..ItemStorageTemplate::regular_item(23, 1)
    };

    let mut profession_args = find_equip_args(&cooking_gear, NULL_SLOT, false, &[]);
    profession_args.has_required_profession_skill = true;
    assert_eq!(
        player.find_equip_slot(profession_args),
        PROFESSION_SLOT_COOKING_GEAR1
    );

    profession_args.proto = &fishing_gear;
    assert_eq!(player.find_equip_slot(profession_args), NULL_SLOT);

    profession_args.proto = &blacksmithing_gear;
    profession_args.profession_slot = Some(0);
    assert_eq!(
        player.find_equip_slot(profession_args),
        PROFESSION_SLOT_PROFESSION1_GEAR2
    );
}

#[test]
fn can_equip_item_preflight_and_runtime_guards_match_cpp_order() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate {
        inventory_type: InventoryType::Head,
        ..ItemStorageTemplate::regular_item(100, 1)
    };
    let mut source = Item::default();
    source.set_count(1);

    assert_eq!(
        player
            .can_equip_item(can_equip_args(NULL_SLOT, Some(&proto), None))
            .result,
        InventoryResult::ItemNotFound
    );

    let mut swap_missing = can_equip_args(NULL_SLOT, None, Some(&source));
    swap_missing.swap = true;
    assert_eq!(
        player.can_equip_item(swap_missing).result,
        InventoryResult::CantSwap
    );

    source.set_loot_generated(true);
    assert_eq!(
        player
            .can_equip_item(can_equip_args(NULL_SLOT, Some(&proto), Some(&source)))
            .result,
        InventoryResult::LootGone
    );
    source.set_loot_generated(false);

    source.set_item_flag(ItemFieldFlags::SOULBOUND);
    source.set_owner_guid(ObjectGuid::create_player(1, 99));
    assert_eq!(
        player
            .can_equip_item(can_equip_args(NULL_SLOT, Some(&proto), Some(&source)))
            .result,
        InventoryResult::NotOwner
    );
    source.remove_item_flag(ItemFieldFlags::SOULBOUND);

    let limited = ItemStorageTemplate {
        max_count: 1,
        ..proto
    };
    source.object_mut().create(ObjectGuid::create_item(1, 900));
    source.object_mut().set_entry(limited.entry);
    let mut stored = Item::default();
    stored.object_mut().create(ObjectGuid::create_item(1, 901));
    stored.object_mut().set_entry(limited.entry);
    stored.set_count(1);
    let stored_items = [ItemStorageRef::new(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        &stored,
        Some(&limited),
    )];
    let mut limit_args = can_equip_args(NULL_SLOT, Some(&limited), Some(&source));
    limit_args.stored_items = &stored_items;
    assert_eq!(
        player.can_equip_item(limit_args).result,
        InventoryResult::ItemMaxCount
    );

    let mut stunned = can_equip_args(NULL_SLOT, Some(&proto), Some(&source));
    stunned.is_stunned = true;
    stunned.is_charmed = true;
    assert_eq!(
        player.can_equip_item(stunned).result,
        InventoryResult::GenericStunned
    );

    let mut combat = can_equip_args(NULL_SLOT, Some(&proto), Some(&source));
    combat.is_in_combat = true;
    assert_eq!(
        player.can_equip_item(combat).result,
        InventoryResult::NotInCombat
    );

    let weapon = ItemStorageTemplate {
        class_id: ItemClass::Weapon,
        inventory_type: InventoryType::Weapon,
        ..ItemStorageTemplate::regular_item(101, 1)
    };
    let mut cooldown = can_equip_args(NULL_SLOT, Some(&weapon), Some(&source));
    cooldown.is_in_combat = true;
    cooldown.weapon_change_timer_active = true;
    assert_eq!(
        player.can_equip_item(cooldown).result,
        InventoryResult::ItemCooldown
    );

    let mut casting = can_equip_args(NULL_SLOT, Some(&weapon), Some(&source));
    casting.current_generic_spell_allows_equip = Some(false);
    assert_eq!(
        player.can_equip_item(casting).result,
        InventoryResult::ClientLockedOut
    );
}

#[test]
fn can_equip_item_destination_use_and_unique_paths_match_cpp() {
    let player = Player::new(None, false);
    let head = ItemStorageTemplate {
        inventory_type: InventoryType::Head,
        ..ItemStorageTemplate::regular_item(200, 1)
    };
    let finger = ItemStorageTemplate {
        inventory_type: InventoryType::Finger,
        ..ItemStorageTemplate::regular_item(201, 1)
    };
    let mut source = Item::default();
    source.set_count(1);
    let mut equipped_head = Item::default();
    equipped_head.set_count(1);
    let equipped = [ItemSlotRef::new(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_HEAD,
        &equipped_head,
    )];

    let outcome = player.can_equip_item(can_equip_args(NULL_SLOT, Some(&head), Some(&source)));
    assert_eq!(outcome.result, InventoryResult::Ok);
    assert_eq!(
        outcome.dest,
        make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_HEAD)
    );
    assert_eq!(outcome.unique_ignore_slot, Some(NULL_SLOT));

    let mut occupied = can_equip_args(NULL_SLOT, Some(&head), Some(&source));
    occupied.equipped_items = &equipped;
    assert_eq!(
        player.can_equip_item(occupied).result,
        InventoryResult::NotEquippable
    );

    let mut can_use = can_equip_args(NULL_SLOT, Some(&head), Some(&source));
    can_use.can_use_result = InventoryResult::CantEquipSkill;
    assert_eq!(
        player.can_equip_item(can_use).result,
        InventoryResult::CantEquipSkill
    );

    let mut source_ring = Item::default();
    source_ring.set_count(1);
    let other_ring = Item::default();
    let rings = [
        ItemSlotRef::new(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_FINGER1, &other_ring),
        ItemSlotRef::new(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_FINGER2, &source_ring),
    ];
    let mut unique = can_equip_args(EQUIPMENT_SLOT_FINGER1, Some(&finger), Some(&source_ring));
    unique.swap = true;
    unique.equipped_items = &rings;
    unique.can_equip_unique_result = InventoryResult::ItemUniqueEquippable;
    let outcome = player.can_equip_item(unique);
    assert_eq!(outcome.result, InventoryResult::ItemUniqueEquippable);
    assert_eq!(outcome.unique_ignore_slot, Some(EQUIPMENT_SLOT_FINGER2));
}

#[test]
fn can_equip_item_quiver_offhand_and_twohand_edges_match_cpp() {
    let player = Player::new(None, false);
    let mut source = Item::default();
    source.set_count(1);
    let bag_quiver = ItemStorageTemplate {
        class_id: ItemClass::Quiver,
        subclass_id: ItemSubClassQuiver::AmmoPouch as u32,
        inventory_type: InventoryType::Bag,
        ..ItemStorageTemplate::regular_item(300, 1)
    };
    let existing_quiver = Item::default();
    let stored_items = [ItemStorageRef::new(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_BAG_START,
        &existing_quiver,
        Some(&bag_quiver),
    )];
    let mut quiver_args = can_equip_args(NULL_SLOT, Some(&bag_quiver), Some(&source));
    quiver_args.stored_items = &stored_items;
    assert_eq!(
        player.can_equip_item(quiver_args).result,
        InventoryResult::OnlyOneAmmo
    );

    let polearm = ItemStorageTemplate {
        class_id: ItemClass::Weapon,
        subclass_id: ItemSubClassWeapon::Polearm as u32,
        inventory_type: InventoryType::Weapon,
        ..ItemStorageTemplate::regular_item(301, 1)
    };
    let mut polearm_args = can_equip_args(EQUIPMENT_SLOT_OFFHAND, Some(&polearm), Some(&source));
    polearm_args.can_dual_wield = true;
    assert_eq!(
        player.can_equip_item(polearm_args).result,
        InventoryResult::TwoHandSkillNotFound
    );

    let offhand_weapon = ItemStorageTemplate {
        inventory_type: InventoryType::WeaponOffhand,
        ..ItemStorageTemplate::regular_item(302, 1)
    };
    assert_eq!(
        player
            .can_equip_item(can_equip_args(
                EQUIPMENT_SLOT_OFFHAND,
                Some(&offhand_weapon),
                Some(&source)
            ))
            .result,
        InventoryResult::TwoHandSkillNotFound
    );

    let mut twohand_used =
        can_equip_args(EQUIPMENT_SLOT_OFFHAND, Some(&offhand_weapon), Some(&source));
    twohand_used.proto_always_allow_dual_wield = true;
    twohand_used.is_two_hand_used = true;
    assert_eq!(
        player.can_equip_item(twohand_used).result,
        InventoryResult::Equipped2handed
    );

    let twohand = ItemStorageTemplate {
        inventory_type: InventoryType::Weapon2Hand,
        ..ItemStorageTemplate::regular_item(303, 1)
    };
    let offhand_item = Item::default();
    let equipped_offhand = [ItemSlotRef::new(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_OFFHAND,
        &offhand_item,
    )];
    let mut twohand_args = can_equip_args(NULL_SLOT, Some(&twohand), Some(&source));
    twohand_args.equipped_items = &equipped_offhand;
    twohand_args.offhand_can_store_result = InventoryResult::InvFull;
    assert_eq!(
        player.can_equip_item(twohand_args).result,
        InventoryResult::InvFull
    );

    twohand_args.swap = true;
    assert_eq!(
        player.can_equip_item(twohand_args).result,
        InventoryResult::CantSwap
    );
}

#[test]
fn can_unequip_item_matches_cpp_position_template_and_runtime_guards() {
    let player = Player::new(None, false);
    let armor = ItemStorageTemplate {
        inventory_type: InventoryType::Chest,
        ..ItemStorageTemplate::regular_item(400, 1)
    };
    let weapon = ItemStorageTemplate {
        class_id: ItemClass::Weapon,
        inventory_type: InventoryType::Weapon,
        ..ItemStorageTemplate::regular_item(401, 1)
    };
    let bag = ItemStorageTemplate {
        inventory_type: InventoryType::Bag,
        ..ItemStorageTemplate::regular_item(402, 1)
    };
    let mut source = Item::default();
    source.set_count(1);

    assert_eq!(
        player.can_unequip_item(can_unequip_args(
            make_item_pos(INVENTORY_SLOT_BAG_START, 0),
            Some(&armor),
            Some(&source),
        )),
        InventoryResult::Ok
    );
    assert_eq!(
        player.can_unequip_item(can_unequip_args(
            make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST),
            Some(&armor),
            None,
        )),
        InventoryResult::Ok
    );
    assert_eq!(
        player.can_unequip_item(can_unequip_args(
            make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST),
            None,
            Some(&source),
        )),
        InventoryResult::ItemNotFound
    );

    source.set_loot_generated(true);
    assert_eq!(
        player.can_unequip_item(can_unequip_args(
            make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST),
            Some(&armor),
            Some(&source),
        )),
        InventoryResult::LootGone
    );
    source.set_loot_generated(false);

    let mut charmed = can_unequip_args(
        make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST),
        Some(&armor),
        Some(&source),
    );
    charmed.is_charmed = true;
    assert_eq!(
        player.can_unequip_item(charmed),
        InventoryResult::ClientLockedOut
    );

    let mut combat = can_unequip_args(
        make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST),
        Some(&armor),
        Some(&source),
    );
    combat.is_in_combat = true;
    assert_eq!(
        player.can_unequip_item(combat),
        InventoryResult::NotInCombat
    );

    let mut arena = can_unequip_args(
        make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST),
        Some(&armor),
        Some(&source),
    );
    arena.is_in_progress_arena = true;
    assert_eq!(
        player.can_unequip_item(arena),
        InventoryResult::NotDuringArenaMatch
    );

    let mut weapon_combat = can_unequip_args(
        make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND),
        Some(&weapon),
        Some(&source),
    );
    weapon_combat.is_in_combat = true;
    assert_eq!(player.can_unequip_item(weapon_combat), InventoryResult::Ok);

    let mut non_empty_bag = can_unequip_args(
        make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START),
        Some(&bag),
        Some(&source),
    );
    non_empty_bag.source_is_not_empty_bag = true;
    assert_eq!(
        player.can_unequip_item(non_empty_bag),
        InventoryResult::DestroyNonemptyBag
    );

    non_empty_bag.swap = true;
    assert_eq!(player.can_unequip_item(non_empty_bag), InventoryResult::Ok);
}

#[test]
fn can_use_item_template_matches_cpp_access_requirement_order() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(500, 1);

    assert_eq!(
        player.can_use_item_template(can_use_template_args(None)),
        InventoryResult::ItemNotFound
    );

    let mut args = can_use_template_args(Some(&proto));
    args.internal_item = true;
    args.faction_horde = true;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::CantEquipEver
    );

    args.internal_item = false;
    args.team = TEAM_ALLIANCE_ID;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::CantEquipEver
    );

    args.faction_horde = false;
    args.faction_alliance = true;
    args.team = TEAM_HORDE_ID;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::CantEquipEver
    );

    args.faction_alliance = false;
    args.allowable_class_matches = false;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::CantEquipEver
    );

    args.allowable_class_matches = true;
    args.allowable_race_matches = false;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::CantEquipEver
    );

    args.allowable_race_matches = true;
    args.required_skill = 164;
    args.required_skill_rank = 75;
    args.required_skill_value = 0;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::ProficiencyNeeded
    );

    args.required_skill_value = 50;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::CantEquipSkill
    );

    args.required_skill_value = 75;
    args.required_spell = 1000;
    args.has_required_spell = false;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::ProficiencyNeeded
    );
}

#[test]
fn can_use_item_template_matches_cpp_late_requirement_order() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(501, 1);
    let mut args = can_use_template_args(Some(&proto));

    args.player_level = 20;
    args.base_required_level = 30;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::CantEquipLevelI
    );

    args.skip_required_level_check = true;
    assert_eq!(player.can_use_item_template(args), InventoryResult::Ok);

    args.skip_required_level_check = false;
    args.player_level = 70;
    args.holiday_id = 1;
    args.holiday_active = false;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::ClientLockedOut
    );

    args.holiday_active = true;
    args.required_reputation_faction = 72;
    args.required_reputation_rank = 5;
    args.player_reputation_rank = 4;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::CantEquipReputation
    );

    args.player_reputation_rank = 5;
    args.effect0_spell_id = Some(483);
    args.effect1_spell_id = Some(9000);
    args.has_effect1_spell = true;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::InternalBagError
    );

    args.has_effect1_spell = false;
    args.artifact_specialization = Some(2);
    args.primary_specialization = 1;
    assert_eq!(
        player.can_use_item_template(args),
        InventoryResult::CantUseItem
    );

    args.primary_specialization = 2;
    assert_eq!(player.can_use_item_template(args), InventoryResult::Ok);
}

#[test]
fn can_use_item_object_matches_cpp_item_level_and_template_order() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(600, 1);
    let mut source = Item::default();
    source.set_count(1);

    assert_eq!(
        player.can_use_item(can_use_args(Some(&proto), None)),
        InventoryResult::ItemNotFound
    );

    let mut dead = can_use_args(Some(&proto), Some(&source));
    dead.is_alive = false;
    assert_eq!(player.can_use_item(dead), InventoryResult::PlayerDead);

    dead.not_loading = false;
    assert_eq!(player.can_use_item(dead), InventoryResult::Ok);

    assert_eq!(
        player.can_use_item(can_use_args(None, Some(&source))),
        InventoryResult::ItemNotFound
    );

    source.set_item_flag(ItemFieldFlags::SOULBOUND);
    source.set_owner_guid(ObjectGuid::create_player(1, 99));
    assert_eq!(
        player.can_use_item(can_use_args(Some(&proto), Some(&source))),
        InventoryResult::NotOwner
    );
    source.remove_item_flag(ItemFieldFlags::SOULBOUND);

    let mut level = can_use_args(Some(&proto), Some(&source));
    level.player_level = 20;
    level.item_required_level = 30;
    level.template_args.internal_item = true;
    assert_eq!(player.can_use_item(level), InventoryResult::CantEquipLevelI);

    let mut template = can_use_args(Some(&proto), Some(&source));
    template.template_args.internal_item = true;
    assert_eq!(
        player.can_use_item(template),
        InventoryResult::CantEquipEver
    );
}

#[test]
fn can_use_item_object_matches_cpp_skill_and_heirloom_morph() {
    let player = Player::new(None, false);
    let armor = ItemStorageTemplate {
        class_id: ItemClass::Armor,
        inventory_type: InventoryType::Chest,
        ..ItemStorageTemplate::regular_item(601, 1)
    };
    let weapon = ItemStorageTemplate {
        class_id: ItemClass::Weapon,
        inventory_type: InventoryType::Weapon,
        ..ItemStorageTemplate::regular_item(602, 1)
    };
    let source = Item::default();

    let mut no_skill = can_use_args(Some(&weapon), Some(&source));
    no_skill.item_skill = SKILL_MAIL;
    no_skill.item_skill_value = 0;
    assert_eq!(
        player.can_use_item(no_skill),
        InventoryResult::ProficiencyNeeded
    );

    no_skill.item_skill_value = 1;
    assert_eq!(player.can_use_item(no_skill), InventoryResult::Ok);

    let mut hunter_mail = can_use_args(Some(&armor), Some(&source));
    hunter_mail.item_skill = SKILL_MAIL;
    hunter_mail.item_skill_value = 0;
    hunter_mail.has_item_skill = false;
    hunter_mail.proto_is_heirloom = true;
    hunter_mail.player_class = CLASS_HUNTER;
    assert_eq!(player.can_use_item(hunter_mail), InventoryResult::Ok);

    let mut warrior_mail = hunter_mail;
    warrior_mail.player_class = CLASS_WARRIOR;
    assert_eq!(
        player.can_use_item(warrior_mail),
        InventoryResult::ProficiencyNeeded
    );

    let mut paladin_plate = can_use_args(Some(&armor), Some(&source));
    paladin_plate.item_skill = SKILL_PLATE_MAIL;
    paladin_plate.item_skill_value = 0;
    paladin_plate.has_item_skill = false;
    paladin_plate.proto_is_heirloom = true;
    paladin_plate.player_class = CLASS_PALADIN;
    assert_eq!(player.can_use_item(paladin_plate), InventoryResult::Ok);
}

#[test]
fn can_equip_unique_item_template_matches_cpp_unique_entry_guards() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(700, 1);
    assert_eq!(
        player.can_equip_unique_item_template(can_equip_unique_template_args(None)),
        InventoryResult::ItemNotFound
    );

    let mut equipped = Item::default();
    equipped.object_mut().set_entry(700);
    equipped.set_count(1);
    let equipped_items = [ItemStorageRef::new(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_FINGER1,
        &equipped,
        Some(&proto),
    )];

    let mut args = can_equip_unique_template_args(Some(&proto));
    args.unique_equippable = true;
    args.equipped_items = &equipped_items;
    assert_eq!(
        player.can_equip_unique_item_template(args),
        InventoryResult::ItemUniqueEquippable
    );

    args.except_slot = EQUIPMENT_SLOT_FINGER1;
    assert_eq!(
        player.can_equip_unique_item_template(args),
        InventoryResult::Ok
    );

    let equipped_gems = [EquippedGemRef::new(EQUIPMENT_SLOT_CHEST, 700, 0)];
    args.equipped_items = &[];
    args.equipped_gems = &equipped_gems;
    args.except_slot = NULL_SLOT;
    assert_eq!(
        player.can_equip_unique_item_template(args),
        InventoryResult::ItemUniqueEquippable
    );
}

#[test]
fn can_equip_unique_item_template_matches_cpp_limit_category_guards() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate {
        item_limit_category: 10,
        ..ItemStorageTemplate::regular_item(701, 1)
    };
    let limit = ItemLimitCategoryTemplate {
        id: 10,
        quantity: 2,
        flags: ITEM_LIMIT_CATEGORY_MODE_EQUIP,
    };
    let mut equipped = Item::default();
    equipped.object_mut().set_entry(702);
    equipped.set_count(1);
    let equipped_items = [ItemStorageRef::new(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_TRINKET1,
        &equipped,
        Some(&proto),
    )];
    let equipped_gems = [EquippedGemRef::new(EQUIPMENT_SLOT_CHEST, 703, 10)];

    let mut args = can_equip_unique_template_args(Some(&proto));
    assert_eq!(
        player.can_equip_unique_item_template(args),
        InventoryResult::NotEquippable
    );

    args.limit_category = Some(&limit);
    args.limit_count = 3;
    assert_eq!(
        player.can_equip_unique_item_template(args),
        InventoryResult::ItemMaxLimitCategoryEquippedExceededIs
    );

    args.limit_count = 2;
    args.equipped_items = &equipped_items;
    assert_eq!(
        player.can_equip_unique_item_template(args),
        InventoryResult::ItemMaxLimitCategoryEquippedExceededIs
    );

    args.equipped_items = &[];
    args.equipped_gems = &equipped_gems;
    assert_eq!(
        player.can_equip_unique_item_template(args),
        InventoryResult::ItemMaxCountEquippedSocketed
    );

    args.except_slot = EQUIPMENT_SLOT_CHEST;
    assert_eq!(
        player.can_equip_unique_item_template(args),
        InventoryResult::Ok
    );
}

#[test]
fn can_equip_unique_item_object_matches_cpp_template_then_gem_order() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(704, 1);
    let source = Item::default();
    let gem_proto = ItemStorageTemplate::regular_item(705, 1);
    let socketed_gems = [
        SocketedGemUniqueRef::new(None, true, None, 1),
        SocketedGemUniqueRef::new(Some(&gem_proto), true, None, 1),
    ];
    let equipped_gems = [EquippedGemRef::new(EQUIPMENT_SLOT_CHEST, 705, 0)];
    let base_equipped_gems = [EquippedGemRef::new(EQUIPMENT_SLOT_CHEST, 704, 0)];

    assert_eq!(
        player.can_equip_unique_item(can_equip_unique_args(None, Some(&proto))),
        InventoryResult::ItemNotFound
    );

    let mut template_first = can_equip_unique_args(Some(&source), Some(&proto));
    template_first.unique_equippable = true;
    template_first.equipped_gems = &base_equipped_gems;
    template_first.socketed_gems = &socketed_gems;
    assert_eq!(
        player.can_equip_unique_item(template_first),
        InventoryResult::ItemUniqueEquippable
    );

    let mut gem_args = can_equip_unique_args(Some(&source), Some(&proto));
    gem_args.socketed_gems = &socketed_gems;
    gem_args.equipped_gems = &equipped_gems;
    assert_eq!(
        player.can_equip_unique_item(gem_args),
        InventoryResult::ItemUniqueEquippable
    );
}

#[test]
fn can_equip_unique_item_object_matches_cpp_socketed_gem_limit_count() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(706, 1);
    let gem_proto = ItemStorageTemplate {
        item_limit_category: 20,
        ..ItemStorageTemplate::regular_item(707, 1)
    };
    let limit = ItemLimitCategoryTemplate {
        id: 20,
        quantity: 2,
        flags: ITEM_LIMIT_CATEGORY_MODE_EQUIP,
    };
    let socketed_gems = [SocketedGemUniqueRef::new(
        Some(&gem_proto),
        false,
        Some(&limit),
        2,
    )];
    let equipped_gems = [EquippedGemRef::new(EQUIPMENT_SLOT_CHEST, 708, 20)];

    let mut source = Item::default();
    source.set_slot(INVENTORY_SLOT_ITEM_START);
    let mut unequipped = can_equip_unique_args(Some(&source), Some(&proto));
    unequipped.socketed_gems = &socketed_gems;
    unequipped.equipped_gems = &equipped_gems;
    assert_eq!(
        player.can_equip_unique_item(unequipped),
        InventoryResult::ItemMaxCountEquippedSocketed
    );

    let mut equipped_source = Item::default();
    equipped_source.set_slot(EQUIPMENT_SLOT_FINGER1);
    let mut equipped = can_equip_unique_args(Some(&equipped_source), Some(&proto));
    equipped.socketed_gems = &socketed_gems;
    equipped.equipped_gems = &equipped_gems;
    assert_eq!(player.can_equip_unique_item(equipped), InventoryResult::Ok);
}

#[test]
fn item_pos_count_containment_matches_cpp_pos_only_check() {
    let target = ItemPosCount::new(make_item_pos(INVENTORY_SLOT_BAG_0, 10), 1);
    let positions = [ItemPosCount::new(
        make_item_pos(INVENTORY_SLOT_BAG_0, 10),
        99,
    )];

    assert!(target.is_contained_in(&positions));
    assert!(
        !ItemPosCount::new(make_item_pos(INVENTORY_SLOT_BAG_0, 11), 1).is_contained_in(&positions)
    );
}

#[test]
fn can_store_item_in_specific_slot_allocates_empty_top_level_like_cpp() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let mut dest = Vec::new();
    let mut count = 7;

    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            &mut dest,
            &proto,
            &mut count,
            false,
            None,
            None,
            false,
            None,
        ),
        InventoryResult::Ok
    );
    assert_eq!(
        dest,
        vec![ItemPosCount::new(
            make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
            7,
        )]
    );
    assert_eq!(count, 0);

    let mut duplicate_count = 3;
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            &mut dest,
            &proto,
            &mut duplicate_count,
            false,
            None,
            None,
            false,
            None,
        ),
        InventoryResult::Ok
    );
    assert_eq!(dest.len(), 1);
    assert_eq!(duplicate_count, 3);
}

#[test]
fn can_store_item_in_specific_slot_merges_existing_stack_like_cpp() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let mut existing = Item::default();
    existing
        .object_mut()
        .create(ObjectGuid::create_item(1, 100));
    existing.object_mut().set_entry(6948);
    existing.set_count(12);
    let mut dest = Vec::new();
    let mut count = 10;

    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            &mut dest,
            &proto,
            &mut count,
            false,
            Some(&existing),
            None,
            false,
            None,
        ),
        InventoryResult::Ok
    );
    assert_eq!(
        dest,
        vec![ItemPosCount::new(
            make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
            8,
        )]
    );
    assert_eq!(count, 2);

    existing.object_mut().set_entry(6949);
    let mut swap_count = 1;
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START + 1,
            &mut Vec::new(),
            &proto,
            &mut swap_count,
            true,
            Some(&existing),
            None,
            false,
            None,
        ),
        InventoryResult::Ok
    );

    let mut blocked_count = 2;
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START + 1,
            &mut Vec::new(),
            &proto,
            &mut blocked_count,
            false,
            Some(&existing),
            None,
            false,
            None,
        ),
        InventoryResult::CantStack
    );
    assert_eq!(blocked_count, 2);
}

#[test]
fn can_store_item_in_specific_slot_applies_source_move_guards_like_cpp() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let mut source = Item::default();
    source.object_mut().create(ObjectGuid::create_item(1, 101));
    source.object_mut().set_entry(6948);
    source.set_count(1);

    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            None,
            Some(&source),
            true,
            None,
        ),
        InventoryResult::DestroyNonemptyBag
    );

    let mut bag_slot_count = 1;
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_BAG_START,
            &mut Vec::new(),
            &proto,
            &mut bag_slot_count,
            false,
            None,
            Some(&source),
            true,
            None,
        ),
        InventoryResult::Ok
    );

    let mut same_source_count = 1;
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            &mut Vec::new(),
            &proto,
            &mut same_source_count,
            false,
            Some(&source),
            Some(&source),
            false,
            None,
        ),
        InventoryResult::Ok
    );

    source.set_item_flag(ItemFieldFlags::CHILD);
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            None,
            Some(&source),
            false,
            None,
        ),
        InventoryResult::WrongBagType3
    );
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            CHILD_EQUIPMENT_SLOT_START,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            None,
            Some(&source),
            false,
            None,
        ),
        InventoryResult::Ok
    );
}

#[test]
fn can_store_item_in_specific_slot_applies_empty_slot_fit_guards_like_cpp() {
    let mut player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let regular_bag_proto = ItemStorageTemplate {
        class_id: ItemClass::Container,
        subclass_id: ItemSubClassContainer::Container as u32,
        container_slots: 2,
        ..ItemStorageTemplate::regular_item(100, 1)
    };
    let herb_bag_proto = ItemStorageTemplate {
        class_id: ItemClass::Container,
        subclass_id: ItemSubClassContainer::HerbContainer as u32,
        container_slots: 2,
        ..ItemStorageTemplate::regular_item(101, 1)
    };
    let herb = ItemStorageTemplate {
        bag_family: BagFamilyMask::HERBS,
        ..ItemStorageTemplate::regular_item(2447, 20)
    };

    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            REAGENT_BAG_SLOT_START,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            None,
            None,
            false,
            None,
        ),
        InventoryResult::WrongBagType
    );
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            BUYBACK_SLOT_START,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            None,
            None,
            false,
            None,
        ),
        InventoryResult::WrongBagType
    );
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_START,
            0,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            None,
            None,
            false,
            None,
        ),
        InventoryResult::WrongBagType
    );

    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, ObjectGuid::create_item(1, 300), 2)
        .unwrap();
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_START,
            2,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            None,
            None,
            false,
            Some(&regular_bag_proto),
        ),
        InventoryResult::WrongBagType
    );
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_START,
            0,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            None,
            None,
            false,
            Some(&herb_bag_proto),
        ),
        InventoryResult::WrongBagType
    );

    let mut dest = Vec::new();
    let mut count = 3;
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_START,
            0,
            &mut dest,
            &herb,
            &mut count,
            false,
            None,
            None,
            false,
            Some(&herb_bag_proto),
        ),
        InventoryResult::Ok
    );
    assert_eq!(
        dest,
        vec![ItemPosCount::new(
            make_item_pos(INVENTORY_SLOT_BAG_START, 0),
            3
        )]
    );
}

#[test]
fn can_store_item_in_specific_slot_preserves_cpp_keyring_gate_condition() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let mut count = 1;

    assert!(!cpp_keyring_family_gate_applies(KEYRING_SLOT_START));
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            KEYRING_SLOT_START,
            &mut Vec::new(),
            &proto,
            &mut count,
            false,
            None,
            None,
            false,
            None,
        ),
        InventoryResult::Ok
    );
}

#[test]
fn can_store_item_in_inventory_slots_merges_matching_stacks_like_cpp() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let mut matching = Item::default();
    matching
        .object_mut()
        .create(ObjectGuid::create_item(1, 200));
    matching.object_mut().set_entry(6948);
    matching.set_count(16);
    let mut wrong_entry = Item::default();
    wrong_entry
        .object_mut()
        .create(ObjectGuid::create_item(1, 201));
    wrong_entry.object_mut().set_entry(6949);
    wrong_entry.set_count(1);
    let slot_items = [
        ItemSlotRef::new(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, &matching),
        ItemSlotRef::new(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START + 1,
            &wrong_entry,
        ),
    ];
    let mut dest = Vec::new();
    let mut count = 6;

    assert_eq!(
        player.can_store_item_in_inventory_slots(
            INVENTORY_SLOT_ITEM_START,
            INVENTORY_SLOT_ITEM_START + 3,
            &mut dest,
            &proto,
            &mut count,
            true,
            None,
            false,
            NULL_BAG,
            NULL_SLOT,
            &slot_items,
        ),
        InventoryResult::Ok
    );
    assert_eq!(
        dest,
        vec![ItemPosCount::new(
            make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
            4,
        )]
    );
    assert_eq!(count, 2);
}

#[test]
fn can_store_item_in_inventory_slots_allocates_empty_slots_like_cpp() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let mut occupied = Item::default();
    occupied
        .object_mut()
        .create(ObjectGuid::create_item(1, 202));
    occupied.object_mut().set_entry(6948);
    occupied.set_count(1);
    let slot_items = [ItemSlotRef::new(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        &occupied,
    )];
    let mut dest = vec![ItemPosCount::new(
        make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1),
        1,
    )];
    let mut count = 7;

    assert_eq!(
        player.can_store_item_in_inventory_slots(
            INVENTORY_SLOT_ITEM_START,
            INVENTORY_SLOT_ITEM_START + 3,
            &mut dest,
            &proto,
            &mut count,
            false,
            None,
            false,
            NULL_BAG,
            NULL_SLOT,
            &slot_items,
        ),
        InventoryResult::Ok
    );
    assert_eq!(
        dest,
        vec![
            ItemPosCount::new(
                make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1),
                1,
            ),
            ItemPosCount::new(
                make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 2),
                7,
            ),
        ]
    );
    assert_eq!(count, 0);
}

#[test]
fn can_store_item_in_inventory_slots_applies_cpp_source_and_skip_rules() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let mut source = Item::default();
    source.object_mut().create(ObjectGuid::create_item(1, 203));
    source.object_mut().set_entry(6948);
    source.set_count(1);

    assert_eq!(
        player.can_store_item_in_inventory_slots(
            INVENTORY_SLOT_ITEM_START,
            INVENTORY_SLOT_ITEM_START + 1,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            Some(&source),
            true,
            NULL_BAG,
            NULL_SLOT,
            &[],
        ),
        InventoryResult::DestroyNonemptyBag
    );

    let slot_items = [ItemSlotRef::new(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        &source,
    )];
    let mut dest = Vec::new();
    let mut count = 1;
    assert_eq!(
        player.can_store_item_in_inventory_slots(
            INVENTORY_SLOT_ITEM_START,
            INVENTORY_SLOT_ITEM_START + 2,
            &mut dest,
            &proto,
            &mut count,
            false,
            Some(&source),
            false,
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START + 1,
            &slot_items,
        ),
        InventoryResult::Ok
    );
    assert_eq!(
        dest,
        vec![ItemPosCount::new(
            make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
            1,
        )]
    );
    assert_eq!(count, 0);
}

#[test]
fn can_store_item_in_bag_applies_cpp_bag_and_source_guards() {
    let mut player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let regular_bag_proto = ItemStorageTemplate {
        class_id: ItemClass::Container,
        subclass_id: ItemSubClassContainer::Container as u32,
        container_slots: 4,
        ..ItemStorageTemplate::regular_item(100, 1)
    };
    let bag_guid = ObjectGuid::create_item(1, 300);

    assert_eq!(
        player.can_store_item_in_bag(
            INVENTORY_SLOT_BAG_START,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            true,
            None,
            false,
            NULL_BAG,
            NULL_SLOT,
            Some(&regular_bag_proto),
            &[],
        ),
        InventoryResult::WrongBagType
    );

    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 4)
        .unwrap();
    assert_eq!(
        player.can_store_item_in_bag(
            INVENTORY_SLOT_BAG_START,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            true,
            None,
            false,
            INVENTORY_SLOT_BAG_START,
            NULL_SLOT,
            Some(&regular_bag_proto),
            &[],
        ),
        InventoryResult::WrongBagType
    );

    let mut source_bag = Item::default();
    source_bag.object_mut().create(bag_guid);
    assert_eq!(
        player.can_store_item_in_bag(
            INVENTORY_SLOT_BAG_START,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            true,
            Some(&source_bag),
            false,
            NULL_BAG,
            NULL_SLOT,
            Some(&regular_bag_proto),
            &[],
        ),
        InventoryResult::WrongBagType
    );

    let mut source = Item::default();
    source.object_mut().create(ObjectGuid::create_item(1, 301));
    assert_eq!(
        player.can_store_item_in_bag(
            INVENTORY_SLOT_BAG_START,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            true,
            Some(&source),
            true,
            NULL_BAG,
            NULL_SLOT,
            Some(&regular_bag_proto),
            &[],
        ),
        InventoryResult::DestroyNonemptyBag
    );

    source.set_item_flag(ItemFieldFlags::CHILD);
    assert_eq!(
        player.can_store_item_in_bag(
            INVENTORY_SLOT_BAG_START,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            true,
            Some(&source),
            false,
            NULL_BAG,
            NULL_SLOT,
            Some(&regular_bag_proto),
            &[],
        ),
        InventoryResult::WrongBagType3
    );
}

#[test]
fn can_store_item_in_bag_applies_cpp_specialized_mode_and_family_rules() {
    let mut player = Player::new(None, false);
    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, ObjectGuid::create_item(1, 310), 2)
        .unwrap();
    let misc = ItemStorageTemplate::regular_item(6948, 20);
    let herb = ItemStorageTemplate {
        bag_family: BagFamilyMask::HERBS,
        ..ItemStorageTemplate::regular_item(2447, 20)
    };
    let regular_bag_proto = ItemStorageTemplate {
        class_id: ItemClass::Container,
        subclass_id: ItemSubClassContainer::Container as u32,
        container_slots: 2,
        ..ItemStorageTemplate::regular_item(100, 1)
    };
    let herb_bag_proto = ItemStorageTemplate {
        class_id: ItemClass::Container,
        subclass_id: ItemSubClassContainer::HerbContainer as u32,
        container_slots: 2,
        ..ItemStorageTemplate::regular_item(101, 1)
    };

    assert_eq!(
        player.can_store_item_in_bag(
            INVENTORY_SLOT_BAG_START,
            &mut Vec::new(),
            &misc,
            &mut 1,
            false,
            false,
            None,
            false,
            NULL_BAG,
            NULL_SLOT,
            Some(&regular_bag_proto),
            &[],
        ),
        InventoryResult::WrongBagType
    );
    assert_eq!(
        player.can_store_item_in_bag(
            INVENTORY_SLOT_BAG_START,
            &mut Vec::new(),
            &misc,
            &mut 1,
            false,
            false,
            None,
            false,
            NULL_BAG,
            NULL_SLOT,
            Some(&herb_bag_proto),
            &[],
        ),
        InventoryResult::WrongBagType
    );

    let mut dest = Vec::new();
    let mut count = 1;
    assert_eq!(
        player.can_store_item_in_bag(
            INVENTORY_SLOT_BAG_START,
            &mut dest,
            &herb,
            &mut count,
            false,
            false,
            None,
            false,
            NULL_BAG,
            NULL_SLOT,
            Some(&herb_bag_proto),
            &[],
        ),
        InventoryResult::Ok
    );
    assert_eq!(
        dest,
        vec![ItemPosCount::new(
            make_item_pos(INVENTORY_SLOT_BAG_START, 0),
            1,
        )]
    );
}

#[test]
fn can_store_item_in_bag_scans_slots_like_cpp_merge_and_empty_modes() {
    let mut player = Player::new(None, false);
    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, ObjectGuid::create_item(1, 320), 3)
        .unwrap();
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let regular_bag_proto = ItemStorageTemplate {
        class_id: ItemClass::Container,
        subclass_id: ItemSubClassContainer::Container as u32,
        container_slots: 3,
        ..ItemStorageTemplate::regular_item(100, 1)
    };
    let mut matching = Item::default();
    matching
        .object_mut()
        .create(ObjectGuid::create_item(1, 321));
    matching.object_mut().set_entry(6948);
    matching.set_count(16);
    let mut wrong_entry = Item::default();
    wrong_entry
        .object_mut()
        .create(ObjectGuid::create_item(1, 322));
    wrong_entry.object_mut().set_entry(6949);
    wrong_entry.set_count(1);
    let slot_items = [
        ItemSlotRef::new(INVENTORY_SLOT_BAG_START, 0, &matching),
        ItemSlotRef::new(INVENTORY_SLOT_BAG_START, 1, &wrong_entry),
    ];
    let mut merge_dest = Vec::new();
    let mut merge_count = 6;

    assert_eq!(
        player.can_store_item_in_bag(
            INVENTORY_SLOT_BAG_START,
            &mut merge_dest,
            &proto,
            &mut merge_count,
            true,
            true,
            None,
            false,
            NULL_BAG,
            NULL_SLOT,
            Some(&regular_bag_proto),
            &slot_items,
        ),
        InventoryResult::Ok
    );
    assert_eq!(
        merge_dest,
        vec![ItemPosCount::new(
            make_item_pos(INVENTORY_SLOT_BAG_START, 0),
            4,
        )]
    );
    assert_eq!(merge_count, 2);

    let mut empty_dest = Vec::new();
    let mut empty_count = 7;
    assert_eq!(
        player.can_store_item_in_bag(
            INVENTORY_SLOT_BAG_START,
            &mut empty_dest,
            &proto,
            &mut empty_count,
            false,
            true,
            None,
            false,
            NULL_BAG,
            2,
            Some(&regular_bag_proto),
            &slot_items,
        ),
        InventoryResult::Ok
    );
    assert!(empty_dest.is_empty());
    assert_eq!(empty_count, 7);
}

#[test]
fn can_take_more_similar_items_matches_cpp_max_count_guards() {
    let player = Player::new(None, false);
    let unlimited = ItemStorageTemplate::regular_item(6948, 20);

    assert_eq!(
        player.can_take_more_similar_items(CanTakeMoreSimilarItemsArgs {
            proto: None,
            count: 3,
            source_item: None,
            current_item_count: 0,
            limit_category: None,
            current_limit_category_count: 0,
        }),
        CanTakeMoreSimilarItemsOutcome {
            result: InventoryResult::ItemMaxCount,
            no_space_count: Some(3),
            offending_item_id: None,
        }
    );
    assert_eq!(
        player.can_take_more_similar_items(CanTakeMoreSimilarItemsArgs {
            proto: Some(&unlimited),
            count: 3,
            source_item: None,
            current_item_count: 999,
            limit_category: None,
            current_limit_category_count: 0,
        }),
        can_take_more_similar_ok()
    );

    let mut source = Item::default();
    source.set_loot_generated(true);
    assert_eq!(
        player.can_take_more_similar_items(CanTakeMoreSimilarItemsArgs {
            proto: Some(&unlimited),
            count: 3,
            source_item: Some(&source),
            current_item_count: 0,
            limit_category: None,
            current_limit_category_count: 0,
        }),
        CanTakeMoreSimilarItemsOutcome {
            result: InventoryResult::LootGone,
            no_space_count: None,
            offending_item_id: None,
        }
    );

    let limited = ItemStorageTemplate {
        max_count: 10,
        ..ItemStorageTemplate::regular_item(6948, 20)
    };
    assert_eq!(
        player.can_take_more_similar_items(CanTakeMoreSimilarItemsArgs {
            proto: Some(&limited),
            count: 4,
            source_item: None,
            current_item_count: 8,
            limit_category: None,
            current_limit_category_count: 0,
        }),
        CanTakeMoreSimilarItemsOutcome {
            result: InventoryResult::ItemMaxCount,
            no_space_count: Some(2),
            offending_item_id: None,
        }
    );

    let max_int = ItemStorageTemplate {
        max_count: i32::MAX,
        ..ItemStorageTemplate::regular_item(6948, 20)
    };
    assert_eq!(
        player.can_take_more_similar_items(CanTakeMoreSimilarItemsArgs {
            proto: Some(&max_int),
            count: 4,
            source_item: None,
            current_item_count: u32::MAX - 4,
            limit_category: None,
            current_limit_category_count: 0,
        }),
        can_take_more_similar_ok()
    );
}

#[test]
fn can_take_more_similar_items_matches_cpp_limit_category_guards() {
    let player = Player::new(None, false);
    let limited_category = ItemStorageTemplate {
        item_limit_category: 77,
        ..ItemStorageTemplate::regular_item(6948, 20)
    };

    assert_eq!(
        player.can_take_more_similar_items(CanTakeMoreSimilarItemsArgs {
            proto: Some(&limited_category),
            count: 3,
            source_item: None,
            current_item_count: 0,
            limit_category: None,
            current_limit_category_count: 0,
        }),
        CanTakeMoreSimilarItemsOutcome {
            result: InventoryResult::NotEquippable,
            no_space_count: Some(3),
            offending_item_id: None,
        }
    );

    let have_limit = ItemLimitCategoryTemplate {
        id: 77,
        quantity: 5,
        flags: ITEM_LIMIT_CATEGORY_MODE_HAVE,
    };
    assert_eq!(
        player.can_take_more_similar_items(CanTakeMoreSimilarItemsArgs {
            proto: Some(&limited_category),
            count: 3,
            source_item: None,
            current_item_count: 0,
            limit_category: Some(&have_limit),
            current_limit_category_count: 4,
        }),
        CanTakeMoreSimilarItemsOutcome {
            result: InventoryResult::ItemMaxLimitCategoryCountExceededIs,
            no_space_count: Some(2),
            offending_item_id: Some(6948),
        }
    );

    let equip_limit = ItemLimitCategoryTemplate {
        id: 77,
        quantity: 1,
        flags: ITEM_LIMIT_CATEGORY_MODE_EQUIP,
    };
    assert_eq!(
        player.can_take_more_similar_items(CanTakeMoreSimilarItemsArgs {
            proto: Some(&limited_category),
            count: 99,
            source_item: None,
            current_item_count: 0,
            limit_category: Some(&equip_limit),
            current_limit_category_count: 99,
        }),
        can_take_more_similar_ok()
    );
}

#[test]
fn item_count_by_entry_matches_cpp_locations_and_skip_item() {
    let player = Player::new(None, false);
    let mut inventory_item = Item::default();
    inventory_item
        .object_mut()
        .create(ObjectGuid::create_item(1, 610));
    inventory_item.object_mut().set_entry(6948);
    inventory_item.set_count(2);
    let mut bank_item = Item::default();
    bank_item
        .object_mut()
        .create(ObjectGuid::create_item(1, 611));
    bank_item.object_mut().set_entry(6948);
    bank_item.set_count(3);
    let mut other_item = Item::default();
    other_item
        .object_mut()
        .create(ObjectGuid::create_item(1, 612));
    other_item.object_mut().set_entry(6949);
    other_item.set_count(7);
    let stored = [
        ItemStorageRef::new(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            &inventory_item,
            None,
        ),
        ItemStorageRef::new(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, &bank_item, None),
        ItemStorageRef::new(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START + 1,
            &other_item,
            None,
        ),
    ];

    assert_eq!(player.item_count_by_entry(6948, false, None, &stored), 2);
    assert_eq!(player.item_count_by_entry(6948, true, None, &stored), 5);
    assert_eq!(
        player.item_count_by_entry(6948, true, Some(&inventory_item), &stored),
        3
    );
}

#[test]
fn item_count_with_limit_category_matches_cpp_everywhere_and_skip_item() {
    let player = Player::new(None, false);
    let limited_template = ItemStorageTemplate {
        item_limit_category: 77,
        ..ItemStorageTemplate::regular_item(6948, 20)
    };
    let other_template = ItemStorageTemplate {
        item_limit_category: 78,
        ..ItemStorageTemplate::regular_item(6949, 20)
    };
    let mut limited_item = Item::default();
    limited_item
        .object_mut()
        .create(ObjectGuid::create_item(1, 620));
    limited_item.object_mut().set_entry(6948);
    limited_item.set_count(2);
    let mut bank_limited_item = Item::default();
    bank_limited_item
        .object_mut()
        .create(ObjectGuid::create_item(1, 621));
    bank_limited_item.object_mut().set_entry(6948);
    bank_limited_item.set_count(3);
    let mut other_item = Item::default();
    other_item
        .object_mut()
        .create(ObjectGuid::create_item(1, 622));
    other_item.object_mut().set_entry(6949);
    other_item.set_count(7);
    let stored = [
        ItemStorageRef::new(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            &limited_item,
            Some(&limited_template),
        ),
        ItemStorageRef::new(
            INVENTORY_SLOT_BAG_0,
            BANK_SLOT_ITEM_START,
            &bank_limited_item,
            Some(&limited_template),
        ),
        ItemStorageRef::new(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START + 1,
            &other_item,
            Some(&other_template),
        ),
    ];

    assert_eq!(player.item_count_with_limit_category(77, None, &stored), 5);
    assert_eq!(
        player.item_count_with_limit_category(77, Some(&limited_item), &stored),
        3
    );
}

#[test]
fn item_by_entry_matches_cpp_for_each_item_order_and_stop() {
    let mut player = Player::new(None, false);
    player.set_inventory_slot_count(INVENTORY_DEFAULT_SIZE);

    let equipped = item_with_guid_entry(640, 900);
    let inventory_bag = item_with_guid_entry(641, 900);
    let inventory_item = item_with_guid_entry(642, 900);
    let bag_item = item_with_guid_entry(643, 900);
    let bank_item = item_with_guid_entry(644, 900);

    player
        .store_top_level_item(EQUIPMENT_SLOT_CHEST, equipped.object().guid())
        .unwrap();
    player
        .store_top_level_item(INVENTORY_SLOT_BAG_START, inventory_bag.object().guid())
        .unwrap();
    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, inventory_bag.object().guid(), 4)
        .unwrap();
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, inventory_item.object().guid())
        .unwrap();
    player
        .store_bag_item(INVENTORY_SLOT_BAG_START, 0, bag_item.object().guid())
        .unwrap();
    player
        .store_top_level_item(BANK_SLOT_ITEM_START, bank_item.object().guid())
        .unwrap();

    let stored = [
        ItemStorageRef::new(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, &bank_item, None),
        ItemStorageRef::new(INVENTORY_SLOT_BAG_START, 0, &bag_item, None),
        ItemStorageRef::new(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            &inventory_item,
            None,
        ),
        ItemStorageRef::new(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_BAG_START,
            &inventory_bag,
            None,
        ),
        ItemStorageRef::new(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST, &equipped, None),
    ];

    let default_found = player
        .item_by_entry(900, ItemSearchLocation::DEFAULT, &stored)
        .unwrap();
    assert_eq!(default_found.item.object().guid(), equipped.object().guid());

    let inventory_found = player
        .item_by_entry(900, ItemSearchLocation::INVENTORY, &stored)
        .unwrap();
    assert_eq!(
        inventory_found.item.object().guid(),
        inventory_bag.object().guid()
    );

    assert!(
        player
            .item_by_entry(901, ItemSearchLocation::EVERYWHERE, &stored)
            .is_none()
    );
}

#[test]
fn item_list_by_entry_matches_cpp_locations_bank_and_reagent_order() {
    let mut player = Player::new(None, false);
    player.set_inventory_slot_count(INVENTORY_DEFAULT_SIZE);

    let equipped = item_with_guid_entry(650, 901);
    let inventory_item = item_with_guid_entry(651, 901);
    let bank_item = item_with_guid_entry(652, 901);
    let reagent_bag = item_with_guid_entry(653, 1);
    let reagent_item = item_with_guid_entry(654, 901);
    let other_item = item_with_guid_entry(655, 902);

    player
        .store_top_level_item(EQUIPMENT_SLOT_HEAD, equipped.object().guid())
        .unwrap();
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, inventory_item.object().guid())
        .unwrap();
    player
        .store_top_level_item(BANK_SLOT_ITEM_START, bank_item.object().guid())
        .unwrap();
    player
        .store_top_level_item(REAGENT_BAG_SLOT_START, reagent_bag.object().guid())
        .unwrap();
    player
        .register_bag_storage(REAGENT_BAG_SLOT_START, reagent_bag.object().guid(), 3)
        .unwrap();
    player
        .store_bag_item(REAGENT_BAG_SLOT_START, 1, reagent_item.object().guid())
        .unwrap();
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START + 1, other_item.object().guid())
        .unwrap();

    let stored = [
        ItemStorageRef::new(REAGENT_BAG_SLOT_START, 1, &reagent_item, None),
        ItemStorageRef::new(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, &bank_item, None),
        ItemStorageRef::new(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            &inventory_item,
            None,
        ),
        ItemStorageRef::new(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_HEAD, &equipped, None),
        ItemStorageRef::new(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START + 1,
            &other_item,
            None,
        ),
    ];

    let without_bank = player.item_list_by_entry(901, false, &stored);
    assert_eq!(
        without_bank
            .iter()
            .map(|stored| stored.item.object().guid())
            .collect::<Vec<_>>(),
        vec![
            equipped.object().guid(),
            inventory_item.object().guid(),
            reagent_item.object().guid(),
        ]
    );

    let with_bank = player.item_list_by_entry(901, true, &stored);
    assert_eq!(
        with_bank
            .iter()
            .map(|stored| stored.item.object().guid())
            .collect::<Vec<_>>(),
        vec![
            equipped.object().guid(),
            inventory_item.object().guid(),
            bank_item.object().guid(),
            reagent_item.object().guid(),
        ]
    );
}

#[test]
fn can_store_item_preflight_matches_cpp_template_source_and_similar_guards() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 20);

    assert_eq!(
        player.can_store_item(
            &mut Vec::new(),
            can_store_args(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, None, 3),
        ),
        CanStoreItemOutcome {
            result: InventoryResult::ItemNotFound,
            no_space_count: Some(3),
        }
    );

    let mut swap_missing = can_store_args(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, None, 3);
    swap_missing.swap = true;
    assert_eq!(
        player.can_store_item(&mut Vec::new(), swap_missing),
        CanStoreItemOutcome {
            result: InventoryResult::CantSwap,
            no_space_count: Some(3),
        }
    );

    let mut source = Item::default();
    source.set_loot_generated(true);
    let mut loot_args = can_store_args(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        Some(&proto),
        3,
    );
    loot_args.source_item = Some(&source);
    assert_eq!(
        player.can_store_item(&mut Vec::new(), loot_args),
        CanStoreItemOutcome {
            result: InventoryResult::LootGone,
            no_space_count: Some(3),
        }
    );

    source.set_loot_generated(false);
    source.set_owner_guid(ObjectGuid::create_player(1, 42));
    source.set_item_flag(ItemFieldFlags::SOULBOUND);
    let mut bound_args = can_store_args(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        Some(&proto),
        3,
    );
    bound_args.source_item = Some(&source);
    assert_eq!(
        player.can_store_item(&mut Vec::new(), bound_args),
        CanStoreItemOutcome {
            result: InventoryResult::NotOwner,
            no_space_count: Some(3),
        }
    );

    let limited_proto = ItemStorageTemplate {
        max_count: 3,
        ..ItemStorageTemplate::regular_item(6948, 20)
    };
    let mut similar_args = can_store_args(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        Some(&limited_proto),
        3,
    );
    let mut existing_limited = Item::default();
    existing_limited
        .object_mut()
        .create(ObjectGuid::create_item(1, 501));
    existing_limited.object_mut().set_entry(6948);
    existing_limited.set_count(3);
    let stored_limited = [ItemStorageRef::new(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START + 1,
        &existing_limited,
        Some(&limited_proto),
    )];
    similar_args.stored_items = &stored_limited;
    assert_eq!(
        player.can_store_item(&mut Vec::new(), similar_args),
        CanStoreItemOutcome {
            result: InventoryResult::ItemMaxCount,
            no_space_count: Some(3),
        }
    );
}

#[test]
fn can_store_item_reports_item_max_count_after_partial_similar_limit_like_cpp() {
    let mut player = Player::new(None, false);
    player.set_inventory_slot_count(16);
    let proto = ItemStorageTemplate {
        max_count: 10,
        ..ItemStorageTemplate::regular_item(6948, 20)
    };
    let mut args = can_store_args(NULL_BAG, NULL_SLOT, Some(&proto), 5);
    let mut existing_limited = Item::default();
    existing_limited
        .object_mut()
        .create(ObjectGuid::create_item(1, 502));
    existing_limited.object_mut().set_entry(6948);
    existing_limited.set_count(7);
    let stored_limited = [ItemStorageRef::new(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START + 1,
        &existing_limited,
        Some(&proto),
    )];
    args.stored_items = &stored_limited;
    let mut dest = Vec::new();

    assert_eq!(
        player.can_store_item(&mut dest, args),
        CanStoreItemOutcome {
            result: InventoryResult::ItemMaxCount,
            no_space_count: Some(2),
        }
    );
    assert_eq!(
        dest,
        vec![ItemPosCount::new(
            make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
            3,
        )]
    );
}

#[test]
fn can_store_item_fills_specific_slot_then_continues_search_like_cpp() {
    let mut player = Player::new(None, false);
    player.set_inventory_slot_count(16);
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let mut existing = Item::default();
    existing
        .object_mut()
        .create(ObjectGuid::create_item(1, 401));
    existing.object_mut().set_entry(6948);
    existing.set_count(15);
    let slot_items = [ItemSlotRef::new(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        &existing,
    )];
    let mut args = can_store_args(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        Some(&proto),
        10,
    );
    args.slot_items = &slot_items;
    let mut dest = Vec::new();

    assert_eq!(
        player.can_store_item(&mut dest, args),
        CanStoreItemOutcome {
            result: InventoryResult::Ok,
            no_space_count: None,
        }
    );
    assert_eq!(
        dest,
        vec![
            ItemPosCount::new(
                make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
                5,
            ),
            ItemPosCount::new(
                make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1),
                5,
            ),
        ]
    );
}

#[test]
fn can_store_item_general_search_handles_new_bag_direct_equip_and_bag_in_bag() {
    let mut player = Player::new(None, false);
    player.set_inventory_slot_count(16);
    let bag_proto = ItemStorageTemplate {
        class_id: ItemClass::Container,
        subclass_id: ItemSubClassContainer::Container as u32,
        bonding: ItemBondingType::None,
        max_stack_size: 1,
        container_slots: 16,
        ..ItemStorageTemplate::regular_item(100, 1)
    };
    let mut dest = Vec::new();

    assert_eq!(
        player.can_store_item(
            &mut dest,
            can_store_args(NULL_BAG, NULL_SLOT, Some(&bag_proto), 1)
        ),
        CanStoreItemOutcome {
            result: InventoryResult::Ok,
            no_space_count: None,
        }
    );
    assert_eq!(
        dest,
        vec![ItemPosCount::new(
            make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START),
            1,
        )]
    );

    let source = Item::default();
    let mut bag_in_bag_args = can_store_args(NULL_BAG, NULL_SLOT, Some(&bag_proto), 1);
    bag_in_bag_args.source_item = Some(&source);
    bag_in_bag_args.source_is_not_empty_bag = true;
    assert_eq!(
        player.can_store_item(&mut Vec::new(), bag_in_bag_args),
        CanStoreItemOutcome {
            result: InventoryResult::BagInBag,
            no_space_count: None,
        }
    );
}

#[test]
fn can_bank_item_preflight_matches_cpp_item_template_and_source_guards() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let mut source = Item::default();
    source.object_mut().create(ObjectGuid::create_item(1, 700));
    source.object_mut().set_entry(6948);
    source.set_count(3);

    assert_eq!(
        player.can_bank_item(
            &mut Vec::new(),
            can_bank_args(
                INVENTORY_SLOT_BAG_0,
                BANK_SLOT_ITEM_START,
                Some(&proto),
                None
            ),
        ),
        InventoryResult::ItemNotFound
    );

    let mut missing_swap = can_bank_args(
        INVENTORY_SLOT_BAG_0,
        BANK_SLOT_ITEM_START,
        Some(&proto),
        None,
    );
    missing_swap.swap = true;
    assert_eq!(
        player.can_bank_item(&mut Vec::new(), missing_swap),
        InventoryResult::CantSwap
    );

    assert_eq!(
        player.can_bank_item(
            &mut Vec::new(),
            can_bank_args(
                INVENTORY_SLOT_BAG_0,
                BANK_SLOT_ITEM_START,
                None,
                Some(&source)
            ),
        ),
        InventoryResult::ItemNotFound
    );

    source.set_loot_generated(true);
    assert_eq!(
        player.can_bank_item(
            &mut Vec::new(),
            can_bank_args(
                INVENTORY_SLOT_BAG_0,
                BANK_SLOT_ITEM_START,
                Some(&proto),
                Some(&source),
            ),
        ),
        InventoryResult::LootGone
    );

    source.set_loot_generated(false);
    source.set_owner_guid(ObjectGuid::create_player(1, 42));
    source.set_item_flag(ItemFieldFlags::SOULBOUND);
    assert_eq!(
        player.can_bank_item(
            &mut Vec::new(),
            can_bank_args(
                INVENTORY_SLOT_BAG_0,
                BANK_SLOT_ITEM_START,
                Some(&proto),
                Some(&source),
            ),
        ),
        InventoryResult::NotOwner
    );

    source.remove_item_flag(ItemFieldFlags::SOULBOUND);
    let mut currency_args = can_bank_args(
        INVENTORY_SLOT_BAG_0,
        BANK_SLOT_ITEM_START,
        Some(&proto),
        Some(&source),
    );
    currency_args.source_is_currency_token = true;
    assert_eq!(
        player.can_bank_item(&mut Vec::new(), currency_args),
        InventoryResult::CantSwap
    );

    let limited_proto = ItemStorageTemplate {
        max_count: 3,
        ..proto
    };
    let mut existing = Item::default();
    existing
        .object_mut()
        .create(ObjectGuid::create_item(1, 701));
    existing.object_mut().set_entry(6948);
    existing.set_count(3);
    let stored = [ItemStorageRef::new(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        &existing,
        Some(&limited_proto),
    )];
    let mut limit_args = can_bank_args(
        INVENTORY_SLOT_BAG_0,
        BANK_SLOT_ITEM_START,
        Some(&limited_proto),
        Some(&source),
    );
    limit_args.stored_items = &stored;
    assert_eq!(
        player.can_bank_item(&mut Vec::new(), limit_args),
        InventoryResult::ItemMaxCount
    );
}

#[test]
fn can_bank_item_specific_bank_bag_slot_matches_cpp_guards() {
    let mut player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 1);
    let mut source = Item::default();
    source.object_mut().create(ObjectGuid::create_item(1, 710));
    source.object_mut().set_entry(6948);
    source.set_count(1);

    assert_eq!(
        player.can_bank_item(
            &mut Vec::new(),
            can_bank_args(
                INVENTORY_SLOT_BAG_0,
                BANK_SLOT_BAG_START,
                Some(&proto),
                Some(&source),
            ),
        ),
        InventoryResult::WrongSlot
    );

    let mut bag_args = can_bank_args(
        INVENTORY_SLOT_BAG_0,
        BANK_SLOT_BAG_START,
        Some(&proto),
        Some(&source),
    );
    bag_args.source_is_bag = true;
    assert_eq!(
        player.can_bank_item(&mut Vec::new(), bag_args),
        InventoryResult::NoBankSlot
    );

    player.set_bank_bag_slot_count(1);
    bag_args.can_use_result = InventoryResult::CantUseItem;
    assert_eq!(
        player.can_bank_item(&mut Vec::new(), bag_args),
        InventoryResult::CantUseItem
    );
}

#[test]
fn can_bank_item_fills_specific_slot_then_continues_bank_search_like_cpp() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let mut source = Item::default();
    source.object_mut().create(ObjectGuid::create_item(1, 720));
    source.object_mut().set_entry(6948);
    source.set_count(10);
    let mut existing = Item::default();
    existing
        .object_mut()
        .create(ObjectGuid::create_item(1, 721));
    existing.object_mut().set_entry(6948);
    existing.set_count(15);
    let slot_items = [ItemSlotRef::new(
        INVENTORY_SLOT_BAG_0,
        BANK_SLOT_ITEM_START,
        &existing,
    )];
    let mut args = can_bank_args(
        INVENTORY_SLOT_BAG_0,
        BANK_SLOT_ITEM_START,
        Some(&proto),
        Some(&source),
    );
    args.slot_items = &slot_items;
    let mut dest = Vec::new();

    assert_eq!(player.can_bank_item(&mut dest, args), InventoryResult::Ok);
    assert_eq!(
        dest,
        vec![
            ItemPosCount::new(make_item_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START), 5),
            ItemPosCount::new(
                make_item_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START + 1),
                5
            ),
        ]
    );
}

#[test]
fn can_bank_item_general_search_and_full_bank_match_cpp() {
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let mut source = Item::default();
    source.object_mut().create(ObjectGuid::create_item(1, 730));
    source.object_mut().set_entry(6948);
    source.set_count(3);
    let player = Player::new(None, false);
    let mut dest = Vec::new();

    assert_eq!(
        player.can_bank_item(
            &mut dest,
            can_bank_args(NULL_BAG, NULL_SLOT, Some(&proto), Some(&source)),
        ),
        InventoryResult::Ok
    );
    assert_eq!(
        dest,
        vec![ItemPosCount::new(
            make_item_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START),
            3,
        )]
    );

    let mut occupied_items = Vec::new();
    for idx in 0..(BANK_SLOT_ITEM_END - BANK_SLOT_ITEM_START) {
        let mut occupied = Item::default();
        occupied
            .object_mut()
            .create(ObjectGuid::create_item(1, 800 + idx as i64));
        occupied.object_mut().set_entry(9999);
        occupied.set_count(1);
        occupied_items.push(occupied);
    }
    let slot_items = occupied_items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            ItemSlotRef::new(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START + idx as u8, item)
        })
        .collect::<Vec<_>>();
    let mut full_args = can_bank_args(NULL_BAG, NULL_SLOT, Some(&proto), Some(&source));
    full_args.slot_items = &slot_items;
    assert_eq!(
        player.can_bank_item(&mut Vec::new(), full_args),
        InventoryResult::BankFull
    );
}

#[test]
fn player_identity_setters_mark_cpp_unit_and_playerdata_bits() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();

    player.set_race_class_gender(1, 2, Gender::Female);
    player.set_selection(ObjectGuid::new(7, 11));

    assert_eq!(player.unit().data().race, 1);
    assert_eq!(player.unit().data().class_id, 2);
    assert_eq!(player.unit().data().player_class_id, 2);
    assert_eq!(player.unit().data().sex, Gender::Female as u8);
    assert_eq!(player.data().native_sex, Gender::Female as u8);
    assert_eq!(player.unit().data().target, ObjectGuid::new(7, 11));
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_NATIVE_SEX_BIT)
    );
}

#[test]
fn player_flags_and_loot_guid_mark_playerdata_bits() {
    let mut player = Player::new(None, false);

    player.set_player_flag(0x20);
    player.set_player_flag_ex(0x04);
    player.set_loot_guid(ObjectGuid::new(9, 3));
    player.set_bank_bag_slot_count(6);
    player.set_primary_specialization(62);
    player.set_honor_level_like_cpp(3);

    assert!(player.has_player_flag(0x20));
    assert!(player.has_player_flag_ex(0x04));
    assert_eq!(player.data().loot_target_guid, ObjectGuid::new(9, 3));
    assert_eq!(player.data().num_bank_slots, 6);
    assert_eq!(player.data().current_spec_id, 62);
    assert_eq!(player.data().honor_level, 3);
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_PARENT_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_FLAGS_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_FLAGS_EX_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_LOOT_TARGET_GUID_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_NUM_BANK_SLOTS_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_CURRENT_SPEC_ID_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_HONOR_LEVEL_BIT)
    );

    player.remove_player_flag(0x20);
    player.remove_player_flag_ex(0x04);
    assert!(!player.has_player_flag(0x20));
    assert!(!player.has_player_flag_ex(0x04));
}

#[test]
fn set_inebriation_matches_cpp_clamp_and_marks_playerdata_bit() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();

    player.set_inebriation_like_cpp(55);

    assert_eq!(player.inebriation_like_cpp(), 55);
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_PARENT_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_INEBRIATION_BIT)
    );

    player.clear_data_changes();
    player.set_inebriation_like_cpp(150);

    assert_eq!(player.inebriation_like_cpp(), 100);
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_INEBRIATION_BIT)
    );
}

#[test]
fn money_matches_cpp_modify_clamps_and_active_playerdata_coinage_bit() {
    let mut player = Player::new(None, false);

    player.set_money(100);
    assert_eq!(player.active_data().coinage, 100);
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_COINAGE_BIT)
    );

    assert!(player.modify_money(-150));
    assert_eq!(player.active_data().coinage, 0);

    player.set_money(MAX_MONEY_AMOUNT - 1);
    assert!(!player.modify_money(2));
    assert_eq!(player.active_data().coinage, MAX_MONEY_AMOUNT - 1);
    assert!(!player.modify_money(i64::MAX));
    assert_eq!(player.active_data().coinage, MAX_MONEY_AMOUNT - 1);

    assert!(player.modify_money(1));
    assert_eq!(player.active_data().coinage, MAX_MONEY_AMOUNT);
}

#[test]
fn scaling_player_level_delta_marks_cpp_parent_and_field_bits() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();

    player.set_scaling_player_level_delta_like_cpp(-1);

    assert_eq!(player.active_data().scaling_player_level_delta, -1);
    let mask = player.active_player_data_changes_mask();
    assert_eq!(mask.get_block(0), 1 << ACTIVE_PLAYER_DATA_PARENT_BIT);
    assert_eq!(mask.get_block(1), 0);
    assert_eq!(
        mask.get_block(2),
        (1 << (ACTIVE_PLAYER_DATA_SCALING_PLAYER_LEVEL_DELTA_PARENT_BIT - 64))
            | (1 << (ACTIVE_PLAYER_DATA_SCALING_PLAYER_LEVEL_DELTA_BIT - 64))
    );
    assert!(mask.blocks()[3..].iter().all(|block| *block == 0));

    player.clear_data_changes();
    player.set_scaling_player_level_delta_like_cpp(-1);
    assert!(!player.active_player_data_changes_mask().is_any_set());
    player.mark_scaling_player_level_delta_changed_like_cpp();
    let mask = player.active_player_data_changes_mask();
    assert_eq!(mask.get_block(0), 1 << ACTIVE_PLAYER_DATA_PARENT_BIT);
    assert_eq!(
        mask.get_block(2),
        (1 << (ACTIVE_PLAYER_DATA_SCALING_PLAYER_LEVEL_DELTA_PARENT_BIT - 64))
            | (1 << (ACTIVE_PLAYER_DATA_SCALING_PLAYER_LEVEL_DELTA_BIT - 64))
    );
}

#[test]
fn active_player_fields_and_inventory_slots_mark_cpp_bits() {
    let mut player = Player::new(None, false);

    player.set_xp(123);
    player.set_next_level_xp(456);
    player.set_honor_like_cpp(789);
    player.set_honor_next_level_like_cpp(8_800);
    player.set_free_primary_professions(2);
    player.set_watched_faction_index_like_cpp(42);
    player.set_inventory_slot_count(16);
    player.set_inv_slot(3, ObjectGuid::new(4, 5));

    assert_eq!(player.active_data().xp, 123);
    assert_eq!(player.active_data().next_level_xp, 456);
    assert_eq!(player.active_data().honor, 789);
    assert_eq!(player.active_data().honor_next_level, 8_800);
    assert_eq!(player.active_data().character_points, 2);
    assert_eq!(player.active_data().watched_faction_index, 42);
    assert_eq!(player.active_data().num_backpack_slots, 16);
    assert_eq!(player.active_data().inv_slots[3], ObjectGuid::new(4, 5));
    assert_eq!(player.active_data().buyback_price, [0; BUYBACK_SLOT_COUNT]);
    assert_eq!(
        player.active_data().buyback_timestamp,
        [0; BUYBACK_SLOT_COUNT]
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_XP_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_NEXT_LEVEL_XP_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_HONOR_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_HONOR_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_HONOR_NEXT_LEVEL_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_CHARACTER_POINTS_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_WATCHED_FACTION_INDEX_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_NUM_BACKPACK_SLOTS_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_INV_SLOTS_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_INV_SLOTS_FIRST_BIT + 3)
    );
    assert_eq!(player.active_data().transmog, Vec::<u32>::new());
    assert_eq!(player.active_data().transmog_update_mask, None);

    let transmog_slot = player.add_transmog_block_like_cpp(0);
    assert_eq!(transmog_slot, 0);
    assert_eq!(player.active_data().transmog, vec![0]);
    assert_eq!(player.active_data().transmog_update_mask, Some(vec![1]));
    assert!(player.add_transmog_flag_like_cpp(transmog_slot, 1 << 7));
    assert_eq!(player.active_data().transmog, vec![1 << 7]);
    assert!(!player.add_transmog_flag_like_cpp(10, 1));
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_TRANSMOG_BIT)
    );

    player.clear_data_changes();
    assert_eq!(player.active_data().conditional_transmog, Vec::<i32>::new());
    assert_eq!(player.active_data().conditional_transmog_update_mask, None);
    assert_eq!(player.add_conditional_transmog_like_cpp(65), 0);
    assert_eq!(player.add_conditional_transmog_like_cpp(96), 1);
    assert_eq!(player.active_data().conditional_transmog, vec![65, 96]);
    assert_eq!(
        player.active_data().conditional_transmog_update_mask,
        Some(vec![0b11])
    );
    assert!(player.remove_conditional_transmog_like_cpp(65));
    assert_eq!(player.active_data().conditional_transmog, vec![96]);
    assert_eq!(
        player.active_data().conditional_transmog_update_mask,
        Some(vec![0b11])
    );
    assert!(!player.remove_conditional_transmog_like_cpp(65));
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_CONDITIONAL_TRANSMOG_BIT)
    );
}

#[test]
fn add_honor_xp_matches_cpp_level_gate_threshold_and_max_shape() {
    let mut low_level = Player::new(None, false);
    assert!(!low_level.add_honor_xp_like_cpp(100, PLAYER_LEVEL_MIN_HONOR_LIKE_CPP - 1));
    assert_eq!(low_level.active_data().honor, 0);
    assert!(!low_level.active_player_data_changes_mask().is_any_set());

    let mut player = Player::new(None, false);
    assert!(player.add_honor_xp_like_cpp(PLAYER_HONOR_NEXT_LEVEL_XP_LIKE_CPP as u32 + 25, 10));
    assert_eq!(player.data().honor_level, 1);
    assert_eq!(player.active_data().honor, 25);
    assert_eq!(
        player.active_data().honor_next_level,
        PLAYER_HONOR_NEXT_LEVEL_XP_LIKE_CPP
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_HONOR_LEVEL_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_HONOR_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_HONOR_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_HONOR_NEXT_LEVEL_BIT)
    );

    player.clear_data_changes();
    player.set_honor_level_like_cpp(PLAYER_MAX_HONOR_LEVEL_LIKE_CPP - 1);
    player.set_honor_like_cpp(PLAYER_HONOR_NEXT_LEVEL_XP_LIKE_CPP - 1);
    player.clear_data_changes();

    assert!(player.add_honor_xp_like_cpp(1, 80));
    assert_eq!(player.data().honor_level, PLAYER_MAX_HONOR_LEVEL_LIKE_CPP);
    assert_eq!(player.active_data().honor, 0);
}

#[test]
fn quest_completed_bit_zero_does_not_mutate_or_mark_mask_like_cpp() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();

    assert!(!player.set_quest_completed_bit_like_cpp(0, true));
    assert_eq!(player.quest_completed_block_like_cpp(0), Some(0));
    assert!(!player.active_player_data_changes_mask().is_any_set());
}

#[test]
fn quest_completed_bit_one_sets_block_zero_and_marks_parent_and_child_like_cpp() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();

    assert!(player.set_quest_completed_bit_like_cpp(1, true));
    assert_eq!(player.quest_completed_block_like_cpp(0), Some(1));
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_QUEST_COMPLETED_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_QUEST_COMPLETED_FIRST_BIT)
    );
}

#[test]
fn quest_completed_boundary_bits_map_to_cpp_blocks_and_children() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();

    assert!(player.set_quest_completed_bit_like_cpp(64, true));
    assert_eq!(player.quest_completed_block_like_cpp(0), Some(1u64 << 63));
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_QUEST_COMPLETED_FIRST_BIT)
    );

    player.clear_data_changes();
    assert!(player.set_quest_completed_bit_like_cpp(65, true));
    assert_eq!(player.quest_completed_block_like_cpp(1), Some(1));
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_QUEST_COMPLETED_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_QUEST_COMPLETED_FIRST_BIT + 1)
    );
}

#[test]
fn quest_completed_clear_removes_only_requested_bit_and_marks_changed_block() {
    let mut player = Player::new(None, false);
    assert!(player.set_quest_completed_bit_like_cpp(1, true));
    assert!(player.set_quest_completed_bit_like_cpp(2, true));
    player.clear_data_changes();

    assert!(player.set_quest_completed_bit_like_cpp(2, false));
    assert_eq!(player.quest_completed_block_like_cpp(0), Some(1));
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_QUEST_COMPLETED_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_QUEST_COMPLETED_FIRST_BIT)
    );
}

#[test]
fn quest_completed_out_of_range_does_not_mutate_or_mark_mask_like_cpp() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();

    let out_of_range = (QUESTS_COMPLETED_BITS_SIZE as u32 * QUESTS_COMPLETED_BITS_PER_BLOCK) + 1;
    assert!(!player.set_quest_completed_bit_like_cpp(out_of_range, true));
    assert!(
        player
            .active_data()
            .quest_completed
            .iter()
            .all(|block| *block == 0)
    );
    assert!(!player.active_player_data_changes_mask().is_any_set());
}

#[test]
fn quest_completed_repeated_set_and_clear_without_change_do_not_mark_mask_like_cpp() {
    let mut player = Player::new(None, false);
    assert!(player.set_quest_completed_bit_like_cpp(65, true));
    player.clear_data_changes();

    assert!(!player.set_quest_completed_bit_like_cpp(65, true));
    assert_eq!(player.quest_completed_block_like_cpp(1), Some(1));
    assert!(!player.active_player_data_changes_mask().is_any_set());

    assert!(player.set_quest_completed_bit_like_cpp(65, false));
    player.clear_data_changes();

    assert!(!player.set_quest_completed_bit_like_cpp(65, false));
    assert_eq!(player.quest_completed_block_like_cpp(1), Some(0));
    assert!(!player.active_player_data_changes_mask().is_any_set());
}

#[test]
fn add_explored_zones_ors_mask_and_marks_cpp_parent_and_child_bits() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();

    assert!(player.add_explored_zones_like_cpp(7, 0x0f));
    assert_eq!(player.explored_zones_block_like_cpp(7), Some(0x0f));
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_EXPLORED_ZONES_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_EXPLORED_ZONES_FIRST_BIT + 7)
    );

    player.clear_data_changes();
    assert!(player.add_explored_zones_like_cpp(7, 0xf0));
    assert_eq!(player.explored_zones_block_like_cpp(7), Some(0xff));
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_EXPLORED_ZONES_FIRST_BIT + 7)
    );
}

#[test]
fn add_explored_zones_repeated_and_out_of_range_are_noops_like_cpp() {
    let mut player = Player::new(None, false);
    assert!(player.add_explored_zones_like_cpp(0, u64::MAX));
    player.clear_data_changes();

    assert!(!player.add_explored_zones_like_cpp(0, 1));
    assert_eq!(player.explored_zones_block_like_cpp(0), Some(u64::MAX));
    assert!(!player.active_player_data_changes_mask().is_any_set());

    assert!(!player.add_explored_zones_like_cpp(PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP, u64::MAX));
    assert!(!player.active_player_data_changes_mask().is_any_set());
}

#[test]
fn explored_zones_db_string_parser_matches_cpp_low_high_words() {
    let blocks = parse_explored_zones_db_string_like_cpp("1 2 bad 4 5");

    assert_eq!(blocks[0], 0x0000_0002_0000_0001);
    assert_eq!(blocks[1], 0x0000_0004_0000_0000);
    assert_eq!(blocks[2], 5);
    assert!(blocks[3..].iter().all(|value| *value == 0));
}

#[test]
fn explored_zones_db_string_parser_ignores_tokens_past_cpp_array() {
    let input = std::iter::repeat_n("1", PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP * 2 + 6)
        .collect::<Vec<_>>()
        .join(" ");
    let blocks = parse_explored_zones_db_string_like_cpp(&input);

    assert!(blocks.iter().all(|value| *value == 0x0000_0001_0000_0001));
}

#[test]
fn explored_zones_db_string_serializer_matches_cpp_low_high_order_and_trailing_space() {
    let mut blocks = [0u64; PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP];
    blocks[0] = 0x0000_0002_0000_0001;
    blocks[1] = 0xFFFF_FFFF_8000_0000;

    let serialized = explored_zones_db_string_from_blocks_like_cpp(&blocks);

    assert!(serialized.starts_with("1 2 2147483648 4294967295 0 0 "));
    assert!(serialized.ends_with(' '));
    assert_eq!(
        serialized.split_whitespace().count(),
        PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP * 2
    );
}

#[test]
fn player_load_explored_zones_marks_cpp_parent_and_child_bits() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();

    assert_eq!(player.load_explored_zones_string_like_cpp("1 2 0 0"), 1);
    assert_eq!(
        player.explored_zones_block_like_cpp(0),
        Some(0x0000_0002_0000_0001)
    );
    assert_eq!(
        player
            .explored_zones_db_string_like_cpp()
            .split_whitespace()
            .take(2)
            .collect::<Vec<_>>(),
        vec!["1", "2"]
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_EXPLORED_ZONES_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_EXPLORED_ZONES_FIRST_BIT)
    );
}

#[test]
fn values_update_splits_player_and_active_player_for_receiver() {
    let mut player = Player::new(None, false);

    player.set_player_flag(0x20);
    player.set_money(50);

    let other_view = player.values_update(false);
    assert!(other_view.has_data());
    assert_eq!(other_view.changed_object_type_mask, 1 << TYPEID_PLAYER);
    assert!(other_view.player_data.is_some());
    assert!(other_view.active_player_data.is_none());

    let self_view = player.values_update(true);
    assert_eq!(
        self_view.changed_object_type_mask,
        (1 << TYPEID_PLAYER) | (1 << TYPEID_ACTIVE_PLAYER)
    );
    assert!(self_view.active_player_data.is_some());
}

#[test]
fn set_battle_pet_data_marks_cpp_player_active_and_unit_fields() {
    let mut player = Player::new(None, false);
    let pet_guid = ObjectGuid::create_global(wow_core::guid::HighGuid::BattlePet, 0, 42);
    player.clear_data_changes();

    player.set_battle_pet_data_like_cpp(pet_guid, 3, 17);

    assert_eq!(player.active_data().summoned_battle_pet_guid, pet_guid);
    assert_eq!(player.data().current_battle_pet_breed_quality, 3);
    assert_eq!(player.unit().data().wild_battle_pet_level, 17);
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_SUMMONED_BATTLE_PET_GUID_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_PARENT_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_CURRENT_BATTLE_PET_BREED_QUALITY_BIT)
    );
    assert!(
        player
            .unit()
            .unit_data_changes_mask()
            .is_set(crate::UNIT_DATA_WILD_BATTLE_PET_LEVEL_BIT)
    );
}

#[test]
fn add_heirloom_marks_active_player_heirlooms_and_flags_dynamic_fields_like_cpp() {
    let mut player = Player::new(None, false);
    player.clear_active_player_data_changes();

    assert_eq!(player.add_heirloom_like_cpp(44_000, 0x03), 0);
    assert_eq!(player.heirlooms_like_cpp(), &[44_000]);
    assert_eq!(player.heirloom_flags_like_cpp(), &[0x03]);
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_HEIRLOOMS_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_HEIRLOOM_FLAGS_BIT)
    );
    assert_eq!(player.active_data().heirlooms_update_mask, Some(vec![1]));
    assert_eq!(
        player.active_data().heirloom_flags_update_mask,
        Some(vec![1])
    );

    assert_eq!(player.add_heirloom_like_cpp(44_001, 0x04), 1);
    assert_eq!(player.heirlooms_like_cpp(), &[44_000, 44_001]);
    assert_eq!(player.heirloom_flags_like_cpp(), &[0x03, 0x04]);
    assert_eq!(player.active_data().heirlooms_update_mask, Some(vec![3]));
    assert_eq!(
        player.active_data().heirloom_flags_update_mask,
        Some(vec![3])
    );
}

#[test]
fn set_heirloom_flags_marks_only_heirloom_flags_dynamic_field_like_cpp() {
    let mut player = Player::new(None, false);
    player.add_heirloom_like_cpp(44_000, 0x01);
    player.add_heirloom_like_cpp(44_001, 0x02);
    player.clear_active_player_data_changes();
    player.active_data.heirlooms_update_mask = None;
    player.active_data.heirloom_flags_update_mask = None;

    assert!(player.set_heirloom_flags_like_cpp(1, 0x06));
    assert!(!player.set_heirloom_flags_like_cpp(2, 0x08));

    assert_eq!(player.heirlooms_like_cpp(), &[44_000, 44_001]);
    assert_eq!(player.heirloom_flags_like_cpp(), &[0x01, 0x06]);
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_PARENT_BIT)
    );
    assert!(
        !player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_HEIRLOOMS_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_HEIRLOOM_FLAGS_BIT)
    );
    assert_eq!(player.active_data().heirlooms_update_mask, None);
    assert_eq!(
        player.active_data().heirloom_flags_update_mask,
        Some(vec![0b10])
    );
}

#[test]
fn set_heirloom_marks_only_heirlooms_dynamic_field_like_cpp() {
    let mut player = Player::new(None, false);
    player.add_heirloom_like_cpp(44_000, 0x01);
    player.add_heirloom_like_cpp(44_001, 0x02);
    player.clear_active_player_data_changes();
    player.active_data.heirlooms_update_mask = None;
    player.active_data.heirloom_flags_update_mask = None;

    assert!(player.set_heirloom_like_cpp(0, 44_002));
    assert!(!player.set_heirloom_like_cpp(2, 44_003));

    assert_eq!(player.heirlooms_like_cpp(), &[44_002, 44_001]);
    assert_eq!(player.heirloom_flags_like_cpp(), &[0x01, 0x02]);
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_HEIRLOOMS_BIT)
    );
    assert!(
        !player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_HEIRLOOM_FLAGS_BIT)
    );
    assert_eq!(player.active_data().heirlooms_update_mask, Some(vec![1]));
    assert_eq!(player.active_data().heirloom_flags_update_mask, None);
}

#[test]
fn add_toy_marks_active_player_toys_dynamic_field_like_cpp() {
    let mut player = Player::new(None, false);
    player.clear_active_player_data_changes();

    assert_eq!(player.add_toy_like_cpp(30_000), 0);
    assert_eq!(player.toys_like_cpp(), &[30_000]);
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_TOYS_BIT)
    );
    assert_eq!(player.active_data().toys_update_mask, Some(vec![1]));

    assert_eq!(player.add_toy_like_cpp(30_001), 1);
    assert_eq!(player.toys_like_cpp(), &[30_000, 30_001]);
    assert_eq!(player.active_data().toys_update_mask, Some(vec![3]));
}

#[test]
fn player_inventory_storage_matches_cpp_get_item_by_pos_rules() {
    let mut player = Player::new(None, false);
    player.set_inventory_slot_count(INVENTORY_DEFAULT_SIZE);
    player.clear_active_player_data_changes();

    let equipped = ObjectGuid::create_item(1, 100);
    let bag_guid = ObjectGuid::create_item(1, 200);
    let bag_item = ObjectGuid::create_item(1, 201);
    let buyback = ObjectGuid::create_item(1, 300);

    player.store_top_level_item(0, equipped).unwrap();
    player
        .store_top_level_item(INVENTORY_SLOT_BAG_START, bag_guid)
        .unwrap();
    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 4)
        .unwrap();
    player
        .store_bag_item(INVENTORY_SLOT_BAG_START, 2, bag_item)
        .unwrap();
    player
        .store_top_level_item(BUYBACK_SLOT_START, buyback)
        .unwrap();

    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, 0),
        Some(equipped)
    );
    assert_eq!(
        player.get_item_by_packed_pos((u16::from(INVENTORY_SLOT_BAG_0) << 8) | 0),
        Some(equipped)
    );
    assert_eq!(
        player.get_bag_by_pos(INVENTORY_SLOT_BAG_START),
        Some(bag_guid)
    );
    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_START, 2),
        Some(bag_item)
    );
    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, BUYBACK_SLOT_START),
        None
    );
    assert_eq!(
        player.get_item_from_buyback_slot(BUYBACK_SLOT_START),
        Some(buyback)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_INV_SLOTS_FIRST_BIT)
    );
}

#[test]
fn visible_item_slot_marks_cpp_playerdata_array_bits() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();

    let visible = VisibleItemValues {
        item_id: 19019,
        item_appearance_mod_id: 7,
        item_visual: 3,
    };
    player.set_visible_item_slot(15, Some(visible));

    assert_eq!(player.data().visible_items[15], visible);
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_VISIBLE_ITEMS_PARENT_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_VISIBLE_ITEMS_FIRST_BIT + 15)
    );

    player.clear_player_data_changes();
    player.set_visible_item_slot(15, None);
    assert_eq!(
        player.data().visible_items[15],
        VisibleItemValues::default()
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_VISIBLE_ITEMS_FIRST_BIT + 15)
    );
}

#[test]
fn explicit_markers_force_default_value_deltas_like_cpp_live_object_masks() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();

    player.mark_inv_slot_changed(0);
    player.mark_visible_item_slot_changed(0);
    player.mark_buyback_price_changed(0);
    player.mark_buyback_timestamp_changed(0);

    assert_eq!(player.active_data().inv_slots[0], ObjectGuid::EMPTY);
    assert_eq!(player.data().visible_items[0], VisibleItemValues::default());
    assert_eq!(player.active_data().buyback_price[0], 0);
    assert_eq!(player.active_data().buyback_timestamp[0], 0);
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_INV_SLOTS_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_INV_SLOTS_FIRST_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_VISIBLE_ITEMS_PARENT_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_VISIBLE_ITEMS_FIRST_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_BUYBACK_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_BUYBACK_PRICE_FIRST_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_BUYBACK_TIMESTAMP_FIRST_BIT)
    );
}

#[test]
fn visualize_item_updates_equipment_storage_and_visible_item_like_cpp() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();
    player.clear_active_player_data_changes();

    let guid = ObjectGuid::create_item(1, 500);
    let visible = VisibleItemValues {
        item_id: 500,
        item_appearance_mod_id: 1,
        item_visual: 2,
    };

    player.visualize_item(0, guid, visible).unwrap();

    assert_eq!(player.get_item_by_pos(INVENTORY_SLOT_BAG_0, 0), Some(guid));
    assert_eq!(player.active_data().inv_slots[0], guid);
    assert_eq!(player.data().visible_items[0], visible);
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_INV_SLOTS_FIRST_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_VISIBLE_ITEMS_FIRST_BIT)
    );

    player.remove_top_level_item(0).unwrap();
    assert_eq!(player.data().visible_items[0], VisibleItemValues::default());
    assert_eq!(player.active_data().inv_slots[0], ObjectGuid::EMPTY);
}

#[test]
fn visualize_item_object_mutates_item_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 500);
    let mut player = Player::new(None, false);
    let mut item = Item::default();
    let visible = VisibleItemValues {
        item_id: 500,
        item_appearance_mod_id: 1,
        item_visual: 2,
    };

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player.clear_data_changes();
    player.clear_active_player_data_changes();
    item.object_mut().create(item_guid);
    item.set_container_guid_and_slot(ObjectGuid::create_item(1, 700), 4);
    item.set_bonding(ItemBondingType::OnEquip);
    item.force_state(ItemUpdateState::Unchanged);
    item.clear_item_data_changes();

    player.visualize_item_object(0, &mut item, visible).unwrap();

    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, 0),
        Some(item_guid)
    );
    assert_eq!(player.active_data().inv_slots[0], item_guid);
    assert_eq!(player.data().visible_items[0], visible);
    assert_eq!(item.data().contained_in, player_guid);
    assert_eq!(item.owner_guid(), player_guid);
    assert_eq!(item.slot(), 0);
    assert_eq!(item.container_guid(), ObjectGuid::EMPTY);
    assert_eq!(item.bag_slot(), INVENTORY_SLOT_BAG_0);
    assert!(item.is_soul_bound());
    assert_eq!(item.update_state(), ItemUpdateState::Changed);
}

#[test]
fn equip_item_object_empty_slot_visualizes_and_flags_item_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 510);
    let mut player = Player::new(None, false);
    let mut item = Item::default();
    let visible = VisibleItemValues {
        item_id: 510,
        item_appearance_mod_id: 4,
        item_visual: 9,
    };

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    item.object_mut().create(item_guid);
    item.set_bonding(ItemBondingType::OnEquip);
    item.force_state(ItemUpdateState::Unchanged);
    item.clear_item_data_changes();

    assert_eq!(
        player
            .equip_item_object(
                make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND),
                &mut item,
                None,
                visible,
            )
            .unwrap(),
        EquipItemObjectOutcome::Equipped
    );

    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND),
        Some(item_guid)
    );
    assert_eq!(
        player.data().visible_items[EQUIPMENT_SLOT_MAINHAND as usize],
        visible
    );
    assert_eq!(item.data().contained_in, player_guid);
    assert_eq!(item.owner_guid(), player_guid);
    assert_eq!(item.slot(), EQUIPMENT_SLOT_MAINHAND);
    assert_eq!(item.container_guid(), ObjectGuid::EMPTY);
    assert!(item.is_soul_bound());
    assert!(item.has_item_flag2(ItemFieldFlags2::EQUIPPED));
    assert_eq!(item.update_state(), ItemUpdateState::Changed);
}

#[test]
fn equip_item_object_merges_existing_stack_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let existing_guid = ObjectGuid::create_item(1, 511);
    let incoming_guid = ObjectGuid::create_item(1, 512);
    let mut player = Player::new(None, false);
    let mut existing = Item::default();
    let mut incoming = Item::default();

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    existing.object_mut().create(existing_guid);
    existing.set_count(2);
    existing.force_state(ItemUpdateState::Unchanged);
    incoming.object_mut().create(incoming_guid);
    incoming.set_count(3);
    incoming.set_item_flag(ItemFieldFlags::REFUNDABLE | ItemFieldFlags::BOP_TRADEABLE);
    incoming.force_state(ItemUpdateState::Unchanged);

    player
        .store_top_level_item(EQUIPMENT_SLOT_FINGER1, existing_guid)
        .unwrap();

    assert_eq!(
        player
            .equip_item_object(
                make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_FINGER1),
                &mut incoming,
                Some(&mut existing),
                VisibleItemValues::default(),
            )
            .unwrap(),
        EquipItemObjectOutcome::Merged
    );

    assert_eq!(existing.count(), 5);
    assert_eq!(existing.update_state(), ItemUpdateState::Changed);
    assert_eq!(incoming.owner_guid(), player_guid);
    assert!(!incoming.has_item_flag(ItemFieldFlags::REFUNDABLE));
    assert!(!incoming.has_item_flag(ItemFieldFlags::BOP_TRADEABLE));
    assert_eq!(incoming.update_state(), ItemUpdateState::Removed);
}

#[test]
fn quick_equip_item_object_visualizes_and_flags_item_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 513);
    let mut player = Player::new(None, false);
    let mut item = Item::default();
    let visible = VisibleItemValues {
        item_id: 513,
        item_appearance_mod_id: 8,
        item_visual: 1,
    };

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    item.object_mut().create(item_guid);
    item.force_state(ItemUpdateState::Unchanged);

    player
        .quick_equip_item_object(
            make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_OFFHAND),
            &mut item,
            visible,
        )
        .unwrap();

    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_OFFHAND),
        Some(item_guid)
    );
    assert_eq!(
        player.data().visible_items[EQUIPMENT_SLOT_OFFHAND as usize],
        visible
    );
    assert_eq!(item.data().contained_in, player_guid);
    assert_eq!(item.owner_guid(), player_guid);
    assert_eq!(item.slot(), EQUIPMENT_SLOT_OFFHAND);
    assert!(item.has_item_flag2(ItemFieldFlags2::EQUIPPED));
    assert_eq!(item.update_state(), ItemUpdateState::Changed);
}

#[test]
fn remove_item_object_unlinks_equipment_without_clearing_owner_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 514);
    let mut player = Player::new(None, false);
    let mut item = Item::default();

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    item.object_mut().create(item_guid);
    item.set_owner_guid(player_guid);
    item.set_contained_in(player_guid);
    item.set_slot(EQUIPMENT_SLOT_MAINHAND);
    item.set_item_flag2(ItemFieldFlags2::EQUIPPED);
    player
        .visualize_item(
            EQUIPMENT_SLOT_MAINHAND,
            item_guid,
            VisibleItemValues {
                item_id: 514,
                item_appearance_mod_id: 3,
                item_visual: 2,
            },
        )
        .unwrap();

    assert_eq!(
        player
            .remove_item_object(
                INVENTORY_SLOT_BAG_0,
                EQUIPMENT_SLOT_MAINHAND,
                Some(&mut item),
                None,
            )
            .unwrap(),
        Some(item_guid)
    );

    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND),
        None
    );
    assert_eq!(
        player.data().visible_items[EQUIPMENT_SLOT_MAINHAND as usize],
        VisibleItemValues::default()
    );
    assert_eq!(item.data().contained_in, ObjectGuid::EMPTY);
    assert_eq!(item.owner_guid(), player_guid);
    assert_eq!(item.slot(), NULL_SLOT);
    assert_eq!(item.container_guid(), ObjectGuid::EMPTY);
    assert!(!item.has_item_flag2(ItemFieldFlags2::EQUIPPED));
}

#[test]
fn remove_item_object_unlinks_bag_item_like_cpp_bag_removeitem() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let bag_guid = ObjectGuid::create_item(1, 800);
    let item_guid = ObjectGuid::create_item(1, 515);
    let mut player = Player::new(None, false);
    let mut bag = Bag::default();
    let mut item = Item::default();

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    bag.item_mut().object_mut().create(bag_guid);
    bag.item_mut().set_owner_guid(player_guid);
    item.object_mut().create(item_guid);
    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 10)
        .unwrap();
    bag.store_item(2, &mut item);
    player
        .store_bag_item(INVENTORY_SLOT_BAG_START, 2, item_guid)
        .unwrap();

    assert_eq!(
        player
            .remove_item_object(INVENTORY_SLOT_BAG_START, 2, Some(&mut item), Some(&mut bag))
            .unwrap(),
        Some(item_guid)
    );

    assert_eq!(player.get_item_by_pos(INVENTORY_SLOT_BAG_START, 2), None);
    assert_eq!(bag.data().slots[2], ObjectGuid::EMPTY);
    assert_eq!(item.data().contained_in, ObjectGuid::EMPTY);
    assert_eq!(item.container_guid(), ObjectGuid::EMPTY);
    assert_eq!(item.slot(), NULL_SLOT);
}

#[test]
fn move_item_from_inventory_object_unlinks_and_clears_refund_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 516);
    let mut player = Player::new(None, false);
    let mut item = Item::default();

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    item.object_mut().create(item_guid);
    item.set_owner_guid(player_guid);
    item.set_contained_in(player_guid);
    item.set_slot(INVENTORY_SLOT_ITEM_START);
    item.set_item_flag(ItemFieldFlags::REFUNDABLE);
    item.set_refund_recipient(player_guid);
    item.set_paid_money(10);
    item.set_paid_extended_cost(20);
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, item_guid)
        .unwrap();

    assert_eq!(
        player
            .move_item_from_inventory_object(
                INVENTORY_SLOT_BAG_0,
                INVENTORY_SLOT_ITEM_START,
                Some(&mut item),
                None,
            )
            .unwrap(),
        Some(item_guid)
    );

    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
        None
    );
    assert_eq!(item.data().contained_in, ObjectGuid::EMPTY);
    assert_eq!(item.owner_guid(), player_guid);
    assert_eq!(item.slot(), NULL_SLOT);
    assert!(!item.has_item_flag(ItemFieldFlags::REFUNDABLE));
    assert_eq!(item.refund_recipient(), ObjectGuid::EMPTY);
    assert_eq!(item.paid_money(), 0);
    assert_eq!(item.paid_extended_cost(), 0);
}

#[test]
fn finalize_move_item_to_inventory_object_marks_original_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 517);
    let other_owner = ObjectGuid::create_player(1, 77);
    let mut player = Player::new(None, false);
    let mut item = Item::default();

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    item.object_mut().create(item_guid);
    item.set_owner_guid(other_owner);
    item.force_state(ItemUpdateState::Unchanged);

    assert!(player.finalize_move_item_to_inventory_object(item_guid, &mut item, false));
    assert_eq!(item.owner_guid(), player_guid);
    assert_eq!(item.update_state(), ItemUpdateState::New);

    item.force_state(ItemUpdateState::Unchanged);
    assert!(player.finalize_move_item_to_inventory_object(item_guid, &mut item, true));
    assert_eq!(item.update_state(), ItemUpdateState::Changed);
}

#[test]
fn finalize_move_item_to_inventory_object_skips_merged_stack_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let original_guid = ObjectGuid::create_item(1, 518);
    let merged_guid = ObjectGuid::create_item(1, 519);
    let mut player = Player::new(None, false);
    let mut merged = Item::default();

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    merged.object_mut().create(merged_guid);
    merged.force_state(ItemUpdateState::Unchanged);

    assert!(!player.finalize_move_item_to_inventory_object(original_guid, &mut merged, false));
    assert_eq!(merged.owner_guid(), ObjectGuid::EMPTY);
    assert_eq!(merged.update_state(), ItemUpdateState::Unchanged);
}

#[test]
fn destroy_item_object_removes_top_level_item_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 520);
    let mut player = Player::new(None, false);
    let mut item = Item::default();

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    item.object_mut().create(item_guid);
    item.set_owner_guid(player_guid);
    item.set_contained_in(player_guid);
    item.set_slot(EQUIPMENT_SLOT_MAINHAND);
    item.set_item_flag(ItemFieldFlags::REFUNDABLE | ItemFieldFlags::BOP_TRADEABLE);
    item.set_item_flag2(ItemFieldFlags2::EQUIPPED);
    item.force_state(ItemUpdateState::Unchanged);
    player
        .visualize_item(
            EQUIPMENT_SLOT_MAINHAND,
            item_guid,
            VisibleItemValues {
                item_id: 520,
                item_appearance_mod_id: 6,
                item_visual: 7,
            },
        )
        .unwrap();

    assert_eq!(
        player
            .destroy_item_object(
                INVENTORY_SLOT_BAG_0,
                EQUIPMENT_SLOT_MAINHAND,
                Some(&mut item),
                None,
            )
            .unwrap(),
        Some(item_guid)
    );

    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND),
        None
    );
    assert_eq!(
        player.data().visible_items[EQUIPMENT_SLOT_MAINHAND as usize],
        VisibleItemValues::default()
    );
    assert_eq!(item.data().contained_in, ObjectGuid::EMPTY);
    assert_eq!(item.owner_guid(), player_guid);
    assert_eq!(item.slot(), NULL_SLOT);
    assert!(!item.has_item_flag(ItemFieldFlags::REFUNDABLE));
    assert!(!item.has_item_flag(ItemFieldFlags::BOP_TRADEABLE));
    assert!(item.has_item_flag2(ItemFieldFlags2::EQUIPPED));
    assert_eq!(item.update_state(), ItemUpdateState::Removed);
}

#[test]
fn destroy_item_object_removes_bag_item_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let bag_guid = ObjectGuid::create_item(1, 801);
    let item_guid = ObjectGuid::create_item(1, 521);
    let mut player = Player::new(None, false);
    let mut bag = Bag::default();
    let mut item = Item::default();

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    bag.item_mut().object_mut().create(bag_guid);
    bag.item_mut().set_owner_guid(player_guid);
    item.object_mut().create(item_guid);
    item.set_item_flag(ItemFieldFlags::REFUNDABLE | ItemFieldFlags::BOP_TRADEABLE);
    item.force_state(ItemUpdateState::Unchanged);
    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 10)
        .unwrap();
    bag.store_item(3, &mut item);
    player
        .store_bag_item(INVENTORY_SLOT_BAG_START, 3, item_guid)
        .unwrap();

    assert_eq!(
        player
            .destroy_item_object(INVENTORY_SLOT_BAG_START, 3, Some(&mut item), Some(&mut bag))
            .unwrap(),
        Some(item_guid)
    );

    assert_eq!(player.get_item_by_pos(INVENTORY_SLOT_BAG_START, 3), None);
    assert_eq!(bag.data().slots[3], ObjectGuid::EMPTY);
    assert_eq!(item.data().contained_in, ObjectGuid::EMPTY);
    assert_eq!(item.container_guid(), ObjectGuid::EMPTY);
    assert_eq!(item.slot(), NULL_SLOT);
    assert!(!item.has_item_flag(ItemFieldFlags::REFUNDABLE));
    assert!(!item.has_item_flag(ItemFieldFlags::BOP_TRADEABLE));
    assert_eq!(item.update_state(), ItemUpdateState::Removed);
}

#[test]
fn destroy_item_count_for_item_object_decrements_partial_stack_like_cpp() {
    let mut player = Player::new(None, false);
    let mut item = Item::default();
    let mut count = 3;

    item.set_count(8);
    item.force_state(ItemUpdateState::Unchanged);

    player
        .destroy_item_count_for_item_object(Some(&mut item), &mut count, None)
        .unwrap();

    assert_eq!(item.count(), 5);
    assert_eq!(count, 0);
    assert_eq!(item.update_state(), ItemUpdateState::Changed);
}

#[test]
fn destroy_item_count_for_item_object_destroys_full_stack_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 522);
    let mut player = Player::new(None, false);
    let mut item = Item::default();
    let mut count = 7;

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    item.object_mut().create(item_guid);
    item.set_owner_guid(player_guid);
    item.set_contained_in(player_guid);
    item.set_slot(INVENTORY_SLOT_ITEM_START);
    item.set_count(5);
    item.force_state(ItemUpdateState::Unchanged);
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, item_guid)
        .unwrap();

    player
        .destroy_item_count_for_item_object(Some(&mut item), &mut count, None)
        .unwrap();

    assert_eq!(count, 2);
    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
        None
    );
    assert_eq!(item.slot(), NULL_SLOT);
    assert_eq!(item.update_state(), ItemUpdateState::Removed);
}

#[test]
fn destroy_item_count_by_entry_plan_matches_cpp_scan_order_and_partial_stop() {
    let player = Player::new(None, false);
    let mut inventory = Item::default();
    let mut bag_item = Item::default();
    let mut bank = Item::default();

    inventory.object_mut().set_entry(900);
    inventory.set_count(2);
    bag_item.object_mut().set_entry(900);
    bag_item.set_count(3);
    bank.object_mut().set_entry(900);
    bank.set_count(5);

    let items = [
        DestroyItemCountItemRef::new(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, &bank),
        DestroyItemCountItemRef::new(INVENTORY_SLOT_BAG_START, 4, &bag_item),
        DestroyItemCountItemRef::new(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, &inventory),
    ];

    let plan = player.destroy_item_count_by_entry_plan(900, 4, false, 16, &items);

    assert_eq!(plan.removed_count, 4);
    assert_eq!(
        plan.actions,
        vec![
            DestroyItemCountAction {
                bag: INVENTORY_SLOT_BAG_0,
                slot: INVENTORY_SLOT_ITEM_START,
                removed_count: 2,
                remaining_count: 0,
                destroy_stack: true,
            },
            DestroyItemCountAction {
                bag: INVENTORY_SLOT_BAG_START,
                slot: 4,
                removed_count: 2,
                remaining_count: 1,
                destroy_stack: false,
            },
        ]
    );
}

#[test]
fn destroy_item_count_by_entry_plan_matches_cpp_unequip_check_for_full_equipment_stack() {
    let player = Player::new(None, false);
    let mut equipped = Item::default();
    let mut bank = Item::default();

    equipped.object_mut().set_entry(901);
    equipped.set_count(1);
    bank.object_mut().set_entry(901);
    bank.set_count(1);

    let mut blocked_equipped =
        DestroyItemCountItemRef::new(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND, &equipped);
    blocked_equipped.can_unequip_result = InventoryResult::CantEquipEver;
    let items = [
        blocked_equipped,
        DestroyItemCountItemRef::new(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, &bank),
    ];

    let plan = player.destroy_item_count_by_entry_plan(901, 1, true, 16, &items);

    assert_eq!(plan.removed_count, 1);
    assert_eq!(
        plan.actions,
        vec![DestroyItemCountAction {
            bag: INVENTORY_SLOT_BAG_0,
            slot: BANK_SLOT_ITEM_START,
            removed_count: 1,
            remaining_count: 0,
            destroy_stack: true,
        }]
    );
}

#[test]
fn destroy_zone_limited_item_plan_matches_cpp_scan_order() {
    let player = Player::new(None, false);
    let items = [
        DestroyFilteredItemRef::new(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST, true),
        DestroyFilteredItemRef::new(INVENTORY_SLOT_BAG_0, KEYRING_SLOT_START, true),
        DestroyFilteredItemRef::new(INVENTORY_SLOT_BAG_START, 2, true),
        DestroyFilteredItemRef::new(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, true),
        DestroyFilteredItemRef::new(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1, false),
    ];

    assert_eq!(
        player.destroy_zone_limited_item_plan(16, &items),
        vec![
            DestroyFilteredItemAction {
                bag: INVENTORY_SLOT_BAG_0,
                slot: INVENTORY_SLOT_ITEM_START,
            },
            DestroyFilteredItemAction {
                bag: INVENTORY_SLOT_BAG_0,
                slot: KEYRING_SLOT_START,
            },
            DestroyFilteredItemAction {
                bag: INVENTORY_SLOT_BAG_START,
                slot: 2,
            },
            DestroyFilteredItemAction {
                bag: INVENTORY_SLOT_BAG_0,
                slot: EQUIPMENT_SLOT_CHEST,
            },
        ]
    );
}

#[test]
fn destroy_conjured_items_plan_matches_cpp_scan_order_without_keyring() {
    let player = Player::new(None, false);
    let items = [
        DestroyFilteredItemRef::new(INVENTORY_SLOT_BAG_0, KEYRING_SLOT_START, true),
        DestroyFilteredItemRef::new(INVENTORY_SLOT_BAG_START, 1, true),
        DestroyFilteredItemRef::new(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST, true),
        DestroyFilteredItemRef::new(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, true),
    ];

    assert_eq!(
        player.destroy_conjured_items_plan(16, &items),
        vec![
            DestroyFilteredItemAction {
                bag: INVENTORY_SLOT_BAG_0,
                slot: INVENTORY_SLOT_ITEM_START,
            },
            DestroyFilteredItemAction {
                bag: INVENTORY_SLOT_BAG_START,
                slot: 1,
            },
            DestroyFilteredItemAction {
                bag: INVENTORY_SLOT_BAG_0,
                slot: EQUIPMENT_SLOT_CHEST,
            },
        ]
    );
}

#[test]
fn store_item_object_mutates_empty_top_level_slot_like_cpp_storeitem() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 600);
    let mut player = Player::new(None, false);
    let mut item = Item::default();

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player.clear_active_player_data_changes();
    item.object_mut().create(item_guid);
    item.set_bonding(ItemBondingType::OnAcquire);
    item.force_state(ItemUpdateState::Unchanged);
    item.clear_item_data_changes();

    player
        .store_item_object(INVENTORY_SLOT_ITEM_START, &mut item, 4)
        .unwrap();

    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
        Some(item_guid)
    );
    assert_eq!(
        player.active_data().inv_slots[INVENTORY_SLOT_ITEM_START as usize],
        item_guid
    );
    assert_eq!(item.count(), 4);
    assert_eq!(item.data().contained_in, player_guid);
    assert_eq!(item.owner_guid(), player_guid);
    assert_eq!(item.slot(), INVENTORY_SLOT_ITEM_START);
    assert_eq!(item.container_guid(), ObjectGuid::EMPTY);
    assert_eq!(item.bag_slot(), INVENTORY_SLOT_BAG_0);
    assert!(item.is_soul_bound());
    assert_eq!(item.update_state(), ItemUpdateState::Changed);
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_INV_SLOTS_FIRST_BIT + INVENTORY_SLOT_ITEM_START as usize)
    );
}

#[test]
fn store_item_object_binds_on_equip_only_for_bag_positions_like_cpp_storeitem() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let mut player = Player::new(None, false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);

    let mut inventory_item = Item::default();
    inventory_item
        .object_mut()
        .create(ObjectGuid::create_item(1, 601));
    inventory_item.set_bonding(ItemBondingType::OnEquip);
    player
        .store_item_object(INVENTORY_SLOT_ITEM_START, &mut inventory_item, 1)
        .unwrap();
    assert!(!inventory_item.is_soul_bound());

    let mut bag_item = Item::default();
    bag_item
        .object_mut()
        .create(ObjectGuid::create_item(1, 602));
    bag_item.set_bonding(ItemBondingType::OnEquip);
    player
        .store_item_object(INVENTORY_SLOT_BAG_START, &mut bag_item, 1)
        .unwrap();
    assert!(bag_item.is_soul_bound());
}

#[test]
fn store_item_object_rejects_occupied_slot_until_stack_merge_registry_exists() {
    let existing = ObjectGuid::create_item(1, 700);
    let incoming = ObjectGuid::create_item(1, 701);
    let mut player = Player::new(None, false);
    let mut item = Item::default();
    item.object_mut().create(incoming);
    item.force_state(ItemUpdateState::Unchanged);

    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, existing)
        .unwrap();
    let result = player.store_item_object(INVENTORY_SLOT_ITEM_START, &mut item, 3);

    assert_eq!(
        result,
        Err(PlayerStorageError::OccupiedPlayerSlot(
            INVENTORY_SLOT_ITEM_START
        ))
    );
    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
        Some(existing)
    );
    assert_eq!(item.count(), 0);
    assert_eq!(item.update_state(), ItemUpdateState::Unchanged);
}

#[test]
fn store_cloned_item_object_keeps_source_and_stores_clone_like_cpp_storeitem_clone() {
    let owner = ObjectGuid::create_player(1, 42);
    let source_guid = ObjectGuid::create_item(1, 760);
    let clone_guid = ObjectGuid::create_item(1, 761);
    let mut player = Player::new(None, false);
    let mut source = Item::default();

    player.unit_mut().world_mut().object_mut().create(owner);
    source.object_mut().create(source_guid);
    source.object_mut().set_entry(6948);
    source.set_count(8);
    source.set_bonding(ItemBondingType::OnAcquire);
    source.set_item_flag(ItemFieldFlags::REFUNDABLE | ItemFieldFlags::BOP_TRADEABLE);
    source.force_state(ItemUpdateState::Unchanged);

    let cloned = player
        .store_cloned_item_object(INVENTORY_SLOT_ITEM_START, &source, clone_guid, 3)
        .unwrap();

    assert_eq!(source.object().guid(), source_guid);
    assert_eq!(source.count(), 8);
    assert!(source.is_refundable());
    assert!(source.is_bop_tradeable());
    assert_eq!(source.update_state(), ItemUpdateState::Unchanged);
    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
        Some(clone_guid)
    );
    assert_eq!(cloned.object().guid(), clone_guid);
    assert_eq!(cloned.object().entry(), 6948);
    assert_eq!(cloned.count(), 3);
    assert_eq!(cloned.owner_guid(), owner);
    assert!(cloned.is_soul_bound());
    assert!(!cloned.is_refundable());
    assert!(!cloned.is_bop_tradeable());
    assert_eq!(cloned.slot(), INVENTORY_SLOT_ITEM_START);
    assert_eq!(cloned.update_state(), ItemUpdateState::New);
}

#[test]
fn split_item_to_empty_top_level_object_matches_cpp_split_allocation() {
    let owner = ObjectGuid::create_player(1, 42);
    let source_guid = ObjectGuid::create_item(1, 762);
    let clone_guid = ObjectGuid::create_item(1, 763);
    let mut player = Player::new(None, false);
    let mut source = Item::default();

    player.unit_mut().world_mut().object_mut().create(owner);
    source.object_mut().create(source_guid);
    source.object_mut().set_entry(6948);
    source.set_count(8);
    source.set_item_flag(ItemFieldFlags::REFUNDABLE | ItemFieldFlags::BOP_TRADEABLE);
    source.force_state(ItemUpdateState::Unchanged);
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, source_guid)
        .unwrap();

    let cloned = player
        .split_item_to_empty_top_level_object(
            INVENTORY_SLOT_ITEM_START + 1,
            &mut source,
            clone_guid,
            3,
        )
        .unwrap();

    assert_eq!(source.count(), 5);
    assert_eq!(source.update_state(), ItemUpdateState::Changed);
    assert!(source.is_refundable());
    assert!(source.is_bop_tradeable());
    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
        Some(source_guid)
    );
    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1),
        Some(clone_guid)
    );
    assert_eq!(cloned.object().guid(), clone_guid);
    assert_eq!(cloned.count(), 3);
    assert!(!cloned.is_refundable());
    assert!(!cloned.is_bop_tradeable());
    assert_eq!(cloned.update_state(), ItemUpdateState::New);
}

#[test]
fn split_item_to_empty_top_level_object_rolls_back_source_like_cpp_on_failure() {
    let owner = ObjectGuid::create_player(1, 42);
    let source_guid = ObjectGuid::create_item(1, 764);
    let occupied_guid = ObjectGuid::create_item(1, 765);
    let clone_guid = ObjectGuid::create_item(1, 766);
    let mut player = Player::new(None, false);
    let mut source = Item::default();

    player.unit_mut().world_mut().object_mut().create(owner);
    source.object_mut().create(source_guid);
    source.object_mut().set_entry(6948);
    source.set_count(8);
    source.force_state(ItemUpdateState::Unchanged);
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, source_guid)
        .unwrap();
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START + 1, occupied_guid)
        .unwrap();

    assert_eq!(
        player.split_item_to_empty_top_level_object(
            INVENTORY_SLOT_ITEM_START + 1,
            &mut source,
            clone_guid,
            3,
        ),
        Err(PlayerStorageError::OccupiedPlayerSlot(
            INVENTORY_SLOT_ITEM_START + 1
        ))
    );

    assert_eq!(source.count(), 8);
    assert_eq!(source.update_state(), ItemUpdateState::Unchanged);
    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1),
        Some(occupied_guid)
    );
}

#[test]
fn store_bag_item_object_mutates_bag_branch_like_cpp_storeitem() {
    let owner = ObjectGuid::create_player(1, 42);
    let bag_guid = ObjectGuid::create_item(1, 800);
    let item_guid = ObjectGuid::create_item(1, 801);
    let mut player = Player::new(None, false);
    let mut bag = Bag::default();
    let mut item = Item::default();

    player.unit_mut().world_mut().object_mut().create(owner);
    bag.try_initialize_created_state(crate::BagCreateInfo {
        guid: bag_guid,
        item_id: 100,
        context: ItemContext::None,
        owner: Some(owner),
        max_durability: 0,
        container_slots: 4,
    })
    .unwrap();
    bag.item_mut().set_slot(INVENTORY_SLOT_BAG_START);
    bag.item_mut().force_state(ItemUpdateState::Unchanged);
    bag.clear_container_data_changes();
    item.object_mut().create(item_guid);
    item.set_bonding(ItemBondingType::Quest);
    item.force_state(ItemUpdateState::Unchanged);

    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 4)
        .unwrap();
    player
        .store_bag_item_object(INVENTORY_SLOT_BAG_START, &mut bag, 2, &mut item, 3)
        .unwrap();

    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_START, 2),
        Some(item_guid)
    );
    assert_eq!(bag.item_by_pos(2), Some(item_guid));
    assert_eq!(item.count(), 3);
    assert_eq!(item.data().contained_in, bag_guid);
    assert_eq!(item.owner_guid(), owner);
    assert_eq!(item.container_guid(), bag_guid);
    assert_eq!(item.bag_slot(), INVENTORY_SLOT_BAG_START);
    assert_eq!(item.slot(), 2);
    assert!(item.is_soul_bound());
    assert_eq!(item.update_state(), ItemUpdateState::Changed);
    assert_eq!(bag.item().update_state(), ItemUpdateState::Changed);
    assert!(
        bag.container_data_changes_mask()
            .is_set(crate::CONTAINER_DATA_SLOTS_FIRST_BIT + 2)
    );
}

#[test]
fn store_bag_item_object_rejects_mismatched_or_occupied_bag_slot() {
    let owner = ObjectGuid::create_player(1, 42);
    let registered_bag = ObjectGuid::create_item(1, 810);
    let actual_bag = ObjectGuid::create_item(1, 811);
    let existing = ObjectGuid::create_item(1, 812);
    let mut player = Player::new(None, false);
    let mut bag = Bag::default();
    let mut item = Item::default();

    bag.try_initialize_created_state(crate::BagCreateInfo {
        guid: actual_bag,
        item_id: 100,
        context: ItemContext::None,
        owner: Some(owner),
        max_durability: 0,
        container_slots: 4,
    })
    .unwrap();
    item.object_mut().create(ObjectGuid::create_item(1, 813));
    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, registered_bag, 4)
        .unwrap();

    assert_eq!(
        player.store_bag_item_object(INVENTORY_SLOT_BAG_START, &mut bag, 2, &mut item, 1),
        Err(PlayerStorageError::MismatchedBagGuid {
            bag: INVENTORY_SLOT_BAG_START,
            expected: registered_bag,
            actual: actual_bag,
        })
    );

    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START + 1, actual_bag, 4)
        .unwrap();
    player
        .store_bag_item(INVENTORY_SLOT_BAG_START + 1, 2, existing)
        .unwrap();
    assert_eq!(
        player.store_bag_item_object(INVENTORY_SLOT_BAG_START + 1, &mut bag, 2, &mut item, 1),
        Err(PlayerStorageError::OccupiedBagItemSlot {
            bag: INVENTORY_SLOT_BAG_START + 1,
            slot: 2,
        })
    );
    assert_eq!(item.count(), 0);
    assert_eq!(bag.item_by_pos(2), None);
}

#[test]
fn store_cloned_bag_item_object_keeps_source_and_stores_clone_like_cpp_storeitem_clone() {
    let owner = ObjectGuid::create_player(1, 42);
    let bag_guid = ObjectGuid::create_item(1, 860);
    let source_guid = ObjectGuid::create_item(1, 861);
    let clone_guid = ObjectGuid::create_item(1, 862);
    let mut player = Player::new(None, false);
    let mut bag = Bag::default();
    let mut source = Item::default();

    player.unit_mut().world_mut().object_mut().create(owner);
    bag.try_initialize_created_state(crate::BagCreateInfo {
        guid: bag_guid,
        item_id: 100,
        context: ItemContext::None,
        owner: Some(owner),
        max_durability: 0,
        container_slots: 4,
    })
    .unwrap();
    bag.item_mut().set_slot(INVENTORY_SLOT_BAG_START);
    source.object_mut().create(source_guid);
    source.object_mut().set_entry(6948);
    source.set_count(8);
    source.set_bonding(ItemBondingType::OnEquip);
    source.set_item_flag(ItemFieldFlags::REFUNDABLE | ItemFieldFlags::BOP_TRADEABLE);
    source.force_state(ItemUpdateState::Unchanged);

    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 4)
        .unwrap();
    let cloned = player
        .store_cloned_bag_item_object(
            INVENTORY_SLOT_BAG_START,
            &mut bag,
            2,
            &source,
            clone_guid,
            3,
        )
        .unwrap();

    assert_eq!(source.object().guid(), source_guid);
    assert_eq!(source.count(), 8);
    assert!(source.is_refundable());
    assert!(source.is_bop_tradeable());
    assert_eq!(source.update_state(), ItemUpdateState::Unchanged);
    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_START, 2),
        Some(clone_guid)
    );
    assert_eq!(bag.item_by_pos(2), Some(clone_guid));
    assert_eq!(cloned.object().guid(), clone_guid);
    assert_eq!(cloned.object().entry(), 6948);
    assert_eq!(cloned.count(), 3);
    assert_eq!(cloned.owner_guid(), owner);
    assert!(!cloned.is_soul_bound());
    assert!(!cloned.is_refundable());
    assert!(!cloned.is_bop_tradeable());
    assert_eq!(cloned.container_guid(), bag_guid);
    assert_eq!(cloned.bag_slot(), INVENTORY_SLOT_BAG_START);
    assert_eq!(cloned.slot(), 2);
    assert_eq!(cloned.update_state(), ItemUpdateState::New);
}

#[test]
fn split_item_to_empty_bag_item_object_matches_cpp_split_allocation() {
    let owner = ObjectGuid::create_player(1, 42);
    let bag_guid = ObjectGuid::create_item(1, 870);
    let source_guid = ObjectGuid::create_item(1, 871);
    let clone_guid = ObjectGuid::create_item(1, 872);
    let mut player = Player::new(None, false);
    let mut bag = Bag::default();
    let mut source = Item::default();

    player.unit_mut().world_mut().object_mut().create(owner);
    bag.try_initialize_created_state(crate::BagCreateInfo {
        guid: bag_guid,
        item_id: 100,
        context: ItemContext::None,
        owner: Some(owner),
        max_durability: 0,
        container_slots: 4,
    })
    .unwrap();
    bag.item_mut().set_slot(INVENTORY_SLOT_BAG_START);
    source.object_mut().create(source_guid);
    source.object_mut().set_entry(6948);
    source.set_count(8);
    source.set_item_flag(ItemFieldFlags::REFUNDABLE | ItemFieldFlags::BOP_TRADEABLE);
    bag.store_item(1, &mut source);
    source.force_state(ItemUpdateState::Unchanged);

    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 4)
        .unwrap();
    player
        .store_bag_item(INVENTORY_SLOT_BAG_START, 1, source_guid)
        .unwrap();
    let cloned = player
        .split_item_to_empty_bag_item_object(
            INVENTORY_SLOT_BAG_START,
            &mut bag,
            2,
            &mut source,
            clone_guid,
            3,
        )
        .unwrap();

    assert_eq!(source.count(), 5);
    assert_eq!(source.update_state(), ItemUpdateState::Changed);
    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_START, 1),
        Some(source_guid)
    );
    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_START, 2),
        Some(clone_guid)
    );
    assert_eq!(bag.item_by_pos(1), Some(source_guid));
    assert_eq!(bag.item_by_pos(2), Some(clone_guid));
    assert_eq!(cloned.object().guid(), clone_guid);
    assert_eq!(cloned.count(), 3);
    assert!(!cloned.is_refundable());
    assert!(!cloned.is_bop_tradeable());
    assert_eq!(cloned.update_state(), ItemUpdateState::New);
}

#[test]
fn split_item_rejects_zero_all_or_too_many_like_cpp_guards() {
    let mut player = Player::new(None, false);
    let mut source = Item::default();
    source.object_mut().create(ObjectGuid::create_item(1, 880));
    source.set_count(8);

    assert_eq!(
        player.split_item_to_empty_top_level_object(
            INVENTORY_SLOT_ITEM_START,
            &mut source,
            ObjectGuid::create_item(1, 881),
            0,
        ),
        Err(PlayerStorageError::InvalidSplitCount {
            available: 8,
            requested: 0,
        })
    );
    assert_eq!(
        player.split_item_to_empty_top_level_object(
            INVENTORY_SLOT_ITEM_START,
            &mut source,
            ObjectGuid::create_item(1, 882),
            8,
        ),
        Err(PlayerStorageError::InvalidSplitCount {
            available: 8,
            requested: 8,
        })
    );
    assert_eq!(
        player.split_item_to_empty_top_level_object(
            INVENTORY_SLOT_ITEM_START,
            &mut source,
            ObjectGuid::create_item(1, 883),
            9,
        ),
        Err(PlayerStorageError::TooFewItemsToSplit {
            available: 8,
            requested: 9,
        })
    );
    assert_eq!(source.count(), 8);
    assert_eq!(source.update_state(), ItemUpdateState::New);
}

#[test]
fn split_item_rejects_loot_and_trade_states_in_cpp_order() {
    let mut player = Player::new(None, false);
    let mut source = Item::default();
    source.object_mut().create(ObjectGuid::create_item(1, 884));
    source.set_count(8);
    source.set_loot_generated(true);
    source.set_in_trade(true);

    assert_eq!(
        player.split_item_to_empty_top_level_object(
            INVENTORY_SLOT_ITEM_START,
            &mut source,
            ObjectGuid::create_item(1, 885),
            8,
        ),
        Err(PlayerStorageError::SplitItemLootGenerated)
    );

    source.set_loot_generated(false);
    assert_eq!(
        player.split_item_to_empty_top_level_object(
            INVENTORY_SLOT_ITEM_START,
            &mut source,
            ObjectGuid::create_item(1, 886),
            8,
        ),
        Err(PlayerStorageError::InvalidSplitCount {
            available: 8,
            requested: 8,
        })
    );
    assert_eq!(
        player.split_item_to_empty_top_level_object(
            INVENTORY_SLOT_ITEM_START,
            &mut source,
            ObjectGuid::create_item(1, 887),
            3,
        ),
        Err(PlayerStorageError::SplitItemInTrade)
    );
    assert_eq!(source.count(), 8);
    assert_eq!(source.update_state(), ItemUpdateState::New);
}

#[test]
fn merge_top_level_item_stack_object_matches_cpp_existing_stack_branch() {
    let owner = ObjectGuid::create_player(1, 42);
    let existing_guid = ObjectGuid::create_item(1, 820);
    let incoming_guid = ObjectGuid::create_item(1, 821);
    let mut player = Player::new(None, false);
    let mut existing = Item::default();
    let mut incoming = Item::default();

    player.unit_mut().world_mut().object_mut().create(owner);
    existing.object_mut().create(existing_guid);
    existing.set_bonding(ItemBondingType::OnEquip);
    existing.set_count(5);
    existing.force_state(ItemUpdateState::Unchanged);
    incoming.object_mut().create(incoming_guid);
    incoming.set_item_flag(ItemFieldFlags::REFUNDABLE | ItemFieldFlags::BOP_TRADEABLE);
    incoming.set_refund_recipient(ObjectGuid::create_player(1, 99));
    incoming.set_paid_money(10);
    incoming.set_paid_extended_cost(20);
    incoming.force_state(ItemUpdateState::Unchanged);

    player
        .store_top_level_item(INVENTORY_SLOT_BAG_START, existing_guid)
        .unwrap();
    player
        .merge_top_level_item_stack_object(
            INVENTORY_SLOT_BAG_START,
            &mut existing,
            &mut incoming,
            3,
        )
        .unwrap();

    assert_eq!(existing.count(), 8);
    assert!(existing.is_soul_bound());
    assert_eq!(existing.update_state(), ItemUpdateState::Changed);
    assert_eq!(incoming.owner_guid(), owner);
    assert!(!incoming.is_refundable());
    assert!(!incoming.is_bop_tradeable());
    assert_eq!(incoming.refund_recipient(), ObjectGuid::EMPTY);
    assert_eq!(incoming.paid_money(), 0);
    assert_eq!(incoming.paid_extended_cost(), 0);
    assert_eq!(incoming.update_state(), ItemUpdateState::Removed);
}

#[test]
fn merge_top_level_item_stack_object_rejects_empty_or_mismatched_slot() {
    let expected = ObjectGuid::create_item(1, 830);
    let actual = ObjectGuid::create_item(1, 831);
    let mut player = Player::new(None, false);
    let mut existing = Item::default();
    let mut incoming = Item::default();
    existing.object_mut().create(actual);

    assert_eq!(
        player.merge_top_level_item_stack_object(
            INVENTORY_SLOT_ITEM_START,
            &mut existing,
            &mut incoming,
            1,
        ),
        Err(PlayerStorageError::EmptyPlayerSlot(
            INVENTORY_SLOT_ITEM_START
        ))
    );

    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, expected)
        .unwrap();
    assert_eq!(
        player.merge_top_level_item_stack_object(
            INVENTORY_SLOT_ITEM_START,
            &mut existing,
            &mut incoming,
            1,
        ),
        Err(PlayerStorageError::MismatchedItemGuid {
            slot: INVENTORY_SLOT_ITEM_START,
            expected,
            actual,
        })
    );
}

#[test]
fn merge_bag_item_stack_object_matches_cpp_existing_stack_branch() {
    let owner = ObjectGuid::create_player(1, 42);
    let bag_guid = ObjectGuid::create_item(1, 840);
    let existing_guid = ObjectGuid::create_item(1, 841);
    let incoming_guid = ObjectGuid::create_item(1, 842);
    let mut player = Player::new(None, false);
    let mut bag = Bag::default();
    let mut existing = Item::default();
    let mut incoming = Item::default();

    player.unit_mut().world_mut().object_mut().create(owner);
    bag.try_initialize_created_state(crate::BagCreateInfo {
        guid: bag_guid,
        item_id: 100,
        context: ItemContext::None,
        owner: Some(owner),
        max_durability: 0,
        container_slots: 4,
    })
    .unwrap();
    bag.item_mut().set_slot(INVENTORY_SLOT_BAG_START);
    bag.item_mut().force_state(ItemUpdateState::Unchanged);
    existing.object_mut().create(existing_guid);
    existing.set_bonding(ItemBondingType::OnEquip);
    existing.set_count(5);
    existing.force_state(ItemUpdateState::Unchanged);
    incoming.object_mut().create(incoming_guid);
    incoming.set_item_flag(ItemFieldFlags::REFUNDABLE | ItemFieldFlags::BOP_TRADEABLE);
    incoming.set_refund_recipient(ObjectGuid::create_player(1, 99));
    incoming.set_paid_money(10);
    incoming.set_paid_extended_cost(20);
    incoming.force_state(ItemUpdateState::Unchanged);
    bag.store_item(2, &mut existing);

    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 4)
        .unwrap();
    player
        .store_bag_item(INVENTORY_SLOT_BAG_START, 2, existing_guid)
        .unwrap();
    player
        .merge_bag_item_stack_object(
            INVENTORY_SLOT_BAG_START,
            &bag,
            2,
            &mut existing,
            &mut incoming,
            3,
        )
        .unwrap();

    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_START, 2),
        Some(existing_guid)
    );
    assert_eq!(bag.item_by_pos(2), Some(existing_guid));
    assert_eq!(existing.count(), 8);
    assert!(!existing.is_soul_bound());
    assert_eq!(existing.update_state(), ItemUpdateState::Changed);
    assert_eq!(bag.item().update_state(), ItemUpdateState::Unchanged);
    assert_eq!(incoming.owner_guid(), owner);
    assert!(!incoming.is_refundable());
    assert!(!incoming.is_bop_tradeable());
    assert_eq!(incoming.refund_recipient(), ObjectGuid::EMPTY);
    assert_eq!(incoming.paid_money(), 0);
    assert_eq!(incoming.paid_extended_cost(), 0);
    assert_eq!(incoming.update_state(), ItemUpdateState::Removed);
}

#[test]
fn merge_bag_item_stack_object_rejects_empty_or_mismatched_slot() {
    let owner = ObjectGuid::create_player(1, 42);
    let bag_guid = ObjectGuid::create_item(1, 850);
    let expected = ObjectGuid::create_item(1, 851);
    let actual = ObjectGuid::create_item(1, 852);
    let mut player = Player::new(None, false);
    let mut bag = Bag::default();
    let mut existing = Item::default();
    let mut incoming = Item::default();

    bag.try_initialize_created_state(crate::BagCreateInfo {
        guid: bag_guid,
        item_id: 100,
        context: ItemContext::None,
        owner: Some(owner),
        max_durability: 0,
        container_slots: 4,
    })
    .unwrap();
    existing.object_mut().create(actual);
    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 4)
        .unwrap();

    assert_eq!(
        player.merge_bag_item_stack_object(
            INVENTORY_SLOT_BAG_START,
            &bag,
            2,
            &mut existing,
            &mut incoming,
            1,
        ),
        Err(PlayerStorageError::EmptyBagItemSlot {
            bag: INVENTORY_SLOT_BAG_START,
            slot: 2,
        })
    );

    player
        .store_bag_item(INVENTORY_SLOT_BAG_START, 2, expected)
        .unwrap();
    assert_eq!(
        player.merge_bag_item_stack_object(
            INVENTORY_SLOT_BAG_START,
            &bag,
            2,
            &mut existing,
            &mut incoming,
            1,
        ),
        Err(PlayerStorageError::MismatchedBagItemGuid {
            bag: INVENTORY_SLOT_BAG_START,
            slot: 2,
            expected,
            actual: ObjectGuid::EMPTY,
        })
    );

    bag.store_item(2, &mut existing);
    assert_eq!(
        player.merge_bag_item_stack_object(
            INVENTORY_SLOT_BAG_START,
            &bag,
            2,
            &mut existing,
            &mut incoming,
            1,
        ),
        Err(PlayerStorageError::MismatchedBagItemGuid {
            bag: INVENTORY_SLOT_BAG_START,
            slot: 2,
            expected,
            actual,
        })
    );
}

#[test]
fn player_get_item_by_guid_scans_everywhere_except_buyback_like_cpp_for_each_item() {
    let mut player = Player::new(None, false);
    player.set_inventory_slot_count(INVENTORY_DEFAULT_SIZE);

    let inventory_item = ObjectGuid::create_item(1, 10);
    let bank_item = ObjectGuid::create_item(1, 11);
    let reagent_bag = ObjectGuid::create_item(1, 12);
    let reagent_item = ObjectGuid::create_item(1, 13);
    let buyback = ObjectGuid::create_item(1, 14);

    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, inventory_item)
        .unwrap();
    player
        .store_top_level_item(BANK_SLOT_ITEM_START, bank_item)
        .unwrap();
    player
        .store_top_level_item(REAGENT_BAG_SLOT_START, reagent_bag)
        .unwrap();
    player
        .register_bag_storage(REAGENT_BAG_SLOT_START, reagent_bag, 3)
        .unwrap();
    player
        .store_bag_item(REAGENT_BAG_SLOT_START, 1, reagent_item)
        .unwrap();
    player
        .store_top_level_item(BUYBACK_SLOT_START, buyback)
        .unwrap();

    assert_eq!(
        player.get_item_by_guid(inventory_item),
        Some(inventory_item)
    );
    assert_eq!(player.get_item_by_guid(bank_item), Some(bank_item));
    assert_eq!(player.get_item_by_guid(reagent_item), Some(reagent_item));
    assert_eq!(player.get_item_by_guid(buyback), None);

    let mut visited = Vec::new();
    let completed = player.for_each_item_guid(ItemSearchLocation::INVENTORY, |guid| {
        visited.push(guid);
        ItemSearchCallbackResult::Continue
    });
    assert!(completed);
    assert!(visited.contains(&inventory_item));
    assert!(!visited.contains(&bank_item));
}

#[test]
fn player_buyback_slots_follow_cpp_current_slot_and_masks() {
    let mut player = Player::new(None, false);
    player.clear_active_player_data_changes();

    let first = ObjectGuid::create_item(1, 1000);
    let second = ObjectGuid::create_item(1, 1001);

    let first_slot = player.add_item_to_buyback_slot(first, 123, 456);
    assert_eq!(first_slot, BUYBACK_SLOT_START);
    assert_eq!(
        player.inventory().current_buyback_slot,
        BUYBACK_SLOT_START + 1
    );
    assert_eq!(player.get_item_from_buyback_slot(first_slot), Some(first));
    assert_eq!(player.active_data().buyback_price[0], 123);
    assert_eq!(player.active_data().buyback_timestamp[0], 456);
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_BUYBACK_PRICE_FIRST_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_BUYBACK_TIMESTAMP_FIRST_BIT)
    );

    let second_slot = player.add_item_to_buyback_slot(second, 200, 500);
    assert_eq!(second_slot, BUYBACK_SLOT_START + 1);
    assert_eq!(
        player.remove_item_from_buyback_slot(first_slot),
        Some(first)
    );
    assert_eq!(player.get_item_from_buyback_slot(first_slot), None);
    assert_eq!(
        player.active_data().inv_slots[first_slot as usize],
        ObjectGuid::EMPTY
    );
    assert_eq!(player.active_data().buyback_price[0], 0);
    assert_eq!(player.active_data().buyback_timestamp[0], 0);
}

#[test]
fn add_item_to_buyback_slot_object_matches_cpp_price_time_and_replacement() {
    let mut player = Player::new(None, false);
    let mut overwritten = item_with_guid_entry(1100, 7000);
    overwritten.set_count(3);
    overwritten.force_state(ItemUpdateState::Unchanged);
    let old_proto = ItemStorageTemplate {
        sell_price: 11,
        ..ItemStorageTemplate::regular_item(7000, 20)
    };

    let old_slot = player
        .add_item_to_buyback_slot_object(&overwritten, Some(&old_proto), 2000, 1000, None)
        .unwrap();
    assert_eq!(old_slot, BUYBACK_SLOT_START);
    assert_eq!(
        player.get_item_from_buyback_slot(old_slot),
        Some(overwritten.object().guid())
    );
    assert_eq!(player.active_data().buyback_price[0], 33);
    assert_eq!(player.active_data().buyback_timestamp[0], 109000);

    player.set_buyback_timestamp(0, 50);
    for slot in BUYBACK_SLOT_START + 1..BUYBACK_SLOT_END {
        let guid = ObjectGuid::create_item(1, 2000 + slot as i64);
        player.add_item_to_buyback_slot(guid, 1, 100 + slot as i64);
    }

    overwritten.object_mut().add_to_world();
    let mut replacement = item_with_guid_entry(1101, 7001);
    replacement.set_count(4);
    let replacement_proto = ItemStorageTemplate {
        sell_price: 9,
        ..ItemStorageTemplate::regular_item(7001, 20)
    };

    let replaced_slot = player
        .add_item_to_buyback_slot_object(
            &replacement,
            Some(&replacement_proto),
            5000,
            3000,
            Some(&mut overwritten),
        )
        .unwrap();

    assert_eq!(replaced_slot, old_slot);
    assert!(!overwritten.object().is_in_world());
    assert_eq!(overwritten.update_state(), ItemUpdateState::Removed);
    assert_eq!(
        player.get_item_from_buyback_slot(replaced_slot),
        Some(replacement.object().guid())
    );
    assert_eq!(player.active_data().buyback_price[0], 36);
    assert_eq!(player.active_data().buyback_timestamp[0], 110000);
    assert_eq!(
        player.inventory().current_buyback_slot,
        BUYBACK_SLOT_START + 1
    );
}

#[test]
fn remove_item_from_buyback_slot_object_matches_cpp_item_side_effects() {
    let mut player = Player::new(None, false);
    let mut item = item_with_guid_entry(1010, 6948);
    item.force_state(ItemUpdateState::Unchanged);
    item.object_mut().add_to_world();

    let slot = player.add_item_to_buyback_slot(item.object().guid(), 123, 456);
    assert_eq!(
        player
            .remove_item_from_buyback_slot_object(slot, Some(&mut item), true)
            .unwrap(),
        Some(item.object().guid())
    );

    assert!(!item.object().is_in_world());
    assert_eq!(item.update_state(), ItemUpdateState::Removed);
    assert_eq!(player.get_item_from_buyback_slot(slot), None);
    assert_eq!(
        player.active_data().inv_slots[slot as usize],
        ObjectGuid::EMPTY
    );
    assert_eq!(player.active_data().buyback_price[0], 0);
    assert_eq!(player.active_data().buyback_timestamp[0], 0);

    let mut keep_state_item = item_with_guid_entry(1011, 6949);
    keep_state_item.force_state(ItemUpdateState::Unchanged);
    keep_state_item.object_mut().add_to_world();
    let keep_slot = player.add_item_to_buyback_slot(keep_state_item.object().guid(), 200, 500);

    player
        .remove_item_from_buyback_slot_object(keep_slot, Some(&mut keep_state_item), false)
        .unwrap();
    assert!(!keep_state_item.object().is_in_world());
    assert_eq!(keep_state_item.update_state(), ItemUpdateState::Unchanged);
}

#[test]
fn remove_item_from_buyback_slot_object_rejects_mismatched_item_ref() {
    let mut player = Player::new(None, false);
    let expected = ObjectGuid::create_item(1, 1020);
    let mut actual = item_with_guid_entry(1021, 6948);

    let slot = player.add_item_to_buyback_slot(expected, 123, 456);
    assert_eq!(
        player.remove_item_from_buyback_slot_object(slot, Some(&mut actual), true),
        Err(PlayerStorageError::MismatchedItemGuid {
            slot,
            expected,
            actual: actual.object().guid(),
        })
    );
    assert_eq!(player.get_item_from_buyback_slot(slot), Some(expected));
    assert_eq!(player.active_data().buyback_price[0], 123);
    assert_eq!(player.active_data().buyback_timestamp[0], 456);
}

#[test]
fn soulbound_tradeable_item_set_matches_cpp_add_remove_and_update() {
    let mut player = Player::new(None, false);
    let mut keep = item_with_guid_entry(1200, 7000);
    keep.set_owner_guid(player.guid());
    let mut expired = item_with_guid_entry(1201, 7001);
    expired.set_owner_guid(player.guid());
    expired.set_create_played_time(10);
    let mut wrong_owner = item_with_guid_entry(1202, 7002);
    wrong_owner.set_owner_guid(ObjectGuid::create_player(1, 99));
    let missing = item_with_guid_entry(1203, 7003);
    let removed_directly = item_with_guid_entry(1204, 7004);

    player.add_tradeable_item(&keep);
    player.add_tradeable_item(&expired);
    player.add_tradeable_item(&wrong_owner);
    player.add_tradeable_item(&missing);
    player.add_tradeable_item(&removed_directly);
    player.remove_tradeable_item(&removed_directly);

    assert!(
        player
            .soulbound_tradeable_items()
            .contains(&keep.object().guid())
    );
    assert!(
        !player
            .soulbound_tradeable_items()
            .contains(&removed_directly.object().guid())
    );

    let removed = player.update_soulbound_trade_items(&[
        SoulboundTradeableItemRef::from_item(&keep, 7_200),
        SoulboundTradeableItemRef::from_item(&expired, 7_211),
        SoulboundTradeableItemRef::new(
            wrong_owner.object().guid(),
            wrong_owner.owner_guid(),
            false,
        ),
    ]);

    assert!(
        player
            .soulbound_tradeable_items()
            .contains(&keep.object().guid())
    );
    assert_eq!(player.soulbound_tradeable_items().len(), 1);
    assert!(removed.contains(&expired.object().guid()));
    assert!(removed.contains(&wrong_owner.object().guid()));
    assert!(removed.contains(&missing.object().guid()));
    assert!(!removed.contains(&removed_directly.object().guid()));
}

#[test]
fn item_duration_list_matches_cpp_add_remove_and_update_plan() {
    let mut player = Player::new(None, false);
    let mut item = item_with_guid_entry(1210, 7100);
    assert_eq!(player.add_item_durations(&item), None);
    assert!(player.item_durations().is_empty());

    item.set_expiration(900);
    assert_eq!(
        player.add_item_durations(&item),
        Some(PlayerItemTimeUpdate {
            item_guid: item.object().guid(),
            expiration: 900,
        })
    );
    player.add_item_durations(&item);
    assert_eq!(
        player.item_durations(),
        &[item.object().guid(), item.object().guid()]
    );

    assert!(player.remove_item_durations(&item));
    assert_eq!(player.item_durations(), &[item.object().guid()]);

    assert!(
        player
            .update_item_duration_plan(
                &[ItemDurationRef::new(item.object().guid(), 900, false)],
                300,
                true,
            )
            .is_empty()
    );
    assert_eq!(
        player.update_item_duration_plan(
            &[ItemDurationRef::new(item.object().guid(), 900, false)],
            300,
            false,
        ),
        vec![UpdateItemDurationAction::UpdateExpiration {
            item_guid: item.object().guid(),
            expiration: 600,
        }]
    );
    assert_eq!(
        player.update_item_duration_plan(
            &[ItemDurationRef::new(item.object().guid(), 900, true)],
            900,
            true,
        ),
        vec![UpdateItemDurationAction::Expire {
            item_guid: item.object().guid(),
        }]
    );
    assert_eq!(
        player.update_item_duration_plan(&[], 1, false),
        vec![UpdateItemDurationAction::MissingItem {
            item_guid: item.object().guid(),
        }]
    );
}

#[test]
fn enchantment_durations_match_cpp_add_replace_remove_and_tick() {
    let mut player = Player::new(None, false);
    let mut item = item_with_guid_entry(1220, 7200);
    item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 500, 5_000, 0);

    assert_eq!(
        player.add_enchantment_durations(&mut item),
        vec![PlayerEnchantTimeUpdate {
            item_guid: item.object().guid(),
            slot: EnchantmentSlot::EnhancementTemporary,
            duration_secs: 5,
        }]
    );
    assert_eq!(
        player.enchant_durations(),
        &[PlayerEnchantDuration {
            item_guid: item.object().guid(),
            slot: EnchantmentSlot::EnhancementTemporary,
            left_duration_ms: 5_000,
        }]
    );

    assert_eq!(
        player.add_enchantment_duration(&mut item, EnchantmentSlot::EnhancementTemporary, 8_000,),
        Some(PlayerEnchantTimeUpdate {
            item_guid: item.object().guid(),
            slot: EnchantmentSlot::EnhancementTemporary,
            duration_secs: 8,
        })
    );
    assert_eq!(item.data().enchantments[1].duration, 5_000);
    assert_eq!(player.enchant_durations()[0].left_duration_ms, 8_000);

    assert!(
        player
            .update_enchant_time(
                &[PlayerEnchantDurationItemRef::new(
                    item.object().guid(),
                    EnchantmentSlot::EnhancementTemporary,
                    500,
                )],
                3_000,
            )
            .is_empty()
    );
    assert_eq!(player.enchant_durations()[0].left_duration_ms, 5_000);
    assert_eq!(
        player.update_enchant_time(
            &[PlayerEnchantDurationItemRef::new(
                item.object().guid(),
                EnchantmentSlot::EnhancementTemporary,
                500,
            )],
            5_000,
        ),
        vec![UpdateEnchantTimeAction::ClearExpired {
            item_guid: item.object().guid(),
            slot: EnchantmentSlot::EnhancementTemporary,
        }]
    );
    assert!(player.enchant_durations().is_empty());
}

#[test]
fn enchantment_duration_remove_saves_left_duration_unlike_reference_cleanup() {
    let mut player = Player::new(None, false);
    let mut item = item_with_guid_entry(1230, 7300);
    item.set_enchantment(EnchantmentSlot::EnhancementPermanent, 600, 9_000, 0);
    player.add_enchantment_duration(&mut item, EnchantmentSlot::EnhancementPermanent, 9_000);
    player.update_enchant_time(
        &[PlayerEnchantDurationItemRef::new(
            item.object().guid(),
            EnchantmentSlot::EnhancementPermanent,
            600,
        )],
        4_000,
    );

    let removed = player.remove_enchantment_durations(&mut item);
    assert_eq!(
        removed,
        vec![PlayerEnchantDuration {
            item_guid: item.object().guid(),
            slot: EnchantmentSlot::EnhancementPermanent,
            left_duration_ms: 5_000,
        }]
    );
    assert_eq!(item.data().enchantments[0].duration, 5_000);
    assert!(player.enchant_durations().is_empty());

    item.set_enchantment_duration(EnchantmentSlot::EnhancementPermanent, 9_000);
    player.add_enchantment_duration(&mut item, EnchantmentSlot::EnhancementPermanent, 7_000);
    let removed_refs = player.remove_enchantment_duration_references(&item);
    assert_eq!(removed_refs[0].left_duration_ms, 7_000);
    assert_eq!(item.data().enchantments[0].duration, 9_000);
    assert!(player.enchant_durations().is_empty());
}

#[test]
fn enchantment_time_update_removes_missing_or_zero_enchantments_like_cpp() {
    let mut player = Player::new(None, false);
    let mut missing = item_with_guid_entry(1240, 7400);
    let mut zero = item_with_guid_entry(1241, 7401);
    player.add_enchantment_duration(&mut missing, EnchantmentSlot::EnhancementSocket, 2_000);
    player.add_enchantment_duration(&mut zero, EnchantmentSlot::EnhancementSocket2, 3_000);

    assert_eq!(
        player.update_enchant_time(
            &[PlayerEnchantDurationItemRef::new(
                zero.object().guid(),
                EnchantmentSlot::EnhancementSocket2,
                0,
            )],
            100,
        ),
        vec![
            UpdateEnchantTimeAction::RemoveMissingEnchantment {
                item_guid: missing.object().guid(),
                slot: EnchantmentSlot::EnhancementSocket,
            },
            UpdateEnchantTimeAction::RemoveMissingEnchantment {
                item_guid: zero.object().guid(),
                slot: EnchantmentSlot::EnhancementSocket2,
            },
        ]
    );
    assert!(player.enchant_durations().is_empty());
}

#[test]
fn send_duration_plans_follow_cpp_duration_lists() {
    let mut player = Player::new(None, false);
    let mut item = item_with_guid_entry(1245, 7450);
    item.set_expiration(1_200);
    player.add_item_durations(&item);
    player.add_item_durations(&item);

    assert_eq!(
        player.send_item_durations_plan(&[ItemDurationRef::new(
            item.object().guid(),
            1_200,
            false,
        )]),
        vec![
            PlayerItemTimeUpdate {
                item_guid: item.object().guid(),
                expiration: 1_200,
            },
            PlayerItemTimeUpdate {
                item_guid: item.object().guid(),
                expiration: 1_200,
            },
        ]
    );

    item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 700, 4_500, 0);
    player.add_enchantment_duration(&mut item, EnchantmentSlot::EnhancementTemporary, 4_500);
    player.add_enchantment_duration(&mut item, EnchantmentSlot::EnhancementPermanent, 9_999);
    assert_eq!(
        player.send_enchantment_durations_plan(),
        vec![
            PlayerEnchantTimeUpdate {
                item_guid: item.object().guid(),
                slot: EnchantmentSlot::EnhancementTemporary,
                duration_secs: 4,
            },
            PlayerEnchantTimeUpdate {
                item_guid: item.object().guid(),
                slot: EnchantmentSlot::EnhancementPermanent,
                duration_secs: 9,
            },
        ]
    );
}

#[test]
fn apply_enchantment_plan_matches_cpp_early_guards() {
    let mut player = Player::new(None, false);
    player.unit_mut().set_level(9);
    let mut item = item_with_guid_entry(1246, 7460);
    item.set_slot(EQUIPMENT_SLOT_CHEST);
    item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 900, 0, 0);

    assert_eq!(
        player.apply_enchantment_plan(
            None,
            EnchantmentSlot::EnhancementTemporary,
            Some(ApplyEnchantmentTemplateRef::new(900)),
            ApplyEnchantmentArgs::apply(),
        ),
        ApplyEnchantmentPlan {
            result: ApplyEnchantmentResult::Skipped(ApplyEnchantmentSkipReason::MissingItem),
        }
    );

    let mut inventory_item = item_with_guid_entry(1247, 7461);
    inventory_item.set_slot(INVENTORY_SLOT_ITEM_START);
    inventory_item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 900, 0, 0);
    assert_eq!(
        player.apply_enchantment_plan(
            Some(&mut inventory_item),
            EnchantmentSlot::EnhancementTemporary,
            Some(ApplyEnchantmentTemplateRef::new(900)),
            ApplyEnchantmentArgs::apply(),
        ),
        ApplyEnchantmentPlan {
            result: ApplyEnchantmentResult::Skipped(ApplyEnchantmentSkipReason::NotEquipped),
        }
    );

    assert_eq!(
        player.apply_enchantment_plan(
            Some(&mut item),
            EnchantmentSlot::EnhancementPermanent,
            Some(ApplyEnchantmentTemplateRef::new(900)),
            ApplyEnchantmentArgs::apply(),
        ),
        ApplyEnchantmentPlan {
            result: ApplyEnchantmentResult::Skipped(ApplyEnchantmentSkipReason::NoEnchantment),
        }
    );
    assert_eq!(
        player.apply_enchantment_plan(
            Some(&mut item),
            EnchantmentSlot::EnhancementTemporary,
            None,
            ApplyEnchantmentArgs::apply(),
        ),
        ApplyEnchantmentPlan {
            result: ApplyEnchantmentResult::Skipped(
                ApplyEnchantmentSkipReason::MissingEnchantmentTemplate,
            ),
        }
    );

    let mut condition_blocked = ApplyEnchantmentTemplateRef::new(900);
    condition_blocked.condition_id = 1;
    condition_blocked.condition_fits = false;
    assert_eq!(
        player.apply_enchantment_plan(
            Some(&mut item),
            EnchantmentSlot::EnhancementTemporary,
            Some(condition_blocked),
            ApplyEnchantmentArgs::apply(),
        ),
        ApplyEnchantmentPlan {
            result: ApplyEnchantmentResult::Skipped(ApplyEnchantmentSkipReason::ConditionFailed,),
        }
    );

    let mut condition_ignored_args = ApplyEnchantmentArgs::apply();
    condition_ignored_args.ignore_condition = true;
    assert_eq!(
        player
            .apply_enchantment_plan(
                Some(&mut item),
                EnchantmentSlot::EnhancementTemporary,
                Some(condition_blocked),
                condition_ignored_args,
            )
            .result,
        ApplyEnchantmentResult::Applied {
            item_guid: item.object().guid(),
            slot: EnchantmentSlot::EnhancementTemporary,
            enchantment_id: 900,
            apply: true,
            effects_allowed: true,
            update_permanent_visible_item: false,
            duration_action: None,
        }
    );

    let mut level_blocked = ApplyEnchantmentTemplateRef::new(900);
    level_blocked.min_level = 10;
    assert_eq!(
        player.apply_enchantment_plan(
            Some(&mut item),
            EnchantmentSlot::EnhancementTemporary,
            Some(level_blocked),
            ApplyEnchantmentArgs::apply(),
        ),
        ApplyEnchantmentPlan {
            result: ApplyEnchantmentResult::Skipped(ApplyEnchantmentSkipReason::PlayerLevelTooLow,),
        }
    );

    let mut skill_blocked = ApplyEnchantmentTemplateRef::new(900);
    skill_blocked.required_skill_id = 164;
    skill_blocked.required_skill_rank = 75;
    skill_blocked.required_skill_value = 74;
    assert_eq!(
        player.apply_enchantment_plan(
            Some(&mut item),
            EnchantmentSlot::EnhancementTemporary,
            Some(skill_blocked),
            ApplyEnchantmentArgs::apply(),
        ),
        ApplyEnchantmentPlan {
            result: ApplyEnchantmentResult::Skipped(
                ApplyEnchantmentSkipReason::RequiredSkillTooLow,
            ),
        }
    );
}

#[test]
fn apply_enchantment_plan_matches_cpp_socket_requirement_order() {
    let mut player = Player::new(None, false);
    player.unit_mut().set_level(80);
    let mut item = item_with_guid_entry(1248, 7462);
    item.set_slot(EQUIPMENT_SLOT_CHEST);
    item.set_enchantment(EnchantmentSlot::EnhancementSocket, 901, 0, 0);

    let mut args = ApplyEnchantmentArgs::apply();
    args.socket_context = Some(ApplyEnchantmentSocketContext::prismatic(None, None));
    assert_eq!(
        player.apply_enchantment_plan(
            Some(&mut item),
            EnchantmentSlot::EnhancementSocket,
            Some(ApplyEnchantmentTemplateRef::new(901)),
            args,
        ),
        ApplyEnchantmentPlan {
            result: ApplyEnchantmentResult::Skipped(
                ApplyEnchantmentSkipReason::MissingPrismaticEnchantment,
            ),
        }
    );

    let mut prismatic = ApplyEnchantmentTemplateRef::new(902);
    prismatic.required_skill_id = 755;
    prismatic.required_skill_rank = 350;
    prismatic.required_skill_value = 349;
    args.socket_context = Some(ApplyEnchantmentSocketContext::prismatic(
        Some(prismatic),
        None,
    ));
    assert_eq!(
        player.apply_enchantment_plan(
            Some(&mut item),
            EnchantmentSlot::EnhancementSocket,
            Some(ApplyEnchantmentTemplateRef::new(901)),
            args,
        ),
        ApplyEnchantmentPlan {
            result: ApplyEnchantmentResult::Skipped(
                ApplyEnchantmentSkipReason::PrismaticRequiredSkillTooLow,
            ),
        }
    );

    prismatic.required_skill_value = 350;
    args.socket_context = Some(ApplyEnchantmentSocketContext::prismatic(
        Some(prismatic),
        Some(ApplyEnchantmentGemRequirementRef::new(755, 400, 399)),
    ));
    assert_eq!(
        player.apply_enchantment_plan(
            Some(&mut item),
            EnchantmentSlot::EnhancementSocket,
            Some(ApplyEnchantmentTemplateRef::new(901)),
            args,
        ),
        ApplyEnchantmentPlan {
            result: ApplyEnchantmentResult::Skipped(
                ApplyEnchantmentSkipReason::GemRequiredSkillTooLow,
            ),
        }
    );

    args.socket_context = Some(ApplyEnchantmentSocketContext::colored(
        1,
        Some(ApplyEnchantmentGemRequirementRef::new(755, 400, 400)),
    ));
    assert_eq!(
        player
            .apply_enchantment_plan(
                Some(&mut item),
                EnchantmentSlot::EnhancementSocket,
                Some(ApplyEnchantmentTemplateRef::new(901)),
                args,
            )
            .result,
        ApplyEnchantmentResult::Applied {
            item_guid: item.object().guid(),
            slot: EnchantmentSlot::EnhancementSocket,
            enchantment_id: 901,
            apply: true,
            effects_allowed: true,
            update_permanent_visible_item: false,
            duration_action: None,
        }
    );
}

#[test]
fn apply_enchantment_plan_updates_duration_and_visible_shape_like_cpp() {
    let mut player = Player::new(None, false);
    player.unit_mut().set_level(80);
    let mut item = item_with_guid_entry(1249, 7463);
    item.set_slot(EQUIPMENT_SLOT_MAINHAND);
    item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 903, 6_000, 0);

    assert_eq!(
        player.apply_enchantment_plan(
            Some(&mut item),
            EnchantmentSlot::EnhancementTemporary,
            Some(ApplyEnchantmentTemplateRef::new(903)),
            ApplyEnchantmentArgs::apply(),
        ),
        ApplyEnchantmentPlan {
            result: ApplyEnchantmentResult::Applied {
                item_guid: item.object().guid(),
                slot: EnchantmentSlot::EnhancementTemporary,
                enchantment_id: 903,
                apply: true,
                effects_allowed: true,
                update_permanent_visible_item: false,
                duration_action: Some(ApplyEnchantmentDurationAction::Added(
                    PlayerEnchantTimeUpdate {
                        item_guid: item.object().guid(),
                        slot: EnchantmentSlot::EnhancementTemporary,
                        duration_secs: 6,
                    },
                )),
            },
        }
    );

    assert_eq!(
        player.apply_enchantment_plan(
            Some(&mut item),
            EnchantmentSlot::EnhancementTemporary,
            Some(ApplyEnchantmentTemplateRef::new(903)),
            ApplyEnchantmentArgs::remove(),
        ),
        ApplyEnchantmentPlan {
            result: ApplyEnchantmentResult::Applied {
                item_guid: item.object().guid(),
                slot: EnchantmentSlot::EnhancementTemporary,
                enchantment_id: 903,
                apply: false,
                effects_allowed: true,
                update_permanent_visible_item: false,
                duration_action: Some(ApplyEnchantmentDurationAction::Removed {
                    item_guid: item.object().guid(),
                    slot: EnchantmentSlot::EnhancementTemporary,
                }),
            },
        }
    );
    assert!(player.enchant_durations().is_empty());

    item.set_enchantment(EnchantmentSlot::EnhancementPermanent, 904, 0, 0);
    item.set_max_durability(100);
    item.set_durability(0);
    assert_eq!(
        player
            .apply_enchantment_plan(
                Some(&mut item),
                EnchantmentSlot::EnhancementPermanent,
                Some(ApplyEnchantmentTemplateRef::new(904)),
                ApplyEnchantmentArgs::apply(),
            )
            .result,
        ApplyEnchantmentResult::Applied {
            item_guid: item.object().guid(),
            slot: EnchantmentSlot::EnhancementPermanent,
            enchantment_id: 904,
            apply: true,
            effects_allowed: false,
            update_permanent_visible_item: true,
            duration_action: None,
        }
    );
}

#[test]
fn apply_enchantment_effect_actions_match_cpp_deferred_noop_and_spell_cases() {
    let player = Player::new(None, false);
    let mut item = item_with_guid_entry(12491, 7464);
    item.set_slot(EQUIPMENT_SLOT_CHEST);

    let effects = [
        ApplyEnchantmentEffectRef::known(ItemEnchantmentType::None, 0, 0),
        ApplyEnchantmentEffectRef::known(ItemEnchantmentType::CombatSpell, 0, 0),
        ApplyEnchantmentEffectRef::known(ItemEnchantmentType::UseSpell, 0, 0),
        ApplyEnchantmentEffectRef::known(ItemEnchantmentType::EquipSpell, 0, 1234),
        ApplyEnchantmentEffectRef::known(ItemEnchantmentType::EquipSpell, 0, 0),
        ApplyEnchantmentEffectRef::known(ItemEnchantmentType::PrismaticSocket, 0, 0),
        ApplyEnchantmentEffectRef::unknown(99, 0, 0),
    ];

    assert_eq!(
        player.apply_enchantment_effect_actions(
            &item,
            None,
            EnchantmentSlot::EnhancementTemporary,
            true,
            &effects,
        ),
        vec![
            ApplyEnchantmentEffectAction::Noop,
            ApplyEnchantmentEffectAction::DeferredCombatSpell,
            ApplyEnchantmentEffectAction::DeferredUseSpell,
            ApplyEnchantmentEffectAction::CastEquipSpell {
                spell_id: 1234,
                item_guid: item.object().guid(),
            },
            ApplyEnchantmentEffectAction::Noop,
            ApplyEnchantmentEffectAction::Noop,
            ApplyEnchantmentEffectAction::Unknown { effect_type: 99 },
        ]
    );
    assert_eq!(
        player.apply_enchantment_effect_actions(
            &item,
            None,
            EnchantmentSlot::EnhancementTemporary,
            false,
            &[ApplyEnchantmentEffectRef::known(
                ItemEnchantmentType::EquipSpell,
                0,
                1234,
            )],
        ),
        vec![ApplyEnchantmentEffectAction::RemoveEquipSpellAura {
            spell_id: 1234,
            item_guid: item.object().guid(),
        }]
    );
}

#[test]
fn item_stat_bonus_actions_match_cpp_apply_item_bonuses_stat_loop() {
    let stats = [
        (ItemModType::Strength as i8, 12),
        (ItemModType::HitRating as i8, 5),
        (-1, 99),
        (ItemModType::SpellPower as i8, 0),
        (-1, 0),
        (-1, 0),
        (-1, 0),
        (-1, 0),
        (-1, 0),
        (-1, 0),
    ];

    assert_eq!(
        item_stat_bonus_actions_like_cpp(&stats, true),
        vec![
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::StatStrength,
                modifier: ApplyEnchantmentUnitModifier::BaseValue,
                amount: 12,
                apply: true,
            },
            ApplyEnchantmentEffectAction::UpdateStatBuffMod(Stats::Strength),
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HitMelee,
                amount: 5,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HitRanged,
                amount: 5,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HitSpell,
                amount: 5,
                apply: true,
            },
        ],
    );
}

#[test]
fn item_scaling_stat_bonus_actions_match_cpp_scaled_stat_loop() {
    let mut stat_ids = [-1; 10];
    stat_ids[0] = ItemModType::Strength as i32;
    stat_ids[1] = ItemModType::HitRating as i32;
    stat_ids[2] = ItemModType::SpellPower as i32;
    let mut bonuses = [0; 10];
    bonuses[0] = 5_000;
    bonuses[1] = 2_500;
    bonuses[2] = 0;

    assert_eq!(
        item_scaling_stat_bonus_actions_like_cpp(&stat_ids, &bonuses, 200, true),
        vec![
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::StatStrength,
                modifier: ApplyEnchantmentUnitModifier::BaseValue,
                amount: 100,
                apply: true,
            },
            ApplyEnchantmentEffectAction::UpdateStatBuffMod(Stats::Strength),
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HitMelee,
                amount: 50,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HitRanged,
                amount: 50,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HitSpell,
                amount: 50,
                apply: true,
            },
        ],
        "C++ computes val = getssdMultiplier(mask) * ScalingStatDistribution::Bonus[i] / 10000"
    );
}

#[test]
fn item_stat_bonus_actions_cover_cpp_item_bonus_only_stat_cases() {
    let stats = [
        (ItemModType::AgiStrInt as i8, 7),
        (ItemModType::ExtraArmor as i8, 40),
        (ItemModType::FireResistance as i8, 9),
        (ItemModType::HasteMeleeRating as i8, 3),
        (ItemModType::HasteRangedRating as i8, 4),
        (-1, 0),
        (-1, 0),
        (-1, 0),
        (-1, 0),
        (-1, 0),
    ];

    assert_eq!(
        item_stat_bonus_actions_like_cpp(&stats, true),
        vec![
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::StatAgility,
                modifier: ApplyEnchantmentUnitModifier::BaseValue,
                amount: 7,
                apply: true,
            },
            ApplyEnchantmentEffectAction::UpdateStatBuffMod(Stats::Agility),
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::StatStrength,
                modifier: ApplyEnchantmentUnitModifier::BaseValue,
                amount: 7,
                apply: true,
            },
            ApplyEnchantmentEffectAction::UpdateStatBuffMod(Stats::Strength),
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::StatIntellect,
                modifier: ApplyEnchantmentUnitModifier::BaseValue,
                amount: 7,
                apply: true,
            },
            ApplyEnchantmentEffectAction::UpdateStatBuffMod(Stats::Intellect),
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::Armor,
                modifier: ApplyEnchantmentUnitModifier::TotalValue,
                amount: 40,
                apply: true,
            },
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::Resistance(SpellSchools::Fire as u32),
                modifier: ApplyEnchantmentUnitModifier::BaseValue,
                amount: 9,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HasteMelee,
                amount: 3,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HasteRanged,
                amount: 4,
                apply: true,
            },
        ],
    );
}

#[test]
fn item_resistance_bonus_actions_match_cpp_template_resistance_loop() {
    let mut resistances = [0i16; 7];
    resistances[SpellSchools::Normal as usize] = 120;
    resistances[SpellSchools::Holy as usize] = 3;
    resistances[SpellSchools::Fire as usize] = 7;

    assert_eq!(
        item_resistance_bonus_actions_like_cpp(&resistances, false),
        vec![
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::Resistance(SpellSchools::Normal as u32),
                modifier: ApplyEnchantmentUnitModifier::BaseValue,
                amount: 120,
                apply: false,
            },
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::Resistance(SpellSchools::Holy as u32),
                modifier: ApplyEnchantmentUnitModifier::BaseValue,
                amount: 3,
                apply: false,
            },
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::Resistance(SpellSchools::Fire as u32),
                modifier: ApplyEnchantmentUnitModifier::BaseValue,
                amount: 7,
                apply: false,
            },
        ],
    );
}

#[test]
fn item_shield_block_bonus_action_matches_cpp_direct_update_field_assignment() {
    assert_eq!(
        item_shield_block_bonus_action_like_cpp(42, true, true),
        Some(ApplyEnchantmentEffectAction::SetShieldBlockValue { amount: 42 }),
    );
    assert_eq!(
        item_shield_block_bonus_action_like_cpp(42, true, false),
        Some(ApplyEnchantmentEffectAction::SetShieldBlockValue { amount: 0 }),
    );
    assert_eq!(
        item_shield_block_bonus_action_like_cpp(42, false, true),
        None
    );
    assert_eq!(item_shield_block_bonus_action_like_cpp(0, true, true), None);
}

#[test]
fn item_weapon_damage_actions_match_cpp_direct_apply_weapon_damage() {
    assert_eq!(
        item_weapon_damage_actions_like_cpp(
            EQUIPMENT_SLOT_MAINHAND,
            InventoryType::Weapon,
            12.0,
            18.0,
            2600,
            true,
            false,
            true,
            false,
            true,
        ),
        vec![
            ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                attack_type: WeaponAttackType::BaseAttack,
                bound: WeaponDamageBoundLikeCpp::Min,
                amount_bits: 12.0f32.to_bits(),
            },
            ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                attack_type: WeaponAttackType::BaseAttack,
                bound: WeaponDamageBoundLikeCpp::Max,
                amount_bits: 18.0f32.to_bits(),
            },
            ApplyEnchantmentEffectAction::SetBaseAttackTime {
                attack_type: WeaponAttackType::BaseAttack,
                time_ms: 2600,
            },
            ApplyEnchantmentEffectAction::UpdateDamagePhysical {
                attack_type: WeaponAttackType::BaseAttack,
            },
        ],
    );

    assert_eq!(
        item_weapon_damage_actions_like_cpp(
            EQUIPMENT_SLOT_MAINHAND,
            InventoryType::Weapon,
            12.0,
            18.0,
            2600,
            false,
            false,
            true,
            false,
            true,
        ),
        vec![
            ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                attack_type: WeaponAttackType::BaseAttack,
                bound: WeaponDamageBoundLikeCpp::Min,
                amount_bits: BASE_MINDAMAGE.to_bits(),
            },
            ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                attack_type: WeaponAttackType::BaseAttack,
                bound: WeaponDamageBoundLikeCpp::Max,
                amount_bits: BASE_MAXDAMAGE.to_bits(),
            },
            ApplyEnchantmentEffectAction::SetBaseAttackTime {
                attack_type: WeaponAttackType::BaseAttack,
                time_ms: 2000,
            },
            ApplyEnchantmentEffectAction::UpdateDamagePhysical {
                attack_type: WeaponAttackType::BaseAttack,
            },
        ],
    );
}

#[test]
fn item_weapon_damage_actions_match_cpp_attack_slot_and_gate_rules() {
    assert_eq!(
        item_weapon_damage_actions_like_cpp(
            EQUIPMENT_SLOT_MAINHAND,
            InventoryType::RangedRight,
            20.0,
            30.0,
            1800,
            true,
            false,
            true,
            true,
            true,
        ),
        vec![
            ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                attack_type: WeaponAttackType::RangedAttack,
                bound: WeaponDamageBoundLikeCpp::Min,
                amount_bits: 20.0f32.to_bits(),
            },
            ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                attack_type: WeaponAttackType::RangedAttack,
                bound: WeaponDamageBoundLikeCpp::Max,
                amount_bits: 30.0f32.to_bits(),
            },
            ApplyEnchantmentEffectAction::UpdateDamagePhysical {
                attack_type: WeaponAttackType::RangedAttack,
            },
        ],
        "C++ skips SetBaseAttackTime when the shapeshift form has CombatRoundTime"
    );
    assert_eq!(
        item_weapon_damage_actions_like_cpp(
            EQUIPMENT_SLOT_OFFHAND,
            InventoryType::WeaponOffhand,
            9.0,
            11.0,
            0,
            true,
            false,
            true,
            false,
            false,
        ),
        vec![
            ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                attack_type: WeaponAttackType::OffAttack,
                bound: WeaponDamageBoundLikeCpp::Min,
                amount_bits: 9.0f32.to_bits(),
            },
            ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                attack_type: WeaponAttackType::OffAttack,
                bound: WeaponDamageBoundLikeCpp::Max,
                amount_bits: 11.0f32.to_bits(),
            },
        ],
        "C++ skips UpdateDamagePhysical when CanModifyStats is false"
    );
    assert!(
        item_weapon_damage_actions_like_cpp(
            EQUIPMENT_SLOT_MAINHAND,
            InventoryType::Weapon,
            12.0,
            18.0,
            2600,
            true,
            false,
            false,
            false,
            true,
        )
        .is_empty()
    );
    assert!(
        item_weapon_damage_actions_like_cpp(
            EQUIPMENT_SLOT_HEAD,
            InventoryType::Weapon,
            12.0,
            18.0,
            2600,
            true,
            false,
            true,
            false,
            true,
        )
        .is_empty()
    );
}

#[test]
fn apply_enchantment_effect_actions_match_cpp_damage_and_totem_attack_slot_rules() {
    let player = Player::new(None, false);
    let mut item = item_with_guid_entry(12492, 7465);
    let weapon = ItemStorageTemplate {
        inventory_type: InventoryType::Weapon,
        ..ItemStorageTemplate::regular_item(7465, 1)
    };
    let ranged = ItemStorageTemplate {
        inventory_type: InventoryType::RangedRight,
        ..ItemStorageTemplate::regular_item(7466, 1)
    };

    item.set_slot(EQUIPMENT_SLOT_MAINHAND);
    assert_eq!(
        player.apply_enchantment_effect_actions(
            &item,
            Some(&weapon),
            EnchantmentSlot::EnhancementTemporary,
            true,
            &[ApplyEnchantmentEffectRef::known(
                ItemEnchantmentType::Damage,
                0,
                0,
            )],
        ),
        vec![ApplyEnchantmentEffectAction::UpdateDamageDoneMods {
            attack_type: WeaponAttackType::BaseAttack,
            modifier_slot: -1,
        }]
    );
    assert_eq!(
        player.apply_enchantment_effect_actions(
            &item,
            Some(&ranged),
            EnchantmentSlot::EnhancementTemporary,
            false,
            &[ApplyEnchantmentEffectRef::known(
                ItemEnchantmentType::Totem,
                0,
                0,
            )],
        ),
        vec![ApplyEnchantmentEffectAction::UpdateDamageDoneMods {
            attack_type: WeaponAttackType::RangedAttack,
            modifier_slot: EnchantmentSlot::EnhancementTemporary as i16,
        }]
    );

    item.set_slot(EQUIPMENT_SLOT_OFFHAND);
    assert_eq!(
        player.apply_enchantment_effect_actions(
            &item,
            Some(&weapon),
            EnchantmentSlot::EnhancementTemporary,
            true,
            &[ApplyEnchantmentEffectRef::known(
                ItemEnchantmentType::Damage,
                0,
                0,
            )],
        ),
        vec![ApplyEnchantmentEffectAction::UpdateDamageDoneMods {
            attack_type: WeaponAttackType::OffAttack,
            modifier_slot: -1,
        }]
    );

    item.set_slot(EQUIPMENT_SLOT_CHEST);
    assert_eq!(
        player.apply_enchantment_effect_actions(
            &item,
            Some(&weapon),
            EnchantmentSlot::EnhancementTemporary,
            true,
            &[ApplyEnchantmentEffectRef::known(
                ItemEnchantmentType::Damage,
                0,
                0,
            )],
        ),
        vec![ApplyEnchantmentEffectAction::Noop]
    );
    assert_eq!(
        player.apply_enchantment_effect_actions(
            &item,
            None,
            EnchantmentSlot::EnhancementTemporary,
            true,
            &[ApplyEnchantmentEffectRef::known(
                ItemEnchantmentType::Damage,
                0,
                0,
            )],
        ),
        vec![ApplyEnchantmentEffectAction::MissingItemTemplateForAttack {
            effect_kind: ApplyEnchantmentEffectKind::Known(ItemEnchantmentType::Damage),
        }]
    );
}

#[test]
fn apply_enchantment_effect_actions_match_cpp_stat_resistance_and_broken_skip() {
    let player = Player::new(None, false);
    let mut item = item_with_guid_entry(12493, 7467);
    item.set_slot(EQUIPMENT_SLOT_CHEST);
    assert_eq!(
        player.apply_enchantment_effect_actions(
            &item,
            None,
            EnchantmentSlot::EnhancementTemporary,
            true,
            &[
                ApplyEnchantmentEffectRef::known(ItemEnchantmentType::Resistance, 17, 2),
                ApplyEnchantmentEffectRef::known(
                    ItemEnchantmentType::Stat,
                    31,
                    ItemModType::Strength as u32,
                ),
            ],
        ),
        vec![
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::Resistance(2),
                modifier: ApplyEnchantmentUnitModifier::TotalValue,
                amount: 17,
                apply: true,
            },
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::StatStrength,
                modifier: ApplyEnchantmentUnitModifier::TotalValue,
                amount: 31,
                apply: true,
            },
            ApplyEnchantmentEffectAction::UpdateStatBuffMod(Stats::Strength),
        ]
    );

    item.set_max_durability(100);
    item.set_durability(0);
    assert!(
        player
            .apply_enchantment_effect_actions(
                &item,
                None,
                EnchantmentSlot::EnhancementTemporary,
                true,
                &[ApplyEnchantmentEffectRef::known(
                    ItemEnchantmentType::Stat,
                    31,
                    ItemModType::Strength as u32,
                )],
            )
            .is_empty()
    );
}

#[test]
fn apply_enchantment_effect_actions_expand_cpp_stat_switch_special_cases() {
    let player = Player::new(None, false);
    let mut item = item_with_guid_entry(12494, 7468);
    item.set_slot(EQUIPMENT_SLOT_CHEST);

    assert_eq!(
        player.apply_enchantment_effect_actions(
            &item,
            None,
            EnchantmentSlot::EnhancementTemporary,
            true,
            &[
                ApplyEnchantmentEffectRef::known(
                    ItemEnchantmentType::Stat,
                    11,
                    ItemModType::HitRating as u32,
                ),
                ApplyEnchantmentEffectRef::known(
                    ItemEnchantmentType::Stat,
                    12,
                    ItemModType::CritRating as u32,
                ),
                ApplyEnchantmentEffectRef::known(
                    ItemEnchantmentType::Stat,
                    13,
                    ItemModType::HasteRating as u32,
                ),
            ],
        ),
        vec![
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HitMelee,
                amount: 11,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HitRanged,
                amount: 11,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HitSpell,
                amount: 11,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::CritMelee,
                amount: 12,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::CritRanged,
                amount: 12,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::CritSpell,
                amount: 12,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HasteMelee,
                amount: 13,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HasteRanged,
                amount: 13,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HasteSpell,
                amount: 13,
                apply: true,
            },
        ]
    );

    assert_eq!(
        player.apply_enchantment_effect_actions(
            &item,
            None,
            EnchantmentSlot::EnhancementTemporary,
            false,
            &[
                ApplyEnchantmentEffectRef::known(
                    ItemEnchantmentType::Stat,
                    20,
                    ItemModType::AttackPower as u32,
                ),
                ApplyEnchantmentEffectRef::known(
                    ItemEnchantmentType::Stat,
                    21,
                    ItemModType::SpellPower as u32,
                ),
                ApplyEnchantmentEffectRef::known(
                    ItemEnchantmentType::Stat,
                    22,
                    ItemModType::BlockValue as u32,
                ),
            ],
        ),
        vec![
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::AttackPower,
                modifier: ApplyEnchantmentUnitModifier::TotalValue,
                amount: 20,
                apply: false,
            },
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::AttackPowerRanged,
                modifier: ApplyEnchantmentUnitModifier::TotalValue,
                amount: 20,
                apply: false,
            },
            ApplyEnchantmentEffectAction::SpellPowerBonus {
                amount: 21,
                apply: false,
            },
            ApplyEnchantmentEffectAction::BaseModFlatValue {
                base_mod: ApplyEnchantmentBaseMod::ShieldBlockValue,
                amount: 22,
                apply: false,
            },
        ]
    );
}

#[test]
fn apply_enchantment_effect_actions_resolve_cpp_random_suffix_amounts() {
    let player = Player::new(None, false);
    let mut item = item_with_guid_entry(12495, 7469);
    item.set_slot(EQUIPMENT_SLOT_CHEST);
    item.set_random_properties_id(-77);
    item.set_property_seed(12_345);

    let random_suffix = ApplyEnchantmentRandomSuffixRef::new(
        77,
        [901, 900, 902, 0, 0],
        [1_000, 2_000, 3_000, 0, 0],
    );

    assert_eq!(
        player.apply_enchantment_effect_actions_for_enchantment(
            &item,
            None,
            EnchantmentSlot::EnhancementTemporary,
            900,
            Some(random_suffix),
            true,
            &[
                ApplyEnchantmentEffectRef::known(ItemEnchantmentType::Resistance, 0, 2),
                ApplyEnchantmentEffectRef::known(
                    ItemEnchantmentType::Stat,
                    0,
                    ItemModType::Strength as u32,
                ),
            ],
        ),
        vec![
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::Resistance(2),
                modifier: ApplyEnchantmentUnitModifier::TotalValue,
                amount: 2_469,
                apply: true,
            },
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::StatStrength,
                modifier: ApplyEnchantmentUnitModifier::TotalValue,
                amount: 2_469,
                apply: true,
            },
            ApplyEnchantmentEffectAction::UpdateStatBuffMod(Stats::Strength),
        ]
    );

    item.set_random_properties_id(-78);
    assert_eq!(
        player.apply_enchantment_effect_actions_for_enchantment(
            &item,
            None,
            EnchantmentSlot::EnhancementTemporary,
            900,
            Some(random_suffix),
            true,
            &[ApplyEnchantmentEffectRef::known(
                ItemEnchantmentType::Stat,
                0,
                ItemModType::Strength as u32,
            )],
        ),
        vec![
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::StatStrength,
                modifier: ApplyEnchantmentUnitModifier::TotalValue,
                amount: 0,
                apply: true,
            },
            ApplyEnchantmentEffectAction::UpdateStatBuffMod(Stats::Strength),
        ]
    );
}

#[test]
fn update_skill_enchantments_plan_matches_cpp_order_and_thresholds() {
    let player = Player::new(None, false);
    let mut later_enchantments = [0; MAX_ENCHANTMENT_SLOT];
    later_enchantments[EnchantmentSlot::EnhancementSocket as usize] = 300;
    later_enchantments[EnchantmentSlot::EnhancementSocketPrismatic as usize] = 400;
    let later = SkillEnchantmentItemRef::new(
        ObjectGuid::create_item(1, 30),
        2,
        later_enchantments,
        [0, 1, 1],
    );

    let mut first_enchantments = [0; MAX_ENCHANTMENT_SLOT];
    first_enchantments[EnchantmentSlot::EnhancementPermanent as usize] = 100;
    let first = SkillEnchantmentItemRef::new(
        ObjectGuid::create_item(1, 10),
        1,
        first_enchantments,
        [1, 1, 1],
    );

    let enchantments = [
        SkillEnchantmentTemplateRef::new(100, 164, 75),
        SkillEnchantmentTemplateRef::new(300, 164, 75),
        SkillEnchantmentTemplateRef::new(400, 164, 75),
    ];

    assert_eq!(
        player.update_skill_enchantments_plan(164, 74, 75, &[later, first], &enchantments),
        vec![
            UpdateSkillEnchantmentAction::Apply {
                item_guid: first.item_guid,
                inventory_slot: 1,
                enchantment_slot: EnchantmentSlot::EnhancementPermanent,
                enchantment_id: 100,
                reason: UpdateSkillEnchantmentReason::EnchantmentRequiredSkill,
            },
            UpdateSkillEnchantmentAction::Apply {
                item_guid: later.item_guid,
                inventory_slot: 2,
                enchantment_slot: EnchantmentSlot::EnhancementSocket,
                enchantment_id: 300,
                reason: UpdateSkillEnchantmentReason::EnchantmentRequiredSkill,
            },
            UpdateSkillEnchantmentAction::Apply {
                item_guid: later.item_guid,
                inventory_slot: 2,
                enchantment_slot: EnchantmentSlot::EnhancementSocket,
                enchantment_id: 300,
                reason: UpdateSkillEnchantmentReason::PrismaticRequiredSkill,
            },
            UpdateSkillEnchantmentAction::Apply {
                item_guid: later.item_guid,
                inventory_slot: 2,
                enchantment_slot: EnchantmentSlot::EnhancementSocketPrismatic,
                enchantment_id: 400,
                reason: UpdateSkillEnchantmentReason::EnchantmentRequiredSkill,
            },
        ]
    );

    assert_eq!(
        player.update_skill_enchantments_plan(164, 75, 74, &[first], &enchantments),
        vec![UpdateSkillEnchantmentAction::Remove {
            item_guid: first.item_guid,
            inventory_slot: 1,
            enchantment_slot: EnchantmentSlot::EnhancementPermanent,
            enchantment_id: 100,
            reason: UpdateSkillEnchantmentReason::EnchantmentRequiredSkill,
        }]
    );
}

#[test]
fn update_skill_enchantments_plan_matches_cpp_missing_template_edges() {
    let player = Player::new(None, false);
    let mut enchantment_ids = [0; MAX_ENCHANTMENT_SLOT];
    enchantment_ids[EnchantmentSlot::EnhancementPermanent as usize] = 100;
    enchantment_ids[EnchantmentSlot::EnhancementTemporary as usize] = 999;
    let item = SkillEnchantmentItemRef::new(
        ObjectGuid::create_item(1, 40),
        0,
        enchantment_ids,
        [1, 1, 1],
    );

    assert_eq!(
        player.update_skill_enchantments_plan(
            164,
            74,
            75,
            &[item],
            &[SkillEnchantmentTemplateRef::new(100, 164, 75)],
        ),
        vec![
            UpdateSkillEnchantmentAction::Apply {
                item_guid: item.item_guid,
                inventory_slot: 0,
                enchantment_slot: EnchantmentSlot::EnhancementPermanent,
                enchantment_id: 100,
                reason: UpdateSkillEnchantmentReason::EnchantmentRequiredSkill,
            },
            UpdateSkillEnchantmentAction::MissingEnchantmentTemplateAbort {
                item_guid: item.item_guid,
                inventory_slot: 0,
                enchantment_slot: EnchantmentSlot::EnhancementTemporary,
                enchantment_id: 999,
            },
        ]
    );

    let mut socket_enchantments = [0; MAX_ENCHANTMENT_SLOT];
    socket_enchantments[EnchantmentSlot::EnhancementSocket as usize] = 300;
    let socket_item = SkillEnchantmentItemRef::new(
        ObjectGuid::create_item(1, 41),
        0,
        socket_enchantments,
        [0, 1, 1],
    );
    assert!(
        player
            .update_skill_enchantments_plan(
                164,
                74,
                75,
                &[socket_item],
                &[SkillEnchantmentTemplateRef::new(300, 755, 100)],
            )
            .is_empty()
    );
}

#[test]
fn send_new_item_plan_matches_cpp_packet_fields_and_delivery() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let mut player = Player::new(None, false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);

    let mut item = item_with_guid_entry(12510, 9001);
    item.set_count(3);
    item.set_slot(7);
    item.set_container_guid_and_slot(ObjectGuid::create_item(1, 700), 4);
    item.set_property_seed(4567);
    item.set_random_properties_id(-89);
    item.set_modifier(ItemModifier::BattlePetSpeciesId, 123);
    item.set_modifier(ItemModifier::BattlePetBreedData, 0x1A00_00BC);
    item.set_modifier(ItemModifier::BattlePetLevel, 25);

    let mut args = SendNewItemArgs::new(3, true, false);
    args.quantity_in_inventory = 9;
    assert_eq!(
        player.send_new_item_plan(Some(&item), SendNewItemTemplateRef::new(777, false), args,),
        Some(SendNewItemPlan {
            player_guid,
            item_guid: item.object().guid(),
            item_entry: 9001,
            item_instance: SendNewItemInstancePlan {
                item_id: 9001,
                random_properties_seed: 4567,
                random_properties_id: -89,
                modifications: vec![
                    SendNewItemModifier {
                        value: 123,
                        modifier_type: ItemModifier::BattlePetSpeciesId as u8,
                    },
                    SendNewItemModifier {
                        value: 0x1A00_00BC,
                        modifier_type: ItemModifier::BattlePetBreedData as u8,
                    },
                    SendNewItemModifier {
                        value: 25,
                        modifier_type: ItemModifier::BattlePetLevel as u8,
                    },
                ],
            },
            slot: 4,
            slot_in_bag: 7,
            quest_log_item_id: 777,
            quantity: 3,
            quantity_in_inventory: 9,
            battle_pet_species_id: 123,
            battle_pet_breed_id: 0xBC,
            battle_pet_breed_quality: 0x1A,
            battle_pet_level: 25,
            pushed: true,
            created: false,
            display_text: SendNewItemDisplayText::Normal,
            dungeon_encounter_id: 0,
            is_encounter_loot: false,
            delivery: SendNewItemDelivery::Direct,
        })
    );

    let mut encounter_args = SendNewItemArgs::new(1, false, true);
    encounter_args.broadcast = true;
    encounter_args.player_in_group = true;
    encounter_args.dungeon_encounter_id = 615;
    encounter_args.quantity_in_inventory = 10;
    assert_eq!(
        player
            .send_new_item_plan(
                Some(&item),
                SendNewItemTemplateRef::new(0, false),
                encounter_args,
            )
            .unwrap()
            .delivery,
        SendNewItemDelivery::GroupBroadcast
    );
    let encounter = player
        .send_new_item_plan(
            Some(&item),
            SendNewItemTemplateRef::new(0, true),
            encounter_args,
        )
        .unwrap();
    assert_eq!(encounter.slot_in_bag, -1);
    assert_eq!(
        encounter.display_text,
        SendNewItemDisplayText::EncounterLoot
    );
    assert!(encounter.is_encounter_loot);
    assert_eq!(encounter.dungeon_encounter_id, 615);
    assert_eq!(encounter.delivery, SendNewItemDelivery::Direct);

    assert_eq!(
        player.send_new_item_plan(None, SendNewItemTemplateRef::new(0, false), args),
        None
    );
}

#[test]
fn remove_arena_enchantments_cleans_duration_list_like_cpp() {
    let mut player = Player::new(None, false);
    let mut allowed = item_with_guid_entry(1250, 7500);
    let mut blocked = item_with_guid_entry(1251, 7501);
    let mut zero = item_with_guid_entry(1252, 7502);

    player.add_enchantment_duration(&mut allowed, EnchantmentSlot::EnhancementTemporary, 10_000);
    player.add_enchantment_duration(&mut blocked, EnchantmentSlot::EnhancementTemporary, 11_000);
    player.add_enchantment_duration(&mut zero, EnchantmentSlot::EnhancementTemporary, 12_000);
    player.add_enchantment_duration(&mut blocked, EnchantmentSlot::EnhancementPermanent, 1_000);

    let actions = player.remove_arena_enchantments(
        EnchantmentSlot::EnhancementTemporary,
        &[
            ArenaEnchantmentItemRef::new(
                allowed.object().guid(),
                INVENTORY_SLOT_BAG_0,
                EQUIPMENT_SLOT_MAINHAND,
                10,
                true,
            ),
            ArenaEnchantmentItemRef::new(
                blocked.object().guid(),
                INVENTORY_SLOT_BAG_0,
                EQUIPMENT_SLOT_OFFHAND,
                20,
                false,
            ),
            ArenaEnchantmentItemRef::new(
                zero.object().guid(),
                INVENTORY_SLOT_BAG_0,
                EQUIPMENT_SLOT_BACK,
                0,
                false,
            ),
        ],
    );

    assert_eq!(
        actions,
        vec![
            RemoveArenaEnchantmentAction::ClearEquippedEnchantment {
                item_guid: blocked.object().guid(),
                enchantment_slot: EnchantmentSlot::EnhancementTemporary,
            },
            RemoveArenaEnchantmentAction::RemoveDurationReference {
                item_guid: zero.object().guid(),
                enchantment_slot: EnchantmentSlot::EnhancementTemporary,
            },
        ]
    );
    assert_eq!(
        player.enchant_durations(),
        &[
            PlayerEnchantDuration {
                item_guid: allowed.object().guid(),
                slot: EnchantmentSlot::EnhancementTemporary,
                left_duration_ms: 10_000,
            },
            PlayerEnchantDuration {
                item_guid: blocked.object().guid(),
                slot: EnchantmentSlot::EnhancementPermanent,
                left_duration_ms: 1_000,
            },
        ]
    );
}

#[test]
fn remove_arena_enchantments_scans_inventory_and_bags_like_cpp() {
    let mut player = Player::new(None, false);
    player.set_inventory_slot_count(2);
    let allowed = item_with_guid_entry(1260, 7600);
    let blocked = item_with_guid_entry(1261, 7601);
    let missing_ref = item_with_guid_entry(1262, 7602);
    let bag = ObjectGuid::create_item(1, 1263);
    let bag_blocked = item_with_guid_entry(1264, 7604);

    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, allowed.object().guid())
        .unwrap();
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START + 1, blocked.object().guid())
        .unwrap();
    player
        .store_top_level_item(INVENTORY_SLOT_BAG_START, bag)
        .unwrap();
    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag, 3)
        .unwrap();
    player
        .store_bag_item(INVENTORY_SLOT_BAG_START, 0, bag_blocked.object().guid())
        .unwrap();
    player
        .store_bag_item(INVENTORY_SLOT_BAG_START, 1, missing_ref.object().guid())
        .unwrap();

    let actions = player.remove_arena_enchantments(
        EnchantmentSlot::EnhancementTemporary,
        &[
            ArenaEnchantmentItemRef::new(
                allowed.object().guid(),
                INVENTORY_SLOT_BAG_0,
                INVENTORY_SLOT_ITEM_START,
                100,
                true,
            ),
            ArenaEnchantmentItemRef::new(
                blocked.object().guid(),
                INVENTORY_SLOT_BAG_0,
                INVENTORY_SLOT_ITEM_START + 1,
                200,
                false,
            ),
            ArenaEnchantmentItemRef::new(
                bag_blocked.object().guid(),
                INVENTORY_SLOT_BAG_START,
                0,
                300,
                false,
            ),
        ],
    );

    assert_eq!(
        actions,
        vec![
            RemoveArenaEnchantmentAction::ClearInventoryEnchantment {
                item_guid: blocked.object().guid(),
                bag: INVENTORY_SLOT_BAG_0,
                slot: INVENTORY_SLOT_ITEM_START + 1,
                enchantment_slot: EnchantmentSlot::EnhancementTemporary,
            },
            RemoveArenaEnchantmentAction::ClearInventoryEnchantment {
                item_guid: bag_blocked.object().guid(),
                bag: INVENTORY_SLOT_BAG_START,
                slot: 0,
                enchantment_slot: EnchantmentSlot::EnhancementTemporary,
            },
            RemoveArenaEnchantmentAction::MissingInventoryItemRef {
                item_guid: missing_ref.object().guid(),
                bag: INVENTORY_SLOT_BAG_START,
                slot: 1,
                enchantment_slot: EnchantmentSlot::EnhancementTemporary,
            },
        ]
    );
}

#[test]
fn titan_grip_and_equipped_weapon_helpers_match_cpp_representable_rules() {
    let mut player = Player::new(None, false);
    let two_hand = ItemStorageTemplate {
        inventory_type: InventoryType::Weapon2Hand,
        class_id: ItemClass::Weapon,
        ..ItemStorageTemplate::regular_item(2000, 1)
    };
    let one_hand = ItemStorageTemplate {
        inventory_type: InventoryType::Weapon,
        class_id: ItemClass::Weapon,
        ..ItemStorageTemplate::regular_item(2001, 1)
    };
    let ranged = ItemStorageTemplate {
        inventory_type: InventoryType::Ranged,
        class_id: ItemClass::Weapon,
        ..ItemStorageTemplate::regular_item(2002, 1)
    };
    let ranged_right_non_wand = ItemStorageTemplate {
        inventory_type: InventoryType::RangedRight,
        class_id: ItemClass::Weapon,
        subclass_id: ItemSubClassWeapon::Bow as u32,
        ..ItemStorageTemplate::regular_item(2003, 1)
    };
    let wand = ItemStorageTemplate {
        inventory_type: InventoryType::RangedRight,
        class_id: ItemClass::Weapon,
        subclass_id: ItemSubClassWeapon::Wand as u32,
        ..ItemStorageTemplate::regular_item(2004, 1)
    };

    assert!(Player::is_use_equipped_weapon(false, false, true));
    assert!(!Player::is_use_equipped_weapon(true, false, true));
    assert!(!Player::is_use_equipped_weapon(false, true, false));

    assert!(!player.can_titan_grip());
    assert_eq!(player.titan_grip_penalty_spell_id(), 0);
    assert!(player.is_two_hand_used_template(Some(&two_hand)));
    assert!(player.is_two_hand_used_template(Some(&ranged)));
    assert!(player.is_two_hand_used_template(Some(&ranged_right_non_wand)));
    assert!(!player.is_two_hand_used_template(Some(&wand)));
    assert!(!player.is_two_hand_used_template(None));

    player.set_can_titan_grip(true, 49152);
    player.set_can_titan_grip(true, 99999);
    assert!(player.can_titan_grip());
    assert_eq!(player.titan_grip_penalty_spell_id(), 49152);
    assert!(!player.is_two_hand_used_template(Some(&two_hand)));

    assert!(Player::is_using_two_handed_weapon_in_one_hand_template(
        Some(&one_hand),
        Some(&two_hand),
    ));
    assert!(Player::is_using_two_handed_weapon_in_one_hand_template(
        Some(&two_hand),
        Some(&one_hand),
    ));
    assert!(!Player::is_using_two_handed_weapon_in_one_hand_template(
        Some(&two_hand),
        None,
    ));
    assert!(!Player::is_using_two_handed_weapon_in_one_hand_template(
        Some(&one_hand),
        Some(&one_hand),
    ));

    assert_eq!(
        player.check_titan_grip_penalty_action(true, false),
        TitanGripPenaltyAction::Cast(49152)
    );
    assert_eq!(
        player.check_titan_grip_penalty_action(true, true),
        TitanGripPenaltyAction::None
    );
    assert_eq!(
        player.check_titan_grip_penalty_action(false, true),
        TitanGripPenaltyAction::Remove(49152)
    );

    player.set_can_titan_grip(false, 0);
    assert_eq!(
        player.check_titan_grip_penalty_action(true, false),
        TitanGripPenaltyAction::None
    );
}

#[test]
fn swap_item_preflight_matches_cpp_no_source_child_and_dead_order() {
    let player = Player::new(None, false);
    let src = make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST);
    let dst = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START);
    let parent = make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_HEAD);

    assert_eq!(
        player.swap_item_preflight_plan(src, dst, true, None, None),
        SwapItemPreflightPlan {
            result: SwapItemPreflightResult::NoSource,
            src_unequip_swap: None,
            dst_unequip_swap: None,
        }
    );

    let mut child_source = SwapItemPreflightItem::regular();
    child_source.is_child = true;
    child_source.parent_pos = Some(parent);
    assert_eq!(
        player.swap_item_preflight_plan(src, dst, false, Some(child_source), None),
        SwapItemPreflightPlan {
            result: SwapItemPreflightResult::ChildRedirect {
                first_src: dst,
                first_dst: src,
                second_src: parent,
                second_dst: dst,
            },
            src_unequip_swap: None,
            dst_unequip_swap: None,
        }
    );

    let mut child_dst = SwapItemPreflightItem::regular();
    child_dst.is_child = true;
    child_dst.parent_pos = Some(parent);
    assert_eq!(
        player.swap_item_preflight_plan(
            dst,
            src,
            true,
            Some(SwapItemPreflightItem::regular()),
            Some(child_dst)
        ),
        SwapItemPreflightPlan {
            result: SwapItemPreflightResult::ChildRedirect {
                first_src: dst,
                first_dst: src,
                second_src: parent,
                second_dst: dst,
            },
            src_unequip_swap: None,
            dst_unequip_swap: None,
        }
    );

    let mut blocked_source = SwapItemPreflightItem::regular();
    blocked_source.can_unequip_result = InventoryResult::CantEquipEver;
    assert_eq!(
        player.swap_item_preflight_plan(src, dst, false, Some(blocked_source), None),
        SwapItemPreflightPlan {
            result: SwapItemPreflightResult::Error(InventoryResult::PlayerDead),
            src_unequip_swap: None,
            dst_unequip_swap: None,
        }
    );
}

#[test]
fn swap_item_preflight_matches_cpp_unequip_and_bag_self_guards() {
    let player = Player::new(None, false);
    let equipped_src = make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST);
    let inventory_dst = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START);
    let source = SwapItemPreflightItem::regular();

    assert_eq!(
        player.swap_item_preflight_plan(equipped_src, inventory_dst, true, Some(source), None),
        SwapItemPreflightPlan {
            result: SwapItemPreflightResult::Continue,
            src_unequip_swap: Some(true),
            dst_unequip_swap: None,
        }
    );

    let mut blocked_source = SwapItemPreflightItem::regular();
    blocked_source.can_unequip_result = InventoryResult::ClientLockedOut;
    assert_eq!(
        player.swap_item_preflight_plan(
            equipped_src,
            inventory_dst,
            true,
            Some(blocked_source),
            None
        ),
        SwapItemPreflightPlan {
            result: SwapItemPreflightResult::Error(InventoryResult::ClientLockedOut),
            src_unequip_swap: Some(true),
            dst_unequip_swap: None,
        }
    );

    let bag_slot = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START);
    let inside_same_bag = make_item_pos(INVENTORY_SLOT_BAG_START, 0);
    assert_eq!(
        player.swap_item_preflight_plan(
            bag_slot,
            inside_same_bag,
            true,
            Some(SwapItemPreflightItem::bag(false)),
            None,
        ),
        SwapItemPreflightPlan {
            result: SwapItemPreflightResult::Error(InventoryResult::BagInBag),
            src_unequip_swap: Some(false),
            dst_unequip_swap: None,
        }
    );
    assert_eq!(
        player.swap_item_preflight_plan(
            inside_same_bag,
            bag_slot,
            true,
            Some(SwapItemPreflightItem::regular()),
            Some(SwapItemPreflightItem::bag(false)),
        ),
        SwapItemPreflightPlan {
            result: SwapItemPreflightResult::Error(InventoryResult::CantSwap),
            src_unequip_swap: None,
            dst_unequip_swap: None,
        }
    );

    let mut blocked_dst = SwapItemPreflightItem::bag(true);
    blocked_dst.can_unequip_result = InventoryResult::CantEquipEver;
    let other_bag_slot = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START + 1);
    assert_eq!(
        player.swap_item_preflight_plan(
            inventory_dst,
            other_bag_slot,
            true,
            Some(SwapItemPreflightItem::bag(true)),
            Some(blocked_dst),
        ),
        SwapItemPreflightPlan {
            result: SwapItemPreflightResult::Error(InventoryResult::CantEquipEver),
            src_unequip_swap: None,
            dst_unequip_swap: Some(true),
        }
    );
}

#[test]
fn swap_item_empty_destination_plan_matches_cpp_move_case() {
    let player = Player::new(None, false);
    let inventory_src = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START);
    let inventory_dst = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1);
    let bank_src = make_item_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START);
    let bank_dst = make_item_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START + 1);
    let equip_dst = make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST);
    let equip_dest = make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST);

    assert_eq!(
        player.swap_item_empty_destination_plan(
            inventory_src,
            inventory_dst,
            true,
            InventoryResult::Ok,
            InventoryResult::Ok,
            InventoryResult::Ok,
            equip_dest,
        ),
        SwapItemEmptyDestinationPlan {
            result: SwapItemEmptyDestinationResult::OccupiedDestination,
        }
    );

    assert_eq!(
        player.swap_item_empty_destination_plan(
            bank_src,
            inventory_dst,
            false,
            InventoryResult::Ok,
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            equip_dest,
        ),
        SwapItemEmptyDestinationPlan {
            result: SwapItemEmptyDestinationResult::MoveToInventory {
                quest_added_from_bank: true,
            },
        }
    );

    assert_eq!(
        player.swap_item_empty_destination_plan(
            inventory_src,
            inventory_dst,
            false,
            InventoryResult::InvFull,
            InventoryResult::Ok,
            InventoryResult::Ok,
            equip_dest,
        ),
        SwapItemEmptyDestinationPlan {
            result: SwapItemEmptyDestinationResult::Error(InventoryResult::InvFull),
        }
    );

    assert_eq!(
        player.swap_item_empty_destination_plan(
            inventory_src,
            bank_dst,
            false,
            InventoryResult::CantSwap,
            InventoryResult::Ok,
            InventoryResult::CantSwap,
            equip_dest,
        ),
        SwapItemEmptyDestinationPlan {
            result: SwapItemEmptyDestinationResult::MoveToBank {
                quest_removed: true,
            },
        }
    );

    assert_eq!(
        player.swap_item_empty_destination_plan(
            inventory_src,
            equip_dst,
            false,
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            InventoryResult::Ok,
            equip_dest,
        ),
        SwapItemEmptyDestinationPlan {
            result: SwapItemEmptyDestinationResult::Equip {
                dest: equip_dest,
                auto_unequip_offhand: true,
            },
        }
    );

    assert_eq!(
        player.swap_item_empty_destination_plan(
            inventory_src,
            make_item_pos(BUYBACK_SLOT_START, 0),
            false,
            InventoryResult::Ok,
            InventoryResult::Ok,
            InventoryResult::Ok,
            equip_dest,
        ),
        SwapItemEmptyDestinationPlan {
            result: SwapItemEmptyDestinationResult::InvalidDestinationNoop,
        }
    );
}

#[test]
fn swap_item_merge_fill_plan_matches_cpp_occupied_non_bag_case() {
    let player = Player::new(None, false);
    let inventory_dst = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START);
    let bank_dst = make_item_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START);
    let equip_dst = make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST);
    let equip_dest = make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST);

    assert_eq!(
        player.swap_item_merge_fill_plan(
            inventory_dst,
            true,
            false,
            3,
            4,
            20,
            InventoryResult::Ok,
            InventoryResult::Ok,
            InventoryResult::Ok,
            equip_dest,
            true,
        ),
        SwapItemMergeFillPlan {
            result: SwapItemMergeFillResult::ContinueToRealSwap,
            send_refund_info: false,
        }
    );

    assert_eq!(
        player.swap_item_merge_fill_plan(
            inventory_dst,
            false,
            false,
            3,
            4,
            20,
            InventoryResult::CantSwap,
            InventoryResult::Ok,
            InventoryResult::Ok,
            equip_dest,
            true,
        ),
        SwapItemMergeFillPlan {
            result: SwapItemMergeFillResult::ContinueToRealSwap,
            send_refund_info: false,
        }
    );

    assert_eq!(
        player.swap_item_merge_fill_plan(
            inventory_dst,
            false,
            false,
            3,
            4,
            20,
            InventoryResult::Ok,
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            equip_dest,
            true,
        ),
        SwapItemMergeFillPlan {
            result: SwapItemMergeFillResult::MoveMergedStackToInventory,
            send_refund_info: true,
        }
    );

    assert_eq!(
        player.swap_item_merge_fill_plan(
            bank_dst,
            false,
            false,
            3,
            4,
            20,
            InventoryResult::CantSwap,
            InventoryResult::Ok,
            InventoryResult::CantSwap,
            equip_dest,
            true,
        ),
        SwapItemMergeFillPlan {
            result: SwapItemMergeFillResult::MoveMergedStackToBank,
            send_refund_info: true,
        }
    );

    assert_eq!(
        player.swap_item_merge_fill_plan(
            equip_dst,
            false,
            false,
            3,
            4,
            20,
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            InventoryResult::Ok,
            equip_dest,
            true,
        ),
        SwapItemMergeFillPlan {
            result: SwapItemMergeFillResult::EquipMergedStack {
                dest: equip_dest,
                auto_unequip_offhand: true,
            },
            send_refund_info: true,
        }
    );

    assert_eq!(
        player.swap_item_merge_fill_plan(
            inventory_dst,
            false,
            false,
            15,
            12,
            20,
            InventoryResult::Ok,
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            equip_dest,
            true,
        ),
        SwapItemMergeFillPlan {
            result: SwapItemMergeFillResult::PartialFill {
                source_remaining_count: 7,
                destination_count: 20,
                send_updates: true,
            },
            send_refund_info: true,
        }
    );
}

#[test]
fn swap_item_real_swap_validation_plan_matches_cpp_bidirectional_checks() {
    let player = Player::new(None, false);
    let inventory_src = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START);
    let bank_dst = make_item_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START);
    let equip_src = make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST);
    let equip_dst = make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_LEGS);
    let equip_dest = make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_LEGS);
    let equip_dest2 = make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST);

    assert_eq!(
        player.swap_item_real_swap_validation_plan(
            inventory_src,
            bank_dst,
            InventoryResult::CantSwap,
            InventoryResult::Ok,
            InventoryResult::CantSwap,
            equip_dest,
            InventoryResult::Ok,
            InventoryResult::Ok,
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            equip_dest2,
            InventoryResult::Ok,
        ),
        SwapItemRealSwapValidationPlan {
            result: SwapItemRealSwapValidationResult::Continue {
                source_target: SwapItemRealSwapTarget::Bank,
                destination_target: SwapItemRealSwapTarget::Inventory,
            },
        }
    );

    assert_eq!(
        player.swap_item_real_swap_validation_plan(
            inventory_src,
            bank_dst,
            InventoryResult::CantSwap,
            InventoryResult::InvFull,
            InventoryResult::CantSwap,
            equip_dest,
            InventoryResult::Ok,
            InventoryResult::Ok,
            InventoryResult::Ok,
            InventoryResult::Ok,
            equip_dest2,
            InventoryResult::Ok,
        ),
        SwapItemRealSwapValidationPlan {
            result: SwapItemRealSwapValidationResult::Error {
                result: InventoryResult::InvFull,
                subject: SwapItemRealSwapValidationSubject::Source,
            },
        }
    );

    assert_eq!(
        player.swap_item_real_swap_validation_plan(
            inventory_src,
            bank_dst,
            InventoryResult::CantSwap,
            InventoryResult::Ok,
            InventoryResult::CantSwap,
            equip_dest,
            InventoryResult::Ok,
            InventoryResult::ClientLockedOut,
            InventoryResult::Ok,
            InventoryResult::Ok,
            equip_dest2,
            InventoryResult::Ok,
        ),
        SwapItemRealSwapValidationPlan {
            result: SwapItemRealSwapValidationResult::Error {
                result: InventoryResult::ClientLockedOut,
                subject: SwapItemRealSwapValidationSubject::Destination,
            },
        }
    );

    assert_eq!(
        player.swap_item_real_swap_validation_plan(
            equip_src,
            equip_dst,
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            InventoryResult::Ok,
            equip_dest,
            InventoryResult::DestroyNonemptyBag,
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            InventoryResult::Ok,
            equip_dest2,
            InventoryResult::Ok,
        ),
        SwapItemRealSwapValidationPlan {
            result: SwapItemRealSwapValidationResult::Error {
                result: InventoryResult::DestroyNonemptyBag,
                subject: SwapItemRealSwapValidationSubject::Source,
            },
        }
    );

    assert_eq!(
        player.swap_item_real_swap_validation_plan(
            make_item_pos(BUYBACK_SLOT_START, 0),
            make_item_pos(BUYBACK_SLOT_START + 1, 0),
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            equip_dest,
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            equip_dest2,
            InventoryResult::CantSwap,
        ),
        SwapItemRealSwapValidationPlan {
            result: SwapItemRealSwapValidationResult::Continue {
                source_target: SwapItemRealSwapTarget::None,
                destination_target: SwapItemRealSwapTarget::None,
            },
        }
    );
}

#[test]
fn swap_item_bag_exchange_plan_matches_cpp_empty_bag_exchange() {
    let player = Player::new(None, false);
    let inventory_src = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START);
    let inventory_dst = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1);
    let bag_slot_src = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START);
    let full_items = [
        SwapBagItemRef::new(0, true),
        SwapBagItemRef::new(2, true),
        SwapBagItemRef::new(4, true),
    ];
    let full_bag = SwapBagRef::new(false, 5, &full_items);
    let empty_bag = SwapBagRef::new(true, 4, &[]);

    assert_eq!(
        player.swap_item_bag_exchange_plan(inventory_src, inventory_dst, None, Some(full_bag)),
        SwapItemBagExchangePlan {
            result: SwapItemBagExchangeResult::Continue,
        }
    );

    assert_eq!(
        player.swap_item_bag_exchange_plan(
            inventory_src,
            inventory_dst,
            Some(empty_bag),
            Some(full_bag),
        ),
        SwapItemBagExchangePlan {
            result: SwapItemBagExchangeResult::Exchange {
                empty_bag_is_source: true,
                moves: vec![
                    SwapBagItemMove {
                        from_slot: 0,
                        to_slot: 0,
                    },
                    SwapBagItemMove {
                        from_slot: 2,
                        to_slot: 1,
                    },
                    SwapBagItemMove {
                        from_slot: 4,
                        to_slot: 2,
                    },
                ],
            },
        }
    );

    assert_eq!(
        player.swap_item_bag_exchange_plan(
            inventory_src,
            inventory_dst,
            Some(full_bag),
            Some(empty_bag),
        ),
        SwapItemBagExchangePlan {
            result: SwapItemBagExchangeResult::Exchange {
                empty_bag_is_source: false,
                moves: vec![
                    SwapBagItemMove {
                        from_slot: 0,
                        to_slot: 0,
                    },
                    SwapBagItemMove {
                        from_slot: 2,
                        to_slot: 1,
                    },
                    SwapBagItemMove {
                        from_slot: 4,
                        to_slot: 2,
                    },
                ],
            },
        }
    );

    assert_eq!(
        player.swap_item_bag_exchange_plan(
            bag_slot_src,
            inventory_dst,
            Some(empty_bag),
            Some(full_bag),
        ),
        SwapItemBagExchangePlan {
            result: SwapItemBagExchangeResult::Continue,
        }
    );

    let blocked_items = [SwapBagItemRef::new(0, true), SwapBagItemRef::new(1, false)];
    let blocked_bag = SwapBagRef::new(false, 2, &blocked_items);
    assert_eq!(
        player.swap_item_bag_exchange_plan(
            inventory_src,
            inventory_dst,
            Some(empty_bag),
            Some(blocked_bag),
        ),
        SwapItemBagExchangePlan {
            result: SwapItemBagExchangeResult::Error(InventoryResult::BagInBag),
        }
    );

    let small_empty_bag = SwapBagRef::new(true, 2, &[]);
    assert_eq!(
        player.swap_item_bag_exchange_plan(
            inventory_src,
            inventory_dst,
            Some(small_empty_bag),
            Some(full_bag),
        ),
        SwapItemBagExchangePlan {
            result: SwapItemBagExchangeResult::Error(InventoryResult::CantSwap),
        }
    );
}

#[test]
fn swap_item_real_swap_execution_plan_matches_cpp_final_actions() {
    let player = Player::new(None, false);
    let inventory_src = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START);
    let equip_dst = make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST);

    assert_eq!(
        player.swap_item_real_swap_execution_plan(
            inventory_src,
            equip_dst,
            SwapItemRealSwapTarget::Equip { dest: equip_dst },
            SwapItemRealSwapTarget::Inventory,
            false,
            false,
            false,
        ),
        SwapItemRealSwapExecutionPlan {
            remove_destination_update: false,
            remove_source_update: false,
            source_target: SwapItemRealSwapTarget::Equip { dest: equip_dst },
            destination_target: SwapItemRealSwapTarget::Inventory,
            apply_item_dependent_auras: true,
            release_loot: false,
            auto_unequip_offhand: true,
        }
    );

    let bag_src = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START);
    let bank_dst = make_item_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START);
    assert!(
        player
            .swap_item_real_swap_execution_plan(
                bag_src,
                bank_dst,
                SwapItemRealSwapTarget::Bank,
                SwapItemRealSwapTarget::Inventory,
                true,
                true,
                false,
            )
            .release_loot
    );

    let bag_dst = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START + 1);
    assert!(
        player
            .swap_item_real_swap_execution_plan(
                bank_dst,
                bag_dst,
                SwapItemRealSwapTarget::Inventory,
                SwapItemRealSwapTarget::Bank,
                true,
                false,
                true,
            )
            .release_loot
    );
    assert!(
        !player
            .swap_item_real_swap_execution_plan(
                bank_dst,
                bag_dst,
                SwapItemRealSwapTarget::Inventory,
                SwapItemRealSwapTarget::Bank,
                false,
                false,
                true,
            )
            .release_loot
    );
}

#[test]
fn swap_item_orchestration_plan_matches_cpp_branch_order() {
    let player = Player::new(None, false);
    let continue_preflight = SwapItemPreflightPlan {
        result: SwapItemPreflightResult::Continue,
        src_unequip_swap: None,
        dst_unequip_swap: None,
    };
    let occupied_destination = SwapItemEmptyDestinationPlan {
        result: SwapItemEmptyDestinationResult::OccupiedDestination,
    };
    let continue_merge = SwapItemMergeFillPlan {
        result: SwapItemMergeFillResult::ContinueToRealSwap,
        send_refund_info: false,
    };
    let inventory_bank_validation = SwapItemRealSwapValidationPlan {
        result: SwapItemRealSwapValidationResult::Continue {
            source_target: SwapItemRealSwapTarget::Inventory,
            destination_target: SwapItemRealSwapTarget::Bank,
        },
    };
    let no_bag_exchange = SwapItemBagExchangePlan {
        result: SwapItemBagExchangeResult::Continue,
    };
    let execution = SwapItemRealSwapExecutionPlan {
        remove_destination_update: false,
        remove_source_update: false,
        source_target: SwapItemRealSwapTarget::Inventory,
        destination_target: SwapItemRealSwapTarget::Bank,
        apply_item_dependent_auras: false,
        release_loot: false,
        auto_unequip_offhand: true,
    };

    assert_eq!(
        player.swap_item_orchestration_plan(
            SwapItemPreflightPlan {
                result: SwapItemPreflightResult::Error(InventoryResult::PlayerDead),
                src_unequip_swap: None,
                dst_unequip_swap: None,
            },
            None,
            None,
            None,
            None,
            None,
        ),
        SwapItemOrchestrationPlan {
            result: SwapItemOrchestrationResult::Error {
                result: InventoryResult::PlayerDead,
                item_order: SwapItemErrorItemOrder::SourceDestination,
            },
        }
    );

    assert_eq!(
        player.swap_item_orchestration_plan(
            continue_preflight,
            Some(SwapItemEmptyDestinationPlan {
                result: SwapItemEmptyDestinationResult::Error(InventoryResult::InvFull),
            }),
            None,
            None,
            None,
            None,
        ),
        SwapItemOrchestrationPlan {
            result: SwapItemOrchestrationResult::Error {
                result: InventoryResult::InvFull,
                item_order: SwapItemErrorItemOrder::SourceOnly,
            },
        }
    );

    let move_to_bank = SwapItemEmptyDestinationPlan {
        result: SwapItemEmptyDestinationResult::MoveToBank {
            quest_removed: true,
        },
    };
    assert_eq!(
        player.swap_item_orchestration_plan(
            continue_preflight,
            Some(move_to_bank),
            None,
            None,
            None,
            None,
        ),
        SwapItemOrchestrationPlan {
            result: SwapItemOrchestrationResult::EmptyDestination(move_to_bank),
        }
    );

    let partial_fill = SwapItemMergeFillPlan {
        result: SwapItemMergeFillResult::PartialFill {
            source_remaining_count: 2,
            destination_count: 20,
            send_updates: true,
        },
        send_refund_info: true,
    };
    assert_eq!(
        player.swap_item_orchestration_plan(
            continue_preflight,
            Some(occupied_destination),
            Some(partial_fill),
            None,
            None,
            None,
        ),
        SwapItemOrchestrationPlan {
            result: SwapItemOrchestrationResult::MergeFill(partial_fill),
        }
    );

    assert_eq!(
        player.swap_item_orchestration_plan(
            continue_preflight,
            Some(occupied_destination),
            Some(continue_merge),
            Some(SwapItemRealSwapValidationPlan {
                result: SwapItemRealSwapValidationResult::Error {
                    result: InventoryResult::CantEquipEver,
                    subject: SwapItemRealSwapValidationSubject::Destination,
                },
            }),
            None,
            None,
        ),
        SwapItemOrchestrationPlan {
            result: SwapItemOrchestrationResult::Error {
                result: InventoryResult::CantEquipEver,
                item_order: SwapItemErrorItemOrder::DestinationSource,
            },
        }
    );

    assert_eq!(
        player.swap_item_orchestration_plan(
            continue_preflight,
            Some(occupied_destination),
            Some(continue_merge),
            Some(inventory_bank_validation),
            Some(SwapItemBagExchangePlan {
                result: SwapItemBagExchangeResult::Error(InventoryResult::BagInBag),
            }),
            None,
        ),
        SwapItemOrchestrationPlan {
            result: SwapItemOrchestrationResult::Error {
                result: InventoryResult::BagInBag,
                item_order: SwapItemErrorItemOrder::SourceDestination,
            },
        }
    );

    assert_eq!(
        player.swap_item_orchestration_plan(
            continue_preflight,
            Some(occupied_destination),
            Some(continue_merge),
            Some(inventory_bank_validation),
            Some(no_bag_exchange.clone()),
            Some(execution),
        ),
        SwapItemOrchestrationPlan {
            result: SwapItemOrchestrationResult::RealSwap {
                bag_exchange: no_bag_exchange,
                execution,
            },
        }
    );
}

#[test]
fn swap_item_orchestration_plan_keeps_phase_gaps_visible() {
    let player = Player::new(None, false);
    let continue_preflight = SwapItemPreflightPlan {
        result: SwapItemPreflightResult::Continue,
        src_unequip_swap: None,
        dst_unequip_swap: None,
    };

    assert_eq!(
        player.swap_item_orchestration_plan(continue_preflight, None, None, None, None, None),
        SwapItemOrchestrationPlan {
            result: SwapItemOrchestrationResult::MissingPhase(
                SwapItemMissingPhase::EmptyDestination,
            ),
        }
    );

    let occupied_destination = SwapItemEmptyDestinationPlan {
        result: SwapItemEmptyDestinationResult::OccupiedDestination,
    };
    assert_eq!(
        player.swap_item_orchestration_plan(
            continue_preflight,
            Some(occupied_destination),
            None,
            None,
            None,
            None,
        ),
        SwapItemOrchestrationPlan {
            result: SwapItemOrchestrationResult::MissingPhase(SwapItemMissingPhase::MergeFill),
        }
    );

    let continue_merge = SwapItemMergeFillPlan {
        result: SwapItemMergeFillResult::ContinueToRealSwap,
        send_refund_info: false,
    };
    let validation = SwapItemRealSwapValidationPlan {
        result: SwapItemRealSwapValidationResult::Continue {
            source_target: SwapItemRealSwapTarget::Inventory,
            destination_target: SwapItemRealSwapTarget::Bank,
        },
    };
    let mismatched_execution = SwapItemRealSwapExecutionPlan {
        remove_destination_update: false,
        remove_source_update: false,
        source_target: SwapItemRealSwapTarget::Bank,
        destination_target: SwapItemRealSwapTarget::Inventory,
        apply_item_dependent_auras: false,
        release_loot: false,
        auto_unequip_offhand: true,
    };

    assert_eq!(
        player.swap_item_orchestration_plan(
            continue_preflight,
            Some(occupied_destination),
            Some(continue_merge),
            Some(validation),
            Some(SwapItemBagExchangePlan {
                result: SwapItemBagExchangeResult::Continue,
            }),
            Some(mismatched_execution),
        ),
        SwapItemOrchestrationPlan {
            result: SwapItemOrchestrationResult::InconsistentRealSwapTargets {
                validation_source_target: SwapItemRealSwapTarget::Inventory,
                validation_destination_target: SwapItemRealSwapTarget::Bank,
                execution_source_target: SwapItemRealSwapTarget::Bank,
                execution_destination_target: SwapItemRealSwapTarget::Inventory,
            },
        }
    );
}
