use crate::domain::video_category::VideoCategoryChoice;

/// A video copy's categories as offered for editing, and whether its
/// channel or playlist has AI categorization on.
#[derive(Debug, Clone, PartialEq)]
pub struct VideoCategoryChoices {
    pub choices: Vec<VideoCategoryChoice>,
    /// Off: deleting its AI verdicts clears them, nothing asks again.
    pub categorization_enabled: bool,
}
