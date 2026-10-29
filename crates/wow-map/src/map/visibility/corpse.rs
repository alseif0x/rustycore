//! Object-only Corpse AddToMap observer selection (Object.cpp:3644).

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader + MapWorldObjectEnvironment,
    Lifecycle: GridLifecycle,
{
    /// Visit the corpse's nearby world cells, including shared-vision seers,
    /// then apply each Player's native phase and viewpoint-distance admission.
    pub fn corpse_birth_visibility_recipients_like_cpp(
        &self,
        corpse_guid: ObjectGuid,
    ) -> Vec<ObjectGuid> {
        let Some(corpse) = self.get_typed_corpse(corpse_guid) else {
            return Vec::new();
        };
        let source = corpse.world();
        if !source.object().is_in_world() {
            return Vec::new();
        }
        let position = source.position();
        let nearby = self.nearby_cell_guids_like_cpp(
            position.x,
            position.y,
            source.get_visibility_range(self) + source.combat_reach(),
        );
        let mut recipients = Vec::new();
        for guid in nearby.world.players {
            let Some(player) = self.get_typed_player(guid) else {
                continue;
            };
            recipients.push(guid);
            recipients.extend(
                player
                    .unit()
                    .subsystems()
                    .control
                    .shared_vision_guids
                    .iter()
                    .filter(|viewer| {
                        self.get_typed_player(**viewer)
                            .is_some_and(|player| player_seer_guid(self, player) == guid)
                    })
                    .copied(),
            );
        }
        for guid in nearby.world.creatures {
            let Some(unit) = self.map_object_record(guid).and_then(|record| {
                record
                    .creature()
                    .map(|creature| creature.unit())
                    .or_else(|| record.pet().map(|pet| pet.creature().unit()))
            }) else {
                continue;
            };
            recipients.extend(
                unit.subsystems()
                    .control
                    .shared_vision_guids
                    .iter()
                    .filter(|viewer| {
                        self.get_typed_player(**viewer)
                            .is_some_and(|player| player_seer_guid(self, player) == guid)
                    })
                    .copied(),
            );
        }
        for guid in nearby.world.dynamic_objects {
            if let Some(dynamic) = self.get_typed_dynamic_object(guid)
                && let Some(player) = self.get_typed_player(dynamic.caster_guid())
                && player_seer_guid(self, player) == guid
            {
                recipients.push(player.guid());
            }
        }
        recipients.sort_unstable();
        recipients.dedup();
        recipients.retain(|guid| {
            let Some(player) = self.get_typed_player(*guid) else {
                return false;
            };
            let world = player.unit().world();
            let seer_guid = player_seer_guid(self, player);
            let viewpoint = self
                .map_object_by_kind(seer_guid, wow_entities::represented_seer_kinds_like_cpp())
                .unwrap_or(world);
            world.object().is_in_world()
                && world.map_id() == source.map_id()
                && world.instance_id() == source.instance_id()
                && world.in_same_phase(source)
                && viewpoint.is_within_dist(
                    source,
                    world.get_sight_range(Some(source), self),
                    false,
                    true,
                    true,
                )
        });
        recipients
    }
}
