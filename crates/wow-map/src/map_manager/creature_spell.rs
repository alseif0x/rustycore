//! One stock-AI spell motor for legacy and dormant canonical Actor ownership.
//! a5f8da2e CombatAI.cpp:53-106/191-224, UnitAI.cpp:61-79,
//! Spell.cpp:3839-3843/8056-8064/8363-8373. Wire-only gaps are retained.
use super::{MapManager as LegacyMapManager, WorldCreature};
use std::collections::{HashMap, VecDeque};
use wow_constants::movement::MovementFlag;
use wow_constants::{PowerType, UnitFlags, UnitState};
use wow_core::{HighGuid, ObjectGuid, Position};
use wow_entities::{Creature, CreatureAiState};

mod action;
mod admission;
mod backend;
mod combat;
mod contracts;
mod finish;
mod frame;
mod hit;
mod log;
mod outcome;
mod policies;
mod preparation;
mod rules;
mod target;
mod turret;
mod validation;

pub use action::*;
pub use admission::target_is_valid;
pub(crate) use backend::SpellMap;
pub use contracts::*;
pub use frame::{SpellLosPending, SpellProgress, SpellPublicationPending, SpellQueue};
pub use hit::{represented_hit_profile, resolve_hit_profile};
pub use log::cast_log;
pub use outcome::*;
pub use policies::*;
use preparation::*;
pub use preparation::{has_noninstant_spell, resets_combat_timers};
pub use rules::{has_nonzero_power_cost, single_unit_topology};
pub use target::classify_target;

impl WorldCreature {
    pub fn spell_has_implicit_cost(
        &self,
        spell: &SpellInfoFacts,
        difficulty: u8,
        policies: &mut SpellPolicies<'_>,
    ) -> bool {
        implicit_cost(spell, difficulty, policies, self)
    }

    pub fn spell_cast_plan(
        &self,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        spell_id: u32,
        spell: &SpellInfoFacts,
        difficulty: u8,
        policies: &mut SpellPolicies<'_>,
    ) -> Result<SpellCastPlan, ()> {
        cast_plan(
            self,
            caster_guid,
            target_guid,
            map_id,
            instance_id,
            spell_id,
            spell,
            difficulty,
            policies,
        )
    }
}

impl LegacyMapManager {
    /// Compatibility preparation retains the original HashMap traversal.
    /// It does not consume a delay/hit draw or advance any runtime clock.
    pub fn prepare_spell_map(
        &mut self,
        map_id: u16,
        instance_id: u32,
        difficulty: u8,
        policies: &mut SpellPolicies<'_>,
    ) -> SpellQueue {
        let guids = self.creature_guids(map_id, instance_id);
        let mut backend = SpellMap::Legacy {
            manager: self,
            map: None,
            map_id,
            instance_id,
        };
        SpellQueue::prepare(
            &mut backend,
            guids,
            map_id,
            instance_id,
            difficulty,
            policies,
        )
    }

    pub fn consume_spell_schedule(&mut self, schedule: SpellSchedule) -> bool {
        preparation::consume_schedule(
            self.find_creature_mut(schedule.map_id, schedule.instance_id, schedule.caster_guid),
            &schedule,
        )
    }
}
