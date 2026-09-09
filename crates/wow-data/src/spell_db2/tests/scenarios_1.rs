//! DB2 spell entry stores regression scenarios, part 1 of 1.
//!
//! Moved out of the spell_db2.rs root under #646; every test is unchanged.

use super::*;

#[test]
fn spell_misc_walks_difficulty_fallback_before_base_like_cpp() {
    let entry = |id, difficulty_id, school_mask| SpellMiscEntry {
        id,
        attributes: [0; 15],
        difficulty_id,
        casting_time_index: 0,
        duration_index: 0,
        range_index: 0,
        school_mask,
        speed: 0.0,
        launch_delay: 0.0,
        min_duration: 0.0,
        spell_icon_file_data_id: 0,
        active_icon_file_data_id: 0,
        content_tuning_id: 0,
        show_future_spell_player_condition_id: 0,
        spell_id: 42,
    };
    let store = SpellMiscStore::from_entries([entry(4200, 0, 1), entry(4201, 1, 4)]);
    let difficulties = crate::DifficultyStore::from_entries([
        crate::DifficultyEntry {
            id: 3,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 2,
            toggle_difficulty_id: 0,
        },
        crate::DifficultyEntry {
            id: 2,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 1,
            toggle_difficulty_id: 0,
        },
        crate::DifficultyEntry {
            id: 1,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
    ]);

    assert_eq!(
        store
            .entry_for_spell_difficulty_with_fallback_like_cpp(42, 3, Some(&difficulties))
            .map(|entry| entry.school_mask),
        Some(4)
    );
}

#[test]
fn spell_categories_uses_cpp_parent_relationship() {
    let store = SpellCategoriesStore::from_entries([SpellCategoriesEntry {
        id: 1,
        difficulty_id: 2,
        category: 3,
        defense_type: 4,
        dispel_type: 5,
        mechanic: 6,
        prevention_type: 7,
        start_recovery_category: 8,
        charge_category: 9,
        spell_id: 10,
    }]);

    assert_eq!(store.get(1).unwrap().spell_id, 10);
}

#[test]
fn hit_metadata_stores_overlay_then_apply_final_removals_like_cpp() {
    let table_hash = 0xAABB_CCDD;
    let removals =
        crate::Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([(table_hash, 2, 2)]);
    let category = |id, spell_id, defense_type, mechanic| SpellCategoriesEntry {
        id,
        difficulty_id: 0,
        category: 0,
        defense_type,
        dispel_type: 0,
        mechanic,
        prevention_type: 0,
        start_recovery_category: 0,
        charge_category: 0,
        spell_id,
    };

    let mut categories =
        SpellCategoriesStore::from_entries([category(1, 100, 1, 2), category(2, 200, 3, 4)]);
    categories.overlay_effective_row_like_cpp(category(1, 100, 5, 6));
    categories.overlay_effective_row_like_cpp(category(1, 100, 7, 8));
    categories.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, &removals);
    assert_eq!(
        categories
            .get(1)
            .map(|entry| (entry.defense_type, entry.mechanic)),
        Some((7, 8))
    );
    assert!(categories.get(2).is_none());

    let mut misc = SpellMiscStore::from_entries([
        test_spell_misc_entry(1, 100, 1),
        test_spell_misc_entry(2, 200, 2),
    ]);
    misc.overlay_effective_row_like_cpp(test_spell_misc_entry(1, 100, 4));
    misc.overlay_effective_row_like_cpp(test_spell_misc_entry(1, 100, 8));
    misc.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, &removals);
    assert_eq!(misc.get(1).map(|entry| entry.school_mask), Some(8));
    assert!(misc.get(2).is_none());

    let mut effects = SpellEffectDb2Store::from_entries([
        test_spell_effect_entry(1, 100, 1),
        test_spell_effect_entry(2, 200, 2),
    ]);
    effects.overlay_effective_row_like_cpp(test_spell_effect_entry(1, 100, 4));
    effects.overlay_effective_row_like_cpp(test_spell_effect_entry(1, 100, 7));
    effects.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, &removals);
    assert_eq!(effects.get(1).map(|entry| entry.effect_mechanic), Some(7));
    assert!(effects.get(2).is_none());
}

#[test]
fn cooldown_and_visual_stores_overlay_then_apply_final_removals_like_cpp() {
    let table_hash = 0xAABB_CCDD;
    let removals = crate::Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
        (table_hash, 2, 2),
        (table_hash, 3, 2),
    ]);

    let cooldown = |id, spell_id, recovery_time| SpellCooldownsEntry {
        id,
        difficulty_id: 2,
        category_recovery_time: recovery_time + 1,
        recovery_time,
        start_recovery_time: recovery_time + 2,
        spell_id,
    };
    let mut cooldowns =
        SpellCooldownsStore::from_entries([cooldown(1, 100, 10), cooldown(2, 200, 20)]);
    cooldowns.overlay_effective_row_like_cpp(cooldown(1, 101, 30));
    cooldowns.overlay_effective_row_like_cpp(cooldown(3, 300, 40));
    cooldowns.overlay_effective_row_like_cpp(cooldown(1, 102, 50));
    cooldowns.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, &removals);
    assert_eq!(cooldowns.get(1), Some(&cooldown(1, 102, 50)));
    assert!(cooldowns.get(2).is_none());
    assert!(cooldowns.get(3).is_none());

    let visual = |id, spell_id, spell_visual_id| SpellXSpellVisualEntry {
        id,
        difficulty_id: 2,
        spell_visual_id,
        probability: 1.0,
        flags: 3,
        priority: 4,
        spell_icon_file_id: 5,
        active_icon_file_id: 6,
        viewer_unit_condition_id: 7,
        viewer_player_condition_id: 8,
        caster_unit_condition_id: 9,
        caster_player_condition_id: 10,
        spell_id,
    };
    let mut visuals =
        SpellXSpellVisualStore::from_entries([visual(1, 100, 10), visual(2, 200, 20)]);
    visuals.overlay_effective_row_like_cpp(visual(1, 101, 30));
    visuals.overlay_effective_row_like_cpp(visual(3, 300, 40));
    visuals.overlay_effective_row_like_cpp(visual(1, 102, 50));
    visuals.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, &removals);
    assert_eq!(visuals.get(1), Some(&visual(1, 102, 50)));
    assert!(visuals.get(2).is_none());
    assert!(visuals.get(3).is_none());
}

#[test]
fn range_and_power_stores_overlay_then_apply_final_removals_like_cpp() {
    let table_hash = 0xAABB_CCDD;
    let removals = crate::Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
        (table_hash, 2, 2),
        (table_hash, 3, 2),
    ]);

    let range = |id, range_max| SpellRangeEntry {
        id,
        display_name: format!("range {id}"),
        display_name_short: format!("r{id}"),
        flags: 1,
        range_min: [0.0, 0.0],
        range_max: [range_max, range_max],
    };
    let mut ranges = SpellRangeStore::from_entries([range(1, 10.0), range(2, 20.0)]);
    // Official overlay, then a custom overlay that must win, plus a custom
    // row that the final tombstone pass has to drop again.
    ranges.overlay_effective_row_like_cpp(range(1, 30.0));
    ranges.overlay_effective_row_like_cpp(range(3, 40.0));
    ranges.overlay_effective_row_like_cpp(range(1, 50.0));
    ranges.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, &removals);
    assert_eq!(ranges.get(1), Some(&range(1, 50.0)));
    assert!(ranges.get(2).is_none());
    assert!(ranges.get(3).is_none());

    let power = |id, spell_id, mana_cost| SpellPowerEntry {
        id,
        order_index: 0,
        mana_cost,
        mana_cost_per_level: 1,
        mana_per_second: 2,
        power_display_id: 3,
        alt_power_bar_id: 4,
        power_cost_pct: 5.0,
        power_cost_max_pct: 6.0,
        power_pct_per_second: 7.0,
        power_type: 0,
        required_aura_spell_id: 8,
        optional_cost: 9,
        spell_id,
    };
    let mut powers = SpellPowerStore::from_entries([power(1, 100, 0), power(2, 200, 20)]);
    powers.overlay_effective_row_like_cpp(power(1, 100, 30));
    powers.overlay_effective_row_like_cpp(power(3, 300, 40));
    powers.overlay_effective_row_like_cpp(power(1, 100, 50));
    powers.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, &removals);
    assert_eq!(powers.get(1), Some(&power(1, 100, 50)));
    assert!(powers.get(2).is_none());
    assert!(powers.get(3).is_none());

    let difficulty = |id, order_index| SpellPowerDifficultyEntry {
        id,
        difficulty_id: 2,
        order_index,
    };
    let mut difficulties =
        SpellPowerDifficultyStore::from_entries([difficulty(1, 0), difficulty(2, 1)]);
    difficulties.overlay_effective_row_like_cpp(difficulty(1, 3));
    difficulties.overlay_effective_row_like_cpp(difficulty(3, 4));
    difficulties.overlay_effective_row_like_cpp(difficulty(1, 5));
    difficulties.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, &removals);
    assert_eq!(difficulties.get(1), Some(&difficulty(1, 5)));
    assert!(difficulties.get(2).is_none());
    assert!(difficulties.get(3).is_none());
}

#[test]
fn spell_power_effective_rows_iterate_in_record_id_order_like_cpp() {
    let power = |id, spell_id| SpellPowerEntry {
        id,
        order_index: 0,
        mana_cost: id as i32,
        mana_cost_per_level: 0,
        mana_per_second: 0,
        power_display_id: 0,
        alt_power_bar_id: 0,
        power_cost_pct: 0.0,
        power_cost_max_pct: 0.0,
        power_pct_per_second: 0.0,
        power_type: 0,
        required_aura_spell_id: 0,
        optional_cost: 0,
        spell_id,
    };
    let store = SpellPowerStore::from_entries([power(30, 100), power(10, 100), power(20, 100)]);
    assert_eq!(
        store
            .entries_by_record_id_like_cpp()
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        vec![10, 20, 30]
    );
}

#[test]
fn remaining_cast_path_stores_overlay_then_apply_final_removals_like_cpp() {
    // Every store the creature-cast path reads must compose DB2, the
    // official overlay, the custom overlay and the final tombstone pass in
    // that order, so record 1 keeps the last overlay and records 2 and 3 are
    // dropped again by the removals.
    let table_hash = 0xAABB_CCDD;
    let removals = crate::Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
        (table_hash, 2, 2),
        (table_hash, 3, 2),
    ]);

    let cast_time = |id, base| SpellCastTimesEntry {
        id,
        base,
        per_level: 1,
        minimum: 2,
    };
    let mut cast_times =
        SpellCastTimesStore::from_entries([cast_time(1, 1_500), cast_time(2, 2_500)]);
    cast_times.overlay_effective_row_like_cpp(cast_time(1, 0));
    cast_times.overlay_effective_row_like_cpp(cast_time(3, 3_500));
    cast_times.overlay_effective_row_like_cpp(cast_time(1, 4_500));
    cast_times.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, &removals);
    assert_eq!(cast_times.get(1), Some(&cast_time(1, 4_500)));
    assert!(cast_times.get(2).is_none());
    assert!(cast_times.get(3).is_none());

    let duration = |id, duration| SpellDurationEntry {
        id,
        duration,
        duration_per_level: 0,
        max_duration: duration,
    };
    let mut durations = SpellDurationStore::from_entries([duration(1, 10), duration(2, 20)]);
    durations.overlay_effective_row_like_cpp(duration(1, 30));
    durations.overlay_effective_row_like_cpp(duration(3, 40));
    durations.overlay_effective_row_like_cpp(duration(1, -1));
    durations.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, &removals);
    assert_eq!(
        durations.get(1),
        Some(&duration(1, -1)),
        "a hotfixed permanent duration must survive as the effective row"
    );
    assert!(durations.get(2).is_none());
    assert!(durations.get(3).is_none());

    let category = |id, flags| SpellCategoryEntry {
        id,
        name: format!("category {id}"),
        flags,
        uses_per_week: 0,
        max_charges: 0,
        charge_recovery_time: 0,
        type_mask: 0,
    };
    let mut categories = SpellCategoryStore::from_entries([category(1, 0), category(2, 0)]);
    categories.overlay_effective_row_like_cpp(category(1, 0x1));
    categories.overlay_effective_row_like_cpp(category(3, 0x1));
    // The cooldown-starts-on-event flag is what the AI cooldown proof reads,
    // so the last overlay of it has to win.
    categories.overlay_effective_row_like_cpp(category(1, 0x8));
    categories.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, &removals);
    assert_eq!(categories.get(1).map(|entry| entry.flags), Some(0x8));
    assert!(categories.get(2).is_none());
    assert!(categories.get(3).is_none());

    let radius = |id, radius| SpellRadiusEntry {
        id,
        radius,
        radius_per_level: 0.0,
        radius_min: 0.0,
        radius_max: radius,
    };
    let mut radii = SpellRadiusStore::from_entries([radius(1, 5.0), radius(2, 6.0)]);
    radii.overlay_effective_row_like_cpp(radius(1, 7.0));
    radii.overlay_effective_row_like_cpp(radius(3, 8.0));
    radii.overlay_effective_row_like_cpp(radius(1, 9.0));
    radii.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, &removals);
    assert_eq!(radii.get(1), Some(&radius(1, 9.0)));
    assert!(radii.get(2).is_none());
    assert!(radii.get(3).is_none());

    let shapeshift = |id, spell_id, mask| SpellShapeshiftEntry {
        id,
        spell_id,
        stance_bar_order: 0,
        shapeshift_exclude: [0, 0],
        shapeshift_mask: [mask, 0],
    };
    let mut shapeshifts =
        SpellShapeshiftStore::from_entries([shapeshift(1, 100, 0x1), shapeshift(2, 200, 0x2)]);
    shapeshifts.overlay_effective_row_like_cpp(shapeshift(1, 100, 0x4));
    shapeshifts.overlay_effective_row_like_cpp(shapeshift(3, 300, 0x8));
    shapeshifts.overlay_effective_row_like_cpp(shapeshift(1, 100, 0x10));
    shapeshifts.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, &removals);
    assert_eq!(shapeshifts.get(1), Some(&shapeshift(1, 100, 0x10)));
    assert!(
        shapeshifts.get(2).is_none(),
        "a tombstoned row must stop gating its spell's stance requirement"
    );
    assert!(shapeshifts.get(3).is_none());

    let interrupts = |id, spell_id, aura| SpellInterruptsEntry {
        id,
        difficulty_id: 0,
        interrupt_flags: 1,
        aura_interrupt_flags: [aura, 0],
        channel_interrupt_flags: [0, 0],
        spell_id,
    };
    let mut interrupt_rows =
        SpellInterruptsStore::from_entries([interrupts(1, 100, 0x1), interrupts(2, 200, 0x2)]);
    interrupt_rows.overlay_effective_row_like_cpp(interrupts(1, 100, 0x4));
    interrupt_rows.overlay_effective_row_like_cpp(interrupts(3, 300, 0x8));
    interrupt_rows.overlay_effective_row_like_cpp(interrupts(1, 100, 0x10));
    interrupt_rows.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, &removals);
    assert_eq!(interrupt_rows.get(1), Some(&interrupts(1, 100, 0x10)));
    assert!(interrupt_rows.get(2).is_none());
    assert!(interrupt_rows.get(3).is_none());
}

#[test]
fn spell_equipped_items_effective_overlay_and_removal_order_like_cpp() {
    let mut store = SpellEquippedItemsStore::from_entries([
        SpellEquippedItemsEntry {
            id: 1,
            spell_id: 100,
            equipped_item_class: 2,
            equipped_item_inv_types: 4,
            equipped_item_subclass: 8,
        },
        SpellEquippedItemsEntry {
            id: 2,
            spell_id: 200,
            equipped_item_class: 3,
            equipped_item_inv_types: 5,
            equipped_item_subclass: 9,
        },
    ]);
    store.overlay_effective_row_like_cpp(SpellEquippedItemsEntry {
        id: 1,
        spell_id: 101,
        equipped_item_class: 6,
        equipped_item_inv_types: 7,
        equipped_item_subclass: 10,
    });
    let removals =
        crate::Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([(0xAABB_CCDD, 2, 2)]);
    store.apply_hotfix_removals_with_table_hash_like_cpp(0xAABB_CCDD, &removals);

    let effective = store
        .entry_for_spell_id_like_cpp(101)
        .expect("overlay replacement");
    assert_eq!(effective.id, 1);
    assert_eq!(effective.equipped_item_class, 6);
    assert!(store.entry_for_spell_id_like_cpp(100).is_none());
    assert!(store.entry_for_spell_id_like_cpp(200).is_none());
}

#[test]
fn spell_equipped_items_duplicate_spell_uses_highest_record_id_like_cpp() {
    let store = SpellEquippedItemsStore::from_entries([
        SpellEquippedItemsEntry {
            id: 9,
            spell_id: 100,
            equipped_item_class: 2,
            equipped_item_inv_types: 4,
            equipped_item_subclass: 8,
        },
        SpellEquippedItemsEntry {
            id: 3,
            spell_id: 100,
            equipped_item_class: -1,
            equipped_item_inv_types: 0,
            equipped_item_subclass: 0,
        },
    ]);

    assert_eq!(
        store
            .entry_for_spell_id_like_cpp(100)
            .map(|entry| (entry.id, entry.equipped_item_class)),
        Some((9, 2))
    );
}

#[test]
fn spell_name_hotfix_rows_override_and_add_effective_ids_like_cpp() {
    let mut store = SpellNameStore::from_entries([SpellNameEntry {
        id: 1,
        name: "File name".to_string(),
    }]);

    store.overlay_hotfix_row_like_cpp(1, "Official name".to_string());
    store.overlay_hotfix_row_like_cpp(2, "SQL-only name".to_string());
    store.overlay_hotfix_row_like_cpp(1, "Custom name".to_string());

    assert_eq!(store.len(), 2);
    assert_eq!(
        store.get(1).map(|entry| entry.name.as_str()),
        Some("Custom name")
    );
    assert_eq!(
        store.get(2).map(|entry| entry.name.as_str()),
        Some("SQL-only name"),
        "a SQL-only SpellName ID must participate in the server-side collision gate"
    );
}

#[test]
fn aura_restriction_overlays_then_final_removals_match_cpp_order() {
    let row = |id, spell_id, caster_aura_spell| SpellAuraRestrictionsEntry {
        id,
        difficulty_id: 0,
        caster_aura_state: 0,
        target_aura_state: 0,
        exclude_caster_aura_state: 0,
        exclude_target_aura_state: 0,
        caster_aura_spell,
        target_aura_spell: 0,
        exclude_caster_aura_spell: 0,
        exclude_target_aura_spell: 0,
        spell_id,
    };
    let table_hash = 0xAABB_CCDD;
    let mut store = SpellAuraRestrictionsStore::from_entries([row(1, 100, 10), row(2, 200, 20)]);

    store.overlay_effective_row_like_cpp(row(1, 100, 11));
    store.overlay_effective_row_like_cpp(row(3, 300, 30));
    store.overlay_effective_row_like_cpp(row(1, 100, 12));
    let removals = crate::Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
        (table_hash, 2, 2),
        (table_hash, 3, 2),
    ]);
    store.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, &removals);

    assert_eq!(store.len(), 1);
    assert_eq!(store.get(1).map(|entry| entry.caster_aura_spell), Some(12));
    assert!(store.get(2).is_none());
    assert!(store.get(3).is_none());
}

#[test]
fn duration_and_radius_helpers_match_cpp_fallbacks() {
    let duration_store = SpellDurationStore::from_entries([SpellDurationEntry {
        id: 7,
        duration: -5000,
        duration_per_level: 0,
        max_duration: 0,
    }]);
    assert_eq!(spell_duration_ms_like_cpp(0, Some(&duration_store)), 0);
    assert_eq!(spell_duration_ms_like_cpp(99, Some(&duration_store)), 0);
    assert_eq!(spell_duration_ms_like_cpp(7, Some(&duration_store)), 5000);

    let infinite_duration_store = SpellDurationStore::from_entries([SpellDurationEntry {
        id: 8,
        duration: -1,
        duration_per_level: 0,
        max_duration: 0,
    }]);
    assert_eq!(
        spell_duration_ms_like_cpp(8, Some(&infinite_duration_store)),
        -1
    );

    let radius_store = SpellRadiusStore::from_entries([
        SpellRadiusEntry {
            id: 11,
            radius: 0.0,
            radius_per_level: 0.0,
            radius_min: 0.0,
            radius_max: 25.0,
        },
        SpellRadiusEntry {
            id: 12,
            radius: 0.0,
            radius_per_level: 0.0,
            radius_min: 10.0,
            radius_max: 25.0,
        },
    ]);
    assert_eq!(spell_effect_radius_like_cpp(0, Some(&radius_store)), 0.0);
    assert_eq!(spell_effect_radius_like_cpp(99, Some(&radius_store)), 0.0);
    assert_eq!(spell_effect_radius_like_cpp(11, Some(&radius_store)), 25.0);
    assert_eq!(spell_effect_radius_like_cpp(12, Some(&radius_store)), 10.0);
}

#[test]
fn load_spell_core_db2_subbatch_when_fixtures_exist() {
    let data_dir = "/home/server/woltk-server-core/Data";
    let locale = "esES";
    let dbc_dir = Path::new(data_dir).join("dbc").join(locale);
    if !dbc_dir.exists() {
        eprintln!(
            "Skipping test: DB2 fixture directory not found at {}",
            dbc_dir.display()
        );
        return;
    }

    macro_rules! load_if_exists {
        ($file:literal, $store:ty) => {
            if dbc_dir.join($file).exists() {
                let _store = <$store>::load(data_dir, locale)
                    .unwrap_or_else(|error| panic!("failed to load {}: {error:#}", $file));
            }
        };
    }

    load_if_exists!("SpellAuraOptions.db2", SpellAuraOptionsStore);
    load_if_exists!("SpellAuraRestrictions.db2", SpellAuraRestrictionsStore);
    load_if_exists!("SpellCastTimes.db2", SpellCastTimesStore);
    load_if_exists!(
        "SpellCastingRequirements.db2",
        SpellCastingRequirementsStore
    );
    load_if_exists!("SpellCategories.db2", SpellCategoriesStore);
    load_if_exists!("SpellCategory.db2", SpellCategoryStore);
    load_if_exists!("SpellClassOptions.db2", SpellClassOptionsStore);
    load_if_exists!("SpellCooldowns.db2", SpellCooldownsStore);
    load_if_exists!("SpellDuration.db2", SpellDurationStore);
    load_if_exists!("SpellEffect.db2", SpellEffectDb2Store);
    load_if_exists!("SpellEquippedItems.db2", SpellEquippedItemsStore);
    load_if_exists!("SpellFocusObject.db2", SpellFocusObjectStore);
    load_if_exists!("SpellInterrupts.db2", SpellInterruptsStore);
    load_if_exists!(
        "SpellItemEnchantmentCondition.db2",
        SpellItemEnchantmentConditionStore
    );
    load_if_exists!("SpellKeyboundOverride.db2", SpellKeyboundOverrideStore);
    load_if_exists!("SpellLabel.db2", SpellLabelStore);
    load_if_exists!("SpellLearnSpell.db2", SpellLearnSpellStore);
    load_if_exists!("SpellLevels.db2", SpellLevelsStore);
    load_if_exists!("SpellMisc.db2", SpellMiscStore);
    load_if_exists!("SpellName.db2", SpellNameStore);
    load_if_exists!("SpellPower.db2", SpellPowerStore);
    load_if_exists!("SpellPowerDifficulty.db2", SpellPowerDifficultyStore);
    load_if_exists!("SpellProcsPerMinute.db2", SpellProcsPerMinuteStore);
    load_if_exists!("SpellProcsPerMinuteMod.db2", SpellProcsPerMinuteModStore);
    load_if_exists!("SpellRadius.db2", SpellRadiusStore);
    load_if_exists!("SpellRange.db2", SpellRangeStore);
    load_if_exists!("SpellReagents.db2", SpellReagentsStore);
    load_if_exists!("SpellReagentsCurrency.db2", SpellReagentsCurrencyStore);
    load_if_exists!("SpellScaling.db2", SpellScalingStore);
    load_if_exists!("SpellShapeshift.db2", SpellShapeshiftStore);
    load_if_exists!("SpellShapeshiftForm.db2", SpellShapeshiftFormStore);
    load_if_exists!("SpellTargetRestrictions.db2", SpellTargetRestrictionsStore);
    load_if_exists!("SpellTotems.db2", SpellTotemsStore);
    load_if_exists!("SpellVisual.db2", SpellVisualStore);
    load_if_exists!("SpellVisualEffectName.db2", SpellVisualEffectNameStore);
    load_if_exists!("SpellVisualKit.db2", SpellVisualKitStore);
    load_if_exists!("SpellVisualMissile.db2", SpellVisualMissileStore);
    load_if_exists!("SpellXSpellVisual.db2", SpellXSpellVisualStore);
}

#[test]
fn blessing_of_auchindoun_fixture_has_only_outgoing_damage_and_xp_auras_like_cpp() {
    let data_dir = "/home/server/woltk-server-core/Data";
    let locale = "esES";
    let path = Path::new(data_dir)
        .join("dbc")
        .join(locale)
        .join("SpellEffect.db2");
    if !path.exists() {
        eprintln!(
            "Skipping test: SpellEffect.db2 not found at {}",
            path.display()
        );
        return;
    }

    let store = SpellEffectDb2Store::load(data_dir, locale).expect("load SpellEffect.db2");
    let mut effects: Vec<_> = store
        .entries_like_cpp()
        .filter(|entry| entry.spell_id == 33_377)
        .map(|entry| {
            (
                entry.effect_index,
                entry.effect,
                entry.effect_aura,
                entry.effect_trigger_spell,
            )
        })
        .collect();
    effects.sort_unstable();
    assert_eq!(effects, vec![(0, 6, 200, 0), (1, 6, 79, 0)]);
}

#[test]
fn spell_power_fixture_maps_relationship_spell_id_and_percent_fields_like_cpp() {
    let data_dir = "/home/server/woltk-server-core/Data";
    let locale = "enUS";
    let dbc_dir = Path::new(data_dir).join("dbc").join(locale);
    if !dbc_dir.join("SpellPower.db2").exists() {
        eprintln!(
            "Skipping test: SpellPower fixture not found at {}",
            dbc_dir.display()
        );
        return;
    }

    let store = SpellPowerStore::load(data_dir, locale).expect("load SpellPower.db2");
    let row = store
        .entries_like_cpp()
        .find(|entry| entry.spell_id == 48_071)
        .expect("Flash Heal row should be keyed by SpellID relationship");
    assert_eq!(row.power_type, 0);
    assert_eq!(row.order_index, 0);
    assert_eq!(row.mana_cost, 0);
    assert_eq!(row.power_cost_pct, 18.0);
    assert_eq!(row.power_cost_max_pct, 0.0);
}

#[test]
fn spell_x_spell_visual_fixture_skips_inline_record_id_like_cpp() {
    let data_dir = "/home/server/woltk-server-core/Data";
    let locale = "enUS";
    let path = Path::new(data_dir)
        .join("dbc")
        .join(locale)
        .join("SpellXSpellVisual.db2");
    if !path.exists() {
        eprintln!(
            "Skipping test: SpellXSpellVisual fixture not found at {}",
            path.display()
        );
        return;
    }

    let store = SpellXSpellVisualStore::load(data_dir, locale).expect("load SpellXSpellVisual.db2");
    let row = store
        .get(345_432)
        .expect("3.4.3 Scarlet Ballista visual row");

    assert_eq!(row.id, 345_432);
    assert_eq!(row.difficulty_id, 0);
    assert_eq!(row.spell_visual_id, 11_704);
    assert_eq!(row.probability, 1.0);
    assert_eq!(row.flags, 0);
    assert_eq!(row.priority, 1);
    assert_eq!(row.spell_icon_file_id, 0);
    assert_eq!(row.active_icon_file_id, 0);
    assert_eq!(row.viewer_unit_condition_id, 0);
    assert_eq!(row.viewer_player_condition_id, 0);
    assert_eq!(row.caster_unit_condition_id, 0);
    assert_eq!(row.caster_player_condition_id, 0);
    assert_eq!(row.spell_id, 53_117);
}

#[test]
fn spell_name_removals_apply_to_shared_effective_store() {
    let mut store = SpellNameStore::from_entries([
        SpellNameEntry {
            id: 100,
            name: String::new(),
        },
        SpellNameEntry {
            id: 200,
            name: String::new(),
        },
    ]);
    store.table_hash_like_cpp = Some(0xAABB_CCDD);
    let removals = crate::Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
        (0xAABB_CCDD, 100, 2),
        (0x1122_3344, 200, 2),
    ]);

    assert_eq!(
        store
            .apply_hotfix_removals_like_cpp(&removals)
            .expect("synthetic table hash"),
        1
    );
    assert!(store.get(100).is_none());
    assert_eq!(store.get(200).map(|entry| entry.name.as_str()), Some(""));
}

#[test]
fn target_restrictions_resolve_only_the_active_difficulty_chain_like_cpp() {
    let row = |id, difficulty_id, target_creature_type| SpellTargetRestrictionsEntry {
        id,
        difficulty_id,
        cone_degrees: 0.0,
        max_targets: 0,
        max_target_level: 0,
        target_creature_type,
        targets: 0,
        width: 0.0,
        spell_id: 100,
    };
    let store = SpellTargetRestrictionsStore::from_entries([
        row(1, 0, 1 << (3 - 1)),
        row(2, 2, 1 << (7 - 1)),
        row(9, 2, 1 << (8 - 1)),
    ]);

    assert_eq!(
        store
            .resolved_for_difficulty_chain_like_cpp(100, [2, 0])
            .map(|entry| entry.id),
        Some(9)
    );
    assert_eq!(
        store
            .resolved_for_difficulty_chain_like_cpp(100, [3, 0])
            .map(|entry| entry.id),
        Some(1)
    );
    assert!(
        store
            .resolved_for_difficulty_chain_like_cpp(100, [3])
            .is_none()
    );
}

#[test]
fn aura_restrictions_resolve_active_difficulty_then_fallback_like_cpp() {
    let row = |id, difficulty_id, caster_aura_spell| SpellAuraRestrictionsEntry {
        id,
        difficulty_id,
        caster_aura_state: 0,
        target_aura_state: 0,
        exclude_caster_aura_state: 0,
        exclude_target_aura_state: 0,
        caster_aura_spell,
        target_aura_spell: 0,
        exclude_caster_aura_spell: 0,
        exclude_target_aura_spell: 0,
        spell_id: 100,
    };
    let store =
        SpellAuraRestrictionsStore::from_entries([row(3, 2, 30), row(1, 0, 10), row(2, 2, 20)]);

    assert_eq!(
        store
            .resolved_for_difficulty_chain_like_cpp(100, [2, 0])
            .map(|entry| (entry.id, entry.caster_aura_spell)),
        Some((3, 30))
    );
    assert_eq!(
        store
            .resolved_for_difficulty_chain_like_cpp(100, [3, 0])
            .map(|entry| entry.id),
        Some(1)
    );
    assert!(
        store
            .resolved_for_difficulty_chain_like_cpp(100, [3])
            .is_none()
    );
}

#[test]
fn casting_requirements_resolve_cpp_record_order_deterministically() {
    let row = |id, spell_id, required_areas_id| SpellCastingRequirementsEntry {
        id,
        spell_id,
        facing_caster_flags: 0,
        min_faction_id: 0,
        min_reputation: 0,
        required_areas_id,
        required_aura_vision: 0,
        requires_spell_focus: 0,
    };
    let store = SpellCastingRequirementsStore::from_entries([
        row(9, 100, 90),
        row(3, 100, 30),
        row(7, 200, 70),
    ]);

    assert_eq!(
        store
            .entry_for_spell_id_like_cpp(100)
            .map(|entry| (entry.id, entry.required_areas_id)),
        Some((9, 90))
    );
    assert!(store.entry_for_spell_id_like_cpp(300).is_none());
}

#[test]
fn target_creature_mask_preserves_cpp_signed_integer_promotion() {
    let entry = SpellTargetRestrictionsEntry {
        id: 1,
        difficulty_id: 0,
        cone_degrees: 0.0,
        max_targets: 0,
        max_target_level: 0,
        target_creature_type: i16::MIN,
        targets: 0,
        width: 0.0,
        spell_id: 100,
    };

    assert_eq!(entry.target_creature_type_mask_like_cpp(), 0xFFFF_8000);
}
