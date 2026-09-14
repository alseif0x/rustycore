use super::Player;
use crate::{AppliedAuraRef, AuraApplicationLikeCpp, AuraThreatSnapshotLikeCpp};
use wow_core::ObjectGuid;

impl Player {
    /// C++ `Unit` owns the Player aura application map; Session owns only
    /// packet and catalog adaptation around these state transitions.
    pub fn insert_player_visible_aura_like_cpp(&mut self, aura: AuraApplicationLikeCpp) {
        self.unit_mut()
            .subsystems_mut()
            .auras
            .insert_runtime_application_like_cpp(aura);
    }

    pub fn remove_player_visible_aura_like_cpp(
        &mut self,
        slot: u8,
    ) -> Option<AuraApplicationLikeCpp> {
        self.unit_mut()
            .subsystems_mut()
            .auras
            .remove_runtime_application_like_cpp(slot)
    }

    pub fn set_player_aura_authority_complete_like_cpp(&mut self, complete: bool) {
        self.unit_mut()
            .subsystems_mut()
            .auras
            .set_persisted_player_aura_authority_complete_like_cpp(complete);
    }

    pub fn tombstone_player_spell_hit_aura_authority_like_cpp(&mut self) {
        self.unit_mut()
            .subsystems_mut()
            .auras
            .tombstone_spell_hit_aura_authority_like_cpp();
    }

    pub fn reset_player_aura_source_authority_like_cpp(&mut self) {
        let auras = &mut self.unit_mut().subsystems_mut().auras;
        auras.clear_runtime_applications_like_cpp();
        auras.reset_player_aura_source_authority_like_cpp();
    }

    pub fn install_player_threat_aura_like_cpp(
        &mut self,
        slot: u8,
        snapshot: AuraThreatSnapshotLikeCpp,
        aura: AuraApplicationLikeCpp,
    ) {
        let auras = &mut self.unit_mut().subsystems_mut().auras;
        auras.insert_threat_snapshot_like_cpp(slot, snapshot);
        auras.insert_runtime_application_like_cpp(aura);
    }

    pub fn remove_player_threat_aura_like_cpp(
        &mut self,
        spell_id: u32,
        caster_guid: ObjectGuid,
        slot: u8,
        effect_mask: u32,
    ) {
        let auras = &mut self.unit_mut().subsystems_mut().auras;
        auras.remove_threat_snapshot_like_cpp(slot);
        for effect_index in 0..u32::BITS {
            let effect_bit = 1_u32 << effect_index;
            if effect_mask & effect_bit != 0 {
                auras.remove_applied(AppliedAuraRef::new(spell_id, caster_guid, slot, effect_bit));
            }
        }
    }

    pub fn apply_player_threat_aura_like_cpp(
        &mut self,
        spell_id: u32,
        caster_guid: ObjectGuid,
        slot: u8,
        snapshot: AuraThreatSnapshotLikeCpp,
    ) {
        let interrupt_flags = snapshot.interrupt_flags();
        let auras = &mut self.unit_mut().subsystems_mut().auras;
        auras.insert_threat_snapshot_like_cpp(slot, snapshot.clone());
        for &(effect_bit, aura_type, amount, misc_value) in snapshot.effects() {
            let aura = AppliedAuraRef::new(spell_id, caster_guid, slot, effect_bit);
            auras.register_applied_aura(aura, None, interrupt_flags[0], interrupt_flags[1]);
            auras.register_applied_aura_effect_like_cpp(aura, aura_type, amount, misc_value);
        }
    }
}
