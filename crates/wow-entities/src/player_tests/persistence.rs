//! Persistence scenarios for [`super`].
//!
//! Split out of player_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

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
    assert_eq!(player.gameplay_state().menu, state.menu);
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
