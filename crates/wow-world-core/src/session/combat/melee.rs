use crate::session::state::SessionCore;
use std::collections::HashMap;
use std::sync::Arc;
use wow_core::ObjectGuid;
use wow_data::{SpellStore, VEHICLE_SEAT_FLAG_CAN_ATTACK};
use wow_entities::AuraApplicationLikeCpp;

/// C++ `Unit::MeleeDamageBonusDone`'s auto-attack percentage term
/// (`Unit.cpp:7620-7627`): `AddPct(DoneTotalMod, amount)` for every active
/// `SPELL_AURA_MOD_AUTOATTACK_DAMAGE` effect. The represented white swing
/// multiplies its rolled damage by the returned factor; `1.0` when nothing is
/// active.
fn represented_autoattack_damage_multiplier_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
) -> f32 {
    crate::session::player_aura_effects_by_spell_aura_type_like_cpp(
        auras,
        spell_store,
        wow_data::spell::aura_types::SPELL_AURA_MOD_AUTOATTACK_DAMAGE,
    )
    .into_iter()
    .fold(1.0_f32, |total, (_, amount)| {
        total * (1.0 + amount as f32 / 100.0)
    })
}

impl SessionCore {
    pub fn set_player_attack_swing_error_like_cpp(&mut self, error: Option<u8>) {
        use wow_packet::ServerPacket;
        use wow_packet::packets::combat::AttackSwingError;

        let Some(publish) = self.mutate_canonical_player_like_cpp(|player| {
            player.set_attack_swing_error_like_cpp(error)
        }) else {
            return;
        };
        if publish {
            if let Some(reason) = error {
                let _ = self.send_tx().send(AttackSwingError { reason }.to_bytes());
            }
        }
    }
}

impl crate::session::HubMut<'_> {
    pub fn add_canonical_attacker_like_cpp(&mut self, victim: ObjectGuid, attacker: ObjectGuid) {
        if self
            .core
            .mutate_canonical_player_by_guid_like_cpp(victim, |victim| {
                victim.unit_mut().add_attacker_like_cpp(attacker)
            })
            .is_some()
        {
            return;
        }
        let _ = self
            .core
            .mutate_canonical_creature_by_guid_like_cpp(victim, |victim| {
                victim.unit_mut().add_attacker_like_cpp(attacker)
            });
    }

    pub fn stop_player_attack_like_cpp(&mut self) -> Option<ObjectGuid> {
        let player_guid = self.core.player_guid()?;
        let target = match self.core.mutate_canonical_player_like_cpp(|player| {
            match player.unit_mut().attack_stop_like_cpp() {
                wow_entities::UnitAttackStopOutcome::Stopped { victim } => Some(victim),
                wow_entities::UnitAttackStopOutcome::NoVictim => None,
            }
        }) {
            Some(Some(victim)) => victim,
            // C++ Unit::AttackStop returns false when m_attacking is null; a
            // stale session mirror must not invent a victim when canonical
            // player state exists and says there is none.
            Some(None) => {
                self.set_combat_target_like_cpp(None);
                self.set_in_combat_like_cpp(false);
                return None;
            }
            None => self.shared().resolved_combat_target_like_cpp().flatten()?,
        };
        self.set_combat_target_like_cpp(None);
        self.set_in_combat_like_cpp(false);
        if self.shared().selection_guid_like_cpp() == Some(target) {
            self.set_selection_guid_like_cpp(None);
        }
        self.remove_canonical_attacker_like_cpp(target, player_guid);
        let _ = self.core.mutate_world_creature(target, |victim| {
            victim
                .creature
                .unit_mut()
                .remove_attacker_like_cpp(player_guid);
        });
        Some(target)
    }
}

impl crate::session::HubRef<'_> {
    pub fn canonical_player_attack_state_like_cpp(&self) -> Option<Option<ObjectGuid>> {
        let guid = self.core.player_guid?;
        let map_id = u32::from(self.core.player_map_id_like_cpp());
        let manager = Arc::clone(self.core.canonical_map_manager.as_ref()?);
        let manager = manager.lock().ok()?;
        let mut result = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if result.is_none()
                && let Some(player) = managed.map().get_typed_player(guid)
            {
                result = Some(player.unit().attacking());
            }
        });
        result
    }

    /// C++ `Unit::MeleeDamageBonusDone`'s `SPELL_AURA_MOD_AUTOATTACK_DAMAGE`
    /// factor for the canonical Player (`Unit.cpp:7620-7627`): `1.0` when the
    /// aura container or the spell store cannot be resolved.
    ///
    /// The owning session writes the result on the canonical Player through the
    /// aura-mutation sync, so the map-owned swing path only reads it.
    pub fn represented_player_autoattack_damage_multiplier_like_cpp(&self) -> f32 {
        let (Some(auras), Some(spell_store)) = (
            self.resolved_player_visible_auras_like_cpp(),
            self.catalogs.spell_store(),
        ) else {
            return 1.0;
        };
        represented_autoattack_damage_multiplier_like_cpp(&auras, spell_store)
    }

    pub fn player_vehicle_seat_allows_attack_like_cpp(&self) -> bool {
        let Some((seat_flags, _)) = self.player_vehicle_seat_state_like_cpp() else {
            return false;
        };
        match seat_flags {
            Some(flags) => flags & VEHICLE_SEAT_FLAG_CAN_ATTACK != 0,
            None => true,
        }
    }
}

impl crate::session::state::SessionCatalogs {
    pub fn player_class_attack_power_coefficients_like_cpp(
        &self,
        class: u8,
    ) -> Option<(u8, u8, u8)> {
        self.chr
            .classes_store
            .as_ref()?
            .get(u32::from(class))
            .map(|entry| {
                (
                    entry.attack_power_per_strength,
                    entry.attack_power_per_agility,
                    entry.ranged_attack_power_per_agility,
                )
            })
    }
}
