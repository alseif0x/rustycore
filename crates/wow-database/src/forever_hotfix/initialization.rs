//! Target creation initialization projections, not legacy hotfix offsets.
//! 02245dcd HotfixDatabase.cpp:452-457,1198-1204,1389-1391;
//! sql/base/dev/hotfixes_database.sql supplies the signed decode widths.
use super::{BTreeSet, ForeverHotfixRepository, LoadError, PreparedStatement};
use wow_persistence::forever::initialization::{
    ClassPowerRow, ClassRow, InitializationOverlays, InitializationRows, MapRow, MovieRow,
    PowerRow, RaceRow, SpecializationRow,
};

const QUERIES: [&str; 7] = [
    "SELECT ID,InstanceType,ExpansionID,ParentMapID,Flags1,Flags2,Flags3 FROM map WHERE (VerifiedBuild>0)=?",
    "SELECT ID,PowerTypeEnum,MinPower,MaxBasePower,CenterPower,DefaultPower,DisplayModifier,RegenInterruptTimeMS,RegenPeace,RegenCombat,Flags FROM power_type WHERE (VerifiedBuild>0)=?",
    "SELECT ID,ClassID,OrderIndex,PetTalentType,Role,Flags,PrimaryStatPriority,MasterySpellID1,MasterySpellID2 FROM chr_specialization WHERE (VerifiedBuild>0)=?",
    "SELECT ID,PowerType,ClassID FROM chr_classes_x_power_types WHERE (VerifiedBuild>0)=?",
    "SELECT ID,Volume,KeyID,AudioFileDataID,SubtitleFileDataID,SubtitleFileFormat FROM movie WHERE (VerifiedBuild>0)=?",
    "SELECT ID,Flags,StartingLevel,CinematicSequenceID,DefaultSpec,HasStrengthBonus,PrimaryStatPriority,DisplayPower,RangedAttackPowerPerAgility,AttackPowerPerAgility,AttackPowerPerStrength,SpellClassSet FROM chr_classes WHERE (VerifiedBuild>0)=?",
    "SELECT ID,Flags,FactionID,CinematicSequenceID,ResSicknessSpellID,StartingLevel,BaseLanguage,CreatureType,Alliance,NeutralRaceID FROM chr_races WHERE (VerifiedBuild>0)=?",
];

impl ForeverHotfixRepository {
    /// Both seven-table batches must succeed before the caller can publish
    /// an effective store. Independent reads are not a SQL snapshot claim.
    pub async fn load_initialization_overlays(&self) -> Result<InitializationOverlays, LoadError> {
        Ok(InitializationOverlays {
            official: self.initialization_batch(false).await?,
            custom: self.initialization_batch(true).await?,
        })
    }

    async fn initialization_batch(&self, custom: bool) -> Result<InitializationRows, LoadError> {
        let mut rows = InitializationRows::default();
        for (table, sql) in QUERIES.iter().enumerate() {
            let mut statement = PreparedStatement::new(*sql);
            statement.set_bool(0, !custom);
            let mut result = self
                .0
                .query(&statement)
                .await
                .map_err(|_| LoadError::Database)?;
            if result.is_empty() {
                continue;
            }
            let mut seen = BTreeSet::new();
            loop {
                let id = result.try_read::<u32>(0).ok_or(LoadError::InvalidRow)?;
                if !seen.insert(id) {
                    return Err(LoadError::InvalidRow);
                }
                let signed = |index| result.try_read::<i32>(index).ok_or(LoadError::InvalidRow);
                let byte = |index| result.try_read::<i8>(index).ok_or(LoadError::InvalidRow);
                match table {
                    0 => rows.maps.push(MapRow {
                        id,
                        instance_type: byte(1)?,
                        expansion: result.try_read::<u8>(2).ok_or(LoadError::InvalidRow)?,
                        parent_map: result.try_read::<i16>(3).ok_or(LoadError::InvalidRow)?,
                        flags: [signed(4)?, signed(5)?, signed(6)?],
                    }),
                    1 => rows.powers.push(PowerRow {
                        id,
                        power_type: byte(1)?,
                        min_power: signed(2)?,
                        max_base_power: signed(3)?,
                        center_power: signed(4)?,
                        default_power: signed(5)?,
                        display_modifier: signed(6)?,
                        regen_interrupt_ms: signed(7)?,
                        regen_peace: result.try_read::<f32>(8).ok_or(LoadError::InvalidRow)?,
                        regen_combat: result.try_read::<f32>(9).ok_or(LoadError::InvalidRow)?,
                        flags: signed(10)?,
                    }),
                    2 => rows.specializations.push(SpecializationRow {
                        id,
                        class: result.try_read::<u8>(1).ok_or(LoadError::InvalidRow)?,
                        order_index: byte(2)?,
                        pet_talent_type: byte(3)?,
                        role: byte(4)?,
                        flags: signed(5)?,
                        primary_stat_priority: byte(6)?,
                        mastery_spells: [signed(7)?, signed(8)?],
                    }),
                    3 => rows.class_powers.push(ClassPowerRow {
                        id,
                        power_type: byte(1)?,
                        class: result.try_read::<u32>(2).ok_or(LoadError::InvalidRow)?,
                    }),
                    4 => rows.movies.push(MovieRow {
                        id,
                        volume: result.try_read::<u8>(1).ok_or(LoadError::InvalidRow)?,
                        key_id: result.try_read::<u8>(2).ok_or(LoadError::InvalidRow)?,
                        audio_file: result.try_read::<u32>(3).ok_or(LoadError::InvalidRow)?,
                        subtitle_file: result.try_read::<u32>(4).ok_or(LoadError::InvalidRow)?,
                        subtitle_format: result.try_read::<u32>(5).ok_or(LoadError::InvalidRow)?,
                    }),
                    5 => rows.classes.push(ClassRow {
                        id,
                        flags: signed(1)?,
                        starting_level: signed(2)?,
                        cinematic: result.try_read::<u16>(3).ok_or(LoadError::InvalidRow)?,
                        default_spec: result.try_read::<u16>(4).ok_or(LoadError::InvalidRow)?,
                        strength_bonus: result.try_read::<u8>(5).ok_or(LoadError::InvalidRow)?,
                        primary_stat_priority: byte(6)?,
                        display_power: byte(7)?,
                        ranged_attack_per_agility: result
                            .try_read::<u8>(8)
                            .ok_or(LoadError::InvalidRow)?,
                        attack_per_agility: result
                            .try_read::<u8>(9)
                            .ok_or(LoadError::InvalidRow)?,
                        attack_per_strength: result
                            .try_read::<u8>(10)
                            .ok_or(LoadError::InvalidRow)?,
                        spell_class_set: result.try_read::<u8>(11).ok_or(LoadError::InvalidRow)?,
                    }),
                    6 => rows.races.push(RaceRow {
                        id,
                        flags: signed(1)?,
                        faction: signed(2)?,
                        cinematic: signed(3)?,
                        resurrection_sickness_spell: signed(4)?,
                        starting_level: signed(5)?,
                        base_language: byte(6)?,
                        creature_type: result.try_read::<u8>(7).ok_or(LoadError::InvalidRow)?,
                        alliance: byte(8)?,
                        neutral_race: byte(9)?,
                    }),
                    _ => unreachable!(),
                }
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_numeric_queries_preserve_column_order_and_build_predicate() {
        let columns = [
            "ID,InstanceType,ExpansionID,ParentMapID,Flags1,Flags2,Flags3",
            "ID,PowerTypeEnum,MinPower,MaxBasePower,CenterPower,DefaultPower,DisplayModifier,RegenInterruptTimeMS,RegenPeace,RegenCombat,Flags",
            "ID,ClassID,OrderIndex,PetTalentType,Role,Flags,PrimaryStatPriority,MasterySpellID1,MasterySpellID2",
            "ID,PowerType,ClassID",
            "ID,Volume,KeyID,AudioFileDataID,SubtitleFileDataID,SubtitleFileFormat",
            "ID,Flags,StartingLevel,CinematicSequenceID,DefaultSpec,HasStrengthBonus,PrimaryStatPriority,DisplayPower,RangedAttackPowerPerAgility,AttackPowerPerAgility,AttackPowerPerStrength,SpellClassSet",
            "ID,Flags,FactionID,CinematicSequenceID,ResSicknessSpellID,StartingLevel,BaseLanguage,CreatureType,Alliance,NeutralRaceID",
        ];
        for (query, fields) in QUERIES.into_iter().zip(columns) {
            assert_eq!(
                query
                    .strip_prefix("SELECT ")
                    .unwrap()
                    .split(" FROM ")
                    .next()
                    .unwrap(),
                fields
            );
            assert!(query.ends_with("WHERE (VerifiedBuild>0)=?"));
            assert!(!query.contains('*') && !query.contains("locale"));
        }
        assert!(QUERIES[0].contains(" FROM map "));
        assert!(QUERIES[1].contains(" FROM power_type "));
        assert!(QUERIES[2].contains(" FROM chr_specialization "));
        assert!(QUERIES[3].contains(" FROM chr_classes_x_power_types "));
        assert!(QUERIES[4].contains(" FROM movie "));
        assert!(QUERIES[5].contains(" FROM chr_classes "));
        assert!(QUERIES[6].contains(" FROM chr_races "));
    }
}
