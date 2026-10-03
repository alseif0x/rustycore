//! Source 02245dcd HotfixDatabase.cpp:1082 and DB2DatabaseLoader::LoadStrings.
use super::{ForeverHotfixRepository, LoadError, NumericRow, PreparedStatement};
use wow_persistence::forever::items::{SparseItemLocaleOverlays, SparseItemLocaleRow};

const QUERY: &str = "SELECT ID,Description_lang,Display3_lang,Display2_lang,Display1_lang,Display_lang FROM item_sparse_locale WHERE (VerifiedBuild>0)=? AND locale=?";

impl ForeverHotfixRepository {
    /// This isolated composition loads esES only, just like its acquired
    /// baseline. Main-table enUS text is loaded separately, never as fallback.
    pub async fn load_sparse_es_es_overlays(&self) -> Result<SparseItemLocaleOverlays, LoadError> {
        Ok(SparseItemLocaleOverlays {
            official: self.sparse_locale_batch(false).await?,
            custom: self.sparse_locale_batch(true).await?,
        })
    }
    async fn sparse_locale_batch(
        &self,
        custom: bool,
    ) -> Result<Vec<SparseItemLocaleRow>, LoadError> {
        let mut query = PreparedStatement::new(QUERY);
        query.set_bool(0, !custom);
        query.set_string(1, "esES");
        let mut result = self
            .0
            .query(&query)
            .await
            .map_err(|_| LoadError::Database)?;
        let mut rows = Vec::new();
        if result.is_empty() {
            return Ok(rows);
        }
        loop {
            let mut r = NumericRow::new(&result, 6)?;
            rows.push(SparseItemLocaleRow {
                id: r.read()?,
                strings: [r.text()?, r.text()?, r.text()?, r.text()?, r.text()?],
            });
            r.finish()?;
            if !result.next_row() {
                break;
            }
        }
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn locale_projection_is_exact_and_unsorted() {
        assert_eq!(
            QUERY.split(" FROM ").next().unwrap(),
            "SELECT ID,Description_lang,Display3_lang,Display2_lang,Display1_lang,Display_lang"
        );
        assert!(QUERY.ends_with("WHERE (VerifiedBuild>0)=? AND locale=?"));
        assert!(!QUERY.contains("ORDER BY"));
    }
}
