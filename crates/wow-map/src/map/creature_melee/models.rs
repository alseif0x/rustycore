use super::*;

/// Packet-independent terms captured at the original calculation sites.
#[derive(Clone, Copy, Debug, Default)]
pub struct MeleePresentation {
    pub outcome: Option<RepresentedMeleeOutcomeLikeCpp>,
    pub full_absorb: bool,
    pub partial_absorb: bool,
    pub fake_damage: bool,
}

impl MeleePresentation {
    pub(super) fn add_absorb(&mut self, remaining: u32) {
        if remaining == 0 {
            self.full_absorb = true;
        } else {
            self.partial_absorb = true;
        }
    }
    pub(super) fn replace_absorb(&mut self, remaining: u32) {
        self.full_absorb = remaining == 0;
        self.partial_absorb = remaining != 0;
    }
}

pub enum MeleeEffect {
    AbsorbLog {
        attacker: ObjectGuid,
        victim: ObjectGuid,
        absorb_spell_id: i32,
        caster: ObjectGuid,
        absorbed: i32,
        original_damage: i32,
    },
    AuraRemoved {
        unit: ObjectGuid,
        slot: u8,
    },
    PlayerHealth {
        guid: ObjectGuid,
        health: i64,
    },
    Values {
        guid: ObjectGuid,
        update: UnitValuesUpdate,
    },
    AttackState {
        attacker: ObjectGuid,
        victim: ObjectGuid,
        presentation: MeleePresentation,
        damage: i32,
        original_damage: i32,
        over_damage: i32,
        blocked: i32,
        absorbed: i32,
        target_level: u8,
    },
    SplitMiss {
        spell_id: i32,
        caster: ObjectGuid,
        victim: ObjectGuid,
    },
    SplitDamage {
        target: ObjectGuid,
        caster: ObjectGuid,
        cast_id: ObjectGuid,
        spell_id: i32,
        visual_id: i32,
        damage: i32,
        original_damage: i32,
        overkill: i32,
        school_mask: u8,
        absorbed: i32,
    },
}

#[derive(Clone, Copy, Debug)]
pub(super) struct CreatureDamageThreatPlanLikeCpp {
    pub suppress: bool,
    pub no_initial_threat: bool,
    pub multiplier: f32,
}
impl Default for CreatureDamageThreatPlanLikeCpp {
    fn default() -> Self {
        Self {
            suppress: false,
            no_initial_threat: false,
            multiplier: 1.0,
        }
    }
}

pub struct MeleeThreatSpellFacts {
    pub suppress: bool,
    pub no_initial_threat: bool,
    pub school_mask: u32,
    pub multiplier: f32,
}

#[derive(Clone, Debug)]
pub struct CreatureDamageThreatOutcomeLikeCpp {
    pub attacker_guid: ObjectGuid,
    pub attacker_authority: wow_entities::OwnedLootAuthority,
    pub attacker_spawn_id: u64,
    pub delta: f32,
}

pub struct CreatureMeleeVictimSyncIdentityLikeCpp {
    pub authority: wow_entities::OwnedLootAuthority,
    pub health_state_revision_authority: wow_entities::HealthStateRevisionAuthorityLikeCpp,
    pub spawn_id: u64,
    pub loot_lifecycle_revision_before: u64,
    pub loot_lifecycle_revision_after: u64,
    pub death_state_before: wow_constants::DeathState,
    pub death_state_after: wow_constants::DeathState,
    pub ai_state_before: wow_entities::CreatureAiState,
    pub ai_state_after: wow_entities::CreatureAiState,
}

pub struct CreatureMeleeVictimSyncStateLikeCpp {
    pub applied_damage: u32,
    pub threat: Option<CreatureDamageThreatOutcomeLikeCpp>,
    pub victim_health_before: u64,
    pub victim_health_after: u64,
    pub victim_health_state_revision_before: u64,
    pub victim_health_state_revision_after: u64,
    pub identity: CreatureMeleeVictimSyncIdentityLikeCpp,
}

#[derive(Clone, Copy)]
pub struct PendingCreatureSwingLikeCpp {
    pub map_id: u16,
    pub instance_id: u32,
    pub attacker_guid: ObjectGuid,
    pub attacker_position: Position,
    pub attacker_combat_reach: f32,
    pub attacker_can_state_update: bool,
    pub victim_guid: ObjectGuid,
}

pub struct CreatureVictimCompatibilitySyncLikeCpp {
    pub swing: PendingCreatureSwingLikeCpp,
    pub state: CreatureMeleeVictimSyncStateLikeCpp,
}

pub(super) enum CreatureMeleeApplyResultLikeCpp {
    Ready,
    Hit {
        victim_applied_damage: u32,
        victim_threat: Option<CreatureDamageThreatOutcomeLikeCpp>,
        victim_health_before: u64,
        victim_health_after: u64,
        victim_health_state_revision_before: u64,
        victim_health_state_revision_after: u64,
        victim_creature_sync_identity: Option<CreatureMeleeVictimSyncIdentityLikeCpp>,
        over_damage: i32,
        target_level: u8,
        events: Vec<MeleeEffect>,
    },
    OutOfRange,
    BadFacing,
    AttackerStateRejected,
    LosRejected,
    AttackerUnavailable,
    VictimNotAlive,
    MissingVictim,
}

#[derive(Clone)]
pub struct MeleeAbsorbConsumption {
    pub slot: u8,
    pub consumed: i32,
    pub removed: bool,
}

pub(super) struct MeleeSwingStateLikeCpp {
    pub hit_info: MeleePresentation,
    pub original_damage: u32,
    pub avoided_outcome: Option<RepresentedMeleeOutcomeLikeCpp>,
    pub creature_victim_presentation: Option<(MeleePresentation, i32)>,
    pub creature_victim_avoided: bool,
    pub outcome_represented: bool,
    pub absorbed_damage: u32,
    pub mana_spent: u32,
    pub absorb_consumptions: Vec<MeleeAbsorbConsumption>,
    pub creature_victim_absorb_events: Vec<MeleeEffect>,
}
impl MeleeSwingStateLikeCpp {
    pub fn new_like_cpp(damage: u32) -> Self {
        Self {
            hit_info: MeleePresentation::default(),
            original_damage: damage,
            avoided_outcome: None,
            creature_victim_presentation: None,
            creature_victim_avoided: false,
            outcome_represented: false,
            absorbed_damage: 0,
            mana_spent: 0,
            absorb_consumptions: Vec::new(),
            creature_victim_absorb_events: Vec::new(),
        }
    }
}

pub struct CreatureMeleePlayerHit {
    pub swing: PendingCreatureSwingLikeCpp,
    pub damage: u32,
    pub over_damage: i32,
    pub target_level: u8,
    pub victim_health_after: u64,
    pub victim_health_state_revision_after: u64,
    pub presentation: MeleePresentation,
    pub original_damage: u32,
    pub absorbed: u32,
    pub mana_spent: u32,
    pub absorb_consumptions: Vec<MeleeAbsorbConsumption>,
    pub split_combat_log_packets: Vec<MeleeEffect>,
    pub self_share_health_updates: Vec<u64>,
}

#[derive(Default)]
pub struct CreatureMeleeSwingOutcome {
    pub melee_precondition_rejections: usize,
    pub attacker_incarnation_rejections: usize,
    pub melee_range_rejections: usize,
    pub melee_facing_rejections: usize,
    pub attacker_state_rejections: usize,
    pub melee_los_rejections: usize,
    pub attacking_interrupt_auras_removed: usize,
    pub melee_outcomes_unrepresented: usize,
    pub canonical_hits: usize,
    pub canonical_creature_hits: usize,
    pub commands: Vec<CreatureMeleePlayerHit>,
    pub events: Vec<MeleeEffect>,
    pub syncs: Vec<CreatureVictimCompatibilitySyncLikeCpp>,
}

#[derive(Clone, Copy)]
pub enum ShareAuraIdentityLikeCpp {
    Player {
        slot: u8,
        spell_id: i32,
        caster_guid: ObjectGuid,
        effect_index: u32,
    },
    Creature {
        applied: wow_entities::AppliedAuraRef,
        effect_index: u32,
    },
}
#[derive(Clone, Copy)]
pub struct ShareAuraSnapshotLikeCpp {
    pub identity: ShareAuraIdentityLikeCpp,
    pub caster_guid: ObjectGuid,
    pub school_mask: u32,
    pub amount: i32,
}
