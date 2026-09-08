//! `SpellCastData` cast flags and optional sections for represented
//! Player-origin normal casts.
//!
//! Classic anchors at `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`:
//! `Spells/Spell.cpp::SendSpellStart` assembles `CAST_FLAG_HAS_TRAJECTORY`
//! plus the conditional `PENDING`, `PROJECTILE`, `POWER_LEFT_SELF`,
//! `IMMUNITY` and `HEAL_PREDICTION` bits and writes the matching optional
//! sections. `Spells/Spell.cpp::SendSpellGo` assembles `CAST_FLAG_UNKNOWN_9`
//! plus `PENDING`, `PROJECTILE`, `POWER_LEFT_SELF`, the Death Knight
//! `NO_GCD | RUNE_LIST` pair, `RUNE_LIST` for `SPELL_EFFECT_ACTIVATE_RUNE`,
//! `ADJUST_MISSILE` for a trajectory cast and `NO_GCD` when the spell has no
//! `StartRecoveryTime`. `Server/Packets/SpellPackets.cpp:334-446` fixes the
//! serialized order, and the presence bits are written from the sections
//! themselves rather than inferred from these flags.
//!
//! Start and Go deliberately do not populate identical optional fields:
//! `IMMUNITY`/`HEAL_PREDICTION` are Start-only and `RUNE_LIST`/
//! `ADJUST_MISSILE` are Go-only, exactly as the two C++ writers do.

use super::*;

/// C++ `SpellCastFlags` (`Spells/Spell.h:76-111`), limited to the bits this
/// represented Player path can decide.
///
/// The remaining conditional bits belong to inputs this path rejects rather
/// than guesses, and are therefore never assembled here:
/// `CAST_FLAG_PROJECTILE` (0x20), `CAST_FLAG_ADJUST_MISSILE` (0x20000),
/// `CAST_FLAG_RUNE_LIST` (0x200000) and `CAST_FLAG_HEAL_PREDICTION`
/// (0x40000000). `CAST_FLAG_PENDING` (0x1) is triggered-only.
pub(in crate::session) mod cast_flags_like_cpp {
    pub(in crate::session) const HAS_TRAJECTORY: u32 = 0x0000_0002;
    pub(in crate::session) const UNKNOWN_9: u32 = 0x0000_0100;
    pub(in crate::session) const POWER_LEFT_SELF: u32 = 0x0000_0800;
    pub(in crate::session) const NO_GCD: u32 = 0x0004_0000;
    pub(in crate::session) const IMMUNITY: u32 = 0x0400_0000;
}

/// C++ `SharedDefines.h` values consumed by the two writers.
const SPELL_ATTR0_USES_RANGED_SLOT_LIKE_CPP: u32 = 0x0000_0002;
const SPELL_ATTR10_USES_RANGED_SLOT_COSMETIC_ONLY_LIKE_CPP: u32 = 0x0000_0004;
const SPELL_ATTR0_CU_NEEDS_AMMO_DATA_LIKE_CPP: u32 = 0x0008_0000;
const SPELL_ATTR8_HEAL_PREDICTION_LIKE_CPP: u32 = 0x0100_0000;
const SPELL_EFFECT_ACTIVATE_RUNE_LIKE_CPP: u32 = 146;
const CLASS_DEATH_KNIGHT_LIKE_CPP: u8 = 6;

/// Which of the two C++ writers is producing the payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) enum PlayerCastPublicationPhaseLikeCpp {
    /// `Spell::SendSpellStart`. `timed` is C++ `m_timer != 0`, which gates the
    /// immunity masks sampled for `CAST_FLAG_IMMUNITY`.
    Start { timed: bool },
    /// `Spell::SendSpellGo`.
    Go,
}

/// A C++ `SendSpellStart`/`SendSpellGo` input this represented Player path
/// does not own yet.
///
/// Each variant would force a *structurally* different `SpellCastData` than
/// the one this path can produce, so admission fails closed instead of
/// publishing a divergent Start/Go pair. This mirrors the existing creature
/// policy in `creature_ai_spell_requires_projectile_payload_like_cpp`: the
/// unrepresented dependency is named, not silently defaulted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) enum UnrepresentedPlayerCastPublicationLikeCpp {
    /// `CAST_FLAG_PROJECTILE` requires `Spell::GetSpellCastDataAmmo`, which
    /// reads the ranged weapon's template and the "Requires No Ammo" aura.
    ProjectileAmmo,
    /// `CAST_FLAG_RUNE_LIST` requires `Spell::m_runesState` and
    /// `Player::GetRunesState`; no rune state is represented.
    RuneList,
    /// `CAST_FLAG_HEAL_PREDICTION` requires `UpdateSpellHealPrediction`.
    HealPrediction,
    /// `CAST_FLAG_ADJUST_MISSILE` requires the request trajectory together
    /// with `Spell::m_delayMoment`; neither is retained by this path.
    MissileTrajectory,
    /// `Spell::GetCastSpellXSpellVisualId` selected a row gated by a
    /// `CasterUnitConditionID`; general `UnitCondition` evaluation is not
    /// represented, and visual zero is not a valid silent substitute.
    VisualUnitCondition,
}

impl UnrepresentedPlayerCastPublicationLikeCpp {
    pub(in crate::session) const fn reason_like_cpp(self) -> &'static str {
        match self {
            Self::ProjectileAmmo => "projectile ammo payload",
            Self::RuneList => "rune state payload",
            Self::HealPrediction => "heal prediction payload",
            Self::MissileTrajectory => "missile trajectory payload",
            Self::VisualUnitCondition => "caster UnitCondition visual selection",
        }
    }
}

impl WorldSession {
    /// C++ `SpellInfo::HasAttribute` for one attribute word at the caster's
    /// current map difficulty, matching the creature representation gate.
    fn represented_cast_spell_attribute_like_cpp(
        &self,
        spell_id: i32,
        attribute_word: usize,
        attribute: u32,
    ) -> bool {
        let config = &self.legacy_creature_aggro_config_like_cpp;
        config.spell_store.as_ref().is_some_and(|store| {
            store.has_attribute_for_difficulty_like_cpp(
                spell_id,
                self.current_map_difficulty_id_like_cpp(),
                config.difficulty_store.as_deref(),
                attribute_word,
                attribute,
            )
        })
    }

    /// C++ `SPELL_ATTR0_CU_NEEDS_AMMO_DATA`, a `SpellMgr` custom attribute.
    fn represented_cast_needs_ammo_data_like_cpp(&self, spell_id: i32) -> bool {
        let config = &self.legacy_creature_aggro_config_like_cpp;
        let Ok(spell_id) = u32::try_from(spell_id) else {
            return true;
        };
        config
            .spell_custom_attribute_store
            .as_ref()
            .is_some_and(|store| {
                creature_ai_spell_difficulty_chain_like_cpp(
                    self.current_map_difficulty_id_like_cpp(),
                    config,
                )
                .into_iter()
                .any(|difficulty_id| {
                    store.attributes_for_spell_difficulty_like_cpp(spell_id, u32::from(difficulty_id))
                        & SPELL_ATTR0_CU_NEEDS_AMMO_DATA_LIKE_CPP
                        != 0
                })
            })
    }

    /// C++ `SendSpellStart`/`SendSpellGo` `CAST_FLAG_PROJECTILE` condition.
    fn represented_cast_is_projectile_like_cpp(&self, spell_id: i32) -> bool {
        self.represented_cast_spell_attribute_like_cpp(
            spell_id,
            0,
            SPELL_ATTR0_USES_RANGED_SLOT_LIKE_CPP,
        ) || self.represented_cast_spell_attribute_like_cpp(
            spell_id,
            10,
            SPELL_ATTR10_USES_RANGED_SLOT_COSMETIC_ONLY_LIKE_CPP,
        ) || self.represented_cast_needs_ammo_data_like_cpp(spell_id)
    }

    /// C++ `SendSpellGo` `CAST_FLAG_RUNE_LIST` condition: any Death Knight
    /// cast that does not ignore power cost, or any `SPELL_EFFECT_ACTIVATE_RUNE`.
    fn represented_cast_needs_rune_list_like_cpp(&self, spell: &wow_data::SpellInfo) -> bool {
        if spell
            .effects()
            .iter()
            .any(|effect| effect.effect == SPELL_EFFECT_ACTIVATE_RUNE_LIKE_CPP)
        {
            return true;
        }
        self.represented_player_class_id_like_cpp() == Some(CLASS_DEATH_KNIGHT_LIKE_CPP)
    }

    fn represented_player_class_id_like_cpp(&self) -> Option<u8> {
        self.with_owned_player_like_cpp(|player| player.unit().data().class_id)
    }

    /// C++ `Unit::GetSchoolImmunityMask` over the represented aura owner.
    ///
    /// `SpellInfo::GetMechanicImmunityMask` is not represented; a caster whose
    /// only immunity is mechanic-based therefore publishes no `CAST_FLAG_IMMUNITY`
    /// where C++ would. This limit is recorded in the #589 checkpoint rather
    /// than hidden behind a zero default.
    fn represented_school_immunity_mask_like_cpp(&self) -> u32 {
        self.with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .aura_school_mask_like_cpp(wow_data::spell::aura_types::SPELL_AURA_SCHOOL_IMMUNITY)
        })
        .unwrap_or(0)
    }

    /// Refuse admission when a faithful Start/Go pair would need an input this
    /// path does not own. Returning `Some` is a rejection, not a defect.
    pub(in crate::session) fn player_cast_unrepresented_publication_like_cpp(
        &self,
        spell: &wow_data::SpellInfo,
        metadata: &SpellCastMetadata,
    ) -> Option<UnrepresentedPlayerCastPublicationLikeCpp> {
        if self.represented_cast_is_projectile_like_cpp(spell.spell_id) {
            return Some(UnrepresentedPlayerCastPublicationLikeCpp::ProjectileAmmo);
        }
        if self.represented_cast_needs_rune_list_like_cpp(spell) {
            return Some(UnrepresentedPlayerCastPublicationLikeCpp::RuneList);
        }
        if spell.cast_time_ms != 0
            && self.represented_cast_spell_attribute_like_cpp(
                spell.spell_id,
                8,
                SPELL_ATTR8_HEAL_PREDICTION_LIKE_CPP,
            )
        {
            return Some(UnrepresentedPlayerCastPublicationLikeCpp::HealPrediction);
        }
        if metadata.request_has_trajectory_like_cpp {
            return Some(UnrepresentedPlayerCastPublicationLikeCpp::MissileTrajectory);
        }
        None
    }

    /// C++ cast flags for one publication phase.
    ///
    /// `CAST_FLAG_PENDING` is deliberately absent: C++ only sets it for a
    /// triggered cast that is not `m_fromClient`, and every cast produced by
    /// this path is a normal client request.
    pub(in crate::session) fn player_cast_flags_like_cpp(
        &self,
        spell: &wow_data::SpellInfo,
        cast_data: &wow_packet::packets::spell::SpellCastData,
        phase: PlayerCastPublicationPhaseLikeCpp,
    ) -> u32 {
        let power_left_self = if cast_data.remaining_power.is_empty() {
            0
        } else {
            cast_flags_like_cpp::POWER_LEFT_SELF
        };
        match phase {
            PlayerCastPublicationPhaseLikeCpp::Start { timed: _ } => {
                let immunity = if cast_data.immunities.school != 0 || cast_data.immunities.value != 0
                {
                    cast_flags_like_cpp::IMMUNITY
                } else {
                    0
                };
                cast_flags_like_cpp::HAS_TRAJECTORY | power_left_self | immunity
            }
            PlayerCastPublicationPhaseLikeCpp::Go => {
                let no_gcd = if spell.cooldown_ms == 0 {
                    cast_flags_like_cpp::NO_GCD
                } else {
                    0
                };
                cast_flags_like_cpp::UNKNOWN_9 | power_left_self | no_gcd
            }
        }
    }

    /// Build the optional `SpellCastData` sections for one publication phase.
    ///
    /// C++ `SendSpellStart` samples the caster's power *before* the debit and
    /// `SendSpellGo` samples what remains *after* it; both call the same
    /// `Unit::GetPower` over `m_powerCost`, so the phase difference comes from
    /// the call site, not from a different computation here.
    pub(in crate::session) fn player_cast_wire_data_for_phase_like_cpp(
        &self,
        spell: &wow_data::SpellInfo,
        phase: PlayerCastPublicationPhaseLikeCpp,
    ) -> wow_packet::packets::spell::SpellCastData {
        let remaining_power = self
            .with_owned_player_like_cpp(|player| {
                let costs =
                    spell.calc_power_costs_like_cpp(player.unit().get_create_mana_like_cpp());
                // C++ gates the whole section on at least one non-health cost.
                if !costs
                    .iter()
                    .any(|cost| cost.power_type != PowerType::Health as i8)
                {
                    return Vec::new();
                }
                costs
                    .iter()
                    .filter_map(|cost| {
                        let power =
                            <PowerType as num_traits::FromPrimitive>::from_i8(cost.power_type)?;
                        Some(wow_packet::packets::spell::SpellPowerData {
                            amount: player.get_power(power),
                            power_type: cost.power_type,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        // `SendSpellGo` writes no immunity section at all; only `SendSpellStart`
        // does, and only while the cast is still timed.
        let immunities = match phase {
            PlayerCastPublicationPhaseLikeCpp::Start { timed: true } => {
                wow_packet::packets::spell::CreatureImmunities {
                    school: self.represented_school_immunity_mask_like_cpp(),
                    value: 0,
                }
            }
            _ => Default::default(),
        };
        wow_packet::packets::spell::SpellCastData {
            remaining_power,
            immunities,
            ..Default::default()
        }
    }
}
