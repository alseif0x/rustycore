//! Preserve the represented primary-effect fallback after per-effect handling.

use super::*;

impl WorldSession {
    pub(super) fn apply_primary_spell_effect_fallback_like_cpp(
        &mut self,
        effect_type: u32,
        spell_info: &wow_data::SpellInfo,
        spell_id: i32,
        player_guid: ObjectGuid,
        cast_id: ObjectGuid,
        spell_visual_id: u32,
    ) -> Result<(), &'static str> {
        match effect_type {
            x if wow_data::spell::spell_effect_types::is_cpp_null_or_unused_noop(x) => {}
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA => {
                if spell_info.effects().is_empty() {
                    self.apply_aura_with_effect_mask_and_provenance_like_cpp(
                        spell_id,
                        player_guid,
                        30000,
                        0x00000001,
                        0x00000001,
                        wow_entities::AuraCastProvenanceLikeCpp {
                            cast_id,
                            spell_visual_id: spell_visual_id.min(i32::MAX as u32) as i32,
                        },
                    )?;
                }
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_INSTAKILL
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_ENVIRONMENTAL_DAMAGE
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL_MECHANICAL
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL_MAX_HEALTH
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL_PCT
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_ADD_EXTRA_ATTACKS
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_INEBRIATE
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_REPUTATION
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_DRAIN
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_ENERGIZE
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_ENERGIZE_PCT
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_BURN
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_HEALTH_LEECH
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_DUAL_WIELD
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_PARRY
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_BLOCK
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_GIVE_HONOR
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_THREAT
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_DISTRACT
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_MODIFY_THREAT_PERCENT
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_ATTACK_ME
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_MODIFY_COOLDOWN
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_MODIFY_CHARGES
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_SANCTUARY
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_SELF_RESURRECT
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_STUCK
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_PLAY_MOVIE
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_DUEL
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_DISMISS_PET
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_FORCE_DESELECT
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_CHANGE_RAID_MARKER
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_SPELL
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_SET
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_UPGRADE_HEIRLOOM
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_UNCAGE_BATTLEPET
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_CHANGE_BATTLEPET_QUALITY
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_LEVEL
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_EXPERIENCE
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_PULL
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_ILLUSION
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_TRADE_SKILL
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_QUEST_COMPLETE
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_KILL_CREDIT
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_KILL_CREDIT2
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_BIND
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_TELEPORT_UNITS
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_DURABILITY_DAMAGE
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_DURABILITY_DAMAGE_PCT => {
            }
            _ => {
                debug!("Spell effect type {} not yet implemented", effect_type);
            }
        }

        Ok(())
    }
}
