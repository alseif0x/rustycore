//! Translate transient SQL rows into the target data owner at composition.
use wow_data::forever_names::{Category, LocaleReserved, NameRecords, Profanity, Reserved};
use wow_persistence::forever::names::NameRows;

pub(super) fn records(rows: NameRows) -> NameRecords {
    NameRecords {
        profanity: rows
            .profanity
            .into_iter()
            .map(|(id, name, language)| Profanity { id, name, language })
            .collect(),
        reserved: rows
            .reserved
            .into_iter()
            .map(|(id, name)| Reserved { id, name })
            .collect(),
        locale_reserved: rows
            .locale_reserved
            .into_iter()
            .map(|(id, name, locale_mask)| LocaleReserved {
                id,
                name,
                locale_mask,
            })
            .collect(),
        categories: rows
            .categories
            .into_iter()
            .map(|(id, creation_charset)| Category {
                id,
                creation_charset,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_language_and_independent_locale_mask_survive_composition() {
        let records = records(NameRows {
            profanity: vec![(1, "synthetic".into(), -1)],
            reserved: vec![(2, "synthetic-global".into())],
            locale_reserved: vec![(3, "synthetic-local".into(), 0x80)],
            categories: vec![(4, 16)],
        });
        let names = records
            .finish(Default::default(), Default::default(), &Default::default())
            .unwrap();
        assert_eq!(names.profanity(6).unwrap().len(), 1);
        assert!(names.profanity(9).unwrap().is_empty());
        assert_eq!(names.reserved().len(), 1);
        assert_eq!(names.locale_reserved(7).unwrap().len(), 1);
        assert!(names.locale_reserved(6).unwrap().is_empty());
        assert_eq!(names.creation_charset(4), 16);
    }
}
