use prost::Message;

// Corresponds to SMSG_PLAYED_TIME
#[derive(Clone, PartialEq, Message)]
pub struct PlayedTime {
    // Total playtime in seconds, as u32.
    // Client expects this to be client-side playtime to sync with server.
    #[prost(uint32, tag = "1")]
    pub total_played_time: u32,
}
