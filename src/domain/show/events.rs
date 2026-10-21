use serde::Serialize;

/// A source's show directory was renamed from `from` to `to` (both absolute).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShowMoved {
    pub from: String,
    pub to: String,
}

impl ShowMoved {
    pub const EVENT_TYPE: &str = "show_moved";
}
