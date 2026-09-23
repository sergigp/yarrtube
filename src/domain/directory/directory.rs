use super::directory_path::DirectoryPath;

/// A listed directory under the videos root and the names of its immediate
/// subdirectories. One level only: videos are stored one directory per video,
/// so a recursive listing of the share would be almost entirely noise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directory {
    pub path: DirectoryPath,
    pub subdirectories: Vec<String>,
}
