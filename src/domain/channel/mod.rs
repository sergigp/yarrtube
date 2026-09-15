#[allow(clippy::module_inception)]
pub mod channel;
pub mod channel_handle;
pub mod errors;
pub mod service;
pub mod video_limit;

pub use channel::Channel;
pub use channel_handle::ChannelHandle;
pub use errors::{CreateChannelError, DeleteChannelError};
pub use service::{ChannelService, CreateChannelOutcome};
pub use video_limit::VideoLimit;
