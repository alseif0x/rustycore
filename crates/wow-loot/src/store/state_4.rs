//! Loot store definitions and templates state definitions, part 4 of 4.
//!
//! Separated from the lib.rs root under #642. Behaviour is preserved.

use super::*;

pub(super) fn add_generated_loot_item_like_cpp<R, FTemplate, FRandom>(
    generated: &mut Vec<GeneratedLootItem>,
    store_item_context: LootStoreItemContext,
    item_context: u8,
    rng: &mut R,
    item_template: &mut FTemplate,
    random_properties: &mut FRandom,
) where
    R: Rng + ?Sized,
    FTemplate: FnMut(u32) -> Option<LootItemTemplateMetadata>,
    FRandom: FnMut(u32, &mut R) -> LootItemRandomProperties,
{
    let item = store_item_context.item;
    let Some(metadata) = item_template(item.item_id) else {
        return;
    };
    let max_stack = metadata.max_stack;
    if max_stack == 0 {
        return;
    }

    let mut count = rng.gen_range(u32::from(item.min_count)..=u32::from(item.max_count));
    let stacks = count / max_stack + u32::from(count % max_stack != 0);

    for _ in 0..stacks {
        if generated.len() >= MAX_NR_LOOT_ITEMS_LIKE_CPP {
            return;
        }

        let stack_count = count.min(max_stack);
        let random_properties = random_properties(item.item_id, rng);
        generated.push(GeneratedLootItem {
            item_id: item.item_id,
            count: stack_count,
            loot_list_id: generated.len() as u32,
            random_properties_id: random_properties.id,
            random_properties_seed: random_properties.seed,
            context: item_context,
            store_item_context,
            free_for_all: metadata.has_multi_drop_flag,
            follow_loot_rules: !item.needs_quest || metadata.has_follow_loot_rules_flag,
            needs_quest: item.needs_quest,
            is_looted: false,
            is_blocked: false,
            is_under_threshold: false,
            is_counted: false,
        });
        count = count.saturating_sub(max_stack);
    }
}

pub(super) fn add_generated_personal_loot_item_like_cpp<R, FTemplate, FRandom>(
    generated: &mut Vec<GeneratedPersonalLootItem>,
    looter: ObjectGuid,
    store_item_context: LootStoreItemContext,
    item_context: u8,
    rng: &mut R,
    item_template: &mut FTemplate,
    random_properties: &mut FRandom,
) where
    R: Rng + ?Sized,
    FTemplate: FnMut(u32) -> Option<LootItemTemplateMetadata>,
    FRandom: FnMut(u32, &mut R) -> LootItemRandomProperties,
{
    let item = store_item_context.item;
    let Some(metadata) = item_template(item.item_id) else {
        return;
    };
    let max_stack = metadata.max_stack;
    if max_stack == 0 {
        return;
    }

    let mut count = rng.gen_range(u32::from(item.min_count)..=u32::from(item.max_count));
    let stacks = count / max_stack + u32::from(count % max_stack != 0);

    for _ in 0..stacks {
        if generated.len() >= MAX_NR_LOOT_ITEMS_LIKE_CPP {
            return;
        }

        let stack_count = count.min(max_stack);
        let random_properties = random_properties(item.item_id, rng);
        generated.push(GeneratedPersonalLootItem {
            looter,
            item: GeneratedLootItem {
                item_id: item.item_id,
                count: stack_count,
                loot_list_id: generated.len() as u32,
                random_properties_id: random_properties.id,
                random_properties_seed: random_properties.seed,
                context: item_context,
                store_item_context,
                free_for_all: metadata.has_multi_drop_flag,
                follow_loot_rules: !item.needs_quest || metadata.has_follow_loot_rules_flag,
                needs_quest: item.needs_quest,
                is_looted: false,
                is_blocked: false,
                is_under_threshold: false,
                is_counted: false,
            },
        });
        count = count.saturating_sub(max_stack);
    }
}

pub(super) fn roll_chance_like_cpp<R: Rng + ?Sized>(rng: &mut R, chance: f32) -> bool {
    if chance <= 0.0 {
        return false;
    }
    if chance >= 100.0 {
        return true;
    }

    rng.gen_range(0.0f32..100.0f32) < chance
}

pub(super) fn append_reference_items_like_cpp(
    store_kind: LootStoreKind,
    entry: u32,
    items: &[LootStoreItem],
    uses: &mut Vec<LootReferenceUse>,
) {
    for item in items {
        if item.reference > 0 {
            uses.push(LootReferenceUse {
                store_kind,
                entry,
                item_id: item.item_id,
                reference: item.reference,
            });
        }
    }
}
