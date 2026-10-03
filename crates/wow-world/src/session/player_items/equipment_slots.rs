//! Equipment slot resolution and the stats an equipped item contributes.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;


impl WorldSession {
    pub(crate) fn represented_weapon_crit_aura_modifier_like_cpp(
        &self,
        attack: WeaponAttackType,
    ) -> f32 {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_weapon_crit_aura_modifier_like_cpp(hub, attack)
    }
}
