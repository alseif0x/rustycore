use crate::SessionSpellState;
pub fn unit_owned_apply_aura_effect_mask_like_cpp(
    spell: &wow_data::SpellInfo,
) -> u32 {
    use wow_data::spell::spell_effect_types::{
        SPELL_EFFECT_APPLY_AREA_AURA_ENEMY, SPELL_EFFECT_APPLY_AREA_AURA_FRIEND,
        SPELL_EFFECT_APPLY_AREA_AURA_OWNER, SPELL_EFFECT_APPLY_AREA_AURA_PARTY,
        SPELL_EFFECT_APPLY_AREA_AURA_PET, SPELL_EFFECT_APPLY_AREA_AURA_RAID,
        SPELL_EFFECT_APPLY_AURA, SPELL_EFFECT_APPLY_AURA_ON_PET,
    };

    const SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS: u32 = 202;
    const SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM: u32 = 271;

    spell.effects().iter().fold(0, |mask, effect| {
        let unit_owned = matches!(
            effect.effect,
            SPELL_EFFECT_APPLY_AURA
                | SPELL_EFFECT_APPLY_AURA_ON_PET
                | SPELL_EFFECT_APPLY_AREA_AURA_PARTY
                | SPELL_EFFECT_APPLY_AREA_AURA_RAID
                | SPELL_EFFECT_APPLY_AREA_AURA_FRIEND
                | SPELL_EFFECT_APPLY_AREA_AURA_ENEMY
                | SPELL_EFFECT_APPLY_AREA_AURA_PET
                | SPELL_EFFECT_APPLY_AREA_AURA_OWNER
                | SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS
                | SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM
        );
        if unit_owned && effect.effect_index < u32::BITS {
            mask | (1u32 << effect.effect_index)
        } else {
            mask
        }
    })
}
use wow_entities::AuraApplicationLikeCpp as AuraApplication;
use wow_world_core::session::{HubMut, HubRef};

impl SessionSpellState {
    pub fn remove_player_visible_aura_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        slot: u8,
    ) -> Option<AuraApplication> {
        self.remove_player_visible_aura_with_access_like_cpp(
            &mut hub.player_aura_removal_access_like_cpp(), slot,
        )
    }

    pub fn remove_player_visible_aura_with_access_like_cpp(
        &mut self,
        player: &mut wow_world_core::session::PlayerAuraRemovalAccessLikeCpp<'_>,
        slot: u8,
    ) -> Option<AuraApplication> {
        player.remove_player_visible_aura_like_cpp(slot)
    }

    pub fn send_aura_update_removed(&self, hub: HubRef<'_>, slot: u8) {
        self.send_aura_update_removed_with_publication_like_cpp(
            hub.core.player_guid(), &hub.core.packet_publication_access_like_cpp(), slot,
        );
    }

    pub fn send_aura_update_removed_with_publication_like_cpp(
        &self,
        target_guid: Option<wow_core::ObjectGuid>,
        publication: &wow_world_core::session::PacketPublicationAccessLikeCpp<'_>,
        slot: u8,
    ) {
        let Some(target_guid) = target_guid else {
            return;
        };
        publication.send_packet(&wow_packet::packets::misc::AuraUpdate {
                unit_guid: target_guid,
                update_all: false,
                auras: vec![wow_packet::packets::misc::AuraInfoLikeCpp {
                    slot,
                    aura_data: None,
                }],
            });
    }
}
