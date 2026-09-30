//! Prepared handoff into a checked, existing canonical map.
use super::*;
use crate::spawn::{RespawnTransfer, RespawnTransferError};

impl crate::manager::MapManager {
    pub fn accept_respawn_transfer<'fence>(&mut self, incoming: RespawnTransfer<'fence>)
        -> Result<(), (RespawnTransferError, RespawnTransfer<'fence>)>
    {
        let key = incoming.key();
        if self.actor_respawn_is_active()
            || self.tick_coordination_like_cpp() != crate::MapTickCoordinationStateLikeCpp::Idle {
            return Err((RespawnTransferError::MapBusy, incoming));
        }
        let Some(incarnation) = self.map_incarnation_like_cpp(key) else {
            return Err((RespawnTransferError::MissingMap, incoming));
        };
        if incarnation != incoming.incarnation() {
            return Err((RespawnTransferError::StaleIncarnation, incoming));
        }
        let Some(managed) = self.find_map_mut(key.map_id, key.instance_id) else {
            return Err((RespawnTransferError::MissingMap, incoming));
        };
        managed.map_mut().accept_respawn_transfer(incoming, incarnation)
    }
}

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where Terrain: TerrainGridLoader, Lifecycle: GridLifecycle,
{
    pub(crate) fn accept_respawn_transfer<'fence>(
        &mut self, incoming: RespawnTransfer<'fence>, incarnation: u64,
    ) -> Result<(), (RespawnTransferError, RespawnTransfer<'fence>)> {
        let key = crate::MapKey::new(self.map_id(), self.instance_id());
        self.respawn_store.accept_transfer(incoming, key, incarnation)
    }
}
