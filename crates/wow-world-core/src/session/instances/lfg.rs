use crate::session::state::SessionCore;

impl SessionCore {
    pub fn lfg_season_is_active_like_cpp(&self, _dungeon_id: u32) -> bool {
        // C++ delegates this to `LFGMgr::IsSeasonActive`, backed by holiday
        // state. The current Rust runtime has no live holiday manager wired
        // into LFG yet; inactive is the C++-safe default for seasonal rows.
        false
    }
}
