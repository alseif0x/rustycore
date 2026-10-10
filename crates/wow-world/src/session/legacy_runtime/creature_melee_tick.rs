//! Legacy creature melee tick and its damage application.
//!
//! Moved out of the Session root under #619. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::creature_melee_sync::{
    CreatureVictimCompatibilitySyncLikeCpp, PendingCreatureSwingLikeCpp,
};
use super::*;

mod absorption;
mod creature_victim;
mod player_victim;
mod secondary_targets;
use creature_victim::creature_victim_damage_like_cpp;
use player_victim::player_victim_damage_like_cpp;
use secondary_targets::{
    MeleeSecondaryTargetsOutcomeLikeCpp, apply_secondary_targets_damage_like_cpp,
};

/// Apply one player's melee swings to a legacy creature.
///
/// Lifted out of `run_combat_tick` by #28. This is the write path that made
/// every logged-in session a writer of shared creature combat state; extracting
/// it is what lets the global loop become its sole owner. Damage arithmetic,
/// tap assignment, threat, the death branch and the swing record are unchanged.
pub(in crate::session) fn apply_player_melee_to_legacy_creature_like_cpp(
    creature: &mut wow_entities::Creature,
    player_guid: ObjectGuid,
    tap_group_guids: &[ObjectGuid],
    canonical_swings: Option<&[crate::session::combat::RepresentedMeleeSwingLikeCpp]>,
) -> Option<PlayerMeleeCreatureHitLikeCpp> {
    if !creature.is_alive() {
        return None;
    }
    // C++ `Unit::AttackerStateUpdate` → `AtTargetAttacked` → `EngageWithTarget`:
    // the swing engages the creature (threat 0) without choosing its victim;
    // the aggro tick's `UpdateVictim` then publishes the reaction (#1344).
    creature.engage_with_target_like_cpp(player_guid);
    let damages: Vec<crate::session::combat::RepresentedMeleeSwingLikeCpp> = match canonical_swings
    {
        Some(swings) => swings.to_vec(),
        None => {
            if !creature.can_swing() {
                return None;
            }
            vec![
                crate::session::combat::RepresentedMeleeSwingLikeCpp::hit_like_cpp(
                    creature.roll_damage()?.max(1),
                ),
            ]
        }
    };
    let entry = creature.entry();
    let level = creature.level();
    let mut swings = Vec::new();
    let mut swing_presentations = Vec::new();
    let mut died = false;
    let mut move_stop = None;
    for swing in damages {
        if !creature.is_alive() {
            break;
        }
        let damage = swing.damage;
        // C++ `DealMeleeDamage` applies nothing for a missed or avoided swing:
        // no damage, no tap and no threat.
        if damage == 0 {
            swings.push((0, false, -1));
            swing_presentations.push((
                swing.hit_info,
                swing.victim_state,
                swing.blocked,
                swing.original_damage,
            ));
            continue;
        }
        let health_before = creature.current_hp();
        creature.set_tapped_by_player(player_guid, tap_group_guids);
        died = creature.take_damage_before_death_state_like_cpp(damage);
        let over_damage = if died {
            damage.saturating_sub(health_before) as i32
        } else {
            -1
        };
        creature
            .unit_mut()
            .subsystems_mut()
            .combat
            .add_threat(player_guid, damage as f32);
        swings.push((damage, died, over_damage));
        swing_presentations.push((
            swing.hit_info,
            swing.victim_state,
            swing.blocked,
            swing.original_damage,
        ));
        if died {
            let combat = &mut creature.unit_mut().subsystems_mut().combat;
            combat.clear_threat();
            combat.clear_attackers();
            move_stop = creature
                .stop_move_spline_like_cpp()
                .map(|stop| (stop.position, stop.spline_id));
            break;
        }
    }
    if canonical_swings.is_none() {
        creature.record_swing();
    }
    let values_update = creature.unit().values_update();
    Some(PlayerMeleeCreatureHitLikeCpp {
        swings,
        swing_presentations,
        entry,
        level,
        died,
        move_stop,
        values_update,
    })
}
/// Runs one global legacy creature melee tick without spawning a loop.
///
/// This is dormant infrastructure for the next runtime slice after movement
/// and lifecycle. C++ contrast: `Creature::Update` calls
/// `DoMeleeAttackIfReady()` from the map object update phase. This function
/// preserves the pre-existing transitional damage bridge while the complete
/// C++ outcome/proc pipeline remains a later runtime slice. Spell-hit RNG
/// accreditation must not turn otherwise valid creature swings into no-ops.
/// The per-swing presentation `run_legacy_creature_melee_tick_once_like_cpp`
/// resolves before `DealMeleeDamage` and publishes at delivery.
///
/// C++ `Unit::CalculateMeleeDamage` fills these terms before the health write.
/// The player-victim phase in `player_victim` writes them and this tick's
/// delivery owns publishing them, so they travel between the two as one value.
/// The split/share locals live in `secondary_targets`, the phase that owns them.
struct MeleeSwingStateLikeCpp {
    pub(super) hit_info: u32,
    pub(super) victim_state: u8,
    pub(super) original_damage: u32,
    pub(super) avoided_outcome: Option<crate::session_rules::RepresentedMeleeOutcomeLikeCpp>,
    // The creature-victim branch publishes through the compatibility
    // bridge, so it carries its own presentation and avoid flag.
    pub(super) creature_victim_presentation: Option<(u32, u8, i32)>,
    pub(super) creature_victim_avoided: bool,
    pub(super) outcome_represented: bool,
    // C++ `CalcAbsorbResist`'s result for this swing: the absorbed amount
    // the packet publishes and every shield it spent. The victim session
    // owns the absorb-log publication and the aura transition, so it
    // receives the consumption list at delivery.
    pub(super) absorbed_damage: u32,
    pub(super) mana_spent: u32,
    pub(super) absorb_consumptions:
        Vec<crate::session::mailbox::CreatureMeleeAbsorbConsumptionLikeCpp>,
    pub(super) creature_victim_absorb_events: Vec<RuntimeEvent>,
}

impl MeleeSwingStateLikeCpp {
    /// The pre-table presentation defaults C++ starts one swing's damage with.
    fn new_like_cpp(damage: u32) -> Self {
        Self {
            hit_info: wow_packet::packets::combat::HIT_INFO_AFFECTS_VICTIM,
            victim_state: wow_packet::packets::combat::VICTIM_STATE_HIT,
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

pub fn run_legacy_creature_melee_tick_once_like_cpp(
    legacy_map_manager: &crate::map_manager::SharedMapManager,
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
    config: &crate::session::LegacyCreatureAggroConfigLikeCpp,
) -> LegacyCreatureMeleeTickOutcomeLikeCpp {
    use crate::map_manager::RuntimeTickOwner;

    let mut outcome = LegacyCreatureMeleeTickOutcomeLikeCpp::default();
    // #1263 F6-8C: the canonical designated owner decides which creature may
    // swing. Read once, before the legacy write guard.
    let ownership =
        canonical_creature_ownership_like_cpp(canonical_map_manager, legacy_map_manager);

    let pending_swings = {
        let mut manager = legacy_map_manager
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if manager.tick_owner() != RuntimeTickOwner::GlobalLegacy {
            outcome.skipped_owner_not_global = true;
            return outcome;
        }
        collect_creature_melee_swings_on_store_like_cpp(&mut *manager, &ownership, &mut outcome)
    };

    let Some(canonical_map_manager) = canonical_map_manager else {
        return outcome;
    };

    let mut creature_victim_syncs = Vec::new();
    for swing in pending_swings {
        // One C++ map update owns attacker validation, RNG consumption, damage,
        // attacking-aura removal, and timer rearm as one serial operation. Hold
        // both transitional owners in the established canonical -> legacy order
        // so a target switch or same-GUID respawn cannot cross that commit.
        let Ok(mut canonical_manager) = canonical_map_manager.lock() else {
            outcome.melee_precondition_rejections += 1;
            continue;
        };
        let mut attackers = LegacyCreatureMeleeAttackersLikeCpp {
            legacy: legacy_map_manager
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            ownership: &ownership,
        };
        execute_creature_melee_swing_on_manager_like_cpp(
            &mut canonical_manager,
            &mut attackers,
            swing,
            config,
            &mut outcome,
            &mut creature_victim_syncs,
        );
    }

    super::creature_melee_sync::replay_creature_victim_syncs_like_cpp(
        legacy_map_manager,
        canonical_map_manager,
        creature_victim_syncs,
        &mut outcome,
    );
    outcome
}

/// The selection half of `Creature::Update` → `DoMeleeAttackIfReady`: which
/// creatures of the store have a swing due against a player or creature
/// victim. #1263 F6-8D3a-1: one body for the legacy store and the admitted
/// canonical store.
pub(super) fn collect_creature_melee_swings_on_store_like_cpp<S>(
    store: &mut S,
    ownership: &CanonicalCreatureOwnershipLikeCpp,
    outcome: &mut LegacyCreatureMeleeTickOutcomeLikeCpp,
) -> Vec<PendingCreatureSwingLikeCpp>
where
    S: CreaturePhaseStoreLikeCpp + ?Sized,
{
    use wow_entities::CurrentSpellSlot;

    let mut pending_swings = Vec::new();
    let map_keys = store.phase_map_keys_like_cpp();
    outcome.maps_seen = map_keys.len();
    for (map_id, instance_id) in map_keys {
        let guids = store.phase_creature_guids_like_cpp(map_id, instance_id);
        for guid in guids {
            let Some(creature) = store.phase_creature_mut_like_cpp(map_id, instance_id, guid)
            else {
                continue;
            };
            outcome.creatures_seen += 1;
            if !creature.can_swing() {
                continue;
            }
            // C++ `TurretAI` calls `SetCanMelee(false)` in its
            // constructor. The transitional selector stores the explicit
            // DB AIName rather than a live AI object, so enforce that
            // constructor side effect at the global melee boundary.
            if creature.lifecycle_metadata().ai_name == "TurretAI" {
                outcome.melee_precondition_rejections += 1;
                continue;
            }
            if !creature.can_melee_like_cpp() {
                outcome.melee_precondition_rejections += 1;
                continue;
            }
            let unit = creature.unit();
            if unit.has_unit_state(UnitState::CHARGING.bits())
                || (unit.has_unit_state(UnitState::CASTING.bits())
                    && !unit
                        .current_spell(CurrentSpellSlot::Channeled)
                        .is_some_and(|spell| spell.allow_actions_during_channel))
            {
                outcome.melee_precondition_rejections += 1;
                continue;
            }
            // #1263 F6-8C: a creature with no canonical incarnation at its
            // exact residence is not the canonical owner's object, so it
            // selects no swing from the legacy representation. The gate sits
            // after C++'s own preconditions (`can_swing`, `CanMelee`, the
            // charging/channelling state), so their order and their counters
            // are unchanged.
            if !ownership.decides_like_cpp(map_id, instance_id, guid) {
                outcome.canonical_incarnation_rejections += 1;
                continue;
            }
            let Some(victim_guid) = creature.ai_ownership().combat_target else {
                continue;
            };
            if !victim_guid.is_player() && !victim_guid.is_any_type_creature() {
                continue;
            }
            pending_swings.push(PendingCreatureSwingLikeCpp {
                map_id,
                instance_id,
                attacker_guid: guid,
                attacker_position: creature.position(),
                attacker_combat_reach: creature.unit().world().combat_reach(),
                attacker_can_state_update: creature
                    .unit()
                    .can_attacker_state_update_melee_like_cpp(false),
                victim_guid,
            });
            outcome.swings_ready += 1;
        }
    }
    pending_swings
}

/// The attacker terms `Unit::CalculateMeleeDamage` reads once the swing is
/// committed, captured after the damage roll so the victim phases can run
/// against the canonical manager while the attacker stays wherever its owner
/// keeps it.
pub(super) struct CreatureMeleeAttackerFactsLikeCpp {
    pub(super) applied_auras: Vec<wow_entities::AppliedAuraRef>,
    pub(super) flags_extra: u32,
    pub(super) level: u8,
    pub(super) is_charmed_owned_by_player_or_player: bool,
    pub(super) base_attack_speed: [u32; wow_entities::MAX_ATTACK],
}

impl CreatureMeleeAttackerFactsLikeCpp {
    fn capture_like_cpp(attacker: &wow_entities::Creature) -> Self {
        Self {
            applied_auras: attacker.unit().subsystems().auras.applied_auras.clone(),
            flags_extra: attacker.lifecycle_metadata().flags_extra,
            level: attacker.level(),
            is_charmed_owned_by_player_or_player: attacker
                .is_charmed_owned_by_player_or_player_like_cpp(),
            base_attack_speed: attacker.unit().base_attack_speed(),
        }
    }
}

/// How one selected swing may begin on its attacker owner.
pub(super) enum CreatureMeleeSwingBeginLikeCpp {
    Ready,
    AttackerMissing,
    /// F6-8C: the attacker is no longer the canonical owner's object.
    NotCanonicalOwner,
}

/// Where the swing body finds its attacker. #1263 F6-8D3a-1: the legacy
/// bridge keeps it on the legacy representation and proves it is the same
/// incarnation as the canonical record; the admitted executor's attacker *is*
/// the canonical record.
pub(super) trait CreatureMeleeAttackersLikeCpp {
    fn begin_swing_like_cpp(
        &mut self,
        canonical_manager: &wow_map::MapManager,
        swing: &PendingCreatureSwingLikeCpp,
    ) -> CreatureMeleeSwingBeginLikeCpp;
    /// One attacker operation; `None` when the attacker is gone.
    fn with_attacker_like_cpp<R>(
        &mut self,
        canonical_manager: &mut wow_map::MapManager,
        swing: &PendingCreatureSwingLikeCpp,
        operation: impl FnOnce(&mut wow_entities::Creature) -> R,
    ) -> Option<R>;
    /// Whether the attacker this swing runs on is the canonical incarnation.
    fn attacker_is_canonical_incarnation_like_cpp(
        &mut self,
        canonical_manager: &wow_map::MapManager,
        swing: &PendingCreatureSwingLikeCpp,
    ) -> bool;
    /// Align the canonical `WorldObject` used by the LOS check to the attacker
    /// position the swing validated (a no-op where they are one object).
    fn align_canonical_attacker_position_like_cpp(
        &mut self,
        canonical_manager: &mut wow_map::MapManager,
        swing: &PendingCreatureSwingLikeCpp,
    );
}

struct LegacyCreatureMeleeAttackersLikeCpp<'a> {
    legacy: std::sync::RwLockWriteGuard<'a, crate::map_manager::MapManager>,
    ownership: &'a CanonicalCreatureOwnershipLikeCpp,
}

impl CreatureMeleeAttackersLikeCpp for LegacyCreatureMeleeAttackersLikeCpp<'_> {
    fn begin_swing_like_cpp(
        &mut self,
        _canonical_manager: &wow_map::MapManager,
        swing: &PendingCreatureSwingLikeCpp,
    ) -> CreatureMeleeSwingBeginLikeCpp {
        if self
            .legacy
            .find_creature(swing.map_id, swing.instance_id, swing.attacker_guid)
            .is_none()
        {
            return CreatureMeleeSwingBeginLikeCpp::AttackerMissing;
        }
        // #1263 F6-8C: the re-validation repeats the ownership gate, because
        // the collect phase released both guards and the incarnation may have
        // gone.
        if !self
            .ownership
            .decides_like_cpp(swing.map_id, swing.instance_id, swing.attacker_guid)
        {
            return CreatureMeleeSwingBeginLikeCpp::NotCanonicalOwner;
        }
        CreatureMeleeSwingBeginLikeCpp::Ready
    }

    fn with_attacker_like_cpp<R>(
        &mut self,
        _canonical_manager: &mut wow_map::MapManager,
        swing: &PendingCreatureSwingLikeCpp,
        operation: impl FnOnce(&mut wow_entities::Creature) -> R,
    ) -> Option<R> {
        // #1263 F6-8D1: the swing operation runs against the canonical
        // attacker type; the legacy store is only the transitional seam.
        self.legacy
            .find_creature_mut(swing.map_id, swing.instance_id, swing.attacker_guid)
            .map(|attacker| operation(&mut attacker.creature))
    }

    fn attacker_is_canonical_incarnation_like_cpp(
        &mut self,
        canonical_manager: &wow_map::MapManager,
        swing: &PendingCreatureSwingLikeCpp,
    ) -> bool {
        let Some(attacker) =
            self.legacy
                .find_creature(swing.map_id, swing.instance_id, swing.attacker_guid)
        else {
            return false;
        };
        let attacker = &attacker.creature;
        canonical_manager
            .find_map(u32::from(swing.map_id), swing.instance_id)
            .and_then(|managed| {
                managed
                    .map()
                    .with_creature_like_cpp(swing.attacker_guid, |canonical_attacker| {
                        canonical_attacker.spawn_id() == attacker.spawn_id()
                            && canonical_attacker
                                .loot_authority_like_cpp()
                                .shares_storage_like_cpp(attacker.loot_authority_like_cpp())
                            && canonical_attacker
                                .unit()
                                .shares_health_state_revision_authority_like_cpp(
                                    &attacker.unit().health_state_revision_authority_like_cpp(),
                                )
                    })
            })
            .unwrap_or(false)
    }

    fn align_canonical_attacker_position_like_cpp(
        &mut self,
        canonical_manager: &mut wow_map::MapManager,
        swing: &PendingCreatureSwingLikeCpp,
    ) {
        // Movement snapshots normally publish this position before melee, but
        // the two global ticks may overlap between their legacy read and
        // canonical write phases. With both owners locked and the incarnation
        // proven equal, align the canonical WorldObject used by the LOS check
        // to the same live position used for range and facing.
        if let Some(managed) =
            canonical_manager.find_map_mut(u32::from(swing.map_id), swing.instance_id)
        {
            let _ = managed
                .map_mut()
                .relocate_map_object_like_cpp(swing.attacker_guid, swing.attacker_position);
        }
    }
}

/// The admitted canonical executor's attackers: the canonical incarnation
/// itself, on the manager the executor holds locked.
pub(super) struct CanonicalCreatureMeleeAttackersLikeCpp;

impl CreatureMeleeAttackersLikeCpp for CanonicalCreatureMeleeAttackersLikeCpp {
    fn begin_swing_like_cpp(
        &mut self,
        canonical_manager: &wow_map::MapManager,
        swing: &PendingCreatureSwingLikeCpp,
    ) -> CreatureMeleeSwingBeginLikeCpp {
        let present = canonical_manager
            .find_map(u32::from(swing.map_id), swing.instance_id)
            .is_some_and(|managed| {
                managed
                    .map()
                    .get_typed_creature(swing.attacker_guid)
                    .is_some()
            });
        if present {
            CreatureMeleeSwingBeginLikeCpp::Ready
        } else {
            CreatureMeleeSwingBeginLikeCpp::AttackerMissing
        }
    }

    fn with_attacker_like_cpp<R>(
        &mut self,
        canonical_manager: &mut wow_map::MapManager,
        swing: &PendingCreatureSwingLikeCpp,
        operation: impl FnOnce(&mut wow_entities::Creature) -> R,
    ) -> Option<R> {
        canonical_manager
            .find_map_mut(u32::from(swing.map_id), swing.instance_id)?
            .map_mut()
            .with_creature_mut_like_cpp(swing.attacker_guid, operation)
    }

    fn attacker_is_canonical_incarnation_like_cpp(
        &mut self,
        canonical_manager: &wow_map::MapManager,
        swing: &PendingCreatureSwingLikeCpp,
    ) -> bool {
        // The attacker is the canonical record the admission fenced.
        canonical_manager
            .find_map(u32::from(swing.map_id), swing.instance_id)
            .is_some_and(|managed| {
                managed
                    .map()
                    .get_typed_creature(swing.attacker_guid)
                    .is_some()
            })
    }

    fn align_canonical_attacker_position_like_cpp(
        &mut self,
        _: &mut wow_map::MapManager,
        _: &PendingCreatureSwingLikeCpp,
    ) {
    }
}

/// One rejected swing's timer consequence, exactly as `DoMeleeAttackIfReady`
/// applies it after `AttackerStateUpdate` returns early.
#[derive(Clone, Copy)]
enum CreatureMeleeRearmLikeCpp {
    /// Range or facing: retry after the short failed-swing delay.
    Retry,
    /// Reset BASE_ATTACK.
    Swing,
    /// No timer consequence.
    None,
}

/// Map a non-hit apply result to its counter and timer consequence.
fn creature_melee_apply_rejection_like_cpp(
    result: &CreatureMeleeApplyResultLikeCpp,
    outcome: &mut LegacyCreatureMeleeTickOutcomeLikeCpp,
) -> CreatureMeleeRearmLikeCpp {
    match result {
        CreatureMeleeApplyResultLikeCpp::OutOfRange => {
            outcome.melee_range_rejections += 1;
            CreatureMeleeRearmLikeCpp::Retry
        }
        CreatureMeleeApplyResultLikeCpp::BadFacing => {
            outcome.melee_facing_rejections += 1;
            CreatureMeleeRearmLikeCpp::Retry
        }
        CreatureMeleeApplyResultLikeCpp::AttackerStateRejected => {
            outcome.attacker_state_rejections += 1;
            CreatureMeleeRearmLikeCpp::Swing
        }
        CreatureMeleeApplyResultLikeCpp::LosRejected => {
            outcome.melee_los_rejections += 1;
            CreatureMeleeRearmLikeCpp::Swing
        }
        CreatureMeleeApplyResultLikeCpp::VictimNotAlive => {
            // `DoMeleeAttackIfReady` resets BASE_ATTACK after
            // `AttackerStateUpdate` returns early for a dead victim.
            outcome.melee_precondition_rejections += 1;
            CreatureMeleeRearmLikeCpp::Swing
        }
        CreatureMeleeApplyResultLikeCpp::AttackerUnavailable
        | CreatureMeleeApplyResultLikeCpp::MissingVictim => {
            outcome.melee_precondition_rejections += 1;
            CreatureMeleeRearmLikeCpp::None
        }
        CreatureMeleeApplyResultLikeCpp::Ready | CreatureMeleeApplyResultLikeCpp::Hit { .. } => {
            CreatureMeleeRearmLikeCpp::None
        }
    }
}

/// One selected swing: C++ `DoMeleeAttackIfReady` → `AttackerStateUpdate` →
/// `CalculateMeleeDamage` → `DealMeleeDamage`, on the canonical manager the
/// caller holds locked and the attacker owner `attackers` resolves.
///
/// #1263 F6-8D3a-1: the legacy bridge and the admitted canonical executor run
/// this one body. Victim-side state is always canonical; the attacker's own
/// operations (timer, RNG, attacking-interrupt auras) go through `attackers`.
pub(super) fn execute_creature_melee_swing_on_manager_like_cpp<A>(
    canonical_manager: &mut wow_map::MapManager,
    attackers: &mut A,
    mut swing: PendingCreatureSwingLikeCpp,
    config: &crate::session::LegacyCreatureAggroConfigLikeCpp,
    outcome: &mut LegacyCreatureMeleeTickOutcomeLikeCpp,
    creature_victim_syncs: &mut Vec<CreatureVictimCompatibilitySyncLikeCpp>,
) where
    A: CreatureMeleeAttackersLikeCpp,
{
    use wow_entities::CurrentSpellSlot;

    match attackers.begin_swing_like_cpp(canonical_manager, &swing) {
        CreatureMeleeSwingBeginLikeCpp::Ready => {}
        CreatureMeleeSwingBeginLikeCpp::AttackerMissing => {
            outcome.melee_precondition_rejections += 1;
            return;
        }
        CreatureMeleeSwingBeginLikeCpp::NotCanonicalOwner => {
            outcome.canonical_incarnation_rejections += 1;
            outcome.melee_precondition_rejections += 1;
            return;
        }
    }
    let rearm = |attackers: &mut A,
                 canonical_manager: &mut wow_map::MapManager,
                 swing: &PendingCreatureSwingLikeCpp,
                 rearm: CreatureMeleeRearmLikeCpp| {
        let _ =
            attackers.with_attacker_like_cpp(canonical_manager, swing, |attacker| match rearm {
                CreatureMeleeRearmLikeCpp::Retry => attacker.record_failed_swing_retry_like_cpp(),
                CreatureMeleeRearmLikeCpp::Swing => attacker.record_swing(),
                CreatureMeleeRearmLikeCpp::None => {}
            });
    };

    let victim_guid = swing.victim_guid;
    let validated = attackers.with_attacker_like_cpp(canonical_manager, &swing, |attacker| {
        if !attacker.can_swing()
            || attacker.ai_ownership().combat_target != Some(victim_guid)
            || attacker.lifecycle_metadata().ai_name == "TurretAI"
            || !attacker.can_melee_like_cpp()
        {
            return None;
        }
        let unit = attacker.unit();
        if unit.has_unit_state(UnitState::CHARGING.bits())
            || (unit.has_unit_state(UnitState::CASTING.bits())
                && !unit
                    .current_spell(CurrentSpellSlot::Channeled)
                    .is_some_and(|spell| spell.allow_actions_during_channel))
        {
            return None;
        }
        Some((
            attacker.position(),
            unit.world().combat_reach(),
            unit.can_attacker_state_update_melee_like_cpp(false),
        ))
    });
    let Some((attacker_position, attacker_combat_reach, attacker_can_state_update)) =
        validated.flatten()
    else {
        outcome.melee_precondition_rejections += 1;
        return;
    };
    swing.attacker_position = attacker_position;
    swing.attacker_combat_reach = attacker_combat_reach;
    swing.attacker_can_state_update = attacker_can_state_update;

    if !attackers.attacker_is_canonical_incarnation_like_cpp(canonical_manager, &swing) {
        outcome.melee_precondition_rejections += 1;
        outcome.attacker_incarnation_rejections += 1;
        return;
    }
    attackers.align_canonical_attacker_position_like_cpp(canonical_manager, &swing);
    let primary_threat_plan = canonical_manager
        .find_map(u32::from(swing.map_id), swing.instance_id)
        .map(|managed| {
            super::creature_melee_threat::plan_creature_damage_threat_like_cpp(
                managed.map(),
                swing.attacker_guid,
                None,
                config.spell_store.as_deref(),
                config.spell_misc_store.as_deref(),
                config.spell_threat_store.as_deref(),
                config.spell_chain_store.as_deref(),
                managed.difficulty(),
                config.difficulty_store.as_deref(),
            )
        })
        .unwrap_or_default();

    let apply = |canonical_manager: &mut wow_map::MapManager,
                 swing: &PendingCreatureSwingLikeCpp,
                 damage,
                 presentation: Option<(u32, u8, i32)>,
                 absorbed: u32,
                 wire_health_before: Option<u64>,
                 represented_damage_done: Option<u32>| {
        if swing.victim_guid.is_player() {
            apply_creature_melee_damage_to_canonical_player_on_map_like_cpp(
                canonical_manager,
                u32::from(swing.map_id),
                swing.instance_id,
                swing.attacker_guid,
                swing.attacker_position,
                swing.attacker_combat_reach,
                swing.attacker_can_state_update,
                swing.victim_guid,
                damage,
                wire_health_before,
            )
        } else {
            apply_creature_melee_damage_to_canonical_creature_on_map_like_cpp(
                canonical_manager,
                u32::from(swing.map_id),
                swing.instance_id,
                swing.attacker_guid,
                swing.attacker_position,
                swing.attacker_combat_reach,
                swing.attacker_can_state_update,
                swing.victim_guid,
                damage,
                presentation,
                absorbed,
                wire_health_before,
                represented_damage_done,
                primary_threat_plan,
            )
        }
    };

    match apply(canonical_manager, &swing, None, None, 0, None, None) {
        CreatureMeleeApplyResultLikeCpp::Ready => {}
        CreatureMeleeApplyResultLikeCpp::Hit { .. } => {
            unreachable!("melee precondition validation must not mutate canonical health")
        }
        rejected => {
            let consequence = creature_melee_apply_rejection_like_cpp(&rejected, outcome);
            rearm(attackers, canonical_manager, &swing, consequence);
            return;
        }
    }

    if !swing.attacker_can_state_update {
        outcome.attacker_state_rejections += 1;
        rearm(
            attackers,
            canonical_manager,
            &swing,
            CreatureMeleeRearmLikeCpp::Swing,
        );
        return;
    }
    // The compatibility bridge preserves the pre-existing damage and wire
    // behavior, but it does not model RollMeleeOutcomeAgainst or later
    // proc/daze draws. Keep gameplay running while preventing a later
    // creature spell from claiming an exact shared-RNG position.
    let rolled = attackers.with_attacker_like_cpp(canonical_manager, &swing, |attacker| {
        let damage = attacker.roll_damage()?;
        attacker.invalidate_runtime_rng_authority_like_cpp();
        Some((
            damage,
            CreatureMeleeAttackerFactsLikeCpp::capture_like_cpp(attacker),
        ))
    });
    let Some((damage, attacker_facts)) = rolled.flatten() else {
        outcome.melee_precondition_rejections += 1;
        // `DoMeleeAttackIfReady` rearms BASE_ATTACK after
        // `AttackerStateUpdate` even if damage calculation cannot produce a
        // represented result.
        rearm(
            attackers,
            canonical_manager,
            &swing,
            CreatureMeleeRearmLikeCpp::Swing,
        );
        return;
    };
    let damage = damage.max(1);

    // C++ `CalculateMeleeDamage` rolls the attack table after mitigation and
    // before the outcome switch (`Unit.cpp:1341-1443`). A player victim
    // resolves miss/dodge/parry/crit here; the block band and the
    // player-victim armour/taken terms remain the documented boundary of
    // this slice. Every term needs the spell store, so without one the
    // pre-table always-hit bridge is preserved.
    let mut state = MeleeSwingStateLikeCpp::new_like_cpp(damage);
    let damage = if swing.victim_guid.is_player() {
        player_victim_damage_like_cpp(
            canonical_manager,
            &attacker_facts,
            &swing,
            config,
            damage,
            &mut state,
        )
    } else {
        creature_victim_damage_like_cpp(
            canonical_manager,
            &attacker_facts,
            &swing,
            config,
            damage,
            &mut state,
        )
    };
    let MeleeSecondaryTargetsOutcomeLikeCpp {
        damage,
        primary_wire_health_before,
        represented_damage_done,
        split_mutation_events,
        split_combat_log_packets,
        share_mutation_events,
        primary_was_share_target,
        primary_player_share_health_updates,
    } = apply_secondary_targets_damage_like_cpp(
        canonical_manager,
        &attacker_facts,
        &swing,
        config,
        damage,
        &mut state,
        creature_victim_syncs,
    );
    if !state.outcome_represented {
        outcome.melee_outcomes_unrepresented += 1;
    }

    // C++ `CalculateMeleeDamage` returns before `DealMeleeDamage` for an
    // avoided swing (`Unit.cpp:1345-1355`, `1395-1407`): no health write, no
    // death check and no proc. The command carries the victim's unchanged
    // canonical tuple so the session's revision gate stays exact.
    if state.avoided_outcome.is_some() {
        let victim = canonical_manager
            .find_map(u32::from(swing.map_id), swing.instance_id)
            .and_then(|managed| managed.map().get_typed_player(swing.victim_guid))
            .map(|victim| {
                (
                    victim.unit().data().health,
                    victim.unit().health_state_revision_like_cpp(),
                    victim.unit().data().level.clamp(0, i32::from(u8::MAX)) as u8,
                )
            });
        let Some((victim_health_after, victim_health_state_revision_after, target_level)) = victim
        else {
            outcome.melee_precondition_rejections += 1;
            return;
        };
        // C++ `Unit::AttackerStateUpdate` removes the attacking-interrupt
        // auras before `CalculateMeleeDamage`, so an avoided swing removes
        // them too (`Unit.cpp:2172-2173`).
        outcome.attacking_interrupt_auras_removed += attackers
            .with_attacker_like_cpp(canonical_manager, &swing, |attacker| {
                let removed = attacker
                    .unit_mut()
                    .remove_attacking_interrupt_auras_like_cpp();
                attacker.record_swing();
                removed
            })
            .unwrap_or_default();
        outcome.commands.push(
            crate::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand {
                attacker_guid: swing.attacker_guid,
                victim_guid: swing.victim_guid,
                map_id: swing.map_id,
                instance_id: swing.instance_id,
                damage: 0,
                over_damage: -1,
                target_level,
                victim_health_after,
                victim_health_state_revision_after,
                hit_info: state.hit_info,
                victim_state: state.victim_state,
                original_damage: state.original_damage,
                absorbed: 0,
                mana_spent: 0,
                absorb_consumptions: Vec::new(),
                split_combat_log_packets: Vec::new(),
                self_share_health_updates: Vec::new(),
            },
        );
        return;
    }

    let (
        victim_applied_damage,
        victim_threat,
        victim_health_before,
        victim_health_after,
        victim_health_state_revision_before,
        victim_health_state_revision_after,
        victim_creature_sync_identity,
        over_damage,
        target_level,
        events,
    ) = match apply(
        canonical_manager,
        &swing,
        Some(damage),
        state.creature_victim_presentation,
        state.absorbed_damage,
        primary_was_share_target
            .then_some(primary_wire_health_before)
            .flatten(),
        (!swing.victim_guid.is_player()).then_some(represented_damage_done),
    ) {
        CreatureMeleeApplyResultLikeCpp::Hit {
            victim_applied_damage,
            victim_threat,
            victim_health_before,
            victim_health_after,
            victim_health_state_revision_before,
            victim_health_state_revision_after,
            victim_creature_sync_identity,
            over_damage,
            target_level,
            events,
        } => (
            victim_applied_damage,
            victim_threat,
            victim_health_before,
            victim_health_after,
            victim_health_state_revision_before,
            victim_health_state_revision_after,
            victim_creature_sync_identity,
            over_damage,
            target_level,
            events,
        ),
        CreatureMeleeApplyResultLikeCpp::Ready => {
            unreachable!("melee apply with represented damage must not return validation readiness")
        }
        rejected => {
            let consequence = creature_melee_apply_rejection_like_cpp(&rejected, outcome);
            rearm(attackers, canonical_manager, &swing, consequence);
            return;
        }
    };
    outcome.attacking_interrupt_auras_removed += attackers
        .with_attacker_like_cpp(canonical_manager, &swing, |attacker| {
            let removed = attacker
                .unit_mut()
                .remove_attacking_interrupt_auras_like_cpp();
            attacker.record_swing();
            removed
        })
        .unwrap_or_default();
    outcome.canonical_hits += 1;
    if swing.victim_guid.is_player() {
        outcome.commands.push(
            crate::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand {
                attacker_guid: swing.attacker_guid,
                victim_guid: swing.victim_guid,
                map_id: swing.map_id,
                instance_id: swing.instance_id,
                damage,
                over_damage,
                target_level,
                victim_health_after,
                victim_health_state_revision_after,
                hit_info: state.hit_info,
                victim_state: state.victim_state,
                original_damage: state.original_damage,
                absorbed: state.absorbed_damage,
                mana_spent: state.mana_spent,
                absorb_consumptions: state.absorb_consumptions.clone(),
                split_combat_log_packets: split_combat_log_packets.clone(),
                self_share_health_updates: primary_player_share_health_updates,
            },
        );
        outcome.plan.events.extend(split_mutation_events);
        outcome.plan.events.extend(share_mutation_events);
    } else {
        if !state.creature_victim_avoided {
            outcome.canonical_creature_hits += 1;
        }
        outcome
            .plan
            .events
            .extend(state.creature_victim_absorb_events);
        outcome.plan.events.extend(split_mutation_events);
        outcome
            .plan
            .events
            .extend(
                split_combat_log_packets
                    .into_iter()
                    .map(|packet_bytes| RuntimeEvent {
                        source_guid: swing.victim_guid,
                        recipients: RecipientRule::MapBroadcastVisible {
                            map_id: swing.map_id,
                            instance_id: swing.instance_id,
                        },
                        packet_bytes,
                    }),
            );
        let mut primary_events = events.into_iter();
        if let Some(attacker_state) = primary_events.next() {
            outcome.plan.events.push(attacker_state);
        }
        outcome.plan.events.extend(share_mutation_events);
        outcome.plan.events.extend(primary_events);
        if victim_health_state_revision_after != victim_health_state_revision_before
            || victim_threat.is_some()
        {
            creature_victim_syncs.push(CreatureVictimCompatibilitySyncLikeCpp {
                swing,
                state: CreatureMeleeVictimSyncStateLikeCpp {
                    applied_damage: victim_applied_damage,
                    threat: victim_threat,
                    victim_health_before,
                    victim_health_after,
                    victim_health_state_revision_before,
                    victim_health_state_revision_after,
                    identity: victim_creature_sync_identity
                        .expect("creature victim commits carry incarnation authority"),
                },
            });
        }
    }
}
