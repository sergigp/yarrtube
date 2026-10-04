use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChannelCreated {
    pub channel_id: String,
}

impl ChannelCreated {
    pub const EVENT_TYPE: &str = "channel_created";
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChannelDeleted {
    pub channel_id: String,
    pub name: String,
    pub path: String,
}

impl ChannelDeleted {
    pub const EVENT_TYPE: &str = "channel_deleted";
}
