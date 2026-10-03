use super::*;

#[test]
fn value_input_decoders_require_complete_sql_rows() {
    let empty = SqlResult::empty();
    assert!(value_inputs::expected_stat(&empty).is_err());
    assert!(value_inputs::expected_stat_mod(&empty).is_err());
    assert!(value_inputs::content_tuning(&empty).is_err());
    assert!(value_inputs::content_tuning_x_expected(&empty).is_err());
    assert!(value_inputs::rand_prop_points(&empty).is_err());
    assert!(value_inputs::mythic_plus_season(&empty).is_err());
}

#[test]
fn six_dependency_decoders_reject_missing_rows_instead_of_defaulting_fields() {
    let empty = SqlResult::empty();
    assert!(custom_sources::talent(&empty).is_err());
    assert!(custom_sources::spell_item_enchantment(&empty).is_err());
    assert!(custom_sources::spell_visual(&empty).is_err());
    assert!(custom_sources::spell_visual_missile(&empty).is_err());
    assert!(custom_sources::spell_visual_effect_name(&empty).is_err());
    assert!(custom_sources::liquid_type(&empty).is_err());
    assert!(custom_sources::unit_condition(&empty).is_err());
    assert_eq!(
        QUERIES
            .iter()
            .map(|query| query.split(" FROM ").next().unwrap().split(',').count())
            .sum::<usize>(),
        668
    );
}

#[test]
fn all_forty_nine_queries_preserve_exact_source_projection_and_batch_predicate() {
    let expected = [
        ("spell_name", "ID,Name"),
        (
            "spell_effect",
            "ID,EffectAura,DifficultyID,EffectIndex,Effect,EffectAmplitude,EffectAttributes,EffectAuraPeriod,EffectBonusCoefficient,EffectChainAmplitude,EffectChainTargets,EffectItemType,EffectMechanic,EffectPointsPerResource,EffectPosFacing,EffectRealPointsPerLevel,EffectTriggerSpell,BonusCoefficientFromAP,PvpMultiplier,Coefficient,Variance,ResourceCoefficient,GroupSizeBasePointsCoefficient,EffectBasePoints,ScalingClass,TargetNodeGraph,EffectMiscValue1,EffectMiscValue2,EffectRadiusIndex1,EffectRadiusIndex2,EffectSpellClassMask1,EffectSpellClassMask2,EffectSpellClassMask3,EffectSpellClassMask4,ImplicitTarget1,ImplicitTarget2,SpellID",
        ),
        (
            "spell_misc",
            "ID,Attributes1,Attributes2,Attributes3,Attributes4,Attributes5,Attributes6,Attributes7,Attributes8,Attributes9,Attributes10,Attributes11,Attributes12,Attributes13,Attributes14,Attributes15,Attributes16,Attributes17,DifficultyID,CastingTimeIndex,DurationIndex,PvPDurationIndex,RangeIndex,SchoolMask,Speed,LaunchDelay,MinDuration,SpellIconFileDataID,ActiveIconFileDataID,ContentTuningID,ShowFutureSpellPlayerConditionID,SpellVisualScript,ActiveSpellVisualScript,SpellID",
        ),
        (
            "spell_aura_options",
            "ID,DifficultyID,CumulativeAura,ProcCategoryRecovery,ProcChance,ProcCharges,SpellProcsPerMinuteID,ProcTypeMask1,ProcTypeMask2,SpellID",
        ),
        (
            "spell_aura_restrictions",
            "ID,DifficultyID,CasterAuraState,TargetAuraState,ExcludeCasterAuraState,ExcludeTargetAuraState,CasterAuraSpell,TargetAuraSpell,ExcludeCasterAuraSpell,ExcludeTargetAuraSpell,CasterAuraType,TargetAuraType,ExcludeCasterAuraType,ExcludeTargetAuraType,SpellID",
        ),
        (
            "spell_casting_requirements",
            "ID,SpellID,FacingCasterFlags,MinFactionID,MinReputation,RequiredAreasID,RequiredAuraVision,RequiresSpellFocus",
        ),
        (
            "spell_categories",
            "ID,DifficultyID,Category,DefenseType,DiminishType,DispelType,Mechanic,PreventionType,StartRecoveryCategory,ChargeCategory,SpellID",
        ),
        (
            "spell_class_options",
            "ID,SpellID,ModalNextSpell,SpellClassSet,SpellClassMask1,SpellClassMask2,SpellClassMask3,SpellClassMask4",
        ),
        (
            "spell_cooldowns",
            "ID,DifficultyID,CategoryRecoveryTime,RecoveryTime,StartRecoveryTime,AuraSpellID,SpellID",
        ),
        ("spell_empower", "ID,SpellID,Unused1000"),
        ("spell_empower_stage", "ID,Stage,DurationMs,SpellEmpowerID"),
        (
            "spell_equipped_items",
            "ID,SpellID,EquippedItemClass,EquippedItemInvTypes,EquippedItemSubclass",
        ),
        (
            "spell_interrupts",
            "ID,DifficultyID,InterruptFlags,AuraInterruptFlags1,AuraInterruptFlags2,ChannelInterruptFlags1,ChannelInterruptFlags2,SpellID",
        ),
        ("spell_label", "ID,LabelID,SpellID"),
        (
            "spell_levels",
            "ID,DifficultyID,MaxLevel,MaxPassiveAuraLevel,BaseLevel,SpellLevel,SpellID",
        ),
        (
            "spell_power",
            "ID,OrderIndex,ManaCost,ManaCostPerLevel,ManaPerSecond,PowerDisplayID,AltPowerBarID,PowerCostPct,PowerCostMaxPct,OptionalCostPct,PowerPctPerSecond,PowerType,RequiredAuraSpellID,OptionalCost,SpellID",
        ),
        ("spell_power_difficulty", "ID,DifficultyID,OrderIndex"),
        (
            "spell_reagents",
            "ID,SpellID,Reagent1,Reagent2,Reagent3,Reagent4,Reagent5,Reagent6,Reagent7,Reagent8,ReagentCount1,ReagentCount2,ReagentCount3,ReagentCount4,ReagentCount5,ReagentCount6,ReagentCount7,ReagentCount8,ReagentRecraftCount1,ReagentRecraftCount2,ReagentRecraftCount3,ReagentRecraftCount4,ReagentRecraftCount5,ReagentRecraftCount6,ReagentRecraftCount7,ReagentRecraftCount8,ReagentSource1,ReagentSource2,ReagentSource3,ReagentSource4,ReagentSource5,ReagentSource6,ReagentSource7,ReagentSource8",
        ),
        (
            "spell_reagents_currency",
            "ID,SpellID,CurrencyTypesID,CurrencyCount,OverrideRecraftCurrencyCount,OrderSource",
        ),
        (
            "spell_scaling",
            "ID,SpellID,MinScalingLevel,MaxScalingLevel",
        ),
        (
            "spell_shapeshift",
            "ID,SpellID,StanceBarOrder,ShapeshiftExclude1,ShapeshiftExclude2,ShapeshiftMask1,ShapeshiftMask2",
        ),
        (
            "spell_target_restrictions",
            "ID,DifficultyID,ConeDegrees,MaxTargets,MaxTargetLevel,TargetCreatureType,Targets,Width,SpellID",
        ),
        (
            "spell_totems",
            "ID,SpellID,RequiredTotemCategoryID1,RequiredTotemCategoryID2,Totem1,Totem2",
        ),
        (
            "spell_x_spell_visual",
            "ID,DifficultyID,SpellVisualID,Probability,Flags,Priority,SpellIconFileID,ActiveIconFileID,ViewerUnitConditionID,ViewerPlayerConditionID,CasterUnitConditionID,CasterPlayerConditionID,SpellID",
        ),
        (
            "difficulty",
            "ID,Name,InstanceType,OrderIndex,OldEnumValue,FallbackDifficultyID,MinPlayers,MaxPlayers,Flags,ItemContext,ToggleDifficultyID,GroupSizeHealthCurveID,GroupSizeDmgCurveID,GroupSizeSpellPointsCurveID,Unknown1105",
        ),
        ("spell_cast_times", "ID,Base,Minimum"),
        (
            "spell_duration",
            "ID,Duration,MaxDuration,DurationPerResource",
        ),
        (
            "spell_range",
            "ID,DisplayName,DisplayNameShort,Flags,RangeMin1,RangeMin2,RangeMax1,RangeMax2",
        ),
        (
            "spell_radius",
            "ID,Radius,RadiusPerLevel,RadiusMin,RadiusMax",
        ),
        ("spell_procs_per_minute", "ID,BaseProcRate,Flags"),
        (
            "spell_procs_per_minute_mod",
            "ID,Type,Param,Coeff,Field_12_1_5_69594_003,SpellProcsPerMinuteID",
        ),
        (
            "spell_learn_spell",
            "ID,SpellID,LearnSpellID,OverridesSpellID",
        ),
        (
            "spell_shapeshift_form",
            "ID,Name,CreatureDisplayID,CreatureType,Flags,AttackIconFileID,BonusActionBar,CombatRoundTime,DamageVariance,MountTypeID,PresetSpellID1,PresetSpellID2,PresetSpellID3,PresetSpellID4,PresetSpellID5,PresetSpellID6,PresetSpellID7,PresetSpellID8",
        ),
        (
            "summon_properties",
            "ID,Control,Faction,Title,Slot,Flags1,Flags2",
        ),
        (
            "battle_pet_species",
            "Description,SourceText,ID,CreatureID,SummonSpellID,IconFileDataID,PetTypeEnum,Flags,SourceTypeEnum,CardUIModelSceneID,LoadoutUIModelSceneID,CovenantID",
        ),
        (
            "spell_category",
            "ID,Name,Flags,UsesPerWeek,MaxCharges,ChargeRecoveryTime,TypeMask",
        ),
        (
            "talent",
            "ID,Description,TierID,Flags,ColumnIndex,TabID,ClassID,SpecID,SpellID,OverridesSpellID,RequiredSpellID,CategoryMask1,CategoryMask2,SpellRank1,SpellRank2,SpellRank3,SpellRank4,SpellRank5,SpellRank6,SpellRank7,SpellRank8,SpellRank9,PrereqTalent1,PrereqTalent2,PrereqTalent3,PrereqRank1,PrereqRank2,PrereqRank3",
        ),
        (
            "spell_item_enchantment",
            "ID,Name,HordeName,Duration,Charges,Effect1,Effect2,Effect3,EffectPointsMin1,EffectPointsMin2,EffectPointsMin3,EffectArg1,EffectArg2,EffectArg3,Flags,EffectScalingPoints1,EffectScalingPoints2,EffectScalingPoints3,ScalingClass,ScalingClassRestricted,ConditionID,RequiredSkillID,RequiredSkillRank,MinLevel,MaxLevel,IconFileDataID,MinItemLevel,MaxItemLevel,TransmogUseConditionID,TransmogCost,Field_12_1_5_69594_021,ItemVisual,ItemLevel",
        ),
        (
            "spell_visual",
            "ID,MissileCastOffset1,MissileCastOffset2,MissileCastOffset3,MissileImpactOffset1,MissileImpactOffset2,MissileImpactOffset3,StateKit,AnimEventSoundID,Flags,MissileAttachment,MissileDestinationAttachment,MissileCastPositionerID,MissileImpactPositionerID,MissileTargetingKit,HostileSpellVisualID,CasterSpellVisualID,SpellVisualMissileSetID,DamageNumberDelay,LowViolenceSpellVisualID,RaidSpellVisualMissileSetID,ReducedUnexpectedCameraMovementSpellVisualID",
        ),
        (
            "spell_visual_missile",
            "CastOffset1,CastOffset2,CastOffset3,ImpactOffset1,ImpactOffset2,ImpactOffset3,ID,SpellVisualEffectNameID,SoundEntriesID,Attachment,DestinationAttachment,CastPositionerID,ImpactPositionerID,FollowGroundHeight,FollowGroundDropSpeed,FollowGroundApproach,Flags,SpellMissileMotionID,AnimKitID,ClutterLevel,DecayTimeAfterImpact,Unused1100,Field_12_1_5_69594_018,Field_12_1_5_69594_019,Field_12_1_5_69594_020,Field_12_1_5_69594_021,SpellVisualMissileSetID",
        ),
        (
            "spell_visual_effect_name",
            "ID,ModelFileDataID,BaseMissileSpeed,Scale,MinAllowedScale,MaxAllowedScale,Alpha,Flags,TextureFileDataID,EffectRadius,Type,GenericID,RibbonQualityID,DissolveEffectID,ModelPosition,Unknown901,Unknown1100",
        ),
        (
            "liquid_type",
            "ID,Name,Texture1,Texture2,Texture3,Texture4,Texture5,Texture6,Flags,SoundBank,SoundID,SpellID,MaxDarkenDepth,FogDarkenIntensity,AmbDarkenIntensity,DirDarkenIntensity,LightID,ParticleScale,ParticleMovement,ParticleTexSlots,MaterialID,MinimapStaticCol,FrameCountTexture1,FrameCountTexture2,FrameCountTexture3,FrameCountTexture4,FrameCountTexture5,FrameCountTexture6,Color1,Color2,Color3,Float1,Float2,Float3,`Float4`,Float5,Float6,Float7,`Float8`,Float9,Float10,Float11,Float12,Float13,Float14,Float15,Float16,Float17,Float18,Float19,Float20,Float21,Float22,Float23,Float24,Float25,Float26,Float27,Float28,Float29,Float30,Float31,Float32,Float33,Float34,Float35,Float36,Float37,Float38,`Int1`,`Int2`,`Int3`,`Int4`,Coefficient1,Coefficient2,Coefficient3,Coefficient4",
        ),
        (
            "expected_stat",
            "ID,ExpansionID,CreatureHealth,PlayerHealth,CreatureAutoAttackDps,CreatureArmor,PlayerMana,PlayerPrimaryStat,PlayerSecondaryStat,ArmorConstant,CreatureSpellDamage,ContentSetID,Lvl",
        ),
        (
            "expected_stat_mod",
            "ID,CreatureHealthMod,PlayerHealthMod,CreatureAutoAttackDPSMod,CreatureArmorMod,PlayerManaMod,PlayerPrimaryStatMod,PlayerSecondaryStatMod,ArmorConstantMod,CreatureSpellDamageMod",
        ),
        (
            "content_tuning",
            "ID,Flags,ExpansionID,HealthItemLevelCurveID,DamageItemLevelCurveID,HealthPrimaryStatCurveID,DamagePrimaryStatCurveID,PrimaryStatScalingModPlayerDataElementCharacterID,PrimaryStatScalingModPlayerDataElementCharacterMultiplier,MinLevel,MaxLevel,MinLevelType,MaxLevelType,TargetLevelDelta,TargetLevelMaxDelta,TargetLevelMin,TargetLevelMax,MinItemLevel,QuestXpMultiplier",
        ),
        (
            "content_tuning_x_expected",
            "ID,ExpectedStatModID,MinMythicPlusSeasonID,MaxMythicPlusSeasonID,ContentTuningID",
        ),
        (
            "rand_prop_points",
            "ID,DamageReplaceStatF,DamageSecondaryF,DamageReplaceStat,DamageSecondary,EpicF1,EpicF2,EpicF3,EpicF4,EpicF5,SuperiorF1,SuperiorF2,SuperiorF3,SuperiorF4,SuperiorF5,GoodF1,GoodF2,GoodF3,GoodF4,GoodF5,Epic1,Epic2,Epic3,Epic4,Epic5,Superior1,Superior2,Superior3,Superior4,Superior5,Good1,Good2,Good3,Good4,Good5",
        ),
        (
            "mythic_plus_season",
            "ID,MilestoneSeason,StartTimeEvent,ExpansionLevel,HeroicLFGDungeonMinGear",
        ),
        (
            "unit_condition",
            "ID,Flags,Variable1,Variable2,Variable3,Variable4,Variable5,Variable6,Variable7,Variable8,Op1,Op2,Op3,Op4,Op5,Op6,Op7,Op8,Value1,Value2,Value3,Value4,Value5,Value6,Value7,Value8",
        ),
    ];
    assert_eq!(QUERIES.len(), expected.len());
    for (query, (table, columns)) in QUERIES.iter().zip(expected) {
        let compact = query.replace(", ", ",");
        assert_eq!(
            compact,
            format!("SELECT {columns} FROM {table} WHERE (`VerifiedBuild` > 0) = ?")
        );
        assert!(!query.contains("ORDER BY") && !query.contains("locale"));
    }
    assert!(QUERIES[2].contains("Attributes17"));
    assert_eq!(
        QUERIES[34]
            .split(" FROM ")
            .next()
            .unwrap()
            .split(',')
            .count(),
        12
    );
}

#[test]
fn all_eight_locale_queries_preserve_source_text_order_without_sorting() {
    let expected = [
        "SELECT ID, Name_lang FROM spell_name_locale WHERE (`VerifiedBuild` > 0) = ? AND locale = ?",
        "SELECT ID, Name_lang FROM difficulty_locale WHERE (`VerifiedBuild` > 0) = ? AND locale = ?",
        "SELECT ID, DisplayName_lang, DisplayNameShort_lang FROM spell_range_locale WHERE (`VerifiedBuild` > 0) = ? AND locale = ?",
        "SELECT ID, Name_lang FROM spell_shapeshift_form_locale WHERE (`VerifiedBuild` > 0) = ? AND locale = ?",
        "SELECT ID, Description_lang, SourceText_lang FROM battle_pet_species_locale WHERE (`VerifiedBuild` > 0) = ? AND locale = ?",
        "SELECT ID, Name_lang FROM spell_category_locale WHERE (`VerifiedBuild` > 0) = ? AND locale = ?",
        "SELECT ID, Description_lang FROM talent_locale WHERE (`VerifiedBuild` > 0) = ? AND locale = ?",
        "SELECT ID, Name_lang, HordeName_lang FROM spell_item_enchantment_locale WHERE (`VerifiedBuild` > 0) = ? AND locale = ?",
    ];
    assert_eq!(locales::QUERIES, expected);
    for query in locales::QUERIES {
        assert!(query.contains("WHERE (`VerifiedBuild` > 0) = ? AND locale = ?"));
        assert!(!query.contains("ORDER BY"));
    }
}
