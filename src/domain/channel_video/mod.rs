#[allow(clippy::module_inception)]
pub mod channel_video;
pub mod events;

pub use channel_video::ChannelVideo;
pub use events::{VideoAddedToChannel, VideoRemovedFromChannel};
