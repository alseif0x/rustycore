//! Build-70170 name-validation data, kept separate from name policy.
//!
//! The source path is `DB2Stores::LoadDB2`/`DB2Manager::LoadHotfixData` at
//! 02245dcd (`DB2Stores.cpp:1416-1457`, `Common.h:50-65`). This module only
//! composes complete DB2 rows and exposes the ordered source catalogs; Unicode
//! checks, normalization, profanity matching and SQL reserved-name policy
//! remain the world/name-rules owner.

mod load;
#[cfg(test)]
mod tests;

use crate::Db2HotfixRemovalStoreLikeCpp;
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub const TOTAL_LOCALES: u8 = 12;
pub const NONE_LOCALE: u8 = 9;
pub const ENGLISH_CHARSET: u8 = 2;

/// `NamesProfanity.db2` row. No Debug implementation: names are client data
/// and should not be rendered by generic diagnostics.
pub struct Profanity {
    pub id: u32,
    pub name: String,
    pub language: i8,
}

/// `NamesReserved.db2` row.
pub struct Reserved {
    pub id: u32,
    pub name: String,
}

/// `NamesReservedLocale.db2` row. Its eight-bit mask is local-only data; it
/// must not be merged into the global reserved-name list.
pub struct LocaleReserved {
    pub id: u32,
    pub name: String,
    pub locale_mask: u8,
}

/// `Cfg_Categories.db2` row. Only CreateCharsetMask is retained here.
pub struct Category {
    pub id: u32,
    pub creation_charset: u8,
}

/// Transient source batches. The caller supplies baseline, official SQL and
/// custom SQL separately to preserve C++ replacement order.
#[derive(Default)]
pub struct NameRecords {
    pub profanity: Vec<Profanity>,
    pub reserved: Vec<Reserved>,
    pub locale_reserved: Vec<LocaleReserved>,
    pub categories: Vec<Category>,
}

/// Immutable source projection used by the name-policy layer.
pub struct ForeverNames {
    profanity_by_locale: [Vec<String>; TOTAL_LOCALES as usize],
    reserved: Vec<String>,
    locale_reserved: [Vec<String>; TOTAL_LOCALES as usize],
    creation_charsets: BTreeMap<u32, u8>,
}

impl NameRecords {
    /// Read all four complete target DB2 files through the checked creation
    /// reader. A missing or incomplete file fails before a catalog is built.
    pub fn load(directory: &std::path::Path) -> Result<Self> {
        Self::load_from(directory)
    }

    /// Compose the checked build-70170 WDC5 baseline, official SQL and custom
    /// SQL, then apply only the final C++ `RecordRemoved` decision for each table.
    pub fn finish(
        self,
        official: Self,
        custom: Self,
        removals: &Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<ForeverNames> {
        // Runtime table hashes observed in the complete 70170 headers:
        // NamesProfanity DA82D96C, NamesReserved 25C1CB13,
        // NamesReservedLocale 3ACAE305, Cfg_Categories C7ED797D.
        let profanity = effective(
            self.profanity,
            official.profanity,
            custom.profanity,
            0xDA82_D96C,
            removals,
            |row| row.id,
            |row| {
                ensure!(
                    row.language == -1 || (0..TOTAL_LOCALES as i8).contains(&row.language),
                    "Invalid NamesProfanity language"
                );
                Ok(())
            },
        )?;
        let reserved = effective(
            self.reserved,
            official.reserved,
            custom.reserved,
            0x25C1_CB13,
            removals,
            |row| row.id,
            |_| Ok(()),
        )?;
        let locale_reserved_rows = effective(
            self.locale_reserved,
            official.locale_reserved,
            custom.locale_reserved,
            0x3ACA_E305,
            removals,
            |row| row.id,
            |_| Ok(()),
        )?;
        let categories = effective(
            self.categories,
            official.categories,
            custom.categories,
            0xC7ED_797D,
            removals,
            |row| row.id,
            |_| Ok(()),
        )?;

        let mut profanity_by_locale = std::array::from_fn(|_| Vec::new());
        for row in profanity.into_values() {
            if row.language == -1 {
                for locale in 0..TOTAL_LOCALES {
                    if locale != NONE_LOCALE {
                        profanity_by_locale[locale as usize].push(row.name.clone());
                    }
                }
            } else {
                profanity_by_locale[row.language as usize].push(row.name);
            }
        }

        let reserved = reserved
            .into_values()
            .map(|row| row.name)
            .collect::<Vec<_>>();
        let mut locale_reserved = std::array::from_fn(|_| Vec::new());
        for row in locale_reserved_rows.into_values() {
            for locale in 0..TOTAL_LOCALES {
                if locale < 8 && row.locale_mask & (1 << locale) != 0 {
                    locale_reserved[locale as usize].push(row.name.clone());
                }
            }
        }

        let creation_charsets = categories
            .into_values()
            .map(|row| (row.id, row.creation_charset))
            .collect();

        Ok(ForeverNames {
            profanity_by_locale,
            reserved,
            locale_reserved,
            creation_charsets,
        })
    }
}

impl ForeverNames {
    /// Return profanity entries for a valid locale. C++ has twelve locale
    /// slots; invalid values are not silently folded into the `none` slot.
    pub fn profanity(&self, locale: u8) -> Option<&[String]> {
        self.profanity_by_locale
            .get(locale as usize)
            .map(Vec::as_slice)
    }

    pub fn reserved(&self) -> &[String] {
        &self.reserved
    }

    /// Local reserved entries are separate from `reserved()`: the source has
    /// twelve locale slots, while NamesReservedLocale.db2 supplies only the
    /// first eight mask bits.
    pub fn locale_reserved(&self, locale: u8) -> Option<&[String]> {
        self.locale_reserved.get(locale as usize).map(Vec::as_slice)
    }

    /// Missing Cfg_Categories rows follow the C++ English fallback.
    pub fn creation_charset(&self, category: u32) -> u8 {
        self.creation_charsets
            .get(&category)
            .copied()
            .unwrap_or(ENGLISH_CHARSET)
    }

    /// Consume these primitive projections into the compiled name-rule owner
    /// without retaining a second catalog of pattern strings at runtime.
    pub fn charset_categories(&self) -> impl Iterator<Item = (u32, u8)> + '_ {
        self.creation_charsets.iter().map(|(&id, &mask)| (id, mask))
    }
}

fn effective<T, Id, Validate>(
    baseline: Vec<T>,
    official: Vec<T>,
    custom: Vec<T>,
    table_hash: u32,
    removals: &Db2HotfixRemovalStoreLikeCpp,
    id: Id,
    validate: Validate,
) -> Result<BTreeMap<u32, T>>
where
    Id: Fn(&T) -> u32,
    Validate: Fn(&T) -> Result<()>,
{
    let mut records = BTreeMap::new();
    for (source, batch) in [
        ("baseline", baseline),
        ("official", official),
        ("custom", custom),
    ] {
        let mut seen = BTreeSet::new();
        for record in batch {
            let record_id = id(&record);
            ensure!(
                seen.insert(record_id),
                "Duplicate name DB2 ID {record_id} in {source} batch"
            );
            records.insert(record_id, record);
        }
    }
    records.retain(|&record_id, _| {
        !i32::try_from(record_id)
            .ok()
            .is_some_and(|id| removals.contains_like_cpp(table_hash, id))
    });
    for record in records.values() {
        validate(record)?;
    }
    Ok(records)
}
