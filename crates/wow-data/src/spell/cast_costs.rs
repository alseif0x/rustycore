//! Effective reagent and currency costs attached to normal Player spell casts.

use std::collections::{BTreeMap, BTreeSet};

use crate::spell_db2::{SpellReagentsCurrencyEntry, SpellReagentsEntry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellReagentCostLikeCpp {
    pub record_id: u32,
    pub reagent: [i32; 8],
    pub reagent_count: [i16; 8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellCurrencyCostLikeCpp {
    pub record_id: u32,
    pub currency_types_id: u16,
    pub currency_count: u16,
}

/// One immutable effective source for the complete rows consumed at cast
/// preparation. `None` on SpellStore means the startup producer is unavailable;
/// an empty catalog means the source loaded and contained no rows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellCastCostStoreLikeCpp {
    reagents_by_spell: BTreeMap<i32, SpellReagentCostLikeCpp>,
    currencies_by_spell: BTreeMap<i32, Vec<SpellCurrencyCostLikeCpp>>,
}

impl SpellCastCostStoreLikeCpp {
    pub fn from_effective_rows_like_cpp(
        reagent_rows: impl IntoIterator<Item = SpellReagentsEntry>,
        currency_rows: impl IntoIterator<Item = SpellReagentsCurrencyEntry>,
    ) -> Self {
        let mut reagent_rows_by_id = BTreeMap::new();
        for row in reagent_rows {
            reagent_rows_by_id.insert(row.id, row);
        }
        let mut currency_rows_by_id = BTreeMap::new();
        for row in currency_rows {
            currency_rows_by_id.insert(row.id, row);
        }

        let mut store = Self::default();
        for row in reagent_rows_by_id.into_values() {
            store.reagents_by_spell.insert(
                row.spell_id,
                SpellReagentCostLikeCpp {
                    record_id: row.id,
                    reagent: row.reagent,
                    reagent_count: row.reagent_count,
                },
            );
        }
        for row in currency_rows_by_id.into_values() {
            store
                .currencies_by_spell
                .entry(row.spell_id as i32)
                .or_default()
                .push(SpellCurrencyCostLikeCpp {
                    record_id: row.id,
                    currency_types_id: row.currency_types_id,
                    currency_count: row.currency_count,
                });
        }
        store
    }

    pub fn reagent_for_spell_like_cpp(&self, spell_id: i32) -> Option<SpellReagentCostLikeCpp> {
        self.reagents_by_spell.get(&spell_id).copied()
    }

    pub fn currencies_for_spell_like_cpp(&self, spell_id: i32) -> &[SpellCurrencyCostLikeCpp] {
        self.currencies_by_spell
            .get(&spell_id)
            .map_or(&[], Vec::as_slice)
    }

    pub fn trainer_reagent_ids_like_cpp(&self) -> BTreeMap<u32, [i32; 8]> {
        self.reagents_by_spell
            .iter()
            .filter_map(|(&spell_id, cost)| {
                let raw_spell_id = spell_id as u32;
                (raw_spell_id != 0).then_some((raw_spell_id, cost.reagent))
            })
            .collect()
    }

    pub fn from_effective_rows_and_removals_like_cpp(
        base_reagents: impl IntoIterator<Item = SpellReagentsEntry>,
        official_reagents: impl IntoIterator<Item = SpellReagentsEntry>,
        custom_reagents: impl IntoIterator<Item = SpellReagentsEntry>,
        removed_reagent_ids: impl IntoIterator<Item = i32>,
        base_currencies: impl IntoIterator<Item = SpellReagentsCurrencyEntry>,
        official_currencies: impl IntoIterator<Item = SpellReagentsCurrencyEntry>,
        custom_currencies: impl IntoIterator<Item = SpellReagentsCurrencyEntry>,
        removed_currency_ids: impl IntoIterator<Item = i32>,
    ) -> Self {
        fn overlay<T: Copy>(
            base: impl IntoIterator<Item = T>,
            official: impl IntoIterator<Item = T>,
            custom: impl IntoIterator<Item = T>,
            record_id: impl Fn(T) -> u32,
            removals: impl IntoIterator<Item = i32>,
        ) -> Vec<T> {
            let mut rows = BTreeMap::new();
            rows.extend(base.into_iter().map(|row| (record_id(row), row)));
            rows.extend(official.into_iter().map(|row| (record_id(row), row)));
            rows.extend(custom.into_iter().map(|row| (record_id(row), row)));
            let removals = removals.into_iter().collect::<BTreeSet<_>>();
            rows.into_iter()
                .filter_map(|(id, row)| (!removals.contains(&(id as i32))).then_some(row))
                .collect()
        }

        Self::from_effective_rows_like_cpp(
            overlay(
                base_reagents,
                official_reagents,
                custom_reagents,
                |row: SpellReagentsEntry| row.id,
                removed_reagent_ids,
            ),
            overlay(
                base_currencies,
                official_currencies,
                custom_currencies,
                |row: SpellReagentsCurrencyEntry| row.id,
                removed_currency_ids,
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reagent(id: u32, spell_id: i32, reagent: i32) -> SpellReagentsEntry {
        SpellReagentsEntry {
            id,
            spell_id,
            reagent: [reagent, 0, 0, 0, 0, 0, 0, 0],
            reagent_count: [7, 0, 0, 0, 0, 0, 0, 0],
        }
    }

    #[test]
    fn trainer_reagent_view_preserves_high_spell_id_bits_and_counts_remain_in_runtime_row() {
        let spell_id = 0xF123_4567u32 as i32;
        let store = SpellCastCostStoreLikeCpp::from_effective_rows_like_cpp(
            [reagent(9, spell_id, 17123)],
            [],
        );

        assert_eq!(
            store
                .trainer_reagent_ids_like_cpp()
                .get(&spell_id_as_u32(spell_id)),
            Some(&[17123, 0, 0, 0, 0, 0, 0, 0])
        );
        assert_eq!(
            store
                .reagent_for_spell_like_cpp(spell_id)
                .unwrap()
                .reagent_count[0],
            7
        );
    }

    #[test]
    fn high_bit_record_removal_matches_raw_update_field_identity() {
        let record_id = 0xF000_0001u32;
        let store = SpellCastCostStoreLikeCpp::from_effective_rows_and_removals_like_cpp(
            [reagent(record_id, 444, 12)],
            [],
            [],
            [record_id as i32],
            [],
            [],
            [],
            [],
        );

        assert_eq!(store.reagent_for_spell_like_cpp(444), None);
    }

    const fn spell_id_as_u32(spell_id: i32) -> u32 {
        spell_id as u32
    }
}
