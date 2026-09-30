//! Ordered per-map action queue with owned LOS and wire publication barriers.
use super::*;
use super::validation::{CastPrefix, SpellCastPending};
use wow_entities::LineOfSightEndpoint;

pub struct SpellQueue {
    actions: VecDeque<SpellAction>,
    outcome: SpellOutcome,
}
pub enum SpellProgress {
    Complete(SpellOutcome), Pending(SpellLosPending),
    Publication { completion: SpellCompletion, continuation: SpellPublicationPending },
}
pub struct SpellLosPending {
    queue: SpellQueue,
    cast: SpellCastPending,
}
pub struct SpellPublicationPending {
    queue: SpellQueue,
    state: super::finish::SpellPublicationState,
}

impl SpellQueue {
    pub(crate) fn prepare(backend: &mut SpellMap<'_>, primaries: Vec<ObjectGuid>,
        map_id: u16, instance_id: u32, difficulty: u8, policies: &mut SpellPolicies<'_>) -> Self {
        let mut outcome = SpellOutcome { maps_seen: 1, ..SpellOutcome::default() };
        let mut actions = Vec::new();
        for guid in primaries {
            let Some(actor) = backend.actor_mut(guid) else { continue; };
            outcome.creatures_seen += 1;
            if !actor.is_alive() || actor.state() != CreatureAiState::InCombat { continue; }
            let Some(victim) = actor.creature.ai_ownership().combat_target else { continue; };
            // Creature-vs-creature effects remain outside this wire-only slice.
            if !victim.is_player() { continue; }
            let ai = (policies.select_ai)(actor.aggro_ai_facts());
            match ai {
                SpellAiKind::Unrepresented => outcome.ai_selection_unrepresented += 1,
                SpellAiKind::Combat => combat::combat(actor, guid, victim, map_id, instance_id,
                    difficulty, policies, &mut outcome, &mut actions),
                SpellAiKind::Turret => turret::turret(actor, guid, victim, map_id, instance_id,
                    difficulty, policies, &mut outcome, &mut actions),
                SpellAiKind::Other => {},
            }
        }
        Self { actions: actions.into(), outcome }
    }

    pub fn into_parts(self) -> (Vec<SpellAction>, SpellOutcome) {
        (self.actions.into(), self.outcome)
    }

    pub(crate) fn from_action(action: SpellAction) -> Self {
        let casts_ready = usize::from(matches!(&action, SpellAction::Cast(_)));
        Self { actions: VecDeque::from([action]), outcome: SpellOutcome { casts_ready, ..SpellOutcome::default() } }
    }

    pub(crate) fn consume(mut self, backend: &mut SpellMap<'_>, policies: &mut SpellPolicies<'_>,
        terrain_enabled: bool) -> SpellProgress {
        while let Some(action) = self.actions.pop_front() {
            match action {
                SpellAction::Schedule(schedule) => {
                    if !consume_schedule(backend.actor_mut(schedule.caster_guid), &schedule) {
                        self.outcome.runtime_rng_authority_rejections += 1;
                    }
                }
                SpellAction::TurretRejectedAttempt(attempt) => {
                    if validation::rejected_attempt(backend, attempt, policies) {
                        self.outcome.turret_rejected_attempt_swings += 1;
                    }
                }
                SpellAction::Cast(cast) => match validation::prepare_cast(backend, cast, policies, terrain_enabled) {
                    CastPrefix::Rejected(result) => self.record(result),
                    CastPrefix::Ready(cast) => {
                        let (result, publication) = finish::finish_cast(backend, cast, true, policies);
                        self.record(result);
                        if let Some((completion, state)) = publication {
                            return SpellProgress::Publication { completion,
                                continuation: SpellPublicationPending { queue: self, state } };
                        }
                    }
                    CastPrefix::Pending(cast) => return SpellProgress::Pending(SpellLosPending { queue: self, cast }),
                },
            }
        }
        SpellProgress::Complete(self.outcome)
    }

    fn record(&mut self, result: SpellValidation) {
        match result {
            SpellValidation::Ready(hit) => {
                self.outcome.canonical_cast_preconditions_passed += 1;
                match hit { SpellHit::Hit => self.outcome.spell_hits += 1, SpellHit::Miss => self.outcome.spell_misses += 1 }
            }
            SpellValidation::OutOfRange => self.outcome.spell_range_rejections += 1,
            SpellValidation::LosRejected => self.outcome.spell_los_rejections += 1,
            SpellValidation::MissingTarget => self.outcome.canonical_cast_missing_target += 1,
            SpellValidation::TargetRejected => self.outcome.canonical_cast_target_rejections += 1,
            SpellValidation::CooldownRejected => self.outcome.canonical_cast_cooldown_rejections += 1,
            SpellValidation::HitResultUnrepresented => self.outcome.spell_hit_results_unrepresented += 1,
            SpellValidation::RuntimeRngAuthorityRejected => self.outcome.runtime_rng_authority_rejections += 1,
            SpellValidation::CasterIncarnationRejected => self.outcome.caster_incarnation_rejections += 1,
        }
        if !matches!(result, SpellValidation::Ready(_)) {
            self.outcome.casts_ready = self.outcome.casts_ready.saturating_sub(1);
        }
    }
}

impl SpellLosPending {
    pub fn partial(&self) -> &SpellOutcome { &self.queue.outcome }
    pub fn into_partial(self) -> SpellOutcome { self.queue.outcome }
    pub(crate) fn validate_live_cast(&self, actor: &WorldCreature, caster: &Creature) -> Result<(), SpellValidation> {
        validate_live_command(&self.cast.cast.command, actor, caster)
    }
    pub fn endpoints(&self) -> (LineOfSightEndpoint, LineOfSightEndpoint) { (self.cast.from, self.cast.to) }
    pub fn key(&self) -> crate::MapKey {
        crate::MapKey::new(u32::from(self.cast.cast.command.map_id), self.cast.cast.command.instance_id)
    }
    pub(crate) fn referenced_guids(&self) -> (ObjectGuid, ObjectGuid) {
        (self.cast.cast.command.caster_guid, self.cast.cast.command.target_guid)
    }
    pub(crate) fn resume(self, backend: &mut SpellMap<'_>, response: bool,
        policies: &mut SpellPolicies<'_>, terrain_enabled: bool) -> SpellProgress {
        let Self { mut queue, cast } = self;
        let (result, publication) = finish::finish_cast(backend, cast, response, policies);
        queue.record(result);
        if let Some((completion, state)) = publication {
            return SpellProgress::Publication { completion,
                continuation: SpellPublicationPending { queue, state } };
        }
        queue.consume(backend, policies, terrain_enabled)
    }
}

impl SpellPublicationPending {
    pub fn partial(&self) -> &SpellOutcome { &self.queue.outcome }
    pub fn into_partial(self) -> SpellOutcome { self.queue.outcome }
    pub(crate) fn key(&self) -> crate::MapKey {
        crate::MapKey::new(u32::from(self.state.command.map_id), self.state.command.instance_id)
    }
    pub(crate) fn referenced_guids(&self) -> (ObjectGuid, ObjectGuid) {
        (self.state.command.caster_guid, self.state.command.target_guid)
    }
    pub(crate) fn validate_live_cast(&self, actor: &WorldCreature, caster: &Creature) -> Result<(), SpellValidation> {
        validate_live_command(&self.state.command, actor, caster)
    }
    pub(crate) fn resume(self, backend: &mut SpellMap<'_>, policies: &mut SpellPolicies<'_>,
        terrain_enabled: bool) -> SpellProgress {
        if self.state.hit == SpellHit::Hit {
            // C++ enters HandleLaunchPhase before SendSpellGo. Every HIT target
            // consumes at least the unconditional `roll_chance_f(critChance)` in
            // PreprocessSpellLaunch, followed by effect-specific value/variance
            // draws that this wire-only slice does not own. Publish the already
            // resolved HIT topology, then stop later creature-spell schedules or
            // hit results from claiming an exact shared-RNG position. Transitional
            // melee and movement continue best-effort instead of freezing gameplay.
            backend.actor_mut(self.state.command.caster_guid).unwrap().invalidate_runtime_rng_authority_like_cpp();
        }
        self.queue.consume(backend, policies, terrain_enabled)
    }
}

fn validate_live_command(command: &SpellCastPlan, actor: &WorldCreature,
    caster: &Creature) -> Result<(), SpellValidation> {
    if !actor.is_alive() || actor.state() != CreatureAiState::InCombat
        || actor.creature.ai_ownership().combat_target != Some(command.target_guid)
        || actor.creature_spell_engagement_epoch_like_cpp() != command.engagement_epoch
    { return Err(SpellValidation::MissingTarget); }
    if !command.caster_incarnation.matches(&actor.creature) || !command.caster_incarnation.matches(caster) {
        return Err(SpellValidation::CasterIncarnationRejected);
    }
    Ok(())
}
