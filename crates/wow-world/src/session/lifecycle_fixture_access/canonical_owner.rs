//! Original fixture owner composition; stale/foreign residence remains an error.

use super::*;

pub fn insert_character_fixture_player_into_canonical_map(
    session: &WorldSession,
    canonical: &SharedCanonicalMapManager,
    map_id: u32,
    instance_id: u32,
) {
    if let Some(handle) = session.player_handle_like_cpp {
        let position = session
            .player_position_like_cpp()
            .expect("canonical Player fixture position");
        let key = wow_map::MapKey::new(map_id, instance_id);
        let mut manager = canonical.lock().unwrap();
        manager.create_world_map(map_id, instance_id);
        match manager.player_residence_like_cpp(handle) {
            Some(wow_map::PlayerResidenceLikeCpp::Detached) => manager
                .attach_player_like_cpp(handle, key, position)
                .expect("attach detached canonical Player fixture"),
            Some(wow_map::PlayerResidenceLikeCpp::Active(current)) if current == key => {}
            Some(wow_map::PlayerResidenceLikeCpp::Active(_)) => {
                manager
                    .detach_player_like_cpp(handle)
                    .expect("detach canonical Player fixture");
                manager
                    .attach_player_like_cpp(handle, key, position)
                    .expect("reattach canonical Player fixture");
            }
            None => panic!("stale canonical Player fixture handle"),
        }
        return;
    }

    let player = session
        .build_initial_player_for_owner_like_cpp(wow_map::MapKey::new(map_id, instance_id), None)
        .expect("complete canonical player fixture");
    let record =
        wow_entities::MapObjectRecord::new_player(player).expect("canonical Player fixture record");
    let mut manager = canonical.lock().unwrap();
    manager
        .create_world_map(map_id, instance_id)
        .map_mut()
        .insert_map_object_record(record)
        .expect("insert canonical Player fixture");
}

#[derive(Debug, Clone, Copy)]
pub struct CharacterSaveRuntimeForTest {
    pub position: Position,
    pub level: i32,
    pub character_points: i32,
    pub near_pending: bool,
}

impl WorldSession {
    pub fn character_prepare_save_runtime_for_test(
        &self,
        original: Position,
        destination: Position,
    ) -> Option<()> {
        self.with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().world_mut().relocate(original);
            player.unit_mut().set_level(60);
            player.set_xp(1234);
            player.set_money(5678);
            player.set_character_points_like_cpp(23);
            let teleport = &mut player.gameplay_state_mut().teleport;
            teleport.near_pending = true;
            teleport.near_destination = Some((571, destination));
        })
    }

    pub fn character_save_runtime_for_test(&self) -> Option<CharacterSaveRuntimeForTest> {
        self.with_owned_player_like_cpp(|player| CharacterSaveRuntimeForTest {
            position: player.unit().world().position(),
            level: player.unit().data().level,
            character_points: player.active_data().character_points,
            near_pending: player.gameplay_state().teleport.near_pending,
        })
    }
}
