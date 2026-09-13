//! Locale projections loaded by C++ `PREPARE_LOCALE_STMT` for TraitMgr.

use std::collections::BTreeMap;

/// Localized fields for one effective `TraitDefinition` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitDefinitionLocaleEntry {
    pub id: u32,
    pub override_name: String,
    pub override_subtext: String,
    pub override_description: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TraitDefinitionLocaleStore {
    locale: String,
    entries: BTreeMap<u32, TraitDefinitionLocaleEntry>,
}

impl TraitDefinitionLocaleStore {
    pub fn from_entries_like_cpp(
        locale: impl Into<String>,
        entries: impl IntoIterator<Item = TraitDefinitionLocaleEntry>,
    ) -> Self {
        Self {
            locale: locale.into(),
            entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
        }
    }

    #[must_use]
    pub fn locale_like_cpp(&self) -> &str {
        &self.locale
    }

    #[must_use]
    pub fn get(&self, id: u32) -> Option<&TraitDefinitionLocaleEntry> {
        self.entries.get(&id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitCurrencySourceLocaleEntry {
    pub id: u32,
    pub requirement: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TraitCurrencySourceLocaleStore {
    locale: String,
    entries: BTreeMap<u32, TraitCurrencySourceLocaleEntry>,
}

impl TraitCurrencySourceLocaleStore {
    pub fn from_entries_like_cpp(
        locale: impl Into<String>,
        entries: impl IntoIterator<Item = TraitCurrencySourceLocaleEntry>,
    ) -> Self {
        Self {
            locale: locale.into(),
            entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
        }
    }

    #[must_use]
    pub fn locale_like_cpp(&self) -> &str {
        &self.locale
    }

    #[must_use]
    pub fn get(&self, id: u32) -> Option<&TraitCurrencySourceLocaleEntry> {
        self.entries.get(&id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
