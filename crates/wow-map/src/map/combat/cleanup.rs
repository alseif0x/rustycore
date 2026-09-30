//! Canonical Player combat graph cleanup under the caller's existing Map borrow.

use super::{GridLifecycle, Map, ObjectGuid, TerrainGridLoader};

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    /// Clear the Player's combat references and attackers, then clean its peers.
    ///
    /// Returns the sorted, deduplicated original union of PvE references, PvP
    /// references and attackers, including GUIDs whose owner is now absent.
    /// The caller uses that same set for its subsequent legacy cleanup. A
    /// missing typed Player returns `None` before any writes.
    ///
    /// Source boundary: `a5f8da2e`, `Unit.cpp:5802-5830` (`CombatStop` and
    /// `CombatStopWithPets`). This is the existing Rust canonical graph stage:
    /// the caller retains its earlier attack stop, owner/map resolution, guard
    /// release, legacy cleanup and publication. The original sorted union and
    /// Player-before-Creature lookup order remain unchanged; this extraction
    /// adds no C++ cast interruption, pet traversal or shared references.
    pub fn clear_player_combat(&mut self, player_guid: ObjectGuid) -> Option<Vec<ObjectGuid>> {
        let player = self.get_typed_player_mut(player_guid)?;

        let mut owner_guids: Vec<ObjectGuid> = player
            .unit()
            .subsystems()
            .combat
            .pve_refs
            .keys()
            .chain(player.unit().subsystems().combat.pvp_refs.keys())
            .chain(player.unit().subsystems().combat.attackers.iter())
            .copied()
            .collect();
        owner_guids.sort_unstable();
        owner_guids.dedup();

        player.unit_mut().subsystems_mut().combat.end_all_combat();
        player.unit_mut().subsystems_mut().combat.clear_attackers();

        for owner_guid in &owner_guids {
            if let Some(owner) = self.get_typed_player_mut(*owner_guid) {
                if owner.unit().attacking() == Some(player_guid) {
                    let _ = owner.unit_mut().attack_stop_like_cpp();
                }
                owner
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(player_guid);
                owner.unit_mut().remove_attacker_like_cpp(player_guid);
            } else if let Some(owner) = self.get_typed_creature_mut(*owner_guid) {
                if owner.unit().attacking() == Some(player_guid) {
                    let _ = owner.unit_mut().attack_stop_like_cpp();
                }
                owner
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(player_guid);
                owner.unit_mut().remove_attacker_like_cpp(player_guid);
            }
        }

        Some(owner_guids)
    }
}

#[cfg(test)]
mod tests;
