use std::cmp::Reverse;
use std::fmt;

use crate::artifact::Artifact;
use crate::size::Size;

/// Statistics for every artifact of one project type.
struct ProjectStats {
    project: &'static str,
    count: usize,
    size: Size,
}

impl ProjectStats {
    fn new(project: &'static str) -> Self {
        Self {
            project,
            count: 0,
            size: Size::default(),
        }
    }

    fn add(&mut self, artifact: &Artifact) {
        self.count += 1;
        self.size += artifact.size();
    }
}

impl fmt::Display for ProjectStats {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "{:>10}  {} ({} directory(ies))",
            self.size, self.project, self.count
        )
    }
}

/// What a scan found, and how much space removing it would free.
pub struct Report {
    artifacts: Vec<Artifact>,
}

impl Report {
    pub fn new(artifacts: Vec<Artifact>) -> Self {
        Self { artifacts }
    }

    pub fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }

    pub fn total_size(&self) -> Size {
        self.artifacts.iter().map(Artifact::size).sum()
    }

    /// Groups the artifacts by project type, biggest total first.
    fn stats_by_project(&self) -> Vec<ProjectStats> {
        let mut all_stats: Vec<ProjectStats> = Vec::new();

        for artifact in &self.artifacts {
            match all_stats
                .iter_mut()
                .find(|stats| stats.project == artifact.project())
            {
                Some(stats) => stats.add(artifact),
                None => {
                    let mut stats = ProjectStats::new(artifact.project());
                    stats.add(artifact);
                    all_stats.push(stats);
                }
            }
        }

        all_stats.sort_by_key(|stats| Reverse(stats.size));
        all_stats
    }
}

impl fmt::Display for Report {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        writeln!(formatter, "Found {} directory(ies):", self.artifacts.len())?;
        for artifact in &self.artifacts {
            writeln!(formatter, "  {artifact}")?;
        }

        writeln!(formatter, "\nBy project type:")?;
        for stats in self.stats_by_project() {
            writeln!(formatter, "  {stats}")?;
        }

        write!(
            formatter,
            "\nSpace that can be freed: {}",
            self.total_size()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn artifact(project: &'static str, bytes: u64) -> Artifact {
        Artifact::new(PathBuf::from("some/path"), project, Size::from_bytes(bytes))
    }

    #[test]
    fn adds_up_every_artifact() {
        let report = Report::new(vec![artifact("Rust", 100), artifact("Node.js", 50)]);

        assert_eq!(report.total_size(), Size::from_bytes(150));
    }

    #[test]
    fn groups_artifacts_by_project_biggest_first() {
        let report = Report::new(vec![
            artifact("Rust", 100),
            artifact("Node.js", 300),
            artifact("Rust", 50),
        ]);

        let stats = report.stats_by_project();

        assert_eq!(stats.len(), 2);
        assert_eq!(
            (stats[0].project, stats[0].count, stats[0].size),
            ("Node.js", 1, Size::from_bytes(300))
        );
        assert_eq!(
            (stats[1].project, stats[1].count, stats[1].size),
            ("Rust", 2, Size::from_bytes(150))
        );
    }
}
