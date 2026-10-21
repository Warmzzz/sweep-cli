use std::path::{self, Component, Path, PathBuf};

/// Turns a path into a single comparable form: absolute, without `.` or `..`,
/// and lowercase on Windows, where `C:\Web` and `c:\web` are the same folder.
pub fn normalize(path: &Path) -> PathBuf {
    let absolute = path::absolute(path).unwrap_or_else(|_| path.to_path_buf());

    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other),
        }
    }

    if cfg!(windows) {
        PathBuf::from(normalized.to_string_lossy().to_lowercase())
    } else {
        normalized
    }
}

/// A list of directories. A path is covered when it is one of them or inside one of them.
#[derive(Default)]
pub struct PathList {
    dirs: Vec<PathBuf>,
}

impl PathList {
    pub fn new(dirs: impl IntoIterator<Item = PathBuf>) -> Self {
        Self {
            dirs: dirs.into_iter().map(|dir| normalize(&dir)).collect(),
        }
    }

    pub fn covers(&self, path: &Path) -> bool {
        // Most scans have no exclusion: skip the normalization work entirely.
        if self.dirs.is_empty() {
            return false;
        }

        let path = normalize(path);
        self.dirs.iter().any(|dir| path.starts_with(dir))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covers_the_directory_and_everything_inside() {
        let list = PathList::new([PathBuf::from("projects/web")]);

        assert!(list.covers(Path::new("projects/web")));
        assert!(list.covers(Path::new("projects/web/node_modules")));
    }

    #[test]
    fn does_not_cover_a_sibling_with_a_similar_name() {
        let list = PathList::new([PathBuf::from("projects/web")]);

        assert!(!list.covers(Path::new("projects/website")));
        assert!(!list.covers(Path::new("projects")));
    }

    #[test]
    fn ignores_dot_and_dot_dot_components() {
        let list = PathList::new([PathBuf::from("./projects/api/../web")]);

        assert!(list.covers(Path::new("projects/web/node_modules")));
    }

    #[test]
    fn empty_list_covers_nothing() {
        assert!(!PathList::default().covers(Path::new("anything")));
    }

    #[cfg(windows)]
    #[test]
    fn ignores_case_on_windows() {
        let list = PathList::new([PathBuf::from("Projects/WEB")]);

        assert!(list.covers(Path::new("projects/web/node_modules")));
    }
}
