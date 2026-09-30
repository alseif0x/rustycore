//! Original map-tail assistance search, with its only I/O boundary made explicit.
use super::*;
use wow_entities::{LineOfSightEndpoint, LineOfSightOptions, LineOfSightQuery};

pub(crate) struct AggroLosPending {
    pub(crate) frame: AggroFrame,
    pub(crate) caller: ObjectGuid,
    pub(crate) assistant: ObjectGuid,
    pub(crate) victim: ObjectGuid,
    pub(crate) from: LineOfSightEndpoint,
    pub(crate) to: LineOfSightEndpoint,
}

impl AggroLosPending {
    pub(crate) fn map_id(&self) -> u16 {
        self.frame.map_id
    }
    pub(crate) fn partial(&self) -> &AggroOutcome {
        &self.frame.outcome
    }
    pub(crate) fn referenced_guids(&self) -> impl Iterator<Item = ObjectGuid> + '_ {
        [self.caller, self.assistant, self.victim]
            .into_iter()
            .chain(self.frame.assistants.iter().copied())
    }
    pub(crate) fn resolve(&self, terrain: &super::super::LiveTerrainHeights) -> bool {
        let source = &self.frame.calls[self.frame.call_index].2;
        terrain.resolve_aggro_los(u32::from(self.frame.map_id), source, self.from, self.to)
    }

    pub(crate) fn resume(
        mut self,
        visible: bool,
        backend: &mut AggroMap<'_>,
        policies: &mut AggroPolicies<'_>,
    ) -> AggroFrame {
        if visible {
            self.frame
                .accept_hostile_assistant(backend, policies, self.assistant);
        }
        // This assistant's pre-LOS gates have already run. Never replay them.
        self.frame.assistant_index += 1;
        self.frame
    }
}

impl AggroFrame {
    pub(crate) fn prepare_tail(
        mut self,
        backend: &mut AggroMap<'_>,
        policies: &mut AggroPolicies<'_>,
        terrain_enabled: bool,
    ) -> AggroTailProgress {
        if !(self.settings.family_assistance_radius > 0.0) {
            return AggroTailProgress::Complete(self.outcome);
        }
        while self.call_index < self.calls.len() {
            let caller_guid = self.calls[self.call_index].0;
            let victim_guid = self.calls[self.call_index].1;
            let caller_faction = self.calls[self.call_index].3;
            if !self.candidates.iter().any(|candidate| {
                candidate.map_id == self.map_id
                    && candidate.instance_id == self.instance_id
                    && candidate.player_guid == victim_guid
            }) && !self.owners.contains_key(&victim_guid)
            {
                self.advance_call();
                continue;
            }
            while self.assistant_index < self.secondary_guids.len() {
                let assistant_guid = self.secondary_guids[self.assistant_index];
                if assistant_guid == caller_guid {
                    self.assistant_index += 1;
                    continue;
                }
                let Some(assistant) = backend.actor_mut(assistant_guid) else {
                    self.assistant_index += 1;
                    continue;
                };
                let flags = assistant.creature.unit().unit_flags_like_cpp();
                if !assistant.is_alive()
                    || assistant.creature.is_in_combat()
                    || assistant.creature.is_in_evade_mode_like_cpp()
                    || assistant.creature.unit().has_unit_state(
                        (UnitState::STUNNED | UnitState::CONFUSED | UnitState::FLEEING).bits(),
                    )
                    || !assistant
                        .creature
                        .has_react_state(wow_entities::ReactState::Aggressive)
                    || assistant.creature.is_civilian_like_cpp()
                    || assistant
                        .creature
                        .unit()
                        .subsystems()
                        .control
                        .charmer_or_owner_guid()
                        .is_some()
                    || flags.intersects(
                        UnitFlags::NON_ATTACKABLE
                            | UnitFlags::IMMUNE_TO_NPC
                            | UnitFlags::UNINTERACTIBLE,
                    )
                    || assistant.creature.unit().data().faction_template != caller_faction
                    || !assistant.position().is_within_dist(
                        &self.calls[self.call_index].2.position(),
                        self.settings.family_assistance_radius,
                    )
                {
                    self.assistant_index += 1;
                    continue;
                }
                if terrain_enabled {
                    // Exactly the old object-to-object collision-height/hit-sphere
                    // endpoints. The continuation owns the original caller_world
                    // clone; the assistant's WorldObject is never cloned.
                    let query = LineOfSightQuery::to_object_like_cpp(
                        &self.calls[self.call_index].2,
                        assistant.creature.unit().world(),
                        LineOfSightOptions::default(),
                    );
                    let from = query.from;
                    let to = query.to;
                    return AggroTailProgress::Pending(AggroLosPending {
                        frame: self,
                        caller: caller_guid,
                        assistant: assistant_guid,
                        victim: victim_guid,
                        from,
                        to,
                    });
                }
                self.accept_hostile_assistant(backend, policies, assistant_guid);
                self.assistant_index += 1;
            }
            if !self.assistants.is_empty() {
                self.outcome.assistance_scheduled += self.assistants.len();
                if let Some(caller) = backend.actor_mut(caller_guid) {
                    caller.schedule_assistance_like_cpp(
                        victim_guid,
                        std::mem::take(&mut self.assistants),
                        self.settings.family_assistance_delay_ms,
                    );
                }
            }
            self.advance_call();
        }
        AggroTailProgress::Complete(self.outcome)
    }

    fn accept_hostile_assistant(
        &mut self,
        backend: &mut AggroMap<'_>,
        policies: &mut AggroPolicies<'_>,
        guid: ObjectGuid,
    ) {
        let victim_guid = self.calls[self.call_index].1;
        if !backend.victim_present(victim_guid) {
            return;
        }
        let Some(assistant) = backend.actor_mut(guid) else {
            return;
        };
        let victim = self.candidates.iter().find(|candidate| {
            candidate.map_id == self.map_id
                && candidate.instance_id == self.instance_id
                && candidate.player_guid == victim_guid
        });
        let victim_snapshot = self.owners.get(&victim_guid);
        if victim
            .is_some_and(|victim| candidate_hostile(assistant, victim, policies).unwrap_or(false))
            || victim_snapshot.is_some_and(|victim| {
                !victim.in_evade_mode
                    && snapshot_hostile(assistant, victim, policies).unwrap_or(false)
            })
        {
            self.assistants.push(guid);
        }
    }

    fn advance_call(&mut self) {
        self.call_index += 1;
        self.assistant_index = 0;
        self.assistants.clear();
    }
}
