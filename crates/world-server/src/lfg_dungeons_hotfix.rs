//! Composition boundary for the effective `LFGDungeons.db2` authority.

use anyhow::{Context, Result, bail};
use tracing::info;
use wow_persistence::{
    LfgDungeonsHotfixLoadOutcomeLikeCpp, LfgDungeonsHotfixPersistencePortLikeCpp,
    LfgDungeonsHotfixRowLikeCpp,
};

fn lfg_dungeons_entry_like_cpp(row: LfgDungeonsHotfixRowLikeCpp) -> wow_data::LfgDungeonsEntry {
    wow_data::LfgDungeonsEntry {
        id: row.id,
        name: row.name,
        description: row.description,
        min_level: row.min_level,
        max_level: row.max_level,
        type_id: row.type_id,
        subtype: row.subtype,
        faction: row.faction,
        icon_texture_file_id: row.icon_texture_file_id,
        rewards_bg_texture_file_id: row.rewards_bg_texture_file_id,
        popup_bg_texture_file_id: row.popup_bg_texture_file_id,
        expansion_level: row.expansion_level,
        map_id: row.map_id,
        difficulty_id: row.difficulty_id,
        min_gear: row.min_gear,
        group_id: row.group_id,
        order_index: row.order_index,
        required_player_condition_id: row.required_player_condition_id,
        target_level: row.target_level,
        target_level_min: row.target_level_min,
        target_level_max: row.target_level_max,
        random_id: row.random_id,
        scenario_id: row.scenario_id,
        final_encounter_id: row.final_encounter_id,
        count_tank: row.count_tank,
        count_healer: row.count_healer,
        count_damage: row.count_damage,
        min_count_tank: row.min_count_tank,
        min_count_healer: row.min_count_healer,
        min_count_damage: row.min_count_damage,
        bonus_reputation_amount: row.bonus_reputation_amount,
        mentor_item_level: row.mentor_item_level,
        mentor_char_level: row.mentor_char_level,
        flags: row.flags,
    }
}

async fn overlay_lfg_dungeons_like_cpp(
    mut store: wow_data::LfgDungeonsStore,
    persistence: &dyn LfgDungeonsHotfixPersistencePortLikeCpp,
) -> Result<wow_data::LfgDungeonsStore> {
    let rows = match persistence.load_lfg_dungeons_hotfix_rows_like_cpp().await {
        LfgDungeonsHotfixLoadOutcomeLikeCpp::Loaded(rows) => rows,
        LfgDungeonsHotfixLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    };
    let count =
        store.apply_hotfix_entries_like_cpp(rows.into_iter().map(lfg_dungeons_entry_like_cpp));
    if count != 0 {
        info!("Loaded {count} LFGDungeons hotfix rows");
    }
    Ok(store)
}

pub(super) async fn load_lfg_dungeons_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn LfgDungeonsHotfixPersistencePortLikeCpp,
) -> Result<wow_data::LfgDungeonsStore> {
    let store = wow_data::LfgDungeonsStore::load(data_dir, locale)
        .context("Failed to load LFGDungeons.db2")?;
    overlay_lfg_dungeons_like_cpp(store, persistence)
        .await
        .context("Failed to load LFGDungeons hotfix rows")
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_persistence::PersistenceFutureLikeCpp;

    struct StubPort(LfgDungeonsHotfixLoadOutcomeLikeCpp);

    impl LfgDungeonsHotfixPersistencePortLikeCpp for StubPort {
        fn load_lfg_dungeons_hotfix_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<'_, LfgDungeonsHotfixLoadOutcomeLikeCpp> {
            Box::pin(async move { self.0.clone() })
        }
    }

    fn row(id: u32, name: &str) -> LfgDungeonsHotfixRowLikeCpp {
        LfgDungeonsHotfixRowLikeCpp {
            id,
            name: name.into(),
            description: "description".into(),
            min_level: 1,
            max_level: 80,
            type_id: 6,
            subtype: 2,
            faction: -1,
            icon_texture_file_id: -2,
            rewards_bg_texture_file_id: -3,
            popup_bg_texture_file_id: -4,
            expansion_level: 3,
            map_id: -5,
            difficulty_id: 4,
            min_gear: 123.5,
            group_id: 5,
            order_index: 6,
            required_player_condition_id: 7,
            target_level: 8,
            target_level_min: 9,
            target_level_max: 10,
            random_id: 11,
            scenario_id: 12,
            final_encounter_id: 13,
            count_tank: 14,
            count_healer: 15,
            count_damage: 16,
            min_count_tank: 17,
            min_count_healer: 18,
            min_count_damage: 19,
            bonus_reputation_amount: 20,
            mentor_item_level: 21,
            mentor_char_level: 22,
            flags: [-23, 24],
        }
    }

    #[tokio::test]
    async fn typed_hotfix_replaces_whole_entry_and_preserves_every_field() {
        let base =
            wow_data::LfgDungeonsStore::from_entries([lfg_dungeons_entry_like_cpp(row(1, "base"))]);
        let expected = lfg_dungeons_entry_like_cpp(row(1, "replacement"));
        let store = overlay_lfg_dungeons_like_cpp(
            base,
            &StubPort(LfgDungeonsHotfixLoadOutcomeLikeCpp::Loaded(vec![row(
                1,
                "replacement",
            )])),
        )
        .await
        .unwrap();

        assert_eq!(store.len(), 1);
        assert_eq!(store.get(1), Some(&expected));
    }

    #[tokio::test]
    async fn empty_success_preserves_db2_only_store() {
        let base =
            wow_data::LfgDungeonsStore::from_entries([lfg_dungeons_entry_like_cpp(row(1, "base"))]);
        let store = overlay_lfg_dungeons_like_cpp(
            base,
            &StubPort(LfgDungeonsHotfixLoadOutcomeLikeCpp::Loaded(Vec::new())),
        )
        .await
        .unwrap();

        assert_eq!(store.get(1).map(|entry| entry.name.as_str()), Some("base"));
    }

    #[tokio::test]
    async fn failed_hotfix_read_returns_no_publishable_store() {
        let base =
            wow_data::LfgDungeonsStore::from_entries([lfg_dungeons_entry_like_cpp(row(1, "base"))]);
        let result = overlay_lfg_dungeons_like_cpp(
            base,
            &StubPort(LfgDungeonsHotfixLoadOutcomeLikeCpp::Failed {
                reason: "decode failed".into(),
            }),
        )
        .await;

        let error = match result {
            Ok(_) => panic!("a failed overlay must not publish the DB2-only store"),
            Err(error) => error,
        };
        assert_eq!(error.to_string(), "decode failed");
    }
}
