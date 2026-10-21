use std::fmt;
use std::path::{Path, PathBuf};

use crate::size::Size;

/// A generated directory that can be removed.
#[derive(Debug)]
pub struct Artifact {
    path: PathBuf,
    project: String,
    size: Size,
}

impl Artifact {
    pub fn new(path: PathBuf, project: String, size: Size) -> Self {
        Self {
            path,
            project,
            size,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn project(&self) -> &str {
        &self.project
    }

    pub fn size(&self) -> Size {
        self.size
    }
}

impl fmt::Display for Artifact {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "{:>10}  [{}] {}",
            self.size,
            self.project,
            self.path.display()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_size_project_and_path() {
        let artifact = Artifact::new(
            PathBuf::from("app/target"),
            "Rust".to_string(),
            Size::from_bytes(2048),
        );

        assert_eq!(
            artifact.to_string(),
            format!("    2.0 KB  [Rust] {}", Path::new("app/target").display())
        );
    }
}
