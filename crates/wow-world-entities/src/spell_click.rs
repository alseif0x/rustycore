use wow_core::ObjectGuid;
use wow_world_core::session::QuestObjectiveAccessLikeCpp;

use crate::{RepresentedSpellClickCreatureSnapshotLikeCpp, WorldEntitiesState};

impl WorldEntitiesState {
    pub fn represented_spell_click_creature_snapshot_like_cpp(
        &self,
        access: &QuestObjectiveAccessLikeCpp<'_>,
        guid: ObjectGuid,
    ) -> Option<RepresentedSpellClickCreatureSnapshotLikeCpp> {
        access.with_spell_click_creature_like_cpp(guid, |creature, pet_owner_guid| {
            RepresentedSpellClickCreatureSnapshotLikeCpp {
                guid: creature.guid(),
                entry: creature.entry(),
                map_id: creature.unit().world().map_id(),
                instance_id: creature.unit().world().instance_id(),
                position: creature.position(),
                phase_shift: creature.unit().world().phase_shift().clone(),
                npc_flags: creature.ai_ownership().npc_flags,
                faction_template_id: creature.unit().data().faction_template.max(0) as u32,
                level: u32::from(creature.level()),
                health: creature.current_health(),
                max_health: creature.max_health(),
                is_alive: creature.is_alive(),
                is_in_world: creature.unit().world().object().is_in_world(),
                is_summon: creature.is_summon_like_cpp(),
                owner_guid: pet_owner_guid.or(creature.unit().subsystems().control.owner_guid),
            }
        })
    }
}
