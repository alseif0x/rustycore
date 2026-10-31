//! Native Player death admitted by a Map combat operation, before zero health.

use std::{future::Future, pin::Pin};
use wow_core::ObjectGuid;
use wow_entities::{PlayerDeathOperationContext, PlayerDeathOperationId};
use wow_map::{MapKey, MapManager, PlayerHandle};
use wow_world_core::runtime_plan::RuntimeEvent;
use wow_world_core::session::mailbox::{
    ApplyCreatureMeleeDamageLikeCppCommand, CreatureAttackStartLikeCppCommand,
};

/// Immutable transport facts, owned by the still-running combat future.
#[derive(Debug)]
pub struct PendingMapPlayerDeath {
    pub handle: PlayerHandle,
    pub map_key: MapKey,
    pub map_incarnation: u64,
    pub residence_revision: u64,
    pub operation: PlayerDeathOperationId,
    pub pvp_death: bool,
    pub creature_melee_prefix: Option<ApplyCreatureMeleeDamageLikeCppCommand>,
    pub attack_start: Option<CreatureAttackStartLikeCppCommand>,
    pub attack_start_events: Vec<RuntimeEvent>,
}

#[derive(Debug)]
pub struct CommittedMapCreatureMelee {
    pub handle: PlayerHandle,
    pub map_key: MapKey,
    pub map_incarnation: u64,
    pub residence_revision: u64,
    pub command: ApplyCreatureMeleeDamageLikeCppCommand,
    pub presentation_stage: wow_world_core::session::mailbox::CreatureMeleePresentationStageLikeCpp,
    pub attack_start: Option<CreatureAttackStartLikeCppCommand>,
    pub attack_start_events: Vec<RuntimeEvent>,
}

pub enum MapPlayerCombatCall {
    NativeDeath(PendingMapPlayerDeath),
    CommittedCreatureMelee(CommittedMapCreatureMelee),
    RuntimeEvents {
        events: Vec<RuntimeEvent>,
        producer_map_key: MapKey,
        producer_map_incarnation: u64,
    },
}

#[derive(Debug)]
pub struct NativePlayerDeathFailure {
    pub reason: &'static str,
    pub unresolved_permit:
        Option<std::sync::Arc<wow_world_core::session::mailbox::SessionPhasePermitLikeCpp>>,
    pub native_handle: Option<PlayerHandle>,
}

impl From<&'static str> for NativePlayerDeathFailure {
    fn from(reason: &'static str) -> Self {
        Self {
            reason,
            unresolved_permit: None,
            native_handle: None,
        }
    }
}

/// The existing Session rail transport supplied by the runtime composition.
pub type MapPlayerCombatCallback<'a> = dyn FnMut(
        MapPlayerCombatCall,
    ) -> Pin<Box<dyn Future<Output = Result<(), NativePlayerDeathFailure>> + Send + 'a>>
    + Send
    + 'a;

pub(crate) fn capture_committed_creature_melee(
    manager: &MapManager,
    command: ApplyCreatureMeleeDamageLikeCppCommand,
) -> Result<CommittedMapCreatureMelee, &'static str> {
    let (handle, residence_revision) = manager
        .current_player_admission_like_cpp(command.victim_guid)
        .ok_or("Committed melee Player handle is unavailable")?;
    let map_key = MapKey::new(u32::from(command.map_id), command.instance_id);
    if manager.player_active_residence_revision_like_cpp(handle)
        != Some((map_key, residence_revision))
    {
        return Err("Committed melee Player residence changed");
    }
    let map_incarnation = manager
        .map_incarnation_like_cpp(map_key)
        .ok_or("Committed melee Map incarnation is unavailable")?;
    Ok(CommittedMapCreatureMelee {
        handle,
        map_key,
        map_incarnation,
        residence_revision,
        command,
        presentation_stage:
            wow_world_core::session::mailbox::CreatureMeleePresentationStageLikeCpp::Whole,
        attack_start: None,
        attack_start_events: Vec::new(),
    })
}

pub(crate) fn admit_map_player_death(
    manager: &mut MapManager,
    map_key: MapKey,
    victim: ObjectGuid,
    attacker: ObjectGuid,
    durability_loss: bool,
) -> Result<PendingMapPlayerDeath, &'static str> {
    let (handle, residence_revision) = manager
        .current_player_admission_like_cpp(victim)
        .ok_or("Native Map death Player handle is unavailable")?;
    if manager.player_active_residence_revision_like_cpp(handle)
        != Some((map_key, residence_revision))
    {
        return Err("Native Map death Player residence changed");
    }
    let map_incarnation = manager
        .map_incarnation_like_cpp(map_key)
        .ok_or("Native Map death incarnation is unavailable")?;
    let map = manager
        .find_map(map_key.map_id, map_key.instance_id)
        .ok_or("Native Map death Map is unavailable")?
        .map();
    let unit = map
        .get_typed_player(attacker)
        .map(|player| player.unit())
        .or_else(|| {
            map.get_typed_creature(attacker)
                .map(|creature| creature.unit())
        })
        .or_else(|| map.get_typed_pet(attacker).map(|pet| pet.creature().unit()));
    // Object.cpp:2254–2260, Unit.cpp:10635: freeze the controlling Player
    // before Player::setDeathState can remove a Pet or run a callback.
    let pvp_death = unit.is_some_and(|unit| {
        match unit
            .subsystems()
            .control
            .charmer_guid
            .or(unit.subsystems().control.owner_guid)
            .filter(|owner| owner.is_player())
        {
            Some(owner) => map.get_typed_player(owner).is_some(),
            None => map.get_typed_player(attacker).is_some(),
        }
    });
    let operation = manager
        .with_player_mut_like_cpp(handle, |player| {
            player.begin_native_death_operation(PlayerDeathOperationContext::new(
                attacker,
                durability_loss,
            ))
        })
        .flatten()
        .ok_or("Native Map death could not admit its Player operation")?;
    Ok(PendingMapPlayerDeath {
        handle,
        map_key,
        map_incarnation,
        residence_revision,
        operation,
        pvp_death,
        creature_melee_prefix: None,
        attack_start: None,
        attack_start_events: Vec::new(),
    })
}
