use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::rules::Rule;

/// The content of a config file. See `config.example.toml` for the format.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default, rename = "rule")]
    pub rules: Vec<Rule>,
    /// Warn when a new minor or major version of sweep is released.
    #[serde(default = "enabled")]
    pub update_check: bool,
}

/// What sweep uses when the user has no config file.
impl Default for Config {
    fn default() -> Self {
        Self {
            rules: Vec::new(),
            update_check: true,
        }
    }
}

fn enabled() -> bool {
    true
}

/// `~/.config/sweep/config.toml`, the same place on every operating system.
pub fn default_path() -> Option<PathBuf> {
    env::home_dir().map(|home| home.join(".config").join("sweep").join("config.toml"))
}

/// The config file to read: the one given on the command line,
/// otherwise the default one, but only if the user created it.
pub fn find(explicit_path: Option<PathBuf>) -> Option<PathBuf> {
    explicit_path.or_else(|| default_path().filter(|path| path.is_file()))
}

/// Reads the user's own settings and rules from a config file.
pub fn load(path: &Path) -> Result<Config> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("Cannot read the config file {}", path.display()))?;

    parse(&text).with_context(|| format!("Invalid config file {}", path.display()))
}

fn parse(text: &str) -> Result<Config> {
    let config: Config = toml::from_str(text)?;

    for rule in &config.rules {
        if let Err(problem) = rule.check() {
            bail!("rule \"{}\": {problem}", rule.name());
        }
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_rules() {
        let rules = parse(
            r#"
            [[rule]]
            name = "Terraform"
            markers = ["*.tf"]
            artifacts = [".terraform"]
            "#,
        )
        .unwrap()
        .rules;

        assert_eq!(rules.len(), 1);
        assert!(rules[0].is_artifact(Path::new("infra/.terraform")));
    }

    #[test]
    fn accepts_an_empty_file() {
        assert!(parse("").unwrap().rules.is_empty());
    }

    #[test]
    fn checks_for_updates_unless_disabled() {
        assert!(parse("").unwrap().update_check);
        assert!(!parse("update_check = false").unwrap().update_check);
    }

    #[test]
    fn refuses_a_misspelled_field() {
        let result = parse(
            r#"
            [[rule]]
            name = "Terraform"
            markers = ["*.tf"]
            artifact = [".terraform"]
            "#,
        );

        assert!(result.is_err());
    }

    #[test]
    fn refuses_an_invalid_rule() {
        let error = parse(
            r#"
            [[rule]]
            name = "Danger"
            markers = ["x.json"]
            artifacts = [".."]
            "#,
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "rule \"Danger\": artifacts must be folder names, not paths"
        );
    }

    #[test]
    fn example_file_is_valid() {
        let rules = parse(include_str!("../config.example.toml")).unwrap().rules;

        assert!(!rules.is_empty());
    }
}
