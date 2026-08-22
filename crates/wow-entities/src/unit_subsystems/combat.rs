// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Unit combat subsystem.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CombatReferenceState {
    pub pvp: bool,
    pub suppressed_for_owner: bool,
    pub timeout_ms: Option<u32>,
}

impl CombatReferenceState {
    pub const fn pve() -> Self {
        Self {
            pvp: false,
            suppressed_for_owner: false,
            timeout_ms: None,
        }
    }

    pub const fn pvp() -> Self {
        Self {
            pvp: true,
            suppressed_for_owner: false,
            timeout_ms: Some(PVP_COMBAT_TIMEOUT_MS),
        }
    }

    pub fn refresh(&mut self) {
        self.suppressed_for_owner = false;
        if self.pvp {
            self.timeout_ms = Some(PVP_COMBAT_TIMEOUT_MS);
        }
    }

    pub fn suppress_for_owner(&mut self) {
        self.suppressed_for_owner = true;
    }

    pub fn update_pvp_timer(&mut self, diff_ms: u32) -> bool {
        if !self.pvp {
            return true;
        }
        let Some(timer) = self.timeout_ms.as_mut() else {
            return true;
        };
        if *timer <= diff_ms {
            return false;
        }
        *timer -= diff_ms;
        true
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CombatBeginContextLikeCpp {
    pub same_unit: bool,
    pub attacker_in_world: bool,
    pub victim_in_world: bool,
    pub attacker_alive: bool,
    pub victim_alive: bool,
    pub same_map: bool,
    pub same_phase: bool,
    pub attacker_unit_state: u32,
    pub victim_unit_state: u32,
    pub attacker_combat_disallowed: bool,
    pub victim_combat_disallowed: bool,
    pub relation_represented: bool,
    pub attacker_is_friendly_to_victim: bool,
    pub victim_is_friendly_to_attacker: bool,
    pub attacker_or_owner_player_is_game_master: bool,
    pub victim_or_owner_player_is_game_master: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CombatSubsystem {
    pub threat: HashMap<ObjectGuid, f32>,
    pub threat_refs: HashMap<ObjectGuid, ThreatReferenceState>,
    pub threatened_by_me: HashMap<ObjectGuid, ThreatReferenceState>,
    pub current_victim_guid: Option<ObjectGuid>,
    pub fixate_guid: Option<ObjectGuid>,
    pub owner_can_have_threat_list: bool,
    pub need_client_update: bool,
    pub threat_update_timer_ms: u32,
    pub pve_refs: HashMap<ObjectGuid, CombatReferenceState>,
    pub pvp_refs: HashMap<ObjectGuid, CombatReferenceState>,
    pub attackers: HashSet<ObjectGuid>,
    pub attacking_guid: Option<ObjectGuid>,
    pub last_damaged_target_guid: Option<ObjectGuid>,
    pub extra_attacks_targets: HashMap<ObjectGuid, u32>,
    pub combat_disallowed: bool,
    /// C++ `AddThreat` lets only the target that caused new threat leave
    /// `ONLINE_STATE_SUPPRESSED` once its suppressing condition has cleared.
    pub(super) pending_suppressed_threat_like_cpp: HashMap<ObjectGuid, f32>,
    /// C++ `TauntUpdate` calls `EvaluateSuppressed(true)` for every reference.
    pub(super) reevaluate_all_suppressed_like_cpp: bool,
}

impl CombatSubsystem {
    pub fn can_begin_combat_like_cpp(context: CombatBeginContextLikeCpp) -> bool {
        if context.same_unit {
            return false;
        }
        if !context.attacker_in_world || !context.victim_in_world {
            return false;
        }
        if !context.attacker_alive || !context.victim_alive {
            return false;
        }
        if !context.same_map {
            return false;
        }
        if !context.same_phase {
            return false;
        }
        if context.attacker_unit_state & UnitState::EVADE.bits() != 0
            || context.victim_unit_state & UnitState::EVADE.bits() != 0
        {
            return false;
        }
        if context.attacker_unit_state & UnitState::IN_FLIGHT.bits() != 0
            || context.victim_unit_state & UnitState::IN_FLIGHT.bits() != 0
        {
            return false;
        }
        if context.attacker_combat_disallowed || context.victim_combat_disallowed {
            return false;
        }
        if context.relation_represented
            && (context.attacker_is_friendly_to_victim || context.victim_is_friendly_to_attacker)
        {
            return false;
        }
        if context.attacker_or_owner_player_is_game_master
            || context.victim_or_owner_player_is_game_master
        {
            return false;
        }
        true
    }

    pub fn initialize_threat_list_capability(&mut self, can_have_threat_list: bool) {
        self.owner_can_have_threat_list = can_have_threat_list;
    }

    pub fn add_threat(&mut self, target: ObjectGuid, amount: f32) -> f32 {
        // C++ ThreatManager::AddThreat is a no-op for owners which cannot
        // own a threat list (triggers, pets, totems, minions and guardians),
        // but it still establishes the owner's combat reference.
        if !self.owner_can_have_threat_list {
            self.set_in_combat_with(target, false, false);
            return 0.0;
        }
        let threat_ref = self.threat_refs.entry(target).or_insert_with(|| {
            let mut threat_ref = ThreatReferenceState::default();
            threat_ref.set_online_state(ThreatOnlineState::Online);
            threat_ref
        });
        if threat_ref.is_suppressed() {
            *self
                .pending_suppressed_threat_like_cpp
                .entry(target)
                .or_default() += amount;
        } else {
            // C++ ThreatReference::AddThreat continues accumulating the base
            // value while an otherwise valid reference is temporarily
            // offline. Offline only excludes it from victim selection.
            threat_ref.add_threat(amount);
        }
        let value = threat_ref.threat();
        self.threat.insert(target, value);
        self.need_client_update = true;
        if self.current_victim_guid.is_none() && threat_ref.is_available() {
            self.current_victim_guid = Some(target);
        }
        value
    }

    pub fn request_taunt_suppression_reevaluation_like_cpp(&mut self) {
        self.reevaluate_all_suppressed_like_cpp = true;
    }

    pub fn take_suppressed_reactivation_requests_like_cpp(
        &mut self,
    ) -> (bool, HashMap<ObjectGuid, f32>) {
        (
            std::mem::take(&mut self.reevaluate_all_suppressed_like_cpp),
            std::mem::take(&mut self.pending_suppressed_threat_like_cpp),
        )
    }

    pub fn set_threat(&mut self, target: ObjectGuid, value: f32) {
        let threat_ref = self.threat_refs.entry(target).or_insert_with(|| {
            let mut threat_ref = ThreatReferenceState::default();
            threat_ref.set_online_state(ThreatOnlineState::Online);
            threat_ref
        });
        threat_ref.base_amount = value.max(0.0);
        self.threat.insert(target, threat_ref.threat());
        self.need_client_update = true;
    }

    pub fn threat_value(&self, target: ObjectGuid) -> Option<f32> {
        self.threat_ref(target).map(ThreatReferenceState::threat)
    }

    pub fn remove_threat(&mut self, target: ObjectGuid) -> Option<f32> {
        let removed = self.threat_refs.remove(&target).map(|state| state.threat());
        self.threat.remove(&target);
        self.threatened_by_me.remove(&target);
        self.pending_suppressed_threat_like_cpp.remove(&target);
        if self.current_victim_guid == Some(target) {
            self.current_victim_guid = None;
        }
        if self.fixate_guid == Some(target) {
            self.fixate_guid = None;
        }
        removed
    }

    pub fn clear_threat(&mut self) {
        self.threat.clear();
        self.threat_refs.clear();
        self.current_victim_guid = None;
        self.fixate_guid = None;
        self.pending_suppressed_threat_like_cpp.clear();
        self.reevaluate_all_suppressed_like_cpp = false;
        self.need_client_update = true;
    }

    pub fn is_threatened_by(&self, target: ObjectGuid) -> bool {
        self.is_threatened_by_with_offline(target, false)
    }

    pub fn is_threatened_by_with_offline(&self, target: ObjectGuid, include_offline: bool) -> bool {
        self.threat_refs
            .get(&target)
            .is_some_and(|threat_ref| include_offline || threat_ref.is_available())
    }

    pub fn threat_ref(&self, target: ObjectGuid) -> Option<&ThreatReferenceState> {
        self.threat_refs.get(&target)
    }

    pub fn threat_ref_mut(&mut self, target: ObjectGuid) -> Option<&mut ThreatReferenceState> {
        self.threat_refs.get_mut(&target)
    }

    pub fn scale_threat(&mut self, target: ObjectGuid, factor: f32) -> Option<f32> {
        let threat_ref = self.threat_refs.get_mut(&target)?;
        threat_ref.scale_threat(factor);
        let value = threat_ref.threat();
        self.threat.insert(target, value);
        self.need_client_update = true;
        Some(value)
    }

    pub fn modify_threat_by_percent(&mut self, target: ObjectGuid, percent: i32) -> Option<f32> {
        let threat_ref = self.threat_refs.get_mut(&target)?;
        threat_ref.modify_threat_by_percent(percent);
        let value = threat_ref.threat();
        self.threat.insert(target, value);
        self.need_client_update = true;
        Some(value)
    }

    pub fn match_unit_threat_to_highest_threat_like_cpp(
        &mut self,
        target: ObjectGuid,
    ) -> Option<f32> {
        let sorted = self.sorted_threat_guids();
        let highest_guid = *sorted
            .iter()
            .find(|guid| self.threat_refs[*guid].is_available())?;
        let mut highest_ref = self.threat_refs[&highest_guid];

        if highest_ref.is_taunting()
            && let Some(next_guid) = sorted
                .into_iter()
                .skip_while(|guid| *guid != highest_guid)
                .nth(1)
        {
            let next_ref = self.threat_refs[&next_guid];
            if next_ref.is_available() && next_ref.threat() > highest_ref.threat() {
                highest_ref = next_ref;
            }
        }

        let current_threat = self
            .threat_refs
            .get(&target)
            .filter(|threat_ref| threat_ref.is_available())
            .map_or(0.0, ThreatReferenceState::threat);
        Some(self.add_threat(target, highest_ref.threat() - current_threat))
    }

    pub fn reset_all_threat(&mut self) {
        for (guid, threat_ref) in &mut self.threat_refs {
            threat_ref.scale_threat(0.0);
            self.threat.insert(*guid, threat_ref.threat());
        }
        self.need_client_update = true;
    }

    pub fn threat_list_size(&self) -> usize {
        self.threat_refs.len()
    }

    pub fn is_threat_list_empty(&self, include_offline: bool) -> bool {
        if include_offline {
            return self.threat_refs.is_empty();
        }
        self.threat_refs
            .values()
            .all(|threat_ref| !threat_ref.is_available())
    }

    pub fn sorted_threat_guids(&self) -> Vec<ObjectGuid> {
        let mut refs: Vec<_> = self
            .threat_refs
            .iter()
            .map(|(guid, threat_ref)| (*guid, *threat_ref))
            .collect();
        refs.sort_by(|(left_guid, left), (right_guid, right)| {
            compare_threat_refs(*right, *left).then_with(|| {
                (left_guid.high_value(), left_guid.low_value())
                    .cmp(&(right_guid.high_value(), right_guid.low_value()))
            })
        });
        refs.into_iter().map(|(guid, _)| guid).collect()
    }

    pub fn set_threat_online_state(
        &mut self,
        target: ObjectGuid,
        online_state: ThreatOnlineState,
    ) -> bool {
        let Some(threat_ref) = self.threat_refs.get_mut(&target) else {
            return false;
        };
        threat_ref.set_online_state(online_state);
        self.need_client_update = true;
        true
    }

    pub fn set_threat_taunt_state(
        &mut self,
        target: ObjectGuid,
        taunt_state: ThreatTauntState,
    ) -> bool {
        let Some(threat_ref) = self.threat_refs.get_mut(&target) else {
            return false;
        };
        threat_ref.set_taunt_state(taunt_state);
        self.need_client_update = true;
        true
    }

    pub fn fixate_target(&mut self, target: Option<ObjectGuid>) -> bool {
        if let Some(target) = target {
            if !self.threat_refs.contains_key(&target) {
                return false;
            }
            self.fixate_guid = Some(target);
        } else {
            self.fixate_guid = None;
        }
        true
    }

    pub fn reselect_victim(
        &mut self,
        melee_candidate_guids: &HashSet<ObjectGuid>,
    ) -> Option<ObjectGuid> {
        if let Some(fixate) = self.fixate_guid {
            if self
                .threat_refs
                .get(&fixate)
                .is_some_and(ThreatReferenceState::is_online)
            {
                self.current_victim_guid = Some(fixate);
                return Some(fixate);
            }
        }

        if let Some(taunter) = self
            .sorted_threat_guids()
            .into_iter()
            .find(|guid| self.threat_refs[guid].is_online() && self.threat_refs[guid].is_taunting())
        {
            self.current_victim_guid = Some(taunter);
            return Some(taunter);
        }

        let sorted = self.sorted_threat_guids();
        let highest_guid = sorted
            .iter()
            .copied()
            .find(|guid| self.threat_refs[guid].is_online())?;
        let Some(old_guid) = self.current_victim_guid else {
            self.current_victim_guid = Some(highest_guid);
            return Some(highest_guid);
        };
        let Some(old_ref) = self.threat_refs.get(&old_guid).copied() else {
            self.current_victim_guid = Some(highest_guid);
            return Some(highest_guid);
        };
        if !old_ref.is_online() || old_guid == highest_guid {
            self.current_victim_guid = Some(highest_guid);
            return Some(highest_guid);
        }

        let highest_ref = self.threat_refs[&highest_guid];
        if old_ref.threat() * 1.1 >= highest_ref.threat() {
            return self.current_victim_guid;
        }
        if old_ref.threat() * 1.3 < highest_ref.threat()
            || melee_candidate_guids.contains(&highest_guid)
        {
            self.current_victim_guid = Some(highest_guid);
            return self.current_victim_guid;
        }

        for next_guid in sorted
            .into_iter()
            .filter(|guid| self.threat_refs[guid].is_online() && *guid != highest_guid)
        {
            if next_guid == old_guid
                || old_ref.threat() * 1.1 >= self.threat_refs[&next_guid].threat()
            {
                break;
            }
            if melee_candidate_guids.contains(&next_guid) {
                self.current_victim_guid = Some(next_guid);
                break;
            }
        }
        self.current_victim_guid
    }

    pub fn put_threatened_by_me_ref(
        &mut self,
        owner: ObjectGuid,
        threat_ref: ThreatReferenceState,
    ) {
        self.threatened_by_me.insert(owner, threat_ref);
    }

    pub fn purge_threatened_by_me_ref(
        &mut self,
        owner: ObjectGuid,
    ) -> Option<ThreatReferenceState> {
        self.threatened_by_me.remove(&owner)
    }

    pub fn is_threatening_anyone(&self, include_offline: bool) -> bool {
        if include_offline {
            return !self.threatened_by_me.is_empty();
        }
        self.threatened_by_me
            .values()
            .any(ThreatReferenceState::is_available)
    }

    pub fn is_threatening_to(&self, owner: ObjectGuid, include_offline: bool) -> bool {
        self.threatened_by_me
            .get(&owner)
            .is_some_and(|threat_ref| include_offline || threat_ref.is_available())
    }

    pub fn threatened_by_me_owner_guids(&self) -> Vec<ObjectGuid> {
        self.threatened_by_me.keys().copied().collect()
    }

    pub fn set_in_combat_with(
        &mut self,
        target: ObjectGuid,
        both_player_controlled: bool,
        add_target_suppressed: bool,
    ) -> bool {
        if let Some(reference) = self.pvp_refs.get_mut(&target) {
            reference.refresh();
            return !reference.suppressed_for_owner;
        }
        if let Some(reference) = self.pve_refs.get_mut(&target) {
            reference.refresh();
            return !reference.suppressed_for_owner;
        }

        let mut reference = if both_player_controlled {
            CombatReferenceState::pvp()
        } else {
            CombatReferenceState::pve()
        };
        if add_target_suppressed {
            reference.suppress_for_owner();
        }
        if reference.pvp {
            self.pvp_refs.insert(target, reference);
        } else {
            self.pve_refs.insert(target, reference);
        }
        true
    }

    pub fn is_in_combat_with(&self, target: ObjectGuid) -> bool {
        self.pve_refs.contains_key(&target) || self.pvp_refs.contains_key(&target)
    }

    pub fn purge_combat_ref_like_cpp(&mut self, target: ObjectGuid) -> bool {
        let removed =
            self.pve_refs.remove(&target).is_some() || self.pvp_refs.remove(&target).is_some();
        if removed {
            self.remove_threat(target);
            self.threatened_by_me.remove(&target);
        }
        removed
    }

    pub fn has_pve_combat(&self) -> bool {
        self.pve_refs
            .values()
            .any(|reference| !reference.suppressed_for_owner)
    }

    pub fn has_pvp_combat(&self) -> bool {
        self.pvp_refs
            .values()
            .any(|reference| !reference.suppressed_for_owner)
    }

    pub fn has_combat(&self) -> bool {
        self.has_pve_combat() || self.has_pvp_combat()
    }

    pub fn suppress_pvp_combat(&mut self) {
        for reference in self.pvp_refs.values_mut() {
            reference.suppress_for_owner();
        }
    }

    pub fn update_pvp_combat(&mut self, diff_ms: u32) -> Vec<ObjectGuid> {
        let expired: Vec<_> = self
            .pvp_refs
            .iter_mut()
            .filter_map(|(guid, reference)| (!reference.update_pvp_timer(diff_ms)).then_some(*guid))
            .collect();
        for guid in &expired {
            self.pvp_refs.remove(guid);
            // C++ `PvPCombatReference::Update` hands the shared reference to
            // `CombatReference::EndCombat`, which clears threat on both units
            // before removing either combat reference. Rust stores each side
            // independently, so clear the owner-side threat state here before
            // the map purges the reciprocal side.
            self.remove_threat(*guid);
            self.threatened_by_me.remove(guid);
        }
        expired
    }

    pub fn revalidate_combat_like_cpp(
        &mut self,
        mut can_begin_combat: impl FnMut(ObjectGuid, CombatReferenceState) -> bool,
    ) -> Vec<ObjectGuid> {
        let mut removed = Vec::new();
        self.pve_refs.retain(|guid, reference| {
            if can_begin_combat(*guid, *reference) {
                true
            } else {
                removed.push(*guid);
                false
            }
        });
        self.pvp_refs.retain(|guid, reference| {
            if can_begin_combat(*guid, *reference) {
                true
            } else {
                removed.push(*guid);
                false
            }
        });
        for guid in &removed {
            self.remove_threat(*guid);
            self.threatened_by_me.remove(guid);
        }
        removed
    }

    pub fn end_all_pve_combat(&mut self) {
        self.pve_refs.clear();
        self.clear_threat();
        self.threatened_by_me.clear();
    }

    pub fn end_all_pvp_combat(&mut self) {
        self.pvp_refs.clear();
    }

    pub fn end_all_combat(&mut self) {
        self.end_all_pve_combat();
        self.end_all_pvp_combat();
    }

    pub fn add_attacker(&mut self, attacker: ObjectGuid) -> bool {
        self.attackers.insert(attacker)
    }

    pub fn remove_attacker(&mut self, attacker: ObjectGuid) -> bool {
        self.attackers.remove(&attacker)
    }

    pub fn clear_attackers(&mut self) {
        self.attackers.clear();
        self.attacking_guid = None;
    }

    pub fn set_attacking(&mut self, victim: Option<ObjectGuid>) {
        self.attacking_guid = victim;
    }

    pub fn set_last_damaged_target_like_cpp(&mut self, target: Option<ObjectGuid>) {
        self.last_damaged_target_guid = target;
    }

    pub fn add_extra_attacks_for_like_cpp(&mut self, target: ObjectGuid, count: u32) -> u32 {
        let entry = self.extra_attacks_targets.entry(target).or_insert(0);
        *entry = entry.saturating_add(count);
        *entry
    }

    pub fn extra_attacks_for_like_cpp(&self, target: ObjectGuid) -> u32 {
        self.extra_attacks_targets
            .get(&target)
            .copied()
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlledOwnerAttackedNotification {
    pub controlled: ObjectGuid,
    pub victim: ObjectGuid,
}
