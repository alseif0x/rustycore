//! Source-backed immutable default-RBAC projection, not an account permission
//! manager. Runtime still rejects explicit grants/denials and nonzero security.
use std::collections::{BTreeMap, BTreeSet};
use wow_persistence::forever::permissions::DefaultPermissionRows;

pub struct DefaultAccountPermissions {
    expanded: BTreeSet<u32>,
}

impl DefaultAccountPermissions {
    pub fn load(rows: DefaultPermissionRows) -> Self {
        Self {
            expanded: expand(rows),
        }
    }
    pub fn skip_sql_reserved_names(&self) -> bool {
        self.expanded.contains(&17)
    }
    pub fn use_character_templates(&self) -> bool {
        self.expanded.contains(&10)
    }
    pub fn use_start_gm_level(&self) -> bool {
        self.expanded.contains(&41)
    }
}

fn expand(rows: DefaultPermissionRows) -> BTreeSet<u32> {
    // AccountMgr::LoadRBAC returns before loading defaults when permission
    // definitions or links are empty. RBACData.cpp:260-283 expands valid IDs.
    if rows.known.is_empty() || rows.links.is_empty() {
        return BTreeSet::new();
    }
    let known: BTreeSet<_> = rows.known.into_iter().collect();
    let mut links = BTreeMap::<u32, Vec<u32>>::new();
    for (id, linked) in rows.links {
        if id != linked {
            links.entry(id).or_default().push(linked);
        }
    }
    let mut pending: BTreeSet<_> = rows.roots.into_iter().collect();
    let mut expanded = BTreeSet::new();
    while let Some(id) = pending.pop_first() {
        if !known.contains(&id) || !expanded.insert(id) {
            continue;
        }
        if let Some(children) = links.get(&id) {
            pending.extend(
                children
                    .iter()
                    .filter(|child| !expanded.contains(child))
                    .copied(),
            );
        }
    }
    expanded
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_graph_keeps_cycles_unknown_ids_and_source_empty_store_boundary() {
        let make = || DefaultPermissionRows {
            known: vec![17, 194, 195],
            links: vec![(195, 194), (194, 195), (194, 17), (195, 999)],
            roots: vec![195, 888],
        };
        assert!(DefaultAccountPermissions::load(make()).skip_sql_reserved_names());
        let mut rows = make();
        rows.roots = vec![999];
        assert!(!DefaultAccountPermissions::load(rows).skip_sql_reserved_names());
        let mut rows = make();
        rows.known.retain(|id| *id != 17);
        assert!(!DefaultAccountPermissions::load(rows).skip_sql_reserved_names());
        let mut rows = make();
        rows.links.clear();
        rows.roots = vec![17];
        assert!(!DefaultAccountPermissions::load(rows).skip_sql_reserved_names());
    }

    #[test]
    fn template_and_gm_permissions_are_expanded_independently_of_name_bypass() {
        let make = || DefaultPermissionRows {
            known: vec![10, 17, 41, 195],
            links: vec![(195, 10), (10, 41), (41, 195)],
            roots: vec![195],
        };
        let permissions = DefaultAccountPermissions::load(make());
        assert!(permissions.use_character_templates() && permissions.use_start_gm_level());
        assert!(!permissions.skip_sql_reserved_names());
        let mut rows = make();
        rows.known.retain(|id| *id != 10);
        let permissions = DefaultAccountPermissions::load(rows);
        assert!(!permissions.use_character_templates() && !permissions.use_start_gm_level());
        let permissions = DefaultAccountPermissions::load(DefaultPermissionRows {
            known: vec![10, 41],
            links: vec![],
            roots: vec![10, 41],
        });
        assert!(!permissions.use_character_templates() && !permissions.use_start_gm_level());
    }
}
