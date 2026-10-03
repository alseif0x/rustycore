//! Source-backed narrow default-RBAC projection, not an account permission
//! manager. Runtime still rejects explicit grants and nonzero security.
use std::collections::{BTreeMap, BTreeSet};
use wow_persistence::forever::names::DefaultNamePermissions;

pub fn default_sql_reserved_bypass(rows: DefaultNamePermissions) -> bool {
    // AccountMgr::LoadRBAC returns before loading defaults when permission
    // definitions or links are empty. RBACData.cpp:260-283 expands valid IDs.
    if rows.known.is_empty() || rows.links.is_empty() {
        return false;
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
    expanded.contains(&17)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_graph_keeps_cycles_unknown_ids_and_source_empty_store_boundary() {
        let make = || DefaultNamePermissions {
            known: vec![17, 194, 195],
            links: vec![(195, 194), (194, 195), (194, 17), (195, 999)],
            roots: vec![195, 888],
        };
        assert!(default_sql_reserved_bypass(make()));
        let mut rows = make();
        rows.roots = vec![999];
        assert!(!default_sql_reserved_bypass(rows));
        let mut rows = make();
        rows.known.retain(|id| *id != 17);
        assert!(!default_sql_reserved_bypass(rows));
        let mut rows = make();
        rows.links.clear();
        rows.roots = vec![17];
        assert!(!default_sql_reserved_bypass(rows));
    }
}
