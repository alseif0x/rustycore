//! Hotfix database prepared statement definitions.
//!
//! These correspond to the `hotfixes` database used by C++ `DB2Manager`.

use super::StatementDef;

/// Chosen RustyCore hotfix statement strategy.
///
/// C++ generates one prepared statement family per DB2 mirror table, while
/// `DB2Manager::LoadHotfixData/Blob/OptionalData` still reads the three control
/// tables directly. RustyCore keeps the control-table statements and ports DB2
/// mirror-table statements only when a typed store consumes them; the generated
/// helpers below preserve exact C++ SQL for broader coverage tests and future
/// store ports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotfixStatementStrategyLikeCpp {
    ControlTablesAndSelectedDb2Overlays,
}

pub const HOTFIX_STATEMENT_STRATEGY_LIKE_CPP: HotfixStatementStrategyLikeCpp =
    HotfixStatementStrategyLikeCpp::ControlTablesAndSelectedDb2Overlays;

/// Prepared statements for the hotfix database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
pub enum HotfixStatements {
    /// C++ `HOTFIX_SEL_AREA_TABLE`.
    SEL_AREA_TABLE,
    /// C++ `HOTFIX_SEL_MOUNT`.
    SEL_MOUNT,
    /// C++ `HOTFIX_SEL_MOUNT_CAPABILITY`.
    SEL_MOUNT_CAPABILITY,
    /// C++ `HOTFIX_SEL_MOUNT_TYPE_X_CAPABILITY`.
    SEL_MOUNT_TYPE_X_CAPABILITY,
    /// C++ `HOTFIX_SEL_MOUNT_X_DISPLAY`.
    SEL_MOUNT_X_DISPLAY,
    /// C++ `HOTFIX_SEL_CREATURE_DISPLAY_INFO`.
    SEL_CREATURE_DISPLAY_INFO,
    /// C++ `HOTFIX_SEL_CREATURE_MODEL_DATA`.
    SEL_CREATURE_MODEL_DATA,
    /// C++ `HOTFIX_SEL_VEHICLE`.
    SEL_VEHICLE,
    /// C++ `HOTFIX_SEL_VEHICLE_SEAT`.
    SEL_VEHICLE_SEAT,
    /// `DB2Manager::LoadHotfixData`.
    SEL_HOTFIX_DATA,
    /// `DB2Manager::LoadHotfixBlob`.
    SEL_HOTFIX_BLOB,
    /// `DB2Manager::LoadHotfixOptionalData`.
    SEL_HOTFIX_OPTIONAL_DATA,
    /// C++ `HOTFIX_SEL_PHASE`.
    SEL_PHASE,
    /// C++ `HOTFIX_SEL_PHASE_X_PHASE_GROUP`.
    SEL_PHASE_X_PHASE_GROUP,
    /// C++ `HOTFIX_SEL_UI_MAP_X_MAP_ART`.
    SEL_UI_MAP_X_MAP_ART,
    /// Generated C++ base hotfix statement.
    GENERATED_BASE {
        /// Exact SQL from C++ `PrepareStatement(HOTFIX_SEL_..., ...)`.
        sql: &'static str,
    },
    /// Generated C++ `*_MAX_ID` statement for a hotfix table.
    GENERATED_MAX_ID {
        /// Hotfix table name, e.g. `area_table`.
        table: &'static str,
    },
    /// Generated C++ `*_LOCALE` statement for a hotfix table.
    GENERATED_LOCALE {
        /// Base hotfix table name without `_locale`, e.g. `area_table`.
        table: &'static str,
        /// Locale columns after `ID`, preserving C++ selected order.
        columns: &'static str,
    },
}

impl HotfixStatements {
    /// Whether this statement backs C++ `DB2Manager::LoadHotfix*`.
    pub const fn is_control_table_like_cpp(self) -> bool {
        matches!(
            self,
            Self::SEL_HOTFIX_DATA | Self::SEL_HOTFIX_BLOB | Self::SEL_HOTFIX_OPTIONAL_DATA
        )
    }

    /// Whether this statement is a typed DB2 mirror-table overlay.
    pub const fn is_selected_overlay_like_cpp(self) -> bool {
        matches!(
            self,
            Self::SEL_AREA_TABLE
                | Self::SEL_MOUNT
                | Self::SEL_MOUNT_CAPABILITY
                | Self::SEL_MOUNT_TYPE_X_CAPABILITY
                | Self::SEL_MOUNT_X_DISPLAY
                | Self::SEL_CREATURE_DISPLAY_INFO
                | Self::SEL_CREATURE_MODEL_DATA
                | Self::SEL_VEHICLE
                | Self::SEL_VEHICLE_SEAT
                | Self::SEL_PHASE
                | Self::SEL_PHASE_X_PHASE_GROUP
                | Self::SEL_UI_MAP_X_MAP_ART
        )
    }

    /// Build a generated C++ base hotfix statement from exact SQL.
    pub const fn base(sql: &'static str) -> Self {
        Self::GENERATED_BASE { sql }
    }

    /// Build a generated C++ `PREPARE_MAX_ID_STMT` equivalent.
    ///
    /// C++ generates these as `SELECT MAX(ID) + 1 FROM <table>` immediately
    /// after the base hotfix statement.
    pub const fn max_id(table: &'static str) -> Self {
        Self::GENERATED_MAX_ID { table }
    }

    /// Build a generated C++ `PREPARE_LOCALE_STMT` equivalent.
    ///
    /// C++ generated locale statements select `ID` plus localized columns from
    /// `<table>_locale` with the same `VerifiedBuild`/`locale` parameters.
    pub const fn locale(table: &'static str, columns: &'static str) -> Self {
        Self::GENERATED_LOCALE { table, columns }
    }
}

impl StatementDef for HotfixStatements {
    fn sql(self) -> &'static str {
        match self {
            Self::SEL_AREA_TABLE => concat!(
                "SELECT ID, ZoneName, AreaName, ContinentID, ParentAreaID, AreaBit, SoundProviderPref, ",
                "SoundProviderPrefUnderwater, AmbienceID, UwAmbience, ZoneMusic, UwZoneMusic, ExplorationLevel, IntroSound, UwIntroSound, FactionGroupMask, ",
                "AmbientMultiplier, MountFlags, PvpCombatWorldStateID, WildBattlePetLevelMin, WildBattlePetLevelMax, WindSettingsID, Flags1, Flags2, ",
                "LiquidTypeID1, LiquidTypeID2, LiquidTypeID3, LiquidTypeID4 FROM area_table WHERE VerifiedBuild > 0"
            ),
            Self::SEL_MOUNT => concat!(
                "SELECT Name, SourceText, Description, ID, MountTypeID, Flags, SourceTypeEnum, SourceSpellID, ",
                "PlayerConditionID, MountFlyRideHeight, UiModelSceneID FROM mount WHERE VerifiedBuild > 0"
            ),
            Self::SEL_MOUNT_CAPABILITY => concat!(
                "SELECT ID, Flags, ReqRidingSkill, ReqAreaID, ReqSpellAuraID, ReqSpellKnownID, ModSpellAuraID, ",
                "ReqMapID FROM mount_capability WHERE VerifiedBuild > 0"
            ),
            Self::SEL_MOUNT_TYPE_X_CAPABILITY => {
                "SELECT ID, MountTypeID, MountCapabilityID, OrderIndex FROM mount_type_x_capability WHERE VerifiedBuild > 0"
            }
            Self::SEL_MOUNT_X_DISPLAY => {
                "SELECT ID, CreatureDisplayInfoID, PlayerConditionID, MountID FROM mount_x_display WHERE VerifiedBuild > 0"
            }
            Self::SEL_CREATURE_DISPLAY_INFO => concat!(
                "SELECT ID, ModelID, SoundID, SizeClass, CreatureModelScale, CreatureModelAlpha, BloodID, ",
                "ExtendedDisplayInfoID, NPCSoundID, ParticleColorID, PortraitCreatureDisplayInfoID, ",
                "PortraitTextureFileDataID, ObjectEffectPackageID, AnimReplacementSetID, Flags, ",
                "StateSpellVisualKitID, PlayerOverrideScale, PetInstanceScale, UnarmedWeaponType, ",
                "MountPoofSpellVisualKitID, DissolveEffectID, Gender, DissolveOutEffectID, CreatureModelMinLod, ",
                "TextureVariationFileDataID1, TextureVariationFileDataID2, TextureVariationFileDataID3, ",
                "TextureVariationFileDataID4 FROM creature_display_info WHERE VerifiedBuild > 0"
            ),
            Self::SEL_CREATURE_MODEL_DATA => concat!(
                "SELECT ID, GeoBox1, GeoBox2, GeoBox3, GeoBox4, GeoBox5, GeoBox6, Flags, FileDataID, ",
                "BloodID, FootprintTextureID, FootprintTextureLength, FootprintTextureWidth, FootprintParticleScale, ",
                "FoleyMaterialID, FootstepCameraEffectID, DeathThudCameraEffectID, SoundID, SizeClass, ",
                "CollisionWidth, CollisionHeight, WorldEffectScale, CreatureGeosetDataID, HoverHeight, ",
                "AttachedEffectScale, ModelScale, MissileCollisionRadius, MissileCollisionPush, MissileCollisionRaise, ",
                "MountHeight, OverrideLootEffectScale, OverrideNameScale, OverrideSelectionRadius, TamedPetBaseScale ",
                "FROM creature_model_data WHERE VerifiedBuild > 0"
            ),
            Self::SEL_VEHICLE => concat!(
                "SELECT ID, Flags, FlagsB, TurnSpeed, PitchSpeed, PitchMin, PitchMax, MouseLookOffsetPitch, ",
                "CameraFadeDistScalarMin, CameraFadeDistScalarMax, CameraPitchOffset, FacingLimitRight, ",
                "FacingLimitLeft, CameraYawOffset, VehicleUIIndicatorID, MissileTargetingID, VehiclePOITypeID, ",
                "UiLocomotionType, SeatID1, SeatID2, SeatID3, SeatID4, SeatID5, SeatID6, SeatID7, SeatID8, ",
                "PowerDisplayID1, PowerDisplayID2, PowerDisplayID3 FROM vehicle WHERE VerifiedBuild > 0"
            ),
            Self::SEL_VEHICLE_SEAT => concat!(
                "SELECT ID, AttachmentOffsetX, AttachmentOffsetY, AttachmentOffsetZ, CameraOffsetX, CameraOffsetY, ",
                "CameraOffsetZ, Flags, FlagsB, FlagsC, AttachmentID, EnterPreDelay, EnterSpeed, EnterGravity, ",
                "EnterMinDuration, EnterMaxDuration, EnterMinArcHeight, EnterMaxArcHeight, EnterAnimStart, ",
                "EnterAnimLoop, RideAnimStart, RideAnimLoop, RideUpperAnimStart, RideUpperAnimLoop, ExitPreDelay, ",
                "ExitSpeed, ExitGravity, ExitMinDuration, ExitMaxDuration, ExitMinArcHeight, ExitMaxArcHeight, ",
                "ExitAnimStart, ExitAnimLoop, ExitAnimEnd, VehicleEnterAnim, VehicleEnterAnimBone, VehicleExitAnim, ",
                "VehicleExitAnimBone, VehicleRideAnimLoop, VehicleRideAnimLoopBone, PassengerAttachmentID, ",
                "PassengerYaw, PassengerPitch, PassengerRoll, VehicleEnterAnimDelay, VehicleExitAnimDelay, ",
                "VehicleAbilityDisplay, EnterUISoundID, ExitUISoundID, UiSkinFileDataID, UiSkin, CameraEnteringDelay, ",
                "CameraEnteringDuration, CameraExitingDelay, CameraExitingDuration, CameraPosChaseRate, ",
                "CameraFacingChaseRate, CameraEnteringZoom, CameraSeatZoomMin, CameraSeatZoomMax, EnterAnimKitID, ",
                "RideAnimKitID, ExitAnimKitID, VehicleEnterAnimKitID, VehicleRideAnimKitID, VehicleExitAnimKitID, ",
                "CameraModeID FROM vehicle_seat WHERE VerifiedBuild > 0"
            ),
            Self::SEL_HOTFIX_DATA => {
                "SELECT Id, UniqueId, TableHash, RecordId, Status FROM hotfix_data ORDER BY Id"
            }
            Self::SEL_HOTFIX_BLOB => {
                "SELECT TableHash, RecordId, locale, `Blob` FROM hotfix_blob ORDER BY TableHash"
            }
            Self::SEL_HOTFIX_OPTIONAL_DATA => {
                "SELECT TableHash, RecordId, locale, `Key`, `Data` FROM hotfix_optional_data ORDER BY TableHash"
            }
            Self::SEL_PHASE => "SELECT ID, Flags FROM phase WHERE VerifiedBuild > 0",
            Self::SEL_PHASE_X_PHASE_GROUP => {
                "SELECT ID, PhaseID, PhaseGroupID FROM phase_x_phase_group WHERE VerifiedBuild > 0"
            }
            Self::SEL_UI_MAP_X_MAP_ART => {
                "SELECT ID, PhaseID, UiMapArtID, UiMapID FROM ui_map_x_map_art WHERE VerifiedBuild > 0"
            }
            Self::GENERATED_BASE { sql } => sql,
            Self::GENERATED_MAX_ID { table } => {
                Box::leak(format!("SELECT MAX(ID) + 1 FROM {table}").into_boxed_str())
            }
            Self::GENERATED_LOCALE { table, columns } => Box::leak(
                format!(
                    "SELECT ID, {columns} FROM {table}_locale WHERE (`VerifiedBuild` > 0) = ? AND locale = ?"
                )
                .into_boxed_str(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HOTFIX_STATEMENT_STRATEGY_LIKE_CPP, HotfixStatementStrategyLikeCpp, HotfixStatements,
    };
    use crate::statements::StatementDef;

    fn cpp_hotfix_database_cpp() -> &'static str {
        "/home/server/woltk-trinity-legacy/src/server/database/Database/Implementation/HotfixDatabase.cpp"
    }

    fn cpp_max_id_tables() -> Vec<String> {
        let contents = std::fs::read_to_string(cpp_hotfix_database_cpp())
            .expect("C++ HotfixDatabase.cpp must be available for parity tests");
        let marker = "SELECT MAX(ID) + 1 FROM ";
        contents
            .lines()
            .filter_map(|line| {
                if !line.contains("PREPARE_MAX_ID_STMT") {
                    return None;
                }
                let start = line.find(marker)? + marker.len();
                let tail = &line[start..];
                let end = tail.find('"')?;
                Some(tail[..end].to_string())
            })
            .collect()
    }

    fn cpp_string_literals(block: &str) -> String {
        let mut output = String::new();
        let bytes = block.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] != b'"' {
                i += 1;
                continue;
            }

            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    if i + 1 < bytes.len() {
                        output.push(bytes[i + 1] as char);
                        i += 2;
                        continue;
                    }
                }
                if bytes[i] == b'"' {
                    i += 1;
                    break;
                }
                output.push(bytes[i] as char);
                i += 1;
            }
        }
        output
    }

    fn cpp_locale_sql() -> Vec<String> {
        let contents = std::fs::read_to_string(cpp_hotfix_database_cpp())
            .expect("C++ HotfixDatabase.cpp must be available for parity tests");
        let mut sql = Vec::new();
        let mut offset = 0;
        while let Some(relative_start) = contents[offset..].find("PREPARE_LOCALE_STMT(") {
            let start = offset + relative_start;
            let Some(relative_end) = contents[start..].find("CONNECTION_SYNCH);") else {
                break;
            };
            let end = start + relative_end + "CONNECTION_SYNCH);".len();
            let block = &contents[start..end];
            if block.contains("SELECT ID") {
                sql.push(cpp_string_literals(block));
            }
            offset = end;
        }
        sql
    }

    fn cpp_base_sql() -> Vec<String> {
        let contents = std::fs::read_to_string(cpp_hotfix_database_cpp())
            .expect("C++ HotfixDatabase.cpp must be available for parity tests");
        let mut sql = Vec::new();
        let mut offset = 0;
        while let Some(relative_start) = contents[offset..].find("PrepareStatement(HOTFIX_SEL_") {
            let start = offset + relative_start;
            let Some(relative_end) = contents[start..].find("CONNECTION_SYNCH);") else {
                break;
            };
            let end = start + relative_end + "CONNECTION_SYNCH);".len();
            let block = &contents[start..end];
            sql.push(cpp_string_literals(block));
            offset = end;
        }
        sql
    }

    fn locale_parts(sql: &str) -> (&str, &str) {
        let rest = sql
            .strip_prefix("SELECT ID, ")
            .expect("C++ locale SQL must select ID first");
        let from = rest
            .find(" FROM ")
            .expect("C++ locale SQL must contain FROM");
        let columns = &rest[..from];
        let table_tail = &rest[from + " FROM ".len()..];
        let table = table_tail
            .strip_suffix("_locale WHERE (`VerifiedBuild` > 0) = ? AND locale = ?")
            .expect("C++ locale SQL must target table_locale with VerifiedBuild and locale");
        (table, columns)
    }

    #[test]
    fn hotfix_strategy_is_control_tables_plus_selected_overlays() {
        assert_eq!(
            HOTFIX_STATEMENT_STRATEGY_LIKE_CPP,
            HotfixStatementStrategyLikeCpp::ControlTablesAndSelectedDb2Overlays
        );
        assert!(HotfixStatements::SEL_HOTFIX_DATA.is_control_table_like_cpp());
        assert!(HotfixStatements::SEL_HOTFIX_BLOB.is_control_table_like_cpp());
        assert!(HotfixStatements::SEL_HOTFIX_OPTIONAL_DATA.is_control_table_like_cpp());
        assert!(!HotfixStatements::SEL_AREA_TABLE.is_control_table_like_cpp());
        assert!(HotfixStatements::SEL_AREA_TABLE.is_selected_overlay_like_cpp());
        assert!(HotfixStatements::SEL_VEHICLE_SEAT.is_selected_overlay_like_cpp());
        assert!(!HotfixStatements::SEL_HOTFIX_DATA.is_selected_overlay_like_cpp());
    }

    #[test]
    fn generated_max_id_statements_cover_cpp_hotfix_tables() {
        let tables = cpp_max_id_tables();
        assert_eq!(tables.len(), 325);

        for table in tables {
            let table: &'static str = Box::leak(table.into_boxed_str());
            assert_eq!(
                HotfixStatements::max_id(table).sql(),
                format!("SELECT MAX(ID) + 1 FROM {table}")
            );
        }
    }

    #[test]
    fn generated_base_statements_cover_cpp_hotfix_tables() {
        let statements = cpp_base_sql();
        assert_eq!(statements.len(), 325);

        for cpp_sql in statements {
            let sql: &'static str = Box::leak(cpp_sql.into_boxed_str());
            assert_eq!(HotfixStatements::base(sql).sql(), sql);
            assert!(sql.contains(" WHERE (`VerifiedBuild` > 0) = ?"));
        }
    }

    #[test]
    fn generated_locale_statements_cover_cpp_hotfix_tables() {
        let statements = cpp_locale_sql();
        assert_eq!(statements.len(), 95);

        for cpp_sql in statements {
            let (table, columns) = locale_parts(&cpp_sql);
            let table: &'static str = Box::leak(table.to_string().into_boxed_str());
            let columns: &'static str = Box::leak(columns.to_string().into_boxed_str());
            assert_eq!(HotfixStatements::locale(table, columns).sql(), cpp_sql);
        }
    }
}
