#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterPetStableRowLikeCpp {
    pub pet_number: u32,
    pub creature_id: u32,
    pub display_id: u32,
    pub level: u8,
    pub experience: u32,
    pub react_state: u8,
    pub slot: i16,
    pub name: String,
    pub was_renamed: bool,
    pub health: u32,
    pub mana: u32,
    pub action_bar: String,
    pub last_save_time: u32,
    pub created_by_spell_id: u32,
    pub pet_type: u8,
    pub specialization_id: u16,
}

impl crate::session::HubRef<'_> {
    /// C++ can load/summon a `character_pet` during the Player lifetime and
    /// pet runtime can cast owner auras. Until those transitions are fully
    /// represented, admit only the complete empty-query state and revoke it
    /// on every represented pet load or mutation.
    pub fn represented_character_pet_aura_source_is_empty_like_cpp(
        &self,
    ) -> bool {
        let Some(pet_lifecycle) = self.player_pet_lifecycle_state_snapshot_like_cpp() else {
            return false;
        };
        pet_lifecycle.character_rows_empty_authority_complete
            && pet_lifecycle.temporary_unsummoned_pet_number == 0
            && self.player_pet_guid_state_like_cpp() == Some(None)
            && pet_lifecycle.stable.current_pet_index.is_none()
            && pet_lifecycle.stable.active_pets.is_empty()
            && pet_lifecycle.stable.stabled_pets.is_empty()
            && pet_lifecycle.stable.unslotted_pets.is_empty()
    }
}
