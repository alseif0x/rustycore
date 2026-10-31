//! Connected player identity projection.

use wow_map::PlayerHandle;

#[derive(Clone, Debug)]
pub struct PlayerDirectoryIdentityLikeCpp {
    pub player_name: String,
    /// The locale selected by this connected session at registration time.
    pub locale: String,
    /// The originating canonical incarnation. Registration moves this into the
    /// registry's private entry binding; legacy fixture identities may omit it.
    player_handle: Option<PlayerHandle>,
    pub account_id: u32,
    /// Battle.net account owning `account_id`, when the login service supplied
    /// the relation. This is a connected-session projection only; offline
    /// targets come from the character identity cache.
    pub battlenet_account_id: u32,
    pub recruiter_id: u32,
    pub race: u8,
    pub class: u8,
    pub sex: u8,
    pub active_expansion: u8,
}

impl PlayerDirectoryIdentityLikeCpp {
    pub fn new(
        player_name: impl Into<String>,
        account_id: u32,
        recruiter_id: u32,
        race: u8,
        class: u8,
        sex: u8,
        active_expansion: u8,
    ) -> Self {
        Self::new_with_bnet(
            player_name,
            account_id,
            0,
            recruiter_id,
            race,
            class,
            sex,
            active_expansion,
            "enUS",
            None,
        )
    }

    pub fn new_with_bnet(
        player_name: impl Into<String>,
        account_id: u32,
        battlenet_account_id: u32,
        recruiter_id: u32,
        race: u8,
        class: u8,
        sex: u8,
        active_expansion: u8,
        locale: impl Into<String>,
        player_handle: Option<PlayerHandle>,
    ) -> Self {
        Self {
            player_name: player_name.into(),
            locale: locale.into(),
            player_handle,
            account_id,
            battlenet_account_id,
            recruiter_id,
            race,
            class,
            sex,
            active_expansion,
        }
    }

    pub(crate) fn take_player_handle(&mut self) -> Option<PlayerHandle> {
        self.player_handle.take()
    }
}
