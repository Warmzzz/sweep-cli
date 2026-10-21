use std::fmt;
use std::path::{Path, PathBuf};

use crate::directory::Directory;
use crate::rules::{RULES, Rule};

/// Version control folders never contain build artifacts, so they are skipped.
const IGNORED_DIRS: &[&str] = &[".git", ".hg", ".svn"];

/// A generated directory that can be removed.
#[derive(Debug)]
pub struct Artifact {
    path: PathBuf,
    project: &'static str,
}

impl fmt::Display for Artifact {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "[{}] {}", self.project, self.path.display())
    }
}

/// Walks `root` and returns every artifact directory, sorted by path.
pub fn scan(root: &Path) -> Vec<Artifact> {
    let mut artifacts = Vec::new();
    let mut to_visit = vec![root.to_path_buf()];

    while let Some(path) = to_visit.pop() {
        let directory = match Directory::read(&path) {
            Ok(directory) => directory,
            Err(error) => {
                eprintln!("Warning: cannot read {}: {error}", path.display());
                continue;
            }
        };

        let matching_rules: Vec<&Rule> = RULES
            .iter()
            .filter(|rule| rule.matches(&directory))
            .collect();

        for subdir in directory.into_subdirs() {
            if let Some(rule) = matching_rules.iter().find(|rule| rule.is_artifact(&subdir)) {
                // An artifact is never walked into: everything inside it goes away with it.
                artifacts.push(Artifact {
                    path: subdir,
                    project: rule.name(),
                });
            } else if !is_ignored(&subdir) {
                to_visit.push(subdir);
            }
        }
    }

    artifacts.sort_by(|a, b| a.path.cmp(&b.path));
    artifacts
}

fn is_ignored(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| IGNORED_DIRS.iter().any(|ignored| name == *ignored))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_file(root: &Path, relative_path: &str) {
        let path = root.join(relative_path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "").unwrap();
    }

    fn create_dir(root: &Path, relative_path: &str) {
        fs::create_dir_all(root.join(relative_path)).unwrap();
    }

    fn found_paths(root: &Path) -> Vec<PathBuf> {
        scan(root)
            .into_iter()
            .map(|artifact| artifact.path.strip_prefix(root).unwrap().to_path_buf())
            .collect()
    }

    #[test]
    fn finds_artifact_next_to_its_marker() {
        let root = TempDir::new().unwrap();
        create_file(root.path(), "app/Cargo.toml");
        create_dir(root.path(), "app/target");

        assert_eq!(found_paths(root.path()), vec![PathBuf::from("app/target")]);
    }

    #[test]
    fn ignores_artifact_name_without_marker() {
        let root = TempDir::new().unwrap();
        create_dir(root.path(), "photos/bin");
        create_dir(root.path(), "photos/target");

        assert!(found_paths(root.path()).is_empty());
    }

    #[test]
    fn recognizes_marker_by_extension() {
        let root = TempDir::new().unwrap();
        create_file(root.path(), "Api/Api.csproj");
        create_dir(root.path(), "Api/bin");
        create_dir(root.path(), "Api/obj");

        assert_eq!(
            found_paths(root.path()),
            vec![PathBuf::from("Api/bin"), PathBuf::from("Api/obj")]
        );
    }

    #[test]
    fn finds_python_cache_in_nested_packages() {
        let root = TempDir::new().unwrap();
        create_file(root.path(), "pyproject.toml");
        create_file(root.path(), "app/models/user.py");
        create_dir(root.path(), "app/models/__pycache__");

        assert_eq!(
            found_paths(root.path()),
            vec![PathBuf::from("app/models/__pycache__")]
        );
    }

    #[test]
    fn does_not_walk_into_an_artifact() {
        let root = TempDir::new().unwrap();
        create_file(root.path(), "package.json");
        create_file(root.path(), "node_modules/lib/package.json");
        create_dir(root.path(), "node_modules/lib/node_modules");

        assert_eq!(
            found_paths(root.path()),
            vec![PathBuf::from("node_modules")]
        );
    }

    #[test]
    fn skips_version_control_folders() {
        let root = TempDir::new().unwrap();
        create_file(root.path(), ".git/Cargo.toml");
        create_dir(root.path(), ".git/target");

        assert!(found_paths(root.path()).is_empty());
    }

    #[test]
    fn displays_project_name_and_path() {
        let artifact = Artifact {
            path: PathBuf::from("app/target"),
            project: "Rust",
        };

        assert_eq!(
            artifact.to_string(),
            format!("[Rust] {}", Path::new("app/target").display())
        );
    }
}
