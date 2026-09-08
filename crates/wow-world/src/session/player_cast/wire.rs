//! `SpellCastData` cast flags and optional sections for represented
//! Player-origin normal casts.
//!
//! Classic anchors at `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`:
//! `Spells/Spell.cpp::SendSpellStart` assembles `CAST_FLAG_HAS_TRAJECTORY`
//! plus the conditional `PENDING`, `PROJECTILE`, `POWER_LEFT_SELF`,
//! `IMMUNITY` and `HEAL_PREDICTION` bits and then writes exactly the sections
//! its own flags selected. `Spells/Spell.cpp::SendSpellGo` assembles
//! `CAST_FLAG_UNKNOWN_9` plus `PENDING`, `PROJECTILE`, `POWER_LEFT_SELF`, the
//! Death Knight `NO_GCD | RUNE_LIST` pair, `RUNE_LIST` for
//! `SPELL_EFFECT_ACTIVATE_RUNE`, `ADJUST_MISSILE` for a trajectory cast, and
//! `NO_GCD` when the spell has no `StartRecoveryTime`.
//! `Server/Packets/SpellPackets.cpp:334-446` fixes the serialized order, and
//! the presence bits are written from the sections themselves rather than
//! inferred from these flags.
//!
//! Start and Go deliberately do not populate identical optional fields:
//! `IMMUNITY` and `HEAL_PREDICTION` are Start-only, `RUNE_LIST` and
//! `ADJUST_MISSILE` are Go-only, and Start samples power before the debit
//! while Go samples what remains after it.
//!
//! # Represented value limits
//!
//! Every section C++ writes is written here with the same presence and the
//! same structure, so the wire shape does not diverge. Three of them carry a
//! represented value whose underlying subsystem is not ported yet, and each
//! degrades exactly the way C++ does for a caster that has nothing to report:
//!
//! * `RemainingRunes` uses `Start`/`Count` of zero because no rune resource is
//!   represented. C++ writes `m_runesState` and `Player::GetRunesState`, and
//!   its own cooldown loop is commented out, so the cooldown vector is empty
//!   on both sides.
//! * `AmmoDisplayID` resolves to zero unless a thrown ranged weapon or the
//!   "Requires No Ammo" aura is represented. C++ `GetSpellCastDataAmmo` also
//!   returns zero for a bow or gun without that aura, and it discards the
//!   inventory type it computes, so `AmmoInventoryType` stays absent.
//! * `MissileTrajectory` carries the request pitch with a zero `TravelTime`
//!   because `Spell::m_delayMoment` needs missile simulation.
//!
//! `Immunities.Value` is likewise zero: `SpellInfo::GetMechanicImmunityMask`
//! is not represented, so a caster whose only immunity is mechanic-based
//! publishes no `CAST_FLAG_IMMUNITY` where C++ would. These are value limits
//! recorded in the #589 checkpoint, not silent structural substitutions.

use super::*;

/// C++ `SpellCastFlags` (`Spells/Spell.h:76-111`).
pub(in crate::session) mod cast_flags_like_cpp {
    pub(in crate::session) const HAS_TRAJECTORY: u32 = 0x0000_0002;
    pub(in crate::session) const PROJECTILE: u32 = 0x0000_0020;
    pub(in crate::session) const UNKNOWN_9: u32 = 0x0000_0100;
    pub(in crate::session) const POWER_LEFT_SELF: u32 = 0x0000_0800;
    pub(in crate::session) const ADJUST_MISSILE: u32 = 0x0002_0000;
    pub(in crate::session) const NO_GCD: u32 = 0x0004_0000;
    pub(in crate::session) const RUNE_LIST: u32 = 0x0020_0000;
    pub(in crate::session) const IMMUNITY: u32 = 0x0400_0000;
    pub(in crate::session) const HEAL_PREDICTION: u32 = 0x4000_0000;
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
    /// `Spell::SendSpellStart`. `timed` is C++ `m_timer != 0`, which gates both
    /// the immunity masks and the heal-prediction section.
    Start { timed: bool },
    /// `Spell::SendSpellGo`.
    Go,
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
            return false;
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
                    store
                        .attributes_for_spell_difficulty_like_cpp(spell_id, u32::from(difficulty_id))
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

    /// C++ `SendSpellGo` `CAST_FLAG_RUNE_LIST`: any `SPELL_EFFECT_ACTIVATE_RUNE`
    /// spell, or any Death Knight cast that does not ignore power cost. A
    /// represented normal request never ignores power cost.
    fn represented_cast_needs_rune_list_like_cpp(&self, spell: &wow_data::SpellInfo) -> bool {
        spell
            .effects()
            .iter()
            .any(|effect| effect.effect == SPELL_EFFECT_ACTIVATE_RUNE_LIKE_CPP)
            || self.represented_player_class_id_like_cpp() == Some(CLASS_DEATH_KNIGHT_LIKE_CPP)
    }

    fn represented_player_class_id_like_cpp(&self) -> Option<u8> {
        self.with_owned_player_like_cpp(|player| player.unit().data().class_id)
    }

    /// C++ `Unit::GetSchoolImmunityMask` over the represented aura owner.
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

    /// C++ `SendSpellStart`/`SendSpellGo` `CAST_FLAG_POWER_LEFT_SELF` rows.
    ///
    /// The whole section is gated on at least one non-health cost, and each
    /// row reports the caster's power at call time: before the debit for
    /// Start, after it for Go.
    fn represented_cast_remaining_power_like_cpp(
        &self,
        spell: &wow_data::SpellInfo,
    ) -> Vec<wow_packet::packets::spell::SpellPowerData> {
        self.with_owned_player_like_cpp(|player| {
            let costs = spell.calc_power_costs_like_cpp(player.unit().get_create_mana_like_cpp());
            if !costs
                .iter()
                .any(|cost| cost.power_type != PowerType::Health as i8)
            {
                return Vec::new();
            }
            costs
                .iter()
                .filter_map(|cost| {
                    let power = <PowerType as num_traits::FromPrimitive>::from_i8(cost.power_type)?;
                    Some(wow_packet::packets::spell::SpellPowerData {
                        amount: player.get_power(power),
                        power_type: cost.power_type,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
    }

    /// C++ `Spell::GetSpellCastDataAmmo` for a Player caster.
    ///
    /// C++ assigns the returned display id unconditionally, so the field is
    /// always present under `CAST_FLAG_PROJECTILE`, and it discards the
    /// inventory type it computed, so `AmmoInventoryType` is never written.
    /// The thrown-weapon and "Requires No Ammo" branches need represented
    /// ranged equipment; without them C++ also returns zero.
    const fn represented_cast_ammo_display_id_like_cpp(&self) -> i32 {
        0
    }

    /// C++ cast flags and the optional sections they select, for one phase.
    ///
    /// Flags are assembled first and the sections are then filled exactly
    /// where the flag selected them, mirroring both C++ writers.
    pub(in crate::session) fn player_cast_publication_like_cpp(
        &self,
        spell: &wow_data::SpellInfo,
        metadata: &SpellCastMetadata,
        phase: PlayerCastPublicationPhaseLikeCpp,
    ) -> (wow_packet::packets::spell::SpellCastData, u32) {
        let remaining_power = self.represented_cast_remaining_power_like_cpp(spell);
        let mut cast_data = wow_packet::packets::spell::SpellCastData::default();
        let mut cast_flags = 0_u32;

        // `CAST_FLAG_PENDING` is deliberately absent from both phases: C++ sets
        // it only for a triggered cast that is not `m_fromClient`, and every
        // cast produced by this path is a normal client request.
        if !remaining_power.is_empty() {
            cast_flags |= cast_flags_like_cpp::POWER_LEFT_SELF;
            cast_data.remaining_power = remaining_power;
        }
        if self.represented_cast_is_projectile_like_cpp(spell.spell_id) {
            cast_flags |= cast_flags_like_cpp::PROJECTILE;
            cast_data.ammo_display_id = Some(self.represented_cast_ammo_display_id_like_cpp());
        }

        match phase {
            PlayerCastPublicationPhaseLikeCpp::Start { timed } => {
                cast_flags |= cast_flags_like_cpp::HAS_TRAJECTORY;
                // C++ samples both immunity masks only while `m_timer != 0`.
                let school = if timed {
                    self.represented_school_immunity_mask_like_cpp()
                } else {
                    0
                };
                if school != 0 {
                    cast_flags |= cast_flags_like_cpp::IMMUNITY;
                    cast_data.immunities =
                        wow_packet::packets::spell::CreatureImmunities { school, value: 0 };
                }
                if timed
                    && self.represented_cast_spell_attribute_like_cpp(
                        spell.spell_id,
                        8,
                        SPELL_ATTR8_HEAL_PREDICTION_LIKE_CPP,
                    )
                {
                    // `UpdateSpellHealPrediction` is not represented; C++ also
                    // writes a zeroed prediction when it has nothing to add.
                    cast_flags |= cast_flags_like_cpp::HEAL_PREDICTION;
                }
            }
            PlayerCastPublicationPhaseLikeCpp::Go => {
                cast_flags |= cast_flags_like_cpp::UNKNOWN_9;
                if self.represented_cast_needs_rune_list_like_cpp(spell) {
                    cast_flags |= cast_flags_like_cpp::RUNE_LIST;
                    if self.represented_player_class_id_like_cpp()
                        == Some(CLASS_DEATH_KNIGHT_LIKE_CPP)
                    {
                        // C++ pairs NO_GCD with RUNE_LIST for the Death Knight
                        // branch only, not for SPELL_EFFECT_ACTIVATE_RUNE.
                        cast_flags |= cast_flags_like_cpp::NO_GCD;
                    }
                    cast_data.remaining_runes =
                        Some(wow_packet::packets::spell::RuneData::default());
                }
                if metadata.request_has_trajectory_like_cpp {
                    cast_flags |= cast_flags_like_cpp::ADJUST_MISSILE;
                    cast_data.missile_trajectory =
                        wow_packet::packets::spell::MissileTrajectoryResult {
                            travel_time: 0,
                            pitch: metadata.request_trajectory_pitch_like_cpp,
                        };
                }
                if spell.cooldown_ms == 0 {
                    cast_flags |= cast_flags_like_cpp::NO_GCD;
                }
            }
        }

        (cast_data, cast_flags)
    }
}
