use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use self_update::backends::github::Update;
use self_update::cargo_crate_version;
use self_update::check_interval::UpdateCheckGuard;
use self_update::version::{bump_is_major, bump_is_minor};

const REPO_OWNER: &str = "Warmzzz";
const REPO_NAME: &str = "sweep-cli";
const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
const CHECK_TIMEOUT: Duration = Duration::from_secs(2);

/// Describes where sweep's releases live and how they are packaged (see `release.yml`).
fn releases() -> Result<self_update::backends::github::Update> {
    Ok(Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name("sweep")
        .bin_path_in_archive("sweep-v{{ version }}-{{ target }}/{{ bin }}")
        // Refuses the download if it does not match the release's SHA256SUMS.txt.
        .checksum_from_asset("SHA256SUMS.txt")
        .current_version(cargo_crate_version!())
        .timeout(Duration::from_secs(60))
        .build()?)
}

/// Downloads the latest release and replaces the running executable, after confirmation.
pub fn update() -> Result<()> {
    match releases()?.update_extended()?.version() {
        Some(version) => println!("sweep is now up to date ({version})."),
        None => println!("sweep is already up to date ({}).", cargo_crate_version!()),
    }
    Ok(())
}

/// Asks GitHub for the latest version in the background, at most once a day.
/// The receiver gets the new version only when it is a minor or major one.
pub fn check_in_background(stamp_path: PathBuf) -> Receiver<String> {
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        // A new user has no `~/.config/sweep` folder yet: without it, the stamp is never saved.
        if let Some(folder) = stamp_path.parent() {
            let _ = fs::create_dir_all(folder);
        }
        let guard = UpdateCheckGuard::new(stamp_path, CHECK_INTERVAL);
        if !guard.should_check().unwrap_or(false) {
            return;
        }
        // Recorded first: offline, sweep would otherwise try (and wait) on every run.
        let _ = guard.record_check();

        if let Some(latest) = latest_version()
            && is_worth_telling(cargo_crate_version!(), &latest)
        {
            let _ = sender.send(latest);
        }
    });

    receiver
}

/// Waits a little for the background check, so a fast scan still gets the news.
pub fn print_notice(receiver: Receiver<String>) {
    if let Ok(latest) = receiver.recv_timeout(CHECK_TIMEOUT) {
        println!(
            "\nA new version of sweep is available: {} -> {latest}\nRun `sweep update` to install it.",
            cargo_crate_version!()
        );
    }
}

fn latest_version() -> Option<String> {
    let newer = releases().ok()?.get_latest_release().ok()?;
    Some(newer.latest()?.version().to_string())
}

/// Bug fix releases (1.1.0 -> 1.1.1) are not worth interrupting anyone for.
fn is_worth_telling(current: &str, latest: &str) -> bool {
    bump_is_major(current, latest).unwrap_or(false)
        || bump_is_minor(current, latest).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tells_about_minor_and_major_versions_only() {
        assert!(is_worth_telling("1.1.0", "1.2.0"));
        assert!(is_worth_telling("1.1.0", "2.0.0"));
        assert!(!is_worth_telling("1.1.0", "1.1.1"));
        assert!(!is_worth_telling("1.1.0", "1.1.0"));
        assert!(!is_worth_telling("1.1.0", "1.0.0"));
        assert!(!is_worth_telling("1.1.0", "not a version"));
    }
}
