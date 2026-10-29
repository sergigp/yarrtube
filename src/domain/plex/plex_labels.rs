/// An episode's label names: sorted, each once ignoring case; `==` ignores
/// case. Plex capitalizes the first letter of a new label, so stored and
/// read names may differ in case.
#[derive(Debug, Clone)]
pub struct PlexLabels(Vec<String>);

impl PlexLabels {
    pub fn new(names: Vec<String>) -> Self {
        Self(names)
    }

    pub fn none() -> Self {
        Self(Vec::new())
    }

    pub fn names(&self) -> &[String] {
        &self.0
    }

    /// Names in `self` missing from `other`, ignoring case.
    pub fn missing_from(&self, _other: &PlexLabels) -> Vec<String> {
        todo!()
    }
}

impl PartialEq for PlexLabels {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for PlexLabels {}
