use super::{
    ACTIVE_PLAYER_DATA_NO_REAGENT_COST_MASK_FIRST_BIT,
    ACTIVE_PLAYER_DATA_NO_REAGENT_COST_MASK_PARENT_BIT, ACTIVE_PLAYER_DATA_SELF_RES_SPELLS_BIT,
    Player,
};
use crate::player_rules::set_dynamic_update_mask_index;
use crate::{
    PlayerResurrectionContinuation, PlayerResurrectionOperation, PlayerResurrectionPhase,
    PlayerResurrectionRequestLikeCpp, PlayerResurrectionStateLikeCpp,
};
use wow_core::ObjectGuid;

impl Player {
    pub fn resurrection_state_like_cpp(&self) -> &PlayerResurrectionStateLikeCpp {
        &self.gameplay_state.resurrection
    }

    pub fn resurrection_state_mut_like_cpp(&mut self) -> &mut PlayerResurrectionStateLikeCpp {
        &mut self.gameplay_state.resurrection
    }

    /// Append one C++ `ActivePlayerData::SelfResSpells` value.
    pub fn add_self_res_spell(&mut self, spell_id: i32) {
        let index = self.active_data.self_res_spells.len();
        self.active_data.self_res_spells.push(spell_id);
        set_dynamic_update_mask_index(&mut self.active_data.self_res_spells_update_mask, index);
        self.mark_active_player_data(ACTIVE_PLAYER_DATA_SELF_RES_SPELLS_BIT);
    }

    /// Remove the first matching value, preserving the order of the remaining
    /// C++ `DynamicUpdateField` entries.
    pub fn remove_self_res_spell(&mut self, spell_id: i32) -> bool {
        let Some(index) = self
            .active_data
            .self_res_spells
            .iter()
            .position(|value| *value == spell_id)
        else {
            return false;
        };

        let previous_len = self.active_data.self_res_spells.len();
        let mut update_mask = self
            .active_data
            .self_res_spells_update_mask
            .take()
            .unwrap_or_else(|| vec![u32::MAX; previous_len.div_ceil(32)]);
        update_mask.resize(previous_len.div_ceil(32), 0);
        self.active_data.self_res_spells.remove(index);

        let new_len = self.active_data.self_res_spells.len();
        if previous_len != 0 {
            let removed_last_index = previous_len - 1;
            update_mask[removed_last_index / 32] &= !(1 << (removed_last_index % 32));
        }
        for shifted_index in index..new_len {
            let block = shifted_index / 32;
            update_mask[block] |= 1 << (shifted_index % 32);
        }
        update_mask.truncate(new_len.div_ceil(32));
        self.active_data.self_res_spells_update_mask = Some(update_mask);
        self.mark_active_player_data(ACTIVE_PLAYER_DATA_SELF_RES_SPELLS_BIT);
        true
    }

    /// Clear the dynamic field and publish its empty size even when already empty.
    pub fn clear_self_res_spells(&mut self) {
        self.active_data.self_res_spells.clear();
        self.active_data.self_res_spells_update_mask = Some(Vec::new());
        self.mark_active_player_data(ACTIVE_PLAYER_DATA_SELF_RES_SPELLS_BIT);
    }

    pub fn self_res_spells(&self) -> &[i32] {
        &self.active_data.self_res_spells
    }

    /// Set C++ `ActivePlayerData::NoReagentCostMask` and dirty its native
    /// array update-field projection.
    pub fn set_no_reagent_cost_mask_like_cpp(&mut self, mask: [u32; 4]) {
        for (index, value) in mask.into_iter().enumerate() {
            if self.active_data.no_reagent_cost_mask[index] != value {
                self.active_data.no_reagent_cost_mask[index] = value;
                self.mark_active_player_data_array(
                    ACTIVE_PLAYER_DATA_NO_REAGENT_COST_MASK_PARENT_BIT,
                    ACTIVE_PLAYER_DATA_NO_REAGENT_COST_MASK_FIRST_BIT,
                    index,
                );
            }
        }
    }

    pub fn no_reagent_cost_mask_like_cpp(&self) -> [u32; 4] {
        self.active_data.no_reagent_cost_mask
    }

    /// C++ `Player::SetPvPDeath` (`Player.h:1147`): `PLAYER_EXTRA_PVP_DEATH`.
    pub fn set_pvp_death_like_cpp(&mut self, on: bool) {
        const PLAYER_EXTRA_PVP_DEATH: u32 = 0x0100;
        if on {
            self.extra_flags |= PLAYER_EXTRA_PVP_DEATH;
        } else {
            self.extra_flags &= !PLAYER_EXTRA_PVP_DEATH;
        }
    }

    pub fn set_resurrection_request_like_cpp(&mut self, request: PlayerResurrectionRequestLikeCpp) {
        self.gameplay_state.resurrection.request = Some(request);
    }

    pub fn clear_resurrection_request_like_cpp(&mut self) {
        self.gameplay_state.resurrection.request = None;
    }

    /// Atomically consume C++ `_resurrectionData` into the Player-owned
    /// delayed operation before its caller reaches the first async boundary.
    pub fn accept_resurrection_request(
        &mut self,
        resurrecter: ObjectGuid,
    ) -> Option<PlayerResurrectionContinuation> {
        let state = &mut self.gameplay_state.resurrection;
        if state.delayed_after_teleport.is_some()
            || !state.request.is_some_and(|request| {
                !request.resurrecter.is_empty() && request.resurrecter == resurrecter
            })
        {
            return None;
        }
        let request = state.request.take()?;
        let continuation = PlayerResurrectionContinuation {
            operation: PlayerResurrectionOperation::RequestData(request),
            phase: PlayerResurrectionPhase::TeleportInFlight,
        };
        state.delayed_after_teleport = Some(continuation);
        Some(continuation)
    }

    /// Admit one complete native resurrection or corpse-recovery operation.
    /// Existing work remains the sole owner until it is explicitly completed.
    pub fn begin_resurrection_operation(
        &mut self,
        operation: PlayerResurrectionOperation,
        phase: PlayerResurrectionPhase,
    ) -> bool {
        let state = &mut self.gameplay_state.resurrection;
        if state.delayed_after_teleport.is_some() {
            return false;
        }
        state.delayed_after_teleport = Some(PlayerResurrectionContinuation { operation, phase });
        true
    }

    /// Atomically update operation-owned facts and advance its phase.
    pub fn update_resurrection_operation(
        &mut self,
        expected: PlayerResurrectionPhase,
        operation: PlayerResurrectionOperation,
        next: PlayerResurrectionPhase,
    ) -> bool {
        let Some(continuation) = self
            .gameplay_state
            .resurrection
            .delayed_after_teleport
            .as_mut()
        else {
            return false;
        };
        if continuation.phase != expected {
            return false;
        }
        continuation.operation = operation;
        continuation.phase = next;
        true
    }

    pub fn advance_resurrection_continuation(
        &mut self,
        expected: PlayerResurrectionPhase,
        next: PlayerResurrectionPhase,
    ) -> bool {
        let Some(continuation) = self
            .gameplay_state
            .resurrection
            .delayed_after_teleport
            .as_mut()
        else {
            return false;
        };
        if continuation.phase != expected {
            return false;
        }
        continuation.phase = next;
        true
    }

    pub fn clear_completed_resurrection_continuation(&mut self) -> bool {
        let state = &mut self.gameplay_state.resurrection;
        if !state
            .delayed_after_teleport
            .is_some_and(|continuation| {
                continuation.phase == PlayerResurrectionPhase::Complete
            })
        {
            return false;
        }
        state.delayed_after_teleport = None;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_core::Position;

    #[test]
    fn player_owns_resurrection_lifecycle_like_cpp() {
        let mut player = Player::new(Some(1), false);
        let resurrecter = ObjectGuid::create_player(1, 77);
        let request = PlayerResurrectionRequestLikeCpp {
            resurrecter,
            map_id: 571,
            position: Position::new(11.0, 22.0, 33.0, 1.5),
            health: 450,
            mana: 120,
            aura: 0,
        };

        player.set_resurrection_request_like_cpp(request);
        player.add_self_res_spell(21169);
        player
            .resurrection_state_mut_like_cpp()
            .death_timer_remaining_ms = 360_000;
        player
            .resurrection_state_mut_like_cpp()
            .area_spirit_healer_guid = ObjectGuid::create_player(1, 88);

        assert_eq!(player.resurrection_state_like_cpp().request, Some(request));
        assert_eq!(player.self_res_spells(), &[21169]);
        assert_eq!(
            player
                .resurrection_state_like_cpp()
                .death_timer_remaining_ms,
            360_000
        );
    }

    #[test]
    fn no_reagent_mask_is_native_active_data_with_array_dirty_projection() {
        let mut player = Player::new(Some(1), false);
        player.clear_active_player_data_changes();
        let mask = [1, 0x20, 0, u32::MAX];
        player.set_no_reagent_cost_mask_like_cpp(mask);

        assert_eq!(player.no_reagent_cost_mask_like_cpp(), mask);
        fn changed_bits(player: &Player) -> Vec<usize> {
            let blocks = player.active_player_data_changes_mask().blocks();
            (0..blocks.len() * 32)
                .filter(|bit| blocks[bit / 32] & (1 << (bit % 32)) != 0)
                .collect::<Vec<_>>()
        }
        assert_eq!(changed_bits(&player), vec![615, 616, 617, 619]);

        player.clear_active_player_data_changes();
        player.set_no_reagent_cost_mask_like_cpp(mask);
        assert!(!player.active_player_data_changes_mask().is_any_set());
        player.set_no_reagent_cost_mask_like_cpp([0; 4]);
        assert_eq!(changed_bits(&player), vec![615, 616, 617, 619]);
        player.clear_active_player_data_changes();
        player.set_no_reagent_cost_mask_like_cpp([0; 4]);
        assert!(!player.active_player_data_changes_mask().is_any_set());
    }

    #[test]
    fn accepted_request_moves_atomically_into_teleport_continuation() {
        let mut player = Player::new(Some(1), false);
        let request = PlayerResurrectionRequestLikeCpp {
            resurrecter: ObjectGuid::create_player(1, 77),
            map_id: 571,
            position: Position::new(11.0, 22.0, 33.0, 1.5),
            health: 450,
            mana: 120,
            aura: 21169,
        };
        player.set_resurrection_request_like_cpp(request);
        assert_eq!(
            player.accept_resurrection_request(ObjectGuid::create_player(1, 78)),
            None
        );
        let continuation = player
            .accept_resurrection_request(request.resurrecter)
            .expect("request accepted");
        assert_eq!(
            continuation,
            PlayerResurrectionContinuation {
                operation: PlayerResurrectionOperation::RequestData(request),
                phase: PlayerResurrectionPhase::TeleportInFlight,
            }
        );
        let state = player.resurrection_state_like_cpp();
        assert!(state.request.is_none());
        assert_eq!(state.delayed_after_teleport, Some(continuation));
        assert!(player
            .accept_resurrection_request(request.resurrecter)
            .is_none());
    }

}
