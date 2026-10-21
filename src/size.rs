use std::fmt;
use std::fs;
use std::iter::Sum;
use std::ops::AddAssign;
use std::path::Path;

/// Units used to display a size, each one 1024 times bigger than the previous.
const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

/// An amount of disk space, displayed in a human-readable unit (e.g. `1.5 GB`).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Size(u64);

impl Size {
    pub fn from_bytes(bytes: u64) -> Self {
        Self(bytes)
    }

    /// Adds up the size of every file inside `path`, sub-directories included.
    /// Unreadable entries are skipped: the result is an estimate.
    pub fn of_directory(path: &Path) -> Self {
        let mut bytes = 0;
        let mut to_visit = vec![path.to_path_buf()];

        while let Some(directory) = to_visit.pop() {
            let Ok(entries) = fs::read_dir(&directory) else {
                continue;
            };

            for entry in entries.flatten() {
                // `metadata` does not follow symbolic links, so linked folders are not counted.
                let Ok(metadata) = entry.metadata() else {
                    continue;
                };

                if metadata.is_dir() {
                    to_visit.push(entry.path());
                } else {
                    bytes += metadata.len();
                }
            }
        }

        Self::from_bytes(bytes)
    }
}

impl AddAssign for Size {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl Sum for Size {
    fn sum<I: Iterator<Item = Self>>(sizes: I) -> Self {
        Self(sizes.map(|size| size.0).sum())
    }
}

impl fmt::Display for Size {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let mut value = self.0 as f64;
        let mut unit = 0;

        while value >= 1024.0 && unit < UNITS.len() - 1 {
            value /= 1024.0;
            unit += 1;
        }

        let text = if unit == 0 {
            format!("{} B", self.0)
        } else {
            format!("{value:.1} {}", UNITS[unit])
        };

        // `pad` keeps alignment flags like `{:>10}` working.
        formatter.pad(&text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn adds_up_files_in_nested_directories() {
        let root = TempDir::new().unwrap();
        fs::create_dir_all(root.path().join("a/b")).unwrap();
        fs::write(root.path().join("one.txt"), [0; 100]).unwrap();
        fs::write(root.path().join("a/b/two.txt"), [0; 50]).unwrap();

        assert_eq!(Size::of_directory(root.path()), Size::from_bytes(150));
    }

    #[test]
    fn displays_human_readable_units() {
        assert_eq!(Size::from_bytes(0).to_string(), "0 B");
        assert_eq!(Size::from_bytes(1023).to_string(), "1023 B");
        assert_eq!(Size::from_bytes(1024).to_string(), "1.0 KB");
        assert_eq!(Size::from_bytes(1536 * 1024).to_string(), "1.5 MB");
        assert_eq!(
            Size::from_bytes(3 * 1024 * 1024 * 1024).to_string(),
            "3.0 GB"
        );
    }

    #[test]
    fn keeps_alignment_flags() {
        assert_eq!(format!("{:>8}", Size::from_bytes(1024)), "  1.0 KB");
    }

    #[test]
    fn sums_sizes() {
        let total: Size = [1, 2, 3].into_iter().map(Size::from_bytes).sum();

        assert_eq!(total, Size::from_bytes(6));
    }
}
