// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn creature_display_power_for_class_like_cpp(&self, unit_class: u8) -> u8 {
        self.catalogs
            .creature_display_power_for_class_like_cpp(unit_class)
    }
    pub(crate) fn mutate_canonical_creature_by_guid_like_cpp<R>(
        &mut self,
        guid: ObjectGuid,
        f: impl FnOnce(&mut wow_entities::Creature) -> R,
    ) -> Option<R> {
        self.core
            .mutate_canonical_creature_by_guid_like_cpp(guid, f)
    }
}

impl WorldSession {
    pub(crate) fn set_canonical_creature_private_object_owner_like_cpp(
        &mut self,
        guid: ObjectGuid,
        owner: ObjectGuid,
    ) -> bool {
        let updated = self
            .core
            .mutate_canonical_creature_by_guid_like_cpp(guid, |creature| {
                creature.unit_mut().set_private_object_owner_like_cpp(owner);
            })
            .is_some();

        updated
    }
    pub(crate) async fn talked_to_creature_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_entry: u32,
        creature_guid: wow_core::ObjectGuid,
    ) {
        // C++ QUEST_OBJECTIVE_TALKTO.
        self.quest_objective_operation_like_cpp(
            item_guid_generator,
            crate::session::QuestObjectiveOperationLikeCpp::StoringValue {
                objective_type: 3,
                object_id: creature_entry as i32,
                add_count: 1,
                credit_guid: creature_guid,
            },
        )
        .await;
    }
    /// C++ `Spell::EffectApplyAura` for a creature target: create the canonical
    /// `AuraApplication` with its represented effect data, misc values and
    /// duration, and publish `SMSG_AURA_UPDATE` to the caster and the
    /// creature's observers.
    ///
    /// Boundaries of this slice: the represented model has no stack/refresh
    /// rule (a second application of the same spell by the same caster is
    /// refused, like the spawn-addon path), the duration comes from the
    /// caller's represented value rather than `SpellDuration.db2`, and the
    /// periodic effect amount is registered but not yet ticked.
    pub(crate) fn apply_creature_aura_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        effect_mask: u32,
        duration_ms: u32,
    ) -> Result<(), &'static str> {
        self.apply_creature_aura_with_provenance_like_cpp(
            spell_id,
            caster_guid,
            target_guid,
            effect_mask,
            duration_ms,
            wow_entities::AuraCastProvenanceLikeCpp::default(),
        )
    }
}
