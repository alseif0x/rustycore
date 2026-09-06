use wow_core::ObjectGuid;
use wow_packet::packets::party::{
    PartyMemberAuraState, PartyMemberPetStats, PartyMemberPhaseStates,
};

use crate::canonical_player_access::{
    CanonicalPlayerPartyStateLikeCpp, CanonicalPlayerPresentationLikeCpp,
    CanonicalPlayerVitalsLikeCpp, canonical_player_party_state_like_cpp,
    canonical_player_presentation_like_cpp, canonical_player_vitals_like_cpp,
    canonical_unit_party_member_visible_auras_like_cpp, with_canonical_player_at_like_cpp,
    with_canonical_player_at_mut_like_cpp,
};
use crate::phasing::party_member_phase_states_like_cpp;
use crate::player_directory::PlayerRegistry;

impl PlayerRegistry {
    pub(crate) fn canonical_at<R>(
        &self,
        guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        read: impl FnOnce(&wow_entities::Player) -> R,
    ) -> Option<R> {
        with_canonical_player_at_like_cpp(
            &self.canonical_map_manager_like_cpp()?,
            guid,
            map_id.into(),
            instance_id,
            read,
        )
    }

    pub(crate) fn canonical_at_mut<R>(
        &self,
        guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        write: impl FnOnce(&mut wow_entities::Player) -> R,
    ) -> Option<R> {
        with_canonical_player_at_mut_like_cpp(
            &self.canonical_map_manager_like_cpp()?,
            guid,
            map_id.into(),
            instance_id,
            write,
        )
    }

    pub(crate) fn party_state(
        &self,
        guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
    ) -> Option<CanonicalPlayerPartyStateLikeCpp> {
        self.canonical_at(
            guid,
            map_id,
            instance_id,
            canonical_player_party_state_like_cpp,
        )
    }

    pub(crate) fn party_gameplay_projection(
        &self,
        guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
    ) -> Option<(
        bool,
        i32,
        PartyMemberPhaseStates,
        Vec<PartyMemberAuraState>,
        Option<PartyMemberPetStats>,
    )> {
        let (in_vehicle, seat, phase, auras, pet_guid) =
            self.canonical_at(guid, map_id, instance_id, |player| {
                (
                    player.gameplay_state().vehicle_seat_flags.is_some(),
                    player
                        .gameplay_state()
                        .vehicle_seat_id
                        .and_then(|seat| i32::try_from(seat).ok())
                        .unwrap_or(0),
                    player.unit().world().phase_shift().clone(),
                    canonical_unit_party_member_visible_auras_like_cpp(player.unit()),
                    player.gameplay_state().pet_guid,
                )
            })?;
        let phase = party_member_phase_states_like_cpp(&phase).unwrap_or_default();
        let pet = pet_guid.and_then(|pet_guid| {
            let manager = self.canonical_map_manager_like_cpp()?;
            let manager = manager.lock().ok()?;
            manager
                .find_map(map_id.into(), instance_id)?
                .map()
                .with_pet_like_cpp(pet_guid, |pet| {
                    if pet.owner_guid() != guid {
                        return None;
                    }
                    let creature = pet.creature();
                    let unit = creature.unit();
                    Some(PartyMemberPetStats {
                        guid: pet_guid,
                        model_id: unit.data().display_id,
                        current_health: i32::try_from(creature.current_health())
                            .unwrap_or(i32::MAX),
                        max_health: i32::try_from(creature.max_health()).unwrap_or(i32::MAX),
                        auras: canonical_unit_party_member_visible_auras_like_cpp(unit),
                        name: unit.world().name().to_string(),
                    })
                })?
        });
        Some((in_vehicle, seat, phase, auras, pet))
    }

    pub(crate) fn create_state(
        &self,
        guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
    ) -> Option<(
        CanonicalPlayerVitalsLikeCpp,
        [u8; 2],
        CanonicalPlayerPresentationLikeCpp,
    )> {
        self.canonical_at(guid, map_id, instance_id, |player| {
            (
                canonical_player_vitals_like_cpp(player),
                player.data().party_type,
                canonical_player_presentation_like_cpp(player),
            )
        })
    }
}
