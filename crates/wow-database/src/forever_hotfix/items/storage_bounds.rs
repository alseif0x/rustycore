use super::LoadError;
use crate::SqlResult;

pub(super) const ITEM_SQUISH_SIZE_QUERY: &str = "SELECT MAX(ID) + 1 FROM item_squish_era";

pub(super) fn decode(row: &SqlResult) -> Result<u32, LoadError> {
    if row.is_empty() {
        return Ok(0);
    }
    if row.field_count() != 1 {
        return Err(LoadError::InvalidRow);
    }
    let value = if row.is_null(0) {
        0
    } else {
        row.try_read::<u64>(0)
            .or_else(|| row.try_read::<i64>(0).map(|value| value as u64))
            .ok_or(LoadError::InvalidRow)?
    };
    Ok(value as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_squish_allocation_query_is_separate_and_source_width_is_narrowed() {
        assert_eq!(
            ITEM_SQUISH_SIZE_QUERY,
            "SELECT MAX(ID) + 1 FROM item_squish_era"
        );
        assert_eq!(decode(&SqlResult::empty()).unwrap(), 0);
    }
}
