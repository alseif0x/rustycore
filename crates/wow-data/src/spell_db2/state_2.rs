//! DB2 spell entry stores state definitions, part 2 of 3.
//!
//! Separated from the spell_db2.rs root under #646. Behaviour is preserved.

use super::*;

impl SpellNameStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellName.db2", |id, idx, r| {
            SpellNameEntry {
                id,
                name: r.get_field_string(idx, 0),
            }
        })
    }

    /// Apply C++ `DB2StorageBase::LoadFromDB` ordering for `SpellName.db2`:
    /// official rows first, then custom rows. The name locale overlays remain
    /// separate; callers that only need `HasRecord` receive the effective ID
    /// set after both base-table passes.
    pub fn apply_hotfix_rows_like_cpp(&mut self, rows: Vec<SpellNameEntry>) -> usize {
        let count = rows.len();
        for entry in rows {
            self.overlay_hotfix_row_like_cpp(entry.id, entry.name);
        }
        count
    }

    /// Load the effective C++ `sSpellNameStore` used by `SpellMgr`.
    ///
    /// This keeps the full DB2 lifecycle in one API: file rows, official SQL
    /// replacements, custom SQL replacements, then final `hotfix_data`
    /// removals by `(TableHash, RecordID)`.
    pub fn load_effective_from_hotfix_rows_like_cpp(
        data_dir: &str,
        locale: &str,
        rows: Vec<SpellNameEntry>,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<(Self, SpellNameEffectiveLoadReportLikeCpp)> {
        let mut store = Self::load(data_dir, locale)?;
        let overlay_rows = store.apply_hotfix_rows_like_cpp(rows);
        let removed_rows = store.apply_hotfix_removals_like_cpp(removals)?;

        Ok((
            store,
            SpellNameEffectiveLoadReportLikeCpp {
                overlay_rows,
                removed_rows,
            },
        ))
    }

    pub(super) fn apply_hotfix_removals_like_cpp(
        &mut self,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<usize> {
        let table_hash = self
            .table_hash_like_cpp()
            .context("SpellName.db2 store is missing its WDC4 table hash")?;
        let before_removals = self.entries.len();
        self.entries
            .retain(|id, _| !removals.contains_like_cpp(table_hash, *id as i32));
        Ok(before_removals.saturating_sub(self.entries.len()))
    }

    pub(super) fn overlay_hotfix_row_like_cpp(&mut self, id: u32, name: String) {
        self.entries.insert(id, SpellNameEntry { id, name });
    }
}

impl SpellPowerStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellPower.db2", |_id, idx, r| {
            SpellPowerEntry {
                id: r.get_field_u32(idx, 0),
                order_index: r.get_field_u8(idx, 1),
                mana_cost: r.get_field_i32(idx, 2),
                mana_cost_per_level: r.get_field_i32(idx, 3),
                mana_per_second: r.get_field_i32(idx, 4),
                power_display_id: r.get_field_u32(idx, 5),
                alt_power_bar_id: r.get_field_i32(idx, 6),
                power_cost_pct: f32_field(r, idx, 7),
                power_cost_max_pct: f32_field(r, idx, 8),
                power_pct_per_second: f32_field(r, idx, 9),
                power_type: r.get_field_i8(idx, 10),
                required_aura_spell_id: r.get_field_i32(idx, 11),
                optional_cost: r.get_field_u32(idx, 12),
                spell_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }

    /// Load C++'s effective power-cost authority: DB2, official SQL, custom
    /// SQL, then final `hotfix_data` tombstones.
    pub fn load_effective_from_hotfix_rows_like_cpp(
        data_dir: &str,
        locale: &str,
        rows: impl IntoIterator<Item = SpellPowerEntry>,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        let mut store = Self::load(data_dir, locale)?;
        for entry in rows {
            store.overlay_effective_row_like_cpp(entry);
        }

        let table_hash = store
            .table_hash_like_cpp()
            .context("SpellPower.db2 store is missing its WDC4 table hash")?;
        store.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, removals);
        Ok(store)
    }

    pub(super) fn overlay_effective_row_like_cpp(&mut self, entry: SpellPowerEntry) {
        self.entries.insert(entry.id, entry);
    }

    pub(super) fn apply_hotfix_removals_with_table_hash_like_cpp(
        &mut self,
        table_hash: u32,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) {
        self.entries
            .retain(|record_id, _| !removals.contains_like_cpp(table_hash, *record_id as i32));
    }

    /// Iterate effective rows in ascending record-ID order.
    ///
    /// C++ applies `SpellPower` rows while iterating its DB2 storage, whose
    /// index is record-ID ordered. The Rust store is a `HashMap`, so callers
    /// that fold several rows into one `SpellInfo` must impose that order
    /// explicitly or a spell with multiple power rows resolves
    /// non-deterministically.
    pub fn entries_by_record_id_like_cpp(&self) -> Vec<&SpellPowerEntry> {
        let mut entries: Vec<&SpellPowerEntry> = self.entries.values().collect();
        entries.sort_by_key(|entry| entry.id);
        entries
    }
}

impl SpellPowerDifficultyStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "SpellPowerDifficulty.db2",
            |_id, idx, r| SpellPowerDifficultyEntry {
                id: r.get_field_u32(idx, 0),
                difficulty_id: r.get_field_u8(idx, 1),
                order_index: r.get_field_u8(idx, 2),
            },
        )
    }

    /// Load C++'s effective power-difficulty authority: DB2, official SQL,
    /// custom SQL, then final `hotfix_data` tombstones.
    pub fn load_effective_from_hotfix_rows_like_cpp(
        data_dir: &str,
        locale: &str,
        rows: impl IntoIterator<Item = SpellPowerDifficultyEntry>,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        let mut store = Self::load(data_dir, locale)?;
        for entry in rows {
            store.overlay_effective_row_like_cpp(entry);
        }

        let table_hash = store
            .table_hash_like_cpp()
            .context("SpellPowerDifficulty.db2 store is missing its WDC4 table hash")?;
        store.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, removals);
        Ok(store)
    }

    pub(super) fn overlay_effective_row_like_cpp(&mut self, entry: SpellPowerDifficultyEntry) {
        self.entries.insert(entry.id, entry);
    }

    pub(super) fn apply_hotfix_removals_with_table_hash_like_cpp(
        &mut self,
        table_hash: u32,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) {
        self.entries
            .retain(|record_id, _| !removals.contains_like_cpp(table_hash, *record_id as i32));
    }
}

impl SpellMiscStore {
    pub fn exact_entry_for_spell_difficulty_like_cpp(
        &self,
        spell_id: u32,
        difficulty_id: u8,
    ) -> Option<&SpellMiscEntry> {
        self.entries
            .values()
            .find(|entry| entry.spell_id == spell_id && entry.difficulty_id == difficulty_id)
    }

    pub fn entry_for_spell_difficulty_like_cpp(
        &self,
        spell_id: u32,
        difficulty_id: u8,
    ) -> Option<&SpellMiscEntry> {
        self.exact_entry_for_spell_difficulty_like_cpp(spell_id, difficulty_id)
            .or_else(|| self.exact_entry_for_spell_difficulty_like_cpp(spell_id, 0))
    }

    pub fn entry_for_spell_difficulty_with_fallback_like_cpp(
        &self,
        spell_id: u32,
        difficulty_id: u8,
        difficulties: Option<&crate::DifficultyStore>,
    ) -> Option<&SpellMiscEntry> {
        let mut current = difficulty_id;
        let mut visited = [false; 256];
        loop {
            if let Some(entry) = self.exact_entry_for_spell_difficulty_like_cpp(spell_id, current) {
                return Some(entry);
            }
            if current == 0 || visited[usize::from(current)] {
                return None;
            }
            visited[usize::from(current)] = true;
            current = difficulties
                .and_then(|store| store.get(u32::from(current)))
                .map_or(0, |difficulty| difficulty.fallback_difficulty_id);
        }
    }
}

impl SpellProcsPerMinuteStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellProcsPerMinute.db2", |id, idx, r| {
            SpellProcsPerMinuteEntry {
                id,
                base_proc_rate: f32_field(r, idx, 0),
                flags: r.get_field_u8(idx, 1),
            }
        })
    }
}

impl SpellProcsPerMinuteModStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "SpellProcsPerMinuteMod.db2",
            |id, idx, r| SpellProcsPerMinuteModEntry {
                id,
                mod_type: r.get_field_u8(idx, 0),
                param: r.get_field_i16(idx, 1),
                coeff: f32_field(r, idx, 2),
                spell_procs_per_minute_id: r.get_relationship_id(idx).unwrap_or(0),
            },
        )
    }
}

impl SpellRadiusStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellRadius.db2", |id, idx, r| {
            SpellRadiusEntry {
                id,
                radius: f32_field(r, idx, 0),
                radius_per_level: f32_field(r, idx, 1),
                radius_min: f32_field(r, idx, 2),
                radius_max: f32_field(r, idx, 3),
            }
        })
    }

    /// Load C++'s effective radius authority: DB2, official SQL, custom SQL,
    /// then final `hotfix_data` tombstones.
    pub fn load_effective_from_hotfix_rows_like_cpp(
        data_dir: &str,
        locale: &str,
        rows: impl IntoIterator<Item = SpellRadiusEntry>,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        let mut store = Self::load(data_dir, locale)?;
        for entry in rows {
            store.overlay_effective_row_like_cpp(entry);
        }
        store.apply_final_hotfix_removals_like_cpp(removals)?;
        Ok(store)
    }
}

impl SpellRangeStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellRange.db2", |id, idx, r| {
            SpellRangeEntry {
                id,
                display_name: r.get_field_string(idx, 0),
                display_name_short: r.get_field_string(idx, 1),
                flags: r.get_field_u8(idx, 2),
                range_min: f32_array::<2>(r, idx, 3),
                range_max: f32_array::<2>(r, idx, 4),
            }
        })
    }

    /// Load C++'s effective range authority: DB2, official SQL, custom SQL,
    /// then final `hotfix_data` tombstones.
    pub fn load_effective_from_hotfix_rows_like_cpp(
        data_dir: &str,
        locale: &str,
        rows: impl IntoIterator<Item = SpellRangeEntry>,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        let mut store = Self::load(data_dir, locale)?;
        for entry in rows {
            store.overlay_effective_row_like_cpp(entry);
        }

        let table_hash = store
            .table_hash_like_cpp()
            .context("SpellRange.db2 store is missing its WDC4 table hash")?;
        store.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, removals);
        Ok(store)
    }

    pub(super) fn overlay_effective_row_like_cpp(&mut self, entry: SpellRangeEntry) {
        self.entries.insert(entry.id, entry);
    }

    pub(super) fn apply_hotfix_removals_with_table_hash_like_cpp(
        &mut self,
        table_hash: u32,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) {
        self.entries
            .retain(|record_id, _| !removals.contains_like_cpp(table_hash, *record_id as i32));
    }
}

impl SpellReagentsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellReagents.db2", |id, idx, r| {
            SpellReagentsEntry {
                id,
                spell_id: r.get_field_i32(idx, 0),
                reagent: std::array::from_fn(|i| r.get_array_element(idx, 1, i, 32) as i32),
                reagent_count: std::array::from_fn(|i| r.get_array_element(idx, 2, i, 16) as i16),
            }
        })
    }
}

impl SpellReagentsCurrencyStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "SpellReagentsCurrency.db2",
            |id, idx, r| SpellReagentsCurrencyEntry {
                id,
                spell_id: r.get_relationship_id(idx).unwrap_or(0),
                currency_types_id: r.get_field_u16(idx, 1),
                currency_count: r.get_field_u16(idx, 2),
            },
        )
    }
}

impl SpellEffectDb2Store {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellEffect.db2", |id, idx, r| {
            SpellEffectDb2Entry {
                id,
                difficulty_id: r.get_field_i32(idx, 0),
                effect_index: r.get_field_i32(idx, 1),
                effect: r.get_field_u32(idx, 2),
                effect_amplitude: f32_field(r, idx, 3),
                effect_attributes: r.get_field_i32(idx, 4),
                effect_aura: r.get_field_i16(idx, 5),
                effect_aura_period: r.get_field_i32(idx, 6),
                effect_base_points: r.get_field_i32(idx, 7),
                effect_bonus_coefficient: f32_field(r, idx, 8),
                effect_chain_amplitude: f32_field(r, idx, 9),
                effect_chain_targets: r.get_field_i32(idx, 10),
                effect_die_sides: r.get_field_i32(idx, 11),
                effect_item_type: r.get_field_i32(idx, 12),
                effect_mechanic: r.get_field_i32(idx, 13),
                effect_points_per_resource: f32_field(r, idx, 14),
                effect_pos_facing: f32_field(r, idx, 15),
                effect_real_points_per_level: f32_field(r, idx, 16),
                effect_trigger_spell: r.get_field_i32(idx, 17),
                bonus_coefficient_from_ap: f32_field(r, idx, 18),
                pvp_multiplier: f32_field(r, idx, 19),
                coefficient: f32_field(r, idx, 20),
                variance: f32_field(r, idx, 21),
                resource_coefficient: f32_field(r, idx, 22),
                group_size_base_points_coefficient: f32_field(r, idx, 23),
                effect_misc_value: std::array::from_fn(|i| {
                    r.get_array_element(idx, 24, i, 32) as i32
                }),
                effect_radius_index: std::array::from_fn(|i| r.get_array_element(idx, 25, i, 32)),
                effect_spell_class_mask: std::array::from_fn(|i| {
                    r.get_array_element(idx, 26, i, 32)
                }),
                implicit_target: std::array::from_fn(|i| {
                    r.get_array_element(idx, 27, i, 16) as i16
                }),
                spell_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }

    /// Load the effective C++ `sSpellEffectStore` authority: DB2 file,
    /// official SQL replacements, custom SQL replacements, then final
    /// `hotfix_data` tombstones.
    pub fn load_effective_from_hotfix_rows_like_cpp(
        data_dir: &str,
        locale: &str,
        rows: impl IntoIterator<Item = SpellEffectDb2Entry>,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        let mut store = Self::load(data_dir, locale)?;
        for entry in rows {
            store.overlay_effective_row_like_cpp(entry);
        }

        let table_hash = store
            .table_hash_like_cpp()
            .context("SpellEffect.db2 store is missing its WDC4 table hash")?;
        store.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, removals);
        Ok(store)
    }

    pub(super) fn overlay_effective_row_like_cpp(&mut self, entry: SpellEffectDb2Entry) {
        self.entries.insert(entry.id, entry);
    }

    pub(super) fn apply_hotfix_removals_with_table_hash_like_cpp(
        &mut self,
        table_hash: u32,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) {
        self.entries
            .retain(|record_id, _| !removals.contains_like_cpp(table_hash, *record_id as i32));
    }
}

impl SpellEquippedItemsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellEquippedItems.db2", |id, idx, r| {
            SpellEquippedItemsEntry {
                id,
                spell_id: r.get_field_i32(idx, 0),
                equipped_item_class: r.get_field_i8(idx, 1),
                equipped_item_inv_types: r.get_field_i32(idx, 2),
                equipped_item_subclass: r.get_field_i32(idx, 3),
            }
        })
    }

    pub fn entry_for_spell_id_like_cpp(&self, spell_id: i32) -> Option<&SpellEquippedItemsEntry> {
        self.entries
            .values()
            .filter(|entry| entry.spell_id == spell_id)
            // C++ iterates DB2's ID-indexed storage in ascending record order
            // and assigns this DIFFICULTY_NONE slot for every matching row.
            // A malformed duplicate therefore resolves to the highest ID.
            .max_by_key(|entry| entry.id)
    }

    /// Load the effective C++ `sSpellEquippedItemsStore` authority: file rows,
    /// official SQL replacements, custom SQL replacements, then final
    /// `hotfix_data` tombstones.
    pub fn load_effective_from_hotfix_rows_like_cpp(
        data_dir: &str,
        locale: &str,
        rows: impl IntoIterator<Item = SpellEquippedItemsEntry>,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        let mut store = Self::load(data_dir, locale)?;
        for entry in rows {
            store.overlay_effective_row_like_cpp(entry);
        }

        let table_hash = store
            .table_hash_like_cpp()
            .context("SpellEquippedItems.db2 store is missing its WDC4 table hash")?;
        store.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, removals);
        Ok(store)
    }

    pub(super) fn overlay_effective_row_like_cpp(&mut self, entry: SpellEquippedItemsEntry) {
        self.entries.insert(entry.id, entry);
    }

    pub(super) fn apply_hotfix_removals_with_table_hash_like_cpp(
        &mut self,
        table_hash: u32,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) {
        self.entries
            .retain(|record_id, _| !removals.contains_like_cpp(table_hash, *record_id as i32));
    }
}

impl SpellFocusObjectStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellFocusObject.db2", |id, idx, r| {
            SpellFocusObjectEntry {
                id,
                name: r.get_field_string(idx, 0),
            }
        })
    }
}

impl SpellInterruptsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellInterrupts.db2", |id, idx, r| {
            SpellInterruptsEntry {
                id,
                difficulty_id: r.get_field_u8(idx, 0),
                interrupt_flags: r.get_field_i16(idx, 1),
                aura_interrupt_flags: std::array::from_fn(|i| {
                    r.get_array_element(idx, 2, i, 32) as i32
                }),
                channel_interrupt_flags: std::array::from_fn(|i| {
                    r.get_array_element(idx, 3, i, 32) as i32
                }),
                spell_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }

    /// Load C++'s effective interrupt authority: DB2, official SQL, custom SQL,
    /// then final `hotfix_data` tombstones.
    ///
    /// Composing it here rather than overlaying SQL onto an already hydrated
    /// `SpellStore` is what makes the tombstone pass reachable: a removed row
    /// must stop contributing its aura/channel masks instead of keeping the file
    /// values a row-keyed overlay would leave behind.
    pub fn load_effective_from_hotfix_rows_like_cpp(
        data_dir: &str,
        locale: &str,
        rows: impl IntoIterator<Item = SpellInterruptsEntry>,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        let mut store = Self::load(data_dir, locale)?;
        for entry in rows {
            store.overlay_effective_row_like_cpp(entry);
        }
        store.apply_final_hotfix_removals_like_cpp(removals)?;
        Ok(store)
    }
}

impl SpellItemEnchantmentConditionStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "SpellItemEnchantmentCondition.db2",
            |id, idx, r| SpellItemEnchantmentConditionEntry {
                id,
                lt_operand_type: std::array::from_fn(|i| r.get_array_element(idx, 0, i, 8) as u8),
                lt_operand: std::array::from_fn(|i| r.get_array_element(idx, 1, i, 32)),
                operator: std::array::from_fn(|i| r.get_array_element(idx, 2, i, 8) as u8),
                rt_operand_type: std::array::from_fn(|i| r.get_array_element(idx, 3, i, 8) as u8),
                rt_operand: std::array::from_fn(|i| r.get_array_element(idx, 4, i, 8) as u8),
                logic: std::array::from_fn(|i| r.get_array_element(idx, 5, i, 8) as u8),
            },
        )
    }
}

impl SpellKeyboundOverrideStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "SpellKeyboundOverride.db2",
            |id, idx, r| SpellKeyboundOverrideEntry {
                id,
                function: r.get_field_string(idx, 0),
                override_type: r.get_field_i8(idx, 1),
                data: r.get_field_i32(idx, 2),
                flags: r.get_field_i32(idx, 3),
            },
        )
    }
}

impl SpellLabelStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellLabel.db2", |id, idx, r| {
            SpellLabelEntry {
                id,
                label_id: r.get_field_u32(idx, 0),
                spell_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl SpellLearnSpellStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellLearnSpell.db2", |id, idx, r| {
            SpellLearnSpellEntry {
                id,
                spell_id: r.get_field_i32(idx, 0),
                learn_spell_id: r.get_field_i32(idx, 1),
                overrides_spell_id: r.get_field_i32(idx, 2),
            }
        })
    }
}

impl SpellLevelsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellLevels.db2", |id, idx, r| {
            SpellLevelsEntry {
                id,
                difficulty_id: r.get_field_u8(idx, 0),
                base_level: r.get_field_i16(idx, 1),
                max_level: r.get_field_i16(idx, 2),
                spell_level: r.get_field_i16(idx, 3),
                max_passive_aura_level: r.get_field_u8(idx, 4),
                spell_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }

    pub fn entry_for_spell_difficulty_like_cpp(
        &self,
        spell_id: u32,
        difficulty_id: u8,
    ) -> Option<&SpellLevelsEntry> {
        self.entries
            .values()
            .find(|entry| entry.spell_id == spell_id && entry.difficulty_id == difficulty_id)
    }
}

impl SpellMiscStore {
    pub fn get_by_spell_id(&self, spell_id: u32) -> Option<&SpellMiscEntry> {
        self.entries
            .values()
            .find(|entry| entry.spell_id == spell_id)
    }

    /// C++ `SpellInfo::IsAutocastable`.
    pub fn is_autocastable_like_cpp(&self, spell_id: u32) -> bool {
        self.get_by_spell_id(spell_id)
            .is_none_or(SpellMiscEntry::is_autocastable_like_cpp)
    }

    pub fn is_passive_like_cpp(&self, spell_id: u32) -> bool {
        self.get_by_spell_id(spell_id)
            .is_some_and(SpellMiscEntry::is_passive_like_cpp)
    }

    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellMisc.db2", |id, idx, r| {
            SpellMiscEntry {
                id,
                attributes: std::array::from_fn(|i| r.get_array_element(idx, 0, i, 32) as i32),
                difficulty_id: r.get_field_u8(idx, 1),
                casting_time_index: r.get_field_u16(idx, 2),
                duration_index: r.get_field_u16(idx, 3),
                range_index: r.get_field_u16(idx, 4),
                school_mask: r.get_field_u8(idx, 5),
                speed: f32_field(r, idx, 6),
                launch_delay: f32_field(r, idx, 7),
                min_duration: f32_field(r, idx, 8),
                spell_icon_file_data_id: r.get_field_i32(idx, 9),
                active_icon_file_data_id: r.get_field_i32(idx, 10),
                content_tuning_id: r.get_field_i32(idx, 11),
                show_future_spell_player_condition_id: r.get_field_i32(idx, 12),
                spell_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }

    /// Load the effective C++ `sSpellMiscStore` authority: DB2 file,
    /// official SQL replacements, custom SQL replacements, then final
    /// `hotfix_data` tombstones.
    pub fn load_effective_from_hotfix_rows_like_cpp(
        data_dir: &str,
        locale: &str,
        rows: impl IntoIterator<Item = SpellMiscEntry>,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        let mut store = Self::load(data_dir, locale)?;
        for entry in rows {
            store.overlay_effective_row_like_cpp(entry);
        }

        let table_hash = store
            .table_hash_like_cpp()
            .context("SpellMisc.db2 store is missing its WDC4 table hash")?;
        store.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, removals);
        Ok(store)
    }

    pub(super) fn overlay_effective_row_like_cpp(&mut self, entry: SpellMiscEntry) {
        self.entries.insert(entry.id, entry);
    }

    pub(super) fn apply_hotfix_removals_with_table_hash_like_cpp(
        &mut self,
        table_hash: u32,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) {
        self.entries
            .retain(|record_id, _| !removals.contains_like_cpp(table_hash, *record_id as i32));
    }
}

impl SpellScalingStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellScaling.db2", |id, idx, r| {
            SpellScalingEntry {
                id,
                spell_id: r.get_field_i32(idx, 0),
                class: r.get_field_i32(idx, 1),
                min_scaling_level: r.get_field_u32(idx, 2),
                max_scaling_level: r.get_field_u32(idx, 3),
                scales_from_item_level: r.get_field_i16(idx, 4),
            }
        })
    }
}

impl SpellShapeshiftStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellShapeshift.db2", |id, idx, r| {
            SpellShapeshiftEntry {
                id,
                spell_id: r.get_field_i32(idx, 0),
                stance_bar_order: r.get_field_i8(idx, 1),
                shapeshift_exclude: std::array::from_fn(|i| {
                    r.get_array_element(idx, 2, i, 32) as i32
                }),
                shapeshift_mask: std::array::from_fn(|i| r.get_array_element(idx, 3, i, 32) as i32),
            }
        })
    }

    /// Load C++'s effective shapeshift authority: DB2, official SQL, custom SQL,
    /// then final `hotfix_data` tombstones. A tombstoned row must stop gating
    /// the spell's stance requirement entirely.
    pub fn load_effective_from_hotfix_rows_like_cpp(
        data_dir: &str,
        locale: &str,
        rows: impl IntoIterator<Item = SpellShapeshiftEntry>,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        let mut store = Self::load(data_dir, locale)?;
        for entry in rows {
            store.overlay_effective_row_like_cpp(entry);
        }
        store.apply_final_hotfix_removals_like_cpp(removals)?;
        Ok(store)
    }
}

impl SpellShapeshiftFormStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellShapeshiftForm.db2", |id, idx, r| {
            SpellShapeshiftFormEntry {
                id,
                name: r.get_field_string(idx, 0),
                creature_type: r.get_field_i8(idx, 1),
                flags: r.get_field_i32(idx, 2),
                attack_icon_file_id: r.get_field_i32(idx, 3),
                bonus_action_bar: r.get_field_i8(idx, 4),
                combat_round_time: r.get_field_i16(idx, 5),
                damage_variance: f32_field(r, idx, 6),
                mount_type_id: r.get_field_u16(idx, 7),
                creature_display_id: std::array::from_fn(|i| r.get_array_element(idx, 8, i, 32)),
                preset_spell_id: std::array::from_fn(|i| r.get_array_element(idx, 9, i, 32)),
            }
        })
    }
}

impl SpellTargetRestrictionsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "SpellTargetRestrictions.db2",
            |id, idx, r| SpellTargetRestrictionsEntry {
                id,
                difficulty_id: r.get_field_u8(idx, 0),
                cone_degrees: f32_field(r, idx, 1),
                max_targets: r.get_field_u8(idx, 2),
                max_target_level: r.get_field_u32(idx, 3),
                target_creature_type: r.get_field_i16(idx, 4),
                targets: r.get_field_i32(idx, 5),
                width: f32_field(r, idx, 6),
                spell_id: r.get_relationship_id(idx).unwrap_or(0),
            },
        )
    }

    pub fn entries_for_spell_id_like_cpp(
        &self,
        spell_id: u32,
    ) -> impl Iterator<Item = &SpellTargetRestrictionsEntry> {
        self.entries
            .values()
            .filter(move |entry| entry.spell_id == spell_id)
    }

    /// Resolve the single `SpellInfo` field through C++'s requested
    /// difficulty followed by `FallbackDifficultyID` chain.
    pub fn resolved_for_difficulty_chain_like_cpp(
        &self,
        spell_id: u32,
        difficulty_chain: impl IntoIterator<Item = u32>,
    ) -> Option<&SpellTargetRestrictionsEntry> {
        difficulty_chain.into_iter().find_map(|difficulty_id| {
            let difficulty_id = u8::try_from(difficulty_id).ok()?;
            self.entries_for_spell_id_like_cpp(spell_id)
                .filter(|entry| entry.difficulty_id == difficulty_id)
                // C++'s ID-indexed DB2 iteration overwrites this difficulty
                // slot in ascending record order, so the highest ID wins.
                .max_by_key(|entry| entry.id)
        })
    }

    /// Compose C++'s final DB2 authority, including SQL replacements and
    /// final `hotfix_data` tombstones.
    pub fn load_effective_from_hotfix_rows_like_cpp(
        data_dir: &str,
        locale: &str,
        rows: impl IntoIterator<Item = SpellTargetRestrictionsEntry>,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        let mut store = Self::load(data_dir, locale)?;
        for entry in rows {
            store.entries.insert(entry.id, entry);
        }
        let table_hash = store
            .table_hash_like_cpp()
            .context("SpellTargetRestrictions.db2 store is missing its WDC4 table hash")?;
        store
            .entries
            .retain(|record_id, _| !removals.contains_like_cpp(table_hash, *record_id as i32));
        Ok(store)
    }
}

impl SpellTotemsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellTotems.db2", |id, idx, r| {
            SpellTotemsEntry {
                id,
                spell_id: r.get_field_i32(idx, 0),
                required_totem_category_id: std::array::from_fn(|i| r.get_array_u16(idx, 1, i)),
                totem: std::array::from_fn(|i| r.get_array_element(idx, 2, i, 32) as i32),
            }
        })
    }
}

impl SpellVisualStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellVisual.db2", |id, idx, r| {
            SpellVisualEntry {
                id,
                missile_cast_offset: f32_array::<3>(r, idx, 0),
                missile_impact_offset: f32_array::<3>(r, idx, 1),
                anim_event_sound_id: r.get_field_u32(idx, 2),
                flags: r.get_field_i32(idx, 3),
                missile_attachment: r.get_field_i8(idx, 4),
                missile_destination_attachment: r.get_field_i8(idx, 5),
                missile_cast_positioner_id: r.get_field_u32(idx, 6),
                missile_impact_positioner_id: r.get_field_u32(idx, 7),
                missile_targeting_kit: r.get_field_i32(idx, 8),
                hostile_spell_visual_id: r.get_field_u32(idx, 9),
                caster_spell_visual_id: r.get_field_u32(idx, 10),
                spell_visual_missile_set_id: r.get_field_u16(idx, 11),
                damage_number_delay: r.get_field_u16(idx, 12),
                low_violence_spell_visual_id: r.get_field_u32(idx, 13),
                raid_spell_visual_missile_set_id: r.get_field_u32(idx, 14),
                reduced_unexpected_camera_movement_spell_visual_id: r.get_field_i32(idx, 15),
                area_model: r.get_field_u16(idx, 16),
                has_missile: r.get_field_i8(idx, 17),
            }
        })
    }
}
