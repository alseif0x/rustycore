//! DB2DatabaseLoader.cpp:48-53 / HotfixDatabase.cpp:1478, 02245dcd.
//! Allocation observation only; no rows/IDs are printed or retained here.
use super::LoadError;
use crate::SqlResult;

pub(super) const RAND_PROP_SIZE_QUERY: &str = "SELECT MAX(ID) + 1 FROM rand_prop_points";

pub(super) fn decode(row: &SqlResult) -> Result<u32, LoadError> {
    if row.is_empty() {
        // No max-result does not extend the source's existing allocation.
        return Ok(0);
    }
    if row.field_count() != 1 {
        return Err(LoadError::InvalidRow);
    }
    let value = if row.is_null(0) {
        0 // source Field::GetUInt64 NULL
    } else {
        row.try_read::<u64>(0)
            .or_else(|| row.try_read::<i64>(0).map(|value| value as u64))
            .ok_or(LoadError::InvalidRow)?
    };
    Ok(narrow_source_size(value))
}

fn narrow_source_size(value: u64) -> u32 {
    value as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn allocation_query_has_no_batch_predicate_and_source_narrowing_is_not_clamping() {
        assert_eq!(
            RAND_PROP_SIZE_QUERY,
            "SELECT MAX(ID) + 1 FROM rand_prop_points"
        );
        assert_eq!(decode(&SqlResult::empty()).unwrap(), 0);
        for (value, expected) in [
            (0, 0),
            (1, 1),
            (u32::MAX as u64, u32::MAX),
            (u32::MAX as u64 + 1, 0),
            (u64::MAX, u32::MAX),
        ] {
            assert_eq!(narrow_source_size(value), expected);
        }
    }
}
