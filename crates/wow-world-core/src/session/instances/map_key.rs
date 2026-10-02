//! Map key resolution used by the represented Session.
//!
//! Moved out of the Session root under #613. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

impl crate::session::state::SessionCore {
    pub fn player_map_id_like_cpp(&self) -> u16 {
        self.current_map_id
    }

    /// The legacy map facade must follow the same map instance that owns the
    /// canonical Player. Instance `0` remains only the bootstrap fallback for
    /// tests/runtime phases where no canonical Player has been materialized.
    pub fn current_legacy_runtime_map_key_like_cpp(&self) -> (u16, u32) {
        let fallback_map_id = self.player_map_id_like_cpp();
        let Some(map_key) = self.current_canonical_player_map_key_like_cpp() else {
            return (fallback_map_id, 0);
        };
        let Ok(map_id) = u16::try_from(map_key.map_id) else {
            return (fallback_map_id, 0);
        };
        (map_id, map_key.instance_id)
    }
}
