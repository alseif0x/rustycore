//! These fixtures hold one actual Player and borrowed DB2 catalogs.
//! They project rows into the admission port; no appearance rule is reimplemented.

use super::*;

pub(super) struct AdmissionFixture {
    pub(super) player: Player,
    pub(super) modified: Option<Arc<ItemModifiedAppearanceStore>>,
    pub(super) items: Option<Arc<ItemStore>>,
    pub(super) search: Option<Arc<ItemSearchNameStore>>,
    pub(super) stats: Option<Arc<ItemStatsStore>>,
    pub(super) effects: Option<Arc<ItemEffectStore>>,
    pub(super) factions: Option<Arc<FactionStore>>,
    pub(super) faction_templates: Option<Arc<FactionTemplateStore>>,
    pub(super) sets: Option<Arc<TransmogSetItemStore>>,
}

impl Default for AdmissionFixture {
    fn default() -> Self {
        Self {
            player: Player::new(Some(1), false),
            modified: None,
            items: None,
            search: None,
            stats: None,
            effects: None,
            factions: None,
            faction_templates: None,
            sets: None,
        }
    }
}

impl AdmissionFixture {
    pub(super) fn install_player(
        &mut self,
        guid: ObjectGuid,
        name: String,
        position: Position,
        map_id: u32,
        race: u8,
        class_id: u8,
        level: u8,
        gender: u8,
    ) {
        self.player.unit_mut().world_mut().object_mut().create(guid);
        self.player.unit_mut().world_mut().set_name(&name);
        self.player.unit_mut().world_mut().set_map(map_id, 0).unwrap();
        self.player.unit_mut().world_mut().relocate(position);
        self.install_identity(map_id as u16, race, class_id, level, gender);
    }

    pub(super) fn install_identity(
        &mut self,
        _map_id: u16,
        race: u8,
        class_id: u8,
        level: u8,
        gender: u8,
    ) {
        self.player.set_race_class_gender(
            race,
            class_id,
            <Gender as num_traits::FromPrimitive>::from_u8(gender).unwrap(),
        );
        self.player.set_level_and_gray_level_like_cpp(level, 0);
    }

    pub(super) fn add_appearance(&mut self, id: u32) -> Option<crate::PlayerValuesUpdate> {
        let (index, flag) = PlayerCollectionStateLikeCpp::permanent_appearance_flag(id)?;
        let had_temporary = self.player.gameplay_state().collections
            .has_temporary_item_appearance_like_cpp(id);
        let result = PlayerCollectionStateLikeCpp::apply_permanent_appearance_fields(
            &mut self.player, id, had_temporary, index, flag,
        )?;
        self.player.gameplay_state_mut().collections.add_item_appearance_like_cpp(id);
        Some(result)
    }

    pub(super) fn load_appearances(
        &mut self,
        rows: impl IntoIterator<Item = (u32, u32)>,
        favorites: impl IntoIterator<Item = u32>,
    ) {
        let (blocks, ids, dense, highest) =
            PlayerCollectionStateLikeCpp::prepare_appearance_blocks(rows);
        if let Some(highest) = highest {
            PlayerCollectionStateLikeCpp::apply_loaded_appearance_fields(
                &mut self.player, highest, &blocks,
            );
        }
        let favorites = PlayerCollectionStateLikeCpp::loaded_appearance_favorites(favorites);
        self.player.gameplay_state_mut().collections.install_appearance_collection_like_cpp(
            ids, dense, favorites,
        );
    }

    pub(super) fn load_illusions(&mut self, rows: impl IntoIterator<Item = (u32, u32)>) {
        let ids = PlayerCollectionStateLikeCpp::loaded_illusion_ids(rows);
        self.player.gameplay_state_mut().collections.replace_transmog_illusions_like_cpp(ids);
    }

    pub(super) fn set_complete(&self, set_id: u32) -> bool {
        PlayerCollectionStateLikeCpp::transmog_set_complete(
            self.sets.as_ref().and_then(|store| store.get_transmog_set_items_like_cpp(set_id))
                .map(|rows| rows.iter().map(|row| row.item_modified_appearance_id)),
            |id| self.modified.as_ref()?.get(id).map(|row| row.item_id),
            |id| self.items.as_ref()?.inventory_type(id),
            |id| self.player.gameplay_state().collections.has_appearance(id),
        )
    }
}

impl AppearanceAdmissionSource for AdmissionFixture {
    fn modified_appearance(&self, id: u32) -> Option<AppearanceModifiedFacts> {
        self.modified.as_ref()?.get(id).map(|row| AppearanceModifiedFacts {
            item_id: row.item_id,
            transmog_source_type_enum: row.transmog_source_type_enum,
        })
    }
    fn search_name(&self, id: u32) -> Option<AppearanceSearchFacts> {
        self.search.as_ref()?.get(id).map(|row| AppearanceSearchFacts {
            allowable_race: row.allowable_race,
            required_level: row.required_level,
            required_skill: row.required_skill,
            required_skill_rank: row.required_skill_rank,
            required_ability: row.required_ability,
        })
    }
    fn item_subclass(&self, id: u32) -> Option<u8> {
        self.items.as_ref()?.get(id).map(|row| row.subclass_id)
    }
    fn sparse_template(&self, id: u32) -> Option<AppearanceSparseFacts> {
        self.stats.as_ref()?.sparse_template(id).map(|row| AppearanceSparseFacts {
            flags: row.flags,
            required_reputation_faction: row.required_reputation_faction,
            required_reputation_rank: row.required_reputation_rank,
            allowable_class: row.allowable_class,
        })
    }
    fn storage_template(&self, id: u32) -> Option<AppearanceStorageFacts> {
        let basic = self.items.as_ref()?.get(id)?;
        let sparse = self.stats.as_ref()?.sparse_template(id)?;
        // The same typed conversions as the application's ItemStorageTemplate projection.
        <ItemBondingType as num_traits::FromPrimitive>::from_u8(sparse.bonding)?;
        Some(AppearanceStorageFacts {
            class_id: <ItemClass as num_traits::FromPrimitive>::from_u8(basic.class_id)?,
            inventory_type: <InventoryType as num_traits::FromPrimitive>::from_i8(
                sparse.inventory_type,
            )?,
        })
    }
    fn has_player_guid(&self) -> bool {
        !self.player.guid().is_empty()
    }
    fn player_race(&self) -> u8 {
        self.player.race_like_cpp()
    }
    fn team_for_race(&self, race: u8) -> u32 {
        // All five original scenarios use human race 1: the APP selector returns 0.
        assert_eq!(race, 1);
        wow_constants::Team::Alliance as u32
    }
    fn player_level(&self) -> u8 {
        self.player.level_like_cpp()
    }
    fn player_skill(&self, skill: u16) -> Option<u16> {
        Some(
            self.player.skill_records_like_cpp().iter()
                .find(|row| row.skill_line_id == u32::from(skill))
                .map(|row| row.current_value)
                .unwrap_or(0),
        )
    }
    fn knows_spell(&self, id: i32) -> bool {
        self.player.gameplay_state().spells.known_spells_like_cpp().contains(&id)
    }
    fn reputation_rank(&self, faction_id: u32) -> Option<u32> {
        let Some(faction) = self.factions.as_ref().and_then(|store| store.get(faction_id)) else {
            return Some(0);
        };
        // The original reputation scenario supplies only this zero-base, non-friendship row.
        // Project its actual unloaded Player standing through the canonical rank helper.
        assert_eq!(faction.reputation_base, [0; 4]);
        assert_eq!(faction.friendship_rep_id, 0);
        let standing = self.player.reputation_like_cpp()
            .faction_like_cpp(faction.reputation_index as u32)
            .map(|state| state.standing).unwrap_or(0);
        Some(u32::from(
            wow_constants::reputation::reputation_rank_from_standing_like_cpp(standing).as_u8(),
        ))
    }
    fn learning_effects(&self, id: u32) -> Vec<(u8, i32)> {
        let mut rows = self.effects.as_ref().map(|store| store.values()
            .filter(|row| row.parent_item_id == id)
            .map(|row| (row.legacy_slot_index, row.spell_id)).collect::<Vec<_>>())
            .unwrap_or_default();
        rows.sort_by_key(|(slot, _)| *slot);
        rows
    }
    fn player_class(&self) -> u8 {
        self.player.class_like_cpp()
    }
    fn class_mask(&self, id: u8) -> u32 {
        PlayerCollectionStateLikeCpp::appearance_class_mask(id)
    }
    fn item_quality(&self, id: u32) -> Option<i8> {
        self.stats.as_ref()?.random_property_template(id).map(|row| row.quality)
    }
    fn weapon_proficiency(&self) -> Option<u32> {
        Some(self.player.weapon_proficiency_like_cpp())
    }
    fn armor_class_mask(&self, subclass: u32) -> u32 {
        PlayerCollectionStateLikeCpp::appearance_armor_class_mask(subclass)
    }
    fn permanent_appearance_exists(&self, id: u32) -> Option<bool> {
        Some(self.player.gameplay_state().collections.item_appearances_like_cpp().contains(&id))
    }
}

mod item_rows;
pub(super) use item_rows::*;

pub(super) fn faction_template_entry(
    id: u32, faction: u16, faction_group: u8, friend_group: u8, enemy: u16,
) -> wow_data::progression_rewards::FactionTemplateEntry {
    let mut enemies = [0; 8];
    enemies[0] = enemy;
    wow_data::progression_rewards::FactionTemplateEntry {
        id, faction, flags: 0, faction_group, friend_group, enemy_group: 0,
        enemies, friend: [0; 8],
    }
}
