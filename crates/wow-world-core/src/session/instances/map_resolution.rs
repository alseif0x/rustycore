//! Resolving the canonical map and manager the represented Session is
//! currently attached to.
//!
//! Moved out of the Session root under #613. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use crate::session::SessionState;

impl crate::session::state::SessionCore {
    pub fn has_world_map_manager_like_cpp(&self) -> bool {
        self.map_manager.is_some()
    }

    pub fn current_canonical_player_map_key_like_cpp(&self) -> Option<wow_map::MapKey> {
        let guid = self.player_guid()?;
        let manager = self.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        if let Some(handle) = self.player_handle_like_cpp
            && handle.guid() == guid
        {
            return match manager.player_residence_like_cpp(handle)? {
                wow_map::PlayerResidenceLikeCpp::Active(key) => Some(key),
                wow_map::PlayerResidenceLikeCpp::Detached => None,
            };
        }
        let mut key = None;
        let mut ambiguous = false;
        manager.do_for_all_maps(|managed| {
            if managed.map().get_typed_player(guid).is_none() {
                return;
            }
            if key.is_some() {
                ambiguous = true;
            } else {
                key = Some(wow_map::MapKey::new(
                    managed.map_id(),
                    managed.instance_id(),
                ));
            }
        });
        (!ambiguous).then_some(key).flatten()
    }

    /// Resolve object access through the player's exact canonical `Map`, as
    /// C++ `ObjectAccessor::GetCreature/GetGameObject(WorldObject const&, ...)`
    /// does through `world_object.GetMap()`. During pre-player bootstrap and
    /// focused represented tests, accept a sole map for the expected map id;
    /// multiple instances without a canonical Player fail closed.
    pub fn canonical_object_lookup_map_key_like_cpp(
        &self,
        fallback_map_id: u32,
    ) -> Option<wow_map::MapKey> {
        if let Some(map_key) = self.current_canonical_player_map_key_like_cpp() {
            return Some(map_key);
        }

        let manager = self.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        if let Some(player_guid) = self.player_guid() {
            let mut player_map_count = 0usize;
            manager.do_for_all_maps(|managed| {
                if managed.map().get_typed_player(player_guid).is_some() {
                    player_map_count = player_map_count.saturating_add(1);
                }
            });
            // A player temporarily visible in two canonical maps is a
            // transfer boundary, not permission to choose one by iteration
            // order. Object-owned mutations fail closed until ownership is
            // unambiguous.
            if player_map_count != 0 || self.state == SessionState::LoggedIn {
                return None;
            }
        }
        let mut fallback_key = None;
        let mut ambiguous = false;
        manager.do_for_all_maps(|managed| {
            if managed.map_id() != fallback_map_id {
                return;
            }
            if fallback_key.is_some() {
                ambiguous = true;
            } else {
                fallback_key = Some(wow_map::MapKey::new(
                    managed.map_id(),
                    managed.instance_id(),
                ));
            }
        });
        (!ambiguous).then_some(fallback_key).flatten()
    }
}
