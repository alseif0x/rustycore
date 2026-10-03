use anyhow::{Result, ensure};
use std::collections::BTreeMap;

/// Localized raw bytes, deliberately not Debug/display or lossy UTF-8.
/// A missing locale and a present empty string remain distinct. No fallback.
#[derive(Clone, Default)]
pub struct SpellText {
    locales: BTreeMap<u8, Vec<u8>>,
}

impl SpellText {
    pub fn from_locale(locale: u8, bytes: Vec<u8>) -> Result<Self> {
        ensure!(
            locale < 12 && locale != 9,
            "Invalid source spell text locale"
        );
        Ok(Self {
            locales: BTreeMap::from([(locale, bytes)]),
        })
    }

    pub fn at(&self, locale: u8) -> Option<&[u8]> {
        self.locales.get(&locale).map(Vec::as_slice)
    }

    pub(super) fn check_sql_main(&self) -> Result<()> {
        ensure!(
            self.locales.len() == 1 && self.locales.contains_key(&0),
            "Main SQL spell text must be enUS only"
        );
        Ok(())
    }

    pub(super) fn with_sql_main(mut self, incoming: Self) -> Result<Self> {
        incoming.check_sql_main()?;
        for (locale, bytes) in incoming.locales {
            self.overlay_locale(locale, bytes);
        }
        Ok(self)
    }

    /// Source AddString: nonempty values overwrite; empty values never erase.
    /// Locale validity is checked once by the completed catalog composition.
    pub(super) fn overlay_locale(&mut self, locale: u8, bytes: Vec<u8>) {
        if !bytes.is_empty() {
            self.locales.insert(locale, bytes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SpellText;
    #[test]
    fn spell_text_retains_raw_bytes_and_distinguishes_absent_empty_locales() {
        let value = SpellText::from_locale(6, vec![0xFF, 0, b'a']).unwrap();
        assert_eq!(value.at(6), Some(&[0xFF, 0, b'a'][..]));
        assert_eq!(value.at(0), None);
        let empty = SpellText::from_locale(0, Vec::new()).unwrap();
        assert_eq!(empty.at(0), Some(&[][..]));
        assert_eq!(empty.at(6), None);
        for invalid in [9, 12, 255] {
            assert!(SpellText::from_locale(invalid, Vec::new()).is_err());
        }
    }
}
