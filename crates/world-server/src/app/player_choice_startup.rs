use anyhow::Context;
use tracing::info;

pub(super) async fn load_player_choice_startup(
    player_choice_catalog_persistence: &dyn wow_persistence::PlayerChoiceCatalogPersistencePortLikeCpp,
    char_titles_store: &wow_data::Db2IdStore,
    quest_package_item_store: &wow_data::progression_rewards::QuestPackageItemStore,
    skill_line_store: &wow_data::SkillLineStore,
    item_stats_store: &wow_data::ItemStatsStore,
    currency_types_store: &wow_data::CurrencyTypesStore,
    faction_store: &wow_data::Db2IdStore,
) -> anyhow::Result<(
    wow_data::PlayerChoiceLoadOutcomeLikeCpp,
    wow_data::PlayerChoiceLocaleLoadReportLikeCpp,
)> {
    let mut player_choice_outcome = crate::player::choice_catalog::load_core_like_cpp(
        player_choice_catalog_persistence,
        |title_id| char_titles_store.contains(title_id),
        |package_id| {
            quest_package_item_store
                .quest_package_items_like_cpp(package_id)
                .next()
                .is_some()
                || quest_package_item_store
                    .quest_package_items_fallback_like_cpp(package_id)
                    .next()
                    .is_some()
        },
        |skill_line_id| {
            skill_line_store.contains_effective_record_like_cpp(skill_line_id)
        },
        |item_id| item_stats_store.sparse_template(item_id).is_some(),
        |currency_id| currency_types_store.has_record(currency_id),
        |faction_id| faction_store.contains(faction_id),
    )
    .await
    .context(
        "Failed to load C++ playerchoice/playerchoice_response/playerchoice_response_reward/playerchoice_response_reward_item/playerchoice_response_reward_currency/playerchoice_response_reward_faction/playerchoice_response_reward_item_choice/playerchoice_response_maw_power rows",
    )?;
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_responses_missing_choice
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response` references non-existing ChoiceId: {} (ResponseId: {}), skipped",
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome.report.skipped_rewards_missing_choice {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward` references non-existing ChoiceId: {} (ResponseId: {}), skipped",
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_rewards_missing_response
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward` references non-existing ResponseId: {} for ChoiceId {}, skipped",
            response_id,
            choice_id
        );
    }
    for (choice_id, response_id, title_id) in &player_choice_outcome.report.invalid_reward_titles {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward` references non-existing Title {} for ChoiceId {}, ResponseId: {}, set to 0",
            title_id,
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id, package_id) in
        &player_choice_outcome.report.invalid_reward_packages
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward` references non-existing QuestPackage {} for ChoiceId {}, ResponseId: {}, set to 0",
            package_id,
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id, skill_line_id) in
        &player_choice_outcome.report.invalid_reward_skill_lines
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward` references non-existing SkillLine {} for ChoiceId {}, ResponseId: {}, set to 0",
            skill_line_id,
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_reward_items_missing_choice
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_item` references non-existing ChoiceId: {} (ResponseId: {}), skipped",
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_reward_items_missing_response
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_item` references non-existing ResponseId: {} for ChoiceId {}, skipped",
            response_id,
            choice_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_reward_items_missing_reward
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_item` references non-existing player choice reward for ChoiceId {}, ResponseId: {}, skipped",
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id, item_id) in &player_choice_outcome
        .report
        .skipped_reward_items_missing_item
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_item` references non-existing item {} for ChoiceId {}, ResponseId: {}, skipped",
            item_id,
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_reward_currencies_missing_choice
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_currency` references non-existing ChoiceId: {} (ResponseId: {}), skipped",
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_reward_currencies_missing_response
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_currency` references non-existing ResponseId: {} for ChoiceId {}, skipped",
            response_id,
            choice_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_reward_currencies_missing_reward
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_currency` references non-existing player choice reward for ChoiceId {}, ResponseId: {}, skipped",
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id, currency_id) in &player_choice_outcome
        .report
        .skipped_reward_currencies_missing_currency
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_currency` references non-existing currency {} for ChoiceId {}, ResponseId: {}, skipped",
            currency_id,
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_reward_factions_missing_choice
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_faction` references non-existing ChoiceId: {} (ResponseId: {}), skipped",
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_reward_factions_missing_response
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_faction` references non-existing ResponseId: {} for ChoiceId {}, skipped",
            response_id,
            choice_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_reward_factions_missing_reward
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_faction` references non-existing player choice reward for ChoiceId {}, ResponseId: {}, skipped",
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id, faction_id) in &player_choice_outcome
        .report
        .skipped_reward_factions_missing_faction
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_faction` references non-existing faction {} for ChoiceId {}, ResponseId: {}, skipped",
            faction_id,
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_reward_item_choices_missing_choice
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_item_choice` references non-existing ChoiceId: {} (ResponseId: {}), skipped",
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_reward_item_choices_missing_response
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_item_choice` references non-existing ResponseId: {} for ChoiceId {}, skipped",
            response_id,
            choice_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_reward_item_choices_missing_reward
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_item_choice` references non-existing player choice reward for ChoiceId {}, ResponseId: {}, skipped",
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id, item_id) in &player_choice_outcome
        .report
        .skipped_reward_item_choices_missing_item
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_item_choice` references non-existing item {} for ChoiceId {}, ResponseId: {}, skipped",
            item_id,
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_maw_powers_missing_choice
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_maw_power` references non-existing ChoiceId: {} (ResponseId: {}), skipped",
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_maw_powers_missing_response
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_maw_power` references non-existing ResponseId: {} for ChoiceId {}, skipped",
            response_id,
            choice_id
        );
    }
    info!(
        "Loaded {} C++ player choices with {} responses, {} base rewards, {} reward items, {} reward currencies, {} reward factions, {} reward item choices, and {} maw powers ({} skipped responses, {} skipped rewards, {} skipped reward items, {} skipped reward currencies, {} skipped reward factions, {} skipped reward item choices, {} skipped maw powers, {} invalid reward refs; live DisplayPlayerChoice flow pending)",
        player_choice_outcome.report.choice_rows_seen,
        player_choice_outcome.report.loaded_responses,
        player_choice_outcome.report.loaded_rewards,
        player_choice_outcome.report.loaded_reward_items,
        player_choice_outcome.report.loaded_reward_currencies,
        player_choice_outcome.report.loaded_reward_factions,
        player_choice_outcome.report.loaded_reward_item_choices,
        player_choice_outcome.report.loaded_maw_powers,
        player_choice_outcome
            .report
            .skipped_responses_missing_choice
            .len(),
        player_choice_outcome
            .report
            .skipped_rewards_missing_choice
            .len()
            + player_choice_outcome
                .report
                .skipped_rewards_missing_response
                .len(),
        player_choice_outcome
            .report
            .skipped_reward_items_missing_choice
            .len()
            + player_choice_outcome
                .report
                .skipped_reward_items_missing_response
                .len()
            + player_choice_outcome
                .report
                .skipped_reward_items_missing_reward
                .len()
            + player_choice_outcome
                .report
                .skipped_reward_items_missing_item
                .len(),
        player_choice_outcome
            .report
            .skipped_reward_currencies_missing_choice
            .len()
            + player_choice_outcome
                .report
                .skipped_reward_currencies_missing_response
                .len()
            + player_choice_outcome
                .report
                .skipped_reward_currencies_missing_reward
                .len()
            + player_choice_outcome
                .report
                .skipped_reward_currencies_missing_currency
                .len(),
        player_choice_outcome
            .report
            .skipped_reward_factions_missing_choice
            .len()
            + player_choice_outcome
                .report
                .skipped_reward_factions_missing_response
                .len()
            + player_choice_outcome
                .report
                .skipped_reward_factions_missing_reward
                .len()
            + player_choice_outcome
                .report
                .skipped_reward_factions_missing_faction
                .len(),
        player_choice_outcome
            .report
            .skipped_reward_item_choices_missing_choice
            .len()
            + player_choice_outcome
                .report
                .skipped_reward_item_choices_missing_response
                .len()
            + player_choice_outcome
                .report
                .skipped_reward_item_choices_missing_reward
                .len()
            + player_choice_outcome
                .report
                .skipped_reward_item_choices_missing_item
                .len(),
        player_choice_outcome
            .report
            .skipped_maw_powers_missing_choice
            .len()
            + player_choice_outcome
                .report
                .skipped_maw_powers_missing_response
                .len(),
        player_choice_outcome.report.invalid_reward_titles.len()
            + player_choice_outcome.report.invalid_reward_packages.len()
            + player_choice_outcome
                .report
                .invalid_reward_skill_lines
                .len()
    );
    let player_choice_locale_report = crate::player::choice_catalog::load_locales_like_cpp(
        &mut player_choice_outcome.store,
        player_choice_catalog_persistence,
    )
    .await
    .context("Failed to load C++ playerchoice_locale/playerchoice_response_locale rows")?;
    for (choice_id, locale_name) in
        &player_choice_locale_report.skipped_choice_locales_missing_choice
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_locale` references non-existing ChoiceId: {} for locale {}, skipped",
            choice_id,
            locale_name
        );
    }
    for (choice_id, response_id, locale_name) in
        &player_choice_locale_report.skipped_response_locales_missing_choice_locale
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_locale` references non-existing ChoiceId: {} for ResponseId {} locale {}, skipped",
            choice_id,
            response_id,
            locale_name
        );
    }
    for (choice_id, response_id, locale_name) in
        &player_choice_locale_report.skipped_response_locales_missing_response
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_locale` references non-existing ResponseId: {} for ChoiceId {} locale {}, skipped",
            response_id,
            choice_id,
            locale_name
        );
    }
    info!(
        "Loaded {} Player Choice locale strings ({} rows seen)",
        player_choice_locale_report.loaded_choice_locale_entries,
        player_choice_locale_report.choice_locale_rows_seen
    );
    info!(
        "Loaded {} Player Choice Response locale strings ({} rows seen)",
        player_choice_locale_report.loaded_response_locale_rows,
        player_choice_locale_report.response_locale_rows_seen
    );
    Ok((player_choice_outcome, player_choice_locale_report))
}
