use std::cell::Cell;
use std::collections::{BTreeMap, HashMap, HashSet};

use super::{
    ItemBonusData, ItemBonusDataError, ItemBonusEntry, ItemBonusSource, ItemBonusTemplateFacts,
    ItemOffsetCurveFacts, ItemScalingConfigFacts, ItemSquishEraFacts,
};

#[derive(Default)]
struct Sources {
    templates: HashMap<u32, ItemBonusTemplateFacts>,
    azerite: HashMap<u32, u32>,
    lists: BTreeMap<u32, Vec<ItemBonusEntry>>,
    effects: HashSet<u32>,
    scaling: HashMap<u32, ItemScalingConfigFacts>,
    offsets: HashMap<u32, ItemOffsetCurveFacts>,
    squish: BTreeMap<u32, ItemSquishEraFacts>,
    squish_bound: u32,
    realm_patch: Option<i32>,
    fail_list: Option<u32>,
    fail_template: bool,
    fail_azerite: bool,
    fail_effect: bool,
    fail_scaling: bool,
    fail_offset: bool,
    fail_squish_bound: bool,
    fail_squish_index: Option<u32>,
    squish_bound_reads: Cell<usize>,
    squish_index_reads: Cell<usize>,
}

impl Sources {
    fn with_template() -> Self {
        let mut sources = Self::default();
        sources.templates.insert(
            7,
            ItemBonusTemplateFacts {
                item_id: 7,
                quality: 3,
                item_level: 100,
                required_level: 60,
                item_stat_type: [5; 10],
                stat_percent_editor: [2; 10],
                item_stat_socket_cost_multiplier: [0.5; 10],
                socket_color: [8, 0, 4],
                bonding: 2,
                content_tuning_id: 11,
                player_level_to_item_level_curve_id: 12,
                item_level_offset_curve_id: 13,
                item_level_offset_item_level: 14,
                item_squish_era_id: 15,
                limit_category: 16,
                effect_ids: vec![0, 70],
                can_disenchant: true,
                can_scrap: false,
                can_salvage: true,
                can_recraft: false,
                cannot_trade_bind_on_pickup: true,
            },
        );
        sources.effects.extend([0, 70, 71, 72]);
        sources
    }

    fn add_list(&mut self, id: u32, rows: impl IntoIterator<Item = ItemBonusEntry>) {
        self.lists.insert(id, rows.into_iter().collect());
    }
}

impl ItemBonusSource for Sources {
    type Error = &'static str;

    fn template_facts(&self, item_id: u32) -> Result<Option<ItemBonusTemplateFacts>, Self::Error> {
        if self.fail_template {
            return Err("template source unavailable");
        }
        Ok(self.templates.get(&item_id).cloned())
    }

    fn azerite_tier_unlock_set_id(&self, item_id: u32) -> Result<Option<u32>, Self::Error> {
        if self.fail_azerite {
            return Err("Azerite source unavailable");
        }
        Ok(self.azerite.get(&item_id).copied())
    }

    fn bonus_list(
        &self,
        list_id: u32,
    ) -> Result<impl Iterator<Item = ItemBonusEntry>, Self::Error> {
        if self.fail_list == Some(list_id) {
            return Err("bonus list source unavailable");
        }
        Ok(self.lists.get(&list_id).into_iter().flatten().copied())
    }

    fn has_item_effect(&self, effect_id: u32) -> Result<bool, Self::Error> {
        if self.fail_effect {
            return Err("effect source unavailable");
        }
        Ok(self.effects.contains(&effect_id))
    }

    fn scaling_config(
        &self,
        config_id: u32,
    ) -> Result<Option<ItemScalingConfigFacts>, Self::Error> {
        if self.fail_scaling {
            return Err("scaling source unavailable");
        }
        Ok(self.scaling.get(&config_id).copied())
    }

    fn offset_curve(&self, curve_id: u32) -> Result<Option<ItemOffsetCurveFacts>, Self::Error> {
        if self.fail_offset {
            return Err("offset source unavailable");
        }
        Ok(self.offsets.get(&curve_id).copied())
    }

    fn item_squish_era_index_bound(&self) -> Result<u32, Self::Error> {
        self.squish_bound_reads
            .set(self.squish_bound_reads.get() + 1);
        if self.fail_squish_bound {
            return Err("squish bound unavailable");
        }
        Ok(self.squish_bound)
    }

    fn item_squish_era(&self, index: u32) -> Result<Option<ItemSquishEraFacts>, Self::Error> {
        self.squish_index_reads
            .set(self.squish_index_reads.get() + 1);
        if self.fail_squish_index == Some(index) {
            return Err("squish row unavailable");
        }
        Ok(self.squish.get(&index).copied())
    }

    fn actual_realm_patch(&self) -> Option<i32> {
        self.realm_patch
    }
}

fn row(bonus_type: u32, values: [i32; 4]) -> ItemBonusEntry {
    ItemBonusEntry { bonus_type, values }
}

#[test]
fn constructor_is_zeroed_and_template_initialization_resets_all_bonus_state() {
    let mut sources = Sources::with_template();
    sources.azerite.insert(7, u32::MAX);
    let template = sources.templates.get(&7).unwrap();
    let mut data = ItemBonusData::new();

    assert_eq!(data.quality(), 0);
    assert_eq!(data.repair_cost_multiplier(), 0.0);
    assert_eq!(data.relic_type(), 0);
    assert_eq!(data.gem_relic_type(), &[0; 3]);
    assert_eq!(data.item_stat_type(), &[0; 10]);

    data.initialize(template, &sources).unwrap();
    assert_eq!(data.quality(), 3);
    assert_eq!(data.item_level(), 100);
    assert_eq!(data.required_level(), 60);
    assert_eq!(data.repair_cost_multiplier(), 1.0);
    assert_eq!(data.item_stat_type(), &[5; 10]);
    assert_eq!(data.stat_percent_editor(), &[2; 10]);
    assert_eq!(data.item_stat_socket_cost_multiplier(), &[0.5; 10]);
    assert_eq!(data.socket_color(), &[8, 0, 4]);
    assert_eq!(data.gem_item_level_bonus(), &[0; 3]);
    assert_eq!(data.gem_relic_type(), &[-1; 3]);
    assert_eq!(data.gem_relic_rank_bonus(), &[0; 3]);
    assert_eq!(data.relic_type(), -1);
    assert_eq!(data.azerite_tier_unlock_set_id(), -1);
    assert_eq!(data.effects(), &[0, 70]);
    assert!(data.can_disenchant());
    assert!(!data.can_scrap());
    assert!(data.can_salvage());
    assert!(!data.can_recraft());
    assert!(data.cannot_trade_bind_on_pickup());

    data.add_bonus(1, [9, 0, 0, 0], &sources).unwrap();
    data.add_bonus(5, [123, 0, 0, 0], &sources).unwrap();
    data.add_bonus(21, [0, 0, 0, 0], &sources).unwrap();
    data.initialize(template, &sources).unwrap();
    assert_eq!(data.item_level_bonus(), 0);
    assert_eq!(data.suffix(), 0);
    assert!(data.can_disenchant());
    data.add_bonus(5, [456, 1, 0, 0], &sources).unwrap();
    assert_eq!(data.suffix(), 456);
}

#[test]
fn missing_instance_template_is_a_noop_before_bonus_list_access() {
    let mut sources = Sources::with_template();
    let mut data = ItemBonusData::new();
    data.initialize(sources.templates.get(&7).unwrap(), &sources)
        .unwrap();
    data.add_bonus(1, [14, 0, 0, 0], &sources).unwrap();
    sources.fail_list = Some(99);

    data.initialize_from_instance(404, &[99], &sources).unwrap();
    assert_eq!(data.item_level(), 100);
    assert_eq!(data.item_level_bonus(), 14);
    assert_eq!(data.socket_color(), &[8, 0, 4]);
}

#[test]
fn bonus_lists_keep_full_ids_duplicates_depth_first_order_and_prefix_on_cycle() {
    let mut sources = Sources::with_template();
    sources.add_list(
        70_000,
        [
            row(1, [1, 0, 0, 0]),
            row(50, [i32::MIN, 0, 0, 0]),
            row(1, [4, 0, 0, 0]),
            row(50, [i32::MIN, 0, 0, 0]),
        ],
    );
    sources.add_list(0x8000_0000, [row(1, [2, 0, 0, 0])]);
    sources.add_list(
        3,
        [
            row(1, [3, 0, 0, 0]),
            row(50, [3, 0, 0, 0]),
            row(1, [100, 0, 0, 0]),
        ],
    );
    let mut data = ItemBonusData::new();

    data.add_bonus_list(70_000, &sources).unwrap();
    assert_eq!(data.item_level_bonus(), 9);
    data.add_bonus_list(123_456, &sources).unwrap();
    assert_eq!(data.item_level_bonus(), 9);

    let error = data.add_bonus_list(3, &sources).unwrap_err();
    assert_eq!(error, ItemBonusDataError::BonusListCycle { list_id: 3 });
    assert_eq!(data.item_level_bonus(), 12);
}

#[test]
fn deep_acyclic_nested_lists_use_iterative_depth_first_traversal() {
    const DEPTH: u32 = 4096;
    let mut sources = Sources::with_template();
    for id in 0..DEPTH {
        sources.add_list(id, [row(50, [(id + 1) as i32, 0, 0, 0])]);
    }
    sources.add_list(DEPTH, [row(1, [1, 0, 0, 0])]);
    let mut data = ItemBonusData::new();

    data.add_bonus_list(0, &sources).unwrap();
    assert_eq!(data.item_level_bonus(), 1);
}

#[test]
fn instance_initialization_resets_then_applies_repeated_list_ids_in_order() {
    let mut sources = Sources::with_template();
    sources.add_list(1, [row(1, [2, 0, 0, 0])]);
    sources.add_list(2, [row(1, [3, 0, 0, 0])]);
    let mut data = ItemBonusData::new();
    data.add_bonus(1, [90, 0, 0, 0], &sources).unwrap();
    data.initialize_from_instance(7, &[1, 2, 1], &sources)
        .unwrap();

    assert_eq!(data.item_level(), 100);
    assert_eq!(data.item_level_bonus(), 7);
}

#[test]
fn scalar_priority_socket_override_and_unhandled_rows_match_native_rules() {
    let sources = Sources::with_template();
    let mut data = ItemBonusData::new();
    data.initialize(sources.templates.get(&7).unwrap(), &sources)
        .unwrap();

    for entry in [
        row(1, [5, 0, 0, 0]),
        row(2, [5, 3, 0, 0]),
        row(3, [4, 0, 0, 0]),
        row(3, [8, 0, 0, 0]),
        row(5, [100, 2, 0, 0]),
        row(5, [200, 2, 0, 0]),
        row(5, [300, 1, 0, 0]),
        row(6, [1, 0, 0, 0]),
        row(6, [-1, 8, 0, 0]),
        row(7, [50, 2, 0, 0]),
        row(7, [60, 2, 0, 0]),
        row(8, [4, 0, 0, 0]),
        row(10, [50, 0, 0, 0]),
        row(10, [50, 0, 0, 0]),
        row(12, [90, 2, 0, 0]),
        row(12, [91, 2, 0, 0]),
        row(12, [92, 1, 0, 0]),
        row(16, [10, 0, 0, 0]),
        row(17, [6, 0, 0, 0]),
        row(18, [22, 0, 0, 0]),
        row(19, [33, 2, 0, 0]),
        row(19, [34, 2, 0, 0]),
        row(19, [35, 1, 0, 0]),
        row(21, [0, 0, 0, 0]),
        row(22, [1, 0, 0, 0]),
        row(35, [44, 0, 0, 0]),
        row(35, [45, 0, 0, 0]),
        row(39, [0, 0, 0, 0]),
        row(41, [1, 0, 0, 0]),
        row(42, [123, 2, 0, 0]),
        row(42, [124, 2, 0, 0]),
        row(42, [125, 1, 0, 0]),
        row(43, [70_000, 2, 0, 0]),
        row(46, [0, 0, 0, 0]),
        row(47, [15, 2, 0, 0]),
        row(47, [7, 1, 0, 0]),
        row(48, [55, 66, 0, 2]),
        row(52, [0, 0, 0, 0]),
        row(53, [77, 2, 0, 0]),
        row(53, [88, 1, 0, 0]),
        row(4, [1, 2, 3, 4]),
        row(9, [1, 2, 3, 4]),
        row(14, [1, 2, 3, 4]),
        row(15, [1, 2, 3, 4]),
        row(20, [1, 2, 3, 4]),
    ] {
        data.add_bonus(entry.bonus_type, entry.values, &sources)
            .unwrap();
    }

    assert_eq!(data.item_level_bonus(), 5);
    assert_eq!(data.item_stat_type()[0], 5);
    assert_eq!(data.stat_percent_editor()[0], 5);
    assert_eq!(data.quality(), 8);
    assert_eq!(data.suffix(), 300);
    assert_eq!(data.socket_color(), &[8, 8, 4]);
    assert_eq!(data.appearance_mod_id(), 50);
    assert_eq!(data.required_level(), 64);
    assert_eq!(data.repair_cost_multiplier(), 0.25);
    assert_eq!(data.disenchant_loot_id(), 92);
    assert_eq!(data.bonding(), 7);
    assert_eq!(data.relic_type(), 6);
    assert_eq!(data.required_level_override(), 22);
    assert_eq!(data.azerite_tier_unlock_set_id(), 35);
    assert!(!data.can_disenchant());
    assert!(data.can_scrap());
    assert!(!data.can_salvage());
    assert!(data.can_recraft());
    assert!(!data.cannot_trade_bind_on_pickup());
    assert_eq!(data.limit_category(), 44);
    assert_eq!(data.item_level(), 125);
    assert_eq!(data.pvp_item_level(), 70_000_u32 as u16);
    assert_eq!(data.pvp_item_level_bonus(), 0);
    assert_eq!(data.item_level_offset_curve_id(), 55);
    assert_eq!(data.item_level_offset_item_level(), 66);
    assert_eq!(data.scaling_config_item_level_bonus(), 88);
}

#[test]
fn quality_uses_first_assignment_then_unsigned_max_and_stat_slot_search_is_ordered() {
    let mut sources = Sources::with_template();
    sources.templates.get_mut(&7).unwrap().item_stat_type = [5, -1, 7, 8, 9, 10, 11, 12, 13, 14];
    let mut data = ItemBonusData::new();
    data.initialize(sources.templates.get(&7).unwrap(), &sources)
        .unwrap();

    data.add_bonus(2, [7, 10, 0, 0], &sources).unwrap();
    assert_eq!(data.item_stat_type()[1], 7);
    assert_eq!(data.item_stat_type()[2], 7);
    assert_eq!(data.stat_percent_editor()[1], 12);
    assert_eq!(data.stat_percent_editor()[2], 2);
    data.add_bonus(3, [2, 0, 0, 0], &sources).unwrap();
    assert_eq!(data.quality(), 2);
    data.add_bonus(3, [1, 0, 0, 0], &sources).unwrap();
    assert_eq!(data.quality(), 2);
    data.add_bonus(3, [-1, 0, 0, 0], &sources).unwrap();
    assert_eq!(data.quality(), u32::MAX);

    let full = Sources::with_template();
    let mut full_data = ItemBonusData::new();
    full_data
        .initialize(full.templates.get(&7).unwrap(), &full)
        .unwrap();
    let original_types = *full_data.item_stat_type();
    let original_percent = *full_data.stat_percent_editor();
    full_data.add_bonus(2, [99, 7, 0, 0], &full).unwrap();
    assert_eq!(full_data.item_stat_type(), &original_types);
    assert_eq!(full_data.stat_percent_editor(), &original_percent);
}

#[test]
fn zero_socket_count_and_zero_color_assignments_consume_only_the_requested_slots() {
    let mut sources = Sources::with_template();
    sources.templates.get_mut(&7).unwrap().socket_color = [0; 3];
    let mut data = ItemBonusData::new();
    data.initialize(sources.templates.get(&7).unwrap(), &sources)
        .unwrap();

    data.add_bonus(6, [0, 8, 0, 0], &sources).unwrap();
    assert_eq!(data.socket_color(), &[0, 0, 0]);
    data.add_bonus(6, [2, 0, 0, 0], &sources).unwrap();
    assert_eq!(data.socket_color(), &[0, 0, 0]);
    data.add_bonus(6, [1, 8, 0, 0], &sources).unwrap();
    assert_eq!(data.socket_color(), &[8, 0, 0]);
}

#[test]
fn scaling_distribution_priorities_are_shared_and_fixed_level_is_retained() {
    let mut sources = Sources::with_template();
    let config = ItemScalingConfigFacts {
        item_offset_curve_id: 20,
        item_level: 250,
        required_level: 70,
        item_squish_era_id: 8,
        flags: 0,
    };
    sources.scaling.insert(20, config);
    sources.scaling.insert(21, config);
    let mut data = ItemBonusData::new();
    data.initialize(sources.templates.get(&7).unwrap(), &sources)
        .unwrap();

    data.add_bonus(11, [0, 4, 100, 101], &sources).unwrap();
    assert!(!data.has_fixed_level());
    assert_eq!(data.content_tuning_id(), 100);
    data.add_bonus(13, [0, 4, 200, 201], &sources).unwrap();
    assert!(!data.has_fixed_level());
    assert_eq!(data.content_tuning_id(), 100);
    data.add_bonus(13, [0, 3, 300, 301], &sources).unwrap();
    assert!(data.has_fixed_level());
    assert_eq!(data.content_tuning_id(), 300);
    data.add_bonus(11, [0, 3, 400, 401], &sources).unwrap();
    assert!(data.has_fixed_level());
    assert_eq!(data.content_tuning_id(), 300);

    data.add_bonus(48, [50, 60, 0, 2], &sources).unwrap();
    assert!(data.has_fixed_level());
    assert_eq!(data.item_level_offset_curve_id(), 50);
    data.add_bonus(49, [20, 1, 0, 0], &sources).unwrap();
    assert!(data.has_fixed_level());
    data.add_bonus(51, [21, 0, 0, 0], &sources).unwrap();
    assert!(data.has_fixed_level());
}

#[test]
fn type49_respects_required_curve_guard_without_updating_its_priority() {
    let mut sources = Sources::with_template();
    sources.scaling.insert(
        10,
        ItemScalingConfigFacts {
            item_offset_curve_id: 0,
            item_level: 900,
            required_level: 1,
            item_squish_era_id: 2,
            flags: 0,
        },
    );
    let mut data = ItemBonusData::new();
    data.initialize(sources.templates.get(&7).unwrap(), &sources)
        .unwrap();
    data.add_bonus(18, [111, 0, 0, 0], &sources).unwrap();
    data.add_bonus(27, [55, 0, -5, 0], &sources).unwrap();

    data.add_bonus(49, [10, 0, 0, 0], &sources).unwrap();
    assert_eq!(data.required_level_override(), 111);
    assert_eq!(data.required_level_curve(), 55);
    assert_eq!(data.item_level_offset_item_level(), 14);
    data.add_bonus(27, [66, 0, -6, 0], &sources).unwrap();
    assert_eq!(data.required_level_curve(), 66);
}

#[test]
fn losing_scaling_and_crafting_priorities_do_not_query_sources() {
    let mut sources = Sources::with_template();
    sources.fail_scaling = true;
    sources.fail_offset = true;
    let mut data = ItemBonusData::new();
    data.initialize(sources.templates.get(&7).unwrap(), &sources)
        .unwrap();
    data.add_bonus(48, [0, 0, 0, 1], &sources).unwrap();
    data.add_bonus(51, [8, 2, 0, 0], &sources).unwrap();

    data.add_bonus(52, [10, 0, 20, 1], &sources).unwrap();
    assert_eq!(data.scaling_config_crafting_quality_item_level_bonus(), 10);
    sources.realm_patch = Some(10);
    sources.fail_squish_bound = true;
    data.add_bonus(52, [30, 0, 40, 2], &sources).unwrap();
    assert_eq!(sources.squish_bound_reads.get(), 0);
}

#[test]
fn signed_accumulator_overflow_pvp_intermediate_and_template_bonding_are_guarded() {
    let mut required_sources = Sources::with_template();
    required_sources
        .templates
        .get_mut(&7)
        .unwrap()
        .required_level = i32::MAX;
    let mut required = ItemBonusData::new();
    required
        .initialize(
            required_sources.templates.get(&7).unwrap(),
            &required_sources,
        )
        .unwrap();
    let error = required
        .add_bonus(8, [1, 0, 0, 0], &required_sources)
        .unwrap_err();
    assert_eq!(
        error,
        ItemBonusDataError::ArithmeticOverflow {
            field: "required_level"
        }
    );
    assert_eq!(required.required_level(), i32::MAX);

    let mut stat_sources = Sources::with_template();
    stat_sources
        .templates
        .get_mut(&7)
        .unwrap()
        .stat_percent_editor = [i32::MAX; 10];
    let mut stat = ItemBonusData::new();
    stat.initialize(stat_sources.templates.get(&7).unwrap(), &stat_sources)
        .unwrap();
    let error = stat.add_bonus(2, [5, 1, 0, 0], &stat_sources).unwrap_err();
    assert_eq!(
        error,
        ItemBonusDataError::ArithmeticOverflow {
            field: "stat_percent_editor"
        }
    );
    assert_eq!(stat.stat_percent_editor()[0], i32::MAX);

    let pvp_sources = Sources::with_template();
    let mut pvp = ItemBonusData::new();
    pvp.initialize(pvp_sources.templates.get(&7).unwrap(), &pvp_sources)
        .unwrap();
    pvp.add_bonus(36, [1, 0, 0, 0], &pvp_sources).unwrap();
    let error = pvp
        .add_bonus(36, [i32::MAX, 0, 0, 0], &pvp_sources)
        .unwrap_err();
    assert_eq!(
        error,
        ItemBonusDataError::ArithmeticOverflow {
            field: "pvp_item_level_bonus"
        }
    );
    assert_eq!(pvp.pvp_item_level_bonus(), 1);

    let mut invalid_template_sources = Sources::with_template();
    invalid_template_sources
        .templates
        .get_mut(&7)
        .unwrap()
        .bonding = 16;
    let mut invalid_template = ItemBonusData::new();
    let error = invalid_template
        .initialize(
            invalid_template_sources.templates.get(&7).unwrap(),
            &invalid_template_sources,
        )
        .unwrap_err();
    assert_eq!(error, ItemBonusDataError::InvalidBondingValue { value: 16 });
    assert_eq!(invalid_template.quality(), 3);
    assert_eq!(invalid_template.bonding(), 0);
}

#[test]
fn effect_rows_preserve_ids_and_prefix_when_catalog_or_capacity_fails() {
    let mut sources = Sources::with_template();
    sources.effects.extend(100..120);
    let mut data = ItemBonusData::new();
    data.initialize(sources.templates.get(&7).unwrap(), &sources)
        .unwrap();
    data.add_bonus(23, [71, 0, 0, 0], &sources).unwrap();
    assert_eq!(data.effects(), &[0, 70, 71]);
    data.add_bonus(23, [999, 0, 0, 0], &sources).unwrap();
    assert_eq!(data.effects(), &[0, 70, 71]);

    let mut too_many = Sources::with_template();
    too_many.effects.extend([13, 14]);
    too_many.effects.extend(100..120);
    let mut full = ItemBonusData::new();
    let mut template = too_many.templates.remove(&7).unwrap();
    template.effect_ids = (1..=12).collect();
    full.initialize(&template, &too_many).unwrap();
    full.add_bonus(23, [13, 0, 0, 0], &too_many).unwrap();
    assert_eq!(full.effects().len(), 13);
    let error = full.add_bonus(23, [14, 0, 0, 0], &too_many).unwrap_err();
    assert_eq!(
        error,
        ItemBonusDataError::EffectCapacityExceeded { effect_id: 14 }
    );
    assert_eq!(full.effects().last(), Some(&13));

    let mut failed = Sources::with_template();
    failed.fail_effect = true;
    let error = full.add_bonus(23, [15, 0, 0, 0], &failed).unwrap_err();
    assert_eq!(
        error,
        ItemBonusDataError::Source("effect source unavailable")
    );
    assert_eq!(full.effects().last(), Some(&13));

    let mut template_sources = Sources::with_template();
    let mut wide_template = template_sources.templates.remove(&7).unwrap();
    wide_template.effect_ids = (0..14).collect();
    let mut template_data = ItemBonusData::new();
    let error = template_data
        .initialize(&wide_template, &template_sources)
        .unwrap_err();
    assert_eq!(
        error,
        ItemBonusDataError::EffectCapacityExceeded { effect_id: 13 }
    );
    assert_eq!(template_data.effects(), &(0..13).collect::<Vec<_>>());
}

#[test]
fn source_and_arithmetic_errors_keep_applied_prefix_and_bonding_range() {
    let mut sources = Sources::with_template();
    sources.add_list(
        1,
        [
            row(1, [5, 0, 0, 0]),
            row(50, [2, 0, 0, 0]),
            row(1, [100, 0, 0, 0]),
        ],
    );
    sources.fail_list = Some(2);
    let mut data = ItemBonusData::new();
    let error = data.add_bonus_list(1, &sources).unwrap_err();
    assert_eq!(
        error,
        ItemBonusDataError::Source("bonus list source unavailable")
    );
    assert_eq!(data.item_level_bonus(), 5);

    data.add_bonus(1, [i32::MAX - 5, 0, 0, 0], &sources)
        .unwrap();
    let error = data.add_bonus(1, [1, 0, 0, 0], &sources).unwrap_err();
    assert_eq!(
        error,
        ItemBonusDataError::ArithmeticOverflow {
            field: "item_level_bonus"
        }
    );
    assert_eq!(data.item_level_bonus(), i32::MAX);

    data.add_bonus(16, [15, 0, 0, 0], &sources).unwrap();
    assert_eq!(data.bonding(), 15);
    let error = data.add_bonus(16, [16, 0, 0, 0], &sources).unwrap_err();
    assert_eq!(error, ItemBonusDataError::InvalidBondingValue { value: 16 });
    assert_eq!(data.bonding(), 15);
    let error = data.add_bonus(16, [-1, 0, 0, 0], &sources).unwrap_err();
    assert_eq!(error, ItemBonusDataError::InvalidBondingValue { value: -1 });

    data.add_bonus(36, [40_000, 0, 0, 0], &sources).unwrap();
    assert_eq!(data.pvp_item_level_bonus(), 40_000_i32 as i16);
}

#[test]
fn scaling_config_priority_missing_rows_sticky_flags_and_type49_curve_priority() {
    let mut sources = Sources::with_template();
    sources.scaling.insert(
        10,
        ItemScalingConfigFacts {
            item_offset_curve_id: 20,
            item_level: 300,
            required_level: 80,
            item_squish_era_id: 22,
            flags: 3,
        },
    );
    sources.offsets.insert(
        20,
        ItemOffsetCurveFacts {
            curve_id: 21,
            offset: -7,
        },
    );
    sources.scaling.insert(
        11,
        ItemScalingConfigFacts {
            item_offset_curve_id: 99,
            item_level: 500,
            required_level: 90,
            item_squish_era_id: 23,
            flags: 0,
        },
    );
    let mut data = ItemBonusData::new();
    data.initialize(sources.templates.get(&7).unwrap(), &sources)
        .unwrap();
    data.add_bonus(27, [40, 42, 5, 0], &sources).unwrap();
    data.add_bonus(49, [10, 2, 0, 0], &sources).unwrap();
    assert_eq!(data.item_level_offset_curve_id(), 21);
    assert_eq!(data.item_level_offset(), -7);
    assert_eq!(data.item_level_offset_item_level(), 300);
    assert_eq!(data.required_level_override(), 80);
    assert_eq!(data.required_level_curve(), 0);
    assert_eq!(data.item_squish_era_id(), 22);
    assert!(!data.scaling_config_uses_player_level());
    assert!(data.ignore_squish());
    assert!(data.restrict_scaling_to_content_tuning());
    data.add_bonus(27, [50, 51, 3, 0], &sources).unwrap();
    assert_eq!(data.required_level_curve(), 50);

    data.add_bonus(51, [11, 1, 0, 0], &sources).unwrap();
    assert_eq!(data.item_level_offset_curve_id(), 21);
    assert_eq!(data.item_level_offset(), -7);
    assert_eq!(data.item_level_offset_item_level(), 0);
    assert_eq!(data.item_squish_era_id(), 23);
    assert!(data.scaling_config_uses_player_level());
    assert!(data.ignore_squish());
    assert!(data.restrict_scaling_to_content_tuning());

    let mut missing = Sources::with_template();
    let mut missing_data = ItemBonusData::new();
    missing_data
        .initialize(missing.templates.get(&7).unwrap(), &missing)
        .unwrap();
    missing_data
        .add_bonus(51, [404, 1, 0, 0], &missing)
        .unwrap();
    missing.scaling.insert(
        405,
        ItemScalingConfigFacts {
            item_offset_curve_id: 0,
            item_level: 900,
            required_level: 1,
            item_squish_era_id: 1,
            flags: 0,
        },
    );
    missing_data
        .add_bonus(51, [405, 2, 0, 0], &missing)
        .unwrap();
    assert_eq!(missing_data.item_squish_era_id(), 15);
    assert_eq!(missing_data.item_level_offset_item_level(), 14);
}

#[test]
fn crafted_level_uses_index_bound_holes_first_enabled_row_and_optional_realm() {
    let mut sources = Sources::with_template();
    sources.squish_bound = 9;
    sources.realm_patch = Some(20);
    sources.squish.insert(
        4,
        ItemSquishEraFacts {
            patch: 10,
            flags: 1,
        },
    );
    sources.squish.insert(
        5,
        ItemSquishEraFacts {
            patch: 30,
            flags: 0,
        },
    );
    sources
        .squish
        .insert(6, ItemSquishEraFacts { patch: 5, flags: 0 });
    let mut data = ItemBonusData::new();
    data.initialize(sources.templates.get(&7).unwrap(), &sources)
        .unwrap();
    data.add_bonus(52, [100, 2, 200, 1], &sources).unwrap();
    assert_eq!(data.scaling_config_crafting_quality_item_level_bonus(), 100);
    assert_eq!(sources.squish_bound_reads.get(), 1);
    assert_eq!(sources.squish_index_reads.get(), 3);

    let mut squished = Sources::with_template();
    squished.squish_bound = 8;
    squished.realm_patch = Some(30);
    squished.squish.insert(
        5,
        ItemSquishEraFacts {
            patch: 30,
            flags: 0,
        },
    );
    let mut squished_data = ItemBonusData::new();
    squished_data
        .initialize(squished.templates.get(&7).unwrap(), &squished)
        .unwrap();
    squished_data
        .add_bonus(52, [100, 4, 200, 1], &squished)
        .unwrap();
    assert_eq!(
        squished_data.scaling_config_crafting_quality_item_level_bonus(),
        200
    );

    let mut no_realm = Sources::with_template();
    no_realm.fail_squish_bound = true;
    no_realm.fail_squish_index = Some(8);
    let mut no_realm_data = ItemBonusData::new();
    no_realm_data
        .initialize(no_realm.templates.get(&7).unwrap(), &no_realm)
        .unwrap();
    no_realm_data
        .add_bonus(52, [321, i32::MAX, 654, 1], &no_realm)
        .unwrap();
    assert_eq!(
        no_realm_data.scaling_config_crafting_quality_item_level_bonus(),
        321
    );
    assert_eq!(no_realm.squish_bound_reads.get(), 0);
    assert_eq!(no_realm.squish_index_reads.get(), 0);

    let mut failed = Sources::with_template();
    failed.realm_patch = Some(10);
    failed.fail_squish_bound = true;
    let mut failed_data = ItemBonusData::new();
    failed_data
        .initialize(failed.templates.get(&7).unwrap(), &failed)
        .unwrap();
    failed_data.add_bonus(1, [7, 0, 0, 0], &failed).unwrap();
    let error = failed_data
        .add_bonus(52, [100, 0, 200, 1], &failed)
        .unwrap_err();
    assert_eq!(
        error,
        ItemBonusDataError::Source("squish bound unavailable")
    );
    assert_eq!(failed_data.item_level_bonus(), 7);
    assert_eq!(
        failed_data.scaling_config_crafting_quality_item_level_bonus(),
        0
    );

    let mut failed_row = Sources::with_template();
    failed_row.realm_patch = Some(10);
    failed_row.squish_bound = 9;
    failed_row.squish.insert(
        4,
        ItemSquishEraFacts {
            patch: 10,
            flags: 1,
        },
    );
    failed_row.fail_squish_index = Some(5);
    let mut failed_row_data = ItemBonusData::new();
    failed_row_data
        .initialize(failed_row.templates.get(&7).unwrap(), &failed_row)
        .unwrap();
    failed_row_data
        .add_bonus(1, [7, 0, 0, 0], &failed_row)
        .unwrap();
    let error = failed_row_data
        .add_bonus(52, [100, 2, 200, 1], &failed_row)
        .unwrap_err();
    assert_eq!(error, ItemBonusDataError::Source("squish row unavailable"));
    assert_eq!(failed_row_data.item_level_bonus(), 7);
    assert_eq!(
        failed_row_data.scaling_config_crafting_quality_item_level_bonus(),
        0
    );

    let mut overflow_start = Sources::with_template();
    overflow_start.realm_patch = Some(10);
    let mut overflow_data = ItemBonusData::new();
    overflow_data
        .initialize(overflow_start.templates.get(&7).unwrap(), &overflow_start)
        .unwrap();
    let error = overflow_data
        .add_bonus(52, [100, i32::MAX, 200, 1], &overflow_start)
        .unwrap_err();
    assert_eq!(
        error,
        ItemBonusDataError::ArithmeticOverflow {
            field: "crafted_item_level_squish_start"
        }
    );
    assert_eq!(overflow_start.squish_bound_reads.get(), 0);
}

#[test]
fn initialization_and_bonus_source_errors_keep_the_completed_prefix() {
    let mut sources = Sources::with_template();
    sources.fail_azerite = true;
    let mut data = ItemBonusData::new();
    let template = sources.templates.get(&7).unwrap();
    let error = data.initialize(template, &sources).unwrap_err();
    assert_eq!(
        error,
        ItemBonusDataError::Source("Azerite source unavailable")
    );
    assert_eq!(data.quality(), 3);
    assert_eq!(data.socket_color(), &[8, 0, 4]);
    assert_eq!(data.appearance_mod_id(), 0);

    let mut list_sources = Sources::with_template();
    list_sources.add_list(
        1,
        [
            row(1, [2, 0, 0, 0]),
            row(23, [70, 0, 0, 0]),
            row(1, [5, 0, 0, 0]),
        ],
    );
    list_sources.fail_effect = true;
    let mut list_data = ItemBonusData::new();
    list_data
        .initialize_from_instance(7, &[1], &list_sources)
        .unwrap_err();
    assert_eq!(list_data.item_level(), 100);
    assert_eq!(list_data.item_level_bonus(), 2);
    assert_eq!(list_data.effects(), &[0, 70]);
}
