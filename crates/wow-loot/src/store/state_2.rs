//! Loot store definitions and templates state definitions, part 2 of 4.
//!
//! Separated from the lib.rs root under #642. Behaviour is preserved.

use super::*;

impl LootStore {
    #[must_use]
    pub fn new(definition: LootStoreDefinition) -> Self {
        Self {
            definition,
            templates: HashMap::new(),
        }
    }

    #[must_use]
    pub fn for_kind_like_cpp(kind: LootStoreKind) -> Self {
        Self::new(kind.definition_like_cpp())
    }

    #[must_use]
    pub const fn definition(&self) -> LootStoreDefinition {
        self.definition
    }

    #[must_use]
    pub fn templates(&self) -> &HashMap<u32, LootTemplate> {
        &self.templates
    }

    #[must_use]
    pub fn have_loot_for(&self, loot_id: u32) -> bool {
        self.templates.contains_key(&loot_id)
    }

    #[must_use]
    pub fn get_loot_for(&self, loot_id: u32) -> Option<&LootTemplate> {
        self.templates.get(&loot_id)
    }

    pub fn clear_like_cpp(&mut self) {
        self.templates.clear();
    }

    pub fn load_rows_like_cpp<I, F>(
        &mut self,
        rows: I,
        mut item_exists: F,
    ) -> Result<u32, LootStoreLoadError>
    where
        I: IntoIterator<Item = LootTemplateRow>,
        F: FnMut(u32) -> bool,
    {
        self.clear_like_cpp();
        let mut count = 0u32;

        for row in rows {
            if row.item.group_id >= 1 << 7 {
                return Err(LootStoreLoadError::InvalidGroupId {
                    table_name: self.definition.table_name,
                    entry: row.entry,
                    item_id: row.item.item_id,
                    group_id: row.item.group_id,
                });
            }

            let item_exists_for_row = row.item.reference != 0 || item_exists(row.item.item_id);
            if !row.item.is_valid_like_cpp(item_exists_for_row) {
                continue;
            }

            self.templates
                .entry(row.entry)
                .or_default()
                .add_entry_like_cpp(row.item);
            count = count.saturating_add(1);
        }

        Ok(count)
    }

    #[must_use]
    pub fn collect_loot_ids_like_cpp(&self) -> HashSet<u32> {
        self.templates.keys().copied().collect()
    }

    #[must_use]
    pub fn reference_uses_like_cpp(&self, store_kind: LootStoreKind) -> Vec<LootReferenceUse> {
        let mut uses = Vec::new();
        for (&entry, template) in &self.templates {
            template.append_reference_uses_like_cpp(store_kind, entry, &mut uses);
        }
        uses
    }

    #[must_use]
    pub fn condition_ids_for_fill_like_cpp(
        &self,
        loot_id: u32,
        store_kind: LootStoreKind,
        stores: &LootStores,
    ) -> Vec<LootConditionId> {
        let Some(template) = self.get_loot_for(loot_id) else {
            return Vec::new();
        };

        let mut ids = Vec::new();
        template.append_condition_ids_for_fill_like_cpp(stores, store_kind, loot_id, 0, &mut ids);
        ids.sort_by_key(|id| (id.source_type, id.source_group, id.source_entry));
        ids.dedup();
        ids
    }

    #[must_use]
    pub fn have_quest_loot_for_like_cpp(&self, loot_id: u32, stores: &LootStores) -> bool {
        self.get_loot_for(loot_id)
            .is_some_and(|template| template.has_quest_drop_like_cpp(stores, 0, &mut |_| true))
    }

    #[must_use]
    pub fn have_quest_loot_for_player_like_cpp<F>(
        &self,
        loot_id: u32,
        stores: &LootStores,
        mut player_has_quest_for_item: F,
    ) -> bool
    where
        F: FnMut(u32) -> bool,
    {
        self.get_loot_for(loot_id).is_some_and(|template| {
            template.has_quest_drop_for_player_like_cpp(stores, 0, &mut player_has_quest_for_item)
        })
    }

    pub fn fill_loot_like_cpp<R, FTemplate, FRate, FAllowed, FRandom>(
        &self,
        loot_id: u32,
        store_kind: LootStoreKind,
        stores: &LootStores,
        options: LootFillOptions,
        rng: &mut R,
        item_template: FTemplate,
        item_chance_rate: FRate,
        mut item_allowed: FAllowed,
        random_properties: FRandom,
    ) -> Result<Vec<GeneratedLootItem>, LootFillError>
    where
        R: Rng + ?Sized,
        FTemplate: FnMut(u32) -> Option<LootItemTemplateMetadata>,
        FRate: FnMut(LootStoreItem) -> f32,
        FAllowed: FnMut(LootStoreItem) -> bool,
        FRandom: FnMut(u32, &mut R) -> LootItemRandomProperties,
    {
        self.fill_loot_with_context_like_cpp(
            loot_id,
            store_kind,
            stores,
            options,
            rng,
            item_template,
            item_chance_rate,
            |context| item_allowed(context.item),
            random_properties,
        )
    }

    pub fn fill_loot_with_context_like_cpp<R, FTemplate, FRate, FAllowed, FRandom>(
        &self,
        loot_id: u32,
        store_kind: LootStoreKind,
        stores: &LootStores,
        options: LootFillOptions,
        rng: &mut R,
        mut item_template: FTemplate,
        mut item_chance_rate: FRate,
        mut item_allowed: FAllowed,
        mut random_properties: FRandom,
    ) -> Result<Vec<GeneratedLootItem>, LootFillError>
    where
        R: Rng + ?Sized,
        FTemplate: FnMut(u32) -> Option<LootItemTemplateMetadata>,
        FRate: FnMut(LootStoreItem) -> f32,
        FAllowed: FnMut(LootStoreItemContext) -> bool,
        FRandom: FnMut(u32, &mut R) -> LootItemRandomProperties,
    {
        let Some(template) = self.get_loot_for(loot_id) else {
            return Err(LootFillError::MissingLootTemplate { loot_id });
        };

        let mut generated = Vec::with_capacity(MAX_NR_LOOT_ITEMS_LIKE_CPP);
        template.process_like_cpp(
            stores,
            &options,
            rng,
            &mut generated,
            &mut item_template,
            &mut item_chance_rate,
            &mut item_allowed,
            &mut random_properties,
            store_kind,
            loot_id,
            0,
        );

        Ok(generated)
    }

    pub fn fill_personal_loot_with_context_like_cpp<R, FTemplate, FRate, FAllowed, FRandom>(
        &self,
        loot_id: u32,
        store_kind: LootStoreKind,
        stores: &LootStores,
        options: LootFillOptions,
        looters: &[ObjectGuid],
        rng: &mut R,
        mut item_template: FTemplate,
        mut item_chance_rate: FRate,
        mut item_allowed: FAllowed,
        mut random_properties: FRandom,
    ) -> Result<Vec<GeneratedPersonalLootItem>, LootFillError>
    where
        R: Rng + ?Sized,
        FTemplate: FnMut(u32) -> Option<LootItemTemplateMetadata>,
        FRate: FnMut(LootStoreItem) -> f32,
        FAllowed: FnMut(LootStoreItemContext, ObjectGuid) -> bool,
        FRandom: FnMut(u32, &mut R) -> LootItemRandomProperties,
    {
        let Some(template) = self.get_loot_for(loot_id) else {
            return Err(LootFillError::MissingLootTemplate { loot_id });
        };

        let mut generated = Vec::with_capacity(MAX_NR_LOOT_ITEMS_LIKE_CPP);
        template.process_personal_like_cpp(
            stores,
            &options,
            looters,
            rng,
            &mut generated,
            &mut item_template,
            &mut item_chance_rate,
            &mut item_allowed,
            &mut random_properties,
            store_kind,
            loot_id,
        );

        Ok(generated)
    }
}

impl LootStoreItem {
    #[must_use]
    pub fn is_reference(self) -> bool {
        self.reference > 0
    }

    #[must_use]
    pub fn is_valid_like_cpp(self, item_exists: bool) -> bool {
        if self.min_count == 0 {
            return false;
        }

        if self.reference == 0 {
            if self.item_id == 0 || !item_exists {
                return false;
            }

            if self.chance == 0.0 && self.group_id == 0 {
                return false;
            }

            if self.chance != 0.0 && self.chance < MIN_NON_ZERO_LOOT_CHANCE_LIKE_CPP {
                return false;
            }

            if self.max_count < self.min_count {
                return false;
            }

            return true;
        }

        self.needs_quest || self.chance != 0.0
    }

    #[must_use]
    pub fn can_roll_as_plain_entry_like_cpp(self, item_exists: bool, loot_mode: u16) -> bool {
        self.reference == 0
            && self.group_id == 0
            && self.is_valid_like_cpp(item_exists)
            && self.loot_mode & loot_mode != 0
    }

    #[must_use]
    pub fn can_roll_as_reference_entry_like_cpp(self, loot_mode: u16) -> bool {
        self.reference != 0 && self.is_valid_like_cpp(true) && self.loot_mode & loot_mode != 0
    }

    #[must_use]
    pub fn roll_like_cpp<R: Rng + ?Sized>(self, rng: &mut R, chance_multiplier: f32) -> bool {
        if self.chance >= 100.0 {
            return true;
        }

        roll_chance_like_cpp(rng, self.chance * chance_multiplier)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LootTemplate {
    pub(super) entries: Vec<LootStoreItem>,
    pub(super) groups: Vec<LootGroup>,
}
