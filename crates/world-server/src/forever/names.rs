//! Translate transient SQL rows into the target data owner at composition.
use wow_data::forever_names::{Category, LocaleReserved, NameRecords, Profanity, Reserved};
use wow_persistence::forever::names::NameRows;
use wow_world::forever::name_rules::{NameRules, Patterns};

pub(super) fn compile(
    names: &wow_data::forever_names::ForeverNames,
    sql_reserved: Vec<String>,
) -> anyhow::Result<NameRules> {
    let mut locales: [Patterns; 12] = std::array::from_fn(|_| Vec::new());
    for (locale, patterns) in locales.iter_mut().enumerate() {
        // DB2Stores.cpp:1416-1458: locale-reserved follows profanity and has
        // the same rejection code. Keep raw expressions; never lowercase them.
        for expression in names
            .profanity(locale as u8)
            .expect("bounded locale")
            .iter()
            .chain(names.locale_reserved(locale as u8).expect("bounded locale"))
        {
            patterns.push(super::name_regex::compile(expression)?);
        }
    }
    let global = names
        .reserved()
        .iter()
        .map(|expression| super::name_regex::compile(expression))
        .collect::<anyhow::Result<Patterns>>()?;
    Ok(NameRules::new(
        locales,
        global,
        sql_reserved,
        names.charset_categories(),
    ))
}

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

    #[test]
    fn compiled_composition_keeps_raw_perl_expressions_and_rule_order() {
        use wow_world::forever::name_rules::{NamePolicy, NameRejection, Validation};
        let names = NameRecords {
            profanity: vec![Profanity {
                id: 1,
                name: "\\bcat\\b".into(),
                language: 6,
            }],
            reserved: vec![Reserved {
                id: 2,
                name: "^elune$".into(),
            }],
            locale_reserved: vec![LocaleReserved {
                id: 3,
                name: "^sun$".into(),
                locale_mask: 1 << 6,
            }],
            categories: vec![Category {
                id: 1,
                creation_charset: 4,
            }],
        }
        .finish(Default::default(), Default::default(), &Default::default())
        .unwrap();
        let rules = compile(&names, vec!["Moon".into()]).unwrap();
        let policy = NamePolicy {
            minimum_units: 2,
            strict_mask: 0,
            creation_charset: rules.creation_charset(1),
        };
        for (name, rejection) in [
            ("CAT", NameRejection::Profane),
            ("Sun", NameRejection::Profane),
            ("Elune", NameRejection::Reserved),
            ("Moon", NameRejection::Reserved),
        ] {
            assert!(
                matches!(rules.check(name, policy, 6, false), Ok(Validation::Rejected(actual)) if actual == rejection)
            );
        }
        assert!(matches!(
            rules.check("Catfish", policy, 6, false),
            Ok(Validation::Accepted(_))
        ));
        assert!(matches!(
            rules.check("Cat", policy, 0, false),
            Ok(Validation::Accepted(_))
        ));
    }
}
