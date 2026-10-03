//! Target column projections, not a copy of the legacy statement binding.
//! 02245dcd HotfixDatabase.cpp:1281-1295 / CfgCategoriesLoadInfo.
//! Intentional Forever contract: NamesReservedLocale uses its own declared
//! LocaleMask table. DB2LoadInfo's global-reserved binding is not retained.
use super::{BTreeSet, ForeverHotfixRepository, LoadError, PreparedStatement};
use wow_persistence::forever::names::{NameOverlays, NameRows};

const QUERIES: [&str; 4] = [
    "SELECT ID,Name,Language FROM names_profanity WHERE (VerifiedBuild>0)=?",
    "SELECT ID,Name FROM names_reserved WHERE (VerifiedBuild>0)=?",
    "SELECT ID,Name,LocaleMask FROM names_reserved_locale WHERE (VerifiedBuild>0)=?",
    "SELECT ID,CreateCharsetMask FROM cfg_categories WHERE (VerifiedBuild>0)=?",
];

impl ForeverHotfixRepository {
    pub async fn load_name_overlays(&self) -> Result<NameOverlays, LoadError> {
        Ok(NameOverlays {
            official: self.name_batch(false).await?,
            custom: self.name_batch(true).await?,
        })
    }

    async fn name_batch(&self, custom: bool) -> Result<NameRows, LoadError> {
        let mut rows = NameRows::default();
        for (table, sql) in QUERIES.iter().enumerate() {
            let mut query = PreparedStatement::new(*sql);
            query.set_bool(0, !custom);
            let mut result = self
                .0
                .query(&query)
                .await
                .map_err(|_| LoadError::Database)?;
            if result.is_empty() {
                continue;
            }
            let mut seen = BTreeSet::new();
            loop {
                let id = result.try_read::<u32>(0).ok_or(LoadError::InvalidRow)?;
                if !seen.insert(id) {
                    return Err(LoadError::InvalidRow);
                }
                match table {
                    0 => rows.profanity.push((
                        id,
                        result.try_read::<String>(1).ok_or(LoadError::InvalidRow)?,
                        result.try_read::<i8>(2).ok_or(LoadError::InvalidRow)?,
                    )),
                    1 => rows.reserved.push((
                        id,
                        result.try_read::<String>(1).ok_or(LoadError::InvalidRow)?,
                    )),
                    2 => rows.locale_reserved.push((
                        id,
                        result.try_read::<String>(1).ok_or(LoadError::InvalidRow)?,
                        result.try_read::<u8>(2).ok_or(LoadError::InvalidRow)?,
                    )),
                    3 => rows
                        .categories
                        .push((id, result.try_read::<u8>(1).ok_or(LoadError::InvalidRow)?)),
                    _ => unreachable!(),
                }
                if !result.next_row() {
                    break;
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
    fn target_name_columns_and_locale_binding_are_explicit() {
        for query in QUERIES {
            assert!(query.starts_with("SELECT ID,"));
            assert!(query.ends_with("WHERE (VerifiedBuild>0)=?"));
            assert!(!query.contains('*'));
        }
        assert!(QUERIES[0].contains("Name,Language FROM names_profanity"));
        assert!(QUERIES[2].contains("Name,LocaleMask FROM names_reserved_locale"));
        assert!(!QUERIES[2].contains(" FROM names_reserved "));
        assert!(QUERIES[3].contains("CreateCharsetMask FROM cfg_categories"));
    }
}
