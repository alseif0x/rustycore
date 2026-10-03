//! Name startup/read queries at the SQL adapter boundary.
use super::{ForeverSessionRepository, LoadError, field};
use crate::PreparedStatement;
use wow_persistence::forever::names::DefaultNamePermissions;
pub(super) const COLLISION_QUERY: &str = "SELECT 1 FROM characters WHERE name = ?";
const RESERVED_QUERY: &str = "SELECT name FROM reserved_name";

impl ForeverSessionRepository {
    pub async fn load_reserved_names(&self) -> Result<Vec<String>, LoadError> {
        // ObjectMgr.cpp:8682-8719 uses CharacterDatabase, never World.
        let mut result = self
            .characters
            .direct_query(RESERVED_QUERY)
            .await
            .map_err(|_| LoadError::Database)?;
        let mut names = Vec::new();
        if !result.is_empty() {
            loop {
                names.push(field::<String>(&result, 0)?);
                if !result.next_row() {
                    break;
                }
            }
        }
        Ok(names)
    }

    pub async fn load_default_name_permissions(
        &self,
        realm: i32,
    ) -> Result<DefaultNamePermissions, LoadError> {
        // AccountMgr.cpp:461-510 and RBACData::ExpandPermissions. The isolated
        // account admits only security zero and no explicit grant/deny rows.
        let mut rows = DefaultNamePermissions::default();
        for (table, sql) in [
            "SELECT id FROM rbac_permissions",
            "SELECT id,linkedId FROM rbac_linked_permissions ORDER BY id ASC",
            "SELECT permissionId FROM rbac_default_permissions WHERE secId=0 AND (realmId=? OR realmId=-1) ORDER BY secId ASC",
        ].iter().enumerate() {
            let mut query = PreparedStatement::new(*sql);
            if table == 2 { query.set_i32(0, realm); }
            let mut result = self.auth.query(&query).await.map_err(|_| LoadError::Database)?;
            if !result.is_empty() {
                loop {
                    let id = field::<u32>(&result, 0)?;
                    match table {
                        0 => rows.known.push(id),
                        1 => rows.links.push((id, field(&result, 1)?)),
                        2 => rows.roots.push(id),
                        _ => unreachable!(),
                    }
                    if !result.next_row() { break; }
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
    fn availability_is_not_account_filtered_or_a_creation_reservation() {
        assert_eq!(COLLISION_QUERY, "SELECT 1 FROM characters WHERE name = ?");
        assert!(!COLLISION_QUERY.contains("account") && !COLLISION_QUERY.contains("delete"));
        assert_eq!(RESERVED_QUERY, "SELECT name FROM reserved_name");
    }
}
