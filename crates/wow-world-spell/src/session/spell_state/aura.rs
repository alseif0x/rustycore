use crate::SessionSpellState;
use wow_entities::AuraApplicationLikeCpp as AuraApplication;
use wow_world_core::session::{HubMut, HubRef};

// The `aura` submodules live beside this file in `aura/`; the stale
// `aura/mod.rs` stub that shadowed them was a leftover of the incomplete
// module move and is removed.
mod effect_queries;
mod spell_hit_authority;

fn attack_speed_multipliers_like_cpp(
    mut query: impl FnMut(i32) -> Option<Vec<(i32, i32)>>,
) -> [f32; 3] {
    use wow_data::spell::aura_types::{
        SPELL_AURA_MELEE_SLOW, SPELL_AURA_MOD_ATTACKSPEED, SPELL_AURA_MOD_MELEE_HASTE,
        SPELL_AURA_MOD_MELEE_HASTE_2, SPELL_AURA_MOD_MELEE_HASTE_3,
        SPELL_AURA_MOD_MELEE_RANGED_HASTE, SPELL_AURA_MOD_MELEE_RANGED_HASTE_2,
        SPELL_AURA_MOD_RANGED_HASTE, SPELL_AURA_MOD_SPEED_SLOW_ALL,
    };

    let mut multipliers = [1.0_f32; 3];
    let mut apply = |aura_type: i32, attacks: &[usize]| {
        for (_, amount) in query(aura_type).unwrap_or_default() {
            let amount = amount as f32;
            let factor = if amount > 0.0 {
                100.0 / (100.0 + amount)
            } else {
                (100.0 - amount) / 100.0
            };
            for attack in attacks {
                if let Some(slot) = multipliers.get_mut(*attack) {
                    *slot *= factor;
                }
            }
        }
    };
    apply(SPELL_AURA_MOD_ATTACKSPEED, &[0]);
    apply(SPELL_AURA_MOD_MELEE_HASTE, &[0, 1]);
    apply(SPELL_AURA_MOD_MELEE_HASTE_2, &[0, 1]);
    apply(SPELL_AURA_MOD_MELEE_HASTE_3, &[0, 1]);
    apply(SPELL_AURA_MOD_RANGED_HASTE, &[2]);
    apply(SPELL_AURA_MOD_MELEE_RANGED_HASTE, &[0, 1, 2]);
    apply(SPELL_AURA_MOD_MELEE_RANGED_HASTE_2, &[0, 1, 2]);
    apply(SPELL_AURA_MELEE_SLOW, &[0, 1, 2]);
    apply(SPELL_AURA_MOD_SPEED_SLOW_ALL, &[0, 1, 2]);
    multipliers
}

/// C++ `SPELLFAMILY_MAGE` (`SharedDefines.h:6438`).
const SPELL_FAMILY_MAGE_LIKE_CPP: i32 = 3;

impl SessionSpellState {
    pub fn sync_shapeshift_form_ownership_with_access_like_cpp(
        &self,
        player: &wow_world_core::session::PlayerAuraRemovalAccessLikeCpp<'_>,
        spell_store: Option<&std::sync::Arc<wow_data::SpellStore>>,
        mutated_spell_id: i32,
        consumer_test: bool,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_form: &mut u32,
    ) -> Option<RepresentedShapeshiftMutationLikeCpp> {
        let Some(store) = spell_store.cloned() else {
            return None;
        };
        if !self.represented_spell_has_mod_shapeshift_effect_with_store_like_cpp(
            spell_store.map(|store| store.as_ref()),
            mutated_spell_id,
        ) {
            return None;
        }
        let mutated_form = store
            .get(mutated_spell_id)
            .and_then(shapeshift_form_of_spell_like_cpp);
        if player.player_has_visible_aura_spell_like_cpp(mutated_spell_id) == Some(true) {
            let form_id = mutated_form?;
            player.set_shapeshift_form_like_cpp(
                form_id,
                consumer_test,
                #[cfg(any(test, feature = "test-fixtures"))]
                fixture_form,
            );
            return Some(RepresentedShapeshiftMutationLikeCpp::Applied { form_id });
        }
        let remaining_form = player
            .visible_auras_snapshot_like_cpp()
            .unwrap_or_default()
            .into_values()
            .find_map(|aura| shapeshift_form_of_spell_like_cpp(store.get(aura.spell_id)?));
        let new_form = remaining_form.unwrap_or(0);
        player.set_shapeshift_form_like_cpp(
            new_form,
            consumer_test,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_form,
        );
        Some(RepresentedShapeshiftMutationLikeCpp::Removed {
            removed_form: mutated_form.unwrap_or(0),
            new_form,
        })
    }
    /// C++ `Unit::HasAuraState(flag)` for the represented Caster: the union of
    /// the unit's aura-driven state bits and its health-derived bits.
    pub fn represented_has_aura_state_like_cpp(&self, hub: HubRef<'_>, aura_state: u32) -> bool {
        let Some(flag) = u8::try_from(aura_state).ok().filter(|flag| *flag != 0) else {
            return false;
        };
        let Some(mask) = self.represented_player_aura_state_mask_like_cpp(hub) else {
            return false;
        };
        u32::from(flag)
            .checked_sub(1)
            .and_then(|bit| 1_u32.checked_shl(bit))
            .is_some_and(|bit| mask & bit != 0)
    }

    /// C++ `Unit::m_unitData->AuraState` for the canonical session player: the
    /// aura-driven bits owned by the represented aura subsystem plus the
    /// alive-health bits `Unit::Update` maintains (WOUNDED_* / HEALTHY_75).
    ///
    /// `None` when the canonical Player owner is unavailable, so callers fail
    /// closed instead of reading an empty mask as an authoritative zero.
    pub fn represented_player_aura_state_mask_like_cpp(&self, hub: HubRef<'_>) -> Option<u32> {
        let aura_driven = hub.core.canonical_player_snapshot_like_cpp(|player| {
            player.unit().subsystems().auras.aura_state_mask
        })?;
        let (health, max_health, alive) = hub.resolved_player_vitals_like_cpp()?;
        Some(
            aura_driven
                | wow_world_core::map_manager::WorldCreature::health_aura_state_like_cpp(
                    u64::from(health),
                    u64::from(max_health),
                    alive,
                ),
        )
    }

    /// Whether any effect of the spell applies `SPELL_AURA_MOD_SHAPESHIFT`, the
    /// gate for the C++ form-change recalculation.
    pub fn represented_spell_has_mod_shapeshift_effect_like_cpp(
        &self,
        hub: HubRef<'_>,
        spell_id: i32,
    ) -> bool {
        self.represented_spell_has_mod_shapeshift_effect_with_store_like_cpp(
            hub.catalogs.spell_store().map(|store| store.as_ref()),
            spell_id,
        )
    }

    pub fn represented_spell_has_mod_shapeshift_effect_with_store_like_cpp(
        &self,
        spell_store: Option<&wow_data::SpellStore>,
        spell_id: i32,
    ) -> bool {
        spell_store
            .and_then(|store| store.get(spell_id))
            .is_some_and(|spell| {
                spell
                    .effects()
                    .iter()
                    .any(wow_data::SpellEffectInfo::is_mod_shapeshift_aura_like_cpp)
            })
    }

    /// C++ `AuraEffect::HandleModAttackSpeed`/`HandleModMeleeSpeedPct`/
    /// `HandleModCombatSpeedPct`/`HandleAuraModRangedHaste`
    /// (`SpellAuraEffects.cpp:4353-4393`): the per-attack `m_modAttackSpeedPct`
    /// product over the player's active attack-speed auras.
    ///
    /// `Unit::ApplyAttackTimePercentMod` converts a positive amount with
    /// `100 / (100 + amount)` and a negative amount with `(100 - amount) / 100`,
    /// which this reproduces. Boundary: the C++
    /// `GetHighestExclusiveSameEffectSpellGroupValue` de-duplication of
    /// `SPELL_AURA_MOD_MELEE_HASTE`/`MELEE_SLOW` needs the spell-group tables
    /// that the represented session does not load yet, so every active effect is
    /// multiplied here.
    pub fn represented_attack_speed_multipliers_like_cpp(&self, hub: HubRef<'_>) -> [f32; 3] {
        attack_speed_multipliers_like_cpp(|aura_type| {
            hub.resolved_aura_effects_by_spell_aura_type_like_cpp(aura_type)
        })
    }

    pub fn represented_attack_speed_multipliers_with_access_like_cpp(
        &self,
        player: &wow_world_core::session::PlayerAuraRemovalAccessLikeCpp<'_>,
        spell_store: Option<&wow_data::SpellStore>,
    ) -> [f32; 3] {
        attack_speed_multipliers_like_cpp(|aura_type| {
            player.aura_effects_by_spell_aura_type_like_cpp(spell_store, aura_type)
        })
    }

    pub fn sync_attack_speed_with_access_like_cpp(
        &self,
        player: &wow_world_core::session::PlayerAuraRemovalAccessLikeCpp<'_>,
        spell_store: Option<&wow_data::SpellStore>,
    ) {
        let multipliers =
            self.represented_attack_speed_multipliers_with_access_like_cpp(player, spell_store);
        let autoattack = player
            .autoattack_damage_multiplier_like_cpp(spell_store)
            .max(0.0);
        player.apply_attack_speed_multipliers_like_cpp(multipliers, autoattack);
    }

    /// Re-install the represented attack-time multipliers on the canonical
    /// Player after any aura mutation, mirroring the C++ aura handlers that call
    /// `Unit::ApplyAttackTimePercentMod` at apply/remove time.
    /// C++ `Unit::ApplyCastTimePercentMod` (`Unit.cpp:10229-10252`), reached from
    /// `AuraEffect::HandleModCastingSpeed` (`SpellAuraEffects.cpp:4272-4315`)
    /// and `HandleModCombatSpeedPct` (`4330-4351`): the caster's cast-time
    /// multiplier over `SPELL_AURA_MOD_CASTING_SPEED_NOT_STACK` (65),
    /// `SPELL_AURA_HASTE_SPELLS` (216), `SPELL_AURA_MELEE_SLOW` (193) and
    /// `SPELL_AURA_MOD_SPEED_SLOW_ALL` (252).
    ///
    /// A total of `1000` or more is C++'s `SetInstantCast(true)`, represented as
    /// a zero multiplier. Boundary: the spell-group de-duplication and the
    /// `ModHasteRegen` cooldown-recovery consumer remain unrepresented.
    pub fn represented_cast_speed_multiplier_like_cpp(&self, hub: HubRef<'_>) -> f32 {
        use wow_data::spell::aura_types::{
            SPELL_AURA_HASTE_SPELLS, SPELL_AURA_MELEE_SLOW, SPELL_AURA_MOD_CASTING_SPEED_NOT_STACK,
            SPELL_AURA_MOD_SPEED_SLOW_ALL,
        };

        let mut multiplier = 1.0_f32;
        for aura_type in [
            SPELL_AURA_MOD_CASTING_SPEED_NOT_STACK,
            SPELL_AURA_HASTE_SPELLS,
            SPELL_AURA_MELEE_SLOW,
            SPELL_AURA_MOD_SPEED_SLOW_ALL,
        ] {
            for (_, amount) in hub
                .resolved_aura_effects_by_spell_aura_type_like_cpp(aura_type)
                .unwrap_or_default()
            {
                if amount >= 1000 {
                    return 0.0;
                }
                let amount = amount as f32;
                multiplier *= if amount > 0.0 {
                    100.0 / (100.0 + amount)
                } else {
                    (100.0 - amount) / 100.0
                };
            }
        }
        multiplier.max(0.0)
    }

    /// Whether the represented application carries an active
    /// `SPELL_AURA_TRANSFORM` effect, the trigger C++ routes to
    /// `AuraEffect::HandleAuraTransform`.
    fn represented_application_has_transform_effect_like_cpp(
        &self,
        spell_store: Option<&wow_data::SpellStore>,
        aura: &AuraApplication,
    ) -> bool {
        spell_store.is_some_and(|store| {
            store.get(aura.spell_id).is_some_and(|spell| {
                spell.effects().iter().any(|effect| {
                    1u32.checked_shl(effect.effect_index)
                        .is_some_and(|bit| aura.effect_mask & bit != 0)
                        && effect.is_aura_like_cpp()
                        && effect.effect_aura == wow_data::spell::aura_types::SPELL_AURA_TRANSFORM
                })
            })
        })
    }

    /// C++ `AuraEffect::HandleAuraTransform` apply path
    /// (`SpellAuraEffects.cpp:1935-1951`) for the canonical Player: the applied
    /// transform aura updates `Unit::m_transformSpell` when there is no current
    /// transform spell info, when the new spell is not positive, or when the
    /// current transform spell is positive. Returns true when the application
    /// carried a transform effect and the state was updated.
    pub fn apply_represented_transform_aura_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        aura: &AuraApplication,
    ) -> bool {
        let spell_store = hub.catalogs.spell_store().map(AsRef::as_ref);
        self.apply_represented_transform_aura_with_access_like_cpp(
            &mut hub.player_aura_removal_access_like_cpp(),
            spell_store,
            aura,
        )
    }

    pub fn apply_represented_transform_aura_with_access_like_cpp(
        &mut self,
        player: &mut wow_world_core::session::PlayerAuraRemovalAccessLikeCpp<'_>,
        spell_store: Option<&wow_data::SpellStore>,
        aura: &AuraApplication,
    ) -> bool {
        if !self.represented_application_has_transform_effect_like_cpp(spell_store, aura) {
            return false;
        }
        let (new_is_positive, current_is_positive) = {
            let Some(store) = spell_store else {
                return false;
            };
            let Some(new_spell) = store.get(aura.spell_id) else {
                return false;
            };
            let current = player
                .aura_subsystem_snapshot_like_cpp()
                .map_or(0, |auras| auras.transform_spell_like_cpp());
            // C++ resolves the current transform through
            // `sSpellMgr->GetSpellInfo(GetTransformSpell(), difficulty)`, so a
            // zero or unloaded current spell is the `!transformSpellInfo`
            // branch that always overwrites.
            let current_is_positive = (current != 0)
                .then(|| store.get(current))
                .flatten()
                .map(wow_data::represented_spell_is_positive_like_cpp);
            (
                wow_data::represented_spell_is_positive_like_cpp(new_spell),
                current_is_positive,
            )
        };
        let spell_id = aura.spell_id;
        let mutated = player.apply_transform_canonical_like_cpp(
            spell_id,
            new_is_positive,
            current_is_positive,
        );
        #[cfg(any(test, feature = "test-fixtures"))]
        if !mutated && player.player_handle_absent_like_cpp() {
            // Handle-less fixtures have no canonical Player to carry the derived
            // transform field, so it only lives for this mutation. Acceptance
            // cases that assert `IsPolymorphed` install a canonical Player
            // owner, which is the production path.
            let _ = player.mutate_player_aura_subsystem_like_cpp(|auras| {
                auras.apply_transform_aura_like_cpp(spell_id, new_is_positive, current_is_positive);
            });
        }
        true
    }

    /// C++ `AuraEffect::HandleAuraTransform` remove path
    /// (`SpellAuraEffects.cpp:2129-2131`): only the aura that owns the current
    /// transform spell clears it.
    pub fn remove_represented_transform_aura_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        aura: &AuraApplication,
    ) -> bool {
        self.remove_represented_transform_aura_with_access_like_cpp(
            &mut hub.player_aura_removal_access_like_cpp(),
            aura,
        )
    }

    pub fn remove_represented_transform_aura_with_access_like_cpp(
        &mut self,
        player: &mut wow_world_core::session::PlayerAuraRemovalAccessLikeCpp<'_>,
        aura: &AuraApplication,
    ) -> bool {
        let spell_id = aura.spell_id;
        let owns_transform = player
            .aura_subsystem_snapshot_like_cpp()
            .is_some_and(|auras| auras.transform_spell_like_cpp() == spell_id);
        if !owns_transform {
            return false;
        }
        let mutated = player.remove_transform_canonical_like_cpp(spell_id);
        #[cfg(any(test, feature = "test-fixtures"))]
        if !mutated && player.player_handle_absent_like_cpp() {
            let _ = player.mutate_player_aura_subsystem_like_cpp(|auras| {
                auras.remove_transform_aura_like_cpp(spell_id);
            });
        }
        true
    }

    /// C++ `Unit::IsPolymorphed` (`Unit.cpp:9993-10004`): the active
    /// `m_transformSpell` classifies as `SPELL_SPECIFIC_MAGE_POLYMORPH`.
    ///
    /// C++ `SpellInfo::_LoadSpellSpecific` derives that specific only from the
    /// MAGE family branch (`SpellInfo.cpp:2665-2671`): family
    /// `SPELLFAMILY_MAGE` (3), family flag `0x1000000` and effect 0 applying
    /// `SPELL_AURA_MOD_CONFUSE`.
    pub fn represented_player_is_polymorphed_like_cpp(&self, hub: HubRef<'_>) -> Option<bool> {
        let transform_spell = hub
            .player_aura_subsystem_snapshot_like_cpp()?
            .transform_spell_like_cpp();
        if transform_spell == 0 {
            return Some(false);
        }
        let spell_store = hub.catalogs.spell_store()?;
        let Some(spell) = spell_store.get(transform_spell) else {
            return Some(false);
        };
        let family_matches = u32::try_from(transform_spell)
            .ok()
            .and_then(|spell_id| {
                hub.catalogs
                    .spell_class_options_store()
                    .and_then(|store| store.entry_for_spell_like_cpp(spell_id))
            })
            .is_some_and(|entry| {
                entry.spell_class_set as i32 == SPELL_FAMILY_MAGE_LIKE_CPP
                    && entry.spell_class_mask[0] & 0x0100_0000 != 0
            });
        let effect_zero_is_confuse = spell.effects().iter().any(|effect| {
            effect.effect_index == 0
                && effect.is_aura_like_cpp()
                && effect.effect_aura == wow_data::spell::aura_types::SPELL_AURA_MOD_CONFUSE
        });
        Some(family_matches && effect_zero_is_confuse)
    }
}

/// The `GetMiscValue` of a spell's first `SPELL_AURA_MOD_SHAPESHIFT` effect, the
/// C++ `ShapeshiftForm` the aura installs.
pub fn shapeshift_form_of_spell_like_cpp(spell: &wow_data::SpellInfo) -> Option<u32> {
    spell
        .effects()
        .iter()
        .find(|effect| effect.is_mod_shapeshift_aura_like_cpp())
        .and_then(|effect| u32::try_from(effect.effect_misc_value_1).ok())
}

/// Direction of one represented `SPELL_AURA_MOD_SHAPESHIFT` mutation, the input
/// the C++ `HandleShapeshiftBoosts` branches need.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedShapeshiftMutationLikeCpp {
    /// The aura was applied and its form now owns `GetShapeshiftForm()`.
    Applied { form_id: u32 },
    /// The aura was removed; `new_form` is the form that remains, `0` when none.
    Removed { removed_form: u32, new_form: u32 },
}
