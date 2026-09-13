//! Connected-session projection used by CMSG_QUERY_PLAYER_NAMES.

use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerNameQuerySnapshotLikeCpp {
    pub guid: ObjectGuid,
    pub name: String,
    pub account_id: u32,
    pub battlenet_account_id: u32,
    pub race: u8,
    pub class: u8,
    pub sex: u8,
    pub level: u8,
}

impl PlayerRegistry {
    /// Return the connected target projection consumed by C++
    /// `PlayerGuidLookupData`. The registry remains the sole owner of the
    /// live session identity; callers receive an owned snapshot only.
    #[must_use]
    pub fn player_name_query_snapshot_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<PlayerNameQuerySnapshotLikeCpp> {
        let entry = self.entries.get(&guid)?;
        Some(PlayerNameQuerySnapshotLikeCpp {
            guid,
            name: entry.identity.player_name.clone(),
            account_id: entry.identity.account_id,
            battlenet_account_id: entry.identity.battlenet_account_id,
            race: entry.identity.race,
            class: entry.identity.class,
            sex: entry.identity.sex,
            level: entry.placement.level,
        })
    }
}
