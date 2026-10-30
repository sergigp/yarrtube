pub mod errors;
#[allow(clippy::module_inception)]
pub mod preferences;
pub mod theme;

pub use errors::{ReadPreferencesError, SavePreferencesError};
pub use preferences::Preferences;
pub use theme::Theme;
