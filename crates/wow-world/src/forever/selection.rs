//! CharacterPackets.cpp::CharacterInfoBasic + CharacterHandler.cpp::HandleCharEnum,
//! target 02245dcd. Pure row projection; Session owns the authorization set.
use super::{SessionError, appearance};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    sync::Arc,
};
use wow_core::{ObjectGuid, Position};
use wow_data::{
    forever_appearance::AppearanceCatalog, forever_initialization::InitializationCatalog,
};
use wow_packet::forever::{
    character_create::CustomizationChoice,
    character_list::{
        CharacterInfo, CharacterInfoBasic, CharacterRestrictionAndMailData, CustomTabardInfo,
        VisualItemInfo,
    },
};
use wow_persistence::forever::{
    LoadError,
    selection::{CharacterRow, SelectionRows},
};

/// Immutable references to the sole effective stores, not copied records.
pub struct SelectionPolicy {
    pub appearance: Arc<AppearanceCatalog>,
    pub initialization: Arc<InitializationCatalog>,
    pub declined_names: bool,
    pub super_district: i32,
    pub class_disable_mask: u32,
}

pub(super) struct Projection {
    pub characters: Vec<CharacterInfo>,
    pub legitimate: HashSet<ObjectGuid>,
    pub recustomize: Vec<u64>,
    pub max_level: i32,
}

pub fn super_district_for_content_set(content: u32, fallback: i32) -> i32 {
    match content {
        136 => 1,
        137 => 2,
        138 => 3,
        140 => 4,
        _ => fallback,
    }
}

pub(super) fn project(
    rows: SelectionRows,
    realm_address: u32,
    policy: &SelectionPolicy,
) -> Result<Projection, SessionError> {
    let mut choices = BTreeMap::<u64, Vec<CustomizationChoice>>::new();
    for row in rows.customizations {
        choices
            .entry(row.guid)
            .or_default()
            .push(CustomizationChoice {
                option_id: row.option,
                choice_id: row.choice,
            });
    }
    let mut projection = Projection {
        characters: Vec::new(),
        legitimate: HashSet::new(),
        recustomize: Vec::new(),
        max_level: 1,
    };
    let mut seen = HashSet::new();
    // Source truncates the local query result at 200, not a SQL LIMIT/order.
    for row in rows.characters.into_iter().take(200) {
        if row.guid == 0 || row.guid >= (1 << 40) || !seen.insert(row.guid) {
            return Err(SessionError::Persistence(LoadError::InvalidRow));
        }
        let counter = row.guid;
        let mut basic = basic(row, realm_address, policy)?;
        basic.customizations = choices.remove(&counter).unwrap_or_default();
        // load_account currently admits only empty persisted collections.
        if !appearance::validate_appearance(
            &policy.appearance,
            basic.race_id,
            basic.class_id,
            basic.sex_id,
            &basic.customizations,
            &BTreeSet::new(),
        ) {
            basic.customizations.clear();
            if basic.flags2 & (1 | 0x10000 | 0x100000) == 0 {
                projection.recustomize.push(counter);
                // Source assignment intentionally discards other Flags2 bits.
                basic.flags2 = 1;
            }
        }
        if basic.flags & (4 | 0x01000000) == 0 {
            projection.legitimate.insert(basic.guid);
        }
        projection.max_level = projection.max_level.max(i32::from(basic.experience_level));
        projection.characters.push(CharacterInfo::new(
            basic,
            CharacterRestrictionAndMailData::empty(),
        ));
    }
    Ok(projection)
}

fn basic(
    row: CharacterRow,
    realm_address: u32,
    policy: &SelectionPolicy,
) -> Result<CharacterInfoBasic, SessionError> {
    let realm = realm_address & 0xFFFF;
    // ObjectGuidFactory::CreatePlayer/Guild uses the realm unmasked. Legacy
    // helpers mask to 13 bits; do not silently reuse their different contract.
    let mut basic = CharacterInfoBasic::new(row.name, row.surname);
    basic.guid = ObjectGuid::new(
        ((2_u64 << 58) | (u64::from(realm) << 42)) as i64,
        row.guid as i64,
    );
    basic.virtual_realm_address = realm_address;
    basic.guild_club_member_id = row.guid | ((u64::from(realm) & 0xFFF) << 48);
    if row.guild != 0 {
        basic.guild_guid = ObjectGuid::new(
            ((28_u64 << 58) | (u64::from(realm) << 42)) as i64,
            row.guild as i64,
        );
    }
    basic.race_id = row.race;
    basic.class_id = row.class;
    basic.sex_id = row.gender;
    basic.experience_level = row.level;
    basic.zone_id = i32::from(row.zone);
    basic.map_id = i32::from(row.map);
    basic.preload_position = Position::new(row.position[0], row.position[1], row.position[2], 0.0);
    let mut player_flags = row.player_flags;
    if player_flags & 0x20 != 0 {
        basic.flags |= 2;
    }
    if row.at_login & 4 != 0 {
        basic.flags |= 0x100;
    }
    if row.at_login & 0x100 != 0 {
        player_flags &= !0x10;
    }
    if player_flags & 0x10 != 0 {
        basic.flags |= 0x2000;
    }
    if row.at_login & 1 != 0 {
        basic.flags |= 0x4000;
    }
    if row.active_ban_guid != 0 {
        basic.flags |= 0x01000000;
    }
    if policy.declined_names
        && row
            .declined_genitive
            .as_ref()
            .is_some_and(|name| !name.is_empty())
    {
        basic.flags |= 0x02000000;
    }
    basic.flags2 = if row.at_login & 8 != 0 {
        1
    } else if row.at_login & 0x40 != 0 {
        0x10000
    } else if row.at_login & 0x80 != 0 {
        0x100000
    } else {
        0
    };
    if player_flags & 0x02000000 != 0 {
        basic.flags2 |= 0x40000;
    }
    if player_flags & 0x10000 != 0 {
        basic.flags2 |= 0x20000000;
    }
    if player_flags & 0x08000000 != 0 {
        basic.flags2 |= 0x40000000;
    }
    if player_flags & 0x00800000 != 0 {
        basic.flags3 |= 2;
    }
    if player_flags & 0x800 != 0 {
        basic.flags3 |= 0x08000000;
    }
    basic.first_login = row.at_login & 0x20 != 0;
    if player_flags & 0x10 == 0 && matches!(row.class, 3 | 6 | 9) && row.pet_entry != 0 {
        // Effective CreatureTemplate lookup/family is not ported here. A raw
        // pet row is NOT proof of an admitted template, nor proof of absence.
        return Err(SessionError::Persistence(LoadError::UnsupportedState));
    }
    basic.list_position = u16::from(row.slot);
    basic.create_time = row.create_time;
    basic.last_active_time = row.logout_time;
    if let Some(spec) = policy
        .initialization
        .specialization(row.class, row.active_talent_group)
    {
        // Source writes uint16 truncation even though the DB2 ID is uint32.
        basic.spec_id = spec.id as u16;
    }
    basic.last_login_version = row.last_login_build;
    basic.personal_tabard = CustomTabardInfo {
        emblem_style: row.personal_tabard[0],
        emblem_color: row.personal_tabard[1],
        border_style: row.personal_tabard[2],
        border_color: row.personal_tabard[3],
        background_color: row.personal_tabard[4],
    };
    basic.visual_items = row.equipment.map(|value| VisualItemInfo {
        item_id: value.item_id,
        transmogrified_item_id: value.visible_item_id,
        subclass: value.subclass,
        inv_type: value.inventory_type,
        display_id: value.display_id,
        display_enchant_id: value.display_enchant_id,
        secondary_item_modified_appearance_id: value.secondary_appearance_id,
        sheathe_category: value.sheathe_category,
    });
    basic.super_district_id = policy.super_district;
    Ok(basic)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(super) fn fixture() -> SelectionPolicy {
    tests::policy()
}
