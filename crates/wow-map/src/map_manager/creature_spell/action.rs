//! Owned cast identity and ordered AI actions.
use super::*;
use wow_entities::OwnedLootAuthority;
/// The exact creature incarnation a cast plan was captured from.
///
/// A GUID and an engagement epoch cannot separate a replacement that reused
/// both, so carry the same three-part identity the melee synchronization path
/// proves: spawn ID, loot-storage authority and health-state revision
/// authority. Holding the two authorities keeps their allocations alive, so
/// neither identity can be recycled while the plan is in flight.
#[derive(Clone)]
pub struct SpellCasterIncarnation {
    spawn_id: u64,
    authority: OwnedLootAuthority,
    health_state_revision_authority: wow_entities::HealthStateRevisionAuthorityLikeCpp,
}

impl SpellCasterIncarnation {
    pub(super) fn capture(creature: &wow_entities::Creature) -> Self {
        Self {
            spawn_id: creature.spawn_id(),
            authority: creature.loot_authority_like_cpp().clone(),
            health_state_revision_authority: creature
                .unit()
                .health_state_revision_authority_like_cpp(),
        }
    }

    pub(super) fn matches(&self, creature: &wow_entities::Creature) -> bool {
        creature.spawn_id() == self.spawn_id
            && creature
                .loot_authority_like_cpp()
                .shares_storage_like_cpp(&self.authority)
            && creature
                .unit()
                .shares_health_state_revision_authority_like_cpp(
                    &self.health_state_revision_authority,
                )
    }
}

impl std::fmt::Debug for SpellCasterIncarnation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `OwnedLootAuthority` is deliberately not `Debug`: its identity is the
        // shared allocation, not a printable value.
        f.debug_struct("SpellCasterIncarnation")
            .field("spawn_id", &self.spawn_id)
            .field(
                "health_state_revision_authority",
                &self.health_state_revision_authority,
            )
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
pub struct SpellCastPlan {
    pub caster_guid: ObjectGuid,
    pub target_guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
    pub spell_id: i32,
    pub spell_x_spell_visual_id: u32,
    pub cast_time_ms: u32,
    pub spell_go_cast_flags: u32,
    pub engagement_epoch: u64,
    pub(super) caster_incarnation: SpellCasterIncarnation,
}

#[derive(Debug)]
pub struct SpellSchedule {
    pub caster_guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
    pub engagement_epoch: u64,
    pub slot: usize,
    pub minimum_ms: u64,
}
#[derive(Debug)]
pub struct SpellRejectedAttempt {
    pub caster_guid: ObjectGuid,
    pub target_guid: ObjectGuid,
    pub map_id: u16,
    pub instance_id: u32,
    pub engagement_epoch: u64,
    pub spell_id: u32,
    pub difficulty_id: u8,
}
#[derive(Debug)]
pub struct SpellCast {
    pub command: SpellCastPlan,
    pub difficulty_id: u8,
    pub turret_ai: bool,
}
#[derive(Debug)]
pub enum SpellAction {
    Cast(SpellCast),
    Schedule(SpellSchedule),
    TurretRejectedAttempt(SpellRejectedAttempt),
}
impl SpellAction {
    pub fn key(&self) -> crate::MapKey {
        let (map_id, instance_id) = match self {
            Self::Cast(value) => (value.command.map_id, value.command.instance_id),
            Self::Schedule(value) => (value.map_id, value.instance_id),
            Self::TurretRejectedAttempt(value) => (value.map_id, value.instance_id),
        };
        crate::MapKey::new(u32::from(map_id), instance_id)
    }
}
