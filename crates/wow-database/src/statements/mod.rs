//! Database statement definitions.
//!
//! Each database has its own statement enum that maps variant → SQL string.
//! The [`StatementDef`] trait is implemented by each enum.

pub mod character;
pub mod hotfix;
pub mod login;
pub mod world;

pub use character::CharStatements;
pub use hotfix::{
    HOTFIX_STATEMENT_STRATEGY_LIKE_CPP, HotfixStatementStrategyLikeCpp, HotfixStatements,
};
pub use login::LoginStatements;
pub use world::WorldStatements;

/// Trait implemented by database statement enums.
///
/// Each variant maps to a static SQL string via [`sql()`](Self::sql).
/// The type parameter on [`Database<S>`](crate::Database) ensures that only
/// the correct statement type can be used with each database connection.
pub trait StatementDef: Copy + Send + Sync + 'static {
    /// Get the SQL string for this statement variant.
    ///
    /// Returns an empty string for variants that have no registered SQL
    /// (should not happen in a correctly configured server).
    fn sql(self) -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_statements_have_sql() {
        // Verify key BNet statements have non-empty SQL
        assert!(!LoginStatements::SEL_REALMLIST.sql().is_empty());
        assert!(!LoginStatements::SEL_BNET_AUTHENTICATION.sql().is_empty());
        assert!(!LoginStatements::SEL_BNET_ACCOUNT_INFO.sql().is_empty());
        assert!(!LoginStatements::UPD_BNET_LAST_LOGIN_INFO.sql().is_empty());
        assert!(!LoginStatements::DEL_EXPIRED_IP_BANS.sql().is_empty());
        assert!(!LoginStatements::INS_ACCOUNT.sql().is_empty());
    }

    #[test]
    fn login_sql_contains_expected_tables() {
        assert!(LoginStatements::SEL_REALMLIST.sql().contains("realmlist"));
        assert!(
            LoginStatements::SEL_BNET_AUTHENTICATION
                .sql()
                .contains("battlenet_accounts")
        );
        assert!(LoginStatements::INS_IP_BANNED.sql().contains("ip_banned"));
        assert!(LoginStatements::SEL_CHECK_PASSWORD.sql().contains("salt"));
    }

    #[test]
    fn login_unregistered_statement_is_empty() {
        // SEL_BNET_ACCOUNT_SALT_BY_ID has no SQL in C# source
        assert!(
            LoginStatements::SEL_BNET_ACCOUNT_SALT_BY_ID
                .sql()
                .is_empty()
        );
    }

    #[test]
    fn character_skill_statement_matches_cpp_shape() {
        let sql = CharStatements::SEL_CHARACTER_SKILLS.sql();
        assert!(sql.contains("skill, value, max, professionSlot"));
        assert!(sql.contains("character_skills"));
        assert_eq!(sql.matches('?').count(), 1);
    }

    #[test]
    fn world_statements_have_sql() {
        assert!(!WorldStatements::DEL_LINKED_RESPAWN.sql().is_empty());
        assert!(!WorldStatements::SEL_CREATURE_TEMPLATE.sql().is_empty());
        assert!(!WorldStatements::SEL_COMMANDS.sql().is_empty());
        assert!(!WorldStatements::INS_CREATURE.sql().is_empty());
    }

    #[test]
    fn gameobjects_in_range_sql_uses_cpp_override_template_addon_priority() {
        let sql = WorldStatements::SEL_GAMEOBJECTS_IN_RANGE.sql();
        assert!(sql.contains("LEFT JOIN gameobject_template_addon gta ON gta.entry = g.id"));
        assert!(sql.contains("LEFT JOIN gameobject_overrides goo ON goo.spawnId = g.guid"));
        assert!(sql.contains("COALESCE(goo.flags, gta.flags, 0)"));
        assert!(sql.contains("COALESCE(goo.faction, gta.faction, 0)"));
        assert!(sql.contains(
            "CASE WHEN goo.spawnId IS NOT NULL OR gta.entry IS NOT NULL THEN 1 ELSE 0 END"
        ));
        assert!(sql.find("goo.flags").unwrap() < sql.find("gta.flags").unwrap());
        assert!(sql.find("goo.faction").unwrap() < sql.find("gta.faction").unwrap());
    }

    #[test]
    fn world_sql_contains_expected_tables() {
        assert!(
            WorldStatements::SEL_CREATURE_TEXT
                .sql()
                .contains("creature_text")
        );
        assert!(
            WorldStatements::SEL_SMART_SCRIPTS
                .sql()
                .contains("smart_scripts")
        );
        assert!(WorldStatements::INS_GAME_TELE.sql().contains("game_tele"));
        assert!(
            WorldStatements::SEL_ITEM_TEMPLATE_ADDON_MONEY_LOOT
                .sql()
                .contains("item_template_addon")
        );
        assert!(
            WorldStatements::SEL_GAMEOBJECT_TEMPLATE_ADDON_MONEY_LOOT
                .sql()
                .contains("gameobject_template_addon")
        );
        assert!(
            WorldStatements::SEL_GAMEOBJECT_TEMPLATE_LOCALE
                .sql()
                .contains("gameobject_template_locale")
        );
        assert!(
            WorldStatements::SEL_GAMEOBJECT_QUEST_ITEMS
                .sql()
                .contains("gameobject_questitem")
        );
        assert_eq!(
            WorldStatements::SEL_GAMEOBJECT_TEMPLATE_LOCALE
                .sql()
                .matches('?')
                .count(),
            2
        );
        assert_eq!(
            WorldStatements::SEL_GAMEOBJECT_QUEST_ITEMS
                .sql()
                .matches('?')
                .count(),
            1
        );
        assert!(
            WorldStatements::SEL_ITEM_TEMPLATE_ADDON_LOOT_METADATA
                .sql()
                .contains("QuestLogItemId")
        );
        assert!(
            WorldStatements::SEL_QUEST_TEMPLATE
                .sql()
                .contains("ItemDrop1")
        );
        assert_eq!(
            WorldStatements::SEL_ITEM_TEMPLATE_ADDON_MONEY_LOOT
                .sql()
                .matches('?')
                .count(),
            1
        );
        assert_eq!(
            WorldStatements::SEL_GAMEOBJECT_TEMPLATE_ADDON_MONEY_LOOT
                .sql()
                .matches('?')
                .count(),
            1
        );
        assert_eq!(
            WorldStatements::SEL_ITEM_TEMPLATE_ADDON_LOOT_METADATA
                .sql()
                .matches('?')
                .count(),
            1
        );
        assert!(
            WorldStatements::SEL_ITEM_LOOT_TEMPLATE_ROWS
                .sql()
                .contains("item_loot_template")
        );
        assert_eq!(
            WorldStatements::SEL_ITEM_LOOT_TEMPLATE_ROWS
                .sql()
                .matches('?')
                .count(),
            1
        );
        assert!(
            WorldStatements::SEL_REFERENCE_LOOT_TEMPLATE_ROWS
                .sql()
                .contains("reference_loot_template")
        );
        assert_eq!(
            WorldStatements::SEL_REFERENCE_LOOT_TEMPLATE_ROWS
                .sql()
                .matches('?')
                .count(),
            1
        );
        for statement in [
            WorldStatements::SEL_CREATURE_LOOT_TEMPLATE_ROWS,
            WorldStatements::SEL_CREATURE_LOOT_TEMPLATE_ALL_ROWS,
            WorldStatements::SEL_FISHING_LOOT_TEMPLATE_ROWS,
            WorldStatements::SEL_FISHING_LOOT_TEMPLATE_ALL_ROWS,
            WorldStatements::SEL_GAMEOBJECT_LOOT_TEMPLATE_ROWS,
            WorldStatements::SEL_GAMEOBJECT_LOOT_TEMPLATE_ALL_ROWS,
            WorldStatements::SEL_MAIL_LOOT_TEMPLATE_ROWS,
            WorldStatements::SEL_MAIL_LOOT_TEMPLATE_ALL_ROWS,
            WorldStatements::SEL_MILLING_LOOT_TEMPLATE_ROWS,
            WorldStatements::SEL_MILLING_LOOT_TEMPLATE_ALL_ROWS,
            WorldStatements::SEL_PICKPOCKETING_LOOT_TEMPLATE_ROWS,
            WorldStatements::SEL_PICKPOCKETING_LOOT_TEMPLATE_ALL_ROWS,
            WorldStatements::SEL_PROSPECTING_LOOT_TEMPLATE_ROWS,
            WorldStatements::SEL_PROSPECTING_LOOT_TEMPLATE_ALL_ROWS,
            WorldStatements::SEL_SKINNING_LOOT_TEMPLATE_ROWS,
            WorldStatements::SEL_SKINNING_LOOT_TEMPLATE_ALL_ROWS,
            WorldStatements::SEL_DISENCHANT_LOOT_TEMPLATE_ROWS,
            WorldStatements::SEL_DISENCHANT_LOOT_TEMPLATE_ALL_ROWS,
            WorldStatements::SEL_SPELL_LOOT_TEMPLATE_ROWS,
            WorldStatements::SEL_SPELL_LOOT_TEMPLATE_ALL_ROWS,
        ] {
            let sql = statement.sql();
            assert!(sql.contains("_loot_template"));
            assert!(sql.contains("GroupId"));
            assert!(sql.matches('?').count() <= 1);
        }
        assert!(
            WorldStatements::SEL_LOOT_TEMPLATE_CONDITION_ROWS
                .sql()
                .contains("conditions")
        );
        assert!(
            WorldStatements::SEL_LOOT_TEMPLATE_CONDITION_ROWS
                .sql()
                .contains("SourceTypeOrReferenceId")
        );
        assert_eq!(
            WorldStatements::SEL_LOOT_TEMPLATE_CONDITION_ROWS
                .sql()
                .matches('?')
                .count(),
            3
        );
        assert!(
            WorldStatements::SEL_LOOT_TEMPLATE_CONDITION_IDS
                .sql()
                .contains("conditions")
        );
        assert!(
            WorldStatements::SEL_LOOT_TEMPLATE_CONDITION_IDS
                .sql()
                .contains("SourceTypeOrReferenceId BETWEEN 1 AND 12")
        );
        assert_eq!(
            WorldStatements::SEL_LOOT_TEMPLATE_CONDITION_IDS
                .sql()
                .matches('?')
                .count(),
            0
        );
        assert!(
            WorldStatements::SEL_LOOT_TEMPLATE_CONDITION_REFERENCE_USES
                .sql()
                .contains("ConditionTypeOrReference < 0")
        );
        assert!(
            WorldStatements::SEL_LOOT_TEMPLATE_CONDITION_REFERENCE_USES
                .sql()
                .contains("ConditionTypeOrReference <> SourceTypeOrReferenceId")
        );
        assert!(
            WorldStatements::SEL_LOOT_TEMPLATE_CONDITION_REFERENCE_USES
                .sql()
                .contains("SourceTypeOrReferenceId < 0")
        );
        assert!(
            WorldStatements::SEL_CONDITION_REFERENCE_TEMPLATE_IDS
                .sql()
                .contains("SourceTypeOrReferenceId < 0")
        );
        assert_eq!(
            WorldStatements::SEL_LOOT_TEMPLATE_CONDITION_REFERENCE_USES
                .sql()
                .matches('?')
                .count(),
            0
        );
        assert_eq!(
            WorldStatements::SEL_CONDITION_REFERENCE_TEMPLATE_IDS
                .sql()
                .matches('?')
                .count(),
            0
        );
        assert!(
            WorldStatements::SEL_ITEM_RANDOM_ENCHANTMENT_TEMPLATE
                .sql()
                .contains("item_random_enchantment_template")
        );
        assert_eq!(
            WorldStatements::SEL_ITEM_RANDOM_ENCHANTMENT_TEMPLATE
                .sql()
                .matches('?')
                .count(),
            0
        );
    }

    #[test]
    fn world_unregistered_statement_is_empty() {
        // SEL_GAMEOBJECT_TARGET has no SQL in C# source
        assert!(WorldStatements::SEL_GAMEOBJECT_TARGET.sql().is_empty());
    }

    #[test]
    fn hotfix_control_statements_match_cpp_tables() {
        assert!(
            HotfixStatements::SEL_HOTFIX_DATA
                .sql()
                .contains("hotfix_data")
        );
        assert!(
            HotfixStatements::SEL_HOTFIX_BLOB
                .sql()
                .contains("hotfix_blob")
        );
        assert!(
            HotfixStatements::SEL_HOTFIX_OPTIONAL_DATA
                .sql()
                .contains("hotfix_optional_data")
        );
        assert!(
            HotfixStatements::SEL_UI_MAP_X_MAP_ART
                .sql()
                .contains("ui_map_x_map_art")
        );
        assert!(
            HotfixStatements::SEL_AREA_TABLE
                .sql()
                .contains("area_table")
        );
        assert!(HotfixStatements::SEL_MOUNT.sql().contains("mount"));
        assert!(
            HotfixStatements::SEL_MOUNT_CAPABILITY
                .sql()
                .contains("mount_capability")
        );
        assert!(
            HotfixStatements::SEL_MOUNT_TYPE_X_CAPABILITY
                .sql()
                .contains("mount_type_x_capability")
        );
        assert!(
            HotfixStatements::SEL_MOUNT_X_DISPLAY
                .sql()
                .contains("mount_x_display")
        );
        assert!(HotfixStatements::SEL_PHASE.sql().contains("phase"));
        assert!(
            HotfixStatements::SEL_PHASE_X_PHASE_GROUP
                .sql()
                .contains("phase_x_phase_group")
        );
    }

    #[test]
    fn world_spawn_loader_statements_match_cpp_objectmgr_tables() {
        let creature = WorldStatements::SEL_CREATURE_SPAWNS.sql();
        assert!(creature.starts_with("SELECT creature.guid, id, map"));
        assert!(creature.contains("FROM creature"));
        assert!(creature.contains("LEFT OUTER JOIN game_event_creature"));
        assert!(creature.contains("pool_members.type = 0"));
        assert_eq!(creature.matches('?').count(), 0);

        let gameobject = WorldStatements::SEL_GAMEOBJECT_SPAWNS.sql();
        assert!(gameobject.starts_with("SELECT gameobject.guid, id, map"));
        assert!(gameobject.contains("FROM gameobject"));
        assert!(gameobject.contains("LEFT OUTER JOIN game_event_gameobject"));
        assert!(gameobject.contains("pool_members.type = 1"));
        assert_eq!(gameobject.matches('?').count(), 0);

        let areatrigger = WorldStatements::SEL_AREATRIGGER_SPAWNS.sql();
        assert!(areatrigger.contains("FROM `areatrigger`"));
        assert!(areatrigger.contains("SpawnId, AreaTriggerCreatePropertiesId"));
        assert_eq!(areatrigger.matches('?').count(), 0);

        assert_eq!(
            WorldStatements::SEL_PHASE_AREAS.sql(),
            "SELECT AreaId, PhaseId FROM `phase_area`"
        );
    }

    #[test]
    fn world_spawn_group_statements_match_cpp_objectmgr_tables() {
        assert_eq!(
            WorldStatements::SEL_SPAWN_GROUP_TEMPLATES.sql(),
            "SELECT groupId, groupName, groupFlags FROM spawn_group_template"
        );
        assert_eq!(
            WorldStatements::SEL_SPAWN_GROUP_MEMBERS.sql(),
            "SELECT groupId, spawnType, spawnId FROM spawn_group"
        );
        assert_eq!(
            WorldStatements::SEL_INSTANCE_SPAWN_GROUPS.sql(),
            "SELECT instanceMapId, bossStateId, bossStates, spawnGroupId, flags FROM instance_spawn_groups"
        );
    }

    #[test]
    fn login_sql_has_correct_placeholders() {
        // SEL_IP_INFO has 1 placeholder
        let sql = LoginStatements::SEL_IP_INFO.sql();
        assert_eq!(sql.matches('?').count(), 1);

        // INS_IP_BANNED has 4 placeholders
        let sql = LoginStatements::INS_IP_BANNED.sql();
        assert_eq!(sql.matches('?').count(), 4);

        // INS_ACCOUNT has 7 placeholders
        let sql = LoginStatements::INS_ACCOUNT.sql();
        assert_eq!(sql.matches('?').count(), 7);
    }
}
