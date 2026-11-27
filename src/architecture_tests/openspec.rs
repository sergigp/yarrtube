//! The artifacts of the OpenSpec changes in progress (`openspec/changes/<change>/`,
//! the archive left out: archived changes are a frozen record).

use std::path::Path;

pub struct Artifact {
    /// Path relative to the repository root.
    pub path: String,
    pub text: String,
}

impl Artifact {
    pub fn new(path: &str, text: &str) -> Self {
        Self {
            path: path.to_string(),
            text: text.to_string(),
        }
    }

    /// The `## ` headings, in order, without the `## `.
    pub fn sections(&self) -> Vec<&str> {
        self.text
            .lines()
            .filter_map(|line| line.strip_prefix("## "))
            .map(str::trim)
            .collect()
    }

    /// The lines of the `## <heading>` section, up to the next `## `.
    pub fn section(&self, heading: &str) -> Vec<&str> {
        self.text
            .lines()
            .skip_while(|line| line.strip_prefix("## ").map(str::trim) != Some(heading))
            .skip(1)
            .take_while(|line| !line.starts_with("## "))
            .collect()
    }
}

/// Every `<file_name>` of the changes in progress.
pub fn active_artifacts(file_name: &str) -> Vec<Artifact> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let changes = root.join("openspec/changes");
    let Ok(entries) = std::fs::read_dir(&changes) else {
        return Vec::new();
    };
    let mut artifacts: Vec<Artifact> = entries
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir() && path.file_name().is_some_and(|name| name != "archive"))
        .map(|path| path.join(file_name))
        .filter(|path| path.is_file())
        .map(|path| {
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            Artifact::new(&relative, &std::fs::read_to_string(&path).unwrap())
        })
        .collect();
    artifacts.sort_by(|a, b| a.path.cmp(&b.path));
    artifacts
}
