//! 02245dcd DB2DatabaseLoader::AddString/Load/LoadStrings and LocalizedString.
//! No implicit locale fallback; source WriteRecord reads exactly Str[locale].
use std::collections::BTreeMap;

/// Metadata field order: Description, Display3, Display2, Display1, Display.
/// No Debug: private client/SQL text must not reach diagnostic logs.
#[derive(Clone, Default)]
pub struct SparseItemStrings {
    locales: BTreeMap<u8, [Vec<u8>; 5]>,
}

impl SparseItemStrings {
    pub fn for_locale(locale: u8, fields: [Vec<u8>; 5]) -> Self {
        Self {
            locales: BTreeMap::from([(locale, fields)]),
        }
    }

    pub fn field(&self, locale: u8, field: usize) -> &[u8] {
        self.locales
            .get(&locale)
            .and_then(|fields| fields.get(field))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Empty AddString input preserves the old slot. Embedded NUL is retained:
    /// source tests input length before copying, then writes a C-string prefix.
    pub fn overlay(&mut self, locale: u8, fields: [Vec<u8>; 5]) {
        let slots = self.locales.entry(locale).or_default();
        for (slot, text) in slots.iter_mut().zip(fields) {
            if !text.is_empty() {
                *slot = text;
            }
        }
    }

    pub(crate) fn overlay_en_us(&mut self, mut sql: Self) {
        if let Some(fields) = sql.locales.remove(&0) {
            self.overlay(0, fields);
        }
    }
}
