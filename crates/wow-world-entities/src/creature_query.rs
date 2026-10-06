use wow_constants::PowerType;
use wow_core::ObjectGuid;
use wow_world_core::session::{
    HubMut, HubRef, RepresentedCreatureAccessLikeCpp, power_type_from_u8_like_cpp,
};

use crate::{CreatureCreateStatsLikeCpp, CreatureSpawnCatalogsLikeCpp, WorldEntitiesState};

impl WorldEntitiesState {
    #[allow(dead_code)]
    pub fn canonical_creature_access_like_cpp(
        &self,
        hub: HubRef<'_>,
        guid: ObjectGuid,
    ) -> Option<RepresentedCreatureAccessLikeCpp> {
        if guid.is_empty() || !guid.is_any_type_creature() {
            return None;
        }
        let map_key = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                hub.core.player_map_id_like_cpp(),
            ))?;
        let manager = hub.core.canonical_map_manager.as_ref()?;
        let Ok(manager) = manager.lock() else {
            return None;
        };
        let map = manager.find_map(map_key.map_id, map_key.instance_id)?;
        map.map()
            .with_creature_or_pet_like_cpp(guid, |creature, _| RepresentedCreatureAccessLikeCpp {
                entry: creature.entry(),
                position: creature.unit().world().position(),
                npc_flags: creature.ai_ownership().npc_flags,
                npc_flags2: creature.ai_ownership().npc_flags2,
                trainer_class: creature.trainer_class_like_cpp(),
                faction_template_id: creature.unit().data().faction_template.max(0) as u32,
            })
    }

    pub fn creature_create_stats_with_catalogs_like_cpp(
        &self,
        hub: HubRef<'_>,
        catalogs: &CreatureSpawnCatalogsLikeCpp,
        entry: u32,
        level: u8,
        unit_class: u8,
        classification: u32,
        regen_health: bool,
        db_cur_health: u32,
        db_cur_mana: u32,
    ) -> CreatureCreateStatsLikeCpp {
        let power_type = power_type_from_u8_like_cpp(
            hub.catalogs
                .creature_display_power_for_class_like_cpp(unit_class),
        );
        let difficulty = catalogs.difficulty.get_like_cpp(entry, 0);

        let base_stats = catalogs.base_stats.get_like_cpp(level, unit_class);
        let health_rate = catalogs
            .health_rates
            .modifier_for_classification_like_cpp(classification);
        let max_health = i64::from(
            (base_stats.generate_health_like_cpp(difficulty) as f32 * health_rate) as u32,
        );
        let health = if regen_health {
            max_health
        } else if db_cur_health != 0 {
            i64::from(((db_cur_health as f32 * health_rate) as u32).max(1))
        } else {
            0
        };

        let base_mana = i32::try_from(base_stats.base_mana).unwrap_or(i32::MAX);
        let initial_power = catalogs.power_types.creature_initial_power_like_cpp(
            power_type as i8,
            base_mana,
            difficulty.mana_modifier,
        );
        // C++ `Creature::SetSpawnHealth` only applies the `curmana` column through
        // `SetPower(POWER_MANA, ...)`. A Focus/Energy/Rage creature keeps the
        // default/full power seeded by `UpdateLevelDependantStats`.
        let power = if !regen_health && power_type == PowerType::Mana {
            i32::try_from(db_cur_mana).unwrap_or(i32::MAX)
        } else {
            initial_power.power
        };

        CreatureCreateStatsLikeCpp {
            health,
            max_health,
            power_type,
            power,
            max_power: initial_power.max_power,
            base_mana,
        }
    }

    #[allow(dead_code)]
    pub fn represented_mount_creature_template_fallback_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        creature_entry: u32,
    ) -> Option<(i32, u32)> {
        let template = hub
            .catalogs
            .creatures
            .template_mount_store
            .as_ref()?
            .get(creature_entry)?;
        let display_id = template
            .choose_display_id_like_cpp(&mut hub.core.driver.represented_runtime_rng_like_cpp)?;
        Some((i32::try_from(display_id).unwrap_or(0), template.vehicle_id))
    }
}
