use crate::domain::shared::Quality;
use std::path::PathBuf;

/// Where, and at which quality, the playlist or channel holding a video
/// wants it: the show directory its files go to and its owner's quality.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoTarget {
    pub output_dir: PathBuf,
    pub quality: Quality,
}
