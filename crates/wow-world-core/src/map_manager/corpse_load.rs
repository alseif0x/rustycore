use std::collections::{BTreeSet, HashMap};

use wow_core::guid::HighGuid;
use wow_core::{ObjectGuid, Position};
use wow_entities::{Corpse, CorpseCustomizationChoice, CorpseType};
use wow_map::Map;

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedMapCorpseRowLikeCpp {
    pub position: Position,
    pub map_id: u16,
    pub display_id: u32,
    pub items: [u32; wow_entities::CORPSE_ITEMS],
    pub race: u8,
    pub class: u8,
    pub sex: u8,
    pub flags: u32,
    pub dynamic_flags: u32,
    pub ghost_time: i64,
    pub corpse_type: CorpseType,
    pub instance_id: u32,
    pub owner_db_guid: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MapCorpseLoadOutcomeLikeCpp {
    pub already_loaded: bool,
    pub rows_seen: u32,
    pub corpses_added: u32,
    pub invalid_type_rows: u32,
    pub invalid_race_rows: u32,
    pub invalid_position_rows: u32,
    pub add_to_map_errors: u32,
}

pub fn parse_corpse_items_like_cpp(item_cache: &str) -> [u32; wow_entities::CORPSE_ITEMS] {
    let mut items = [0; wow_entities::CORPSE_ITEMS];
    let tokens = item_cache.split_whitespace().collect::<Vec<_>>();
    if tokens.len() == items.len() {
        for (slot, token) in tokens.into_iter().enumerate() {
            items[slot] = token.parse().unwrap_or(0);
        }
    }
    items
}

pub fn materialize_loaded_map_corpses_like_cpp(
    map: &mut Map,
    realm_id: u16,
    rows: Vec<LoadedMapCorpseRowLikeCpp>,
    phases: &HashMap<u64, BTreeSet<u32>>,
    customizations: &HashMap<u64, Vec<CorpseCustomizationChoice>>,
    faction_templates_by_race: &HashMap<u8, i32>,
) -> MapCorpseLoadOutcomeLikeCpp {
    if map.corpse_data_loaded_like_cpp() {
        return MapCorpseLoadOutcomeLikeCpp {
            already_loaded: true,
            ..Default::default()
        };
    }

    let mut outcome = MapCorpseLoadOutcomeLikeCpp::default();
    for row in rows {
        outcome.rows_seen = outcome.rows_seen.saturating_add(1);
        // C++ `Map::LoadCorpseData` consumes the map-local counter when it
        // calls `LoadCorpseFromDB(GenerateLowGuid(), fields)`. The latter only
        // validates map coordinates near the end, so even a rejected position
        // has already advanced the GUID generator.
        let Ok(low_guid) = map.generate_low_guid_like_cpp(HighGuid::Corpse) else {
            outcome.add_to_map_errors = outcome.add_to_map_errors.saturating_add(1);
            continue;
        };
        if row.map_id != map.map_id() as u16 || row.instance_id != map.instance_id() {
            outcome.add_to_map_errors = outcome.add_to_map_errors.saturating_add(1);
            continue;
        }
        if !row.position.is_valid_map_coord_like_cpp() {
            outcome.invalid_position_rows = outcome.invalid_position_rows.saturating_add(1);
            continue;
        }
        let Some(faction_template) = faction_templates_by_race.get(&row.race).copied() else {
            outcome.invalid_race_rows = outcome.invalid_race_rows.saturating_add(1);
            continue;
        };

        let mut corpse = Corpse::new_at(row.corpse_type, row.ghost_time);
        let corpse_guid = ObjectGuid::create_world_object(
            HighGuid::Corpse,
            0,
            realm_id,
            row.map_id,
            0,
            0,
            low_guid,
        );
        corpse.world_mut().object_mut().create(corpse_guid);
        if corpse
            .world_mut()
            .set_map(u32::from(row.map_id), row.instance_id)
            .is_err()
        {
            outcome.add_to_map_errors = outcome.add_to_map_errors.saturating_add(1);
            continue;
        }
        corpse.world_mut().relocate(row.position);
        corpse.set_display_id(row.display_id);
        corpse.set_race(row.race);
        corpse.set_class(row.class);
        corpse.set_sex(row.sex);
        corpse.replace_all_flags(row.flags);
        corpse.replace_all_corpse_dynamic_flags(row.dynamic_flags);
        corpse.set_owner_guid(ObjectGuid::create_player(
            realm_id,
            row.owner_db_guid as i64,
        ));
        corpse.set_faction_template(faction_template);
        for (slot, item) in row.items.into_iter().enumerate() {
            corpse.set_item(slot, item);
        }
        for phase_id in phases
            .get(&row.owner_db_guid)
            .into_iter()
            .flatten()
            .copied()
        {
            corpse.world_mut().phase_shift_mut().insert(phase_id);
        }
        corpse.set_customizations(
            customizations
                .get(&row.owner_db_guid)
                .cloned()
                .unwrap_or_default(),
        );

        // C++ loads these fields before AddCorpse/AddToMap, so they form the
        // clean baseline rather than a later VALUES delta.
        corpse.clear_corpse_data_changes();
        corpse.world_mut().object_mut().clear_update_mask(false);
        match map.register_loaded_corpse_like_cpp(corpse) {
            Ok(_) => outcome.corpses_added = outcome.corpses_added.saturating_add(1),
            Err(_) => {
                outcome.add_to_map_errors = outcome.add_to_map_errors.saturating_add(1);
            }
        }
    }

    map.mark_corpse_data_loaded_like_cpp();
    outcome
}
