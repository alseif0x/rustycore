//! Session scenarios exercising the represented persistence responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn update_talent_data_includes_loaded_talents_and_glyphs_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_talent_store(Arc::new(wow_data::TalentStore::from_entries([
        test_talent_entry_like_cpp(101, 2, 50_101),
        test_talent_entry_like_cpp(202, 1, 50_202),
    ])));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(50_101, test_spell_info_like_cpp(50_101));
    spell_store.insert(50_202, test_spell_info_like_cpp(50_202));
    session.set_spell_store(Arc::new(spell_store));
    let talent_tabs = install_test_talent_tab_store_like_cpp(&mut session);

    session.set_represented_active_talent_group_like_cpp(1);
    session.set_represented_bonus_talent_groups_like_cpp(1);
    assert!(session.load_represented_talent_row_like_cpp(&talent_tabs, 101, 2, 0));
    assert!(session.load_represented_talent_row_like_cpp(&talent_tabs, 202, 1, 1));
    assert!(session.load_represented_glyph_row_like_cpp(&glyph_catalog::catalog(456), 1, 3, 456));

    let packet = session.represented_update_talent_data_packet_like_cpp();

    assert_eq!(packet.active_group, 1);
    assert_eq!(packet.groups.len(), 2);
    assert_eq!(
        packet.groups[0].talents,
        vec![wow_packet::packets::misc::TalentInfoLikeCpp {
            talent_id: 101,
            rank: 2,
        }]
    );
    assert_eq!(
        packet.groups[1].talents,
        vec![wow_packet::packets::misc::TalentInfoLikeCpp {
            talent_id: 202,
            rank: 1,
        }]
    );
    assert_eq!(packet.groups[1].glyph_ids[3], 456);
}
#[test]
fn talent_reset_persistence_plan_clears_active_preserves_inactive_and_keeps_zero_cost_marker() {
    let (mut session, _, _) = make_session();
    session.set_talent_store(Arc::new(wow_data::TalentStore::from_entries([
        test_talent_entry_like_cpp(101, 0, 50_101),
        test_talent_entry_like_cpp(202, 1, 50_202),
    ])));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        50_101,
        wow_data::SpellInfo {
            effects: vec![wow_data::SpellEffectInfo {
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_SPELL,
                effect_trigger_spell: 60_101,
                ..Default::default()
            }],
            ..test_spell_info_like_cpp(50_101)
        },
    );
    spell_store.insert(60_101, test_spell_info_like_cpp(60_101));
    spell_store.insert(50_202, test_spell_info_like_cpp(50_202));
    session.set_spell_store(Arc::new(spell_store));
    let talent_tabs = install_test_talent_tab_store_like_cpp(&mut session);

    session.set_represented_active_talent_group_like_cpp(0);
    assert!(session.load_represented_talent_row_like_cpp(&talent_tabs, 101, 0, 0));
    assert!(session.load_represented_talent_row_like_cpp(&talent_tabs, 202, 1, 1));
    session.mark_represented_talents_loaded_like_cpp();
    session.set_known_spells_like_cpp(vec![50_101, 60_101, 50_202]);

    let (state_plan, request) = session
        .represented_talent_reset_persistence_plan_like_cpp(42, 777, 777, 0, 123)
        .expect("coherent talent state should produce an atomic reset plan");

    assert_eq!(state_plan.active_group, 0);
    assert_eq!(state_plan.active_talents, BTreeMap::from([(101, 0)]));
    assert!(state_plan.post_talents[0].is_empty());
    assert_eq!(state_plan.post_talents[1], BTreeMap::from([(202, 1)]));
    assert_eq!(request.player_guid, 42);
    assert_eq!(request.money_before, 777);
    assert_eq!(request.money_after, 777);
    assert_eq!(request.reset_cost, 0);
    assert_eq!(request.reset_time_secs, 123);
    assert_eq!(
        request.retained_talents,
        vec![wow_persistence::PlayerTalentResetSaveRowLikeCpp {
            talent_id: 202,
            rank: 1,
            talent_group: 1,
        }],
        "the inactive talent group is reinserted after the delete-all"
    );

    assert_eq!(session.player_gold_like_cpp(), 0);
    assert_eq!(
        session
            .represented_update_talent_data_packet_like_cpp()
            .groups[0]
            .talents
            .len(),
        1,
        "building the transaction plan is pure and does not publish the reset"
    );
}
#[test]
fn talent_reset_persistence_plan_carries_capped_fee_and_empty_retained_set() {
    let (mut session, _, _) = make_session();
    session.mark_represented_talents_loaded_like_cpp();
    let month = 30 * 24 * 60 * 60;
    let now = 10 * month;
    session.set_represented_talent_reset_state_like_cpp(500_000, now);
    let cost = session
        .represented_next_reset_talents_cost_like_cpp(now)
        .expect("fixture specialization owner");
    assert_eq!(cost, 500_000);

    let (_, request) = session
        .represented_talent_reset_persistence_plan_like_cpp(42, 1_000_000, 500_000, cost, now)
        .expect("loaded empty active group should still persist its reset");

    assert_eq!(request.player_guid, 42);
    assert_eq!(request.money_before, 1_000_000);
    assert_eq!(request.money_after, 500_000);
    assert_eq!(request.reset_cost, 500_000);
    assert_eq!(request.reset_time_secs, now);
    assert!(request.retained_talents.is_empty());
}
#[test]
fn character_glyph_load_filters_invalid_rows_like_cpp() {
    let (mut session, _, _) = make_session();
    let glyph_properties =
        wow_data::GlyphPropertiesStore::from_entries([wow_data::GlyphPropertiesEntry {
            id: 123,
            spell_id: 10,
            glyph_type: 1,
            glyph_exclusive_category_id: 0,
            spell_icon_file_data_id: 0,
            glyph_slot_flags: 0,
        }]);

    assert!(session.load_represented_glyph_row_like_cpp(&glyph_properties, 0, 0, 123));
    assert!(!session.load_represented_glyph_row_like_cpp(&glyph_properties, 4, 0, 123));
    assert!(!session.load_represented_glyph_row_like_cpp(&glyph_properties, 0, 6, 123));
    assert!(!session.load_represented_glyph_row_like_cpp(&glyph_properties, 0, 1, 999));
    session.mark_represented_glyphs_loaded_like_cpp();

    let packet = session.represented_update_talent_data_packet_like_cpp();
    assert_eq!(packet.groups[0].glyph_ids[0], 123);
    assert_eq!(packet.groups[0].glyph_ids[1], 0);
}
#[test]
fn cuf_profile_loader_rejects_cpp_oob_id_bug() {
    let (mut session, _, _) = make_session();
    session.clear_represented_cuf_profiles_like_cpp();

    assert!(!session.load_represented_cuf_profile_like_cpp(
        wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP as u8,
        cuf_profile_for_save_test("Invalid", 99)
    ));
    assert!(
        session
            .represented_load_cuf_profiles_packet_like_cpp()
            .expect("handle-less CUF fixture")
            .profiles
            .is_empty(),
        "C++ checks id > MAX_CUF_PROFILES before indexing an array of MAX_CUF_PROFILES; Rust rejects id == MAX to avoid the legacy OOB bug"
    );
}
#[test]
fn vendor_currency_purchase_plan_does_not_publish_before_commit_like_cpp() {
    let (mut session, _, _) = make_session();
    session.player_race = 1;
    session.set_currency_types_store(Arc::new(wow_data::CurrencyTypesStore::from_entries([
        currency_entry(395),
        currency_entry(396),
    ])));
    session.player_currencies.insert(
        396,
        PlayerCurrency {
            state: PlayerCurrencyState::Unchanged,
            quantity: 10,
            weekly_quantity: 0,
            tracked_quantity: 0,
            increased_cap_quantity: 0,
            earned_quantity: 0,
            flags: 0,
        },
    );
    let runtime_before = session.player_currencies_like_cpp().unwrap();
    let mut planned = runtime_before.clone();

    let gain = session
        .plan_add_currency_vendor_like_cpp(&mut planned, 395, 3)
        .expect("represented vendor currency should be plannable")
        .expect("the uncapped gain should be nonzero");
    assert!(crate::session_rules::plan_remove_currency_like_cpp(
        &mut planned,
        396,
        4
    ));

    let request = session.plan_player_currency_save_like_cpp(42, &mut planned);

    assert_eq!(gain.quantity, 3);
    assert_eq!(planned.get(&395).map(|currency| currency.quantity), Some(3));
    assert_eq!(planned.get(&396).map(|currency| currency.quantity), Some(6));
    assert_eq!(
        planned.get(&395).map(|currency| currency.state),
        Some(PlayerCurrencyState::Unchanged)
    );
    assert_eq!(
        planned.get(&396).map(|currency| currency.state),
        Some(PlayerCurrencyState::Unchanged)
    );
    assert_eq!(request.rows.len(), 2);
    assert_eq!(
        session.player_currencies_like_cpp(),
        Some(runtime_before),
        "a definite rollback or cancellation before COMMIT must leave runtime unchanged"
    );
}
#[test]
fn vendor_currency_purchase_publishes_only_committed_plan_like_cpp() {
    let (mut session, _, _) = make_session();
    session.player_race = 1;
    session.set_currency_types_store(Arc::new(wow_data::CurrencyTypesStore::from_entries([
        currency_entry(395),
        currency_entry(396),
    ])));
    session.player_currencies.insert(
        396,
        PlayerCurrency {
            state: PlayerCurrencyState::Unchanged,
            quantity: 10,
            weekly_quantity: 0,
            tracked_quantity: 0,
            increased_cap_quantity: 0,
            earned_quantity: 0,
            flags: 0,
        },
    );
    let mut planned = session.player_currencies_like_cpp().unwrap();
    session
        .plan_add_currency_vendor_like_cpp(&mut planned, 395, 3)
        .unwrap();
    assert!(crate::session_rules::plan_remove_currency_like_cpp(
        &mut planned,
        396,
        4
    ));
    let request = session.plan_player_currency_save_like_cpp(42, &mut planned);
    assert_eq!(request.rows.len(), 2);

    // This synchronous publication is the post-COMMIT half used by
    // `handle_buy_item`; no fallible/async operation separates it from the
    // durable success branch.
    session.set_player_currencies_like_cpp(planned);

    assert_eq!(session.player_currency_quantity(395), Some(3));
    assert_eq!(session.player_currency_quantity(396), Some(6));
    assert!(
        session
            .player_currencies_like_cpp()
            .unwrap()
            .values()
            .all(|currency| currency.state == PlayerCurrencyState::Unchanged)
    );
}
#[test]
fn vendor_currency_unknown_commit_is_quarantined_without_money_evidence_like_cpp() {
    assert_eq!(
        reconcile_absolute_player_money_commit_like_cpp(100, 100, Some(100)),
        AbsolutePlayerMoneyCommitReconciliationLikeCpp::Indeterminate,
        "currency-only vendor transactions use equal money markers, so a lost COMMIT reply must require relog instead of guessing"
    );
}
#[test]
fn load_character_titles_parses_known_title_words_like_cpp() {
    let (mut session, _, _) = make_session();
    session.represented_learn_title_like_cpp(999);
    session.represented_set_chosen_title_like_cpp(999);

    session.load_represented_character_titles_like_cpp("1 2 invalid 8", 33);

    assert!(session.represented_has_title_like_cpp(0));
    assert!(session.represented_has_title_like_cpp(33));
    assert!(session.represented_has_title_like_cpp(99));
    assert!(!session.represented_has_title_like_cpp(65));
    assert!(!session.represented_has_title_like_cpp(999));
    assert_eq!(session.represented_chosen_title_like_cpp(), 33);
}
#[test]
fn load_character_titles_clears_unknown_chosen_title_like_cpp() {
    let (mut session, _, _) = make_session();

    session.load_represented_character_titles_like_cpp("1 0", 33);

    assert!(session.represented_has_title_like_cpp(0));
    assert!(!session.represented_has_title_like_cpp(33));
    assert_eq!(session.represented_chosen_title_like_cpp(), 0);
}
#[test]
fn load_character_titles_sets_canonical_player_title_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 31_100);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TitleOwner".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical title owner");

    session.load_represented_character_titles_like_cpp("4", 2);

    assert_eq!(session.represented_chosen_title_like_cpp(), 2);
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.data().player_title)
            .unwrap(),
        2
    );
}
#[test]
fn loaded_total_stat_percentage_uses_persisted_effect_amount_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 17);
    session.set_player_guid(Some(player_guid));

    let mut spell_store = SpellStore::new();
    spell_store.insert(
        20_600,
        SpellInfo {
            spell_id: 20_600,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE,
                effect_base_points: 2,
                effect_die_sides: 1,
                effect_misc_value_1: 4,
                effect_misc_value_2: 0,
                ..Default::default()
            }],
            ..test_spell_info_like_cpp(20_600)
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    assert_eq!(
        session.load_represented_character_auras_like_cpp(
            [CharacterAuraRowLikeCpp {
                caster_guid: player_guid,
                spell_id: 20_600,
                effect_mask: 1,
                recalculate_mask: 0,
                difficulty: 0,
                stack_count: 1,
                max_duration_ms: -1,
                remain_time_ms: -1,
                remain_charges: 0,
            }],
            [CharacterAuraEffectRowLikeCpp {
                caster_guid: player_guid,
                spell_id: 20_600,
                effect_mask: 1,
                effect_index: 0,
                amount: 7,
                base_amount: 3,
            }],
            0,
        ),
        1
    );
    assert_eq!(
        session.represented_total_stat_multipliers_like_cpp(),
        [1.07; 5]
    );
    assert_eq!(
        session.represented_total_stat_buff_multipliers_like_cpp(),
        [1.0, 1.0, 1.0, 1.0, 1.07]
    );

    let aura = session
        .visible_auras
        .values()
        .find(|aura| aura.spell_id == 20_600)
        .expect("loaded total-stat aura");
    assert_eq!(aura.difficulty_id, 0);
    assert_eq!(aura.represented_effect, None);
    let slot = aura.slot;
    session.remove_aura(slot).expect("remove loaded aura");
    assert_eq!(
        session.represented_total_stat_multipliers_like_cpp(),
        [1.0; 5]
    );
    assert_eq!(
        session.represented_total_stat_buff_multipliers_like_cpp(),
        [1.0; 5]
    );
}
#[test]
fn loaded_condition_counts_persisted_gems_without_socket_template_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 90_524);
    let item_guid = ObjectGuid::create_item(1, 90_525);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(
        &canonical,
        player_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );
    session
        .mutate_canonical_player_like_cpp(|player| player.unit_mut().set_level(80))
        .unwrap();
    session.set_spell_item_enchantment_store(Arc::new(SpellItemEnchantmentStore::from_entries([
        SpellItemEnchantmentEntry {
            id: 912,
            effect_arg: [0; 3],
            effect_points_min: [0; 3],
            item_visual: 0,
            flags: SpellItemEnchantmentFlags::empty(),
            required_skill_id: 0,
            required_skill_rank: 0,
            item_level: 1,
            charges: 0,
            effect: [ItemEnchantmentType::None as u8; 3],
            condition_id: 44,
            min_level: 1,
            max_level: 0,
        },
    ])));
    session.set_spell_item_enchantment_condition_store(Arc::new(
        SpellItemEnchantmentConditionStore::from_entries([SpellItemEnchantmentConditionEntry {
            id: 44,
            lt_operand_type: [2, 0, 0, 0, 0],
            lt_operand: [0; 5],
            operator: [5, 0, 0, 0, 0],
            rt_operand_type: [0; 5],
            rt_operand: [1, 0, 0, 0, 0],
            logic: [0; 5],
        }]),
    ));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_templates(std::iter::empty::<(u32, ItemSparseTemplateEntry)>())
            .with_gem_properties([(800, 55)]),
    ));
    session.set_gem_properties_store(Arc::new(GemPropertiesStore::from_entries([
        GemPropertiesEntry {
            id: 55,
            enchant_id: 0,
            gem_type: 2,
            min_item_level: 0,
        },
    ])));

    session.insert_inventory_item_like_cpp(
        EQUIPMENT_SLOT_CHEST,
        InventoryItem {
            guid: item_guid,
            entry_id: 700,
            db_guid: item_guid.counter() as u64,
            inventory_type: Some(InventoryType::Chest as u8),
        },
    );
    let mut item = session.make_inventory_item_object(
        item_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_CHEST,
    );
    item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 912, 6_000, 0);
    session.insert_inventory_item_object(item);

    let blocked = session.apply_loaded_equipped_item_enchantments_like_cpp(item_guid);
    assert!(matches!(
        blocked.plans.as_slice(),
        [ApplyEnchantmentPlan {
            result: ApplyEnchantmentResult::Skipped(ApplyEnchantmentSkipReason::ConditionFailed)
        }]
    ));
    assert!(blocked.duration_updates.is_empty());

    assert!(
        session
            .items
            .stats_store
            .as_ref()
            .and_then(|store| store.socket_template(700))
            .is_none(),
        "C++ EnchantmentFitsRequirements counts SocketedGem rows and does not gate them on GetSocketColor(0)"
    );

    session.update_inventory_item_object_like_cpp(item_guid, |item| {
        item.set_gems(vec![SocketedGem {
            item_id: 800,
            context: 0,
            bonus_list_ids: Vec::new(),
        }]);
    });
    let allowed = session.apply_loaded_equipped_item_enchantments_like_cpp(item_guid);
    assert!(matches!(
        allowed.plans.as_slice(),
        [ApplyEnchantmentPlan {
            result: ApplyEnchantmentResult::Applied { .. }
        }]
    ));
    assert_eq!(allowed.duration_updates.len(), 1);
}
#[test]
fn loaded_legacy_ranged_slot_uses_max_attack_like_cpp_get_attack_by_slot() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 90_534);
    let item_guid = ObjectGuid::create_item(1, 90_535);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(
        &canonical,
        player_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_level(80);
            player
                .unit_mut()
                .set_unit_flags2_like_cpp(UnitFlags2::DISARM_RANGED);
        })
        .unwrap();
    session.set_spell_item_enchantment_store(Arc::new(SpellItemEnchantmentStore::from_entries([
        SpellItemEnchantmentEntry {
            id: 903,
            effect_arg: [0; 3],
            effect_points_min: [0; 3],
            item_visual: 0,
            flags: SpellItemEnchantmentFlags::empty(),
            required_skill_id: 0,
            required_skill_rank: 0,
            item_level: 1,
            charges: 0,
            effect: [ItemEnchantmentType::None as u8; 3],
            condition_id: 0,
            min_level: 1,
            max_level: 0,
        },
    ])));

    let mut item = session.make_inventory_item_object(
        item_guid,
        705,
        player_guid,
        1,
        1,
        ItemContext::None,
        EQUIPMENT_SLOT_RANGED,
    );
    item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 903, 6_000, 0);
    session.insert_inventory_item_object(item);

    let outcome = session.apply_loaded_equipped_item_enchantments_like_cpp(item_guid);

    assert!(matches!(
        outcome.plans.as_slice(),
        [ApplyEnchantmentPlan {
            result: ApplyEnchantmentResult::Applied { .. }
        }]
    ));
    assert_eq!(outcome.duration_updates.len(), 1);
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| player.enchant_durations().len()),
        Some(1),
        "C++ GetAttackBySlot maps legacy EQUIPMENT_SLOT_RANGED to MAX_ATTACK, so DISARM_RANGED does not gate this slot"
    );
}
