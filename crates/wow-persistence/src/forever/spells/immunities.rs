//! World-only creature immunity source rows, not client hotfixes or live state.
//! Signed SQL masks are retained until source-width casts in domain startup.
pub struct CreatureImmunityRow {
    pub id: i32,
    pub school: i8,
    pub dispel: i16,
    pub mechanics: i64,
    pub effects: Vec<u8>,
    pub auras: Vec<u8>,
    pub immune_aoe: bool,
    pub immune_chain: bool,
}
