//! Short source borrows for one synchronous creature-melee operation.
//! Canonical actors remain in their Map slot throughout every victim lookup.
use super::*;
use crate::MapKey;
use crate::map::CreatureActorWitness;
use wow_constants::unit::UnitState;
use wow_entities::CurrentSpellSlot;

pub(super) enum CreatureMeleeSource<'a> {
    Legacy(&'a mut WorldCreature),
    CanonicalSelected {
        key: MapKey,
        guid: ObjectGuid,
        witness: CreatureActorWitness,
    },
}

pub(super) enum SourceAdmission {
    Precondition,
    Incarnation,
}

impl CreatureMeleeSource<'_> {
    // These raw borrows remain private to this source implementation. Phase
    // methods return only scalars/resolved effects and never export an actor.
    fn actor<'s>(&'s self, manager: &'s MapManager) -> &'s WorldCreature {
        match self {
            Self::Legacy(actor) => actor,
            Self::CanonicalSelected { key, guid, witness } => {
                let map = manager
                    .find_map(key.map_id, key.instance_id)
                    .expect("the validated map remains borrowed by the synchronous motor")
                    .map();
                assert!(
                    map.creature_actor_witness(*guid)
                        .is_some_and(|current| current.same_actor(witness))
                );
                map.creature_actor(*guid)
                    .expect("the admitted actor remains in its slot")
            }
        }
    }

    fn actor_mut<'s>(&'s mut self, manager: &'s mut MapManager) -> &'s mut WorldCreature {
        match self {
            Self::Legacy(actor) => actor,
            Self::CanonicalSelected { key, guid, witness } => {
                let map = manager
                    .find_map_mut(key.map_id, key.instance_id)
                    .expect("the validated map remains borrowed by the synchronous motor")
                    .map_mut();
                assert!(
                    map.creature_actor_witness(*guid)
                        .is_some_and(|current| current.same_actor(witness))
                );
                map.creature_actor_mut(*guid)
                    .expect("the admitted actor remains in its slot")
            }
        }
    }

    pub(super) fn prepare_swing(
        &mut self,
        canonical_manager: &mut MapManager,
        swing: &mut PendingCreatureSwingLikeCpp,
    ) -> Result<(), SourceAdmission> {
        let attacker = self.actor(canonical_manager);
        if !attacker.can_swing()
            || attacker.creature.ai_ownership().combat_target != Some(swing.victim_guid)
            || attacker.creature.lifecycle_metadata().ai_name == "TurretAI"
            || !attacker.creature.can_melee_like_cpp()
        {
            return Err(SourceAdmission::Precondition);
        }
        let unit = attacker.creature.unit();
        if unit.has_unit_state(UnitState::CHARGING.bits())
            || (unit.has_unit_state(UnitState::CASTING.bits())
                && !unit
                    .current_spell(CurrentSpellSlot::Channeled)
                    .is_some_and(|spell| spell.allow_actions_during_channel))
        {
            return Err(SourceAdmission::Precondition);
        }
        swing.attacker_position = attacker.position();
        swing.attacker_combat_reach = unit.world().combat_reach();
        swing.attacker_can_state_update = unit.can_attacker_state_update_melee_like_cpp(false);

        // The preceding read borrow ends before the Legacy alignment write.
        if let Self::Legacy(attacker) = self {
            Self::verify_and_align_legacy(canonical_manager, attacker, swing)?;
        }
        // CanonicalSelected was admitted by token/workset/witness. Its own
        // WorldObject is already authoritative; no proof between copies or
        // compatibility relocation is performed.
        Ok(())
    }

    fn verify_and_align_legacy(
        canonical_manager: &mut MapManager,
        attacker: &WorldCreature,
        swing: &PendingCreatureSwingLikeCpp,
    ) -> Result<(), SourceAdmission> {
        let canonical_attacker_is_same_incarnation = canonical_manager
            .find_map(u32::from(swing.map_id), swing.instance_id)
            .and_then(|managed| {
                managed
                    .map()
                    .with_creature_like_cpp(swing.attacker_guid, |canonical_attacker| {
                        canonical_attacker.spawn_id() == attacker.creature.spawn_id()
                            && canonical_attacker
                                .loot_authority_like_cpp()
                                .shares_storage_like_cpp(
                                    attacker.creature.loot_authority_like_cpp(),
                                )
                            && canonical_attacker
                                .unit()
                                .shares_health_state_revision_authority_like_cpp(
                                    &attacker
                                        .creature
                                        .unit()
                                        .health_state_revision_authority_like_cpp(),
                                )
                    })
            })
            .unwrap_or(false);
        if !canonical_attacker_is_same_incarnation {
            return Err(SourceAdmission::Incarnation);
        }
        // Movement snapshots normally publish this position before melee, but
        // the two global ticks may overlap between their legacy read and
        // canonical write phases. With both owners locked and the incarnation
        // proven equal, align the canonical WorldObject used by the LOS check to
        // the same live position used for range and facing.
        if let Some(managed) =
            canonical_manager.find_map_mut(u32::from(swing.map_id), swing.instance_id)
        {
            let _ = managed
                .map_mut()
                .relocate_map_object_like_cpp(swing.attacker_guid, swing.attacker_position);
        }
        Ok(())
    }

    pub(super) fn effects(
        &self,
        manager: &MapManager,
        catalogs: &impl CreatureMeleeCatalogsLikeCpp,
        difficulty: u8,
    ) -> Vec<AppliedAuraEffectLikeCpp> {
        let attacker = self.actor(manager);
        catalogs.creature_effects(
            &attacker.creature.unit().subsystems().auras.applied_auras,
            difficulty,
        )
    }

    pub(super) fn flags_extra(&self, manager: &MapManager) -> u32 {
        self.actor(manager)
            .creature
            .lifecycle_metadata()
            .flags_extra
    }

    pub(super) fn level(&self, manager: &MapManager) -> u8 {
        self.actor(manager).creature.level()
    }

    pub(super) fn is_player_controlled(&self, manager: &MapManager) -> bool {
        self.actor(manager)
            .creature
            .is_charmed_owned_by_player_or_player_like_cpp()
    }

    pub(super) fn base_attack_speed(&self, manager: &MapManager) -> u32 {
        self.actor(manager).creature.unit().base_attack_speed()[0]
    }

    pub(super) fn roll_damage(&mut self, manager: &mut MapManager) -> Option<u32> {
        self.actor_mut(manager).roll_damage()
    }

    pub(super) fn invalidate_rng(&mut self, manager: &mut MapManager) {
        self.actor_mut(manager)
            .invalidate_runtime_rng_authority_like_cpp();
    }

    pub(super) fn retry(&mut self, manager: &mut MapManager) {
        self.actor_mut(manager).record_failed_swing_retry_like_cpp();
    }

    pub(super) fn rearm(&mut self, manager: &mut MapManager) {
        self.actor_mut(manager).record_swing();
    }

    pub(super) fn finish_swing(&mut self, manager: &mut MapManager) -> usize {
        // Deliberately no late alive gate: preserve removal -> timer reset,
        // including after a successful primary commit against the same actor.
        let attacker = self.actor_mut(manager);
        let removed = attacker
            .creature
            .unit_mut()
            .remove_attacking_interrupt_auras_like_cpp();
        attacker.record_swing();
        removed
    }
}
