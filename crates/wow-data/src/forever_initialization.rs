//! Target numeric initialization data. Transient baselines become an immutable
//! effective catalog only after official/custom overlays and final removals.
//! An effective catalog is a prerequisite, not a fully initialized Player.
//! DB2Metadata/DB2LoadInfo at 02245dcd; hashes from local 70170 headers.

use crate::wdc4::creation::{CreationDb2, CreationTable};
use anyhow::Result;
use std::path::Path;
mod effective;
mod identity;
pub use effective::{
    CLASS_HASH, CLASS_POWER_HASH, InitializationCatalog, MAP_HASH, MOVIE_HASH, POWER_HASH,
    RACE_HASH, SPECIALIZATION_HASH,
};
pub use identity::{ClassRecord, RaceRecord};

#[derive(Clone, Copy)]
pub enum MapBaseline {
    Complete,
    /// Explicit 70170/esES prefix: 71 present / eight unknown rows. Unknown
    /// maps are not admitted, manufactured or declared known absent.
    AvailablePrefix,
}

#[derive(Clone, Copy)]
pub struct MapRecord {
    pub id: u32,
    pub instance_type: i8,
    pub expansion: u8,
    pub parent_map: i16,
    pub flags: [i32; 3],
}

impl MapRecord {
    /// DB2Structure.h::MapEntry::Instanceable: instance/raid/BG/arena/scenario.
    pub fn instanceable(&self) -> bool {
        matches!(self.instance_type, 1..=5)
    }
}

#[derive(Clone, Copy)]
pub struct PowerRecord {
    pub id: u32,
    pub power_type: i8,
    pub min_power: i32,
    pub max_base_power: i32,
    pub center_power: i32,
    pub default_power: i32,
    pub display_modifier: i32,
    pub regen_interrupt_ms: i32,
    pub regen_peace: f32,
    pub regen_combat: f32,
    pub flags: i32,
}

#[derive(Clone, Copy)]
pub struct SpecializationRecord {
    pub id: u32,
    /// DB2LoadInfo declares ClassID unsigned despite signed DB2Meta storage.
    pub class: u8,
    pub order_index: i8,
    pub pet_talent_type: i8,
    pub role: i8,
    pub flags: i32,
    pub primary_stat_priority: i8,
    pub mastery_spells: [i32; 2],
}

#[derive(Default)]
pub struct InitializationRecords {
    pub maps: Vec<MapRecord>,
    pub powers: Vec<PowerRecord>,
    pub specializations: Vec<SpecializationRecord>,
    pub class_powers: Vec<ClassPowerRecord>,
    pub movies: Vec<MovieRecord>,
    pub classes: Vec<ClassRecord>,
    pub races: Vec<RaceRecord>,
    pub unknown_map_records: usize,
}

#[derive(Clone, Copy)]
pub struct ClassPowerRecord {
    pub id: u32,
    pub power_type: i8,
    /// DB2Meta stores a byte parent; SQL/DB2LoadInfo declare uint32 ClassID.
    pub class: u32,
}

/// Numeric projection only. Summary/locales and full wire serialization are
/// deliberately not claimed by the creation movie-presence operation.
#[derive(Clone, Copy)]
pub struct MovieRecord {
    pub id: u32,
    pub volume: u8,
    pub key_id: u8,
    pub audio_file: u32,
    pub subtitle_file: u32,
    pub subtitle_format: u32,
}

impl InitializationRecords {
    /// All seven acquisitions must validate before publishing the startup
    /// batch. A malformed complete file never falls back to an available one.
    pub fn load(directory: &Path, map_baseline: MapBaseline) -> Result<Self> {
        let (class_records, race_records) = identity::load(directory)?;
        let maps = match map_baseline {
            MapBaseline::Complete => CreationDb2::open(directory, CreationTable::Map)?,
            MapBaseline::AvailablePrefix => CreationDb2::open_initial_map_prefix(directory)?,
        };
        let map_records = maps
            .ids()
            .map(|id| {
                Ok(MapRecord {
                    id,
                    instance_type: maps.bits(id, 8, 0)? as i8,
                    expansion: maps.bits(id, 9, 0)? as u8,
                    parent_map: maps.bits(id, 13, 0)? as i16,
                    flags: [
                        maps.bits(id, 25, 0)? as i32,
                        maps.bits(id, 25, 1)? as i32,
                        maps.bits(id, 25, 2)? as i32,
                    ],
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let powers = CreationDb2::open(directory, CreationTable::PowerType)?;
        let power_records = powers
            .ids()
            .map(|id| {
                Ok(PowerRecord {
                    id,
                    power_type: powers.bits(id, 3, 0)? as i8,
                    min_power: powers.bits(id, 4, 0)? as i32,
                    max_base_power: powers.bits(id, 5, 0)? as i32,
                    center_power: powers.bits(id, 6, 0)? as i32,
                    default_power: powers.bits(id, 7, 0)? as i32,
                    display_modifier: powers.bits(id, 8, 0)? as i32,
                    regen_interrupt_ms: powers.bits(id, 9, 0)? as i32,
                    regen_peace: f32::from_bits(powers.bits(id, 10, 0)?),
                    regen_combat: f32::from_bits(powers.bits(id, 11, 0)?),
                    flags: powers.bits(id, 12, 0)? as i32,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let specializations = CreationDb2::open(directory, CreationTable::Specialization)?;
        let specialization_records = specializations
            .ids()
            .map(|id| {
                Ok(SpecializationRecord {
                    id,
                    class: specializations.bits(id, 4, 0)? as u8,
                    order_index: specializations.bits(id, 5, 0)? as i8,
                    pet_talent_type: specializations.bits(id, 6, 0)? as i8,
                    role: specializations.bits(id, 7, 0)? as i8,
                    flags: specializations.bits(id, 8, 0)? as i32,
                    primary_stat_priority: specializations.bits(id, 10, 0)? as i8,
                    mastery_spells: [
                        specializations.bits(id, 12, 0)? as i32,
                        specializations.bits(id, 12, 1)? as i32,
                    ],
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let class_powers = CreationDb2::open(directory, CreationTable::ClassPower)?;
        let class_power_records = class_powers
            .ids()
            .map(|id| {
                Ok(ClassPowerRecord {
                    id,
                    power_type: class_powers.bits(id, 0, 0)? as i8,
                    class: class_powers.bits(id, 1, 0)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let movies = CreationDb2::open(directory, CreationTable::Movie)?;
        let movie_records = movies
            .ids()
            .map(|id| {
                Ok(MovieRecord {
                    id,
                    volume: movies.bits(id, 1, 0)? as u8,
                    key_id: movies.bits(id, 2, 0)? as u8,
                    audio_file: movies.bits(id, 3, 0)?,
                    subtitle_file: movies.bits(id, 4, 0)?,
                    subtitle_format: movies.bits(id, 5, 0)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            maps: map_records,
            powers: power_records,
            specializations: specialization_records,
            class_powers: class_power_records,
            movies: movie_records,
            classes: class_records,
            races: race_records,
            unknown_map_records: match map_baseline {
                MapBaseline::Complete => 0,
                MapBaseline::AvailablePrefix => 8,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_instance_types_are_exact_not_every_nonzero_type() {
        for instance_type in i8::MIN..=i8::MAX {
            let map = MapRecord {
                id: 0,
                instance_type,
                expansion: 0,
                parent_map: -1,
                flags: [0; 3],
            };
            assert_eq!(
                map.instanceable(),
                matches!(instance_type, 1 | 2 | 3 | 4 | 5)
            );
        }
    }
}
