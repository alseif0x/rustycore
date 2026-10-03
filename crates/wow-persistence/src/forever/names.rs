//! Transient target name-rule projections. No Debug: patterns are private
//! client data, not diagnostics. Effective precedence belongs to wow-data.

#[derive(Default)]
pub struct NameRows {
    /// ID, Name, signed Language
    pub profanity: Vec<(u32, String, i8)>,
    /// ID, Name
    pub reserved: Vec<(u32, String)>,
    /// ID, Name, unsigned LocaleMask
    pub locale_reserved: Vec<(u32, String, u8)>,
    /// ID, unsigned CreateCharsetMask
    pub categories: Vec<(u32, u8)>,
}

pub struct NameOverlays {
    pub official: NameRows,
    pub custom: NameRows,
}
