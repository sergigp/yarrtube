use super::SummaryPromptSection;
use crate::domain::shared::ValidationError;

/// The prompt shipped with this version; used until the user saves one.
const SHIPPED: &str = include_str!("../../../prompts/summaries/v3.md");

/// The summary prompt document: a free-form note, then one `## <heading>`
/// section per `SummaryPromptSection`, each using only its own variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryPrompt(String);

/// One `## <heading>` section: its heading and the lines below it.
struct Section<'a> {
    heading: &'a str,
    lines: Vec<&'a str>,
}

impl SummaryPrompt {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        let sections = Self::sections(&value);
        Self::ensure_known(&sections)?;
        Self::ensure_unique(&sections)?;
        Self::ensure_complete(&sections)?;
        Self::ensure_not_empty(&sections)?;
        Self::ensure_variables_allowed(&sections)?;
        Ok(Self(value))
    }

    /// A prompt as it was saved, without checking it: rules that changed
    /// since must not lose the user's text. Whoever uses it checks it then.
    pub fn from_stored(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn shipped() -> Self {
        Self(SHIPPED.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Every section in order; the note before the first one is left out.
    fn sections(value: &str) -> Vec<Section<'_>> {
        value.lines().fold(Vec::new(), |mut sections, line| {
            match (line.strip_prefix("## "), sections.last_mut()) {
                (Some(heading), _) => sections.push(Section {
                    heading: heading.trim(),
                    lines: Vec::new(),
                }),
                (None, Some(section)) => section.lines.push(line),
                (None, None) => {}
            }
            sections
        })
    }

    fn ensure_known(sections: &[Section]) -> Result<(), ValidationError> {
        match sections
            .iter()
            .find(|section| SummaryPromptSection::from_heading(section.heading).is_none())
        {
            Some(section) => Err(ValidationError(format!(
                "Prompt has an unknown section \"{}\"",
                section.heading
            ))),
            None => Ok(()),
        }
    }

    fn ensure_unique(sections: &[Section]) -> Result<(), ValidationError> {
        match sections.iter().enumerate().find(|(index, section)| {
            sections[..*index]
                .iter()
                .any(|earlier| earlier.heading == section.heading)
        }) {
            Some((_, section)) => Err(ValidationError(format!(
                "Prompt has the \"{}\" section twice",
                section.heading
            ))),
            None => Ok(()),
        }
    }

    fn ensure_complete(sections: &[Section]) -> Result<(), ValidationError> {
        match SummaryPromptSection::ALL.into_iter().find(|expected| {
            !sections
                .iter()
                .any(|section| section.heading == expected.heading())
        }) {
            Some(missing) => Err(ValidationError(format!(
                "Prompt is missing the \"{}\" section",
                missing.heading()
            ))),
            None => Ok(()),
        }
    }

    fn ensure_not_empty(sections: &[Section]) -> Result<(), ValidationError> {
        match sections
            .iter()
            .find(|section| section.lines.iter().all(|line| line.trim().is_empty()))
        {
            Some(section) => Err(ValidationError(format!(
                "Prompt section \"{}\" is empty",
                section.heading
            ))),
            None => Ok(()),
        }
    }

    fn ensure_variables_allowed(sections: &[Section]) -> Result<(), ValidationError> {
        sections.iter().try_for_each(|section| {
            let allowed = SummaryPromptSection::from_heading(section.heading)
                .map_or(&[][..], SummaryPromptSection::variables);
            match section
                .lines
                .iter()
                .flat_map(|line| Self::variables(line))
                .find(|variable| !allowed.contains(variable))
            {
                Some(variable) => Err(ValidationError(format!(
                    "Prompt section \"{}\" can't use {{{variable}}}; it can use {}",
                    section.heading,
                    allowed
                        .iter()
                        .map(|name| format!("{{{name}}}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ))),
                None => Ok(()),
            }
        })
    }

    /// The `{name}` variables in `line`: lowercase letters and `_` between
    /// braces. Other braced text (`{"match": true}`, `{}`) isn't one.
    fn variables(line: &str) -> Vec<&str> {
        line.split('{')
            .skip(1)
            .filter_map(|after| after.split_once('}').map(|(name, _)| name))
            .filter(|name| {
                !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '_')
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_accept_the_shipped_prompt() {
        assert_eq!(SummaryPrompt::new(SHIPPED), Ok(SummaryPrompt::shipped()));
    }

    #[test]
    fn it_should_reject_a_missing_section() {
        let prompt = remove_section("map user");

        assert_eq!(
            SummaryPrompt::new(prompt),
            Err(rejected("Prompt is missing the \"map user\" section"))
        );
    }

    #[test]
    fn it_should_reject_an_unknown_section() {
        let prompt = format!("{SHIPPED}\n## sytem\n\nBe brief.\n");

        assert_eq!(
            SummaryPrompt::new(prompt),
            Err(rejected("Prompt has an unknown section \"sytem\""))
        );
    }

    #[test]
    fn it_should_reject_a_repeated_section() {
        let prompt = format!("{SHIPPED}\n## user\n\nTitle: {{title}}\n");

        assert_eq!(
            SummaryPrompt::new(prompt),
            Err(rejected("Prompt has the \"user\" section twice"))
        );
    }

    #[test]
    fn it_should_reject_an_empty_section() {
        let prompt = replace_section("chapters", "\n   \n\n");

        assert_eq!(
            SummaryPrompt::new(prompt),
            Err(rejected("Prompt section \"chapters\" is empty"))
        );
    }

    #[test]
    fn it_should_reject_a_variable_the_section_cant_use() {
        let prompt = replace_section("reduce system", "\nSummarize {transcript} in {lang}.\n\n");

        assert_eq!(
            SummaryPrompt::new(prompt),
            Err(rejected(
                "Prompt section \"reduce system\" can't use {transcript}; it can use {lang}"
            ))
        );
    }

    #[test]
    fn it_should_ignore_variables_in_the_note_before_the_first_section() {
        let prompt = format!("Placeholders like {{name}} are filled in.\n\n{SHIPPED}");

        assert!(SummaryPrompt::new(prompt).is_ok());
    }

    #[test]
    fn it_should_ignore_braces_that_are_not_variables() {
        let prompt = replace_section(
            "system",
            "\nReply in {lang} as {\"match\": true}, {Title} or {}.\n\n",
        );

        assert!(SummaryPrompt::new(prompt).is_ok());
    }

    /// The shipped prompt with `heading`'s text replaced by `text`.
    fn replace_section(heading: &str, text: &str) -> String {
        let (before, after) = split_at_section(heading);
        format!("{before}## {heading}\n{text}{after}")
    }

    fn remove_section(heading: &str) -> String {
        let (before, after) = split_at_section(heading);
        format!("{before}{after}")
    }

    /// The shipped prompt before the `heading` line, and from the next
    /// section on.
    fn split_at_section(heading: &str) -> (&'static str, &'static str) {
        let start = SHIPPED.find(&format!("## {heading}\n")).unwrap();
        let next = SHIPPED[start + 3..]
            .find("\n## ")
            .map_or(SHIPPED.len(), |offset| start + 3 + offset + 1);
        (&SHIPPED[..start], &SHIPPED[next..])
    }

    fn rejected(message: &str) -> ValidationError {
        ValidationError(message.to_string())
    }
}
