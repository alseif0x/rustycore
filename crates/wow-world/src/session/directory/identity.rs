//! Connected player identity projection.

#[derive(Clone, Debug)]
pub struct PlayerDirectoryIdentityLikeCpp {
    pub player_name: String,
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
    ) -> Self {
        Self {
            player_name: player_name.into(),
            account_id,
            battlenet_account_id,
            recruiter_id,
            race,
            class,
            sex,
            active_expansion,
        }
    }
}
