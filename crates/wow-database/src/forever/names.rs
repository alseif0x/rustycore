//! Name startup/read queries at the SQL adapter boundary.
use super::{ForeverSessionRepository, LoadError, field};
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
