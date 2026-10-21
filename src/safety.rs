use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::path_list::{PathList, normalize};

/// Folders owned by the operating system: nothing inside them is ever deleted,
/// even if it looks like a build artifact (e.g. a globally installed npm package).
#[cfg(windows)]
fn system_dirs() -> Vec<PathBuf> {
    [
        "SystemRoot",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "ProgramData",
    ]
    .into_iter()
    .filter_map(env::var_os)
    .map(PathBuf::from)
    .collect()
}

#[cfg(not(windows))]
fn system_dirs() -> Vec<PathBuf> {
    [
        "/bin",
        "/boot",
        "/dev",
        "/etc",
        "/lib",
        "/lib64",
        "/proc",
        "/sbin",
        "/sys",
        "/usr",
        "/Applications",
        "/Library",
        "/System",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

/// The last line of defense: checks each directory right before it is deleted.
pub struct SafetyGuard {
    scan_root: PathBuf,
    system_dirs: PathList,
    /// Paths that must survive, with the reason given when a deletion would remove them.
    vital_paths: Vec<(PathBuf, &'static str)>,
}

impl SafetyGuard {
    pub fn new(scan_root: &Path) -> Self {
        let vital_paths = [
            (env::home_dir(), "it contains your home directory"),
            (env::current_dir().ok(), "it contains the current directory"),
            (
                env::current_exe().ok(),
                "it contains the running sweep program",
            ),
        ]
        .into_iter()
        .filter_map(|(path, reason)| Some((normalize(&path?), reason)))
        .collect();

        Self {
            scan_root: normalize(scan_root),
            system_dirs: PathList::new(system_dirs()),
            vital_paths,
        }
    }

    /// Returns `Ok` when `path` can be deleted, or the reason why it must not be.
    pub fn check(&self, path: &Path) -> Result<(), &'static str> {
        let normalized = normalize(path);

        if normalized == self.scan_root || !normalized.starts_with(&self.scan_root) {
            return Err("it is not inside the scanned directory");
        }

        if self.system_dirs.covers(path) {
            return Err("it is inside a system directory");
        }

        if let Some((_, reason)) = self
            .vital_paths
            .iter()
            .find(|(vital, _)| vital.starts_with(&normalized))
        {
            return Err(reason);
        }

        // Checked last, as close as possible to the deletion itself.
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() => Err("it is a symbolic link"),
            Ok(metadata) if metadata.is_dir() => Ok(()),
            Ok(_) => Err("it is no longer a directory"),
            Err(_) => Err("it no longer exists"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn allows_a_directory_inside_the_scan_root() {
        let root = TempDir::new().unwrap();
        let target = root.path().join("app/target");
        fs::create_dir_all(&target).unwrap();

        assert_eq!(SafetyGuard::new(root.path()).check(&target), Ok(()));
    }

    #[test]
    fn refuses_the_scan_root_and_anything_outside_it() {
        let root = TempDir::new().unwrap();
        let other = TempDir::new().unwrap();
        let guard = SafetyGuard::new(root.path());

        assert!(guard.check(root.path()).is_err());
        assert!(guard.check(other.path()).is_err());
    }

    #[test]
    fn refuses_a_directory_that_no_longer_exists() {
        let root = TempDir::new().unwrap();

        let result = SafetyGuard::new(root.path()).check(&root.path().join("gone"));

        assert_eq!(result, Err("it no longer exists"));
    }

    #[test]
    fn refuses_a_file() {
        let root = TempDir::new().unwrap();
        let file = root.path().join("target");
        fs::write(&file, "").unwrap();

        let result = SafetyGuard::new(root.path()).check(&file);

        assert_eq!(result, Err("it is no longer a directory"));
    }

    #[test]
    fn refuses_the_folder_of_the_running_program() {
        // Tests run from `target/debug/deps`, inside this project's `target` folder.
        let project = Path::new(env!("CARGO_MANIFEST_DIR"));

        let result = SafetyGuard::new(project).check(&project.join("target"));

        assert_eq!(result, Err("it contains the running sweep program"));
    }

    #[test]
    fn refuses_anything_inside_a_system_directory() {
        let system_dir = system_dirs().remove(0);
        let filesystem_root = system_dir.ancestors().last().unwrap();

        let result = SafetyGuard::new(filesystem_root).check(&system_dir.join("node_modules"));

        assert_eq!(result, Err("it is inside a system directory"));
    }
}
