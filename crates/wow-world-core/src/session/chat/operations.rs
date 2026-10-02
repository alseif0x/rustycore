use crate::entity_update_bridge::player_values_update_to_update_object;
use crate::session::state::SessionCore;
use wow_entities::{
    UNIT_DATA_BITS, UNIT_DATA_EMOTE_STATE_BIT, UNIT_DATA_MODS_PARENT_BIT, UnitDataUpdate,
    UnitDataValues, UpdateMask,
};

impl SessionCore {
    pub fn player_emote_state_update_packet_like_cpp(
        &self,
        emote_state: u32,
    ) -> Option<wow_packet::packets::update::UpdateObject> {
        let guid = self.player_guid()?;
        let mut mask = UpdateMask::new(UNIT_DATA_BITS);
        mask.set(UNIT_DATA_MODS_PARENT_BIT);
        mask.set(UNIT_DATA_EMOTE_STATE_BIT);
        let update = wow_entities::PlayerValuesUpdate {
            changed_object_type_mask: 0,
            object_data: None,
            unit_data: Some(UnitDataUpdate {
                mask,
                values: UnitDataValues {
                    emote_state: emote_state.min(i32::MAX as u32) as i32,
                    ..Default::default()
                },
            }),
            player_data: None,
            active_player_data: None,
        };
        player_values_update_to_update_object(guid, self.player_map_id_like_cpp(), &update)
    }
}
