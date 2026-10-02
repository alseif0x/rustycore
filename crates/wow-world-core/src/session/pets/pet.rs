use crate::session::state::SessionCore;
use wow_core::{ObjectGuid, Position};
use wow_entities::{AccessorObjectKind, Pet};

impl SessionCore {
    pub fn with_canonical_pet_mut_like_cpp<R>(
        &self,
        pet_guid: ObjectGuid,
        mutate: impl FnOnce(&mut Pet) -> R,
    ) -> Option<R> {
        let mut manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
        let mut mutate = Some(mutate);
        let mut result = None;
        manager.do_for_all_maps_mut(|managed| {
            if result.is_some() {
                return;
            }
            if let Some(pet) = managed.map_mut().get_typed_pet_mut(pet_guid) {
                result = Some(mutate.take().expect("pet mutation consumed once")(pet));
            }
        });
        result
    }

    pub fn represented_pet_position_like_cpp(&self, pet_guid: ObjectGuid) -> Option<Position> {
        let map_id = u32::from(self.player_map_id_like_cpp());
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
        let managed = manager.find_map(map_id, instance_id)?;
        managed.map().with_world_object_by_kinds_like_cpp(
            pet_guid,
            &[AccessorObjectKind::Pet],
            |object| object.position(),
        )
    }
}
