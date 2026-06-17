// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadSceneTemplates` represented store.

use std::collections::HashMap;

use anyhow::Result;
use wow_database::{WorldDatabase, WorldStatements};

use crate::{ScriptIdLikeCpp, ScriptNameInternerLikeCpp};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneTemplateLikeCpp {
    pub scene_id: u32,
    pub playback_flags: u32,
    pub scene_package_id: u32,
    pub encrypted: bool,
    pub script_id: ScriptIdLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneTemplateRowLikeCpp {
    pub scene_id: u32,
    pub flags: u32,
    pub script_package_id: u32,
    pub encrypted: u8,
    pub script_name: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SceneTemplateLoadReportLikeCpp {
    pub rows_seen: usize,
    /// C++ currently declares `count` but never increments it in
    /// `ObjectMgr::LoadSceneTemplates`; this preserves that observable log value.
    pub cpp_logged_count_bug_like_cpp: usize,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SceneTemplateStoreLikeCpp {
    templates: HashMap<u32, SceneTemplateLikeCpp>,
}

pub struct SceneTemplateLoadOutcomeLikeCpp {
    pub store: SceneTemplateStoreLikeCpp,
    pub report: SceneTemplateLoadReportLikeCpp,
}

impl SceneTemplateStoreLikeCpp {
    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = SceneTemplateRowLikeCpp>,
        script_names: &mut ScriptNameInternerLikeCpp,
    ) -> SceneTemplateLoadOutcomeLikeCpp {
        let mut templates = HashMap::new();
        let mut report = SceneTemplateLoadReportLikeCpp::default();

        for row in rows {
            report.rows_seen += 1;
            let script_id = script_names.get_script_id_like_cpp(row.script_name, true);
            templates.insert(
                row.scene_id,
                SceneTemplateLikeCpp {
                    scene_id: row.scene_id,
                    playback_flags: row.flags,
                    scene_package_id: row.script_package_id,
                    encrypted: row.encrypted != 0,
                    script_id,
                },
            );
        }

        SceneTemplateLoadOutcomeLikeCpp {
            store: Self { templates },
            report,
        }
    }

    /// C++ `ObjectMgr::LoadSceneTemplates`.
    pub async fn load_like_cpp(
        db: &WorldDatabase,
        script_names: &mut ScriptNameInternerLikeCpp,
    ) -> Result<SceneTemplateLoadOutcomeLikeCpp> {
        let stmt = db.prepare(WorldStatements::SEL_SCENE_TEMPLATES);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(SceneTemplateRowLikeCpp {
                    scene_id: result.read(0),
                    flags: result.read(1),
                    script_package_id: result.read(2),
                    encrypted: result.read(3),
                    script_name: result.read(4),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(rows, script_names))
    }

    /// C++ `ObjectMgr::GetSceneTemplate`.
    pub fn get_scene_template_like_cpp(&self, scene_id: u32) -> Option<&SceneTemplateLikeCpp> {
        self.templates.get(&scene_id)
    }

    pub fn len(&self) -> usize {
        self.templates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(scene_id: u32, script_name: &str) -> SceneTemplateRowLikeCpp {
        SceneTemplateRowLikeCpp {
            scene_id,
            flags: 0x10,
            script_package_id: 77,
            encrypted: 1,
            script_name: script_name.to_string(),
        }
    }

    #[test]
    fn scene_templates_load_fields_and_intern_script_names_like_cpp() {
        let mut scripts = ScriptNameInternerLikeCpp::new();
        let outcome = SceneTemplateStoreLikeCpp::from_rows_like_cpp(
            [row(1, "scene_intro"), row(2, "")],
            &mut scripts,
        );

        let scene = outcome.store.get_scene_template_like_cpp(1).unwrap();
        assert_eq!(scene.scene_id, 1);
        assert_eq!(scene.playback_flags, 0x10);
        assert_eq!(scene.scene_package_id, 77);
        assert!(scene.encrypted);
        assert_eq!(
            scripts.get_script_name_like_cpp(scene.script_id),
            "scene_intro"
        );
        assert!(scripts.is_script_database_bound_like_cpp(scene.script_id));

        let empty_script_scene = outcome.store.get_scene_template_like_cpp(2).unwrap();
        assert_eq!(empty_script_scene.script_id, ScriptIdLikeCpp::NONE);
        assert_eq!(outcome.report.rows_seen, 2);
        assert_eq!(outcome.report.cpp_logged_count_bug_like_cpp, 0);
    }

    #[test]
    fn scene_templates_duplicate_scene_id_overwrites_like_cpp() {
        let mut scripts = ScriptNameInternerLikeCpp::new();
        let outcome = SceneTemplateStoreLikeCpp::from_rows_like_cpp(
            [
                SceneTemplateRowLikeCpp {
                    flags: 1,
                    ..row(7, "first")
                },
                SceneTemplateRowLikeCpp {
                    flags: 2,
                    ..row(7, "second")
                },
            ],
            &mut scripts,
        );

        let scene = outcome.store.get_scene_template_like_cpp(7).unwrap();
        assert_eq!(scene.playback_flags, 2);
        assert_eq!(scripts.get_script_name_like_cpp(scene.script_id), "second");
        assert_eq!(outcome.store.len(), 1);
        assert_eq!(outcome.report.rows_seen, 2);
    }
}
