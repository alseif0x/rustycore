//! Process-local target identity cache used by CMSG_QUERY_PLAYER_NAMES.
//!
//! TrinityCore loads this projection once through `CharacterCache` and keeps
//! it current as character administration succeeds. The cache is deliberately
//! limited to the fields consumed by `PlayerGuidLookupData`; it is not a second
//! Player store and it does not contain gameplay state.

use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterIdentityCacheEntryLikeCpp {
    pub guid_low: u64,
    pub name: String,
    pub account_id: u32,
    pub race: u8,
    pub class: u8,
    pub sex: u8,
    pub level: u8,
    pub is_deleted: bool,
}

/// Canonical process-local identity projection for target-name queries.
///
/// The write methods are used only by the startup loader and the character
/// administration adapter after a committed mutation. Readers receive cloned
/// rows, so no lock or cache entry escapes into a packet or async operation.
pub struct CharacterIdentityCacheLikeCpp {
    entries: RwLock<HashMap<u64, CharacterIdentityCacheEntryLikeCpp>>,
    battlenet_by_game_account: RwLock<HashMap<u32, u32>>,
}

impl Default for CharacterIdentityCacheLikeCpp {
    fn default() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            battlenet_by_game_account: RwLock::new(HashMap::new()),
        }
    }
}

impl CharacterIdentityCacheLikeCpp {
    pub fn replace_all(
        &self,
        entries: impl IntoIterator<Item = CharacterIdentityCacheEntryLikeCpp>,
        battlenet_by_game_account: impl IntoIterator<Item = (u32, u32)>,
    ) -> usize {
        let entries = entries
            .into_iter()
            .map(|entry| (entry.guid_low, entry))
            .collect::<HashMap<_, _>>();
        let count = entries.len();
        *self
            .entries
            .write()
            .expect("character identity cache lock is not poisoned") = entries;
        *self
            .battlenet_by_game_account
            .write()
            .expect("Battle.net account cache lock is not poisoned") =
            battlenet_by_game_account.into_iter().collect();
        count
    }

    pub fn get(&self, guid_low: u64) -> Option<CharacterIdentityCacheEntryLikeCpp> {
        self.entries
            .read()
            .expect("character identity cache lock is not poisoned")
            .get(&guid_low)
            .cloned()
    }

    pub fn battlenet_account_id(&self, account_id: u32) -> u32 {
        self.battlenet_by_game_account
            .read()
            .expect("Battle.net account cache lock is not poisoned")
            .get(&account_id)
            .copied()
            .unwrap_or(0)
    }

    pub fn upsert(&self, entry: CharacterIdentityCacheEntryLikeCpp) {
        self.entries
            .write()
            .expect("character identity cache lock is not poisoned")
            .insert(entry.guid_low, entry);
    }

    pub fn remove(&self, guid_low: u64) {
        self.entries
            .write()
            .expect("character identity cache lock is not poisoned")
            .remove(&guid_low);
    }

    pub fn update_name(&self, guid_low: u64, name: impl Into<String>) {
        if let Some(entry) = self
            .entries
            .write()
            .expect("character identity cache lock is not poisoned")
            .get_mut(&guid_low)
        {
            entry.name = name.into();
        }
    }

    pub fn update_identity(
        &self,
        guid_low: u64,
        name: impl Into<String>,
        race: Option<u8>,
        sex: Option<u8>,
    ) {
        if let Some(entry) = self
            .entries
            .write()
            .expect("character identity cache lock is not poisoned")
            .get_mut(&guid_low)
        {
            entry.name = name.into();
            if let Some(race) = race {
                entry.race = race;
            }
            if let Some(sex) = sex {
                entry.sex = sex;
            }
        }
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries
            .read()
            .expect("character identity cache lock is not poisoned")
            .len()
    }
}
