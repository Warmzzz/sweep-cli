use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::artifact::Artifact;
use crate::directory::Directory;
use crate::path_list::PathList;
use crate::rules::Rule;
use crate::size::Size;

/// Version control folders never contain build artifacts, so they are skipped.
const IGNORED_DIRS: &[&str] = &[".git", ".hg", ".svn"];

/// Walks `root` and returns every artifact directory with its size, biggest first.
/// Excluded directories are neither walked into nor reported.
pub fn scan(root: &Path, rules: &[Rule], exclusions: &PathList) -> Vec<Artifact> {
    // Measuring a size reads every file of the artifact: this is the slow part,
    // so the artifacts are measured in parallel, spread over all CPU cores.
    let mut artifacts: Vec<Artifact> = find_artifacts(root, rules, exclusions)
        .into_par_iter()
        .map(|(path, project)| {
            let size = Size::of_directory(&path);
            Artifact::new(path, project, size)
        })
        .collect();

    artifacts.sort_by(|a, b| b.size().cmp(&a.size()).then(a.path().cmp(b.path())));
    artifacts
}

/// Returns the path of every artifact directory, with the project type that generated it.
fn find_artifacts(root: &Path, rules: &[Rule], exclusions: &PathList) -> Vec<(PathBuf, String)> {
    let mut found = Vec::new();
    let mut to_visit = vec![root.to_path_buf()];

    while let Some(path) = to_visit.pop() {
        let directory = match Directory::read(&path) {
            Ok(directory) => directory,
            Err(error) => {
                eprintln!("Warning: cannot read {}: {error}", path.display());
                continue;
            }
        };

        let matching_rules: Vec<&Rule> = rules
            .iter()
            .filter(|rule| rule.matches(&directory))
            .collect();

        for subdir in directory.into_subdirs() {
            if is_ignored(&subdir) || exclusions.covers(&subdir) {
                continue;
            }

            if let Some(rule) = matching_rules.iter().find(|rule| rule.is_artifact(&subdir)) {
                // An artifact is never walked into: everything inside it goes away with it.
                found.push((subdir, rule.name().to_string()));
            } else {
                to_visit.push(subdir);
            }
        }
    }

    found
}

fn is_ignored(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| IGNORED_DIRS.iter().any(|ignored| name == *ignored))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules;
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
        scan(root, &rules::built_in(), &PathList::default())
            .into_iter()
            .map(|artifact| artifact.path().strip_prefix(root).unwrap().to_path_buf())
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
    fn uses_the_rules_it_is_given() {
        let root = TempDir::new().unwrap();
        create_file(root.path(), "infra/main.tf");
        create_dir(root.path(), "infra/.terraform");
        let rules = vec![Rule::new("Terraform", &["*.tf"], &[".terraform"])];

        let artifacts = scan(root.path(), &rules, &PathList::default());

        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].project(), "Terraform");
    }

    #[test]
    fn skips_excluded_directories() {
        let root = TempDir::new().unwrap();
        create_file(root.path(), "keep/Cargo.toml");
        create_dir(root.path(), "keep/target");
        create_file(root.path(), "clean/Cargo.toml");
        create_dir(root.path(), "clean/target");

        let exclusions = PathList::new([root.path().join("keep")]);
        let found: Vec<PathBuf> = scan(root.path(), &rules::built_in(), &exclusions)
            .iter()
            .map(|artifact| {
                artifact
                    .path()
                    .strip_prefix(root.path())
                    .unwrap()
                    .to_path_buf()
            })
            .collect();

        assert_eq!(found, vec![PathBuf::from("clean/target")]);
    }

    #[test]
    fn measures_artifacts_and_lists_biggest_first() {
        let root = TempDir::new().unwrap();
        create_file(root.path(), "small/Cargo.toml");
        create_file(root.path(), "big/Cargo.toml");
        fs::create_dir_all(root.path().join("small/target")).unwrap();
        fs::create_dir_all(root.path().join("big/target/debug")).unwrap();
        fs::write(root.path().join("small/target/a"), [0; 10]).unwrap();
        fs::write(root.path().join("big/target/debug/b"), [0; 500]).unwrap();

        let sizes: Vec<Size> = scan(root.path(), &rules::built_in(), &PathList::default())
            .iter()
            .map(Artifact::size)
            .collect();

        assert_eq!(sizes, vec![Size::from_bytes(500), Size::from_bytes(10)]);
    }
}
