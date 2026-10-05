use crate::SessionSpellState;
const SPELL_AURA_EFFECT_IMMUNITY_LIKE_CPP: i64 = 37;
const SPELL_AURA_STATE_IMMUNITY_LIKE_CPP: i64 = 38;
const SPELL_AURA_MECHANIC_IMMUNITY_LIKE_CPP: i64 = 77;
const SPELL_AURA_MECHANIC_IMMUNITY_MASK_LIKE_CPP: i64 = 147;
const SPELL_AURA_MOD_IMMUNE_AURA_APPLY_SCHOOL_LIKE_CPP: i64 = 267;
const SPELL_EFFECT_ATTRIBUTE_NO_IMMUNITY_LIKE_CPP: i64 = 0x0000_0001;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrainerAuraRestrictionResultLikeCpp {
    Pass,
    Fail,
    Indeterminate,
}
fn trainer_aura_restriction_result_like_cpp(
    restriction: &wow_data::SpellAuraRestrictionsEntry,
    has_aura_spell: impl Fn(i32) -> bool,
) -> TrainerAuraRestrictionResultLikeCpp {
    // `TRIGGERED_FULL_MASK` does not contain the separate debug-only
    // `TRIGGERED_IGNORE_CASTER_AURASTATE` bit, so C++ still evaluates these
    // caster fields as well as the target fields. The session model does not
    // yet own Unit's complete AuraState mask; do not guess their result.
    if restriction.caster_aura_state != 0
        || restriction.target_aura_state != 0
        || restriction.exclude_caster_aura_state != 0
        || restriction.exclude_target_aura_state != 0
    {
        return TrainerAuraRestrictionResultLikeCpp::Indeterminate;
    }
    let required = [restriction.caster_aura_spell, restriction.target_aura_spell];
    if required
        .into_iter()
        .any(|spell_id| spell_id != 0 && !has_aura_spell(spell_id))
    {
        return TrainerAuraRestrictionResultLikeCpp::Fail;
    }
    let excluded = [
        restriction.exclude_caster_aura_spell,
        restriction.exclude_target_aura_spell,
    ];
    if excluded
        .into_iter()
        .any(|spell_id| spell_id != 0 && has_aura_spell(spell_id))
    {
        return TrainerAuraRestrictionResultLikeCpp::Fail;
    }
    TrainerAuraRestrictionResultLikeCpp::Pass
}
fn trainer_target_restriction_admits_player_like_cpp(
    store: &wow_data::SpellTargetRestrictionsStore,
    spell_id: u32,
    difficulty_chain: impl IntoIterator<Item = u32>,
) -> bool {
    const CREATURE_TYPEMASK_HUMANOID_LIKE_CPP: u32 = 1 << (7 - 1);
    store
        .resolved_for_difficulty_chain_like_cpp(spell_id, difficulty_chain)
        .is_none_or(|restriction| {
            let mask = restriction.target_creature_type_mask_like_cpp();
            if mask != 0 && mask & CREATURE_TYPEMASK_HUMANOID_LIKE_CPP == 0 {
                return false;
            }
            // `Trainer::TeachSpell` supplies the player as the object target.
            // Mirror `SpellInfo::CheckExplicitTarget` rather than treating the
            // DB2 mask as a type schema: with a non-null Unit target, C++ only
            // validates this special subset. Self passes ally/party/raid and
            // fails enemy/minipet/passenger unless one of the passing flags is
            // also present. Other flags fall through as SPELL_CAST_OK.
            const UNIT_RAID: u32 = 0x0000_0004;
            const UNIT_PARTY: u32 = 0x0000_0008;
            const UNIT_ENEMY: u32 = 0x0000_0080;
            const UNIT_ALLY: u32 = 0x0000_0100;
            const UNIT_MINIPET: u32 = 0x0001_0000;
            const UNIT_PASSENGER: u32 = 0x0010_0000;
            const SPECIAL_UNIT_FLAGS: u32 =
                UNIT_RAID | UNIT_PARTY | UNIT_ENEMY | UNIT_ALLY | UNIT_MINIPET | UNIT_PASSENGER;
            const SELF_ASSIST_FLAGS: u32 = UNIT_RAID | UNIT_PARTY | UNIT_ALLY;
            let explicit_targets = restriction.targets as u32;
            explicit_targets & SPECIAL_UNIT_FLAGS == 0 || explicit_targets & SELF_ASSIST_FLAGS != 0
        })
}
fn failed_trainer_cast_resolution_like_cpp(
    effective_effects: Vec<SpellAcquisitionEffectLikeCpp>,
) -> PlayerCastAcquisitionResolutionLikeCpp {
    PlayerCastAcquisitionResolutionLikeCpp {
        reached_immediate_phase: false,
        executed_hit_target_effect_mask: 0,
        effective_effects,
        executed_dual_wield_effects: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        TrainerAuraRestrictionResultLikeCpp, trainer_aura_restriction_result_like_cpp,
        trainer_target_restriction_admits_player_like_cpp,
    };
    use wow_data::{
        SpellAuraRestrictionsEntry, SpellTargetRestrictionsEntry, SpellTargetRestrictionsStore,
    };

    fn row(
        id: u32,
        difficulty_id: u8,
        target_creature_type: i16,
        targets: i32,
    ) -> SpellTargetRestrictionsEntry {
        SpellTargetRestrictionsEntry {
            id,
            difficulty_id,
            cone_degrees: 0.0,
            max_targets: 0,
            max_target_level: 0,
            target_creature_type,
            targets,
            width: 0.0,
            spell_id: 100,
        }
    }

    #[test]
    fn trainer_target_restriction_uses_active_row_without_merging_siblings_like_cpp() {
        let store = SpellTargetRestrictionsStore::from_entries([
            row(1, 0, 1 << (3 - 1), 0),
            row(2, 2, 1 << (7 - 1), 0),
        ]);

        assert!(trainer_target_restriction_admits_player_like_cpp(
            &store,
            100,
            [2, 0]
        ));
        assert!(!trainer_target_restriction_admits_player_like_cpp(
            &store,
            100,
            [0]
        ));
        assert!(trainer_target_restriction_admits_player_like_cpp(
            &store,
            200,
            [2, 0]
        ));
    }

    #[test]
    fn trainer_target_restriction_mirrors_cpp_self_explicit_target_checks() {
        let item = SpellTargetRestrictionsStore::from_entries([row(
            1,
            0,
            1 << (7 - 1),
            0x0000_0010, // TARGET_FLAG_ITEM
        )]);
        let player_and_destination = SpellTargetRestrictionsStore::from_entries([row(
            2,
            0,
            1 << (7 - 1),
            0x0000_0002 | 0x0000_0040,
        )]);
        let enemy = SpellTargetRestrictionsStore::from_entries([row(
            3,
            0,
            1 << (7 - 1),
            0x0000_0080, // TARGET_FLAG_UNIT_ENEMY
        )]);

        // C++ already has a non-null Unit object target here, so item-only
        // falls through `CheckExplicitTarget` successfully.
        assert!(trainer_target_restriction_admits_player_like_cpp(
            &item,
            100,
            [0]
        ));
        assert!(trainer_target_restriction_admits_player_like_cpp(
            &player_and_destination,
            100,
            [0]
        ));
        assert!(!trainer_target_restriction_admits_player_like_cpp(
            &enemy,
            100,
            [0]
        ));
    }

    #[test]
    fn trainer_aura_spell_restrictions_distinguish_pass_fail_and_unknown_state_like_cpp() {
        let mut restriction = SpellAuraRestrictionsEntry {
            id: 1,
            difficulty_id: 0,
            caster_aura_state: 0,
            target_aura_state: 0,
            exclude_caster_aura_state: 0,
            exclude_target_aura_state: 0,
            caster_aura_spell: 999,
            target_aura_spell: 0,
            exclude_caster_aura_spell: 0,
            exclude_target_aura_spell: 0,
            spell_id: 100,
        };

        assert_eq!(
            trainer_aura_restriction_result_like_cpp(&restriction, |spell_id| spell_id == 999),
            TrainerAuraRestrictionResultLikeCpp::Pass
        );
        assert_eq!(
            trainer_aura_restriction_result_like_cpp(&restriction, |_| false),
            TrainerAuraRestrictionResultLikeCpp::Fail
        );

        restriction.caster_aura_state = 7;
        assert_eq!(
            trainer_aura_restriction_result_like_cpp(&restriction, |spell_id| spell_id == 999),
            TrainerAuraRestrictionResultLikeCpp::Indeterminate
        );
    }
}
use std::collections::BTreeSet;
use wow_data::SpellLinkedTypeLikeCpp;
use wow_data::spell::spell_effect_types::{
    SPELL_EFFECT_DUAL_WIELD, SPELL_EFFECT_LEARN_SPELL, SPELL_EFFECT_SKILL, SPELL_EFFECT_SKILL_STEP,
};
use wow_data::{
    SpellAcquisitionCatalogLikeCpp, SpellAcquisitionEffectLikeCpp,
    SpellAcquisitionResolvedEffectsLookupLikeCpp, SpellAcquisitionResolvedMetadataLookupLikeCpp,
};
use wow_spell_acquisition::{
    PlayerCastAcquisitionResolutionLikeCpp, PlayerExecutedDualWieldEffectLikeCpp,
};
impl crate::SessionSpellState {
    /// Resolve the bounded normal-trainer wrapper against current represented
    /// player state. C++ checks effect immunity per target in `AddUnitTarget`;
    /// ordinary buffs do not suppress unrelated trainer effects. Until the
    /// canonical Unit immunity containers exist, derive the bounded blockers
    /// from complete active-aura effects and negative `SPELL_LINK_AURA` rows.
    /// Missing aura/link authority still fails closed.
    pub fn resolve_trainer_wrapper_cast_acquisition_like_cpp(
        &self,
        hub: wow_world_core::session::HubRef<'_>,
        spell_id: u32,
    ) -> Option<PlayerCastAcquisitionResolutionLikeCpp> {
        self.resolve_trainer_wrapper_cast_acquisition_with_access_like_cpp(
            &hub.core.owned_spell_acquisition_access_like_cpp(),
            &hub.player_condition_access_like_cpp(),
            hub.catalogs
                .spell_catalogs
                .spell_acquisition_catalog()
                .map(AsRef::as_ref),
            hub.catalogs.difficulty_store().map(AsRef::as_ref),
            hub.catalogs.map_store().map(AsRef::as_ref),
            hub.catalogs.disable_mgr().map(AsRef::as_ref),
            hub.catalogs
                .spell_catalogs
                .spell_target_restrictions_store()
                .map(AsRef::as_ref),
            hub.catalogs
                .spell_catalogs
                .spell_aura_restrictions_store()
                .map(AsRef::as_ref),
            hub.catalogs.spell_pet_aura_store_like_cpp(),
            hub.catalogs.spell_catalogs.spell_linked_store_like_cpp(),
            spell_id,
        )
    }

    /// Resolve one wrapper using borrowed, selected canonical inputs. Store
    /// selection performs no player query; each query stays at its cast phase.
    pub fn resolve_trainer_wrapper_cast_acquisition_with_access_like_cpp(
        &self,
        spell: &wow_world_core::session::OwnedSpellAcquisitionAccessLikeCpp<'_>,
        player: &wow_world_core::session::PlayerConditionAccessLikeCpp<'_>,
        catalog: Option<&SpellAcquisitionCatalogLikeCpp>,
        difficulties: Option<&wow_data::DifficultyStore>,
        maps: Option<&wow_data::MapStore>,
        disables: Option<&wow_data::DisableMgrLikeCpp>,
        targets: Option<&wow_data::SpellTargetRestrictionsStore>,
        restrictions: Option<&wow_data::SpellAuraRestrictionsStore>,
        pet_auras: Option<&wow_data::SpellPetAuraStoreLikeCpp>,
        linked: Option<&wow_data::SpellLinkedStoreLikeCpp>,
        spell_id: u32,
    ) -> Option<PlayerCastAcquisitionResolutionLikeCpp> {
        let catalog = catalog?;
        let difficulty_chain = self.difficulty_chain_for_acquisition_like_cpp(
            difficulties,
            u32::from(spell.current_map_difficulty_id_like_cpp()),
        );
        let effective_effects = match catalog.resolved_effects_for_difficulty_chain_like_cpp(
            spell_id,
            difficulty_chain.iter().copied(),
        ) {
            SpellAcquisitionResolvedEffectsLookupLikeCpp::Covered(effects) => {
                effects.into_iter().cloned().collect::<Vec<_>>()
            }
            SpellAcquisitionResolvedEffectsLookupLikeCpp::MissingCoverage { .. }
            | SpellAcquisitionResolvedEffectsLookupLikeCpp::Indeterminate(_) => return None,
        };
        let map_id = u32::from(spell.player_map_id_like_cpp());
        let (_, area_id) = player.player_zone_area_like_cpp()?;
        let map_instance_type = maps
            .and_then(|store| store.get(map_id))
            .map(|entry| entry.instance_type);
        if disables?.is_disabled_for_like_cpp(
            wow_data::DISABLE_TYPE_SPELL,
            spell_id,
            Some(wow_data::DisableWorldObjectRefLikeCpp {
                // The C++ trainer path casts from the player, not the NPC.
                type_id: wow_constants::TypeId::Player,
                map_id,
                area_id,
                is_pet: false,
                is_battle_arena: map_instance_type == Some(wow_data::MAP_ARENA_LIKE_CPP),
                is_battleground: map_instance_type == Some(wow_data::MAP_BATTLEGROUND_LIKE_CPP),
                player_map_difficulty: None,
            }),
            0,
            maps,
        ) {
            // Spell::prepare rejects disabled spells before CheckCast and any
            // effect/pet hook. Trainer::TeachSpell has already charged and
            // emitted both visuals, so this is a resolved no-effect cast even
            // when later metadata is unsupported by the reduced projection.
            return Some(failed_trainer_cast_resolution_like_cpp(effective_effects));
        }
        // C++ copies this single SpellInfo field from the first row in the
        // active difficulty/fallback chain, then checks the player target's
        // HUMANOID mask. Do not combine sibling difficulty rows: a heroic
        // restriction cannot reject a normal cast (or vice versa).
        if !trainer_target_restriction_admits_player_like_cpp(
            targets?,
            spell_id,
            difficulty_chain.iter().copied(),
        ) {
            // C++ charges and publishes both trainer visuals before the
            // triggered cast reaches CheckExplicitTarget/CheckTarget.
            return Some(failed_trainer_cast_resolution_like_cpp(effective_effects));
        }
        // An empty aura map proves absence only after both persisted aura
        // tables completed successfully during login. This authority is also
        // required before resolving positive/negative aura-spell gates.
        if player.resolved_player_aura_authority_complete_like_cpp() != Some(true) {
            return None;
        }
        // Startup proves only DIFFICULTY_NONE. Resolve the active row against
        // the current self-target where the session owns exact aura-spell
        // presence. A definite cast failure happens after fee/visuals in C++;
        // state-based rows remain unavailable until Unit AuraState is owned.
        let visible_auras = player.resolved_visible_auras_like_cpp()?;
        let aura_restriction_result = restrictions?
            .resolved_for_difficulty_chain_like_cpp(spell_id, difficulty_chain.iter().copied())
            .map_or(TrainerAuraRestrictionResultLikeCpp::Pass, |restriction| {
                trainer_aura_restriction_result_like_cpp(restriction, |required_spell_id| {
                    visible_auras
                        .values()
                        .any(|aura| aura.spell_id == required_spell_id)
                })
            });
        match aura_restriction_result {
            TrainerAuraRestrictionResultLikeCpp::Pass => {}
            TrainerAuraRestrictionResultLikeCpp::Fail => {
                return Some(failed_trainer_cast_resolution_like_cpp(effective_effects));
            }
            TrainerAuraRestrictionResultLikeCpp::Indeterminate => return None,
        }
        // Re-audit the active variant because startup's immutable authority
        // proves the difficulty-none closure. C++ reaches these effect and
        // pet-aura hooks only after CheckCast has accepted the target/aura
        // gates above, so a definite pre-effect cast failure must win over an
        // unsupported hook and preserve the paid, visualized no-effect cast.
        let pet_auras = pet_auras?;
        for effect in &effective_effects {
            let effect_index = effect.effect_index_checked().ok()?;
            if pet_auras
                .get_pet_aura_like_cpp(spell_id, effect_index)
                .is_some()
            {
                return None;
            }
            let effect_type = effect.effect_type_checked().ok()?;
            match effect_type {
                SPELL_EFFECT_LEARN_SPELL | SPELL_EFFECT_SKILL_STEP | SPELL_EFFECT_DUAL_WIELD => {
                    if effect.effect_mechanic_raw != 0
                        || effect.effect_aura_raw != 0
                        || !effect.targets_player_like_cpp()
                    {
                        return None;
                    }
                }
                // C++ `Trainer::TeachSpell` invokes castable wrappers as
                // `player->CastSpell(player, ...)`. EffectSkill is HANDLE_HIT,
                // has no implicit target, and mutates that player caster.
                SPELL_EFFECT_SKILL => {
                    if effect.effect_mechanic_raw != 0 || effect.effect_aura_raw != 0 {
                        return None;
                    }
                }
                0 => {}
                3 if matches!(spell_id, 33_388 | 34_090) => {}
                other if wow_data::spell::spell_effect_types::is_cpp_null_or_unused_noop(other) => {
                }
                _ => return None,
            }
        }
        let (no_immunities, is_channeled) = match catalog
            .resolved_misc_for_difficulty_chain_like_cpp(spell_id, difficulty_chain.iter().copied())
        {
            SpellAcquisitionResolvedMetadataLookupLikeCpp::Present(misc) => (
                misc.no_immunities_checked().ok()?,
                misc.is_channeled_checked().ok()?,
            ),
            SpellAcquisitionResolvedMetadataLookupLikeCpp::CoveredWithoutRow => (false, false),
            SpellAcquisitionResolvedMetadataLookupLikeCpp::MissingCoverage { .. }
            | SpellAcquisitionResolvedMetadataLookupLikeCpp::Indeterminate(_) => return None,
        };
        // C++ enters the channel lifecycle even for this triggered player
        // cast. The reduced trainer projection cannot reproduce channel state
        // or updates, so active-difficulty channel wrappers fail closed.
        if is_channeled {
            return None;
        }
        let immunized_effect_mask = self.active_auras_immunized_trainer_effect_mask_like_cpp(
            player,
            difficulties,
            linked,
            catalog,
            spell_id,
            &effective_effects,
            no_immunities,
        )?;
        let mut executed_hit_target_effect_mask = 0_u32;
        let mut executed_dual_wield_effects = Vec::new();
        for effect in &effective_effects {
            let effect_type = effect.effect_type_checked().ok()?;
            if !matches!(
                effect_type,
                SPELL_EFFECT_LEARN_SPELL
                    | SPELL_EFFECT_SKILL_STEP
                    | SPELL_EFFECT_SKILL
                    | SPELL_EFFECT_DUAL_WIELD
            ) {
                continue;
            }
            let effect_index = effect.effect_index_checked().ok()?;
            let effect_bit = 1_u32.checked_shl(u32::from(effect_index))?;
            if immunized_effect_mask & effect_bit != 0 {
                continue;
            }
            executed_hit_target_effect_mask |= effect_bit;
            if effect_type == SPELL_EFFECT_DUAL_WIELD {
                executed_dual_wield_effects.push(PlayerExecutedDualWieldEffectLikeCpp {
                    effect_record_id: effect.record_id,
                    effect_index,
                });
            }
        }
        Some(PlayerCastAcquisitionResolutionLikeCpp {
            reached_immediate_phase: true,
            executed_hit_target_effect_mask,
            effective_effects,
            executed_dual_wield_effects,
        })
    }
    /// Build the C++ `SpellMgr::GetSpellInfo` fallback chain for a selected
    /// difficulty. Casts use the current map difficulty; retained auras use
    /// the difficulty whose `SpellInfo` was selected when they were created.
    fn difficulty_chain_for_acquisition_like_cpp(
        &self,
        difficulties: Option<&wow_data::DifficultyStore>,
        requested: u32,
    ) -> Vec<u32> {
        let mut chain = vec![requested];
        let mut visited = BTreeSet::from([requested]);
        let mut current = requested;
        while let Some(difficulty) = difficulties.and_then(|store| store.get(current)) {
            let fallback = u32::from(difficulty.fallback_difficulty_id);
            if !visited.insert(fallback) {
                break;
            }
            chain.push(fallback);
            current = fallback;
        }
        chain
    }
    fn active_auras_immunized_trainer_effect_mask_like_cpp(
        &self,
        player: &wow_world_core::session::PlayerConditionAccessLikeCpp<'_>,
        difficulties: Option<&wow_data::DifficultyStore>,
        linked: Option<&wow_data::SpellLinkedStoreLikeCpp>,
        catalog: &SpellAcquisitionCatalogLikeCpp,
        trainer_spell_id: u32,
        trainer_effects: &[SpellAcquisitionEffectLikeCpp],
        no_immunities: bool,
    ) -> Option<u32> {
        let linked = linked?;
        let visible_auras = player.resolved_visible_auras_like_cpp()?;
        let mut immunized_effect_mask = 0_u32;
        for aura in visible_auras.values() {
            let aura_spell_id = u32::try_from(aura.spell_id).ok().filter(|id| *id != 0)?;
            if linked
                .get_spell_linked_like_cpp(SpellLinkedTypeLikeCpp::Aura, aura_spell_id)
                .is_some_and(|effects| {
                    effects
                        .iter()
                        .any(|effect| *effect < 0 && effect.unsigned_abs() == trainer_spell_id)
                })
            {
                // C++ `IMMUNITY_ID` rejects the spell rather than one effect.
                return Some(u32::MAX);
            }
            if no_immunities {
                // C++ checks IMMUNITY_ID before SPELL_ATTR0_NO_IMMUNITIES,
                // then bypasses all remaining spell/effect immunities.
                continue;
            }
            let aura_difficulty_chain = self.difficulty_chain_for_acquisition_like_cpp(
                difficulties,
                u32::from(aura.difficulty_id),
            );
            let effects = match catalog.resolved_effects_for_difficulty_chain_like_cpp(
                aura_spell_id,
                aura_difficulty_chain,
            ) {
                SpellAcquisitionResolvedEffectsLookupLikeCpp::Covered(effects) => effects,
                SpellAcquisitionResolvedEffectsLookupLikeCpp::MissingCoverage { .. }
                | SpellAcquisitionResolvedEffectsLookupLikeCpp::Indeterminate(_) => return None,
            };
            let mut unresolved_effect_mask = aura.effect_mask;
            for effect in effects {
                let effect_index = effect.effect_index_checked().ok()?;
                let effect_bit = 1_u32.checked_shl(u32::from(effect_index))?;
                if unresolved_effect_mask & effect_bit == 0 {
                    continue;
                }
                unresolved_effect_mask &= !effect_bit;
                match effect.effect_aura_raw {
                    SPELL_AURA_EFFECT_IMMUNITY_LIKE_CPP => {
                        for trainer_effect in trainer_effects {
                            let trainer_effect_type = trainer_effect.effect_type_checked().ok()?;
                            if trainer_effect.effect_attributes_raw
                                & SPELL_EFFECT_ATTRIBUTE_NO_IMMUNITY_LIKE_CPP
                                == 0
                                && effect.effect_misc_value_raw[0] == i64::from(trainer_effect_type)
                            {
                                let trainer_effect_index =
                                    trainer_effect.effect_index_checked().ok()?;
                                immunized_effect_mask |=
                                    1_u32.checked_shl(u32::from(trainer_effect_index))?;
                            }
                        }
                    }
                    SPELL_AURA_STATE_IMMUNITY_LIKE_CPP => {
                        for trainer_effect in trainer_effects {
                            if trainer_effect.effect_aura_raw != 0
                                && effect.effect_misc_value_raw[0] == trainer_effect.effect_aura_raw
                            {
                                let trainer_effect_index =
                                    trainer_effect.effect_index_checked().ok()?;
                                immunized_effect_mask |=
                                    1_u32.checked_shl(u32::from(trainer_effect_index))?;
                            }
                        }
                    }
                    SPELL_AURA_MECHANIC_IMMUNITY_LIKE_CPP
                    | SPELL_AURA_MECHANIC_IMMUNITY_MASK_LIKE_CPP
                        if trainer_effects
                            .iter()
                            .any(|trainer_effect| trainer_effect.effect_mechanic_raw != 0) =>
                    {
                        // Startup authority excludes this shape. If a stale or
                        // injected catalog violates that invariant, do not
                        // approximate C++'s mechanic-mask switch table.
                        return None;
                    }
                    SPELL_AURA_MOD_IMMUNE_AURA_APPLY_SCHOOL_LIKE_CPP
                        if trainer_effects
                            .iter()
                            .any(|trainer_effect| trainer_effect.effect_aura_raw != 0) =>
                    {
                        return None;
                    }
                    _ => {}
                }
            }
            if unresolved_effect_mask != 0 {
                return None;
            }
        }
        Some(immunized_effect_mask)
    }
}
