use super::*;

impl WorldCreature {
    /// Legacy runtime presentation captured without copying the actor or motor.
    pub fn capture_visibility_candidate(&self) -> CreatureVisibilityCandidate {
        CreatureVisibilityCandidate {
            target: self.creature.unit().capture_visibility_target(),
            create: CreatureCreateFacts::legacy(self),
            initial_auras: CreatureInitialAuraFacts::capture(
                self.guid(), self.level(), &self.creature.unit().subsystems().auras,
            ),
        }
    }

    pub fn capture_message_source(&self) -> CreatureMessageSourceFacts {
        CreatureMessageSourceFacts::capture(&self.creature)
    }
}

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    /// Current canonical compatibility observation, including typed Pets.
    /// Caller releases the guard before observing Player and running CanSee.
    pub fn capture_compatible_creature_visibility(
        &self,
        position: &Position,
        visibility_range: f32,
        source_combat_reach: f32,
        seer_phase: &PhaseShift,
    ) -> Vec<CreatureVisibilityCandidate> {
        let nearby = self.nearby_cell_guids_like_cpp(
            position.x, position.y, visibility_range + source_combat_reach,
        );
        nearby.world.creatures.into_iter().chain(nearby.grid.creatures)
            .filter_map(|guid| {
                self.with_creature_or_pet_like_cpp(guid, |creature, _owner| {
                    let world = creature.unit().world();
                    if !world.object().is_in_world()
                        || world.map_id() != self.map_id()
                        || !wow_core::visibility_distance_allows_like_cpp(
                            position, source_combat_reach, &world.position(),
                            world.combat_reach(), visibility_range,
                        )
                        || !seer_phase.can_see(world.phase_shift())
                    {
                        return None;
                    }
                    Some(CreatureVisibilityCandidate::compatible(creature))
                }).flatten()
            })
            .collect()
    }

    /// Match the current exact-Creature Message lookup; Pets are not accepted.
    /// No alive/in-world/phase gate is added to this source read.
    pub fn capture_compatible_creature_message_source(
        &self,
        guid: ObjectGuid,
    ) -> Option<CreatureMessageSourceFacts> {
        self.with_creature_like_cpp(guid, CreatureMessageSourceFacts::capture)
    }
}
