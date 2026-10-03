//! Complete target spell hotfix batches; read-only, not a SQL snapshot.
//! 02245dcd HotfixDatabase.cpp and DB2Store::LoadFromDB/DB2DatabaseLoader.
mod core;
mod costs;
mod custom_sources;
mod dependencies;
mod locales;
mod row;
mod storage_bounds;
mod value_inputs;
use super::{ForeverHotfixRepository, LoadError, PreparedStatement};
use row::SpellRow;
use wow_persistence::forever::spells::{SpellOverlays, SpellRows};

const QUERIES: [&str; 49] = [
    "SELECT ID, Name FROM spell_name WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, EffectAura, DifficultyID, EffectIndex, Effect, EffectAmplitude, EffectAttributes, EffectAuraPeriod, EffectBonusCoefficient, EffectChainAmplitude, EffectChainTargets, EffectItemType, EffectMechanic, EffectPointsPerResource, EffectPosFacing, EffectRealPointsPerLevel, EffectTriggerSpell, BonusCoefficientFromAP, PvpMultiplier, Coefficient, Variance, ResourceCoefficient, GroupSizeBasePointsCoefficient, EffectBasePoints, ScalingClass, TargetNodeGraph, EffectMiscValue1, EffectMiscValue2, EffectRadiusIndex1, EffectRadiusIndex2, EffectSpellClassMask1, EffectSpellClassMask2, EffectSpellClassMask3, EffectSpellClassMask4, ImplicitTarget1, ImplicitTarget2, SpellID FROM spell_effect WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, Attributes1, Attributes2, Attributes3, Attributes4, Attributes5, Attributes6, Attributes7, Attributes8, Attributes9, Attributes10, Attributes11, Attributes12, Attributes13, Attributes14, Attributes15, Attributes16, Attributes17, DifficultyID, CastingTimeIndex, DurationIndex, PvPDurationIndex, RangeIndex, SchoolMask, Speed, LaunchDelay, MinDuration, SpellIconFileDataID, ActiveIconFileDataID, ContentTuningID, ShowFutureSpellPlayerConditionID, SpellVisualScript, ActiveSpellVisualScript, SpellID FROM spell_misc WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, DifficultyID, CumulativeAura, ProcCategoryRecovery, ProcChance, ProcCharges, SpellProcsPerMinuteID, ProcTypeMask1, ProcTypeMask2, SpellID FROM spell_aura_options WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, DifficultyID, CasterAuraState, TargetAuraState, ExcludeCasterAuraState, ExcludeTargetAuraState, CasterAuraSpell, TargetAuraSpell, ExcludeCasterAuraSpell, ExcludeTargetAuraSpell, CasterAuraType, TargetAuraType, ExcludeCasterAuraType, ExcludeTargetAuraType, SpellID FROM spell_aura_restrictions WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, SpellID, FacingCasterFlags, MinFactionID, MinReputation, RequiredAreasID, RequiredAuraVision, RequiresSpellFocus FROM spell_casting_requirements WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, DifficultyID, Category, DefenseType, DiminishType, DispelType, Mechanic, PreventionType, StartRecoveryCategory, ChargeCategory, SpellID FROM spell_categories WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, SpellID, ModalNextSpell, SpellClassSet, SpellClassMask1, SpellClassMask2, SpellClassMask3, SpellClassMask4 FROM spell_class_options WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, DifficultyID, CategoryRecoveryTime, RecoveryTime, StartRecoveryTime, AuraSpellID, SpellID FROM spell_cooldowns WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, SpellID, Unused1000 FROM spell_empower WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, Stage, DurationMs, SpellEmpowerID FROM spell_empower_stage WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, SpellID, EquippedItemClass, EquippedItemInvTypes, EquippedItemSubclass FROM spell_equipped_items WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, DifficultyID, InterruptFlags, AuraInterruptFlags1, AuraInterruptFlags2, ChannelInterruptFlags1, ChannelInterruptFlags2, SpellID FROM spell_interrupts WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, LabelID, SpellID FROM spell_label WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, DifficultyID, MaxLevel, MaxPassiveAuraLevel, BaseLevel, SpellLevel, SpellID FROM spell_levels WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, OrderIndex, ManaCost, ManaCostPerLevel, ManaPerSecond, PowerDisplayID, AltPowerBarID, PowerCostPct, PowerCostMaxPct, OptionalCostPct, PowerPctPerSecond, PowerType, RequiredAuraSpellID, OptionalCost, SpellID FROM spell_power WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, DifficultyID, OrderIndex FROM spell_power_difficulty WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, SpellID, Reagent1, Reagent2, Reagent3, Reagent4, Reagent5, Reagent6, Reagent7, Reagent8, ReagentCount1, ReagentCount2, ReagentCount3, ReagentCount4, ReagentCount5, ReagentCount6, ReagentCount7, ReagentCount8, ReagentRecraftCount1, ReagentRecraftCount2, ReagentRecraftCount3, ReagentRecraftCount4, ReagentRecraftCount5, ReagentRecraftCount6, ReagentRecraftCount7, ReagentRecraftCount8, ReagentSource1, ReagentSource2, ReagentSource3, ReagentSource4, ReagentSource5, ReagentSource6, ReagentSource7, ReagentSource8 FROM spell_reagents WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, SpellID, CurrencyTypesID, CurrencyCount, OverrideRecraftCurrencyCount, OrderSource FROM spell_reagents_currency WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, SpellID, MinScalingLevel, MaxScalingLevel FROM spell_scaling WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, SpellID, StanceBarOrder, ShapeshiftExclude1, ShapeshiftExclude2, ShapeshiftMask1, ShapeshiftMask2 FROM spell_shapeshift WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, DifficultyID, ConeDegrees, MaxTargets, MaxTargetLevel, TargetCreatureType, Targets, Width, SpellID FROM spell_target_restrictions WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, SpellID, RequiredTotemCategoryID1, RequiredTotemCategoryID2, Totem1, Totem2 FROM spell_totems WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, DifficultyID, SpellVisualID, Probability, Flags, Priority, SpellIconFileID, ActiveIconFileID, ViewerUnitConditionID, ViewerPlayerConditionID, CasterUnitConditionID, CasterPlayerConditionID, SpellID FROM spell_x_spell_visual WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, Name, InstanceType, OrderIndex, OldEnumValue, FallbackDifficultyID, MinPlayers, MaxPlayers, Flags, ItemContext, ToggleDifficultyID, GroupSizeHealthCurveID, GroupSizeDmgCurveID, GroupSizeSpellPointsCurveID, Unknown1105 FROM difficulty WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, Base, Minimum FROM spell_cast_times WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, Duration, MaxDuration, DurationPerResource FROM spell_duration WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, DisplayName, DisplayNameShort, Flags, RangeMin1, RangeMin2, RangeMax1, RangeMax2 FROM spell_range WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, Radius, RadiusPerLevel, RadiusMin, RadiusMax FROM spell_radius WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, BaseProcRate, Flags FROM spell_procs_per_minute WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, Type, Param, Coeff, Field_12_1_5_69594_003, SpellProcsPerMinuteID FROM spell_procs_per_minute_mod WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, SpellID, LearnSpellID, OverridesSpellID FROM spell_learn_spell WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, Name, CreatureDisplayID, CreatureType, Flags, AttackIconFileID, BonusActionBar, CombatRoundTime, DamageVariance, MountTypeID, PresetSpellID1, PresetSpellID2, PresetSpellID3, PresetSpellID4, PresetSpellID5, PresetSpellID6, PresetSpellID7, PresetSpellID8 FROM spell_shapeshift_form WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, Control, Faction, Title, Slot, Flags1, Flags2 FROM summon_properties WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT Description, SourceText, ID, CreatureID, SummonSpellID, IconFileDataID, PetTypeEnum, Flags, SourceTypeEnum, CardUIModelSceneID, LoadoutUIModelSceneID, CovenantID FROM battle_pet_species WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, Name, Flags, UsesPerWeek, MaxCharges, ChargeRecoveryTime, TypeMask FROM spell_category WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, Description, TierID, Flags, ColumnIndex, TabID, ClassID, SpecID, SpellID, OverridesSpellID, RequiredSpellID, CategoryMask1, CategoryMask2, SpellRank1, SpellRank2, SpellRank3, SpellRank4, SpellRank5, SpellRank6, SpellRank7, SpellRank8, SpellRank9, PrereqTalent1, PrereqTalent2, PrereqTalent3, PrereqRank1, PrereqRank2, PrereqRank3 FROM talent WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, Name, HordeName, Duration, Charges, Effect1, Effect2, Effect3, EffectPointsMin1, EffectPointsMin2, EffectPointsMin3, EffectArg1, EffectArg2, EffectArg3, Flags, EffectScalingPoints1, EffectScalingPoints2, EffectScalingPoints3, ScalingClass, ScalingClassRestricted, ConditionID, RequiredSkillID, RequiredSkillRank, MinLevel, MaxLevel, IconFileDataID, MinItemLevel, MaxItemLevel, TransmogUseConditionID, TransmogCost, Field_12_1_5_69594_021, ItemVisual, ItemLevel FROM spell_item_enchantment WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, MissileCastOffset1, MissileCastOffset2, MissileCastOffset3, MissileImpactOffset1, MissileImpactOffset2, MissileImpactOffset3, StateKit, AnimEventSoundID, Flags, MissileAttachment, MissileDestinationAttachment, MissileCastPositionerID, MissileImpactPositionerID, MissileTargetingKit, HostileSpellVisualID, CasterSpellVisualID, SpellVisualMissileSetID, DamageNumberDelay, LowViolenceSpellVisualID, RaidSpellVisualMissileSetID, ReducedUnexpectedCameraMovementSpellVisualID FROM spell_visual WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT CastOffset1, CastOffset2, CastOffset3, ImpactOffset1, ImpactOffset2, ImpactOffset3, ID, SpellVisualEffectNameID, SoundEntriesID, Attachment, DestinationAttachment, CastPositionerID, ImpactPositionerID, FollowGroundHeight, FollowGroundDropSpeed, FollowGroundApproach, Flags, SpellMissileMotionID, AnimKitID, ClutterLevel, DecayTimeAfterImpact, Unused1100, Field_12_1_5_69594_018, Field_12_1_5_69594_019, Field_12_1_5_69594_020, Field_12_1_5_69594_021, SpellVisualMissileSetID FROM spell_visual_missile WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, ModelFileDataID, BaseMissileSpeed, Scale, MinAllowedScale, MaxAllowedScale, Alpha, Flags, TextureFileDataID, EffectRadius, Type, GenericID, RibbonQualityID, DissolveEffectID, ModelPosition, Unknown901, Unknown1100 FROM spell_visual_effect_name WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, Name, Texture1, Texture2, Texture3, Texture4, Texture5, Texture6, Flags, SoundBank, SoundID, SpellID, MaxDarkenDepth, FogDarkenIntensity, AmbDarkenIntensity, DirDarkenIntensity, LightID, ParticleScale, ParticleMovement, ParticleTexSlots, MaterialID, MinimapStaticCol, FrameCountTexture1, FrameCountTexture2, FrameCountTexture3, FrameCountTexture4, FrameCountTexture5, FrameCountTexture6, Color1, Color2, Color3, Float1, Float2, Float3, `Float4`, Float5, Float6, Float7, `Float8`, Float9, Float10, Float11, Float12, Float13, Float14, Float15, Float16, Float17, Float18, Float19, Float20, Float21, Float22, Float23, Float24, Float25, Float26, Float27, Float28, Float29, Float30, Float31, Float32, Float33, Float34, Float35, Float36, Float37, Float38, `Int1`, `Int2`, `Int3`, `Int4`, Coefficient1, Coefficient2, Coefficient3, Coefficient4 FROM liquid_type WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, ExpansionID, CreatureHealth, PlayerHealth, CreatureAutoAttackDps, CreatureArmor, PlayerMana, PlayerPrimaryStat, PlayerSecondaryStat, ArmorConstant, CreatureSpellDamage, ContentSetID, Lvl FROM expected_stat WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, CreatureHealthMod, PlayerHealthMod, CreatureAutoAttackDPSMod, CreatureArmorMod, PlayerManaMod, PlayerPrimaryStatMod, PlayerSecondaryStatMod, ArmorConstantMod, CreatureSpellDamageMod FROM expected_stat_mod WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, Flags, ExpansionID, HealthItemLevelCurveID, DamageItemLevelCurveID, HealthPrimaryStatCurveID, DamagePrimaryStatCurveID, PrimaryStatScalingModPlayerDataElementCharacterID, PrimaryStatScalingModPlayerDataElementCharacterMultiplier, MinLevel, MaxLevel, MinLevelType, MaxLevelType, TargetLevelDelta, TargetLevelMaxDelta, TargetLevelMin, TargetLevelMax, MinItemLevel, QuestXpMultiplier FROM content_tuning WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, ExpectedStatModID, MinMythicPlusSeasonID, MaxMythicPlusSeasonID, ContentTuningID FROM content_tuning_x_expected WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, DamageReplaceStatF, DamageSecondaryF, DamageReplaceStat, DamageSecondary, EpicF1, EpicF2, EpicF3, EpicF4, EpicF5, SuperiorF1, SuperiorF2, SuperiorF3, SuperiorF4, SuperiorF5, GoodF1, GoodF2, GoodF3, GoodF4, GoodF5, Epic1, Epic2, Epic3, Epic4, Epic5, Superior1, Superior2, Superior3, Superior4, Superior5, Good1, Good2, Good3, Good4, Good5 FROM rand_prop_points WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, MilestoneSeason, StartTimeEvent, ExpansionLevel, HeroicLFGDungeonMinGear FROM mythic_plus_season WHERE (`VerifiedBuild` > 0) = ?",
    "SELECT ID, Flags, Variable1, Variable2, Variable3, Variable4, Variable5, Variable6, Variable7, Variable8, Op1, Op2, Op3, Op4, Op5, Op6, Op7, Op8, Value1, Value2, Value3, Value4, Value5, Value6, Value7, Value8 FROM unit_condition WHERE (`VerifiedBuild` > 0) = ?",
];

impl ForeverHotfixRepository {
    /// Official then custom for each source store. Every query/typed decode
    /// succeeds before the caller receives either batch; no partial publication.
    /// Legal repeated IDs preserve query order, without ID/build sorting.
    pub async fn load_spell_overlays(&self) -> Result<SpellOverlays, LoadError> {
        let mut rows = SpellOverlays {
            official: SpellRows::default(),
            custom: SpellRows::default(),
        };
        for (table, sql) in QUERIES.iter().enumerate() {
            for custom in [false, true] {
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
                let target = if custom {
                    &mut rows.custom
                } else {
                    &mut rows.official
                };
                if table == 46 {
                    if result.field_count() != value_inputs::RAND_PROP_COLUMNS {
                        return Err(LoadError::InvalidRow);
                    }
                    // Source DB2DatabaseLoader::Load queries this only after
                    // a nonempty main result and before publishing new IDs.
                    // No VerifiedBuild filter and no snapshot/union inference.
                    let statement = PreparedStatement::new(storage_bounds::RAND_PROP_SIZE_QUERY);
                    let size = self
                        .0
                        .query(&statement)
                        .await
                        .map_err(|_| LoadError::Database)?;
                    target.rand_prop_points_sql_index_size = Some(storage_bounds::decode(&size)?);
                }
                loop {
                    match table {
                        0 => target.spell_names.push(core::spell_name(&result)?),
                        1 => target.spell_effects.push(core::spell_effect(&result)?),
                        2 => target.spell_misc.push(core::spell_misc(&result)?),
                        3 => target
                            .spell_aura_options
                            .push(core::spell_aura_options(&result)?),
                        4 => target
                            .spell_aura_restrictions
                            .push(core::spell_aura_restrictions(&result)?),
                        5 => target
                            .spell_casting_requirements
                            .push(core::spell_casting_requirements(&result)?),
                        6 => target
                            .spell_categories
                            .push(core::spell_categories(&result)?),
                        7 => target
                            .spell_class_options
                            .push(core::spell_class_options(&result)?),
                        8 => target.spell_cooldowns.push(core::spell_cooldowns(&result)?),
                        9 => target.spell_empowers.push(costs::spell_empower(&result)?),
                        10 => target
                            .spell_empower_stages
                            .push(costs::spell_empower_stage(&result)?),
                        11 => target
                            .spell_equipped_items
                            .push(core::spell_equipped_items(&result)?),
                        12 => target
                            .spell_interrupts
                            .push(core::spell_interrupts(&result)?),
                        13 => target.spell_labels.push(core::spell_label(&result)?),
                        14 => target.spell_levels.push(core::spell_levels(&result)?),
                        15 => target.spell_powers.push(costs::spell_power(&result)?),
                        16 => target
                            .spell_power_difficulties
                            .push(costs::spell_power_difficulty(&result)?),
                        17 => target.spell_reagents.push(costs::spell_reagents(&result)?),
                        18 => target
                            .spell_reagents_currencies
                            .push(costs::spell_reagents_currency(&result)?),
                        19 => target.spell_scaling.push(costs::spell_scaling(&result)?),
                        20 => target
                            .spell_shapeshifts
                            .push(costs::spell_shapeshift(&result)?),
                        21 => target
                            .spell_target_restrictions
                            .push(costs::spell_target_restrictions(&result)?),
                        22 => target.spell_totems.push(costs::spell_totems(&result)?),
                        23 => target
                            .spell_x_spell_visuals
                            .push(costs::spell_x_spell_visual(&result)?),
                        24 => target.difficulties.push(dependencies::difficulty(&result)?),
                        25 => target
                            .spell_cast_times
                            .push(dependencies::spell_cast_times(&result)?),
                        26 => target
                            .spell_durations
                            .push(dependencies::spell_duration(&result)?),
                        27 => target
                            .spell_ranges
                            .push(dependencies::spell_range(&result)?),
                        28 => target
                            .spell_radii
                            .push(dependencies::spell_radius(&result)?),
                        29 => target
                            .spell_procs_per_minute
                            .push(dependencies::spell_procs_per_minute(&result)?),
                        30 => target
                            .spell_procs_per_minute_mods
                            .push(dependencies::spell_procs_per_minute_mod(&result)?),
                        31 => target
                            .spell_learn_spells
                            .push(dependencies::spell_learn_spell(&result)?),
                        32 => target
                            .spell_shapeshift_forms
                            .push(dependencies::spell_shapeshift_form(&result)?),
                        33 => target
                            .summon_properties
                            .push(dependencies::summon_properties(&result)?),
                        34 => target
                            .battle_pet_species
                            .push(dependencies::battle_pet_species(&result)?),
                        35 => target
                            .spell_category_definitions
                            .push(dependencies::spell_category(&result)?),
                        36 => target.talents.push(custom_sources::talent(&result)?),
                        37 => target
                            .spell_item_enchantments
                            .push(custom_sources::spell_item_enchantment(&result)?),
                        38 => target
                            .spell_visuals
                            .push(custom_sources::spell_visual(&result)?),
                        39 => target
                            .spell_visual_missiles
                            .push(custom_sources::spell_visual_missile(&result)?),
                        40 => target
                            .spell_visual_effect_names
                            .push(custom_sources::spell_visual_effect_name(&result)?),
                        41 => target
                            .liquid_types
                            .push(custom_sources::liquid_type(&result)?),
                        42 => target
                            .expected_stats
                            .push(value_inputs::expected_stat(&result)?),
                        43 => target
                            .expected_stat_mods
                            .push(value_inputs::expected_stat_mod(&result)?),
                        44 => target
                            .content_tunings
                            .push(value_inputs::content_tuning(&result)?),
                        45 => target
                            .content_tuning_x_expected
                            .push(value_inputs::content_tuning_x_expected(&result)?),
                        46 => target
                            .rand_prop_points
                            .push(value_inputs::rand_prop_points(&result)?),
                        47 => target
                            .mythic_plus_seasons
                            .push(value_inputs::mythic_plus_season(&result)?),
                        48 => target
                            .unit_conditions
                            .push(custom_sources::unit_condition(&result)?),
                        _ => unreachable!(),
                    }
                    if !result.next_row() {
                        break;
                    }
                }
            }
        }
        Ok(rows)
    }
}

#[cfg(test)]
mod tests;
