//! Composition of immutable C++ ObjectMgr query catalogs.

use anyhow::{Result, anyhow};
use wow_persistence::{
    WorldQueryCatalogLoadOutcomeLikeCpp, WorldQueryCatalogPersistencePortLikeCpp,
};

fn loaded<T>(outcome: WorldQueryCatalogLoadOutcomeLikeCpp<T>) -> Result<T> {
    match outcome {
        WorldQueryCatalogLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        WorldQueryCatalogLoadOutcomeLikeCpp::Failed { reason } => Err(anyhow!(reason)),
    }
}

pub(crate) async fn load_like_cpp(
    port: &dyn WorldQueryCatalogPersistencePortLikeCpp,
) -> Result<(
    wow_data::CreatureQueryCatalogLikeCpp,
    wow_data::GameObjectQueryCatalogLikeCpp,
    wow_data::PageTextCatalogLikeCpp,
)> {
    let creature_rows = loaded(port.load_creature_query_catalog_like_cpp().await)?;
    let mut displays = std::collections::HashMap::<u32, Vec<_>>::new();
    for row in creature_rows.displays {
        displays
            .entry(row.entry)
            .or_default()
            .push(wow_data::CreatureQueryDisplayLikeCpp {
                display_id: row.display_id,
                scale: row.scale,
                probability: row.probability,
            });
    }
    let creatures = wow_data::CreatureQueryCatalogLikeCpp::from_rows_like_cpp(
        creature_rows
            .templates
            .into_iter()
            .map(|row| wow_data::CreatureQueryTemplateLikeCpp {
                entry: row.entry,
                name: row.name,
                subname: row.subname,
                title_alt: row.title_alt,
                icon_name: row.icon_name,
                creature_type: row.creature_type,
                creature_family: row.creature_family,
                classification: row.classification,
                kill_credits: row.kill_credits,
                civilian: row.civilian,
                racial_leader: row.racial_leader,
                movement_id: row.movement_id,
                required_expansion: row.required_expansion,
                vignette_id: row.vignette_id,
                unit_class: row.unit_class,
                widget_set_id: row.widget_set_id,
                widget_set_unit_condition_id: row.widget_set_unit_condition_id,
                hp_multi: row.hp_multi,
                energy_multi: row.energy_multi,
                creature_difficulty_id: row.creature_difficulty_id,
                type_flags: row.type_flags,
                displays: displays.remove(&row.entry).unwrap_or_default(),
            }),
        creature_rows
            .locales
            .into_iter()
            .map(|row| wow_data::CreatureQueryLocaleLikeCpp {
                entry: row.entry,
                locale: row.locale,
                name: row.name,
                subname: row.subname,
                title_alt: row.title_alt,
            }),
    );

    let gameobject_rows = loaded(port.load_gameobject_query_catalog_like_cpp().await)?;
    let gameobjects = wow_data::GameObjectQueryCatalogLikeCpp::from_rows_like_cpp(
        gameobject_rows
            .templates
            .into_iter()
            .map(|row| wow_data::GameObjectQueryTemplateLikeCpp {
                entry: row.entry,
                go_type: row.go_type,
                display_id: row.display_id,
                name: row.name,
                icon_name: row.icon_name,
                cast_bar_caption: row.cast_bar_caption,
                unk_string: row.unk_string,
                size: row.size,
                data: row.data,
                content_tuning_id: row.content_tuning_id,
                min_money: row.min_money,
                max_money: row.max_money,
            }),
        gameobject_rows
            .locales
            .into_iter()
            .map(|row| wow_data::GameObjectQueryLocaleLikeCpp {
                entry: row.entry,
                locale: row.locale,
                name: row.name,
                cast_bar_caption: row.cast_bar_caption,
                unk_string: row.unk_string,
            }),
    );

    let page_rows = loaded(port.load_page_text_catalog_like_cpp().await)?;
    let pages = wow_data::PageTextCatalogLikeCpp::from_rows_like_cpp(
        page_rows
            .pages
            .into_iter()
            .map(|row| wow_data::PageTextLikeCpp {
                id: row.id,
                text: row.text,
                next_page_id: row.next_page_id,
                player_condition_id: row.player_condition_id,
                flags: row.flags,
            }),
        page_rows
            .locales
            .into_iter()
            .map(|row| wow_data::PageTextLocaleLikeCpp {
                id: row.id,
                locale: row.locale,
                text: row.text,
            }),
    );

    Ok((creatures, gameobjects, pages))
}
