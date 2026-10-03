//! Target default-RBAC bootstrap read, not an account grants/denials loader.
use super::{ForeverSessionRepository, LoadError, field};
use crate::PreparedStatement;
use wow_persistence::forever::permissions::DefaultPermissionRows;

const QUERIES: [&str; 3] = [
    "SELECT id FROM rbac_permissions",
    "SELECT id,linkedId FROM rbac_linked_permissions ORDER BY id ASC",
    "SELECT permissionId FROM rbac_default_permissions WHERE secId=0 AND (realmId=? OR realmId=-1) ORDER BY secId ASC",
];

impl ForeverSessionRepository {
    pub async fn load_default_permissions(
        &self,
        realm: i32,
    ) -> Result<DefaultPermissionRows, LoadError> {
        // AccountMgr.cpp:461-510 / RBACData::ExpandPermissions, 02245dcd.
        // Ordinary account only; customized RBAC remains an admission error.
        let mut rows = DefaultPermissionRows::default();
        for (table, sql) in QUERIES.iter().enumerate() {
            let mut query = PreparedStatement::new(*sql);
            if table == 2 {
                query.set_i32(0, realm);
            }
            let mut result = self
                .auth
                .query(&query)
                .await
                .map_err(|_| LoadError::Database)?;
            if !result.is_empty() {
                loop {
                    let id = field::<u32>(&result, 0)?;
                    match table {
                        0 => rows.known.push(id),
                        1 => rows.links.push((id, field(&result, 1)?)),
                        2 => rows.roots.push(id),
                        _ => unreachable!(),
                    }
                    if !result.next_row() {
                        break;
                    }
                }
            }
        }
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_rbac_queries_preserve_the_old_order_and_realm_security_scope() {
        assert_eq!(
            QUERIES,
            [
                "SELECT id FROM rbac_permissions",
                "SELECT id,linkedId FROM rbac_linked_permissions ORDER BY id ASC",
                "SELECT permissionId FROM rbac_default_permissions WHERE secId=0 AND (realmId=? OR realmId=-1) ORDER BY secId ASC",
            ]
        );
    }
}
