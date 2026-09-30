//! Infallible commit after whole-map borrowed admission under the same guards.

use super::{CreatureActorTransportSummary, GridLifecycle, Map, MapInstance, PreparedCreatureTransport, PreparedCreatureThreatDelta, TerrainGridLoader};
use crate::map::ObjectEntry;
use crate::map_manager::WorldCreature;
use wow_core::ObjectGuid;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    pub(super) fn commit_creature_transport(
        &mut self,
        source: &mut MapInstance,
        planned: PreparedCreatureTransport,
        summary: &mut CreatureActorTransportSummary,
    ) -> Option<PreparedCreatureThreatDelta> {
        let guid = planned.guid;
        let mut actor = match source.take_creature_transport_slot(planned.source_slot) {
            Ok(taken) => taken.into_actor(),
            Err(_) => unreachable!("full preflight and exclusive source borrow preserve every slot"),
        };
        let old_entry = self.entity_world.take(&guid)
            .expect("exclusive preflighted Record cannot disappear");
        self.unindex_map_object_record_by_spawn_id_like_cpp(old_entry.as_ref());
        let old_record = match old_entry {
            ObjectEntry::Record(record) => record,
            ObjectEntry::CreatureActor(_) => unreachable!("preflight rejects Actor collisions"),
        };
        if planned.source_wins {
            self.insert_transported_actor(guid, actor);
            // Keep the losing canonical Creature alive through reinsertion.
            drop(old_record);
            summary.source_inner_winners += 1;
        } else {
            let canonical_creature = old_record.into_creature()
                .expect("preflight requires the complete exact Creature body");
            let losing_source = std::mem::replace(&mut actor.creature, canonical_creature);
            self.insert_transported_actor(guid, actor);
            // The outer motor/create_data were moved, never reconstructed.
            drop(losing_source);
            summary.canonical_inner_winners += 1;
        }
        summary.transported += 1;
        planned.threat_delta
    }

    fn insert_transported_actor(&mut self, guid: ObjectGuid, actor: WorldCreature) {
        let entry = ObjectEntry::from_creature_actor(actor);
        self.index_map_object_record_by_spawn_id_like_cpp(entry.as_ref());
        self.entity_world.restore_transport_entry(guid, entry)
            .expect("same exclusive Map borrow preserves the vacated original GUID slot");
        // Ownership promotion preserves existing canonical GUID membership in
        // cells/grids, active sets, formation, move queues and visibility. It
        // is the already-in-world store replacement, not AddToMap: do not
        // replay UnitAdd, formation/vehicle hooks, active locks or provenance.
    }

    pub(super) fn apply_transported_threat_delta(
        &mut self,
        delta: PreparedCreatureThreatDelta,
    ) {
        let guid = delta.guid;
        // Every winning inner is now installed. Reciprocal writes change only
        // threatened_by_me; they never alter the owner's outgoing threat_refs.
        for added_guid in delta.mirrored {
            let threat_ref = self.with_creature_like_cpp(guid, |creature| {
                creature.unit().subsystems().combat.threat_ref(added_guid).copied()
            }).flatten();
            let Some(threat_ref) = threat_ref else { continue; };
            if let Some(player) = self.get_typed_player_mut(added_guid) {
                player.unit_mut().subsystems_mut().combat.put_threatened_by_me_ref(guid, threat_ref);
            } else if let Some(creature) = self.get_typed_creature_mut(added_guid) {
                creature.unit_mut().subsystems_mut().combat.put_threatened_by_me_ref(guid, threat_ref);
            }
        }
        for removed_guid in delta.removed {
            if let Some(player) = self.get_typed_player_mut(removed_guid) {
                player.unit_mut().subsystems_mut().combat.purge_threatened_by_me_ref(guid);
            } else if let Some(creature) = self.get_typed_creature_mut(removed_guid) {
                creature.unit_mut().subsystems_mut().combat.purge_threatened_by_me_ref(guid);
            }
        }
    }
}
