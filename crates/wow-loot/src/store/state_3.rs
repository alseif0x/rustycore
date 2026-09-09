//! Loot store definitions and templates state definitions, part 3 of 4.
//!
//! Separated from the lib.rs root under #642. Behaviour is preserved.

use super::*;

impl LootTemplate {
    pub fn add_entry_like_cpp(&mut self, item: LootStoreItem) {
        if item.group_id > 0 && item.reference == 0 {
            let index = usize::from(item.group_id - 1);
            if index >= self.groups.len() {
                self.groups.resize_with(index + 1, LootGroup::default);
            }
            self.groups[index].add_entry_like_cpp(item);
        } else {
            self.entries.push(item);
        }
    }

    #[must_use]
    pub fn entries(&self) -> &[LootStoreItem] {
        &self.entries
    }

    #[must_use]
    pub fn groups(&self) -> &[LootGroup] {
        &self.groups
    }

    #[must_use]
    pub fn is_reference_like_cpp(&self, item_id: u32) -> bool {
        self.entries
            .iter()
            .any(|item| item.item_id == item_id && item.reference > 0)
    }

    #[must_use]
    pub fn has_condition_link_target_like_cpp(&self, source_entry: u32) -> bool {
        if self.entries.iter().any(|item| item.item_id == source_entry) {
            return true;
        }

        self.groups
            .iter()
            .any(|group| group.has_condition_link_target_like_cpp(source_entry))
    }

    pub(super) fn append_reference_uses_like_cpp(
        &self,
        store_kind: LootStoreKind,
        entry: u32,
        uses: &mut Vec<LootReferenceUse>,
    ) {
        append_reference_items_like_cpp(store_kind, entry, &self.entries, uses);

        for group in &self.groups {
            group.append_reference_uses_like_cpp(store_kind, entry, uses);
        }
    }

    pub(super) fn append_condition_ids_for_fill_like_cpp(
        &self,
        stores: &LootStores,
        store_kind: LootStoreKind,
        entry: u32,
        group_id: u8,
        ids: &mut Vec<LootConditionId>,
    ) {
        if group_id > 0 {
            let index = usize::from(group_id - 1);
            if let Some(group) = self.groups.get(index) {
                group.append_condition_ids_for_fill_like_cpp(store_kind, entry, ids);
            }
            return;
        }

        for item in &self.entries {
            if item.reference > 0 {
                let Some(reference_store) = stores.get(&LootStoreKind::Reference) else {
                    continue;
                };
                let Some(reference_template) = reference_store.get_loot_for(item.reference) else {
                    continue;
                };
                reference_template.append_condition_ids_for_fill_like_cpp(
                    stores,
                    LootStoreKind::Reference,
                    item.reference,
                    item.group_id,
                    ids,
                );
            } else {
                ids.push(LootConditionId {
                    source_type: condition_source_type_for_loot_store_kind_like_cpp(store_kind),
                    source_group: entry,
                    source_entry: item.item_id,
                });
            }
        }

        for group in &self.groups {
            group.append_condition_ids_for_fill_like_cpp(store_kind, entry, ids);
        }
    }

    pub(super) fn has_quest_drop_like_cpp<F>(
        &self,
        stores: &LootStores,
        group_id: u8,
        player_has_quest_for_item: &mut F,
    ) -> bool
    where
        F: FnMut(u32) -> bool,
    {
        if group_id > 0 {
            let index = usize::from(group_id - 1);
            return self
                .groups
                .get(index)
                .is_some_and(|group| group.has_quest_drop_like_cpp(player_has_quest_for_item));
        }

        for item in &self.entries {
            if item.reference > 0 {
                let Some(reference_store) = stores.get(&LootStoreKind::Reference) else {
                    continue;
                };
                let Some(reference_template) = reference_store.get_loot_for(item.reference) else {
                    continue;
                };
                if reference_template.has_quest_drop_like_cpp(
                    stores,
                    item.group_id,
                    player_has_quest_for_item,
                ) {
                    return true;
                }
            } else if item.needs_quest {
                return true;
            }
        }

        self.groups
            .iter()
            .any(|group| group.has_quest_drop_like_cpp(player_has_quest_for_item))
    }

    pub(super) fn has_quest_drop_for_player_like_cpp<F>(
        &self,
        stores: &LootStores,
        group_id: u8,
        player_has_quest_for_item: &mut F,
    ) -> bool
    where
        F: FnMut(u32) -> bool,
    {
        if group_id > 0 {
            let index = usize::from(group_id - 1);
            return self.groups.get(index).is_some_and(|group| {
                group.has_quest_drop_for_player_like_cpp(player_has_quest_for_item)
            });
        }

        for item in &self.entries {
            if item.reference > 0 {
                let Some(reference_store) = stores.get(&LootStoreKind::Reference) else {
                    continue;
                };
                let Some(reference_template) = reference_store.get_loot_for(item.reference) else {
                    continue;
                };
                if reference_template.has_quest_drop_for_player_like_cpp(
                    stores,
                    item.group_id,
                    player_has_quest_for_item,
                ) {
                    return true;
                }
            } else if player_has_quest_for_item(item.item_id) {
                return true;
            }
        }

        self.groups
            .iter()
            .any(|group| group.has_quest_drop_for_player_like_cpp(player_has_quest_for_item))
    }

    pub(super) fn process_like_cpp<R, FTemplate, FRate, FAllowed, FRandom>(
        &self,
        stores: &LootStores,
        options: &LootFillOptions,
        rng: &mut R,
        generated: &mut Vec<GeneratedLootItem>,
        item_template: &mut FTemplate,
        item_chance_rate: &mut FRate,
        item_allowed: &mut FAllowed,
        random_properties: &mut FRandom,
        store_kind: LootStoreKind,
        entry: u32,
        group_id: u8,
    ) where
        R: Rng + ?Sized,
        FTemplate: FnMut(u32) -> Option<LootItemTemplateMetadata>,
        FRate: FnMut(LootStoreItem) -> f32,
        FAllowed: FnMut(LootStoreItemContext) -> bool,
        FRandom: FnMut(u32, &mut R) -> LootItemRandomProperties,
    {
        if generated.len() >= MAX_NR_LOOT_ITEMS_LIKE_CPP {
            return;
        }

        if group_id > 0 {
            let index = usize::from(group_id - 1);
            if let Some(group) = self.groups.get(index) {
                group.process_like_cpp(
                    options.loot_mode,
                    rng,
                    generated,
                    item_template,
                    item_allowed,
                    random_properties,
                    options.item_context,
                    store_kind,
                    entry,
                );
            }
            return;
        }

        for item in &self.entries {
            if generated.len() >= MAX_NR_LOOT_ITEMS_LIKE_CPP {
                return;
            }

            if item.loot_mode & options.loot_mode == 0 {
                continue;
            }

            let chance_rate = if options.rates_allowed {
                item_chance_rate(*item)
            } else {
                1.0
            };

            if !item.roll_like_cpp(rng, chance_rate) {
                continue;
            }

            if item.reference > 0 {
                let Some(reference_store) = stores.get(&LootStoreKind::Reference) else {
                    continue;
                };
                let Some(reference_template) = reference_store.get_loot_for(item.reference) else {
                    continue;
                };

                let max_count =
                    ((f32::from(item.max_count)) * options.referenced_amount_rate) as u32;
                for _ in 0..max_count {
                    reference_template.process_like_cpp(
                        stores,
                        options,
                        rng,
                        generated,
                        item_template,
                        item_chance_rate,
                        item_allowed,
                        random_properties,
                        LootStoreKind::Reference,
                        item.reference,
                        item.group_id,
                    );
                    if generated.len() >= MAX_NR_LOOT_ITEMS_LIKE_CPP {
                        return;
                    }
                }
            } else {
                let store_item_context = LootStoreItemContext {
                    store_kind,
                    entry,
                    item: *item,
                };
                if !item_allowed(store_item_context) {
                    continue;
                }
                add_generated_loot_item_like_cpp(
                    generated,
                    store_item_context,
                    options.item_context,
                    rng,
                    item_template,
                    random_properties,
                );
            }
        }

        for group in &self.groups {
            if generated.len() >= MAX_NR_LOOT_ITEMS_LIKE_CPP {
                return;
            }
            group.process_like_cpp(
                options.loot_mode,
                rng,
                generated,
                item_template,
                item_allowed,
                random_properties,
                options.item_context,
                store_kind,
                entry,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn process_personal_like_cpp<R, FTemplate, FRate, FAllowed, FRandom>(
        &self,
        stores: &LootStores,
        options: &LootFillOptions,
        looters: &[ObjectGuid],
        rng: &mut R,
        generated: &mut Vec<GeneratedPersonalLootItem>,
        item_template: &mut FTemplate,
        item_chance_rate: &mut FRate,
        item_allowed: &mut FAllowed,
        random_properties: &mut FRandom,
        store_kind: LootStoreKind,
        entry: u32,
    ) where
        R: Rng + ?Sized,
        FTemplate: FnMut(u32) -> Option<LootItemTemplateMetadata>,
        FRate: FnMut(LootStoreItem) -> f32,
        FAllowed: FnMut(LootStoreItemContext, ObjectGuid) -> bool,
        FRandom: FnMut(u32, &mut R) -> LootItemRandomProperties,
    {
        for item in &self.entries {
            if generated.len() >= MAX_NR_LOOT_ITEMS_LIKE_CPP {
                return;
            }

            if item.loot_mode & options.loot_mode == 0 {
                continue;
            }

            let chance_rate = if options.rates_allowed {
                item_chance_rate(*item)
            } else {
                1.0
            };

            if !item.roll_like_cpp(rng, chance_rate) {
                continue;
            }

            if item.reference > 0 {
                let Some(reference_store) = stores.get(&LootStoreKind::Reference) else {
                    continue;
                };
                let Some(reference_template) = reference_store.get_loot_for(item.reference) else {
                    continue;
                };

                let max_count =
                    ((f32::from(item.max_count)) * options.referenced_amount_rate) as u32;
                let mut got_loot = Vec::new();
                for _ in 0..max_count {
                    let eligible = reference_template.personal_looters_for_template_like_cpp(
                        stores,
                        LootStoreKind::Reference,
                        item.reference,
                        item.group_id,
                        looters,
                        item_allowed,
                    );
                    if eligible.is_empty() {
                        break;
                    }

                    let not_yet_looted = eligible
                        .iter()
                        .copied()
                        .filter(|looter| !got_loot.contains(looter))
                        .collect::<Vec<_>>();
                    let candidates = if not_yet_looted.is_empty() {
                        got_loot.clear();
                        eligible
                    } else {
                        not_yet_looted
                    };
                    let chosen_looter = candidates[rng.gen_range(0..candidates.len())];
                    reference_template.process_for_personal_looter_like_cpp(
                        stores,
                        options,
                        rng,
                        generated,
                        item_template,
                        item_chance_rate,
                        item_allowed,
                        random_properties,
                        LootStoreKind::Reference,
                        item.reference,
                        item.group_id,
                        chosen_looter,
                    );
                    got_loot.push(chosen_looter);
                }
            } else {
                let candidates = looters
                    .iter()
                    .copied()
                    .filter(|looter| {
                        item_allowed(
                            LootStoreItemContext {
                                store_kind,
                                entry,
                                item: *item,
                            },
                            *looter,
                        )
                    })
                    .collect::<Vec<_>>();
                if candidates.is_empty() {
                    continue;
                }

                let chosen_looter = candidates[rng.gen_range(0..candidates.len())];
                add_generated_personal_loot_item_like_cpp(
                    generated,
                    chosen_looter,
                    LootStoreItemContext {
                        store_kind,
                        entry,
                        item: *item,
                    },
                    options.item_context,
                    rng,
                    item_template,
                    random_properties,
                );
            }
        }

        for group in &self.groups {
            if generated.len() >= MAX_NR_LOOT_ITEMS_LIKE_CPP {
                return;
            }

            let candidates = looters
                .iter()
                .copied()
                .filter(|looter| {
                    group.has_drop_for_personal_looter_like_cpp(
                        store_kind,
                        entry,
                        *looter,
                        item_allowed,
                    )
                })
                .collect::<Vec<_>>();
            if candidates.is_empty() {
                continue;
            }

            let chosen_looter = candidates[rng.gen_range(0..candidates.len())];
            group.process_personal_root_like_cpp(
                options.loot_mode,
                rng,
                generated,
                item_template,
                random_properties,
                options.item_context,
                chosen_looter,
                store_kind,
                entry,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn process_for_personal_looter_like_cpp<R, FTemplate, FRate, FAllowed, FRandom>(
        &self,
        stores: &LootStores,
        options: &LootFillOptions,
        rng: &mut R,
        generated: &mut Vec<GeneratedPersonalLootItem>,
        item_template: &mut FTemplate,
        item_chance_rate: &mut FRate,
        item_allowed: &mut FAllowed,
        random_properties: &mut FRandom,
        store_kind: LootStoreKind,
        entry: u32,
        group_id: u8,
        looter: ObjectGuid,
    ) where
        R: Rng + ?Sized,
        FTemplate: FnMut(u32) -> Option<LootItemTemplateMetadata>,
        FRate: FnMut(LootStoreItem) -> f32,
        FAllowed: FnMut(LootStoreItemContext, ObjectGuid) -> bool,
        FRandom: FnMut(u32, &mut R) -> LootItemRandomProperties,
    {
        let mut generated_for_looter = Vec::new();
        self.process_like_cpp(
            stores,
            options,
            rng,
            &mut generated_for_looter,
            item_template,
            item_chance_rate,
            &mut |context| item_allowed(context, looter),
            random_properties,
            store_kind,
            entry,
            group_id,
        );
        for mut item in generated_for_looter {
            item.loot_list_id = generated.len() as u32;
            generated.push(GeneratedPersonalLootItem { looter, item });
        }
    }

    pub(super) fn personal_looters_for_template_like_cpp<FAllowed>(
        &self,
        stores: &LootStores,
        store_kind: LootStoreKind,
        entry: u32,
        group_id: u8,
        looters: &[ObjectGuid],
        item_allowed: &mut FAllowed,
    ) -> Vec<ObjectGuid>
    where
        FAllowed: FnMut(LootStoreItemContext, ObjectGuid) -> bool,
    {
        looters
            .iter()
            .copied()
            .filter(|looter| {
                self.has_drop_for_personal_looter_like_cpp(
                    stores,
                    store_kind,
                    entry,
                    group_id,
                    *looter,
                    item_allowed,
                )
            })
            .collect()
    }

    pub(super) fn has_drop_for_personal_looter_like_cpp<FAllowed>(
        &self,
        stores: &LootStores,
        store_kind: LootStoreKind,
        entry: u32,
        group_id: u8,
        looter: ObjectGuid,
        item_allowed: &mut FAllowed,
    ) -> bool
    where
        FAllowed: FnMut(LootStoreItemContext, ObjectGuid) -> bool,
    {
        if group_id > 0 {
            let index = usize::from(group_id - 1);
            return self.groups.get(index).is_some_and(|group| {
                group.has_drop_for_personal_looter_like_cpp(store_kind, entry, looter, item_allowed)
            });
        }

        for item in &self.entries {
            if item.reference > 0 {
                let Some(reference_store) = stores.get(&LootStoreKind::Reference) else {
                    continue;
                };
                let Some(reference_template) = reference_store.get_loot_for(item.reference) else {
                    continue;
                };
                if reference_template.has_drop_for_personal_looter_like_cpp(
                    stores,
                    LootStoreKind::Reference,
                    item.reference,
                    item.group_id,
                    looter,
                    item_allowed,
                ) {
                    return true;
                }
            } else if item_allowed(
                LootStoreItemContext {
                    store_kind,
                    entry,
                    item: *item,
                },
                looter,
            ) {
                return true;
            }
        }

        self.groups.iter().any(|group| {
            group.has_drop_for_personal_looter_like_cpp(store_kind, entry, looter, item_allowed)
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LootGroup {
    pub(super) explicitly_chanced: Vec<LootStoreItem>,
    pub(super) equal_chanced: Vec<LootStoreItem>,
}

impl LootGroup {
    pub fn add_entry_like_cpp(&mut self, item: LootStoreItem) {
        if item.chance != 0.0 {
            self.explicitly_chanced.push(item);
        } else {
            self.equal_chanced.push(item);
        }
    }

    #[must_use]
    pub fn explicitly_chanced(&self) -> &[LootStoreItem] {
        &self.explicitly_chanced
    }

    #[must_use]
    pub fn equal_chanced(&self) -> &[LootStoreItem] {
        &self.equal_chanced
    }

    pub(super) fn has_quest_drop_like_cpp<F>(&self, player_has_quest_for_item: &mut F) -> bool
    where
        F: FnMut(u32) -> bool,
    {
        self.explicitly_chanced
            .iter()
            .chain(self.equal_chanced.iter())
            .any(|item| item.needs_quest && player_has_quest_for_item(item.item_id))
    }

    pub(super) fn has_quest_drop_for_player_like_cpp<F>(
        &self,
        player_has_quest_for_item: &mut F,
    ) -> bool
    where
        F: FnMut(u32) -> bool,
    {
        self.explicitly_chanced
            .iter()
            .chain(self.equal_chanced.iter())
            .any(|item| player_has_quest_for_item(item.item_id))
    }

    pub fn roll_like_cpp<R, F>(
        &self,
        loot_mode: u16,
        rng: &mut R,
        mut item_allowed: F,
    ) -> Option<LootStoreItem>
    where
        R: Rng + ?Sized,
        F: FnMut(LootStoreItem) -> bool,
    {
        self.roll_with_context_like_cpp(
            loot_mode,
            rng,
            |context| item_allowed(context.item),
            LootStoreKind::Creature,
            0,
        )
    }

    pub(super) fn roll_with_context_like_cpp<R, F>(
        &self,
        loot_mode: u16,
        rng: &mut R,
        mut item_allowed: F,
        store_kind: LootStoreKind,
        entry: u32,
    ) -> Option<LootStoreItem>
    where
        R: Rng + ?Sized,
        F: FnMut(LootStoreItemContext) -> bool,
    {
        let possible_explicit: Vec<LootStoreItem> = self
            .explicitly_chanced
            .iter()
            .copied()
            .filter(|item| {
                item.loot_mode & loot_mode != 0
                    && item_allowed(LootStoreItemContext {
                        store_kind,
                        entry,
                        item: *item,
                    })
            })
            .collect();

        if !possible_explicit.is_empty() {
            let mut roll = rng.gen_range(0.0f32..100.0f32);
            for item in possible_explicit {
                if item.chance >= 100.0 {
                    return Some(item);
                }

                roll -= item.chance;
                if roll < 0.0 {
                    return Some(item);
                }
            }
        }

        let possible_equal: Vec<LootStoreItem> = self
            .equal_chanced
            .iter()
            .copied()
            .filter(|item| {
                item.loot_mode & loot_mode != 0
                    && item_allowed(LootStoreItemContext {
                        store_kind,
                        entry,
                        item: *item,
                    })
            })
            .collect();

        if possible_equal.is_empty() {
            return None;
        }

        let index = rng.gen_range(0..possible_equal.len());
        Some(possible_equal[index])
    }

    pub(super) fn append_reference_uses_like_cpp(
        &self,
        store_kind: LootStoreKind,
        entry: u32,
        uses: &mut Vec<LootReferenceUse>,
    ) {
        append_reference_items_like_cpp(store_kind, entry, &self.explicitly_chanced, uses);
        append_reference_items_like_cpp(store_kind, entry, &self.equal_chanced, uses);
    }

    pub(super) fn append_condition_ids_for_fill_like_cpp(
        &self,
        store_kind: LootStoreKind,
        entry: u32,
        ids: &mut Vec<LootConditionId>,
    ) {
        for item in self
            .explicitly_chanced
            .iter()
            .chain(self.equal_chanced.iter())
        {
            ids.push(LootConditionId {
                source_type: condition_source_type_for_loot_store_kind_like_cpp(store_kind),
                source_group: entry,
                source_entry: item.item_id,
            });
        }
    }

    pub(super) fn has_condition_link_target_like_cpp(&self, source_entry: u32) -> bool {
        self.explicitly_chanced
            .iter()
            .chain(self.equal_chanced.iter())
            .any(|item| item.item_id == source_entry)
    }

    pub(super) fn process_like_cpp<R, FTemplate, FAllowed, FRandom>(
        &self,
        loot_mode: u16,
        rng: &mut R,
        generated: &mut Vec<GeneratedLootItem>,
        item_template: &mut FTemplate,
        item_allowed: &mut FAllowed,
        random_properties: &mut FRandom,
        item_context: u8,
        store_kind: LootStoreKind,
        entry: u32,
    ) where
        R: Rng + ?Sized,
        FTemplate: FnMut(u32) -> Option<LootItemTemplateMetadata>,
        FAllowed: FnMut(LootStoreItemContext) -> bool,
        FRandom: FnMut(u32, &mut R) -> LootItemRandomProperties,
    {
        if let Some(item) =
            self.roll_with_context_like_cpp(loot_mode, rng, item_allowed, store_kind, entry)
        {
            add_generated_loot_item_like_cpp(
                generated,
                LootStoreItemContext {
                    store_kind,
                    entry,
                    item,
                },
                item_context,
                rng,
                item_template,
                random_properties,
            );
        }
    }

    pub(super) fn has_drop_for_personal_looter_like_cpp<FAllowed>(
        &self,
        store_kind: LootStoreKind,
        entry: u32,
        looter: ObjectGuid,
        item_allowed: &mut FAllowed,
    ) -> bool
    where
        FAllowed: FnMut(LootStoreItemContext, ObjectGuid) -> bool,
    {
        self.explicitly_chanced
            .iter()
            .chain(self.equal_chanced.iter())
            .any(|item| {
                item_allowed(
                    LootStoreItemContext {
                        store_kind,
                        entry,
                        item: *item,
                    },
                    looter,
                )
            })
    }

    pub(super) fn process_personal_root_like_cpp<R, FTemplate, FRandom>(
        &self,
        loot_mode: u16,
        rng: &mut R,
        generated: &mut Vec<GeneratedPersonalLootItem>,
        item_template: &mut FTemplate,
        random_properties: &mut FRandom,
        item_context: u8,
        looter: ObjectGuid,
        store_kind: LootStoreKind,
        entry: u32,
    ) where
        R: Rng + ?Sized,
        FTemplate: FnMut(u32) -> Option<LootItemTemplateMetadata>,
        FRandom: FnMut(u32, &mut R) -> LootItemRandomProperties,
    {
        if let Some(item) =
            self.roll_with_context_like_cpp(loot_mode, rng, |_| true, store_kind, entry)
        {
            add_generated_personal_loot_item_like_cpp(
                generated,
                looter,
                LootStoreItemContext {
                    store_kind,
                    entry,
                    item,
                },
                item_context,
                rng,
                item_template,
                random_properties,
            );
        }
    }
}
