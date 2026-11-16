//! Consuming SQL DTO -> item composition. Main text belongs to enUS only.
use wow_data::forever_birth::{
    item_quantities::ItemEffectRelationRecord,
    item_records::{
        AzeriteEmpoweredItemRecord, ItemBonusRecord, ItemEffectRecord, ItemOffsetCurveRecord,
        ItemRecord, ItemRecords, ItemScalingConfigRecord, ItemSquishEraRecord,
    },
    item_sparse::{SparseItemRecord, SparseItemStrings},
};
use wow_persistence::forever::items::{ItemRows, SparseItemRow};

fn mask(words: [i32; 2]) -> u64 {
    u64::from(words[0] as u32) | (u64::from(words[1] as u32) << 32)
}

pub(super) fn records(rows: ItemRows) -> ItemRecords {
    ItemRecords {
        items: rows
            .items
            .into_iter()
            .map(|r| ItemRecord {
                id: r.id,
                class: r.class,
                subclass: r.subclass,
                material: r.material,
                inventory_type: r.inventory_type,
                sheathe: r.sheathe,
                pet_food: r.pet_food,
                sound_override: r.sound_override,
                icon_file: r.icon_file,
                group_sounds: r.group_sounds,
                content_tuning: r.content_tuning,
                modified_crafting_reagent: r.modified_crafting_reagent,
                unknown_1200: r.unknown_1200,
                crafting_quality: r.crafting_quality,
                squish_era: r.squish_era,
                recraft_reagent_percentage: r.recraft_reagent_percentage,
                order_source: r.order_source,
            })
            .collect(),
        sparse: rows.sparse.into_iter().map(sparse).collect(),
        effects: rows
            .effects
            .into_iter()
            .map(|r| ItemEffectRecord {
                id: r.id,
                legacy_slot: r.legacy_slot,
                trigger: r.trigger,
                charges: r.charges,
                cooldown: r.cooldown,
                category_cooldown: r.category_cooldown,
                spell_category: r.spell_category,
                spell: r.spell,
                specialization: r.specialization,
                player_condition: r.player_condition,
            })
            .collect(),
        relations: rows
            .relations
            .into_iter()
            .map(|r| ItemEffectRelationRecord {
                id: r.id,
                effect: r.effect,
                item: r.item,
            })
            .collect(),
        item_bonuses: rows
            .item_bonuses
            .into_iter()
            .map(|r| ItemBonusRecord {
                id: r.id,
                values: r.values,
                parent: r.parent,
                bonus_type: r.bonus_type,
                order: r.order,
            })
            .collect(),
        azerite_empowered_items: rows
            .azerite_empowered_items
            .into_iter()
            .map(|r| AzeriteEmpoweredItemRecord {
                id: r.id,
                item: r.item,
                tier_unlock_set: r.tier_unlock_set,
                power_set: r.power_set,
            })
            .collect(),
        item_scaling_configs: rows
            .item_scaling_configs
            .into_iter()
            .map(|r| ItemScalingConfigRecord {
                id: r.id,
                offset_curve: r.offset_curve,
                item_level: r.item_level,
                required_level: r.required_level,
                squish_era: r.squish_era,
                flags: r.flags,
            })
            .collect(),
        item_offset_curves: rows
            .item_offset_curves
            .into_iter()
            .map(|r| ItemOffsetCurveRecord {
                id: r.id,
                curve: r.curve,
                offset: r.offset,
            })
            .collect(),
        item_squish_eras: rows
            .item_squish_eras
            .into_iter()
            .map(|r| ItemSquishEraRecord {
                id: r.id,
                patch: r.patch,
                curve: r.curve,
                flags: r.flags,
            })
            .collect(),
        item_squish_era_sql_index_size: rows.item_squish_era_sql_index_size,
        item_squish_era_file_last_index: None,
        unknown_baseline_records: [0; 9], // Overlay batches do not certify unknown baseline IDs.
    }
}

fn sparse(r: SparseItemRow) -> SparseItemRecord {
    SparseItemRecord {
        id: r.id,
        strings: SparseItemStrings::for_locale(0, r.strings),
        expansion: r.expansion,
        damage_variance: r.damage_variance,
        limit_category: r.limit_category,
        duration: r.duration,
        quality_modifier: r.quality_modifier,
        bag_family: r.bag_family,
        start_quest: r.start_quest,
        language: r.language,
        item_range: r.item_range,
        socket_percentage: r.socket_percentage,
        stat_percent: r.stat_percent,
        stat_bonus: r.stat_bonus,
        stackable: r.stackable,
        max_count: r.max_count,
        min_reputation: r.min_reputation,
        required_ability: r.required_ability,
        allowable_race: mask(r.allowable_race),
        sell_price: r.sell_price,
        buy_price: r.buy_price,
        vendor_stack: r.vendor_stack,
        price_variance: r.price_variance,
        price_random: r.price_random,
        flags: r.flags,
        faction_related: r.faction_related,
        modified_crafting_reagent: r.modified_crafting_reagent,
        content_tuning: r.content_tuning,
        player_level_curve: r.player_level_curve,
        item_level_offset_curve: r.item_level_offset_curve,
        item_level_offset: r.item_level_offset,
        squish_era: r.squish_era,
        name_description: r.name_description,
        transmog_holiday: r.transmog_holiday,
        holiday: r.holiday,
        gem_properties: r.gem_properties,
        socket_enchantment: r.socket_enchantment,
        totem_category: r.totem_category,
        instance_bound: r.instance_bound,
        zone_bound: r.zone_bound,
        item_set: r.item_set,
        lock: r.lock,
        page: r.page,
        delay: r.delay,
        min_faction: r.min_faction,
        required_skill_rank: r.required_skill_rank,
        required_skill: r.required_skill,
        item_level: r.item_level,
        allowable_class: r.allowable_class,
        artifact: r.artifact,
        spell_weight: r.spell_weight,
        spell_weight_category: r.spell_weight_category,
        socket_type: r.socket_type,
        sheathe: r.sheathe,
        material: r.material,
        page_material: r.page_material,
        bonding: r.bonding,
        damage_type: r.damage_type,
        container_slots: r.container_slots,
        required_pvp_medal: r.required_pvp_medal,
        required_pvp_rank: r.required_pvp_rank,
        required_level: r.required_level,
        inventory_type: r.inventory_type,
        quality: r.quality,
        ammunition: r.ammunition,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_persistence::forever::items::{
        AzeriteEmpoweredItemRow, ItemBonusRow, ItemEffectRow, ItemOffsetCurveRow, ItemRelationRow,
        ItemRow, ItemScalingConfigRow, ItemSquishEraRow,
    };
    #[test]
    fn signed_masks_and_consumed_item_effect_relation_widths_are_not_narrowed() {
        assert_eq!(mask([-1, i32::MIN]), 0x8000_0000_FFFF_FFFF);
        let result = records(ItemRows {
            items: vec![ItemRow {
                id: u32::MAX,
                class: i32::MIN,
                subclass: u8::MAX,
                material: 255,
                inventory_type: i8::MIN,
                sheathe: 255,
                pet_food: i32::MIN,
                sound_override: i8::MIN,
                icon_file: i32::MAX,
                group_sounds: u32::MAX,
                content_tuning: i32::MIN,
                modified_crafting_reagent: -1,
                unknown_1200: 255,
                crafting_quality: -1,
                squish_era: -1,
                recraft_reagent_percentage: f32::from_bits(0x8000_0000),
                order_source: 255,
            }],
            effects: vec![ItemEffectRow {
                id: u32::MAX,
                legacy_slot: 255,
                trigger: 255,
                charges: i16::MIN,
                cooldown: i32::MIN,
                category_cooldown: i32::MAX,
                spell_category: u16::MAX,
                spell: -1,
                specialization: u16::MAX,
                player_condition: i32::MIN,
            }],
            relations: vec![ItemRelationRow {
                id: u32::MAX,
                effect: -1,
                item: u32::MAX,
            }],
            item_bonuses: vec![ItemBonusRow {
                id: u32::MAX,
                values: [i32::MIN, -1, 0x0102_0304, i32::MAX],
                parent: u16::MAX,
                bonus_type: u8::MAX,
                order: u8::MAX,
            }],
            azerite_empowered_items: vec![AzeriteEmpoweredItemRow {
                id: u32::MAX,
                item: -1,
                tier_unlock_set: u32::MAX,
                power_set: u32::MAX,
            }],
            item_scaling_configs: vec![ItemScalingConfigRow {
                id: u32::MAX,
                offset_curve: i32::MIN,
                item_level: -1,
                required_level: i32::MAX,
                squish_era: 0x8000_0001u32 as i32,
                flags: -1,
            }],
            item_offset_curves: vec![ItemOffsetCurveRow {
                id: u32::MAX,
                curve: i32::MIN,
                offset: i32::MAX,
            }],
            item_squish_eras: vec![ItemSquishEraRow {
                id: u32::MAX,
                patch: i32::MIN,
                curve: -1,
                flags: i32::MAX,
            }],
            item_squish_era_sql_index_size: Some(12),
            ..Default::default()
        });
        assert_eq!(result.items[0].class, i32::MIN);
        assert_eq!(result.items[0].inventory_type, i8::MIN);
        assert_eq!(result.items[0].group_sounds, u32::MAX);
        assert_eq!(
            result.items[0].recraft_reagent_percentage.to_bits(),
            0x8000_0000
        );
        assert_eq!(result.effects[0].charges, i16::MIN);
        assert_eq!(result.effects[0].spell_category, u16::MAX);
        assert_eq!(result.effects[0].spell, -1);
        assert_eq!(result.relations[0].item, u32::MAX);
        assert_eq!(result.relations[0].effect, -1);
        assert_eq!(
            result.item_bonuses[0].values,
            [i32::MIN, -1, 0x0102_0304, i32::MAX]
        );
        assert_eq!(result.item_bonuses[0].parent, u16::MAX);
        assert_eq!(result.azerite_empowered_items[0].item, -1);
        assert_eq!(result.azerite_empowered_items[0].tier_unlock_set, u32::MAX);
        assert_eq!(
            result.item_scaling_configs[0].squish_era,
            0x8000_0001u32 as i32
        );
        assert_eq!(result.item_offset_curves[0].offset, i32::MAX);
        assert_eq!(result.item_squish_eras[0].id, u32::MAX);
        assert_eq!(result.item_squish_era_sql_index_size, Some(12));
        assert_eq!(result.item_squish_era_file_last_index, None);
        assert_eq!(result.unknown_baseline_records, [0; 9]);
    }
}
