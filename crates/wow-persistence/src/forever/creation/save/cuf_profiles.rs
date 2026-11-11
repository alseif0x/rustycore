//! Frozen five-slot CUF projection; canonical construction/loading is absent.

/// Private profile names deliberately have no Debug implementation.
pub struct NewCufProfile {
    pub name: String,
    pub frame_height: u16,
    pub frame_width: u16,
    pub sort_by: u8,
    pub health_text: u8,
    pub bool_options: [bool; 27],
    pub top_point: u8,
    pub bottom_point: u8,
    pub left_point: u8,
    pub top_offset: u16,
    pub bottom_offset: u16,
    pub left_offset: u16,
}
