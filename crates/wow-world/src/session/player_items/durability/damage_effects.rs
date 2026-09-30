//! damage effects for the existing durability owner.

use super::*;

impl WorldSession {
    /// C++ `Spell::EffectDurabilityDamage` (`SpellEffects.cpp:4316-4352`).
    pub(in crate::session) fn apply_durability_damage_effect_like_cpp(
        &mut self,
        effect: u32,
        damage: i32,
        slot: i32,
        target_guid: ObjectGuid,
    ) {
        if self.player_guid() != Some(target_guid) {
            return;
        }
        let effect = i32::try_from(effect).unwrap_or(0);
        if slot < 0 {
            self.apply_represented_durability_points_loss_all_like_cpp(damage, slot < -1);
            // C++ logs `-1`/`-1` for the all-items branch
            // (`SpellEffects.cpp:4328`).
            self.record_spell_execute_log_durability_damage_like_cpp(effect, target_guid, -1, -1);
            return;
        }
        let Ok(slot) = u8::try_from(slot) else {
            return;
        };
        if slot >= INVENTORY_SLOT_BAG_END {
            return;
        }
        let item_entry = self
            .resolved_inventory_item_like_cpp(slot)
            .map(|item| item.entry_id);
        if self.apply_represented_durability_points_loss_at_slot_like_cpp(slot, damage)
            && let Some(item_entry) = item_entry
        {
            // C++ logs the item entry and the slot as `ItemID`/`Amount`
            // (`SpellEffects.cpp:4339`).
            self.record_spell_execute_log_durability_damage_like_cpp(
                effect,
                target_guid,
                i32::try_from(item_entry).unwrap_or(i32::MAX),
                i32::from(slot),
            );
        }
    }
    /// C++ `Spell::EffectDurabilityDamagePCT` (`SpellEffects.cpp:4354-4373`).
    ///
    /// C++ has no `ExecuteLogEffectDurabilityDamage` call on this branch, so
    /// the percent variant publishes no execute-log row.
    pub(in crate::session) fn apply_durability_damage_pct_effect_like_cpp(
        &mut self,
        damage: i32,
        slot: i32,
        target_guid: ObjectGuid,
    ) {
        if self.player_guid() != Some(target_guid) {
            return;
        }
        if slot < 0 {
            self.apply_represented_durability_loss_all_like_cpp(
                f64::from(damage) / 100.0,
                slot < -1,
            );
            return;
        }
        let Ok(slot) = u8::try_from(slot) else {
            return;
        };
        if slot >= INVENTORY_SLOT_BAG_END || damage <= 0 {
            return;
        }
        self.apply_represented_durability_loss_at_slot_like_cpp(slot, f64::from(damage) / 100.0);
    }
    /// C++ `GetTotalAuraMultiplier(SPELL_AURA_MOD_DURABILITY_LOSS)`.
    pub(super) fn represented_durability_loss_aura_multiplier_like_cpp(&self) -> f32 {
        self.resolved_aura_effects_by_spell_aura_type_like_cpp(
            wow_data::spell::aura_types::SPELL_AURA_MOD_DURABILITY_LOSS,
        )
        .unwrap_or_default()
        .into_iter()
        .fold(1.0, |acc, (_, amount)| acc * (1.0 + amount as f32 / 100.0))
    }
    /// C++ `HasAuraType(SPELL_AURA_PREVENT_DURABILITY_LOSS)`.
    pub(super) fn represented_prevent_durability_loss_like_cpp(&self) -> bool {
        self.resolved_aura_effects_by_spell_aura_type_like_cpp(
            wow_data::spell::aura_types::SPELL_AURA_PREVENT_DURABILITY_LOSS,
        )
        .is_some_and(|effects| !effects.is_empty())
    }
    /// C++ `Unit::Kill` player-victim durability branch (`Unit.cpp:10639-10648`).
    ///
    /// Applies `Player::DurabilityLossAll(baseLoss, false)` and returns the
    /// `SMSG_DURABILITY_DAMAGE_DEATH` percent C++ derives as
    /// `baseLoss - baseLoss * GetTotalAuraMultiplier(MOD_DURABILITY_LOSS)`.
    /// C++ truncates that value to `uint32`, so with no aura the message percent
    /// is 0; that legacy behaviour is reproduced rather than repaired.
    pub(crate) fn apply_represented_durability_loss_on_death_like_cpp(&mut self) -> u32 {
        let base_loss = f64::from(self.durability_loss_on_death_rate_like_cpp());
        let multiplier = f64::from(self.represented_durability_loss_aura_multiplier_like_cpp());
        let loss = (base_loss - base_loss * multiplier) as u32;
        self.apply_represented_durability_loss_all_like_cpp(base_loss, false);
        loss
    }
    /// C++ `Unit::Kill` creature-killer durability branch (`Unit.cpp:10639-10648`).
    ///
    /// The map commits the lethal swing before delivery, so `over_damage >= 0`
    /// is the represented kill signal. C++ applies the PvE condition
    /// `durabilityLoss && !player && !victim->InBattleground()`; a battleground
    /// victim is skipped. The loss message is published before the melee result
    /// presentation, matching the C++ order inside `DealMeleeDamage`.
    pub(crate) fn publish_creature_melee_death_durability_loss_like_cpp(
        &mut self,
        over_damage: i32,
    ) {
        if over_damage < 0 || self.represented_player_in_battleground_like_cpp() {
            return;
        }
        let loss = self.apply_represented_durability_loss_on_death_like_cpp();
        self.send_packet(&wow_packet::packets::misc::DurabilityDamageDeath {
            percent: loss as i32,
        });
    }
    /// C++ `Player::InBattleground` (`Player.h:2335`) read through the canonical
    /// Player's represented battleground state.
    #[must_use]
    pub(crate) fn represented_player_in_battleground_like_cpp(&self) -> bool {
        self.with_owned_player_like_cpp(|player| {
            player
                .battleground_state_like_cpp()
                .in_battleground_like_cpp()
        })
        .unwrap_or(false)
    }
}
