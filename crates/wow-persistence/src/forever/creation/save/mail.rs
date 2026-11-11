//! Frozen mail deque projection; canonical ownership, lifecycle and acknowledgement remain absent.
pub struct NewMailSave {
    pub updated: bool,
    pub messages: Vec<NewMailSaveEntry>,
}

pub enum NewMailSaveEntry {
    Unchanged,
    Changed {
        message_id: u64,
        items: Vec<u64>,
        removed_items: Vec<u64>,
        expire_time: i64,
        deliver_time: i64,
        money: u64,
        cod: u64,
        checked: u32,
    },
    Deleted {
        message_id: u64,
        items: Vec<u64>,
    },
}
