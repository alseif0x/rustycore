//! Exact first query of SpellMgr::LoadSpellInfoCustomAttributes, 02245dcd.
use super::{LoadError, SpellCustomAttributeRow, SqlResult};
pub(super) const QUERY: &str = "SELECT entry, attributes FROM spell_custom_attr";

pub(super) fn decode(row: &SqlResult) -> Result<SpellCustomAttributeRow, LoadError> {
    // Match source GetUInt32 metadata as well as values: NULL, signed/text or
    // wrong-width columns are errors. Never mask unknown custom flag bits.
    decode_columns(row.field_count(), |index| {
        row.try_read_typed::<u32>(index).map(u64::from)
    })
}

fn decode_columns(
    columns: usize,
    mut read: impl FnMut(usize) -> Option<u64>,
) -> Result<SpellCustomAttributeRow, LoadError> {
    if columns != 2 {
        return Err(LoadError::InvalidRow);
    }
    let mut field = |index| {
        read(index)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or(LoadError::InvalidRow)
    };
    Ok(SpellCustomAttributeRow {
        spell: field(0)?,
        attributes: field(1)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_preserves_source_projection_without_sort_dedup_or_hotfix_filter() {
        assert_eq!(QUERY, "SELECT entry, attributes FROM spell_custom_attr");
    }

    #[test]
    fn all_unsigned_bits_and_zero_are_retained_in_column_order() {
        for (spell, attributes) in [(0, 0), (u32::MAX, u32::MAX), (7, 0x8000_0008)] {
            let values = [u64::from(spell), u64::from(attributes)];
            let mut seen = Vec::new();
            assert_eq!(
                decode_columns(2, |index| {
                    seen.push(index);
                    Some(values[index])
                }),
                Ok(SpellCustomAttributeRow { spell, attributes })
            );
            assert_eq!(seen, [0, 1]);
        }
    }

    #[test]
    fn typed_source_metadata_rejects_signed_text_null_and_wrong_integer_widths() {
        use crate::rust_type_compatible_with_database_field_like_cpp;
        for name in ["INT UNSIGNED", "UNSIGNED INT", "UNSIGNED LONG"] {
            assert!(rust_type_compatible_with_database_field_like_cpp::<u32>(
                name
            ));
        }
        for name in [
            "INT",
            "NULL",
            "VARCHAR",
            "UNSIGNED TINYINT",
            "UNSIGNED SMALLINT",
            "UNSIGNED BIGINT",
            "DOUBLE",
            "BIT",
        ] {
            assert!(!rust_type_compatible_with_database_field_like_cpp::<u32>(
                name
            ));
        }
    }

    #[test]
    fn missing_extra_null_and_overwide_columns_are_errors_not_empty_rows() {
        for columns in [0, 1, 3] {
            assert_eq!(
                decode_columns(columns, |_| panic!("bad shape read")),
                Err(LoadError::InvalidRow)
            );
        }
        for values in [
            [None, Some(0)],
            [Some(0), None],
            [Some(u64::from(u32::MAX) + 1), Some(0)],
            [Some(0), Some(u64::MAX)],
        ] {
            assert_eq!(
                decode_columns(2, |index| values[index]),
                Err(LoadError::InvalidRow)
            );
        }
        assert_eq!(decode(&SqlResult::empty()), Err(LoadError::InvalidRow));
    }
}
