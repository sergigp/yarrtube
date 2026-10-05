/// The release this binary was built for, stamped by the release build
/// (`YARRTUBE_VERSION`); `dev` when built without one. Cargo.toml's version
/// is deliberately not used: CI bumps only the git tag.
pub const APP_VERSION: &str = match option_env!("YARRTUBE_VERSION") {
    Some(version) => version,
    None => "dev",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_report_dev_without_a_build_version() {
        assert_eq!(APP_VERSION, "dev");
    }
}
