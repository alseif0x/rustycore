//! Creature lookup, catalogs and nearby search for the represented Session.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    #[allow(dead_code)]
    pub(crate) fn canonical_creature_access_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<RepresentedCreatureAccessLikeCpp> {
        let (state, hub) = crate::session::split_world_entities_ref(self);
        state.canonical_creature_access_like_cpp(hub, guid)
    }
    /// Nearby creature candidates for one represented player position.
    ///
    /// #1263 F6-8B/F6-8E. The canonical creature on the canonical map is the
    /// one authority for the movement/visibility transition, so the
    /// spline-bearing candidate set is selected from it, with its own phase and
    /// `CanSeeOrDetect` filtering.
    pub(crate) fn visible_world_creatures_from_map_like_cpp(
        &self,
        map_id: u16,
        position: &wow_core::Position,
    ) -> Vec<wow_entities::Creature> {
        if self
            .core
            .current_canonical_player_map_key_like_cpp()
            .is_some_and(|key| key.map_id != u32::from(map_id))
        {
            return Vec::new();
        }

        // These candidates carry the canonical runtime state, including the
        // live `Unit::movespline` a create block projects.
        self.visible_creatures_from_canonical_map_like_cpp(map_id, position)
            .unwrap_or_default()
    }
    pub(crate) fn visible_creatures_from_canonical_map_like_cpp(
        &self,
        map_id: u16,
        position: &wow_core::Position,
    ) -> Option<Vec<wow_entities::Creature>> {
        let requested_map_id = u32::from(map_id);
        let player_map_key = self.core.current_canonical_player_map_key_like_cpp();
        let player_map_key = player_map_key?;
        if player_map_key.map_id != requested_map_id {
            return Some(Vec::new());
        }
        let source_combat_reach = self
            .core
            .with_owned_player_like_cpp(|player| player.unit().world().combat_reach())?;
        let player_phase_shift = self.represented_player_phase_shift_like_cpp()?;
        let manager = self.core.canonical_map_manager.as_ref()?;
        let Ok(manager) = manager.lock() else {
            return None;
        };
        let map = manager.find_map(player_map_key.map_id, player_map_key.instance_id)?;
        let visibility_range = self.config.player_map_visibility_range_like_cpp(map_id);
        let nearby = map.map().nearby_cell_guids_like_cpp(
            position.x,
            position.y,
            visibility_range + source_combat_reach,
        );

        let mut candidates = Vec::new();
        for guid in nearby
            .world
            .creatures
            .into_iter()
            .chain(nearby.grid.creatures)
        {
            let Some(creature) = map
                .map()
                .with_creature_or_pet_like_cpp(guid, |creature, _owner| creature.clone())
            else {
                continue;
            };
            let world = creature.unit().world();
            if !world.object().is_in_world()
                || world.map_id() != requested_map_id
                // C++ visibility is a horizontal (XY) range test: CanSeeOrDetect ->
                // IsWithinDist(obj, GetSightRange, is3D=false) (Object.cpp:1587-1609).
                // #NEXT.R8.ENTITIES.1223 — use 2D so vertically-separated objects (e.g. ICC
                // layered floors) within horizontal range are not dropped.
                || !wow_core::visibility_distance_allows_like_cpp(
                    position,
                    source_combat_reach,
                    &world.position(),
                    world.combat_reach(),
                    visibility_range,
                )
                || !player_phase_shift.can_see(world.phase_shift())
            {
                continue;
            }
            candidates.push(creature.clone());
        }
        drop(manager);

        self.core.with_owned_player_like_cpp(move |player| {
            candidates
                .into_iter()
                .filter(|creature| {
                    player.unit().can_see_or_detect_unit_like_cpp(
                        creature.unit(),
                        false,
                        true,
                        false,
                    )
                })
                .map(|creature| {
                    // #1263 F6-8B: the visibility candidate must carry the
                    // canonical runtime state, because the create projection at
                    // the call site reads the live `Unit::movespline` from it.
                    // The caller computes that projection on demand (the
                    // runtime installation changes no field it reads).
                    let create_data =
                        crate::creature_projection::creature_create_data_like_cpp(&creature);
                    crate::creature_projection::projected_creature_preserving_runtime_like_cpp(
                        creature,
                        &create_data,
                    )
                })
                .collect()
        })
    }
    pub fn set_creature_onkill_reputation_store(
        &mut self,
        store: Arc<CreatureOnKillReputationStoreLikeCpp>,
    ) {
        self.catalogs.creatures.onkill_reputation_store = Some(store);
    }
    pub fn set_creature_template_mount_store(
        &mut self,
        store: Arc<CreatureTemplateMountStoreLikeCpp>,
    ) {
        self.catalogs.creatures.template_mount_store = Some(store);
    }
    pub fn set_creature_template_lifecycle_store_like_cpp(
        &mut self,
        store: Arc<CreatureTemplateLifecycleStoreLikeCpp>,
    ) {
        self.catalogs.creatures.template_lifecycle_store_like_cpp = Some(store);
    }
    pub fn set_creature_display_info_store(&mut self, store: Arc<CreatureDisplayInfoStore>) {
        self.catalogs.creatures.display_info_store = Some(store);
    }
    pub fn set_creature_display_info_extra_store(
        &mut self,
        store: Arc<CreatureDisplayInfoExtraStore>,
    ) {
        self.catalogs.creatures.display_info_extra_store = Some(store);
    }
    pub fn set_creature_model_info_store(
        &mut self,
        store: Arc<wow_data::CreatureModelInfoStoreLikeCpp>,
    ) {
        self.catalogs.creatures.model_info_store = Some(store);
    }
    #[cfg(test)]
    pub fn set_creature_addon_store_like_cpp(&mut self, store: Arc<CreatureAddonStoreLikeCpp>) {
        self.catalogs.creature_addon_store_like_cpp = Some(store);
    }
    #[cfg(test)]
    pub(crate) fn creature_addon_store_like_cpp(&self) -> Option<&Arc<CreatureAddonStoreLikeCpp>> {
        self.catalogs.creature_addon_store_like_cpp.as_ref()
    }
    #[cfg(test)]
    pub(crate) fn set_creature_difficulty_store_like_cpp(
        &mut self,
        store: Arc<CreatureDifficultyStoreLikeCpp>,
    ) {
        self.catalogs.creature_difficulty_store_like_cpp = Some(store);
    }
    #[cfg(test)]
    pub(crate) fn set_creature_base_stats_store_like_cpp(
        &mut self,
        store: Arc<CreatureBaseStatsStoreLikeCpp>,
    ) {
        self.catalogs.creature_base_stats_store_like_cpp = Some(store);
    }
    pub(crate) fn creature_create_stats_with_catalogs_like_cpp(
        &self,
        catalogs: &CreatureSpawnCatalogsLikeCpp,
        entry: u32,
        level: u8,
        unit_class: u8,
        classification: u32,
        regen_health: bool,
        db_cur_health: u32,
        db_cur_mana: u32,
    ) -> CreatureCreateStatsLikeCpp {
        let (state, hub) = crate::session::split_world_entities_ref(self);
        state.creature_create_stats_with_catalogs_like_cpp(
            hub,
            catalogs,
            entry,
            level,
            unit_class,
            classification,
            regen_health,
            db_cur_health,
            db_cur_mana,
        )
    }
    pub fn set_creature_model_data_store(&mut self, store: Arc<CreatureModelDataStore>) {
        self.catalogs.creatures.model_data_store = Some(store);
    }
}
