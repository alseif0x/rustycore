//! Lazy DB2 and canonical Player projections for appearance admission.

use super::*;
use wow_entities::{
    AppearanceAdmissionSource, AppearanceModifiedFacts, AppearanceSearchFacts,
    AppearanceSparseFacts, AppearanceStorageFacts, PlayerCollectionStateLikeCpp,
};

impl WorldSession {
    /// Bounded C++ CollectionMgr::CanAddAppearance; the domain owns all gates.
    pub fn can_add_item_appearance_represented_like_cpp(
        &self,
        item_modified_appearance_id: u32,
    ) -> bool {
        PlayerCollectionStateLikeCpp::can_add_appearance(self, item_modified_appearance_id)
    }
}

impl AppearanceAdmissionSource for WorldSession {
    fn modified_appearance(&self, id: u32) -> Option<AppearanceModifiedFacts> {
        self.items
            .modified_appearance_store
            .as_ref()?
            .get(id)
            .map(|row| AppearanceModifiedFacts {
                item_id: row.item_id,
                transmog_source_type_enum: row.transmog_source_type_enum,
            })
    }

    fn search_name(&self, item_id: u32) -> Option<AppearanceSearchFacts> {
        self.items
            .search_name_store
            .as_ref()?
            .get(item_id)
            .map(|row| AppearanceSearchFacts {
                allowable_race: row.allowable_race,
                required_level: row.required_level,
                required_skill: row.required_skill,
                required_skill_rank: row.required_skill_rank,
                required_ability: row.required_ability,
            })
    }

    fn item_subclass(&self, item_id: u32) -> Option<u8> {
        self.items
            .store
            .as_ref()?
            .get(item_id)
            .map(|row| row.subclass_id)
    }

    fn sparse_template(&self, item_id: u32) -> Option<AppearanceSparseFacts> {
        self.items
            .stats_store
            .as_ref()?
            .sparse_template(item_id)
            .map(|row| AppearanceSparseFacts {
                flags: row.flags,
                required_reputation_faction: row.required_reputation_faction,
                required_reputation_rank: row.required_reputation_rank,
                allowable_class: row.allowable_class,
            })
    }

    fn storage_template(&self, item_id: u32) -> Option<AppearanceStorageFacts> {
        self.item_storage_template(item_id)
            .map(|row| AppearanceStorageFacts {
                class_id: row.class_id,
                inventory_type: row.inventory_type,
            })
    }

    fn has_player_guid(&self) -> bool {
        self.player_guid().is_some()
    }

    fn player_race(&self) -> u8 {
        self.player_race_like_cpp()
    }

    fn team_for_race(&self, race: u8) -> u32 {
        player_team_id_for_race_cpp(race)
    }

    fn player_level(&self) -> u8 {
        self.player_level_like_cpp()
    }

    fn player_skill(&self, skill: u16) -> Option<u16> {
        self.resolved_player_skill_value_like_cpp(skill)
    }

    fn knows_spell(&self, spell_id: i32) -> bool {
        self.known_spells_like_cpp().contains(&spell_id)
    }

    fn reputation_rank(&self, faction_id: u32) -> Option<u32> {
        self.represented_item_reputation_rank_like_cpp(faction_id)
    }

    fn learning_effects(&self, item_id: u32) -> Vec<(u8, i32)> {
        self.represented_item_effect_spell_ids_like_cpp(item_id)
    }

    fn player_class(&self) -> u8 {
        self.player_class_like_cpp()
    }

    fn class_mask(&self, class_id: u8) -> u32 {
        player_class_mask_for_transmog_like_cpp(class_id)
    }

    fn item_quality(&self, item_id: u32) -> Option<i8> {
        self.item_template_quality(item_id)
    }

    fn weapon_proficiency(&self) -> Option<u32> {
        self.represented_player_weapon_proficiency_like_cpp()
    }

    fn armor_class_mask(&self, subclass: u32) -> u32 {
        player_class_by_armor_subclass_like_cpp(subclass)
    }

    fn permanent_appearance_exists(&self, id: u32) -> Option<bool> {
        self.player_collection_state_snapshot_like_cpp()
            .map(|collections| collections.item_appearances_like_cpp().contains(&id))
    }
}
