// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadTrinityStrings` data model.

use std::collections::HashMap;

use tracing::warn;

pub const LANG_LEVEL_MINREQUIRED_LIKE_CPP: u32 = 49;
pub const LANG_LEVEL_MINREQUIRED_AND_ITEM_LIKE_CPP: u32 = 50;

const DEFAULT_LOCALE_INDEX_LIKE_CPP: usize = 0;
const OLD_TOTAL_LOCALES_LIKE_CPP: usize = 9;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrinityStringEntryLikeCpp {
    pub entry: u32,
    pub content: [String; OLD_TOTAL_LOCALES_LIKE_CPP],
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TrinityStringStoreLikeCpp {
    by_entry: HashMap<u32, TrinityStringEntryLikeCpp>,
}

impl TrinityStringStoreLikeCpp {
    pub fn from_entries_like_cpp(
        entries: impl IntoIterator<Item = TrinityStringEntryLikeCpp>,
    ) -> Self {
        Self {
            by_entry: entries
                .into_iter()
                .map(|entry| (entry.entry, entry))
                .collect(),
        }
    }

    pub fn get_like_cpp(&self, entry: u32, locale: &str) -> &str {
        let Some(trinity_string) = self.by_entry.get(&entry) else {
            warn!(
                target: "sql.sql",
                "Trinity string entry {entry} not found in DB."
            );
            return "<error>";
        };

        let locale_idx = locale_index_like_cpp(locale);
        if let Some(localized) = trinity_string.content.get(locale_idx)
            && !localized.is_empty()
        {
            return localized;
        }
        trinity_string.content[DEFAULT_LOCALE_INDEX_LIKE_CPP].as_str()
    }

    pub fn len(&self) -> usize {
        self.by_entry.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_entry.is_empty()
    }
}

pub fn locale_index_like_cpp(locale: &str) -> usize {
    match locale {
        "koKR" => 1,
        "frFR" => 2,
        "deDE" => 3,
        "zhCN" => 4,
        "zhTW" => 5,
        "esES" => 6,
        "esMX" => 7,
        "ruRU" => 8,
        _ => DEFAULT_LOCALE_INDEX_LIKE_CPP,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trinity_entry(entry: u32, default: &str, es: &str) -> TrinityStringEntryLikeCpp {
        let mut content: [String; OLD_TOTAL_LOCALES_LIKE_CPP] =
            std::array::from_fn(|_| String::new());
        content[DEFAULT_LOCALE_INDEX_LIKE_CPP] = default.to_string();
        content[6] = es.to_string();
        TrinityStringEntryLikeCpp { entry, content }
    }

    #[test]
    fn trinity_string_store_uses_locale_then_default_like_cpp() {
        let store = TrinityStringStoreLikeCpp::from_entries_like_cpp([trinity_entry(
            LANG_LEVEL_MINREQUIRED_LIKE_CPP,
            "Need level %u.",
            "Necesitas nivel %u.",
        )]);

        assert_eq!(
            store.get_like_cpp(LANG_LEVEL_MINREQUIRED_LIKE_CPP, "esES"),
            "Necesitas nivel %u."
        );
        assert_eq!(
            store.get_like_cpp(LANG_LEVEL_MINREQUIRED_LIKE_CPP, "frFR"),
            "Need level %u."
        );
    }

    #[test]
    fn trinity_string_store_missing_entry_returns_error_like_cpp() {
        let store = TrinityStringStoreLikeCpp::default();

        assert_eq!(store.get_like_cpp(12345, "esES"), "<error>");
    }
}
