// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `AreaTriggerDataStore::LoadAreaTriggerTemplates` template/action subset.

use std::collections::HashMap;

use anyhow::Result;
use wow_database::{WorldDatabase, WorldStatements};

use crate::WorldSafeLocStore;

pub const AREATRIGGER_ACTION_CAST_LIKE_CPP: u32 = 0;
pub const AREATRIGGER_ACTION_ADDAURA_LIKE_CPP: u32 = 1;
pub const AREATRIGGER_ACTION_TELEPORT_LIKE_CPP: u32 = 2;
pub const AREATRIGGER_ACTION_MAX_LIKE_CPP: u32 = 3;

pub const AREATRIGGER_ACTION_USER_ANY_LIKE_CPP: u32 = 0;
pub const AREATRIGGER_ACTION_USER_FRIEND_LIKE_CPP: u32 = 1;
pub const AREATRIGGER_ACTION_USER_ENEMY_LIKE_CPP: u32 = 2;
pub const AREATRIGGER_ACTION_USER_RAID_LIKE_CPP: u32 = 3;
pub const AREATRIGGER_ACTION_USER_PARTY_LIKE_CPP: u32 = 4;
pub const AREATRIGGER_ACTION_USER_CASTER_LIKE_CPP: u32 = 5;
pub const AREATRIGGER_ACTION_USER_MAX_LIKE_CPP: u32 = 6;

pub const AREATRIGGER_FLAG_NONE_LIKE_CPP: u32 = 0;
pub const AREATRIGGER_FLAG_IS_SERVER_SIDE_LIKE_CPP: u32 = 0x01;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AreaTriggerIdLikeCpp {
    pub id: u32,
    pub is_custom: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaTriggerActionLikeCpp {
    pub param: u32,
    pub action_type: u32,
    pub target_type: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaTriggerPosition2LikeCpp {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaTriggerPosition3LikeCpp {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaTriggerTemplateRowLikeCpp {
    pub id: u32,
    pub is_custom: bool,
    pub flags: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaTriggerTemplateActionRowLikeCpp {
    pub area_trigger_id: u32,
    pub is_custom: bool,
    pub action_type: u32,
    pub action_param: u32,
    pub target_type: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaTriggerPolygonVertexRowLikeCpp {
    pub create_properties_id: u32,
    pub is_custom: bool,
    pub idx: u32,
    pub vertice_x: f32,
    pub vertice_y: f32,
    pub vertice_target_x: Option<f32>,
    pub vertice_target_y: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaTriggerSplinePointRowLikeCpp {
    pub create_properties_id: u32,
    pub is_custom: bool,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaTriggerTemplateLikeCpp {
    pub id: AreaTriggerIdLikeCpp,
    pub flags: u32,
    pub actions: Vec<AreaTriggerActionLikeCpp>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AreaTriggerTemplateLoadReportLikeCpp {
    pub template_rows_seen: usize,
    pub action_rows_seen: usize,
    pub loaded_templates: usize,
    pub loaded_actions: usize,
    pub polygon_vertex_rows_seen: usize,
    pub spline_point_rows_seen: usize,
    pub loaded_polygon_vertices: usize,
    pub loaded_polygon_target_vertices: usize,
    pub loaded_spline_points: usize,
    pub skipped_actions_invalid_action_type: Vec<(AreaTriggerIdLikeCpp, u32, u32)>,
    pub skipped_actions_invalid_target_type: Vec<(AreaTriggerIdLikeCpp, u32, u32)>,
    pub skipped_actions_invalid_teleport_world_safe_loc: Vec<(AreaTriggerIdLikeCpp, u32)>,
    pub invalid_partial_target_vertices: Vec<(AreaTriggerIdLikeCpp, u32)>,
}

#[derive(Debug, Clone, Default)]
pub struct AreaTriggerTemplateStore {
    templates: HashMap<AreaTriggerIdLikeCpp, AreaTriggerTemplateLikeCpp>,
    polygon_vertices_by_create_properties:
        HashMap<AreaTriggerIdLikeCpp, Vec<AreaTriggerPosition2LikeCpp>>,
    polygon_target_vertices_by_create_properties:
        HashMap<AreaTriggerIdLikeCpp, Vec<AreaTriggerPosition2LikeCpp>>,
    spline_points_by_create_properties:
        HashMap<AreaTriggerIdLikeCpp, Vec<AreaTriggerPosition3LikeCpp>>,
}

pub struct AreaTriggerTemplateLoadOutcomeLikeCpp {
    pub store: AreaTriggerTemplateStore,
    pub report: AreaTriggerTemplateLoadReportLikeCpp,
}

impl AreaTriggerTemplateStore {
    pub fn from_keys(keys: impl IntoIterator<Item = (u32, bool)>) -> Self {
        Self {
            templates: keys
                .into_iter()
                .map(|(id, is_custom)| {
                    let id = AreaTriggerIdLikeCpp { id, is_custom };
                    (
                        id,
                        AreaTriggerTemplateLikeCpp {
                            id,
                            flags: AREATRIGGER_FLAG_NONE_LIKE_CPP,
                            actions: Vec::new(),
                        },
                    )
                })
                .collect(),
            polygon_vertices_by_create_properties: HashMap::new(),
            polygon_target_vertices_by_create_properties: HashMap::new(),
            spline_points_by_create_properties: HashMap::new(),
        }
    }

    pub fn from_rows_like_cpp(
        template_rows: impl IntoIterator<Item = AreaTriggerTemplateRowLikeCpp>,
        action_rows: impl IntoIterator<Item = AreaTriggerTemplateActionRowLikeCpp>,
        polygon_vertex_rows: impl IntoIterator<Item = AreaTriggerPolygonVertexRowLikeCpp>,
        spline_point_rows: impl IntoIterator<Item = AreaTriggerSplinePointRowLikeCpp>,
        world_safe_locs: &WorldSafeLocStore,
    ) -> AreaTriggerTemplateLoadOutcomeLikeCpp {
        let mut report = AreaTriggerTemplateLoadReportLikeCpp::default();
        let mut actions_by_area_trigger: HashMap<
            AreaTriggerIdLikeCpp,
            Vec<AreaTriggerActionLikeCpp>,
        > = HashMap::new();

        for row in action_rows {
            report.action_rows_seen += 1;
            let area_trigger_id = AreaTriggerIdLikeCpp {
                id: row.area_trigger_id,
                is_custom: row.is_custom,
            };

            if row.action_type >= AREATRIGGER_ACTION_MAX_LIKE_CPP {
                report.skipped_actions_invalid_action_type.push((
                    area_trigger_id,
                    row.action_type,
                    row.action_param,
                ));
                continue;
            }

            if row.target_type >= AREATRIGGER_ACTION_USER_MAX_LIKE_CPP {
                report.skipped_actions_invalid_target_type.push((
                    area_trigger_id,
                    row.target_type,
                    row.action_param,
                ));
                continue;
            }

            if row.action_type == AREATRIGGER_ACTION_TELEPORT_LIKE_CPP
                && !world_safe_locs.contains(row.action_param)
            {
                report
                    .skipped_actions_invalid_teleport_world_safe_loc
                    .push((area_trigger_id, row.action_param));
                continue;
            }

            actions_by_area_trigger
                .entry(area_trigger_id)
                .or_default()
                .push(AreaTriggerActionLikeCpp {
                    param: row.action_param,
                    action_type: row.action_type,
                    target_type: row.target_type,
                });
            report.loaded_actions += 1;
        }

        let mut polygon_vertices_by_create_properties: HashMap<
            AreaTriggerIdLikeCpp,
            Vec<AreaTriggerPosition2LikeCpp>,
        > = HashMap::new();
        let mut polygon_target_vertices_by_create_properties: HashMap<
            AreaTriggerIdLikeCpp,
            Vec<AreaTriggerPosition2LikeCpp>,
        > = HashMap::new();
        for row in polygon_vertex_rows {
            report.polygon_vertex_rows_seen += 1;
            let create_properties_id = AreaTriggerIdLikeCpp {
                id: row.create_properties_id,
                is_custom: row.is_custom,
            };

            polygon_vertices_by_create_properties
                .entry(create_properties_id)
                .or_default()
                .push(AreaTriggerPosition2LikeCpp {
                    x: row.vertice_x,
                    y: row.vertice_y,
                });
            report.loaded_polygon_vertices += 1;

            match (row.vertice_target_x, row.vertice_target_y) {
                (Some(x), Some(y)) => {
                    polygon_target_vertices_by_create_properties
                        .entry(create_properties_id)
                        .or_default()
                        .push(AreaTriggerPosition2LikeCpp { x, y });
                    report.loaded_polygon_target_vertices += 1;
                }
                (None, None) => {}
                _ => report
                    .invalid_partial_target_vertices
                    .push((create_properties_id, row.idx)),
            }
        }

        let mut spline_points_by_create_properties: HashMap<
            AreaTriggerIdLikeCpp,
            Vec<AreaTriggerPosition3LikeCpp>,
        > = HashMap::new();
        for row in spline_point_rows {
            report.spline_point_rows_seen += 1;
            let create_properties_id = AreaTriggerIdLikeCpp {
                id: row.create_properties_id,
                is_custom: row.is_custom,
            };
            spline_points_by_create_properties
                .entry(create_properties_id)
                .or_default()
                .push(AreaTriggerPosition3LikeCpp {
                    x: row.x,
                    y: row.y,
                    z: row.z,
                });
            report.loaded_spline_points += 1;
        }

        let mut templates = HashMap::new();
        for row in template_rows {
            report.template_rows_seen += 1;
            let id = AreaTriggerIdLikeCpp {
                id: row.id,
                is_custom: row.is_custom,
            };
            templates.insert(
                id,
                AreaTriggerTemplateLikeCpp {
                    id,
                    flags: row.flags,
                    actions: actions_by_area_trigger.remove(&id).unwrap_or_default(),
                },
            );
        }

        report.loaded_templates = templates.len();
        AreaTriggerTemplateLoadOutcomeLikeCpp {
            store: Self {
                templates,
                polygon_vertices_by_create_properties,
                polygon_target_vertices_by_create_properties,
                spline_points_by_create_properties,
            },
            report,
        }
    }

    pub async fn load_like_cpp(
        db: &WorldDatabase,
        world_safe_locs: &WorldSafeLocStore,
    ) -> Result<AreaTriggerTemplateLoadOutcomeLikeCpp> {
        let mut action_rows = Vec::new();
        let mut action_result = db
            .query(&db.prepare(WorldStatements::SEL_AREATRIGGER_TEMPLATE_ACTIONS))
            .await?;
        if !action_result.is_empty() {
            loop {
                action_rows.push(AreaTriggerTemplateActionRowLikeCpp {
                    area_trigger_id: action_result.read(0),
                    is_custom: action_result.read(1),
                    action_type: action_result.read(2),
                    action_param: action_result.read(3),
                    target_type: action_result.read(4),
                });

                if !action_result.next_row() {
                    break;
                }
            }
        }

        let mut polygon_vertex_rows = Vec::new();
        let mut polygon_vertex_result = db
            .query(&db.prepare(WorldStatements::SEL_AREATRIGGER_CREATE_PROPERTIES_POLYGON_VERTICES))
            .await?;
        if !polygon_vertex_result.is_empty() {
            loop {
                polygon_vertex_rows.push(AreaTriggerPolygonVertexRowLikeCpp {
                    create_properties_id: polygon_vertex_result.read(0),
                    is_custom: polygon_vertex_result.read(1),
                    idx: polygon_vertex_result.read(2),
                    vertice_x: polygon_vertex_result.read(3),
                    vertice_y: polygon_vertex_result.read(4),
                    vertice_target_x: if polygon_vertex_result.is_null(5) {
                        None
                    } else {
                        Some(polygon_vertex_result.read(5))
                    },
                    vertice_target_y: if polygon_vertex_result.is_null(6) {
                        None
                    } else {
                        Some(polygon_vertex_result.read(6))
                    },
                });

                if !polygon_vertex_result.next_row() {
                    break;
                }
            }
        }

        let mut spline_point_rows = Vec::new();
        let mut spline_point_result = db
            .query(&db.prepare(WorldStatements::SEL_AREATRIGGER_CREATE_PROPERTIES_SPLINE_POINTS))
            .await?;
        if !spline_point_result.is_empty() {
            loop {
                spline_point_rows.push(AreaTriggerSplinePointRowLikeCpp {
                    create_properties_id: spline_point_result.read(0),
                    is_custom: spline_point_result.read(1),
                    x: spline_point_result.read(2),
                    y: spline_point_result.read(3),
                    z: spline_point_result.read(4),
                });

                if !spline_point_result.next_row() {
                    break;
                }
            }
        }

        let mut template_rows = Vec::new();
        let mut template_result = db
            .query(&db.prepare(WorldStatements::SEL_AREATRIGGER_TEMPLATES))
            .await?;
        if !template_result.is_empty() {
            loop {
                template_rows.push(AreaTriggerTemplateRowLikeCpp {
                    id: template_result.read(0),
                    is_custom: template_result.read(1),
                    flags: template_result.read(2),
                });

                if !template_result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(
            template_rows,
            action_rows,
            polygon_vertex_rows,
            spline_point_rows,
            world_safe_locs,
        ))
    }

    pub fn contains(&self, id: u32, is_custom: bool) -> bool {
        self.templates
            .contains_key(&AreaTriggerIdLikeCpp { id, is_custom })
    }

    pub fn get_template_like_cpp(
        &self,
        id: AreaTriggerIdLikeCpp,
    ) -> Option<&AreaTriggerTemplateLikeCpp> {
        self.templates.get(&id)
    }

    pub fn len(&self) -> usize {
        self.templates.len()
    }

    pub fn action_len(&self) -> usize {
        self.templates
            .values()
            .map(|template| template.actions.len())
            .sum()
    }

    pub fn polygon_vertices_like_cpp(
        &self,
        id: AreaTriggerIdLikeCpp,
    ) -> Option<&[AreaTriggerPosition2LikeCpp]> {
        self.polygon_vertices_by_create_properties
            .get(&id)
            .map(Vec::as_slice)
    }

    pub fn polygon_target_vertices_like_cpp(
        &self,
        id: AreaTriggerIdLikeCpp,
    ) -> Option<&[AreaTriggerPosition2LikeCpp]> {
        self.polygon_target_vertices_by_create_properties
            .get(&id)
            .map(Vec::as_slice)
    }

    pub fn spline_points_like_cpp(
        &self,
        id: AreaTriggerIdLikeCpp,
    ) -> Option<&[AreaTriggerPosition3LikeCpp]> {
        self.spline_points_by_create_properties
            .get(&id)
            .map(Vec::as_slice)
    }

    pub fn polygon_vertex_len(&self) -> usize {
        self.polygon_vertices_by_create_properties
            .values()
            .map(Vec::len)
            .sum()
    }

    pub fn spline_point_len(&self) -> usize {
        self.spline_points_by_create_properties
            .values()
            .map(Vec::len)
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{WorldSafeLoc, WorldSafeLocStore};
    use wow_core::Position;

    fn safe_locs(ids: impl IntoIterator<Item = u32>) -> WorldSafeLocStore {
        WorldSafeLocStore::from_locs_for_test(ids.into_iter().map(|id| WorldSafeLoc {
            id,
            map_id: 0,
            position: Position::new(0.0, 0.0, 0.0, 0.0),
        }))
    }

    fn template(id: u32, is_custom: bool, flags: u32) -> AreaTriggerTemplateRowLikeCpp {
        AreaTriggerTemplateRowLikeCpp {
            id,
            is_custom,
            flags,
        }
    }

    fn action(
        id: u32,
        is_custom: bool,
        action_type: u32,
        action_param: u32,
        target_type: u32,
    ) -> AreaTriggerTemplateActionRowLikeCpp {
        AreaTriggerTemplateActionRowLikeCpp {
            area_trigger_id: id,
            is_custom,
            action_type,
            action_param,
            target_type,
        }
    }

    fn polygon_vertex(
        create_properties_id: u32,
        is_custom: bool,
        idx: u32,
        x: f32,
        y: f32,
        target: Option<(f32, f32)>,
    ) -> AreaTriggerPolygonVertexRowLikeCpp {
        AreaTriggerPolygonVertexRowLikeCpp {
            create_properties_id,
            is_custom,
            idx,
            vertice_x: x,
            vertice_y: y,
            vertice_target_x: target.map(|(target_x, _)| target_x),
            vertice_target_y: target.map(|(_, target_y)| target_y),
        }
    }

    fn partial_polygon_vertex(
        create_properties_id: u32,
        is_custom: bool,
        idx: u32,
    ) -> AreaTriggerPolygonVertexRowLikeCpp {
        AreaTriggerPolygonVertexRowLikeCpp {
            create_properties_id,
            is_custom,
            idx,
            vertice_x: 1.0,
            vertice_y: 2.0,
            vertice_target_x: Some(3.0),
            vertice_target_y: None,
        }
    }

    fn spline_point(
        create_properties_id: u32,
        is_custom: bool,
        x: f32,
        y: f32,
        z: f32,
    ) -> AreaTriggerSplinePointRowLikeCpp {
        AreaTriggerSplinePointRowLikeCpp {
            create_properties_id,
            is_custom,
            x,
            y,
            z,
        }
    }

    #[test]
    fn area_trigger_template_store_keys_by_id_and_custom_flag_like_cpp() {
        let store = AreaTriggerTemplateStore::from_keys([(7, false), (7, true)]);

        assert!(store.contains(7, false));
        assert!(store.contains(7, true));
        assert!(!store.contains(8, false));
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn load_templates_moves_valid_actions_into_matching_template_like_cpp() {
        let outcome = AreaTriggerTemplateStore::from_rows_like_cpp(
            [template(
                10,
                false,
                AREATRIGGER_FLAG_IS_SERVER_SIDE_LIKE_CPP,
            )],
            [
                action(
                    10,
                    false,
                    AREATRIGGER_ACTION_CAST_LIKE_CPP,
                    123,
                    AREATRIGGER_ACTION_USER_ANY_LIKE_CPP,
                ),
                action(
                    10,
                    false,
                    AREATRIGGER_ACTION_TELEPORT_LIKE_CPP,
                    7,
                    AREATRIGGER_ACTION_USER_CASTER_LIKE_CPP,
                ),
            ],
            [],
            [],
            &safe_locs([7]),
        );

        let loaded = outcome
            .store
            .get_template_like_cpp(AreaTriggerIdLikeCpp {
                id: 10,
                is_custom: false,
            })
            .unwrap();

        assert_eq!(outcome.report.template_rows_seen, 1);
        assert_eq!(outcome.report.action_rows_seen, 2);
        assert_eq!(outcome.report.loaded_templates, 1);
        assert_eq!(outcome.report.loaded_actions, 2);
        assert_eq!(loaded.flags, AREATRIGGER_FLAG_IS_SERVER_SIDE_LIKE_CPP);
        assert_eq!(loaded.actions.len(), 2);
        assert_eq!(loaded.actions[0].param, 123);
        assert_eq!(loaded.actions[1].param, 7);
    }

    #[test]
    fn load_templates_skips_invalid_actions_like_cpp() {
        let outcome = AreaTriggerTemplateStore::from_rows_like_cpp(
            [template(10, false, 0)],
            [
                action(10, false, AREATRIGGER_ACTION_MAX_LIKE_CPP, 1, 0),
                action(10, false, 0, 2, AREATRIGGER_ACTION_USER_MAX_LIKE_CPP),
                action(10, false, AREATRIGGER_ACTION_TELEPORT_LIKE_CPP, 999, 0),
            ],
            [],
            [],
            &safe_locs([7]),
        );

        let loaded = outcome
            .store
            .get_template_like_cpp(AreaTriggerIdLikeCpp {
                id: 10,
                is_custom: false,
            })
            .unwrap();

        assert!(loaded.actions.is_empty());
        assert_eq!(
            outcome.report.skipped_actions_invalid_action_type,
            [(
                AreaTriggerIdLikeCpp {
                    id: 10,
                    is_custom: false
                },
                AREATRIGGER_ACTION_MAX_LIKE_CPP,
                1
            )]
        );
        assert_eq!(
            outcome.report.skipped_actions_invalid_target_type,
            [(
                AreaTriggerIdLikeCpp {
                    id: 10,
                    is_custom: false
                },
                AREATRIGGER_ACTION_USER_MAX_LIKE_CPP,
                2
            )]
        );
        assert_eq!(
            outcome
                .report
                .skipped_actions_invalid_teleport_world_safe_loc,
            [(
                AreaTriggerIdLikeCpp {
                    id: 10,
                    is_custom: false
                },
                999
            )]
        );
    }

    #[test]
    fn actions_without_template_are_kept_only_in_staging_like_cpp() {
        let outcome = AreaTriggerTemplateStore::from_rows_like_cpp(
            [template(10, false, 0)],
            [action(99, false, AREATRIGGER_ACTION_CAST_LIKE_CPP, 1, 0)],
            [],
            [],
            &safe_locs([]),
        );

        assert_eq!(outcome.report.loaded_actions, 1);
        assert_eq!(outcome.store.action_len(), 0);
    }

    #[test]
    fn load_templates_stages_polygon_vertices_and_spline_points_like_cpp() {
        let outcome = AreaTriggerTemplateStore::from_rows_like_cpp(
            [],
            [],
            [
                polygon_vertex(90, false, 0, 1.0, 2.0, Some((10.0, 20.0))),
                polygon_vertex(90, false, 1, 3.0, 4.0, None),
            ],
            [
                spline_point(90, false, 5.0, 6.0, 7.0),
                spline_point(90, false, 8.0, 9.0, 10.0),
            ],
            &safe_locs([]),
        );
        let id = AreaTriggerIdLikeCpp {
            id: 90,
            is_custom: false,
        };

        assert_eq!(outcome.report.polygon_vertex_rows_seen, 2);
        assert_eq!(outcome.report.spline_point_rows_seen, 2);
        assert_eq!(outcome.report.loaded_polygon_vertices, 2);
        assert_eq!(outcome.report.loaded_polygon_target_vertices, 1);
        assert_eq!(outcome.report.loaded_spline_points, 2);
        assert_eq!(
            outcome.store.polygon_vertices_like_cpp(id).unwrap(),
            [
                AreaTriggerPosition2LikeCpp { x: 1.0, y: 2.0 },
                AreaTriggerPosition2LikeCpp { x: 3.0, y: 4.0 },
            ]
        );
        assert_eq!(
            outcome.store.polygon_target_vertices_like_cpp(id).unwrap(),
            [AreaTriggerPosition2LikeCpp { x: 10.0, y: 20.0 }]
        );
        assert_eq!(
            outcome.store.spline_points_like_cpp(id).unwrap(),
            [
                AreaTriggerPosition3LikeCpp {
                    x: 5.0,
                    y: 6.0,
                    z: 7.0,
                },
                AreaTriggerPosition3LikeCpp {
                    x: 8.0,
                    y: 9.0,
                    z: 10.0,
                },
            ]
        );
    }

    #[test]
    fn load_templates_keeps_base_vertex_but_skips_partial_target_like_cpp() {
        let outcome = AreaTriggerTemplateStore::from_rows_like_cpp(
            [],
            [],
            [partial_polygon_vertex(77, true, 4)],
            [],
            &safe_locs([]),
        );
        let id = AreaTriggerIdLikeCpp {
            id: 77,
            is_custom: true,
        };

        assert_eq!(outcome.report.loaded_polygon_vertices, 1);
        assert_eq!(outcome.report.loaded_polygon_target_vertices, 0);
        assert_eq!(outcome.report.invalid_partial_target_vertices, [(id, 4)]);
        assert_eq!(
            outcome.store.polygon_vertices_like_cpp(id).unwrap(),
            [AreaTriggerPosition2LikeCpp { x: 1.0, y: 2.0 }]
        );
        assert!(outcome.store.polygon_target_vertices_like_cpp(id).is_none());
    }
}
