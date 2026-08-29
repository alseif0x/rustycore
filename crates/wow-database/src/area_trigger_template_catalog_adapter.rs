//! MariaDB adapter for C++ AreaTrigger template startup rows.

use std::sync::Arc;

use wow_persistence::{
    AreaTriggerCreatePropertiesOrbitPersistenceRowLikeCpp,
    AreaTriggerCreatePropertiesPersistenceRowLikeCpp,
    AreaTriggerPolygonVertexPersistenceRowLikeCpp, AreaTriggerSplinePointPersistenceRowLikeCpp,
    AreaTriggerTemplateActionPersistenceRowLikeCpp, AreaTriggerTemplateCatalogLoadOutcomeLikeCpp,
    AreaTriggerTemplateCatalogPersistencePortLikeCpp, AreaTriggerTemplateCatalogRowsLikeCpp,
    AreaTriggerTemplatePersistenceRowLikeCpp, PersistenceFutureLikeCpp,
};

use crate::{DatabaseError, SqlResult, WorldDatabase, WorldStatements};

/// Existing represented Rust query/failure order. C++ loads templates before
/// create properties; correcting that difference is deliberately separate
/// from this dependency-boundary extraction (#505).
const STARTUP_STATEMENTS_LIKE_RUST: [WorldStatements; 6] = [
    WorldStatements::SEL_AREATRIGGER_TEMPLATE_ACTIONS,
    WorldStatements::SEL_AREATRIGGER_CREATE_PROPERTIES_POLYGON_VERTICES,
    WorldStatements::SEL_AREATRIGGER_CREATE_PROPERTIES_SPLINE_POINTS,
    WorldStatements::SEL_AREATRIGGER_CREATE_PROPERTIES,
    WorldStatements::SEL_AREATRIGGER_CREATE_PROPERTIES_ORBIT,
    WorldStatements::SEL_AREATRIGGER_TEMPLATES,
];

async fn query_rows_like_cpp<T>(
    db: &WorldDatabase,
    statement: WorldStatements,
    mut decode: impl FnMut(&SqlResult) -> T,
) -> Result<Vec<T>, DatabaseError> {
    let mut result = db.query(&db.prepare(statement)).await?;
    let mut rows = Vec::new();
    if result.is_empty() {
        return Ok(rows);
    }

    loop {
        rows.push(decode(&result));
        if !result.next_row() {
            break;
        }
    }
    Ok(rows)
}

async fn load_rows_like_cpp(
    db: &WorldDatabase,
) -> Result<AreaTriggerTemplateCatalogRowsLikeCpp, DatabaseError> {
    let action_rows = query_rows_like_cpp(db, STARTUP_STATEMENTS_LIKE_RUST[0], |row| {
        AreaTriggerTemplateActionPersistenceRowLikeCpp {
            area_trigger_id: row.read(0),
            is_custom: row.read(1),
            action_type: row.read(2),
            action_param: row.read(3),
            target_type: row.read(4),
        }
    })
    .await?;

    let polygon_vertex_rows = query_rows_like_cpp(db, STARTUP_STATEMENTS_LIKE_RUST[1], |row| {
        AreaTriggerPolygonVertexPersistenceRowLikeCpp {
            create_properties_id: row.read(0),
            is_custom: row.read(1),
            idx: row.read(2),
            vertice_x: row.read(3),
            vertice_y: row.read(4),
            vertice_target_x: (!row.is_null(5)).then(|| row.read(5)),
            vertice_target_y: (!row.is_null(6)).then(|| row.read(6)),
        }
    })
    .await?;

    let spline_point_rows = query_rows_like_cpp(db, STARTUP_STATEMENTS_LIKE_RUST[2], |row| {
        AreaTriggerSplinePointPersistenceRowLikeCpp {
            create_properties_id: row.read(0),
            is_custom: row.read(1),
            x: row.read(2),
            y: row.read(3),
            z: row.read(4),
        }
    })
    .await?;

    let create_properties_rows = query_rows_like_cpp(db, STARTUP_STATEMENTS_LIKE_RUST[3], |row| {
        AreaTriggerCreatePropertiesPersistenceRowLikeCpp {
            id: row.read(0),
            is_custom: row.read(1),
            area_trigger_id: row.read(2),
            is_areatrigger_custom: row.read(3),
            flags: row.read(4),
            move_curve_id: row.read(5),
            scale_curve_id: row.read(6),
            morph_curve_id: row.read(7),
            facing_curve_id: row.read(8),
            anim_id: row.read(9),
            anim_kit_id: row.read(10),
            decal_properties_id: row.read(11),
            time_to_target: row.read(12),
            time_to_target_scale: row.read(13),
            shape: row.read(14),
            shape_data: [
                row.read(15),
                row.read(16),
                row.read(17),
                row.read(18),
                row.read(19),
                row.read(20),
                row.read(21),
                row.read(22),
            ],
            script_name: row.read(23),
        }
    })
    .await?;

    let orbit_rows = query_rows_like_cpp(db, STARTUP_STATEMENTS_LIKE_RUST[4], |row| {
        AreaTriggerCreatePropertiesOrbitPersistenceRowLikeCpp {
            create_properties_id: row.read(0),
            is_custom: row.read(1),
            start_delay: row.read(2),
            circle_radius: row.read(3),
            blend_from_radius: row.read(4),
            initial_angle: row.read(5),
            z_offset: row.read(6),
            counter_clockwise: row.read(7),
            can_loop: row.read(8),
        }
    })
    .await?;

    let template_rows = query_rows_like_cpp(db, STARTUP_STATEMENTS_LIKE_RUST[5], |row| {
        AreaTriggerTemplatePersistenceRowLikeCpp {
            id: row.read(0),
            is_custom: row.read(1),
            flags: row.read(2),
        }
    })
    .await?;

    Ok(AreaTriggerTemplateCatalogRowsLikeCpp {
        action_rows,
        polygon_vertex_rows,
        spline_point_rows,
        create_properties_rows,
        orbit_rows,
        template_rows,
    })
}

pub struct MariaDbAreaTriggerTemplateCatalogPersistenceAdapterLikeCpp {
    world_db: Arc<WorldDatabase>,
}

impl MariaDbAreaTriggerTemplateCatalogPersistenceAdapterLikeCpp {
    pub fn new(world_db: Arc<WorldDatabase>) -> Self {
        Self { world_db }
    }
}

impl AreaTriggerTemplateCatalogPersistencePortLikeCpp
    for MariaDbAreaTriggerTemplateCatalogPersistenceAdapterLikeCpp
{
    fn load_template_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, AreaTriggerTemplateCatalogLoadOutcomeLikeCpp> {
        Box::pin(async move {
            match load_rows_like_cpp(&self.world_db).await {
                Ok(rows) => AreaTriggerTemplateCatalogLoadOutcomeLikeCpp::Loaded(rows),
                Err(error) => AreaTriggerTemplateCatalogLoadOutcomeLikeCpp::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_trigger_statement_order_preserves_the_represented_rust_contract() {
        assert_eq!(
            STARTUP_STATEMENTS_LIKE_RUST,
            [
                WorldStatements::SEL_AREATRIGGER_TEMPLATE_ACTIONS,
                WorldStatements::SEL_AREATRIGGER_CREATE_PROPERTIES_POLYGON_VERTICES,
                WorldStatements::SEL_AREATRIGGER_CREATE_PROPERTIES_SPLINE_POINTS,
                WorldStatements::SEL_AREATRIGGER_CREATE_PROPERTIES,
                WorldStatements::SEL_AREATRIGGER_CREATE_PROPERTIES_ORBIT,
                WorldStatements::SEL_AREATRIGGER_TEMPLATES,
            ]
        );
    }
}
