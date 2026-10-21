use std::fmt;
use std::fs;

use rayon::prelude::*;

use crate::artifact::Artifact;
use crate::safety::SafetyGuard;
use crate::size::Size;

/// What happened to one artifact.
enum Outcome {
    Deleted(Size),
    Skipped,
    Failed,
}

/// Totals of a cleanup, displayed once every deletion is done.
#[derive(Debug, Default)]
pub struct CleanupSummary {
    deleted: usize,
    skipped: usize,
    failed: usize,
    freed: Size,
}

impl CleanupSummary {
    fn record(&mut self, outcome: Outcome) {
        match outcome {
            Outcome::Deleted(size) => {
                self.deleted += 1;
                self.freed += size;
            }
            Outcome::Skipped => self.skipped += 1,
            Outcome::Failed => self.failed += 1,
        }
    }
}

impl fmt::Display for CleanupSummary {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "Deleted {} directory(ies), freed {}.",
            self.deleted, self.freed
        )?;

        if self.skipped > 0 {
            write!(formatter, " Skipped for safety: {}.", self.skipped)?;
        }
        if self.failed > 0 {
            write!(formatter, " Failed: {}.", self.failed)?;
        }

        Ok(())
    }
}

/// Deletes every artifact that the guard allows, in parallel.
pub fn delete_all(artifacts: &[Artifact], guard: &SafetyGuard) -> CleanupSummary {
    let outcomes: Vec<Outcome> = artifacts
        .par_iter()
        .map(|artifact| delete(artifact, guard))
        .collect();

    let mut summary = CleanupSummary::default();
    for outcome in outcomes {
        summary.record(outcome);
    }
    summary
}

fn delete(artifact: &Artifact, guard: &SafetyGuard) -> Outcome {
    let path = artifact.path();

    if let Err(reason) = guard.check(path) {
        eprintln!("  Skipped {}: {reason}", path.display());
        return Outcome::Skipped;
    }

    match fs::remove_dir_all(path) {
        Ok(()) => {
            println!("  Deleted {}", path.display());
            Outcome::Deleted(artifact.size())
        }
        Err(error) => {
            eprintln!("  Failed {}: {error}", path.display());
            Outcome::Failed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn artifact_in(dir: &TempDir, name: &str, bytes: u64) -> Artifact {
        let path = dir.path().join(name);
        fs::create_dir_all(&path).unwrap();
        Artifact::new(path, "Rust".to_string(), Size::from_bytes(bytes))
    }

    #[test]
    fn deletes_artifacts_and_adds_up_freed_space() {
        let root = TempDir::new().unwrap();
        let artifacts = vec![
            artifact_in(&root, "a/target", 100),
            artifact_in(&root, "b/target", 50),
        ];

        let summary = delete_all(&artifacts, &SafetyGuard::new(root.path()));

        assert!(artifacts.iter().all(|artifact| !artifact.path().exists()));
        assert_eq!((summary.deleted, summary.freed), (2, Size::from_bytes(150)));
    }

    #[test]
    fn keeps_artifacts_refused_by_the_guard() {
        let root = TempDir::new().unwrap();
        let elsewhere = TempDir::new().unwrap();
        let artifacts = vec![artifact_in(&elsewhere, "target", 100)];

        let summary = delete_all(&artifacts, &SafetyGuard::new(root.path()));

        assert!(artifacts[0].path().exists());
        assert_eq!((summary.deleted, summary.skipped), (0, 1));
    }

    #[test]
    fn displays_only_relevant_counts() {
        let summary = CleanupSummary {
            deleted: 2,
            freed: Size::from_bytes(2048),
            ..CleanupSummary::default()
        };

        assert_eq!(
            summary.to_string(),
            "Deleted 2 directory(ies), freed 2.0 KB."
        );
    }
}
