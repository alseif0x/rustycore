//! Composition consumes SQL startup DTOs into the target's effective catalog.
//! No SQL pool, Session state or second mutable catalog lives here.
use wow_data::forever_initialization::{
    ClassPowerRecord, ClassRecord, InitializationRecords, MapRecord, MovieRecord, PowerRecord,
    RaceRecord, SpecializationRecord,
};
use wow_persistence::forever::initialization::InitializationRows;

pub(super) fn records(rows: InitializationRows) -> InitializationRecords {
    InitializationRecords {
        classes: rows
            .classes
            .into_iter()
            .map(|row| ClassRecord {
                id: row.id,
                flags: row.flags,
                starting_level: row.starting_level,
                cinematic: row.cinematic,
                default_spec: row.default_spec,
                strength_bonus: row.strength_bonus,
                primary_stat_priority: row.primary_stat_priority,
                display_power: row.display_power,
                ranged_attack_per_agility: row.ranged_attack_per_agility,
                attack_per_agility: row.attack_per_agility,
                attack_per_strength: row.attack_per_strength,
                spell_class_set: row.spell_class_set,
            })
            .collect(),
        races: rows
            .races
            .into_iter()
            .map(|row| RaceRecord {
                id: row.id,
                flags: row.flags,
                faction: row.faction,
                cinematic: row.cinematic,
                resurrection_sickness_spell: row.resurrection_sickness_spell,
                starting_level: row.starting_level,
                base_language: row.base_language,
                creature_type: row.creature_type,
                alliance: row.alliance,
                neutral_race: row.neutral_race,
            })
            .collect(),
        maps: rows
            .maps
            .into_iter()
            .map(|row| MapRecord {
                id: row.id,
                instance_type: row.instance_type,
                expansion: row.expansion,
                parent_map: row.parent_map,
                flags: row.flags,
            })
            .collect(),
        powers: rows
            .powers
            .into_iter()
            .map(|row| PowerRecord {
                id: row.id,
                power_type: row.power_type,
                min_power: row.min_power,
                max_base_power: row.max_base_power,
                center_power: row.center_power,
                default_power: row.default_power,
                display_modifier: row.display_modifier,
                regen_interrupt_ms: row.regen_interrupt_ms,
                regen_peace: row.regen_peace,
                regen_combat: row.regen_combat,
                flags: row.flags,
            })
            .collect(),
        specializations: rows
            .specializations
            .into_iter()
            .map(|row| SpecializationRecord {
                id: row.id,
                class: row.class,
                order_index: row.order_index,
                pet_talent_type: row.pet_talent_type,
                role: row.role,
                flags: row.flags,
                primary_stat_priority: row.primary_stat_priority,
                mastery_spells: row.mastery_spells,
            })
            .collect(),
        // SQL batches are overlays, never declarations about unavailable client
        // sections. Their effective catalog retains the baseline's own count.
        unknown_map_records: 0,
        class_powers: rows
            .class_powers
            .into_iter()
            .map(|row| ClassPowerRecord {
                id: row.id,
                power_type: row.power_type,
                class: row.class,
            })
            .collect(),
        movies: rows
            .movies
            .into_iter()
            .map(|row| MovieRecord {
                id: row.id,
                volume: row.volume,
                key_id: row.key_id,
                audio_file: row.audio_file,
                subtitle_file: row.subtitle_file,
                subtitle_format: row.subtitle_format,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_persistence::forever::initialization::{
        ClassRow, MapRow, PowerRow, RaceRow, SpecializationRow,
    };

    #[test]
    fn signed_widths_arrays_and_float_bits_survive_composition_without_narrowing() {
        let rows = records(InitializationRows {
            classes: vec![ClassRow {
                id: 255,
                flags: i32::MIN,
                starting_level: -1,
                cinematic: u16::MAX,
                default_spec: u16::MAX,
                strength_bonus: 255,
                primary_stat_priority: -128,
                display_power: -1,
                ranged_attack_per_agility: 255,
                attack_per_agility: 254,
                attack_per_strength: 253,
                spell_class_set: 252,
            }],
            races: vec![RaceRow {
                id: u32::MAX,
                flags: i32::MIN,
                faction: -1,
                cinematic: i32::MAX,
                resurrection_sickness_spell: i32::MIN,
                starting_level: -2,
                base_language: -128,
                creature_type: 255,
                alliance: -1,
                neutral_race: -2,
            }],
            maps: vec![MapRow {
                id: 0,
                instance_type: -1,
                expansion: 255,
                parent_map: -1,
                flags: [i32::MIN, -2, i32::MAX],
            }],
            powers: vec![PowerRow {
                id: 1,
                power_type: -2,
                min_power: i32::MIN,
                max_base_power: i32::MAX,
                center_power: -1,
                default_power: -2,
                display_modifier: -3,
                regen_interrupt_ms: -4,
                regen_peace: -0.0,
                regen_combat: 0.5,
                flags: -5,
            }],
            specializations: vec![SpecializationRow {
                id: 2,
                class: 255,
                order_index: -1,
                pet_talent_type: -2,
                role: -3,
                flags: i32::MIN,
                primary_stat_priority: -4,
                mastery_spells: [-1, i32::MAX],
            }],
            ..Default::default()
        });
        assert_eq!(rows.maps[0].flags, [i32::MIN, -2, i32::MAX]);
        assert_eq!(rows.maps[0].parent_map, -1);
        assert_eq!(rows.powers[0].min_power, i32::MIN);
        assert_eq!(rows.powers[0].regen_peace.to_bits(), (-0.0f32).to_bits());
        assert_eq!(rows.specializations[0].mastery_spells, [-1, i32::MAX]);
        assert_eq!(rows.specializations[0].class, 255);
        assert_eq!(rows.specializations[0].order_index, -1);
        assert_eq!(rows.unknown_map_records, 0);
        let class = &rows.classes[0];
        assert_eq!(
            (class.id, class.flags, class.starting_level),
            (255, i32::MIN, -1)
        );
        assert_eq!((class.cinematic, class.default_spec), (u16::MAX, u16::MAX));
        assert_eq!(
            (
                class.strength_bonus,
                class.primary_stat_priority,
                class.display_power
            ),
            (255, -128, -1)
        );
        assert_eq!(
            (
                class.ranged_attack_per_agility,
                class.attack_per_agility,
                class.attack_per_strength,
                class.spell_class_set
            ),
            (255, 254, 253, 252)
        );
        let race = &rows.races[0];
        assert_eq!(
            (race.id, race.flags, race.faction),
            (u32::MAX, i32::MIN, -1)
        );
        assert_eq!(
            (
                race.cinematic,
                race.resurrection_sickness_spell,
                race.starting_level
            ),
            (i32::MAX, i32::MIN, -2)
        );
        assert_eq!(
            (
                race.base_language,
                race.creature_type,
                race.alliance,
                race.neutral_race
            ),
            (-128, 255, -1, -2)
        );
    }
}
