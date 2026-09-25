// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature kill contracts: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::ObjectGuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) struct PendingCreatureKillRewardLikeCpp {
    pub(in crate::session) killer_guid: ObjectGuid,
    pub(in crate::session) creature_guid: ObjectGuid,
    pub(in crate::session) creature_entry: u32,
    pub(in crate::session) creature_level: u8,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedCreatureKillEventLikeCpp {
    KillerProc {
        attacker_guid: ObjectGuid,
        victim_guid: ObjectGuid,
    },
    TapperTargetDiesProc {
        tapper_guid: ObjectGuid,
        victim_guid: ObjectGuid,
    },
    VictimDeathProc {
        victim_guid: ObjectGuid,
    },
    DeliveredKillingBlowCriteria {
        player_guid: ObjectGuid,
        victim_guid: ObjectGuid,
        quantity: u32,
    },
    DeathStateJustDied {
        victim_guid: ObjectGuid,
    },
    ZoneScriptUnitDeath {
        unit_guid: ObjectGuid,
    },
    TapperPetKilledUnitAi {
        tapper_guid: ObjectGuid,
        pet_guid: ObjectGuid,
        victim_guid: ObjectGuid,
    },
    LootFlagsApplied {
        creature_guid: ObjectGuid,
        lootable: bool,
        can_skin: bool,
        skinnable: bool,
    },
    CreatureOnHealthDepletedAi {
        creature_guid: ObjectGuid,
        attacker_guid: ObjectGuid,
        is_kill: bool,
    },
    CreatureJustDiedAi {
        creature_guid: ObjectGuid,
        killer_guid: ObjectGuid,
    },
    ScriptMgrOnCreatureKill {
        killer_guid: ObjectGuid,
        creature_guid: ObjectGuid,
    },
    CreatureKillReputationAwarded {
        creature_guid: ObjectGuid,
        faction_id: u32,
        reputation: i32,
        spillover_only: bool,
    },
}
