//! Spell scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn before_add_spell_packets_keep_cpp_order_without_name_query_injection() {
    let (mut session, send_rx) = make_session_with_send_capacity(64);
    let guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(guid));

    assert!(
        session
            .send_initial_packets_before_add_to_map(
                guid,
                &Position::ZERO,
                571,
                0,
                CharacterLoginLocationLikeCpp {
                    map_id: 571,
                    bind_area_id: Some(0),
                    position: Position::ZERO,
                },
                vec![123],
                Vec::new(),
                Vec::new(),
                Vec::new(),
                [0; 180],
                Vec::new(),
                false,
            )
            .await
    );

    let opcodes = drain_server_opcodes(&send_rx);
    assert!(
        !opcodes.contains(&ServerOpcodes::QueryPlayerNamesResponse),
        "C++ ContactList serialization does not synchronously publish name-query results"
    );
    let expected = [
        ServerOpcodes::ContactList,
        ServerOpcodes::BindPointUpdate,
        ServerOpcodes::UpdateTalentData,
        ServerOpcodes::SendKnownSpells,
        ServerOpcodes::SendUnlearnSpells,
        ServerOpcodes::SendSpellHistory,
        ServerOpcodes::SendSpellCharges,
        ServerOpcodes::ActiveGlyphs,
    ];
    let positions = expected.map(|opcode| {
        opcodes
            .iter()
            .position(|candidate| *candidate == opcode)
            .unwrap_or_else(|| panic!("missing {opcode:?}"))
    });
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "C++ orders ContactList -> talents -> known/unlearn/history/charges -> ActiveGlyphs"
    );
}
#[test]
fn skill_rewarded_login_changes_use_real_spell_levels_and_conditions_like_cpp() {
    fn ability(
        id: u32,
        spell: i32,
        acquire_method: i8,
        min_skill_line_rank: i16,
        flags: i8,
    ) -> wow_data::SkillLineAbilityRecord {
        wow_data::SkillLineAbilityRecord {
            id,
            race_mask: 1,
            skill_line: 164,
            spell,
            min_skill_line_rank,
            class_mask: 1,
            supercedes_spell: 0,
            acquire_method,
            trivial_rank_high: 0,
            trivial_rank_low: 0,
            flags,
            num_skill_ups: 0,
            skillup_skill_line_id: 0,
        }
    }

    fn spell_info(spell_id: i32) -> wow_data::SpellInfo {
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

    let (mut session, _) = make_session_with_send_capacity(1);
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 10, 0);
    session.set_skill_store(Arc::new(
        wow_data::SkillStore::from_skill_line_abilities_like_cpp([
            ability(
                1,
                900,
                wow_data::skill::SKILL_LINE_ABILITY_LEARNED_ON_SKILL_VALUE_LIKE_CPP,
                50,
                0,
            ),
            ability(
                2,
                901,
                wow_data::skill::SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
                0,
                0,
            ),
            ability(
                3,
                902,
                wow_data::skill::SKILL_LINE_ABILITY_REWARDED_FROM_QUEST_LIKE_CPP,
                0,
                wow_data::skill::SKILL_LINE_ABILITY_CAN_FALLBACK_TO_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
            ),
            ability(
                4,
                903,
                wow_data::skill::SKILL_LINE_ABILITY_LEARNED_ON_SKILL_LEARN_LIKE_CPP,
                0,
                0,
            ),
        ]),
    ));

    let mut spell_store = wow_data::SpellStore::new();
    for spell_id in [900, 901, 902] {
        spell_store.insert(spell_id, spell_info(spell_id));
    }
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_levels_store(Arc::new(wow_data::SpellLevelsStore::from_entries([
        wow_data::SpellLevelsEntry {
            id: 1,
            difficulty_id: 0,
            base_level: 1,
            max_level: 0,
            spell_level: 1,
            max_passive_aura_level: 0,
            spell_id: 900,
        },
        wow_data::SpellLevelsEntry {
            id: 2,
            difficulty_id: 0,
            base_level: 20,
            max_level: 0,
            spell_level: 1,
            max_passive_aura_level: 0,
            spell_id: 901,
        },
        wow_data::SpellLevelsEntry {
            id: 3,
            difficulty_id: 0,
            base_level: 1,
            max_level: 0,
            spell_level: 1,
            max_passive_aura_level: 0,
            spell_id: 902,
        },
        wow_data::SpellLevelsEntry {
            id: 4,
            difficulty_id: 0,
            base_level: 1,
            max_level: 0,
            spell_level: 1,
            max_passive_aura_level: 0,
            spell_id: 903,
        },
    ])));
    session.set_spell_misc_store(Arc::new(SpellMiscStore::from_entries([SpellMiscEntry {
        id: 1,
        spell_id: 902,
        show_future_spell_player_condition_id: 77,
        ..SpellMiscEntry::default()
    }])));
    session.set_player_condition_store(Arc::new(wow_data::PlayerConditionStore::from_entries([
        PlayerConditionEntry {
            id: 77,
            class_mask: 1,
            ..PlayerConditionEntry::default()
        },
    ])));

    let changes = session.skill_rewarded_spell_changes_for_login_like_cpp(164, 40, 1, 1, 10);

    assert_eq!(
        changes.remove,
        vec![900],
        "C++ removes an OnSkillValue spell while the skill is below its required rank"
    );
    assert_eq!(
        changes.learn,
        vec![902],
        "the level-gated spell and the ability without real SpellInfo must be skipped"
    );
}
#[test]
fn login_passive_total_stat_aura_defers_values_update_until_create_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let player_guid = ObjectGuid::create_player(1, 84);
    let spell_id = 90_085;
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1_000, 40);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 777, 1_320, 5, 10);
    session.set_known_spells_like_cpp(vec![spell_id]);
    let mut spell_store = total_stat_percentage_spell_store_like_cpp(spell_id, false);
    let mut attributes = [0; 15];
    attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
    spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    session.set_spell_store(Arc::new(spell_store));

    assert_eq!(session.state(), crate::session::SessionState::Authed);
    assert_eq!(session.apply_login_passive_known_spell_auras_like_cpp(), 1);
    assert_eq!(
        session.represented_total_stat_multipliers_like_cpp(),
        [1.0, 1.0, 2.0, 1.0, 1.0]
    );
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(
        !opcodes.contains(&ServerOpcodes::UpdateObject),
        "C++ folds login passive modifiers into UpdateAllStats/CreateObject instead of sending pre-create VALUES"
    );
}
#[test]
fn login_combat_snapshot_clamps_saved_health_after_persisted_stat_auras_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 85);
    let spell_id = 90_086;
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1_000, 40);
    session.set_spell_store(Arc::new(total_stat_percentage_spell_store_like_cpp(
        spell_id, false,
    )));

    assert_eq!(
        session.load_represented_character_auras_like_cpp(
            [crate::session::CharacterAuraRowLikeCpp {
                caster_guid: player_guid,
                spell_id: spell_id as u32,
                effect_mask: 1,
                recalculate_mask: 0,
                difficulty: 0,
                stack_count: 1,
                max_duration_ms: -1,
                remain_time_ms: -1,
                remain_charges: 0,
            }],
            [crate::session::CharacterAuraEffectRowLikeCpp {
                caster_guid: player_guid,
                spell_id: spell_id as u32,
                effect_mask: 1,
                effect_index: 0,
                amount: 100,
                base_amount: 100,
            }],
            0,
        ),
        1
    );

    let (combat, _, current_power0) = session
        .player_login_combat_stats_like_cpp(1, 5, 80, Some(15), 2_000)
        .expect("login combat snapshot");
    assert_eq!(combat.max_health, 20);
    assert_eq!(
        combat.health, 15,
        "saved health valid under the persisted stamina aura must not be pre-clamped to the unbuffed max"
    );
    assert_eq!(
        current_power0, 1_320,
        "saved primary power is clamped after the final aura/item projection like C++"
    );
}
#[test]
fn login_known_spells_include_account_mounts_even_when_use_condition_fails_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(8);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_known_spells_like_cpp(vec![635]);
    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([
        wow_data::MountEntry {
            id: 1,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: 100,
            player_condition_id: 42,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
        wow_data::MountEntry {
            id: 2,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: 101,
            player_condition_id: 43,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
    ])));
    session.set_player_condition_store(Arc::new(wow_data::PlayerConditionStore::from_entries([
        wow_data::PlayerConditionEntry {
            id: 42,
            class_mask: 1,
            ..Default::default()
        },
        wow_data::PlayerConditionEntry {
            id: 43,
            class_mask: 1 << 1,
            ..Default::default()
        },
    ])));

    session.set_account_mounts_like_cpp(vec![
        AccountMount {
            spell_id: 100,
            flags: 0,
        },
        AccountMount {
            spell_id: 101,
            flags: 0,
        },
    ]);

    let login_spells = session.login_known_spells_after_account_collections_like_cpp();
    assert!(login_spells.contains(&635));
    assert!(login_spells.contains(&100));
    assert!(
        login_spells.contains(&101),
        "C++ CollectionMgr::AddMount stores/learns the mount before evaluating PlayerCondition; the condition applies to using it"
    );
}
#[test]
fn send_known_spells_filters_disabled_and_inactive_like_cpp() {
    assert_eq!(active_known_spell_for_send_like_cpp(118, 1, 0), Some(118));
    assert_eq!(
        active_known_spell_for_send_like_cpp(118, 0, 0),
        None,
        "C++ Player::SendKnownSpells skips inactive spells"
    );
    assert_eq!(
        active_known_spell_for_send_like_cpp(118, 1, 1),
        None,
        "C++ Player::SendKnownSpells skips disabled spells"
    );
    assert_eq!(active_known_spell_for_send_like_cpp(0, 1, 0), None);
}
#[test]
fn login_known_spells_filters_complete_has_spell_mirror_like_cpp() {
    let (mut session, _) = make_session_with_send_capacity(1);
    session.set_known_spells_like_cpp(vec![100, 200]);
    assert!(
        session.set_complete_represented_player_spell_rows_like_cpp([
            crate::session::RepresentedPlayerSpellLikeCpp {
                spell_id: 100,
                active: false,
                disabled: false,
                dependent: false,
                favorite: false,
                state: crate::session::RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
            crate::session::RepresentedPlayerSpellLikeCpp {
                spell_id: 200,
                active: true,
                disabled: false,
                dependent: false,
                favorite: false,
                state: crate::session::RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
            crate::session::RepresentedPlayerSpellLikeCpp {
                spell_id: 300,
                active: true,
                disabled: true,
                dependent: false,
                favorite: false,
                state: crate::session::RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
        ])
    );

    assert_eq!(session.known_spells_like_cpp(), &[100, 200]);
    assert_eq!(
        session.login_known_spells_after_account_collections_like_cpp(),
        vec![200],
        "C++ Player::SendKnownSpells excludes inactive and disabled PlayerSpellMap rows"
    );
}
#[test]
fn load_spells_keeps_inactive_non_disabled_spells_for_add_spell_side_effects_like_cpp() {
    assert_eq!(
        loaded_spell_for_add_spell_side_effects_like_cpp(118, 0),
        Some(118),
        "C++ Player::_LoadSpells still calls AddSpell for inactive rows; SendKnownSpells filters them later"
    );
    assert_eq!(
        loaded_spell_for_add_spell_side_effects_like_cpp(118, 1),
        None,
        "C++ AddSpell returns before cast side effects for disabled spell rows"
    );
    assert_eq!(loaded_spell_for_add_spell_side_effects_like_cpp(0, 0), None);
}
#[test]
fn login_skill_reward_spells_retain_cpp_dependent_ownership() {
    let mut known = vec![100, 200];
    let mut side_effects = vec![100, 200];
    let mut dependent = HashSet::from([200]);
    let mut removed = HashSet::new();

    apply_skill_rewarded_spell_changes_to_login_like_cpp(
        &mut known,
        &mut side_effects,
        &mut dependent,
        &mut removed,
        wow_data::SkillRewardedSpellChangesLikeCpp {
            learn: vec![300],
            remove: vec![200],
        },
    );

    assert_eq!(known, vec![100, 300]);
    assert_eq!(side_effects, vec![100, 300]);
    assert_eq!(dependent, HashSet::from([300]));
    assert_eq!(removed, HashSet::from([200]));
}
#[test]
fn send_known_spells_favorites_are_subset_of_sent_spells_like_cpp() {
    let favorites = HashSet::from([635, 999]);

    assert_eq!(
        favorite_known_spells_for_send_like_cpp(&[118, 635, 133], &favorites),
        vec![635],
        "C++ only marks favorite spells while iterating spells that are actually sent"
    );
}
#[test]
fn spell_history_entry_from_db_splits_spell_and_category_cooldowns_like_cpp() {
    let entry = spell_history_entry_from_db_like_cpp(133, 6948, 1_030, 12, 1_010, 1_000)
        .expect("future cooldown should be serialized");

    assert_eq!(entry.spell_id, 133);
    assert_eq!(entry.item_id, 6948);
    assert_eq!(entry.category, 12);
    assert_eq!(entry.recovery_time_ms, 30_000);
    assert_eq!(entry.category_recovery_time_ms, 10_000);
    assert_eq!(entry.mod_rate, 1.0);
    assert!(!entry.on_hold);
}
#[test]
fn spell_history_entry_omits_recovery_when_category_last_longer_like_cpp() {
    let entry = spell_history_entry_from_db_like_cpp(133, 0, 1_005, 12, 1_010, 1_000)
        .expect("future category cooldown should be serialized");

    assert_eq!(entry.category, 12);
    assert_eq!(entry.recovery_time_ms, 0);
    assert_eq!(entry.category_recovery_time_ms, 10_000);
}
#[test]
fn spell_history_entry_skips_expired_cooldowns_like_cpp() {
    assert_eq!(
        spell_history_entry_from_db_like_cpp(133, 0, 1_000, 12, 1_010, 1_000),
        None
    );
}
#[test]
fn spell_charge_entry_uses_first_recharge_and_consumed_count_like_cpp() {
    let entry = spell_charge_entry_from_db_like_cpp(42, 1_045, 2, 1_000)
        .expect("future charge should be serialized");

    assert_eq!(entry.category, 42);
    assert_eq!(entry.next_recovery_time_ms, 45_000);
    assert_eq!(entry.charge_mod_rate, 1.0);
    assert_eq!(entry.consumed_charges, 2);
}
#[test]
fn spell_charge_entry_skips_expired_recharges_like_cpp() {
    assert_eq!(
        spell_charge_entry_from_db_like_cpp(42, 1_000, 1, 1_000),
        None
    );
}
#[test]
fn account_mount_spells_are_dependent_and_not_saved_to_character_spell_like_cpp() {
    assert!(
        WorldSession::account_mount_spells_are_session_dependent_like_cpp(),
        "C++ CollectionMgr::AddMount calls Player::LearnSpell(spellId, true); Player::_SaveSpells skips dependent spells, so account mounts must not be persisted into character_spell"
    );
}
#[test]
fn top_level_bank_destination_applies_obtain_spells_like_cpp_store_item() {
    assert!(bank_store_destination_applies_obtain_spells_like_cpp(
        INVENTORY_SLOT_BAG_0
    ));
    assert!(bank_store_destination_applies_obtain_spells_like_cpp(
        wow_entities::INVENTORY_SLOT_BAG_START
    ));
    assert!(
        !bank_store_destination_applies_obtain_spells_like_cpp(wow_entities::BANK_SLOT_BAG_START),
        "C++ _StoreItem excludes bank-bag containers but not bag-0 bank slots"
    );
}
#[tokio::test]
async fn binder_activate_fans_spell_go_to_visible_nearby_observers_like_cpp() {
    let (mut session, sender_rx, canonical) = make_bank_slot_session(16);
    insert_bank_test_player_in_world(&session, &canonical);
    // Login adopts the canonical Player handle and the character arrives alive
    // with a faction. The cast identity allocator fails closed without the
    // handle, HandleBinderActivateOpcode returns early for a caster that is not
    // alive, and the interaction reaction check fails closed without a faction
    // template, so this fixture installs all three like production does.
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    assert!(
        crate::canonical_player_access::configure_canonical_player_vitals_for_test(
            &canonical,
            session.player_guid().expect("loaded player"),
            (100, 100, wow_constants::PowerType::Mana, 100, 100, 100),
        )
    );
    session.set_player_faction_template_like_cpp(1);
    let innkeeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 32);
    insert_banker_creature(&canonical, innkeeper, NPCFlags1::INNKEEPER.bits());
    session.set_player_zone_area_like_cpp(12, 34);
    install_bind_spell_fixture(&mut session);

    let registry = Arc::new(crate::session::directory::PlayerRegistry::default());
    session.set_player_registry(Arc::clone(&registry));
    let (mut nearby_visible, nearby_visible_rx) = make_binder_observer(
        43,
        Position::new(10.0, 0.0, 0.0, 0.0),
        innkeeper,
        true,
        &registry,
        &canonical,
    );
    let (mut nearby_hidden, nearby_hidden_rx) = make_binder_observer(
        44,
        Position::new(12.0, 0.0, 0.0, 0.0),
        innkeeper,
        false,
        &registry,
        &canonical,
    );
    let (mut distant_visible, distant_visible_rx) = make_binder_observer(
        45,
        Position::new(5_000.0, 0.0, 0.0, 0.0),
        innkeeper,
        true,
        &registry,
        &canonical,
    );

    session
        .handle_binder_activate(Hello { unit: innkeeper })
        .await;
    let activating_player_spell_go = sender_rx.try_recv().expect("activator SpellGo");

    nearby_visible
        .process_represented_session_commands_like_cpp()
        .await;
    nearby_hidden
        .process_represented_session_commands_like_cpp()
        .await;
    distant_visible
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(
        nearby_visible_rx
            .try_recv()
            .expect("visible nearby observer SpellGo"),
        activating_player_spell_go
    );
    assert!(nearby_visible_rx.try_recv().is_err());
    assert!(
        nearby_hidden_rx.try_recv().is_err(),
        "C++ HaveAtClient gate rejects a non-visible innkeeper"
    );
    assert!(
        distant_visible_rx.try_recv().is_err(),
        "C++ MessageDistDeliverer rejects observers outside visibility range"
    );
}
