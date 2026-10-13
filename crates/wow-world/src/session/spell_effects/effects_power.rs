//! Represented effect that restores spell charges.
//!
//! Moved out of the Session root under #621. Behaviour is preserved. The power,
//! energy and execute-log bodies moved to `wow_world_spell` (#1363 slice 9c).

use super::*;

impl WorldSession {
    /// C++ `Spell::EffectModifySpellCharges`.
    ///
    /// C++ restores one consumed charge per positive `damage` step on
    /// `effectInfo->MiscValue`. Packet fanout for `SendSetSpellCharges` is
    /// still outside this represented seam.
    pub(in crate::session) fn apply_modify_spell_charges_effect_like_cpp(
        &mut self,
        damage: i32,
        charge_category_id: i32,
        target_guid: ObjectGuid,
    ) -> u32 {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        if target_guid != player_guid || damage <= 0 {
            return 0;
        }
        let Ok(charge_category_id) = u32::try_from(charge_category_id) else {
            return 0;
        };
        if charge_category_id == 0 {
            return 0;
        }

        let restored = self
            .core
            .mutate_canonical_player_like_cpp(|player| {
                let history = &mut player.unit_mut().subsystems_mut().spells.history;
                let mut restored = 0;
                for _ in 0..damage {
                    if history.restore_charge(charge_category_id) {
                        restored += 1;
                    }
                }
                restored
            })
            .unwrap_or(0);

        let mut represented_restored = 0;
        for _ in 0..damage {
            if self.restore_represented_character_spell_charge_like_cpp(charge_category_id) {
                represented_restored += 1;
            }
        }

        restored.max(represented_restored)
    }
}
