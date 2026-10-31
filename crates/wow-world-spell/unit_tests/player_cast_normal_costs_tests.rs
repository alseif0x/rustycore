use super::*;

use wow_data::spell_db2::{
    SpellCategoriesEntry, SpellCategoryEntry, SpellCategoryStore, SpellClassOptionsEntry,
    SpellClassOptionsStore, SpellCooldownsEntry, SpellEffectDb2Store, SpellInterruptsStore,
    SpellMiscEntry, SpellMiscStore, SpellPowerDifficultyStore, SpellPowerStore,
    SpellReagentsCurrencyEntry, SpellReagentsEntry, SpellShapeshiftStore,
};
use wow_data::{DifficultyEntry, DifficultyStore, EffectiveCoreSpellDb2StoresLikeCpp};
use wow_data::{ItemEffectEntry, ItemEffectStore};
use wow_entities::{Player, SpellHistory, TRIGGERED_CAST_DIRECTLY_LIKE_CPP};

fn currency_cost_store() -> wow_data::SpellCastCostStoreLikeCpp {
    wow_data::SpellCastCostStoreLikeCpp::from_effective_rows_like_cpp(
        [],
        [SpellReagentsCurrencyEntry {
            id: 1,
            spell_id: 133,
            currency_types_id: 395,
            currency_count: 5,
        }],
    )
}

fn reagent_cost_store(entry_id: i32, count: i16) -> wow_data::SpellCastCostStoreLikeCpp {
    wow_data::SpellCastCostStoreLikeCpp::from_effective_rows_like_cpp(
        [SpellReagentsEntry {
            id: 1,
            spell_id: 133,
            reagent: [entry_id, 0, 0, 0, 0, 0, 0, 0],
            reagent_count: [count, 0, 0, 0, 0, 0, 0, 0],
        }],
        [],
    )
}

fn complete_difficulty_zero_store(
    attributes: [u32; 15],
    category_id: i16,
    charge_category_id: i16,
    cooldown_ms: i32,
    category_cooldown_ms: i32,
    start_recovery_ms: i32,
) -> (SpellStore, SpellCategoryStore) {
    let spell_id = 133_u32;
    let spell_store = wow_world_test_support::spell_catalog::canonical_spell_store([(spell_id, 0)])
        .hydrate_effective_core_db2_like_cpp(EffectiveCoreSpellDb2StoresLikeCpp::new(
            wow_data::SpellCategoriesStore::from_entries([SpellCategoriesEntry {
                id: 1,
                difficulty_id: 0,
                category: category_id,
                defense_type: 0,
                dispel_type: 0,
                mechanic: 0,
                prevention_type: 0,
                start_recovery_category: 12,
                charge_category: charge_category_id,
                spell_id,
            }]),
            SpellMiscStore::from_entries([SpellMiscEntry {
                id: 1,
                spell_id,
                difficulty_id: 0,
                attributes: attributes.map(|word| word as i32),
                ..Default::default()
            }]),
            SpellEffectDb2Store::from_entries([]),
            SpellShapeshiftStore::from_entries([]),
            SpellInterruptsStore::from_entries([]),
            wow_data::SpellCastTimesStore::from_entries([]),
            wow_data::SpellCooldownsStore::from_entries([SpellCooldownsEntry {
                id: 1,
                difficulty_id: 0,
                category_recovery_time: category_cooldown_ms,
                recovery_time: cooldown_ms,
                start_recovery_time: start_recovery_ms,
                spell_id,
            }]),
            wow_data::SpellCastingRequirementsStore::from_entries([]),
            SpellPowerStore::from_entries([]),
            SpellPowerDifficultyStore::from_entries([]),
        ));
    let category_store = SpellCategoryStore::from_entries([
        SpellCategoryEntry {
            id: category_id.max(0) as u32,
            name: String::new(),
            flags: 0,
            uses_per_week: 0,
            max_charges: 0,
            charge_recovery_time: 0,
            type_mask: 0,
        },
        SpellCategoryEntry {
            id: charge_category_id.max(0) as u32,
            name: String::new(),
            flags: 0,
            uses_per_week: 0,
            max_charges: 3,
            charge_recovery_time: 30_000,
            type_mask: 0,
        },
    ]);
    (spell_store, category_store)
}

fn difficulty_zero() -> DifficultyStore {
    DifficultyStore::from_entries([DifficultyEntry {
        id: 0,
        instance_type: 0,
        flags: 0,
        fallback_difficulty_id: 0,
        toggle_difficulty_id: 0,
    }])
}

fn facts<'a>(
    spell: &'a SpellInfo,
    costs: &'a wow_data::SpellCastCostStoreLikeCpp,
) -> ServerNormalCastCostFactsLikeCpp<'a> {
    ServerNormalCastCostFactsLikeCpp {
        spell,
        metadata: SpellCastMetadata {
            actual_spell_difficulty_id_like_cpp: Some(0),
            server_normal_cast_like_cpp: true,
            ..Default::default()
        },
        costs: Some(costs),
        physical_item: None,
        physical_item_state_complete: true,
        inventory_authority_complete: false,
        item_effect_store: None,
        cast_item_no_reagent_cost: None,
        no_reagent_while_preparing: false,
        no_reagent_aura_authority_complete: true,
        no_reagent_cost_mask: [0; 4],
        spell_store: None,
        spell_categories: None,
        class_options: None,
        difficulties: None,
        reagent_counts: None,
        currency_quantities: Some(HashMap::from([(395, 4)])),
        currency_authority_complete: true,
    }
}

fn spell() -> SpellInfo {
    SpellInfo {
        spell_id: 133,
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

#[test]
fn currency_only_cost_uses_all_source_no_reagent_gates_without_item_authority() {
    let spell = spell();
    let costs = currency_cost_store();

    let mut preparation = facts(&spell, &costs);
    preparation.no_reagent_while_preparing = true;
    preparation.currency_authority_complete = false;
    preparation.currency_quantities = None;
    assert_eq!(
        server_normal_cost_result_like_cpp(preparation),
        ServerNormalCostResultLikeCpp::Affordable
    );

    let mut aura_mask = facts(&spell, &costs);
    aura_mask.no_reagent_cost_mask = [0x20, 0, 0, 0];
    let class_options = SpellClassOptionsStore::from_entries([SpellClassOptionsEntry {
        id: 1,
        spell_id: 133,
        modal_next_spell: 0,
        spell_class_set: 0,
        spell_class_mask: [0x20, 0, 0, 0],
    }]);
    aura_mask.class_options = Some(&class_options);
    let difficulties = difficulty_zero();
    aura_mask.difficulties = Some(&difficulties);
    assert_eq!(
        server_normal_cost_result_like_cpp(aura_mask),
        ServerNormalCostResultLikeCpp::Affordable
    );

    let mut unmatched_aura_mask = facts(&spell, &costs);
    unmatched_aura_mask.no_reagent_cost_mask = [0x20, 0, 0, 0];
    let unmatched_class_options = SpellClassOptionsStore::from_entries([SpellClassOptionsEntry {
        id: 1,
        spell_id: 133,
        modal_next_spell: 0,
        spell_class_set: 0,
        spell_class_mask: [0x1, 0, 0, 0],
    }]);
    unmatched_aura_mask.class_options = Some(&unmatched_class_options);
    unmatched_aura_mask.difficulties = Some(&difficulties);
    unmatched_aura_mask.inventory_authority_complete = true;
    assert_eq!(
        server_normal_cost_result_like_cpp(unmatched_aura_mask),
        ServerNormalCostResultLikeCpp::MissingCurrency(395),
        "unmatched active aura class mask does not exempt currency costs"
    );

    let mut ordinary = facts(&spell, &costs);
    ordinary.inventory_authority_complete = true;
    ordinary.currency_quantities = Some(HashMap::from([(395, 4)]));
    assert_eq!(
        server_normal_cost_result_like_cpp(ordinary),
        ServerNormalCostResultLikeCpp::MissingCurrency(395)
    );

    let mut direct = facts(&spell, &costs);
    direct.metadata.trigger_cast_flags_like_cpp = TRIGGERED_CAST_DIRECTLY_LIKE_CPP;
    direct.inventory_authority_complete = true;
    direct.currency_quantities = Some(HashMap::from([(395, 4)]));
    assert_eq!(
        server_normal_cost_result_like_cpp(direct),
        ServerNormalCostResultLikeCpp::MissingCurrency(395),
        "DIRECT does not imply ignore reagent or currency costs"
    );

    let mut ignored = facts(&spell, &costs);
    ignored.metadata.trigger_cast_flags_like_cpp =
        wow_entities::TRIGGERED_IGNORE_POWER_AND_REAGENT_COST_LIKE_CPP;
    ignored.costs = None;
    ignored.inventory_authority_complete = false;
    ignored.currency_authority_complete = false;
    assert_eq!(
        server_normal_cost_result_like_cpp(ignored),
        ServerNormalCostResultLikeCpp::Affordable,
        "IGNORE_POWER_AND_REAGENT_COST bypasses both item and currency rows"
    );

    let mut missing_aura_authority = facts(&spell, &costs);
    missing_aura_authority.no_reagent_aura_authority_complete = false;
    assert_eq!(
        server_normal_cost_result_like_cpp(missing_aura_authority),
        ServerNormalCostResultLikeCpp::Unavailable(
            "complete untombstoned Player aura authority unavailable"
        )
    );
}

#[test]
fn no_reagent_physical_item_skips_currency_cost_with_current_item_identity() {
    let spell = spell();
    let costs = currency_cost_store();
    let item_guid = wow_core::ObjectGuid::create_item(1, 44);
    let mut item_cost = facts(&spell, &costs);
    item_cost.inventory_authority_complete = true;
    item_cost.physical_item = Some(PhysicalCastItemLikeCpp {
        guid: item_guid,
        entry_id: 700,
        spell_charges: [0; wow_entities::MAX_ITEM_SPELLS],
    });
    item_cost.metadata.cast_item_guid = Some(item_guid);
    item_cost.metadata.cast_item_entry = Some(700);
    item_cost.cast_item_no_reagent_cost = Some(true);
    item_cost.currency_authority_complete = false;
    item_cost.currency_quantities = None;

    assert_eq!(
        server_normal_cost_result_like_cpp(item_cost),
        ServerNormalCostResultLikeCpp::Affordable
    );
}

#[test]
fn currency_debit_requires_complete_inventory_and_currency_save_authority() {
    let spell = spell();
    let costs = currency_cost_store();

    let mut available = facts(&spell, &costs);
    available.inventory_authority_complete = true;
    available.currency_quantities = Some(HashMap::from([(395, 7)]));
    assert_eq!(
        server_normal_cost_plan_like_cpp(available)
            .unwrap()
            .currency_debits,
        [(395, 5)]
    );

    let mut missing_inventory = facts(&spell, &costs);
    missing_inventory.currency_quantities = Some(HashMap::from([(395, 7)]));
    assert_eq!(
        server_normal_cost_result_like_cpp(missing_inventory),
        ServerNormalCostResultLikeCpp::Unavailable(
            "complete Player inventory authority unavailable for full-save cost capture"
        )
    );

    let mut missing_currency_authority = facts(&spell, &costs);
    missing_currency_authority.inventory_authority_complete = true;
    missing_currency_authority.currency_authority_complete = false;
    assert_eq!(
        server_normal_cost_result_like_cpp(missing_currency_authority),
        ServerNormalCostResultLikeCpp::Unavailable(
            "complete Player currency authority unavailable for currency debit"
        )
    );
}

#[test]
fn physical_item_identity_is_required_even_when_effective_cost_rows_are_empty() {
    let spell = spell();
    let no_costs = wow_data::SpellCastCostStoreLikeCpp::from_effective_rows_like_cpp([], []);
    let mut facts = facts(&spell, &no_costs);
    facts.metadata.cast_item_guid = Some(wow_core::ObjectGuid::create_item(1, 99));
    facts.metadata.cast_item_entry = Some(700);
    assert_eq!(
        server_normal_cost_result_like_cpp(facts),
        ServerNormalCostResultLikeCpp::Unavailable("physical cast item identity unavailable")
    );
}

#[test]
fn reagent_overlap_adds_one_for_expendable_physical_item_and_clears_pointer() {
    let spell = spell();
    let costs = reagent_cost_store(700, 1);
    let item_guid = wow_core::ObjectGuid::create_item(1, 44);
    let effects = ItemEffectStore::from_entries([ItemEffectEntry {
        id: 1,
        legacy_slot_index: 0,
        trigger_type: 0,
        charges: -1,
        cooldown_msec: -1,
        category_cooldown_msec: -1,
        spell_category_id: 0,
        spell_id: 1000,
        chr_specialization_id: 0,
        parent_item_id: 700,
    }]);
    let mut overlap = facts(&spell, &costs);
    overlap.physical_item_state_complete = true;
    overlap.inventory_authority_complete = true;
    overlap.physical_item = Some(PhysicalCastItemLikeCpp {
        guid: item_guid,
        entry_id: 700,
        spell_charges: [-1, 0, 0, 0, 0],
    });
    overlap.metadata.cast_item_guid = Some(item_guid);
    overlap.metadata.cast_item_entry = Some(700);
    overlap.metadata.cast_item_level = Some(17);
    overlap.item_effect_store = Some(&effects);
    overlap.cast_item_no_reagent_cost = Some(false);
    overlap.reagent_counts = Some(HashMap::from([(700, 2)]));

    let plan = server_normal_cost_plan_like_cpp(overlap).unwrap();
    assert_eq!(plan.reagent_debits, [(700, 2)]);
    assert!(plan.clears_cast_item_before_go);
    // The execution adapter clears only GUID and entry. Item level is the
    // already computed source value and remains in the cast metadata.
    let mut metadata = SpellCastMetadata {
        cast_item_guid: Some(item_guid),
        cast_item_entry: Some(700),
        cast_item_level: Some(17),
        ..Default::default()
    };
    if plan.clears_cast_item_before_go {
        metadata.cast_item_guid = None;
        metadata.cast_item_entry = None;
    }
    assert_eq!(metadata.cast_item_guid, None);
    assert_eq!(metadata.cast_item_entry, None);
    assert_eq!(metadata.cast_item_level, Some(17));

    let mut insufficient = facts(&spell, &costs);
    insufficient.inventory_authority_complete = true;
    insufficient.physical_item = Some(PhysicalCastItemLikeCpp {
        guid: item_guid,
        entry_id: 700,
        spell_charges: [-1, 0, 0, 0, 0],
    });
    insufficient.metadata.cast_item_guid = Some(item_guid);
    insufficient.metadata.cast_item_entry = Some(700);
    insufficient.item_effect_store = Some(&effects);
    insufficient.cast_item_no_reagent_cost = Some(false);
    insufficient.reagent_counts = Some(HashMap::from([(700, 1)]));
    assert_eq!(
        server_normal_cost_result_like_cpp(insufficient),
        ServerNormalCostResultLikeCpp::MissingReagent(700)
    );
}

#[test]
fn cast_item_effect_order_uses_last_charge_bearing_effect_for_consumption() {
    let item_guid = wow_core::ObjectGuid::create_item(1, 45);
    let effects = ItemEffectStore::from_entries([
        ItemEffectEntry {
            id: 3,
            legacy_slot_index: 1,
            trigger_type: 0,
            charges: 1,
            cooldown_msec: -1,
            category_cooldown_msec: -1,
            spell_category_id: 0,
            spell_id: 1002,
            chr_specialization_id: 0,
            parent_item_id: 700,
        },
        ItemEffectEntry {
            id: 2,
            legacy_slot_index: 0,
            trigger_type: 0,
            charges: -1,
            cooldown_msec: -1,
            category_cooldown_msec: -1,
            spell_category_id: 0,
            spell_id: 1001,
            chr_specialization_id: 0,
            parent_item_id: 700,
        },
        ItemEffectEntry {
            id: 1,
            legacy_slot_index: 2,
            trigger_type: 0,
            charges: 1,
            cooldown_msec: -1,
            category_cooldown_msec: -1,
            spell_category_id: 0,
            spell_id: 1000,
            chr_specialization_id: 0,
            parent_item_id: 700,
        },
    ]);
    let item = PhysicalCastItemLikeCpp {
        guid: item_guid,
        entry_id: 700,
        spell_charges: [-1, 2, 2, 0, 0],
    };
    let ServerNormalCastItemUsePlanResultLikeCpp::Planned(plan) =
        server_normal_cast_item_use_plan_like_cpp(Some(item), Some(&effects), Some(1), 0)
    else {
        panic!("loaded physical cast item should produce a take plan");
    };
    assert_eq!(plan.updated_charges, [0, 1, 1, 0, 0]);
    assert!(plan.writes_charges);
    assert!(plan.marks_item_changed);
    assert!(
        !plan.consumes_one_item,
        "the final effect leaves one charge"
    );

    let consume_effects = ItemEffectStore::from_entries([
        ItemEffectEntry {
            id: 1,
            legacy_slot_index: 2,
            trigger_type: 0,
            charges: 1,
            cooldown_msec: -1,
            category_cooldown_msec: -1,
            spell_category_id: 0,
            spell_id: 1000,
            chr_specialization_id: 0,
            parent_item_id: 700,
        },
        ItemEffectEntry {
            id: 2,
            legacy_slot_index: 0,
            trigger_type: 0,
            charges: -1,
            cooldown_msec: -1,
            category_cooldown_msec: -1,
            spell_category_id: 0,
            spell_id: 1001,
            chr_specialization_id: 0,
            parent_item_id: 700,
        },
    ]);
    let mut final_charge_item = item;
    final_charge_item.spell_charges = [-1, 0, 2, 0, 0];
    let ServerNormalCastItemUsePlanResultLikeCpp::Planned(consume) =
        server_normal_cast_item_use_plan_like_cpp(
            Some(final_charge_item),
            Some(&consume_effects),
            Some(1),
            0,
        )
    else {
        panic!("loaded physical cast item should produce a take plan");
    };
    assert!(
        consume.consumes_one_item,
        "the last effect reaches zero charges"
    );

    assert_eq!(
        server_normal_cast_item_use_plan_like_cpp(
            Some(final_charge_item),
            Some(&consume_effects),
            Some(1),
            wow_entities::TRIGGERED_IGNORE_CAST_ITEM_LIKE_CPP,
        ),
        ServerNormalCastItemUsePlanResultLikeCpp::NoItem
    );
    assert!(matches!(
        server_normal_cast_item_use_plan_like_cpp(
            Some(final_charge_item),
            Some(&consume_effects),
            Some(1),
            wow_entities::TRIGGERED_IGNORE_POWER_AND_REAGENT_COST_LIKE_CPP,
        ),
        ServerNormalCastItemUsePlanResultLikeCpp::Planned(plan)
            if plan.consumes_one_item
    ));
    assert!(matches!(
        server_normal_cast_item_use_plan_like_cpp(
            Some(final_charge_item),
            Some(&consume_effects),
            Some(20),
            0,
        ),
        ServerNormalCastItemUsePlanResultLikeCpp::Planned(plan)
            if plan.marks_item_changed
                && !plan.writes_charges
                && plan.updated_charges == final_charge_item.spell_charges
                && plan.consumes_one_item
    ));
    assert!(!server_normal_take_cast_item_before_effects_like_cpp(
        false, 0
    ));
    assert!(!server_normal_take_cast_item_before_effects_like_cpp(
        true,
        SPELL_ATTR1_IS_CHANNELLED_LIKE_CPP
    ));
    assert!(server_normal_take_cast_item_before_effects_like_cpp(
        true, 0
    ));
}

#[test]
fn cast_item_cooldown_uses_first_matching_record_not_lowest_charge_slot() {
    let spell = spell();
    let (store, categories) = complete_difficulty_zero_store([0; 15], 8, 0, 4_000, 0, 0);
    let item_guid = wow_core::ObjectGuid::create_item(1, 46);
    let effects = ItemEffectStore::from_entries([
        ItemEffectEntry {
            id: 50,
            legacy_slot_index: 0,
            trigger_type: 0,
            charges: 0,
            cooldown_msec: 50_000,
            category_cooldown_msec: 60_000,
            spell_category_id: 19,
            spell_id: 133,
            chr_specialization_id: 0,
            parent_item_id: 700,
        },
        ItemEffectEntry {
            id: 10,
            legacy_slot_index: 4,
            trigger_type: 0,
            charges: 0,
            cooldown_msec: 7_000,
            category_cooldown_msec: 9_000,
            spell_category_id: 11,
            spell_id: 133,
            chr_specialization_id: 0,
            parent_item_id: 700,
        },
    ]);
    let plan = server_normal_cooldown_plan_like_cpp(ServerNormalCooldownFactsLikeCpp {
        spell: &spell,
        metadata: SpellCastMetadata {
            actual_spell_difficulty_id_like_cpp: Some(0),
            cast_item_entry: Some(700),
            cast_item_guid: Some(item_guid),
            ..Default::default()
        },
        spell_store: Some(&store),
        difficulties: None,
        item_effect_store: Some(&effects),
        spell_categories: Some(&categories),
        cast_item_is_potion: Some(false),
    })
    .expect("complete actual key and item effect catalog resolve cooldown");

    assert_eq!(
        plan.mutation,
        ServerNormalCooldownMutationLikeCpp::StartCooldown {
            spell_id: 133,
            item_id: 700,
            cooldown_ms: 7_000,
            category_id: 11,
            category_cooldown_ms: 9_000,
        },
        "the first matching spell in Source record order wins"
    );
}

#[test]
fn native_cooldown_is_mutated_before_the_effect_and_flags_remain_independent() {
    let attrs = [0; 15];
    let (store, categories) = complete_difficulty_zero_store(attrs, 8, 0, 4_000, 1_500, 1_500);
    let spell = spell();
    let facts = ServerNormalCooldownFactsLikeCpp {
        spell: &spell,
        metadata: SpellCastMetadata {
            actual_spell_difficulty_id_like_cpp: Some(0),
            server_normal_cast_like_cpp: true,
            ..Default::default()
        },
        spell_store: Some(&store),
        difficulties: None,
        item_effect_store: None,
        spell_categories: Some(&categories),
        cast_item_is_potion: Some(false),
    };
    let plan = server_normal_cooldown_plan_like_cpp(facts.clone()).unwrap();
    assert!(matches!(
        plan.mutation,
        ServerNormalCooldownMutationLikeCpp::StartCooldown {
            spell_id: 133,
            cooldown_ms: 4_000,
            category_id: 8,
            category_cooldown_ms: 1_500,
            ..
        }
    ));
    let mut history = SpellHistory {
        cooldowns_loaded: true,
        ..Default::default()
    };
    let mut events = Vec::new();
    let now_ms = wow_world_core::session::unix_now_ms_like_cpp();
    assert!(apply_server_normal_cooldown_plan_like_cpp(
        &mut history,
        plan,
        now_ms,
    ));
    events.push("native cooldown");
    assert!(history.has_cooldown_membership_like_cpp(133, 8));
    let cooldown = history
        .cooldown(133)
        .expect("native history stores cooldown");
    let saved_unix_secs = cooldown.cooldown_end_ms / 1_000;
    assert!(saved_unix_secs > now_ms / 1_000);
    assert!(
        saved_unix_secs > 1_700_000_000,
        "full-save seconds remain Unix time"
    );
    history.update_at_unix_nanos(u128::from(cooldown.cooldown_end_ms - 1) * 1_000_000);
    assert!(history.has_cooldown_membership_like_cpp(133, 8));
    history.update_at_unix_nanos(u128::from(cooldown.cooldown_end_ms + 1) * 1_000_000);
    assert!(!history.has_cooldown_membership_like_cpp(133, 8));
    events.push("effect");
    assert_eq!(events, ["native cooldown", "effect"]);

    let direct = ServerNormalCooldownFactsLikeCpp {
        metadata: SpellCastMetadata {
            actual_spell_difficulty_id_like_cpp: Some(0),
            trigger_cast_flags_like_cpp: TRIGGERED_CAST_DIRECTLY_LIKE_CPP,
            ..Default::default()
        },
        ..facts.clone()
    };
    assert!(
        matches!(
            server_normal_cooldown_plan_like_cpp(direct)
                .unwrap()
                .mutation,
            ServerNormalCooldownMutationLikeCpp::StartCooldown { .. }
        ),
        "DIRECT preserves ordinary cooldown behavior"
    );

    let ignore_cooldown = ServerNormalCooldownFactsLikeCpp {
        metadata: SpellCastMetadata {
            actual_spell_difficulty_id_like_cpp: Some(0),
            trigger_cast_flags_like_cpp:
                wow_entities::TRIGGERED_IGNORE_SPELL_AND_CATEGORY_CD_LIKE_CPP,
            ..Default::default()
        },
        ..facts
    };
    let skipped = server_normal_cooldown_plan_like_cpp(ignore_cooldown).unwrap();
    assert_eq!(skipped.mutation, ServerNormalCooldownMutationLikeCpp::None);
}

#[test]
fn native_cooldown_charge_and_start_cast_flags_select_distinct_phases() {
    let mut attributes = [0; 15];
    attributes[12] = SPELL_ATTR12_START_COOLDOWN_ON_CAST_START_LIKE_CPP;
    let (store, categories) = complete_difficulty_zero_store(attributes, 8, 9, 4_000, 1_500, 1_500);
    let spell = spell();
    let facts = ServerNormalCooldownFactsLikeCpp {
        spell: &spell,
        metadata: SpellCastMetadata {
            actual_spell_difficulty_id_like_cpp: Some(0),
            server_normal_cast_like_cpp: true,
            ..Default::default()
        },
        spell_store: Some(&store),
        difficulties: None,
        item_effect_store: None,
        spell_categories: Some(&categories),
        cast_item_is_potion: Some(false),
    };
    let plan = server_normal_cooldown_plan_like_cpp(facts.clone()).unwrap();
    assert!(plan.starts_at_cast_start);
    assert!(matches!(
        plan.mutation,
        ServerNormalCooldownMutationLikeCpp::ConsumeCharge {
            charge_category_id: 9,
            recovery_ms: 30_000,
            max_charges: 3,
        }
    ));
    let mut history = SpellHistory {
        charges_loaded: true,
        ..Default::default()
    };
    assert!(apply_server_normal_cooldown_plan_like_cpp(
        &mut history,
        plan,
        10_000,
    ));
    assert_eq!(history.consumed_charges(9), 1);
}

#[test]
fn loaded_native_cooldown_membership_blocks_after_end_until_history_maintenance() {
    let (store, categories) = complete_difficulty_zero_store([0; 15], 8, 0, 4_000, 0, 0);
    let difficulties = difficulty_zero();
    let serverside = ServersideSpellStoreLikeCpp::default();
    let mut player = Player::new(None, false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(wow_core::ObjectGuid::create_player(1, 904));
    player.set_player_aura_authority_complete_like_cpp(true);
    let history = &mut player.unit_mut().subsystems_mut().spells.history;
    history.cooldowns_loaded = true;
    let expired_end_ms = wow_world_core::session::unix_now_ms_like_cpp().saturating_sub(1);
    assert!(history.add_cooldown(133, 0, expired_end_ms, 8, expired_end_ms, false));
    assert!(store.contains_spell_info_exact_like_cpp(133, 0));
    assert!(
        !wow_world_spell::active_spell_cooldown_aura_affects_like_cpp(
            &player,
            133,
            0,
            &store,
            &serverside,
            None,
            Some(&difficulties),
        )
        .expect("complete aura and actual spell catalogs prove no active ignore-cooldown aura")
    );

    assert!(
        !server_normal_cooldown_ready_like_cpp(
            &player,
            &spell(),
            0,
            SpellCastMetadata {
                actual_spell_difficulty_id_like_cpp: Some(0),
                server_normal_cast_like_cpp: true,
                ..Default::default()
            },
            &store,
            &serverside,
            None,
            Some(&difficulties),
            None,
            Some(&categories),
        )
        .expect("loaded SpellHistory and current catalogs resolve readiness")
    );
    assert_eq!(
        player
            .unit()
            .subsystems()
            .spells
            .history
            .remaining_cooldown_ms(133, 8, wow_world_core::session::unix_now_ms_like_cpp()),
        0,
        "the loaded row can be expired while still blocking before maintenance"
    );
}

#[test]
fn source_event_and_passive_cooldowns_do_not_start_on_normal_cast() {
    let spell = spell();
    let mut attributes = [0; 15];
    attributes[0] = SPELL_ATTR0_COOLDOWN_ON_EVENT_LIKE_CPP;
    let (event_store, categories) =
        complete_difficulty_zero_store(attributes, 8, 0, 4_000, 1_500, 1_500);
    let event_facts = ServerNormalCooldownFactsLikeCpp {
        spell: &spell,
        metadata: SpellCastMetadata {
            actual_spell_difficulty_id_like_cpp: Some(0),
            ..Default::default()
        },
        spell_store: Some(&event_store),
        difficulties: None,
        item_effect_store: None,
        spell_categories: Some(&categories),
        cast_item_is_potion: Some(false),
    };
    assert_eq!(
        server_normal_cooldown_plan_like_cpp(event_facts)
            .unwrap()
            .mutation,
        ServerNormalCooldownMutationLikeCpp::None
    );

    let mut passive_attributes = [0; 15];
    passive_attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
    let (passive_store, categories) =
        complete_difficulty_zero_store(passive_attributes, 8, 0, 4_000, 1_500, 1_500);
    let passive_facts = ServerNormalCooldownFactsLikeCpp {
        spell: &spell,
        metadata: SpellCastMetadata {
            actual_spell_difficulty_id_like_cpp: Some(0),
            ..Default::default()
        },
        spell_store: Some(&passive_store),
        difficulties: None,
        item_effect_store: None,
        spell_categories: Some(&categories),
        cast_item_is_potion: Some(false),
    };
    assert_eq!(
        server_normal_cooldown_plan_like_cpp(passive_facts)
            .unwrap()
            .mutation,
        ServerNormalCooldownMutationLikeCpp::None
    );
}
