pub mod errors;
#[allow(clippy::module_inception)]
pub mod preferences;

pub use errors::{ReadPreferencesError, SavePreferencesError};
pub use preferences::Preferences;
