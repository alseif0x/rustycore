//! Vehicle.db2 / VehicleSeat.db2 readers for C++ `VehicleKit` parity.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;
use wow_core::Position;
use wow_database::{HotfixDatabase, HotfixStatements, WorldDatabase};
use wow_entities::{VehicleAccessory, VehicleSeatAddon, VehicleSeatInfo, VehicleTemplate};

use crate::wdc4::Wdc4Reader;

pub const MAX_VEHICLE_SEATS_LIKE_CPP: usize = 8;
pub const VEHICLE_SEAT_FLAG_SHOULD_USE_VEH_SEAT_EXIT_ANIM_ON_VOLUNTARY_EXIT: i32 = 0x0000_0008;
pub const VEHICLE_SEAT_FLAG_DISABLE_GRAVITY: i32 = 0x0000_0004;
pub const VEHICLE_SEAT_FLAG_CAN_CONTROL: i32 = 0x0000_0800;
pub const VEHICLE_SEAT_FLAG_UNCONTROLLED: i32 = 0x0000_2000;
pub const VEHICLE_SEAT_FLAG_CAN_ATTACK: i32 = 0x0000_4000;
pub const VEHICLE_SEAT_FLAG_PASSENGER_NOT_SELECTABLE: i32 = 0x0010_0000;
pub const VEHICLE_SEAT_FLAG_UNK18: i32 = 0x0002_0000;
pub const VEHICLE_SEAT_FLAG_CAN_ENTER_OR_EXIT: i32 = 0x0200_0000;
pub const VEHICLE_SEAT_FLAG_CAN_SWITCH: i32 = 0x0400_0000;
pub const VEHICLE_SEAT_FLAG_B_USABLE_FORCED: i32 = 0x0000_0002;
pub const VEHICLE_SEAT_FLAG_B_EJECTABLE: i32 = 0x0000_0020;
pub const VEHICLE_SEAT_FLAG_B_USABLE_FORCED_2: i32 = 0x0000_0040;
pub const VEHICLE_SEAT_FLAG_B_USABLE_FORCED_3: i32 = 0x0000_0100;
pub const VEHICLE_SEAT_FLAG_B_KEEP_PET: i32 = 0x0002_0000;
pub const VEHICLE_SEAT_FLAG_B_USABLE_FORCED_4: i32 = 0x0200_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VehicleEntry {
    pub id: u32,
    pub flags: i32,
    pub flags_b: i32,
    pub seat_ids: [u16; MAX_VEHICLE_SEATS_LIKE_CPP],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleSeatEntry {
    pub id: u32,
    pub attachment_offset_x: f32,
    pub attachment_offset_y: f32,
    pub attachment_offset_z: f32,
    pub flags: i32,
    pub flags_b: i32,
    pub flags_c: i32,
}

impl VehicleSeatEntry {
    pub fn has_flag(&self, flag: i32) -> bool {
        self.flags & flag != 0
    }

    pub fn has_flag_b(&self, flag: i32) -> bool {
        self.flags_b & flag != 0
    }

    pub fn can_enter_or_exit_like_cpp(&self) -> bool {
        self.has_flag(
            VEHICLE_SEAT_FLAG_CAN_ENTER_OR_EXIT
                | VEHICLE_SEAT_FLAG_CAN_CONTROL
                | VEHICLE_SEAT_FLAG_SHOULD_USE_VEH_SEAT_EXIT_ANIM_ON_VOLUNTARY_EXIT,
        )
    }

    pub fn usable_by_override_like_cpp(&self) -> bool {
        self.has_flag(VEHICLE_SEAT_FLAG_UNCONTROLLED | VEHICLE_SEAT_FLAG_UNK18)
            || self.has_flag_b(
                VEHICLE_SEAT_FLAG_B_USABLE_FORCED
                    | VEHICLE_SEAT_FLAG_B_USABLE_FORCED_2
                    | VEHICLE_SEAT_FLAG_B_USABLE_FORCED_3
                    | VEHICLE_SEAT_FLAG_B_USABLE_FORCED_4,
            )
    }

    pub fn can_switch_from_seat_like_cpp(&self) -> bool {
        self.has_flag(VEHICLE_SEAT_FLAG_CAN_SWITCH)
    }

    pub fn is_ejectable_like_cpp(&self) -> bool {
        self.has_flag_b(VEHICLE_SEAT_FLAG_B_EJECTABLE)
    }

    pub fn to_vehicle_seat_info_like_cpp(&self) -> VehicleSeatInfo {
        VehicleSeatInfo {
            id: self.id,
            attachment_offset: Position::new(
                self.attachment_offset_x,
                self.attachment_offset_y,
                self.attachment_offset_z,
                0.0,
            ),
            can_enter_or_exit: self.can_enter_or_exit_like_cpp(),
            usable_by_override: self.usable_by_override_like_cpp(),
            can_control: self.has_flag(VEHICLE_SEAT_FLAG_CAN_CONTROL),
            disables_gravity: self.has_flag(VEHICLE_SEAT_FLAG_DISABLE_GRAVITY),
            passenger_not_selectable: self.has_flag(VEHICLE_SEAT_FLAG_PASSENGER_NOT_SELECTABLE),
            keep_pet: self.has_flag_b(VEHICLE_SEAT_FLAG_B_KEEP_PET),
        }
    }
}

pub struct VehicleStore {
    by_id: HashMap<u32, VehicleEntry>,
}

pub struct VehicleSeatStore {
    by_id: HashMap<u32, VehicleSeatEntry>,
}

#[derive(Debug, Clone, Default)]
pub struct VehicleTemplateStoreLikeCpp {
    by_creature_entry: HashMap<u32, VehicleTemplate>,
}

#[derive(Debug, Clone, Default)]
pub struct VehicleAccessoryStoreLikeCpp {
    by_spawn_guid: HashMap<u64, Vec<VehicleAccessory>>,
    by_creature_entry: HashMap<u32, Vec<VehicleAccessory>>,
}

impl VehicleStore {
    pub fn from_entries(entries: impl IntoIterator<Item = VehicleEntry>) -> Self {
        Self {
            by_id: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
        }
    }

    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("Vehicle.db2");
        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = Vec::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            let mut seat_ids = [0u16; MAX_VEHICLE_SEATS_LIKE_CPP];
            for (offset, seat_id) in seat_ids.iter_mut().enumerate() {
                *seat_id = reader.get_array_u16(idx, 17, offset);
            }
            entries.push(VehicleEntry {
                id,
                flags: reader.get_field_i32(idx, 0),
                flags_b: reader.get_field_i32(idx, 1),
                seat_ids,
            });
        }

        let store = Self::from_entries(entries);
        info!("Loaded {} vehicles from {}", store.len(), path.display());
        Ok(store)
    }

    pub async fn load_with_hotfixes(
        data_dir: &str,
        locale: &str,
        hotfix_db: &HotfixDatabase,
    ) -> Result<Self> {
        let mut store = Self::load(data_dir, locale)?;
        let hotfix_rows = store.load_hotfix_rows(hotfix_db).await?;
        if hotfix_rows != 0 {
            info!("Loaded {hotfix_rows} Vehicle hotfix rows");
        }
        Ok(store)
    }

    async fn load_hotfix_rows(&mut self, db: &HotfixDatabase) -> Result<usize> {
        let stmt = db.prepare(HotfixStatements::SEL_VEHICLE);
        let mut result = db.query(&stmt).await?;
        if result.is_empty() {
            return Ok(0);
        }

        let mut count = 0usize;
        loop {
            let mut seat_ids = [0u16; MAX_VEHICLE_SEATS_LIKE_CPP];
            for (offset, seat_id) in seat_ids.iter_mut().enumerate() {
                *seat_id = result.read(18 + offset);
            }
            let entry = VehicleEntry {
                id: result.read(0),
                flags: result.read(1),
                flags_b: result.read(2),
                seat_ids,
            };
            self.by_id.insert(entry.id, entry);
            count += 1;

            if !result.next_row() {
                break;
            }
        }
        Ok(count)
    }

    pub fn get(&self, id: u32) -> Option<&VehicleEntry> {
        self.by_id.get(&id)
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

impl VehicleSeatStore {
    pub fn from_entries(entries: impl IntoIterator<Item = VehicleSeatEntry>) -> Self {
        Self {
            by_id: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
        }
    }

    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("VehicleSeat.db2");
        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = Vec::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            entries.push(VehicleSeatEntry {
                id,
                attachment_offset_x: f32::from_bits(reader.get_array_element(idx, 0, 0, 32)),
                attachment_offset_y: f32::from_bits(reader.get_array_element(idx, 0, 1, 32)),
                attachment_offset_z: f32::from_bits(reader.get_array_element(idx, 0, 2, 32)),
                flags: reader.get_field_i32(idx, 2),
                flags_b: reader.get_field_i32(idx, 3),
                flags_c: reader.get_field_i32(idx, 4),
            });
        }

        let store = Self::from_entries(entries);
        info!(
            "Loaded {} vehicle seats from {}",
            store.len(),
            path.display()
        );
        Ok(store)
    }

    pub async fn load_with_hotfixes(
        data_dir: &str,
        locale: &str,
        hotfix_db: &HotfixDatabase,
    ) -> Result<Self> {
        let mut store = Self::load(data_dir, locale)?;
        let hotfix_rows = store.load_hotfix_rows(hotfix_db).await?;
        if hotfix_rows != 0 {
            info!("Loaded {hotfix_rows} VehicleSeat hotfix rows");
        }
        Ok(store)
    }

    async fn load_hotfix_rows(&mut self, db: &HotfixDatabase) -> Result<usize> {
        let stmt = db.prepare(HotfixStatements::SEL_VEHICLE_SEAT);
        let mut result = db.query(&stmt).await?;
        if result.is_empty() {
            return Ok(0);
        }

        let mut count = 0usize;
        loop {
            let entry = VehicleSeatEntry {
                id: result.read(0),
                attachment_offset_x: result.read(1),
                attachment_offset_y: result.read(2),
                attachment_offset_z: result.read(3),
                flags: result.read(7),
                flags_b: result.read(8),
                flags_c: result.read(9),
            };
            self.by_id.insert(entry.id, entry);
            count += 1;

            if !result.next_row() {
                break;
            }
        }
        Ok(count)
    }

    pub fn get(&self, id: u32) -> Option<&VehicleSeatEntry> {
        self.by_id.get(&id)
    }

    pub fn seat_defs_for_vehicle_like_cpp(
        &self,
        vehicle: &VehicleEntry,
    ) -> Vec<(i8, VehicleSeatInfo, VehicleSeatAddon)> {
        vehicle
            .seat_ids
            .iter()
            .enumerate()
            .filter_map(|(seat_index, seat_id)| {
                if *seat_id == 0 {
                    return None;
                }

                let seat = self.get(u32::from(*seat_id))?;
                Some((
                    i8::try_from(seat_index).ok()?,
                    seat.to_vehicle_seat_info_like_cpp(),
                    VehicleSeatAddon::default(),
                ))
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

impl VehicleTemplateStoreLikeCpp {
    pub fn from_entries(entries: impl IntoIterator<Item = (u32, VehicleTemplate)>) -> Self {
        Self {
            by_creature_entry: entries.into_iter().collect(),
        }
    }

    pub async fn load_like_cpp(db: &WorldDatabase) -> Result<Self> {
        let mut result = db
            .direct_query("SELECT `creatureId`, `despawnDelayMs` FROM `vehicle_template`")
            .await?;
        if result.is_empty() {
            info!("Loaded 0 Vehicle Template entries");
            return Ok(Self::default());
        }

        let mut entries = HashMap::new();
        loop {
            let creature_entry = result.read::<u32>(0);
            let despawn_delay_ms = result.read::<i32>(1);
            entries.insert(creature_entry, VehicleTemplate { despawn_delay_ms });

            if !result.next_row() {
                break;
            }
        }

        let store = Self {
            by_creature_entry: entries,
        };
        info!("Loaded {} Vehicle Template entries", store.len());
        Ok(store)
    }

    pub fn get(&self, creature_entry: u32) -> Option<&VehicleTemplate> {
        self.by_creature_entry.get(&creature_entry)
    }

    pub fn despawn_delay_ms_like_cpp(&self, creature_entry: u32) -> i32 {
        self.get(creature_entry)
            .map(|template| template.despawn_delay_ms)
            .unwrap_or(1)
    }

    pub fn len(&self) -> usize {
        self.by_creature_entry.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_creature_entry.is_empty()
    }
}

impl VehicleAccessoryStoreLikeCpp {
    pub fn from_parts(
        by_spawn_guid: impl IntoIterator<Item = (u64, Vec<VehicleAccessory>)>,
        by_creature_entry: impl IntoIterator<Item = (u32, Vec<VehicleAccessory>)>,
    ) -> Self {
        Self {
            by_spawn_guid: by_spawn_guid.into_iter().collect(),
            by_creature_entry: by_creature_entry.into_iter().collect(),
        }
    }

    pub async fn load_like_cpp(db: &WorldDatabase) -> Result<Self> {
        let mut store = Self::default();
        let template_rows = store.load_template_accessories_like_cpp(db).await?;
        let spawn_rows = store.load_spawn_accessories_like_cpp(db).await?;
        info!(
            "Loaded {template_rows} vehicle template accessories and {spawn_rows} vehicle accessories"
        );
        Ok(store)
    }

    async fn load_template_accessories_like_cpp(&mut self, db: &WorldDatabase) -> Result<usize> {
        let mut result = db
            .direct_query(
                "SELECT `entry`, `accessory_entry`, `seat_id`, `minion`, `summontype`, `summontimer` \
                 FROM `vehicle_template_accessory`",
            )
            .await?;
        if result.is_empty() {
            return Ok(0);
        }

        let mut count = 0usize;
        loop {
            let entry = result.read::<u32>(0);
            let accessory = VehicleAccessory {
                accessory_entry: result.read(1),
                seat_id: result.read::<i8>(2),
                is_minion: result.read::<bool>(3),
                summoned_type: result.read(4),
                summon_time_ms: result.read(5),
            };
            self.by_creature_entry
                .entry(entry)
                .or_default()
                .push(accessory);
            count += 1;

            if !result.next_row() {
                break;
            }
        }
        Ok(count)
    }

    async fn load_spawn_accessories_like_cpp(&mut self, db: &WorldDatabase) -> Result<usize> {
        let mut result = db
            .direct_query(
                "SELECT `guid`, `accessory_entry`, `seat_id`, `minion`, `summontype`, `summontimer` \
                 FROM `vehicle_accessory`",
            )
            .await?;
        if result.is_empty() {
            return Ok(0);
        }

        let mut count = 0usize;
        loop {
            let guid = result.read::<u64>(0);
            let accessory = VehicleAccessory {
                accessory_entry: result.read(1),
                seat_id: result.read::<i8>(2),
                is_minion: result.read::<bool>(3),
                summoned_type: result.read(4),
                summon_time_ms: result.read(5),
            };
            self.by_spawn_guid.entry(guid).or_default().push(accessory);
            count += 1;

            if !result.next_row() {
                break;
            }
        }
        Ok(count)
    }

    pub fn accessories_for_vehicle_like_cpp(
        &self,
        spawn_guid_low: Option<u64>,
        creature_entry: u32,
    ) -> Option<&[VehicleAccessory]> {
        if let Some(accessories) =
            spawn_guid_low.and_then(|guid| self.by_spawn_guid.get(&guid).map(Vec::as_slice))
        {
            return Some(accessories);
        }
        self.by_creature_entry
            .get(&creature_entry)
            .map(Vec::as_slice)
    }

    pub fn template_len(&self) -> usize {
        self.by_creature_entry.values().map(Vec::len).sum()
    }

    pub fn spawn_len(&self) -> usize {
        self.by_spawn_guid.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.template_len() == 0 && self.spawn_len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vehicle_seat_flags_match_cpp_helpers() {
        let control = VehicleSeatEntry {
            id: 1,
            attachment_offset_x: 1.5,
            attachment_offset_y: 2.5,
            attachment_offset_z: 3.5,
            flags: VEHICLE_SEAT_FLAG_CAN_CONTROL,
            flags_b: 0,
            flags_c: 0,
        };
        assert!(control.can_enter_or_exit_like_cpp());
        assert!(!control.usable_by_override_like_cpp());
        let control_info = control.to_vehicle_seat_info_like_cpp();
        assert_eq!(
            control_info.attachment_offset,
            Position::new(1.5, 2.5, 3.5, 0.0)
        );
        assert!(control_info.can_control);
        assert!(!control_info.disables_gravity);
        assert!(!control_info.passenger_not_selectable);
        assert!(!control_info.keep_pet);

        let forced = VehicleSeatEntry {
            id: 2,
            attachment_offset_x: 0.0,
            attachment_offset_y: 0.0,
            attachment_offset_z: 0.0,
            flags: 0,
            flags_b: VEHICLE_SEAT_FLAG_B_USABLE_FORCED_3 | VEHICLE_SEAT_FLAG_B_KEEP_PET,
            flags_c: 0,
        };
        assert!(!forced.can_enter_or_exit_like_cpp());
        assert!(forced.usable_by_override_like_cpp());
        assert!(forced.to_vehicle_seat_info_like_cpp().keep_pet);

        let passenger_flags = VehicleSeatEntry {
            id: 3,
            attachment_offset_x: 0.0,
            attachment_offset_y: 0.0,
            attachment_offset_z: 0.0,
            flags: VEHICLE_SEAT_FLAG_DISABLE_GRAVITY | VEHICLE_SEAT_FLAG_PASSENGER_NOT_SELECTABLE,
            flags_b: 0,
            flags_c: 0,
        }
        .to_vehicle_seat_info_like_cpp();
        assert!(passenger_flags.disables_gravity);
        assert!(passenger_flags.passenger_not_selectable);
    }

    #[test]
    fn vehicle_seat_handler_flags_match_cpp_helpers() {
        let switchable = VehicleSeatEntry {
            id: 1,
            attachment_offset_x: 0.0,
            attachment_offset_y: 0.0,
            attachment_offset_z: 0.0,
            flags: VEHICLE_SEAT_FLAG_CAN_SWITCH,
            flags_b: 0,
            flags_c: 0,
        };
        let ejectable = VehicleSeatEntry {
            id: 2,
            attachment_offset_x: 0.0,
            attachment_offset_y: 0.0,
            attachment_offset_z: 0.0,
            flags: 0,
            flags_b: VEHICLE_SEAT_FLAG_B_EJECTABLE,
            flags_c: 0,
        };

        assert!(switchable.can_switch_from_seat_like_cpp());
        assert!(!switchable.is_ejectable_like_cpp());
        assert!(!ejectable.can_switch_from_seat_like_cpp());
        assert!(ejectable.is_ejectable_like_cpp());
    }

    #[test]
    fn vehicle_seat_defs_follow_cpp_seat_order() {
        let vehicles = VehicleStore::from_entries([VehicleEntry {
            id: 10,
            flags: 0,
            flags_b: 0,
            seat_ids: [100, 0, 101, 0, 0, 0, 0, 0],
        }]);
        let seats = VehicleSeatStore::from_entries([
            VehicleSeatEntry {
                id: 0,
                attachment_offset_x: 9.0,
                attachment_offset_y: 9.0,
                attachment_offset_z: 9.0,
                flags: VEHICLE_SEAT_FLAG_CAN_ENTER_OR_EXIT,
                flags_b: 0,
                flags_c: 0,
            },
            VehicleSeatEntry {
                id: 100,
                attachment_offset_x: 1.0,
                attachment_offset_y: 2.0,
                attachment_offset_z: 3.0,
                flags: VEHICLE_SEAT_FLAG_CAN_ENTER_OR_EXIT,
                flags_b: 0,
                flags_c: 0,
            },
            VehicleSeatEntry {
                id: 101,
                attachment_offset_x: 4.0,
                attachment_offset_y: 5.0,
                attachment_offset_z: 6.0,
                flags: 0,
                flags_b: VEHICLE_SEAT_FLAG_B_USABLE_FORCED,
                flags_c: 0,
            },
        ]);

        let defs = seats.seat_defs_for_vehicle_like_cpp(vehicles.get(10).unwrap());
        assert_eq!(defs.len(), 2);
        assert!(defs.iter().all(|(_, seat, _)| seat.id != 0));
        assert_eq!(defs[0].0, 0);
        assert_eq!(defs[0].1.id, 100);
        assert_eq!(
            defs[0].1.attachment_offset,
            Position::new(1.0, 2.0, 3.0, 0.0)
        );
        assert!(defs[0].1.can_enter_or_exit);
        assert_eq!(defs[1].0, 2);
        assert_eq!(defs[1].1.id, 101);
        assert_eq!(
            defs[1].1.attachment_offset,
            Position::new(4.0, 5.0, 6.0, 0.0)
        );
        assert!(defs[1].1.usable_by_override);
    }

    #[test]
    fn vehicle_template_despawn_delay_defaults_to_one_ms_like_cpp() {
        let store = VehicleTemplateStoreLikeCpp::from_entries([(
            1234,
            VehicleTemplate {
                despawn_delay_ms: 2500,
            },
        )]);

        assert_eq!(store.despawn_delay_ms_like_cpp(1234), 2500);
        assert_eq!(store.despawn_delay_ms_like_cpp(9999), 1);
    }

    #[test]
    fn vehicle_accessories_prefer_spawn_guid_like_cpp() {
        let template_accessory = VehicleAccessory {
            accessory_entry: 10,
            seat_id: 1,
            is_minion: true,
            summoned_type: 8,
            summon_time_ms: 100,
        };
        let spawn_accessory = VehicleAccessory {
            accessory_entry: 20,
            seat_id: 2,
            is_minion: false,
            summoned_type: 6,
            summon_time_ms: 200,
        };
        let store = VehicleAccessoryStoreLikeCpp::from_parts(
            [(77, vec![spawn_accessory])],
            [(1234, vec![template_accessory])],
        );

        assert_eq!(
            store.accessories_for_vehicle_like_cpp(Some(77), 1234),
            Some([spawn_accessory].as_slice())
        );
        assert_eq!(
            store.accessories_for_vehicle_like_cpp(None, 1234),
            Some([template_accessory].as_slice())
        );
        assert_eq!(
            store.accessories_for_vehicle_like_cpp(Some(99), 1234),
            Some([template_accessory].as_slice())
        );
        assert!(
            store
                .accessories_for_vehicle_like_cpp(Some(99), 1)
                .is_none()
        );
    }

    #[test]
    fn load_vehicle_db2_uses_physical_wdc4_arrays_like_cpp_meta() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "enUS";
        let vehicle_path = std::path::Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("Vehicle.db2");
        let seat_path = std::path::Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("VehicleSeat.db2");
        if !vehicle_path.exists() || !seat_path.exists() {
            eprintln!(
                "Skipping test: Vehicle.db2 or VehicleSeat.db2 not found under {}",
                std::path::Path::new(data_dir)
                    .join("dbc")
                    .join(locale)
                    .display()
            );
            return;
        }

        let vehicles = VehicleStore::load(data_dir, locale)
            .expect("Vehicle.db2 should load with physical WDC4 field indices");
        let seats = VehicleSeatStore::load(data_dir, locale)
            .expect("VehicleSeat.db2 should load with physical WDC4 field indices");

        assert!(!vehicles.is_empty());
        assert!(!seats.is_empty());
        assert!(
            vehicles
                .by_id
                .values()
                .any(|vehicle| vehicle.seat_ids.iter().any(|seat_id| *seat_id != 0))
        );
    }
}
