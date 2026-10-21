use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::rules::Rule;

/// The content of a config file. See `config.example.toml` for the format.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigFile {
    #[serde(default, rename = "rule")]
    rules: Vec<Rule>,
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

/// Reads the user's own rules from a config file.
pub fn load_rules(path: &Path) -> Result<Vec<Rule>> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("Cannot read the config file {}", path.display()))?;

    parse_rules(&text).with_context(|| format!("Invalid config file {}", path.display()))
}

fn parse_rules(text: &str) -> Result<Vec<Rule>> {
    let config: ConfigFile = toml::from_str(text)?;

    for rule in &config.rules {
        if let Err(problem) = rule.check() {
            bail!("rule \"{}\": {problem}", rule.name());
        }
    }

    Ok(config.rules)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_rules() {
        let rules = parse_rules(
            r#"
            [[rule]]
            name = "Terraform"
            markers = ["*.tf"]
            artifacts = [".terraform"]
            "#,
        )
        .unwrap();

        assert_eq!(rules.len(), 1);
        assert!(rules[0].is_artifact(Path::new("infra/.terraform")));
    }

    #[test]
    fn accepts_an_empty_file() {
        assert!(parse_rules("").unwrap().is_empty());
    }

    #[test]
    fn refuses_a_misspelled_field() {
        let result = parse_rules(
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
        let error = parse_rules(
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
        let rules = parse_rules(include_str!("../config.example.toml")).unwrap();

        assert!(!rules.is_empty());
    }
}
