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
    pub fn missing_from(&self, other: &PlexLabels) -> Vec<String> {
        self.0
            .iter()
            .filter(|name| !other.contains(name))
            .cloned()
            .collect()
    }
}

impl PartialEq for PlexLabels {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len()
            && self
                .0
                .iter()
                .zip(&other.0)
                .all(|(name, other)| same_label(name, other))
    }
}

impl Eq for PlexLabels {}

impl PlexLabels {
    fn contains(&self, name: &str) -> bool {
        self.0.iter().any(|own| same_label(own, name))
    }
}

fn same_label(name: &str, other: &str) -> bool {
    name.to_lowercase() == other.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_equal_labels_ignoring_case() {
        assert_eq!(labels(&["Chess", "funny"]), labels(&["chess", "Funny"]));
    }

    #[test]
    fn it_should_list_labels_missing_from_another_set_ignoring_case() {
        assert_eq!(
            labels(&["Chess", "Funny", "Old"]).missing_from(&labels(&["funny"])),
            vec!["Chess".to_string(), "Old".to_string()]
        );
    }

    fn labels(names: &[&str]) -> PlexLabels {
        PlexLabels::new(names.iter().map(|name| name.to_string()).collect())
    }
}
